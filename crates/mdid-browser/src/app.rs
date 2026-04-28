use leptos::*;
use serde::{Deserialize, Serialize};
use std::fmt;

const DEFAULT_FIELD_POLICY_JSON: &str = "[\n  {\n    \"header\": \"patient_id\",\n    \"phi_type\": \"patient_id\",\n    \"action\": \"encode\"\n  },\n  {\n    \"header\": \"patient_name\",\n    \"phi_type\": \"patient_name\",\n    \"action\": \"review\"\n  }\n]";
const IDLE_SUMMARY: &str = "Awaiting submission.";
const IDLE_REVIEW_QUEUE: &str = "No review items yet.";
const FETCH_UNAVAILABLE_MESSAGE: &str =
    "Runtime submission is only available from a wasm32 browser build.";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InputMode {
    CsvText,
    XlsxBase64,
    PdfBase64,
}

impl InputMode {
    #[cfg_attr(not(test), allow(dead_code))]
    fn from_file_name(file_name: &str) -> Option<Self> {
        let file_name = file_name.to_lowercase();

        if file_name.ends_with(".csv") {
            Some(Self::CsvText)
        } else if file_name.ends_with(".xlsx") {
            Some(Self::XlsxBase64)
        } else if file_name.ends_with(".pdf") {
            Some(Self::PdfBase64)
        } else {
            None
        }
    }

    fn from_select_value(value: &str) -> Self {
        match value {
            "xlsx-base64" => Self::XlsxBase64,
            "pdf-base64" => Self::PdfBase64,
            _ => Self::CsvText,
        }
    }

    fn select_value(self) -> &'static str {
        match self {
            Self::CsvText => "csv-text",
            Self::XlsxBase64 => "xlsx-base64",
            Self::PdfBase64 => "pdf-base64",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::CsvText => "CSV text",
            Self::XlsxBase64 => "XLSX base64",
            Self::PdfBase64 => "PDF base64",
        }
    }

    fn payload_hint(self) -> &'static str {
        match self {
            Self::CsvText => "Paste CSV rows here",
            Self::XlsxBase64 => "Paste base64-encoded XLSX content here",
            Self::PdfBase64 => "Paste base64-encoded PDF content here",
        }
    }

    fn disclosure_copy(self) -> Option<&'static str> {
        match self {
            Self::CsvText => None,
            Self::XlsxBase64 => Some(
                "XLSX mode only processes the first non-empty worksheet. Sheet selection is not supported in this browser flow.",
            ),
            Self::PdfBase64 => Some("PDF mode is review-only: it reports text-layer candidates and OCR-required pages, but does not perform OCR, visual redaction, handwriting handling, or PDF rewrite/export."),
        }
    }

    fn endpoint(self) -> &'static str {
        match self {
            Self::CsvText => "/tabular/deidentify",
            Self::XlsxBase64 => "/tabular/deidentify/xlsx",
            Self::PdfBase64 => "/pdf/deidentify",
        }
    }

    fn is_pdf(self) -> bool {
        matches!(self, Self::PdfBase64)
    }
}

#[derive(Clone, Eq, PartialEq)]
struct BrowserFlowState {
    input_mode: InputMode,
    payload: String,
    source_name: String,
    imported_file_name: Option<String>,
    field_policy_json: String,
    result_output: String,
    summary: String,
    review_queue: String,
    error_banner: Option<String>,
    is_submitting: bool,
    state_revision: u64,
    next_submission_token: u64,
    active_submission_token: Option<u64>,
}

// BrowserFlowState may carry PHI-bearing local payloads, file names, and runtime text;
// keep this Debug implementation redacted for those fields.
impl fmt::Debug for BrowserFlowState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BrowserFlowState")
            .field("input_mode", &self.input_mode)
            .field("payload", &"<redacted>")
            .field("source_name", &"<redacted>")
            .field("imported_file_name", &self.imported_file_name.as_ref().map(|_| "<redacted>"))
            .field("field_policy_json", &"<redacted>")
            .field("result_output", &"<redacted>")
            .field("summary", &self.summary)
            .field("review_queue", &"<redacted>")
            .field("error_banner", &self.error_banner.as_ref().map(|_| "<redacted>"))
            .field("is_submitting", &self.is_submitting)
            .field("state_revision", &self.state_revision)
            .field("next_submission_token", &self.next_submission_token)
            .field("active_submission_token", &self.active_submission_token)
            .finish()
    }
}

impl Default for BrowserFlowState {
    fn default() -> Self {
        Self {
            input_mode: InputMode::CsvText,
            payload: String::new(),
            source_name: "local-review.pdf".to_string(),
            imported_file_name: None,
            field_policy_json: DEFAULT_FIELD_POLICY_JSON.to_string(),
            result_output: String::new(),
            summary: IDLE_SUMMARY.to_string(),
            review_queue: IDLE_REVIEW_QUEUE.to_string(),
            error_banner: None,
            is_submitting: false,
            state_revision: 0,
            next_submission_token: 1,
            active_submission_token: None,
        }
    }
}

impl BrowserFlowState {
    #[cfg_attr(not(test), allow(dead_code))]
    fn apply_imported_file(&mut self, file_name: &str, payload: &str, mode: InputMode) {
        self.input_mode = mode;
        self.source_name = file_name.to_string();
        self.imported_file_name = Some(file_name.to_string());
        self.invalidate_generated_state();
        self.payload = payload.to_string();
    }

    #[cfg_attr(not(test), allow(dead_code))]
    fn suggested_export_file_name(&self) -> &'static str {
        match self.input_mode {
            InputMode::CsvText => "mdid-browser-output.csv",
            InputMode::XlsxBase64 => "mdid-browser-output.xlsx.base64.txt",
            InputMode::PdfBase64 => "mdid-browser-review-report.txt",
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    fn can_export_output(&self) -> bool {
        !self.result_output.trim().is_empty()
    }

    fn clear_generated_state(&mut self) {
        self.result_output.clear();
        self.summary = IDLE_SUMMARY.to_string();
        self.review_queue = IDLE_REVIEW_QUEUE.to_string();
        self.error_banner = None;
    }

    fn invalidate_generated_state(&mut self) {
        self.state_revision += 1;
        self.clear_generated_state();
    }

    fn validate_submission(&self) -> Result<RuntimeSubmitRequest, String> {
        if self.payload.trim().is_empty() {
            return Err(format!(
                "{} payload is required before submitting.",
                self.input_mode.label()
            ));
        }

        if self.input_mode.is_pdf() && self.source_name.trim().is_empty() {
            return Err("PDF source name is required before submitting.".to_string());
        }

        if !self.input_mode.is_pdf() && self.field_policy_json.trim().is_empty() {
            return Err("Field policy JSON is required before submitting.".to_string());
        }

        build_submit_request(
            self.input_mode,
            &self.payload,
            &self.source_name,
            &self.field_policy_json,
        )
    }

    fn begin_submit(&mut self) -> Result<SubmissionHandle, ()> {
        if self.is_submitting {
            return Err(());
        }

        self.result_output.clear();
        self.summary = "Submitting to runtime...".to_string();
        self.review_queue = IDLE_REVIEW_QUEUE.to_string();
        self.error_banner = None;
        self.is_submitting = true;

        match self.validate_submission() {
            Ok(request) => {
                let submission_token = self.next_submission_token;
                self.next_submission_token += 1;
                self.active_submission_token = Some(submission_token);

                Ok(SubmissionHandle {
                    request,
                    input_mode: self.input_mode,
                    submission_token,
                    state_revision: self.state_revision,
                })
            }
            Err(message) => {
                self.active_submission_token = None;
                self.is_submitting = false;
                self.clear_generated_state();
                self.error_banner = Some(message);
                Err(())
            }
        }
    }

    fn apply_runtime_success(
        &mut self,
        submission_token: u64,
        state_revision: u64,
        response: RuntimeResponseEnvelope,
    ) {
        if self.active_submission_token != Some(submission_token) {
            return;
        }

        self.active_submission_token = None;
        self.is_submitting = false;

        if self.state_revision != state_revision {
            return;
        }

        self.result_output = response.rewritten_output;
        self.summary = response.summary;
        self.review_queue = response.review_queue;
        self.error_banner = None;
    }

    fn apply_runtime_error(&mut self, submission_token: u64, state_revision: u64, message: String) {
        if self.active_submission_token != Some(submission_token) {
            return;
        }

        self.active_submission_token = None;
        self.is_submitting = false;

        if self.state_revision != state_revision {
            return;
        }

        self.result_output.clear();
        self.summary = IDLE_SUMMARY.to_string();
        self.review_queue = IDLE_REVIEW_QUEUE.to_string();
        self.error_banner = Some(message);
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct FieldPolicyRequest {
    header: String,
    phi_type: String,
    action: String,
}

#[derive(Clone, Eq, PartialEq, Serialize)]
struct CsvSubmitRequest {
    csv: String,
    policies: Vec<FieldPolicyRequest>,
}

#[derive(Clone, Eq, PartialEq, Serialize)]
struct XlsxSubmitRequest {
    workbook_base64: String,
    field_policies: Vec<FieldPolicyRequest>,
}

#[derive(Clone, Eq, PartialEq, Serialize)]
struct PdfSubmitRequest {
    pdf_bytes_base64: String,
    source_name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RuntimeSubmitRequest {
    endpoint: &'static str,
    body_json: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SubmissionHandle {
    request: RuntimeSubmitRequest,
    input_mode: InputMode,
    submission_token: u64,
    state_revision: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
struct RuntimeSummary {
    total_rows: usize,
    encoded_cells: usize,
    review_required_cells: usize,
    failed_rows: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
struct RuntimeReviewCandidate {
    row_index: usize,
    column: String,
    value: String,
    phi_type: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
struct CsvRuntimeSuccessResponse {
    csv: String,
    summary: RuntimeSummary,
    review_queue: Vec<RuntimeReviewCandidate>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
struct XlsxRuntimeSuccessResponse {
    rewritten_workbook_base64: String,
    summary: RuntimeSummary,
    review_queue: Vec<RuntimeReviewCandidate>,
}

#[derive(Clone, PartialEq, Deserialize)]
struct PdfRuntimeSuccessResponse {
    summary: PdfExtractionSummary,
    page_statuses: Vec<PdfPageStatusResponse>,
    review_queue: Vec<PdfReviewCandidate>,
    // PDF mode is review-only; rewrite/export bytes are intentionally ignored.
    #[allow(dead_code)]
    rewritten_pdf_bytes_base64: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
struct PdfExtractionSummary {
    total_pages: usize,
    text_layer_pages: usize,
    ocr_required_pages: usize,
    extracted_candidates: usize,
    review_required_candidates: usize,
}

#[derive(Clone, Eq, PartialEq, Deserialize)]
struct PdfPageStatusResponse {
    page: PdfPageRef,
    status: String,
}

#[derive(Clone, Eq, PartialEq, Deserialize)]
struct PdfPageRef {
    label: String,
    page_number: usize,
}

#[derive(Clone, PartialEq, Deserialize)]
struct PdfReviewCandidate {
    page: PdfPageRef,
    source_text: String,
    phi_type: String,
    confidence: u8,
    decision: String,
}

#[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
struct ErrorBody {
    code: String,
    message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RuntimeResponseEnvelope {
    rewritten_output: String,
    summary: String,
    review_queue: String,
}

fn build_submit_request(
    input_mode: InputMode,
    payload: &str,
    source_name: &str,
    field_policy_json: &str,
) -> Result<RuntimeSubmitRequest, String> {
    if input_mode.is_pdf() {
        if source_name.trim().is_empty() {
            return Err("PDF source name is required before submitting.".to_string());
        }

        let body_json = serde_json::to_string(&PdfSubmitRequest {
            pdf_bytes_base64: payload.trim().to_string(),
            source_name: source_name.trim().to_string(),
        })
        .map_err(|error| format!("Failed to serialize runtime request: {error}"))?;

        return Ok(RuntimeSubmitRequest {
            endpoint: input_mode.endpoint(),
            body_json,
        });
    }

    let policies: Vec<FieldPolicyRequest> = serde_json::from_str(field_policy_json)
        .map_err(|error| format!("Field policy JSON must be a JSON array of policies: {error}"))?;

    if policies.is_empty() {
        return Err("Field policy JSON must include at least one policy.".to_string());
    }

    let body_json = match input_mode {
        InputMode::CsvText => serde_json::to_string(&CsvSubmitRequest {
            csv: payload.trim().to_string(),
            policies,
        }),
        InputMode::XlsxBase64 => serde_json::to_string(&XlsxSubmitRequest {
            workbook_base64: payload.trim().to_string(),
            field_policies: policies,
        }),
        InputMode::PdfBase64 => unreachable!("PDF requests are handled before policy parsing"),
    }
    .map_err(|error| format!("Failed to serialize runtime request: {error}"))?;

    Ok(RuntimeSubmitRequest {
        endpoint: input_mode.endpoint(),
        body_json,
    })
}

fn parse_runtime_success(
    input_mode: InputMode,
    response_body: &str,
) -> Result<RuntimeResponseEnvelope, String> {
    match input_mode {
        InputMode::CsvText => {
            let parsed: CsvRuntimeSuccessResponse = serde_json::from_str(response_body)
                .map_err(|error| format!("Failed to parse runtime success response: {error}"))?;
            Ok(RuntimeResponseEnvelope {
                rewritten_output: parsed.csv,
                summary: format_summary(&parsed.summary),
                review_queue: format_review_queue(&parsed.review_queue),
            })
        }
        InputMode::XlsxBase64 => {
            let parsed: XlsxRuntimeSuccessResponse = serde_json::from_str(response_body)
                .map_err(|error| format!("Failed to parse runtime success response: {error}"))?;
            Ok(RuntimeResponseEnvelope {
                rewritten_output: parsed.rewritten_workbook_base64,
                summary: format_summary(&parsed.summary),
                review_queue: format_review_queue(&parsed.review_queue),
            })
        }
        InputMode::PdfBase64 => {
            let parsed: PdfRuntimeSuccessResponse = serde_json::from_str(response_body)
                .map_err(|error| format!("Failed to parse runtime success response: {error}"))?;
            Ok(RuntimeResponseEnvelope {
                rewritten_output:
                    "PDF rewrite/export unavailable: runtime returned review-only PDF analysis."
                        .to_string(),
                summary: format_pdf_summary(&parsed.summary, &parsed.page_statuses),
                review_queue: format_pdf_review_queue(&parsed.review_queue),
            })
        }
    }
}

#[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
fn parse_runtime_error(status: u16, response_body: &str) -> String {
    const MAX_MESSAGE_LEN: usize = 240;

    let message = serde_json::from_str::<ErrorEnvelope>(response_body)
        .map(|envelope| format!("{}: {}", envelope.error.code, envelope.error.message))
        .unwrap_or_else(|_| {
            let trimmed = response_body.trim();
            if trimmed.is_empty() {
                format!("runtime request failed with status {status}")
            } else {
                format!("runtime request failed with status {status}: {trimmed}")
            }
        });

    truncate_for_banner(&message, MAX_MESSAGE_LEN)
}

#[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
fn truncate_for_banner(message: &str, max_chars: usize) -> String {
    let char_count = message.chars().count();
    if char_count <= max_chars {
        return message.to_string();
    }

    let truncated = message
        .chars()
        .take(max_chars.saturating_sub(1))
        .collect::<String>();
    format!("{truncated}…")
}

fn format_summary(summary: &RuntimeSummary) -> String {
    format!(
        "total_rows: {}\nencoded_cells: {}\nreview_required_cells: {}\nfailed_rows: {}",
        summary.total_rows,
        summary.encoded_cells,
        summary.review_required_cells,
        summary.failed_rows
    )
}

fn format_review_queue(review_queue: &[RuntimeReviewCandidate]) -> String {
    if review_queue.is_empty() {
        return "No review items returned.".to_string();
    }

    review_queue
        .iter()
        .map(|candidate| {
            format!(
                "- row {} / {} / {}: {}",
                candidate.row_index, candidate.column, candidate.phi_type, candidate.value
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_pdf_summary(
    summary: &PdfExtractionSummary,
    page_statuses: &[PdfPageStatusResponse],
) -> String {
    let mut lines = vec![
        format!("total_pages: {}", summary.total_pages),
        format!("text_layer_pages: {}", summary.text_layer_pages),
        format!("ocr_required_pages: {}", summary.ocr_required_pages),
        format!("extracted_candidates: {}", summary.extracted_candidates),
        format!(
            "review_required_candidates: {}",
            summary.review_required_candidates
        ),
        "page_statuses:".to_string(),
    ];

    lines.extend(page_statuses.iter().map(|page_status| {
        format!(
            "- page {} ({}): {}",
            page_status.page.page_number, page_status.page.label, page_status.status
        )
    }));

    lines.join("\n")
}

fn format_pdf_review_queue(review_queue: &[PdfReviewCandidate]) -> String {
    if review_queue.is_empty() {
        return "No review items returned.".to_string();
    }

    review_queue
        .iter()
        .map(|candidate| {
            format!(
                "- page {} / {} / confidence {} / {}: {}",
                candidate.page.page_number,
                candidate.phi_type,
                candidate.confidence,
                candidate.decision,
                candidate.source_text
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(target_arch = "wasm32")]
async fn perform_runtime_request(request: RuntimeSubmitRequest) -> Result<String, String> {
    use gloo_net::http::Request;

    let response = Request::post(request.endpoint)
        .header("content-type", "application/json")
        .body(request.body_json)
        .map_err(|error| format!("Failed to build runtime request: {error}"))?
        .send()
        .await
        .map_err(|error| parse_runtime_error(0, &error.to_string()))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("Failed to read runtime response: {error}"))?;

    if (200..300).contains(&status) {
        Ok(body)
    } else {
        Err(parse_runtime_error(status, &body))
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn perform_runtime_request(_request: RuntimeSubmitRequest) -> Result<String, String> {
    Err(FETCH_UNAVAILABLE_MESSAGE.to_string())
}

#[component]
pub fn App() -> impl IntoView {
    let state = create_rw_signal(BrowserFlowState::default());

    let on_mode_change = move |event| {
        let next_mode = InputMode::from_select_value(&event_target_value(&event));
        state.update(|state| {
            state.input_mode = next_mode;
            state.invalidate_generated_state();
        });
    };

    let on_payload_input = move |event| {
        let next_payload = event_target_value(&event);
        state.update(|state| {
            state.payload = next_payload;
            state.invalidate_generated_state();
        });
    };

    let on_source_name_input = move |event| {
        let next_source_name = event_target_value(&event);
        state.update(|state| {
            state.source_name = next_source_name;
            state.invalidate_generated_state();
        });
    };

    let on_field_policy_input = move |event| {
        let next_policy = event_target_value(&event);
        state.update(|state| {
            state.field_policy_json = next_policy;
            state.invalidate_generated_state();
        });
    };

    let on_submit = move |_| {
        let maybe_request = state.with_untracked(|state| {
            let mut next_state = state.clone();
            let request = next_state.begin_submit().ok();
            (next_state, request)
        });

        state.set(maybe_request.0);

        if let Some(handle) = maybe_request.1 {
            spawn_local(async move {
                match perform_runtime_request(handle.request).await {
                    Ok(body) => match parse_runtime_success(handle.input_mode, &body) {
                        Ok(response) => state.update(|state| {
                            state.apply_runtime_success(
                                handle.submission_token,
                                handle.state_revision,
                                response,
                            )
                        }),
                        Err(message) => state.update(|state| {
                            state.apply_runtime_error(
                                handle.submission_token,
                                handle.state_revision,
                                message,
                            )
                        }),
                    },
                    Err(message) => state.update(|state| {
                        state.apply_runtime_error(
                            handle.submission_token,
                            handle.state_revision,
                            message,
                        )
                    }),
                }
            });
        }
    };

    view! {
        <main class="tabular-flow-shell">
            <h1>"med-de-id browser tool"</h1>
            <p>"Bounded tabular de-identification and PDF review flow"</p>

            <section>
                <h2>"Input"</h2>
                <label>
                    "Input mode"
                    <select on:change=on_mode_change prop:value=move || state.get().input_mode.select_value()>
                        <option value="csv-text">"CSV text"</option>
                        <option value="xlsx-base64">"XLSX base64"</option>
                        <option value="pdf-base64">"PDF base64"</option>
                    </select>
                </label>

                <label>
                    "Payload"
                    <textarea
                        on:input=on_payload_input
                        prop:value=move || state.get().payload
                        placeholder=move || state.get().input_mode.payload_hint()
                        rows="12"
                    />
                </label>

                <Show when=move || state.get().input_mode.disclosure_copy().is_some()>
                    <p class="input-disclosure">
                        {move || state.get().input_mode.disclosure_copy().unwrap_or_default()}
                    </p>
                </Show>

                <Show when=move || state.get().input_mode.is_pdf()>
                    <label>
                        "Source name"
                        <input
                            on:input=on_source_name_input
                            prop:value=move || state.get().source_name
                            type="text"
                        />
                    </label>
                </Show>

                <Show when=move || !state.get().input_mode.is_pdf()>
                    <label>
                        "Field policy JSON"
                        <textarea
                            on:input=on_field_policy_input
                            prop:value=move || state.get().field_policy_json
                            rows="10"
                        />
                    </label>
                </Show>

                <button on:click=on_submit disabled=move || state.get().is_submitting type="button">
                    {move || if state.get().is_submitting { "Submitting..." } else { "Submit" }}
                </button>
            </section>

            <Show when=move || state.get().error_banner.is_some()>
                <section aria-live="polite" class="error-banner">
                    <h2>"Error"</h2>
                    <p>{move || state.get().error_banner.unwrap_or_default()}</p>
                </section>
            </Show>

            <section>
                <h2>"Rewritten output"</h2>
                <pre>{move || state.get().result_output}</pre>
            </section>

            <section>
                <h2>"Summary"</h2>
                <pre>{move || state.get().summary}</pre>
            </section>

            <section>
                <h2>"Review queue"</h2>
                <pre>{move || state.get().review_queue}</pre>
            </section>
        </main>
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_submit_request, format_review_queue, format_summary, parse_runtime_error,
        parse_runtime_success, InputMode, RuntimeReviewCandidate, RuntimeSummary, BrowserFlowState,
        DEFAULT_FIELD_POLICY_JSON, FETCH_UNAVAILABLE_MESSAGE, IDLE_REVIEW_QUEUE, IDLE_SUMMARY,
    };
    use serde_json::json;

    #[test]
    fn browser_flow_state_defaults_to_csv_shell() {
        let state = BrowserFlowState::default();

        assert_eq!(state.input_mode, InputMode::CsvText);
        assert!(state.payload.is_empty());
        assert_eq!(state.field_policy_json, DEFAULT_FIELD_POLICY_JSON);
        assert!(state.result_output.is_empty());
        assert_eq!(state.summary, IDLE_SUMMARY);
        assert_eq!(state.review_queue, IDLE_REVIEW_QUEUE);
        assert!(state.error_banner.is_none());
        assert!(!state.is_submitting);
        assert_eq!(state.state_revision, 0);
        assert_eq!(state.next_submission_token, 1);
        assert!(state.active_submission_token.is_none());
    }

    #[test]
    fn browser_flow_state_debug_redacts_imported_file_metadata() {
        let state = BrowserFlowState {
            payload: "name\nJane Patient".to_string(),
            source_name: "Jane Patient.csv".to_string(),
            imported_file_name: Some("Jane Patient.csv".to_string()),
            field_policy_json: r#"{"name":"Jane Patient"}"#.to_string(),
            result_output: "redacted output for Jane Patient".to_string(),
            ..BrowserFlowState::default()
        };

        let debug_output = format!("{state:?}");

        assert!(!debug_output.contains("Jane Patient.csv"));
        assert!(!debug_output.contains("Jane Patient"));
        assert!(!debug_output.contains("redacted output"));
        assert!(debug_output.contains("input_mode"));
        assert!(debug_output.contains("is_submitting"));
    }

    #[test]
    fn browser_flow_state_debug_redacts_error_banner() {
        let state = BrowserFlowState {
            error_banner: Some("Runtime fallback included response body for Jane Patient".to_string()),
            ..BrowserFlowState::default()
        };

        let debug_output = format!("{state:?}");

        assert!(!debug_output.contains("Jane Patient"));
        assert!(debug_output.contains("error_banner"));
    }

    #[test]
    fn file_import_metadata_updates_payload_source_and_clears_generated_state() {
        let mut state = BrowserFlowState {
            result_output: "old output".to_string(),
            summary: "old summary".to_string(),
            review_queue: "old review".to_string(),
            error_banner: Some("old error".to_string()),
            ..BrowserFlowState::default()
        };

        state.apply_imported_file("report.pdf", "UERG", InputMode::PdfBase64);

        assert_eq!(state.input_mode, InputMode::PdfBase64);
        assert_eq!(state.payload, "UERG");
        assert_eq!(state.source_name, "report.pdf");
        assert_eq!(state.result_output, "");
        assert_eq!(state.summary, IDLE_SUMMARY);
        assert_eq!(state.review_queue, IDLE_REVIEW_QUEUE);
        assert!(state.error_banner.is_none());
        assert_eq!(state.imported_file_name.as_deref(), Some("report.pdf"));
    }

    #[test]
    fn imported_file_name_selects_mode_from_safe_extension() {
        assert_eq!(InputMode::from_file_name("patients.csv"), Some(InputMode::CsvText));
        assert_eq!(InputMode::from_file_name("workbook.XLSX"), Some(InputMode::XlsxBase64));
        assert_eq!(InputMode::from_file_name("scan.PDF"), Some(InputMode::PdfBase64));
        assert_eq!(InputMode::from_file_name("archive.zip"), None);
    }

    #[test]
    fn export_filename_is_safe_and_mode_specific() {
        let mut state = BrowserFlowState {
            imported_file_name: Some("Jane Patient.csv".to_string()),
            ..BrowserFlowState::default()
        };
        assert_eq!(state.suggested_export_file_name(), "mdid-browser-output.csv");

        state.input_mode = InputMode::XlsxBase64;
        state.imported_file_name = Some("clinic workbook.xlsx".to_string());
        assert_eq!(state.suggested_export_file_name(), "mdid-browser-output.xlsx.base64.txt");

        state.input_mode = InputMode::PdfBase64;
        state.imported_file_name = Some("scan.pdf".to_string());
        assert_eq!(state.suggested_export_file_name(), "mdid-browser-review-report.txt");
    }

    #[test]
    fn export_is_available_only_after_runtime_output_exists() {
        let mut state = BrowserFlowState::default();
        assert!(!state.can_export_output());

        state.result_output = "rewritten".to_string();
        assert!(state.can_export_output());

        state.result_output = "   ".to_string();
        assert!(!state.can_export_output());
    }

    #[test]
    fn submit_requires_payload_before_runtime_request() {
        let mut state = BrowserFlowState::default();
        let result = state.begin_submit();

        assert!(result.is_err());
        assert_eq!(
            state.error_banner.as_deref(),
            Some("CSV text payload is required before submitting.")
        );
        assert!(state.result_output.is_empty());
        assert_eq!(state.summary, IDLE_SUMMARY);
        assert_eq!(state.review_queue, IDLE_REVIEW_QUEUE);
    }

    #[test]
    fn submit_requires_non_blank_field_policy_before_runtime_request() {
        let mut state = BrowserFlowState {
            payload: "patient_id,name\n1,Alice".to_string(),
            field_policy_json: "   \n\t".to_string(),
            ..BrowserFlowState::default()
        };

        let result = state.begin_submit();

        assert!(result.is_err());
        assert_eq!(
            state.error_banner.as_deref(),
            Some("Field policy JSON is required before submitting.")
        );
        assert!(state.result_output.is_empty());
        assert_eq!(state.summary, IDLE_SUMMARY);
        assert_eq!(state.review_queue, IDLE_REVIEW_QUEUE);
    }

    #[test]
    fn xlsx_mode_disclosure_matches_runtime_limits() {
        assert_eq!(InputMode::CsvText.disclosure_copy(), None);
        assert_eq!(
            InputMode::XlsxBase64.disclosure_copy(),
            Some(
                "XLSX mode only processes the first non-empty worksheet. Sheet selection is not supported in this browser flow.",
            )
        );
    }

    #[test]
    fn pdf_mode_disclosure_matches_review_only_runtime_limits() {
        assert_eq!(
            InputMode::PdfBase64.payload_hint(),
            "Paste base64-encoded PDF content here"
        );
        assert_eq!(
            InputMode::PdfBase64.disclosure_copy(),
            Some("PDF mode is review-only: it reports text-layer candidates and OCR-required pages, but does not perform OCR, visual redaction, handwriting handling, or PDF rewrite/export.")
        );
        assert_eq!(InputMode::PdfBase64.endpoint(), "/pdf/deidentify");
    }

    #[test]
    fn build_submit_request_targets_pdf_endpoint_without_field_policies() {
        let request = build_submit_request(
            InputMode::PdfBase64,
            "JVBERi0xLjQK...\n",
            "Ignored Report.pdf",
            "",
        )
        .unwrap();

        assert_eq!(request.endpoint, "/pdf/deidentify");
        let body: serde_json::Value = serde_json::from_str(&request.body_json).unwrap();
        assert_eq!(body["pdf_bytes_base64"], "JVBERi0xLjQK...");
        assert_eq!(body["source_name"], "Ignored Report.pdf");
        assert!(body.get("policies").is_none());
        assert!(body.get("field_policies").is_none());
    }

    #[test]
    fn pdf_submit_requires_source_name_before_runtime_request() {
        let mut state = BrowserFlowState {
            input_mode: InputMode::PdfBase64,
            payload: "JVBERi0xLjQK".to_string(),
            source_name: "   ".to_string(),
            ..BrowserFlowState::default()
        };

        let result = state.begin_submit();

        assert!(result.is_err());
        assert_eq!(
            state.error_banner.as_deref(),
            Some("PDF source name is required before submitting.")
        );
    }

    #[test]
    fn parse_pdf_runtime_success_renders_review_only_summary_and_page_statuses() {
        let response = parse_runtime_success(
            InputMode::PdfBase64,
            &json!({
                "summary": {
                    "total_pages": 2,
                    "text_layer_pages": 1,
                    "ocr_required_pages": 1,
                    "extracted_candidates": 1,
                    "review_required_candidates": 1
                },
                "page_statuses": [
                    {"page": {"label": "radiology/report.pdf", "page_number": 1}, "status": "text_layer_present"},
                    {"page": {"label": "radiology/report.pdf", "page_number": 2}, "status": "ocr_required"}
                ],
                "review_queue": [
                    {
                        "page": {"label": "radiology/report.pdf", "page_number": 1},
                        "source_text": "Alice Smith",
                        "phi_type": "patient_name",
                        "confidence": 20,
                        "decision": "needs_review"
                    }
                ],
                "rewritten_pdf_bytes_base64": null
            })
            .to_string(),
        )
        .unwrap();

        assert_eq!(
            response.rewritten_output,
            "PDF rewrite/export unavailable: runtime returned review-only PDF analysis."
        );
        assert!(response.summary.contains("total_pages: 2"));
        assert!(response.summary.contains("ocr_required_pages: 1"));
        assert!(response.summary.contains("page_statuses:"));
        assert!(response
            .summary
            .contains("- page 1 (radiology/report.pdf): text_layer_present"));
        assert!(response
            .summary
            .contains("- page 2 (radiology/report.pdf): ocr_required"));
        assert_eq!(
            response.review_queue,
            "- page 1 / patient_name / confidence 20 / needs_review: Alice Smith"
        );
    }

    #[test]
    fn build_submit_request_targets_csv_endpoint() {
        let request = build_submit_request(
            InputMode::CsvText,
            "patient_id,patient_name\nMRN-001,Alice Smith\n",
            "local-review.pdf",
            DEFAULT_FIELD_POLICY_JSON,
        )
        .unwrap();

        assert_eq!(request.endpoint, "/tabular/deidentify");
        let body: serde_json::Value = serde_json::from_str(&request.body_json).unwrap();
        assert_eq!(body["csv"], "patient_id,patient_name\nMRN-001,Alice Smith");
        assert!(body["policies"].is_array());
        assert!(body.get("field_policies").is_none());
    }

    #[test]
    fn build_submit_request_targets_xlsx_endpoint() {
        let request = build_submit_request(
            InputMode::XlsxBase64,
            "UEsDBBQAAAAIA...\n",
            "local-review.pdf",
            DEFAULT_FIELD_POLICY_JSON,
        )
        .unwrap();

        assert_eq!(request.endpoint, "/tabular/deidentify/xlsx");
        let body: serde_json::Value = serde_json::from_str(&request.body_json).unwrap();
        assert_eq!(body["workbook_base64"], "UEsDBBQAAAAIA...");
        assert!(body["field_policies"].is_array());
        assert!(body.get("policies").is_none());
    }

    #[test]
    fn build_submit_request_rejects_non_array_policy_json() {
        let error = build_submit_request(
            InputMode::CsvText,
            "patient_id\n1",
            "local-review.pdf",
            "{\"columns\":{}}",
        )
        .unwrap_err();

        assert!(error.contains("Field policy JSON must be a JSON array of policies"));
    }

    #[test]
    fn parse_csv_runtime_success_renders_rewritten_csv() {
        let response = parse_runtime_success(
            InputMode::CsvText,
            &json!({
                "csv": "patient_id,patient_name\ntok-123,Alice Smith\n",
                "summary": {
                    "total_rows": 1,
                    "encoded_cells": 1,
                    "review_required_cells": 1,
                    "failed_rows": 0
                },
                "review_queue": [
                    {
                        "row_index": 1,
                        "column": "patient_name",
                        "value": "Alice Smith",
                        "phi_type": "patient_name"
                    }
                ]
            })
            .to_string(),
        )
        .unwrap();

        assert_eq!(
            response.rewritten_output,
            "patient_id,patient_name\ntok-123,Alice Smith\n"
        );
        assert!(response.summary.contains("total_rows: 1"));
        assert_eq!(
            response.review_queue,
            "- row 1 / patient_name / patient_name: Alice Smith"
        );
    }

    #[test]
    fn parse_xlsx_runtime_success_renders_rewritten_workbook_base64() {
        let response = parse_runtime_success(
            InputMode::XlsxBase64,
            &json!({
                "rewritten_workbook_base64": "UEsDBBQAAAAIA...",
                "summary": {
                    "total_rows": 2,
                    "encoded_cells": 2,
                    "review_required_cells": 2,
                    "failed_rows": 0
                },
                "review_queue": []
            })
            .to_string(),
        )
        .unwrap();

        assert_eq!(response.rewritten_output, "UEsDBBQAAAAIA...");
        assert!(response.summary.contains("encoded_cells: 2"));
        assert_eq!(response.review_queue, "No review items returned.");
    }

    #[test]
    fn parse_runtime_error_prefers_error_envelope_and_truncates() {
        let error = parse_runtime_error(
            422,
            &json!({
                "error": {
                    "code": "invalid_tabular_request",
                    "message": "x".repeat(260)
                }
            })
            .to_string(),
        );

        assert!(error.starts_with("invalid_tabular_request: x"));
        assert!(error.ends_with('…'));
        assert!(error.chars().count() <= 240);
    }

    #[test]
    fn formatters_render_bounded_summary_and_review_queue() {
        let summary = RuntimeSummary {
            total_rows: 2,
            encoded_cells: 1,
            review_required_cells: 1,
            failed_rows: 0,
        };
        let review = vec![RuntimeReviewCandidate {
            row_index: 2,
            column: "patient_name".to_string(),
            value: "Alice Smith".to_string(),
            phi_type: "patient_name".to_string(),
        }];

        assert_eq!(
            format_summary(&summary),
            "total_rows: 2\nencoded_cells: 1\nreview_required_cells: 1\nfailed_rows: 0"
        );
        assert_eq!(
            format_review_queue(&review),
            "- row 2 / patient_name / patient_name: Alice Smith"
        );
    }

    #[test]
    fn runtime_failure_path_keeps_browser_honest() {
        let mut state = BrowserFlowState {
            payload: "patient_id\n1".to_string(),
            ..BrowserFlowState::default()
        };

        let request = state.begin_submit().unwrap();
        assert_eq!(state.summary, "Submitting to runtime...");
        assert!(state.is_submitting);
        assert_eq!(request.request.endpoint, "/tabular/deidentify");

        state.apply_runtime_error(
            request.submission_token,
            request.state_revision,
            FETCH_UNAVAILABLE_MESSAGE.to_string(),
        );

        assert_eq!(
            state.error_banner.as_deref(),
            Some(FETCH_UNAVAILABLE_MESSAGE)
        );
        assert_eq!(state.summary, IDLE_SUMMARY);
        assert_eq!(state.review_queue, IDLE_REVIEW_QUEUE);
        assert!(!state.is_submitting);
    }

    #[test]
    fn overlapping_submission_attempt_is_blocked_while_request_is_in_flight() {
        let mut state = BrowserFlowState {
            payload: "patient_id\n1".to_string(),
            ..BrowserFlowState::default()
        };

        let first = state.begin_submit().unwrap();
        let second = state.begin_submit();

        assert!(second.is_err());
        assert!(state.is_submitting);
        assert_eq!(state.active_submission_token, Some(first.submission_token));
    }

    #[test]
    fn editing_during_in_flight_request_invalidates_stale_response_without_clearing_spinner() {
        let mut state = BrowserFlowState {
            payload: "patient_id,patient_name\nMRN-001,Alice Smith".to_string(),
            result_output: "old-result".to_string(),
            summary: "old-summary".to_string(),
            review_queue: "old-review".to_string(),
            error_banner: Some("old-error".to_string()),
            ..BrowserFlowState::default()
        };

        let submission = state.begin_submit().unwrap();
        state.payload = "patient_id,patient_name\nMRN-002,Bob Jones".to_string();
        state.invalidate_generated_state();

        assert!(state.is_submitting);
        assert_eq!(state.summary, IDLE_SUMMARY);
        assert_eq!(state.review_queue, IDLE_REVIEW_QUEUE);
        assert!(state.error_banner.is_none());
        assert_eq!(state.state_revision, submission.state_revision + 1);

        let response = parse_runtime_success(
            InputMode::CsvText,
            &json!({
                "csv": "patient_id,patient_name\ntok-123,Alice Smith\n",
                "summary": {
                    "total_rows": 1,
                    "encoded_cells": 1,
                    "review_required_cells": 1,
                    "failed_rows": 0
                },
                "review_queue": [
                    {
                        "row_index": 1,
                        "column": "patient_name",
                        "value": "Alice Smith",
                        "phi_type": "patient_name"
                    }
                ]
            })
            .to_string(),
        )
        .unwrap();

        state.apply_runtime_success(
            submission.submission_token,
            submission.state_revision,
            response,
        );

        assert!(!state.is_submitting);
        assert!(state.result_output.is_empty());
        assert_eq!(state.summary, IDLE_SUMMARY);
        assert_eq!(state.review_queue, IDLE_REVIEW_QUEUE);
        assert!(state.error_banner.is_none());
        assert_eq!(state.payload, "patient_id,patient_name\nMRN-002,Bob Jones");
    }
}
