use axum::{
    extract::{rejection::JsonRejection, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use calamine::Reader;
use mdid_adapters::{
    ConservativeMediaInput, ConservativeMediaMetadataEntry, CsvTabularAdapter, DicomAdapterError,
    FieldPolicy, FieldPolicyAction, PdfAdapterError, XlsxTabularAdapter,
};
use mdid_application::{
    ApplicationError, ApplicationService, ConservativeMediaDeidentificationOutput,
    ConservativeMediaDeidentificationService, DicomDeidentificationOutput,
    DicomDeidentificationService, PdfDeidentificationOutput, PdfDeidentificationService,
    TabularDeidentificationOutput, TabularDeidentificationService,
};
use mdid_domain::{
    AuditEvent, AuditEventKind, BatchSummary, ConservativeMediaCandidate, ConservativeMediaFormat,
    ConservativeMediaSummary, DecodeRequest, DecodeRequestError, DicomDeidentificationSummary,
    DicomPhiCandidate, DicomPrivateTagPolicy, MappingRecord, MappingScope, PdfExtractionSummary,
    PdfPageRef, PdfPhiCandidate, PdfRewriteStatus, PdfScanStatus, PhiCandidate, SurfaceKind,
};
use mdid_vault::{LocalVaultStore, PortableVaultArtifact, VaultError};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::{
    collections::BTreeMap,
    io::{Cursor, Read, Write},
    path::PathBuf,
};
use tempfile::tempdir;
use xmltree::{Element, XMLNode};
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};

#[derive(Clone, Default)]
pub struct RuntimeState {
    pub application: ApplicationService,
}

#[derive(Debug, Deserialize)]
struct CreatePipelineRequest {
    name: String,
}

#[derive(Debug, Deserialize)]
struct DicomDeidentifyRequest {
    dicom_bytes_base64: String,
    source_name: String,
    private_tag_policy: DicomPrivateTagPolicy,
}

#[derive(Deserialize)]
struct PdfDeidentifyRequest {
    pdf_bytes_base64: String,
    source_name: String,
}

#[derive(Debug, Deserialize)]
struct VaultDecodeRequest {
    vault_path: PathBuf,
    vault_passphrase: String,
    record_ids: Vec<uuid::Uuid>,
    output_target: String,
    justification: String,
    requested_by: SurfaceKind,
}

#[derive(Debug, Deserialize)]
struct VaultExportRequest {
    vault_path: PathBuf,
    vault_passphrase: String,
    record_ids: Vec<uuid::Uuid>,
    export_passphrase: String,
    context: String,
    requested_by: SurfaceKind,
}

#[derive(Debug, Deserialize)]
struct VaultAuditEventsRequest {
    vault_path: PathBuf,
    vault_passphrase: String,
    kind: Option<AuditEventKind>,
    actor: Option<SurfaceKind>,
    #[serde(default, deserialize_with = "deserialize_optional_limit")]
    limit: Option<usize>,
    #[serde(default)]
    offset: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct PortableArtifactInspectionRequest {
    artifact: PortableVaultArtifact,
    portable_passphrase: String,
}

#[derive(Debug, Deserialize)]
struct PortableArtifactImportRequest {
    vault_path: PathBuf,
    vault_passphrase: String,
    artifact: PortableVaultArtifact,
    portable_passphrase: String,
    context: String,
    requested_by: SurfaceKind,
}

#[derive(Debug, Deserialize)]
struct TabularDeidentifyRequest {
    csv: String,
    policies: Vec<FieldPolicyRequest>,
}

#[derive(Debug, Deserialize)]
struct TabularXlsxDeidentifyRequest {
    workbook_base64: String,
    field_policies: Vec<FieldPolicyRequest>,
}

#[derive(Deserialize)]
struct ConservativeMediaDeidentifyRequest {
    artifact_label: String,
    format: ConservativeMediaFormat,
    metadata: Vec<ConservativeMediaMetadataEntryRequest>,
    #[serde(default)]
    ocr_or_visual_review_required: bool,
    #[serde(default)]
    unsupported_payload: bool,
    #[serde(
        default,
        rename = "media_bytes_base64",
        deserialize_with = "deserialize_field_presence"
    )]
    media_bytes_base64_present: bool,
    #[serde(
        default,
        rename = "image_bytes",
        deserialize_with = "deserialize_field_presence"
    )]
    image_bytes_present: bool,
    #[serde(
        default,
        rename = "file_bytes",
        deserialize_with = "deserialize_field_presence"
    )]
    file_bytes_present: bool,
    #[serde(
        default,
        rename = "base64",
        deserialize_with = "deserialize_field_presence"
    )]
    base64_present: bool,
}

impl ConservativeMediaDeidentifyRequest {
    fn contains_media_byte_payload(&self) -> bool {
        self.media_bytes_base64_present
            || self.image_bytes_present
            || self.file_bytes_present
            || self.base64_present
    }
}

#[derive(Debug, Deserialize)]
struct PrivacyFilterSummaryRequest {
    report: Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PrivacyFilterTextRequest {
    text: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OcrHandoffSummaryRequest {
    handoff: Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OcrToPrivacyFilterSummaryRequest {
    handoff: Value,
}

const PRIVACY_FILTER_TEXT_MAX_BYTES: usize = 1_048_576;
const INVALID_PRIVACY_FILTER_TEXT_REQUEST_MESSAGE: &str =
    "Privacy Filter text request requires non-empty text no larger than 1048576 bytes.";

fn deserialize_field_presence<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let _ = serde_json::Value::deserialize(deserializer)?;
    Ok(true)
}

#[derive(Deserialize)]
struct ConservativeMediaMetadataEntryRequest {
    key: String,
    value: String,
}

#[derive(Debug, Deserialize)]
struct FieldPolicyRequest {
    header: String,
    phi_type: String,
    action: FieldPolicyActionRequest,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum FieldPolicyActionRequest {
    Encode,
    Review,
    Ignore,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Debug, Serialize)]
struct DicomDeidentifyResponse {
    sanitized_file_name: String,
    rewritten_dicom_bytes_base64: String,
    summary: DicomDeidentificationSummary,
    review_queue: Vec<DicomPhiCandidate>,
}

#[derive(Debug, Serialize)]
struct PdfDeidentifyResponse {
    summary: PdfExtractionSummary,
    page_statuses: Vec<PdfPageStatusResponse>,
    review_queue: Vec<PdfPhiCandidate>,
    rewrite_status: PdfRewriteStatus,
    no_rewritten_pdf: bool,
    review_only: bool,
    rewritten_pdf_bytes_base64: Option<String>,
}

#[derive(Debug, Serialize)]
struct PdfPageStatusResponse {
    page: PdfPageRef,
    status: PdfScanStatus,
}

#[derive(Debug, Serialize)]
struct VaultDecodeResponse {
    values: Vec<mdid_domain::DecodedValue>,
    audit_event: mdid_domain::AuditEvent,
}

#[derive(Debug, Serialize)]
struct VaultExportResponse {
    artifact: mdid_vault::PortableVaultArtifact,
}

#[derive(Debug, Serialize)]
struct VaultAuditEventsResponse {
    events: Vec<AuditEvent>,
    limit: usize,
    offset: usize,
    total_matching_events: usize,
    next_offset: Option<usize>,
    has_more: bool,
}

#[derive(Debug, Serialize)]
struct PortableArtifactInspectionResponse {
    record_count: usize,
    records: Vec<PortableArtifactInspectionRecordPreview>,
}

#[derive(Debug, Serialize)]
struct PortableArtifactImportResponse {
    imported_record_count: usize,
    duplicate_record_count: usize,
    audit_event: AuditEvent,
}

#[derive(Debug, Serialize)]
struct PortableArtifactInspectionRecordPreview {
    id: uuid::Uuid,
    scope: MappingScope,
    phi_type: String,
    token: String,
    original_value: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
struct TabularDeidentifyResponse {
    csv: String,
    summary: BatchSummary,
    review_queue: Vec<PhiCandidate>,
}

#[derive(Debug, Serialize)]
struct TabularXlsxDeidentifyResponse {
    rewritten_workbook_base64: String,
    summary: BatchSummary,
    review_queue: Vec<PhiCandidate>,
    worksheet_disclosure: Option<XlsxSheetDisclosureResponse>,
}

#[derive(Debug, Serialize)]
struct XlsxSheetDisclosureResponse {
    selected_sheet_name: String,
    selected_sheet_index: usize,
    total_sheet_count: usize,
    disclosure: &'static str,
}

#[derive(Debug, Serialize)]
struct ConservativeMediaDeidentifyResponse {
    summary: ConservativeMediaSummary,
    review_queue: Vec<ConservativeMediaCandidate>,
    rewritten_media_bytes_base64: Option<String>,
}

#[derive(Debug, Serialize)]
struct PrivacyFilterSummaryResponse {
    artifact: &'static str,
    mode: String,
    engine: String,
    network_api_called: bool,
    preview_policy: String,
    input_char_count: u64,
    detected_span_count: u64,
    category_counts: BTreeMap<String, u64>,
    non_goals: Vec<String>,
}

#[derive(Debug, Serialize)]
struct OcrHandoffSummaryResponse {
    artifact: &'static str,
    candidate: String,
    engine: String,
    engine_status: String,
    scope: &'static str,
    privacy_filter_contract: &'static str,
    ready_for_text_pii_eval: bool,
    network_api_called: bool,
    non_goals: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    line_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    char_count: Option<u64>,
}

#[derive(Debug, Serialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    code: &'static str,
    message: &'static str,
}

pub fn build_router(state: RuntimeState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/pipelines", post(create_pipeline))
        .route("/ocr-handoff/summary", post(ocr_handoff_summary))
        .route(
            "/ocr-to-privacy-filter/summary",
            post(ocr_to_privacy_filter_summary),
        )
        .route("/privacy-filter/summary", post(privacy_filter_summary))
        .route("/privacy-filter/text", post(privacy_filter_text))
        .route("/tabular/deidentify", post(tabular_deidentify))
        .route("/tabular/deidentify/xlsx", post(tabular_xlsx_deidentify))
        .route(
            "/media/conservative/deidentify",
            post(conservative_media_deidentify),
        )
        .route("/pdf/deidentify", post(pdf_deidentify))
        .route("/dicom/deidentify", post(dicom_deidentify))
        .route("/vault/decode", post(vault_decode))
        .route("/vault/export", post(vault_export))
        .route("/vault/audit/events", post(vault_audit_events))
        .route(
            "/portable-artifacts/inspect",
            post(portable_artifact_inspect),
        )
        .route("/portable-artifacts/import", post(portable_artifact_import))
        .with_state(state)
}

pub fn build_default_router() -> Router {
    build_router(RuntimeState::default())
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, Json(HealthResponse { status: "ok" }))
}

async fn create_pipeline(
    State(state): State<RuntimeState>,
    Json(payload): Json<CreatePipelineRequest>,
) -> impl IntoResponse {
    let pipeline = state.application.register_pipeline(payload.name);
    (StatusCode::CREATED, Json(pipeline))
}

async fn ocr_handoff_summary(
    payload: Result<Json<OcrHandoffSummaryRequest>, JsonRejection>,
) -> Response {
    let Json(payload) = match payload {
        Ok(payload) => payload,
        Err(_) => return invalid_ocr_handoff_summary_request_response().into_response(),
    };

    match build_ocr_handoff_summary(&payload.handoff) {
        Some(summary) => (StatusCode::OK, Json(summary)).into_response(),
        None => invalid_ocr_handoff_summary_request_response().into_response(),
    }
}

async fn ocr_to_privacy_filter_summary(
    payload: Result<Json<OcrToPrivacyFilterSummaryRequest>, JsonRejection>,
) -> Response {
    let Json(payload) = match payload {
        Ok(payload) => payload,
        Err(_) => return invalid_ocr_to_privacy_filter_summary_request_response().into_response(),
    };

    if build_ocr_handoff_summary(&payload.handoff).is_none() {
        return invalid_ocr_to_privacy_filter_summary_request_response().into_response();
    }

    let normalized_text = match payload
        .handoff
        .get("normalized_text")
        .and_then(Value::as_str)
    {
        Some(text) if !text.trim().is_empty() && text.len() <= PRIVACY_FILTER_TEXT_MAX_BYTES => {
            text
        }
        _ => return invalid_ocr_to_privacy_filter_summary_request_response().into_response(),
    };

    let report = build_runtime_privacy_filter_text_report(normalized_text);
    match build_privacy_filter_summary(&report) {
        Some(summary) => (StatusCode::OK, Json(summary)).into_response(),
        None => internal_error_response().into_response(),
    }
}

fn build_ocr_handoff_summary(handoff: &Value) -> Option<OcrHandoffSummaryResponse> {
    let handoff = handoff.as_object()?;
    if contains_incompatible_ocr_handoff_marker(handoff) {
        return None;
    }

    if handoff.get("ready_for_text_pii_eval")?.as_bool()? != true {
        return None;
    }
    if handoff.get("privacy_filter_contract")?.as_str()? != "text_only_normalized_input" {
        return None;
    }
    if handoff.get("scope")?.as_str()? != "printed_text_line_extraction_only" {
        return None;
    }

    let candidate = safe_identifier(handoff.get("candidate")?.as_str()?)?.to_owned();
    let engine = safe_identifier(handoff.get("engine")?.as_str()?)?.to_owned();
    let engine_status = safe_identifier(handoff.get("engine_status")?.as_str()?)?.to_owned();
    let input_non_goals = handoff.get("non_goals")?.as_array()?;
    if !input_non_goals.iter().all(|non_goal| {
        non_goal
            .as_str()
            .and_then(safe_ocr_handoff_non_goal)
            .is_some()
    }) {
        return None;
    }

    let line_count = optional_u64(handoff, "line_count")?;
    let char_count = optional_u64(handoff, "char_count")?;

    Some(OcrHandoffSummaryResponse {
        artifact: "ocr_handoff_summary",
        candidate,
        engine,
        engine_status,
        scope: "printed_text_line_extraction_only",
        privacy_filter_contract: "text_only_normalized_input",
        ready_for_text_pii_eval: true,
        network_api_called: false,
        non_goals: vec![
            "visual_redaction".to_owned(),
            "image_pixel_redaction".to_owned(),
            "final_pdf_rewrite_export".to_owned(),
        ],
        line_count,
        char_count,
    })
}

fn optional_u64(report: &Map<String, Value>, field: &str) -> Option<Option<u64>> {
    match report.get(field) {
        Some(value) => value.as_u64().map(Some),
        None => Some(None),
    }
}

fn contains_incompatible_ocr_handoff_marker(report: &Map<String, Value>) -> bool {
    contains_incompatible_ocr_handoff_value(&Value::Object(report.clone()), true)
}

fn contains_incompatible_ocr_handoff_value(value: &Value, is_top_level: bool) -> bool {
    match value {
        Value::Object(object) => object.iter().any(|(key, value)| {
            is_incompatible_ocr_handoff_field(key, value, is_top_level)
                || contains_incompatible_ocr_handoff_value(value, false)
        }),
        Value::Array(values) => values
            .iter()
            .any(|value| contains_incompatible_ocr_handoff_value(value, false)),
        _ => false,
    }
}

fn is_incompatible_ocr_handoff_field(key: &str, value: &Value, is_top_level: bool) -> bool {
    const INCOMPATIBLE_MARKERS: &[&str] = &[
        "image_bytes",
        "image_bytes_base64",
        "masked_text",
        "spans",
        "preview",
        "previews",
        "bbox",
        "image",
        "pdf_rewrite",
        "pdf_export",
        "visual_redaction",
        "visual_redaction_result",
        "pixel_redaction",
        "agent_id",
        "controller_step",
        "complete_command",
        "claim",
        "raw_text",
        "text",
        "ocr_output",
        "path",
        "file_path",
    ];

    (key == "network_api_called" && value.as_bool() == Some(true))
        || INCOMPATIBLE_MARKERS.contains(&key)
        || (!is_top_level && matches!(key, "extracted_text" | "normalized_text" | "source"))
}

fn safe_ocr_handoff_non_goal(non_goal: &str) -> Option<&str> {
    (!contains_phi_sentinel(non_goal)
        && matches!(
            non_goal,
            "visual_redaction"
                | "image_pixel_redaction"
                | "final_pdf_rewrite_export"
                | "handwriting_recognition"
                | "full_page_detection_or_segmentation"
                | "complete_ocr_pipeline"
        ))
    .then_some(non_goal)
}

async fn privacy_filter_summary(
    payload: Result<Json<PrivacyFilterSummaryRequest>, JsonRejection>,
) -> Response {
    let Json(payload) = match payload {
        Ok(payload) => payload,
        Err(_) => return invalid_privacy_filter_summary_request_response().into_response(),
    };

    match build_privacy_filter_summary(&payload.report) {
        Some(summary) => (StatusCode::OK, Json(summary)).into_response(),
        None => invalid_privacy_filter_summary_request_response().into_response(),
    }
}

async fn privacy_filter_text(
    payload: Result<Json<PrivacyFilterTextRequest>, JsonRejection>,
) -> Response {
    let Json(payload) = match payload {
        Ok(payload) => payload,
        Err(_) => return invalid_privacy_filter_text_request_response().into_response(),
    };

    if payload.text.trim().is_empty() || payload.text.len() > PRIVACY_FILTER_TEXT_MAX_BYTES {
        return invalid_privacy_filter_text_request_response().into_response();
    }

    let report = build_runtime_privacy_filter_text_report(&payload.text);
    match build_privacy_filter_summary(&report) {
        Some(summary) => (StatusCode::OK, Json(summary)).into_response(),
        None => internal_error_response().into_response(),
    }
}

fn build_runtime_privacy_filter_text_report(text: &str) -> Value {
    let category_counts = [
        ("NAME", count_literal(text, "Patient Jane Example")),
        ("MRN", count_literal(text, "MRN-12345")),
        ("EMAIL", count_literal(text, "jane@example.com")),
        ("PHONE", count_literal(text, "555-123-4567")),
        ("ID", count_literal(text, "A1234567")),
    ]
    .into_iter()
    .filter(|(_, count)| *count > 0)
    .map(|(category, count)| (category.to_owned(), count))
    .collect::<BTreeMap<_, _>>();
    let detected_span_count = category_counts.values().sum::<u64>();

    json!({
        "artifact": "privacy_filter_report",
        "mode": "text",
        "summary": {
            "input_char_count": text.chars().count() as u64,
            "detected_span_count": detected_span_count,
            "category_counts": category_counts,
        },
        "metadata": {
            "engine": "runtime_text_mock",
            "network_api_called": false,
            "preview_policy": "redacted_placeholders_only"
        },
        "non_goals": [
            "text-only candidate",
            "No OCR",
            "No visual redaction",
            "No image pixel redaction",
            "No handwriting recognition",
            "No PDF rewrite/export",
            "browser_ui",
            "desktop_ui"
        ]
    })
}

fn count_literal(text: &str, needle: &str) -> u64 {
    text.match_indices(needle).count() as u64
}

fn build_privacy_filter_summary(report: &Value) -> Option<PrivacyFilterSummaryResponse> {
    let report = report.as_object()?;
    if contains_incompatible_privacy_filter_marker(report) {
        return None;
    }
    if contains_true_network_api_called(report) {
        return None;
    }
    if let Some(artifact) = report.get("artifact") {
        let artifact = artifact.as_str()?;
        if artifact != "privacy_filter_report" {
            return None;
        }
    }

    let summary = report
        .get("summary")
        .and_then(Value::as_object)
        .unwrap_or(report);
    let metadata = report
        .get("metadata")
        .and_then(Value::as_object)
        .unwrap_or(report);

    let input_char_count = required_u64(summary, "input_char_count")?;
    let detected_span_count = required_u64(summary, "detected_span_count")?;
    let category_counts = extract_category_counts(summary)?;
    let non_goals = extract_non_goals(report)?;
    let network_api_called = metadata.get("network_api_called")?.as_bool()?;
    if network_api_called {
        return None;
    }

    Some(PrivacyFilterSummaryResponse {
        artifact: "privacy_filter_summary",
        mode: safe_mode(report.get("mode").and_then(Value::as_str).unwrap_or("text"))?.to_owned(),
        engine: safe_identifier(metadata.get("engine")?.as_str()?)?.to_owned(),
        network_api_called,
        preview_policy: safe_preview_policy(metadata.get("preview_policy")?.as_str()?)?.to_owned(),
        input_char_count,
        detected_span_count,
        category_counts,
        non_goals,
    })
}

fn contains_incompatible_privacy_filter_marker(report: &Map<String, Value>) -> bool {
    const INCOMPATIBLE_MARKERS: &[&str] = &[
        "ocr_output",
        "image_bytes",
        "visual_redaction",
        "pixel_redaction",
        "pdf_rewrite",
        "pdf_export",
        "agent_id",
        "controller_step",
        "complete_command",
        "claim",
    ];

    report.iter().any(|(key, value)| {
        INCOMPATIBLE_MARKERS.contains(&key.as_str())
            || match value {
                Value::Object(object) => contains_incompatible_privacy_filter_marker(object),
                Value::Array(values) => values.iter().any(|value| match value {
                    Value::Object(object) => contains_incompatible_privacy_filter_marker(object),
                    _ => false,
                }),
                _ => false,
            }
    })
}

fn contains_true_network_api_called(report: &Map<String, Value>) -> bool {
    report.iter().any(|(key, value)| {
        (key == "network_api_called" && value.as_bool() == Some(true))
            || match value {
                Value::Object(object) => contains_true_network_api_called(object),
                Value::Array(values) => values.iter().any(|value| match value {
                    Value::Object(object) => contains_true_network_api_called(object),
                    _ => false,
                }),
                _ => false,
            }
    })
}

fn required_u64(report: &Map<String, Value>, field: &str) -> Option<u64> {
    report.get(field)?.as_u64()
}

fn extract_category_counts(report: &Map<String, Value>) -> Option<BTreeMap<String, u64>> {
    report
        .get("category_counts")?
        .as_object()?
        .iter()
        .map(|(category, count)| {
            Some((
                safe_category_identifier(category)?.to_owned(),
                count.as_u64()?,
            ))
        })
        .collect::<Option<BTreeMap<String, u64>>>()
}

fn extract_non_goals(report: &Map<String, Value>) -> Option<Vec<String>> {
    match report.get("non_goals") {
        Some(non_goals) => non_goals
            .as_array()?
            .iter()
            .map(|non_goal| safe_non_goal(non_goal.as_str()?).map(ToOwned::to_owned))
            .collect::<Option<Vec<_>>>(),
        None => Some(default_privacy_filter_non_goals()),
    }
}

fn default_privacy_filter_non_goals() -> Vec<String> {
    vec![
        "No OCR".to_owned(),
        "No image pixel redaction".to_owned(),
        "No PDF rewrite/export".to_owned(),
    ]
}

fn safe_mode(mode: &str) -> Option<&str> {
    matches!(mode, "text" | "mock" | "summary_only").then_some(mode)
}

fn safe_preview_policy(preview_policy: &str) -> Option<&str> {
    matches!(
        preview_policy,
        "redacted_preview_only" | "masked-only" | "masked_only" | "redacted_placeholders_only"
    )
    .then_some(preview_policy)
}

fn safe_identifier(identifier: &str) -> Option<&str> {
    (!identifier.is_empty()
        && identifier.len() <= 128
        && !contains_phi_sentinel(identifier)
        && identifier
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-')))
    .then_some(identifier)
}

fn safe_category_identifier(category: &str) -> Option<&str> {
    matches!(category, "NAME" | "MRN" | "EMAIL" | "PHONE" | "ID").then_some(category)
}

fn safe_non_goal(non_goal: &str) -> Option<&str> {
    (!contains_phi_sentinel(non_goal)
        && matches!(
            non_goal,
            "text-only candidate"
                | "No OCR"
                | "ocr"
                | "No visual redaction"
                | "visual_redaction"
                | "No image pixel redaction"
                | "image_pixel_redaction"
                | "No handwriting recognition"
                | "handwriting_recognition"
                | "No PDF rewrite/export"
                | "final_pdf_rewrite_export"
                | "No network call unless explicitly configured"
                | "browser_ui"
                | "desktop_ui"
        ))
    .then_some(non_goal)
}

fn contains_phi_sentinel(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("patient jane example")
        || lower.contains("mrn-")
        || lower.contains("jane@example.com")
        || lower.contains("555-123-4567")
        || lower.contains("555-")
        || lower.contains("alice smith")
}

async fn tabular_deidentify(
    payload: Result<Json<TabularDeidentifyRequest>, JsonRejection>,
) -> Response {
    let Json(payload) = match payload {
        Ok(payload) => payload,
        Err(_) => return invalid_tabular_request_response().into_response(),
    };

    let temp_dir = match tempdir() {
        Ok(dir) => dir,
        Err(_) => return internal_error_response().into_response(),
    };
    let vault_path = temp_dir.path().join("runtime-tabular-vault.mdid");
    let mut vault = match LocalVaultStore::create(&vault_path, "correct horse battery staple") {
        Ok(vault) => vault,
        Err(_) => return internal_error_response().into_response(),
    };

    let policies = payload
        .policies
        .into_iter()
        .map(FieldPolicy::from)
        .collect::<Vec<_>>();

    let output = match TabularDeidentificationService.deidentify_csv(
        &payload.csv,
        &policies,
        &mut vault,
        SurfaceKind::Browser,
    ) {
        Ok(output) => output,
        Err(error) => return map_application_error(&error).into_response(),
    };

    tabular_success_response(output).into_response()
}

async fn tabular_xlsx_deidentify(
    payload: Result<Json<TabularXlsxDeidentifyRequest>, JsonRejection>,
) -> Response {
    let Json(payload) = match payload {
        Ok(payload) => payload,
        Err(_) => return invalid_tabular_xlsx_request_response().into_response(),
    };

    let workbook_bytes = match STANDARD.decode(&payload.workbook_base64) {
        Ok(bytes) => bytes,
        Err(_) => return invalid_tabular_xlsx_request_response().into_response(),
    };

    let temp_dir = match tempdir() {
        Ok(dir) => dir,
        Err(_) => return internal_error_response().into_response(),
    };
    let vault_path = temp_dir.path().join("runtime-tabular-xlsx-vault.mdid");
    let mut vault = match LocalVaultStore::create(&vault_path, "correct horse battery staple") {
        Ok(vault) => vault,
        Err(_) => return internal_error_response().into_response(),
    };

    let policies = payload
        .field_policies
        .into_iter()
        .map(FieldPolicy::from)
        .collect::<Vec<_>>();

    let extracted = match XlsxTabularAdapter::new(policies).extract(&workbook_bytes) {
        Ok(extracted) => extracted,
        Err(_) => return invalid_tabular_xlsx_request_response().into_response(),
    };

    let output = match TabularDeidentificationService.deidentify_extracted(
        extracted,
        &mut vault,
        SurfaceKind::Browser,
    ) {
        Ok(output) => output,
        Err(error) => return map_tabular_xlsx_application_error(&error).into_response(),
    };

    match tabular_xlsx_success_response(&workbook_bytes, output) {
        Ok(response) => response.into_response(),
        Err(_) => internal_error_response().into_response(),
    }
}

async fn conservative_media_deidentify(
    payload: Result<Json<ConservativeMediaDeidentifyRequest>, JsonRejection>,
) -> Response {
    let Json(payload) = match payload {
        Ok(payload) => payload,
        Err(_) => return invalid_conservative_media_request_response().into_response(),
    };

    if payload.contains_media_byte_payload() {
        return conservative_media_bytes_not_accepted_response().into_response();
    }

    let input = ConservativeMediaInput::from(payload);
    let output =
        match ConservativeMediaDeidentificationService::default().deidentify_metadata(input) {
            Ok(output) => output,
            Err(ApplicationError::ConservativeMediaAdapter(_)) => {
                return invalid_conservative_media_request_response().into_response()
            }
            Err(_) => return internal_error_response().into_response(),
        };

    conservative_media_success_response(output).into_response()
}

async fn dicom_deidentify(Json(payload): Json<DicomDeidentifyRequest>) -> Response {
    let dicom_bytes = match STANDARD.decode(&payload.dicom_bytes_base64) {
        Ok(bytes) => bytes,
        Err(_) => return invalid_dicom_response().into_response(),
    };

    let temp_dir = match tempdir() {
        Ok(dir) => dir,
        Err(_) => return internal_error_response().into_response(),
    };
    let vault_path = temp_dir.path().join("runtime-dicom-vault.mdid");
    let mut vault = match LocalVaultStore::create(&vault_path, "correct horse battery staple") {
        Ok(vault) => vault,
        Err(_) => return internal_error_response().into_response(),
    };

    let output = match DicomDeidentificationService.deidentify_bytes(
        &dicom_bytes,
        &payload.source_name,
        payload.private_tag_policy,
        &mut vault,
        SurfaceKind::Browser,
    ) {
        Ok(output) => output,
        Err(error) => return map_application_error(&error).into_response(),
    };

    success_response(output).into_response()
}

async fn pdf_deidentify(payload: Result<Json<PdfDeidentifyRequest>, JsonRejection>) -> Response {
    let Json(payload) = match payload {
        Ok(payload) => payload,
        Err(_) => return invalid_pdf_response().into_response(),
    };

    let pdf_bytes = match STANDARD.decode(&payload.pdf_bytes_base64) {
        Ok(bytes) => bytes,
        Err(_) => return invalid_pdf_response().into_response(),
    };

    let output = match PdfDeidentificationService.deidentify_bytes(&pdf_bytes, &payload.source_name)
    {
        Ok(output) => output,
        Err(error) => return map_pdf_application_error(&error).into_response(),
    };

    pdf_success_response(output).into_response()
}

async fn vault_decode(payload: Result<Json<VaultDecodeRequest>, JsonRejection>) -> Response {
    let Json(payload) = match payload {
        Ok(payload) => payload,
        Err(_) => return invalid_decode_request_response().into_response(),
    };

    let request = match DecodeRequest::new(
        payload.record_ids,
        payload.output_target,
        payload.justification,
        payload.requested_by,
    ) {
        Ok(request) => request,
        Err(DecodeRequestError::DuplicateRecordId) => {
            return duplicate_record_id_response().into_response();
        }
        Err(_) => return invalid_decode_request_response().into_response(),
    };

    let mut vault = match LocalVaultStore::unlock(&payload.vault_path, &payload.vault_passphrase) {
        Ok(vault) => vault,
        Err(error) => return map_vault_error(&error).into_response(),
    };

    match vault.decode(request) {
        Ok(decoded) => (
            StatusCode::OK,
            Json(VaultDecodeResponse {
                values: decoded.values,
                audit_event: decoded.audit_event,
            }),
        )
            .into_response(),
        Err(error) => map_vault_error(&error).into_response(),
    }
}

async fn vault_export(payload: Result<Json<VaultExportRequest>, JsonRejection>) -> Response {
    let Json(payload) = match payload {
        Ok(payload) => payload,
        Err(_) => return invalid_export_request_response().into_response(),
    };

    if has_duplicate_record_id(&payload.record_ids) {
        return duplicate_record_id_response().into_response();
    }

    let mut vault = match LocalVaultStore::unlock(&payload.vault_path, &payload.vault_passphrase) {
        Ok(vault) => vault,
        Err(error) => return map_export_vault_error(&error).into_response(),
    };

    match vault.export_portable(
        &payload.record_ids,
        &payload.export_passphrase,
        payload.requested_by,
        &payload.context,
    ) {
        Ok(artifact) => (StatusCode::OK, Json(VaultExportResponse { artifact })).into_response(),
        Err(error) => map_export_vault_error(&error).into_response(),
    }
}

async fn vault_audit_events(
    payload: Result<Json<VaultAuditEventsRequest>, JsonRejection>,
) -> Response {
    let Json(payload) = match payload {
        Ok(payload) => payload,
        Err(_) => return invalid_audit_events_request_response().into_response(),
    };

    let vault = match LocalVaultStore::unlock(&payload.vault_path, &payload.vault_passphrase) {
        Ok(vault) => vault,
        Err(error) => return map_audit_events_vault_error(&error).into_response(),
    };

    let limit = payload.limit.unwrap_or(100).min(100);
    let offset = payload.offset.unwrap_or(0);
    let filtered = vault
        .audit_events()
        .iter()
        .rev()
        .filter(|event| payload.kind.is_none_or(|kind| event.kind == kind))
        .filter(|event| payload.actor.is_none_or(|actor| event.actor == actor))
        .cloned()
        .collect::<Vec<_>>();
    let total_matching_events = filtered.len();
    let mut filtered_events = filtered
        .into_iter()
        .skip(offset)
        .take(limit.saturating_add(1))
        .collect::<Vec<_>>();
    let has_more = filtered_events.len() > limit;
    if has_more {
        filtered_events.truncate(limit);
    }
    let returned_events = filtered_events.len();
    let next_offset = has_more.then_some(offset.saturating_add(returned_events));

    (
        StatusCode::OK,
        Json(VaultAuditEventsResponse {
            events: filtered_events,
            limit,
            offset,
            total_matching_events,
            next_offset,
            has_more,
        }),
    )
        .into_response()
}

async fn portable_artifact_inspect(
    payload: Result<Json<PortableArtifactInspectionRequest>, JsonRejection>,
) -> Response {
    let Json(payload) = match payload {
        Ok(payload) => payload,
        Err(_) => return invalid_portable_artifact_inspection_request_response().into_response(),
    };

    if payload.portable_passphrase.trim().is_empty() {
        return invalid_portable_artifact_inspection_request_response().into_response();
    }

    let snapshot = match payload.artifact.unlock(&payload.portable_passphrase) {
        Ok(snapshot) => snapshot,
        Err(error) => return map_portable_artifact_inspection_error(&error).into_response(),
    };

    let records = snapshot
        .records
        .into_iter()
        .map(PortableArtifactInspectionRecordPreview::from)
        .collect::<Vec<_>>();

    (
        StatusCode::OK,
        Json(PortableArtifactInspectionResponse {
            record_count: records.len(),
            records,
        }),
    )
        .into_response()
}

async fn portable_artifact_import(
    payload: Result<Json<PortableArtifactImportRequest>, JsonRejection>,
) -> Response {
    let Json(payload) = match payload {
        Ok(payload) => payload,
        Err(_) => return invalid_portable_artifact_import_request_response().into_response(),
    };

    if payload.vault_passphrase.trim().is_empty()
        || payload.portable_passphrase.trim().is_empty()
        || payload.context.trim().is_empty()
    {
        return invalid_portable_artifact_import_request_response().into_response();
    }

    let mut vault = match LocalVaultStore::unlock(&payload.vault_path, &payload.vault_passphrase) {
        Ok(vault) => vault,
        Err(error) => return map_portable_artifact_import_unlock_error(&error).into_response(),
    };

    match vault.import_portable(
        payload.artifact,
        &payload.portable_passphrase,
        payload.requested_by,
        &payload.context,
    ) {
        Ok(result) => (
            StatusCode::OK,
            Json(PortableArtifactImportResponse {
                imported_record_count: result.imported_records.len(),
                duplicate_record_count: result.duplicate_records.len(),
                audit_event: result.audit_event,
            }),
        )
            .into_response(),
        Err(error) => map_portable_artifact_import_vault_error(&error).into_response(),
    }
}

fn map_application_error(error: &ApplicationError) -> (StatusCode, Json<ErrorEnvelope>) {
    match error {
        ApplicationError::DicomAdapter(DicomAdapterError::Parse(_))
        | ApplicationError::DicomAdapter(DicomAdapterError::Value(_)) => invalid_dicom_response(),
        ApplicationError::TabularAdapter(_) => invalid_tabular_request_response(),
        _ => internal_error_response(),
    }
}

fn map_tabular_xlsx_application_error(
    error: &ApplicationError,
) -> (StatusCode, Json<ErrorEnvelope>) {
    match error {
        ApplicationError::TabularAdapter(_) => invalid_tabular_xlsx_request_response(),
        _ => internal_error_response(),
    }
}

fn map_pdf_application_error(error: &ApplicationError) -> (StatusCode, Json<ErrorEnvelope>) {
    match error {
        ApplicationError::PdfAdapter(PdfAdapterError::Parse(_)) => invalid_pdf_response(),
        _ => internal_error_response(),
    }
}

impl From<ConservativeMediaMetadataEntryRequest> for ConservativeMediaMetadataEntry {
    fn from(value: ConservativeMediaMetadataEntryRequest) -> Self {
        Self {
            key: value.key,
            value: value.value,
        }
    }
}

impl From<ConservativeMediaDeidentifyRequest> for ConservativeMediaInput {
    fn from(value: ConservativeMediaDeidentifyRequest) -> Self {
        Self {
            artifact_label: value.artifact_label,
            format: value.format,
            metadata: value.metadata.into_iter().map(Into::into).collect(),
            requires_visual_review: value.ocr_or_visual_review_required,
            unsupported_payload: value.unsupported_payload,
        }
    }
}

impl From<FieldPolicyActionRequest> for FieldPolicyAction {
    fn from(value: FieldPolicyActionRequest) -> Self {
        match value {
            FieldPolicyActionRequest::Encode => Self::Encode,
            FieldPolicyActionRequest::Review => Self::Review,
            FieldPolicyActionRequest::Ignore => Self::Ignore,
        }
    }
}

impl From<FieldPolicyRequest> for FieldPolicy {
    fn from(value: FieldPolicyRequest) -> Self {
        Self {
            header: value.header,
            phi_type: value.phi_type,
            action: value.action.into(),
        }
    }
}

impl From<MappingRecord> for PortableArtifactInspectionRecordPreview {
    fn from(value: MappingRecord) -> Self {
        Self {
            id: value.id,
            scope: value.scope,
            phi_type: value.phi_type,
            token: value.token,
            original_value: value.original_value,
            created_at: value.created_at,
        }
    }
}

fn map_vault_error(error: &VaultError) -> (StatusCode, Json<ErrorEnvelope>) {
    match error {
        VaultError::UnknownRecord(_) => unknown_record_response(),
        VaultError::UnlockFailed => vault_unlock_failed_response(),
        VaultError::BlankPassphrase
        | VaultError::EmptyExportScope
        | VaultError::DuplicateRecordId
        | VaultError::BlankExportContext
        | VaultError::BlankImportContext => invalid_decode_request_response(),
        VaultError::Io(_)
        | VaultError::Serde(_)
        | VaultError::UnsupportedKdfAlgorithm(_)
        | VaultError::UnsupportedKdfVersion(_)
        | VaultError::InvalidKdfParameters
        | VaultError::InvalidNonceLength { .. }
        | VaultError::KeyDerivation
        | VaultError::InvalidArtifact => invalid_vault_target_response(),
        VaultError::AlreadyExists(_) | VaultError::Encrypt => internal_error_response(),
    }
}

fn map_audit_events_vault_error(error: &VaultError) -> (StatusCode, Json<ErrorEnvelope>) {
    match error {
        VaultError::BlankPassphrase => invalid_audit_events_request_response(),
        _ => map_vault_error(error),
    }
}

fn map_export_vault_error(error: &VaultError) -> (StatusCode, Json<ErrorEnvelope>) {
    match error {
        VaultError::BlankPassphrase
        | VaultError::EmptyExportScope
        | VaultError::DuplicateRecordId
        | VaultError::BlankExportContext => invalid_export_request_response(),
        VaultError::UnknownRecord(_) => unknown_export_record_response(),
        _ => map_vault_error(error),
    }
}

fn map_portable_artifact_inspection_error(error: &VaultError) -> (StatusCode, Json<ErrorEnvelope>) {
    match error {
        VaultError::UnlockFailed => portable_artifact_unlock_failed_response(),
        VaultError::BlankPassphrase => invalid_portable_artifact_inspection_request_response(),
        VaultError::Io(_)
        | VaultError::Serde(_)
        | VaultError::UnsupportedKdfAlgorithm(_)
        | VaultError::UnsupportedKdfVersion(_)
        | VaultError::InvalidKdfParameters
        | VaultError::InvalidNonceLength { .. }
        | VaultError::KeyDerivation
        | VaultError::InvalidArtifact => invalid_portable_artifact_response(),
        VaultError::UnknownRecord(_)
        | VaultError::EmptyExportScope
        | VaultError::DuplicateRecordId
        | VaultError::BlankExportContext
        | VaultError::BlankImportContext
        | VaultError::AlreadyExists(_)
        | VaultError::Encrypt => internal_error_response(),
    }
}

fn map_portable_artifact_import_vault_error(
    error: &VaultError,
) -> (StatusCode, Json<ErrorEnvelope>) {
    match error {
        VaultError::BlankPassphrase | VaultError::BlankImportContext => {
            invalid_portable_artifact_import_request_response()
        }
        VaultError::UnlockFailed => portable_artifact_unlock_failed_response(),
        VaultError::Io(_)
        | VaultError::Serde(_)
        | VaultError::UnsupportedKdfAlgorithm(_)
        | VaultError::UnsupportedKdfVersion(_)
        | VaultError::InvalidKdfParameters
        | VaultError::InvalidNonceLength { .. }
        | VaultError::KeyDerivation
        | VaultError::InvalidArtifact => invalid_portable_artifact_response(),
        VaultError::UnknownRecord(_)
        | VaultError::EmptyExportScope
        | VaultError::DuplicateRecordId
        | VaultError::BlankExportContext
        | VaultError::AlreadyExists(_)
        | VaultError::Encrypt => internal_error_response(),
    }
}

fn map_portable_artifact_import_unlock_error(
    error: &VaultError,
) -> (StatusCode, Json<ErrorEnvelope>) {
    match error {
        VaultError::BlankPassphrase => invalid_portable_artifact_import_request_response(),
        VaultError::UnlockFailed => vault_unlock_failed_response(),
        VaultError::Io(_)
        | VaultError::Serde(_)
        | VaultError::UnsupportedKdfAlgorithm(_)
        | VaultError::UnsupportedKdfVersion(_)
        | VaultError::InvalidKdfParameters
        | VaultError::InvalidNonceLength { .. }
        | VaultError::KeyDerivation
        | VaultError::InvalidArtifact => invalid_vault_target_response(),
        VaultError::UnknownRecord(_)
        | VaultError::EmptyExportScope
        | VaultError::DuplicateRecordId
        | VaultError::BlankExportContext
        | VaultError::BlankImportContext
        | VaultError::AlreadyExists(_)
        | VaultError::Encrypt => internal_error_response(),
    }
}

fn success_response(
    output: DicomDeidentificationOutput,
) -> (StatusCode, Json<DicomDeidentifyResponse>) {
    (
        StatusCode::OK,
        Json(DicomDeidentifyResponse {
            sanitized_file_name: output.sanitized_file_name,
            rewritten_dicom_bytes_base64: STANDARD.encode(output.bytes),
            summary: output.summary,
            review_queue: output.review_queue,
        }),
    )
}

fn tabular_success_response(
    output: TabularDeidentificationOutput,
) -> (StatusCode, Json<TabularDeidentifyResponse>) {
    (
        StatusCode::OK,
        Json(TabularDeidentifyResponse {
            csv: output.csv,
            summary: output.summary,
            review_queue: output.review_queue,
        }),
    )
}

fn tabular_xlsx_success_response(
    original_workbook: &[u8],
    output: TabularDeidentificationOutput,
) -> Result<(StatusCode, Json<TabularXlsxDeidentifyResponse>), XlsxRewriteError> {
    let extracted = CsvTabularAdapter::new(Vec::new())
        .extract(output.csv.as_bytes())
        .expect("tabular application output should remain valid CSV");
    let headers = extracted
        .columns
        .iter()
        .map(|column| column.name.as_str())
        .collect::<Vec<_>>();
    let rows = extracted
        .rows
        .iter()
        .map(|row| row.iter().map(|value| value.as_str()).collect::<Vec<_>>())
        .collect::<Vec<_>>();

    Ok((
        StatusCode::OK,
        Json(TabularXlsxDeidentifyResponse {
            rewritten_workbook_base64: STANDARD.encode(rewrite_xlsx_workbook_bytes(
                original_workbook,
                &headers,
                &rows,
            )?),
            summary: output.summary,
            review_queue: output.review_queue,
            worksheet_disclosure: output.worksheet_disclosure.map(|disclosure| {
                XlsxSheetDisclosureResponse {
                    selected_sheet_name: disclosure.selected_sheet_name,
                    selected_sheet_index: disclosure.selected_sheet_index,
                    total_sheet_count: disclosure.total_sheet_count,
                    disclosure: disclosure.disclosure,
                }
            }),
        }),
    ))
}

fn conservative_media_success_response(
    output: ConservativeMediaDeidentificationOutput,
) -> (StatusCode, Json<ConservativeMediaDeidentifyResponse>) {
    (
        StatusCode::OK,
        Json(ConservativeMediaDeidentifyResponse {
            summary: output.summary,
            review_queue: output.review_queue,
            rewritten_media_bytes_base64: None,
        }),
    )
}

fn pdf_success_response(
    output: PdfDeidentificationOutput,
) -> (StatusCode, Json<PdfDeidentifyResponse>) {
    (
        StatusCode::OK,
        Json(PdfDeidentifyResponse {
            summary: output.summary,
            page_statuses: output
                .page_statuses
                .into_iter()
                .map(|page_status| PdfPageStatusResponse {
                    page: page_status.page,
                    status: page_status.status,
                })
                .collect(),
            review_queue: output.review_queue,
            rewrite_status: output.rewrite_status,
            no_rewritten_pdf: output.no_rewritten_pdf,
            review_only: output.review_only,
            rewritten_pdf_bytes_base64: output
                .rewritten_pdf_bytes
                .map(|bytes| STANDARD.encode(bytes)),
        }),
    )
}

fn invalid_dicom_response() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(ErrorEnvelope {
            error: ErrorBody {
                code: "invalid_dicom",
                message: "request body did not contain a valid DICOM payload",
            },
        }),
    )
}

fn invalid_pdf_response() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(ErrorEnvelope {
            error: ErrorBody {
                code: "invalid_pdf",
                message: "request body did not contain a valid PDF payload",
            },
        }),
    )
}

fn invalid_privacy_filter_summary_request_response() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorEnvelope {
            error: ErrorBody {
                code: "invalid_privacy_filter_summary_request",
                message: "request body did not contain a valid privacy filter report object",
            },
        }),
    )
}

fn invalid_ocr_handoff_summary_request_response() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorEnvelope {
            error: ErrorBody {
                code: "invalid_ocr_handoff_summary_request",
                message: "request body did not contain a valid OCR handoff report object",
            },
        }),
    )
}

fn invalid_ocr_to_privacy_filter_summary_request_response() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorEnvelope {
            error: ErrorBody {
                code: "invalid_ocr_to_privacy_filter_summary_request",
                message: "request body did not contain a valid OCR handoff report for text-only Privacy Filter summary",
            },
        }),
    )
}

fn invalid_privacy_filter_text_request_response() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorEnvelope {
            error: ErrorBody {
                code: "invalid_privacy_filter_text_request",
                message: INVALID_PRIVACY_FILTER_TEXT_REQUEST_MESSAGE,
            },
        }),
    )
}

fn invalid_tabular_request_response() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(ErrorEnvelope {
            error: ErrorBody {
                code: "invalid_tabular_request",
                message: "request body did not contain a valid tabular deidentification request",
            },
        }),
    )
}

fn invalid_tabular_xlsx_request_response() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(ErrorEnvelope {
            error: ErrorBody {
                code: "invalid_tabular_xlsx_request",
                message:
                    "request body did not contain a valid XLSX tabular deidentification request",
            },
        }),
    )
}

fn invalid_conservative_media_request_response() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(ErrorEnvelope {
            error: ErrorBody {
                code: "invalid_conservative_media_request",
                message: "request body did not contain a valid conservative media deidentification request",
            },
        }),
    )
}

fn conservative_media_bytes_not_accepted_response() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(ErrorEnvelope {
            error: ErrorBody {
                code: "invalid_conservative_media_request",
                message: "metadata-only media review does not accept media bytes",
            },
        }),
    )
}

fn invalid_decode_request_response() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(ErrorEnvelope {
            error: ErrorBody {
                code: "invalid_decode_request",
                message: "request body did not contain a valid vault decode request",
            },
        }),
    )
}

fn invalid_export_request_response() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(ErrorEnvelope {
            error: ErrorBody {
                code: "invalid_export_request",
                message: "request body did not contain a valid vault export request",
            },
        }),
    )
}

fn duplicate_record_id_response() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorEnvelope {
            error: ErrorBody {
                code: "duplicate_record_id",
                message: "duplicate record id is not allowed",
            },
        }),
    )
}

fn has_duplicate_record_id(record_ids: &[uuid::Uuid]) -> bool {
    let mut seen_record_ids = std::collections::HashSet::with_capacity(record_ids.len());
    record_ids
        .iter()
        .any(|record_id| !seen_record_ids.insert(*record_id))
}

fn invalid_audit_events_request_response() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorEnvelope {
            error: ErrorBody {
                code: "invalid_audit_events_request",
                message: "request body did not contain a valid vault audit events request",
            },
        }),
    )
}

fn invalid_portable_artifact_inspection_request_response() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(ErrorEnvelope {
            error: ErrorBody {
                code: "invalid_portable_artifact_inspection_request",
                message:
                    "request body did not contain a valid portable artifact inspection request",
            },
        }),
    )
}

fn invalid_portable_artifact_import_request_response() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(ErrorEnvelope {
            error: ErrorBody {
                code: "invalid_portable_artifact_import_request",
                message: "request body did not contain a valid portable artifact import request",
            },
        }),
    )
}

fn invalid_portable_artifact_response() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(ErrorEnvelope {
            error: ErrorBody {
                code: "invalid_portable_artifact",
                message: "portable artifact could not be read as a usable portable vault artifact",
            },
        }),
    )
}

fn invalid_vault_target_response() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(ErrorEnvelope {
            error: ErrorBody {
                code: "invalid_vault_target",
                message: "vault target could not be read as a usable vault artifact",
            },
        }),
    )
}

fn unknown_record_response() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorEnvelope {
            error: ErrorBody {
                code: "unknown_record",
                message: "decode scope referenced a record that does not exist",
            },
        }),
    )
}

fn unknown_export_record_response() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorEnvelope {
            error: ErrorBody {
                code: "unknown_record",
                message: "export scope referenced a record that does not exist",
            },
        }),
    )
}

fn vault_unlock_failed_response() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorEnvelope {
            error: ErrorBody {
                code: "vault_unlock_failed",
                message: "vault could not be unlocked with the supplied passphrase",
            },
        }),
    )
}

fn portable_artifact_unlock_failed_response() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorEnvelope {
            error: ErrorBody {
                code: "portable_artifact_unlock_failed",
                message: "portable artifact could not be unlocked with the supplied passphrase",
            },
        }),
    )
}

fn internal_error_response() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorEnvelope {
            error: ErrorBody {
                code: "internal_error",
                message: "internal server error",
            },
        }),
    )
}

#[derive(Debug, thiserror::Error)]
enum XlsxRewriteError {
    #[error("xlsx archive could not be opened: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("worksheet xml could not be parsed: {0}")]
    Xml(#[from] xmltree::ParseError),
    #[error("worksheet xml could not be written: {0}")]
    XmlWrite(#[from] xmltree::Error),
    #[error("xlsx archive I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("xlsx workbook metadata was missing {0}")]
    MissingPart(&'static str),
}

fn rewrite_xlsx_workbook_bytes(
    original_workbook: &[u8],
    headers: &[&str],
    rows: &[Vec<&str>],
) -> Result<Vec<u8>, XlsxRewriteError> {
    let worksheet_path = find_first_non_empty_worksheet_path(original_workbook)?;
    let original_sheet = XlsxTabularAdapter::new(Vec::new())
        .extract(original_workbook)
        .map_err(|_| XlsxRewriteError::MissingPart("worksheet range"))?;
    let original_headers = original_sheet
        .columns
        .iter()
        .map(|column| column.name.as_str())
        .collect::<Vec<_>>();
    let original_rows = original_sheet
        .rows
        .iter()
        .map(|row| row.iter().map(|value| value.as_str()).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let rewritten_sheet_xml = rewrite_sheet_xml(
        read_zip_entry(original_workbook, &worksheet_path)?.as_slice(),
        &original_headers,
        &original_rows,
        headers,
        rows,
    )?;

    let mut archive = ZipArchive::new(Cursor::new(original_workbook))?;
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));

    for index in 0..archive.len() {
        let mut file = archive.by_index(index)?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)?;
        let options = SimpleFileOptions::default().compression_method(file.compression());
        writer.start_file(file.name(), options)?;
        if file.name() == worksheet_path {
            writer.write_all(&rewritten_sheet_xml)?;
        } else {
            writer.write_all(&contents)?;
        }
    }

    Ok(writer.finish()?.into_inner())
}

fn find_first_non_empty_worksheet_path(workbook_bytes: &[u8]) -> Result<String, XlsxRewriteError> {
    let workbook_xml =
        Element::parse(read_zip_entry(workbook_bytes, "xl/workbook.xml")?.as_slice())?;
    let workbook_rels =
        Element::parse(read_zip_entry(workbook_bytes, "xl/_rels/workbook.xml.rels")?.as_slice())?;

    let ordered_sheets = workbook_xml
        .get_child("sheets")
        .ok_or(XlsxRewriteError::MissingPart("sheets"))?
        .children
        .iter()
        .filter_map(|node| match node {
            XMLNode::Element(sheet) if sheet.name == "sheet" => Some(sheet),
            _ => None,
        })
        .map(|sheet| {
            Ok((
                sheet
                    .attributes
                    .get("name")
                    .cloned()
                    .ok_or(XlsxRewriteError::MissingPart("sheet name"))?,
                sheet
                    .attributes
                    .iter()
                    .find(|(key, _)| key.ends_with("id"))
                    .map(|(_, value)| value.clone())
                    .ok_or(XlsxRewriteError::MissingPart("sheet relationship id"))?,
            ))
        })
        .collect::<Result<Vec<_>, XlsxRewriteError>>()?;

    let sheet_name = select_first_non_empty_sheet_name(workbook_bytes)?;
    let relationship_id = ordered_sheets
        .iter()
        .find(|(name, _)| name == &sheet_name)
        .map(|(_, relationship_id)| relationship_id.as_str())
        .ok_or(XlsxRewriteError::MissingPart(
            "selected worksheet relationship",
        ))?;

    let target = workbook_rels
        .children
        .iter()
        .filter_map(|node| match node {
            XMLNode::Element(relationship) if relationship.name == "Relationship" => {
                Some(relationship)
            }
            _ => None,
        })
        .find(|relationship| {
            relationship.attributes.get("Id").map(|id| id.as_str()) == Some(relationship_id)
        })
        .and_then(|relationship| relationship.attributes.get("Target"))
        .cloned()
        .ok_or(XlsxRewriteError::MissingPart("worksheet target"))?;

    Ok(normalize_workbook_target(&target))
}

fn select_first_non_empty_sheet_name(workbook_bytes: &[u8]) -> Result<String, XlsxRewriteError> {
    let mut workbook =
        calamine::open_workbook_from_rs::<calamine::Xlsx<_>, _>(Cursor::new(workbook_bytes))
            .map_err(|_| XlsxRewriteError::MissingPart("readable worksheet"))?;
    let sheet_names = workbook.sheet_names().to_owned();
    let mut selected_sheet_name = sheet_names
        .first()
        .cloned()
        .ok_or(XlsxRewriteError::MissingPart("worksheet"))?;

    for (sheet_index, sheet_name) in sheet_names.iter().enumerate() {
        let range = workbook
            .worksheet_range(sheet_name)
            .map_err(|_| XlsxRewriteError::MissingPart("worksheet range"))?;
        let has_non_blank_cells = range
            .rows()
            .flatten()
            .any(|cell| !cell.to_string().trim().is_empty());

        if sheet_index == 0 {
            selected_sheet_name = sheet_name.clone();
            if has_non_blank_cells {
                break;
            }
            continue;
        }

        if has_non_blank_cells {
            selected_sheet_name = sheet_name.clone();
            break;
        }
    }

    Ok(selected_sheet_name)
}

fn rewrite_sheet_xml(
    worksheet_xml: &[u8],
    original_headers: &[&str],
    original_rows: &[Vec<&str>],
    headers: &[&str],
    rows: &[Vec<&str>],
) -> Result<Vec<u8>, XlsxRewriteError> {
    let mut worksheet = Element::parse(worksheet_xml)?;
    let sheet_data = worksheet
        .get_mut_child("sheetData")
        .ok_or(XlsxRewriteError::MissingPart("sheetData"))?;

    for (row_index, (original_row, rewritten_row)) in std::iter::once((original_headers, headers))
        .chain(
            original_rows
                .iter()
                .zip(rows.iter())
                .map(|(original, rewritten)| (original.as_slice(), rewritten.as_slice())),
        )
        .enumerate()
    {
        for (column_index, (original_value, rewritten_value)) in
            original_row.iter().zip(rewritten_row.iter()).enumerate()
        {
            if original_value == rewritten_value {
                continue;
            }
            let reference = format!("{}{}", excel_column_name(column_index), row_index + 1);
            upsert_inline_string_cell(sheet_data, row_index + 1, &reference, rewritten_value);
        }
    }

    let mut rewritten = Vec::new();
    worksheet.write(&mut rewritten)?;
    Ok(rewritten)
}

fn upsert_inline_string_cell(
    sheet_data: &mut Element,
    row_number: usize,
    reference: &str,
    value: &str,
) {
    let row = get_or_create_row(sheet_data, row_number);
    let cell = get_or_create_cell(row, reference);
    cell.attributes.insert("r".into(), reference.into());
    cell.attributes.insert("t".into(), "inlineStr".into());
    cell.children.clear();

    let mut inline_string = Element::new("is");
    let mut text = Element::new("t");
    text.children.push(XMLNode::Text(value.to_string()));
    inline_string.children.push(XMLNode::Element(text));
    cell.children.push(XMLNode::Element(inline_string));
}

fn get_or_create_row(sheet_data: &mut Element, row_number: usize) -> &mut Element {
    if let Some(index) = sheet_data.children.iter().position(|node| {
        matches!(node, XMLNode::Element(row)
            if row.name == "row"
                && row.attributes.get("r").and_then(|value| value.parse::<usize>().ok()) == Some(row_number))
    }) {
        return element_mut(&mut sheet_data.children[index]);
    }

    let mut row = Element::new("row");
    row.attributes.insert("r".into(), row_number.to_string());
    sheet_data.children.push(XMLNode::Element(row));
    let last_index = sheet_data.children.len() - 1;
    element_mut(&mut sheet_data.children[last_index])
}

fn get_or_create_cell<'a>(row: &'a mut Element, reference: &str) -> &'a mut Element {
    if let Some(index) = row.children.iter().position(|node| {
        matches!(node, XMLNode::Element(cell)
            if cell.name == "c" && cell.attributes.get("r").map(|value| value.as_str()) == Some(reference))
    }) {
        return element_mut(&mut row.children[index]);
    }

    let mut cell = Element::new("c");
    cell.attributes.insert("r".into(), reference.into());
    row.children.push(XMLNode::Element(cell));
    let last_index = row.children.len() - 1;
    element_mut(&mut row.children[last_index])
}

fn element_mut(node: &mut XMLNode) -> &mut Element {
    match node {
        XMLNode::Element(element) => element,
        _ => unreachable!("selected xml node should always be an element"),
    }
}

fn normalize_workbook_target(target: &str) -> String {
    if let Some(stripped) = target.strip_prefix("/") {
        stripped.to_string()
    } else if let Some(stripped) = target.strip_prefix("xl/") {
        format!("xl/{stripped}")
    } else {
        format!("xl/{target}")
    }
}

fn read_zip_entry(workbook_bytes: &[u8], path: &str) -> Result<Vec<u8>, XlsxRewriteError> {
    let mut archive = ZipArchive::new(Cursor::new(workbook_bytes))?;
    let mut file = archive.by_name(path)?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;
    Ok(contents)
}

fn excel_column_name(mut index: usize) -> String {
    let mut name = String::new();
    loop {
        name.insert(0, (b'A' + (index % 26) as u8) as char);
        if index < 26 {
            break;
        }
        index = (index / 26) - 1;
    }
    name
}

fn deserialize_optional_limit<'de, D>(deserializer: D) -> Result<Option<usize>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let limit = Option::<usize>::deserialize(deserializer)?;

    match limit {
        Some(0) => Err(serde::de::Error::custom("limit must be greater than zero")),
        Some(limit) => Ok(Some(limit)),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use mdid_adapters::{DicomAdapter, DicomAdapterError};
    use mdid_domain::DicomPrivateTagPolicy;
    use std::backtrace::Backtrace;
    use tower::ServiceExt;

    #[tokio::test]
    async fn ocr_to_privacy_filter_summary_accepts_fixture_handoff_without_raw_phi() {
        let handoff: Value = serde_json::from_str(include_str!(
            "../../../scripts/ocr_eval/fixtures/ocr_handoff_expected_shape.json"
        ))
        .expect("fixture should be valid JSON");
        let response = build_default_router()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/ocr-to-privacy-filter/summary")
                    .header("content-type", "application/json")
                    .body(Body::from(json!({ "handoff": handoff }).to_string()))
                    .unwrap(),
            )
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let body_text = String::from_utf8(body.to_vec()).expect("response should be utf8");
        assert!(!body_text.contains("Jane Example"));
        assert!(!body_text.contains("MRN-12345"));
        assert!(!body_text.contains("jane@example.com"));
        assert!(!body_text.contains("555-123-4567"));
        let value: Value = serde_json::from_str(&body_text).expect("response should be json");
        assert_eq!(value["artifact"], "privacy_filter_summary");
        assert_eq!(value["network_api_called"], false);
        assert_eq!(value["category_counts"]["NAME"], 1);
        assert_eq!(value["category_counts"]["MRN"], 1);
        assert_eq!(value["category_counts"]["EMAIL"], 1);
        assert_eq!(value["category_counts"]["PHONE"], 1);
    }

    #[tokio::test]
    async fn ocr_to_privacy_filter_summary_rejects_unknown_request_fields_without_phi_echo() {
        let response = build_default_router()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/ocr-to-privacy-filter/summary")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "handoff": {},
                            "Patient Jane Example": "MRN-12345"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_text = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_text.contains("invalid_ocr_to_privacy_filter_summary_request"));
        assert!(!body_text.contains("Patient Jane Example"));
        assert!(!body_text.contains("MRN-12345"));
    }

    #[tokio::test]
    async fn ocr_to_privacy_filter_summary_rejects_visual_redaction_marker_without_phi_echo() {
        let mut handoff: Value = serde_json::from_str(include_str!(
            "../../../scripts/ocr_eval/fixtures/ocr_handoff_expected_shape.json"
        ))
        .expect("fixture should be valid JSON");
        handoff["visual_redaction"] = json!({ "preview": "Patient Jane Example MRN-12345" });

        let body_text = post_ocr_to_privacy_filter_summary(handoff, StatusCode::BAD_REQUEST).await;
        assert!(body_text.contains("invalid_ocr_to_privacy_filter_summary_request"));
        assert!(!body_text.contains("Patient Jane Example"));
        assert!(!body_text.contains("MRN-12345"));
    }

    #[tokio::test]
    async fn ocr_to_privacy_filter_summary_rejects_empty_normalized_text() {
        let mut handoff: Value = serde_json::from_str(include_str!(
            "../../../scripts/ocr_eval/fixtures/ocr_handoff_expected_shape.json"
        ))
        .expect("fixture should be valid JSON");
        handoff["normalized_text"] = json!("   ");

        let body_text = post_ocr_to_privacy_filter_summary(handoff, StatusCode::BAD_REQUEST).await;
        assert!(body_text.contains("invalid_ocr_to_privacy_filter_summary_request"));
        assert!(!body_text.contains("Jane Example"));
    }

    #[tokio::test]
    async fn ocr_handoff_summary_accepts_existing_fixture_contract() {
        let handoff = serde_json::from_str::<Value>(include_str!(
            "../../../scripts/ocr_eval/fixtures/ocr_handoff_expected_shape.json"
        ))
        .expect("fixture should be valid JSON");
        let response = build_default_router()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/ocr-handoff/summary")
                    .header("content-type", "application/json")
                    .body(Body::from(json!({ "handoff": handoff }).to_string()))
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should buffer");
        let summary = serde_json::from_slice::<Value>(&body).expect("response should be JSON");
        assert_eq!(summary["artifact"], "ocr_handoff_summary");
        assert_eq!(summary["candidate"], "PP-OCRv5_mobile_rec");
        assert_eq!(summary["engine"], "PP-OCRv5-mobile-bounded-spike");
        assert_eq!(summary["scope"], "printed_text_line_extraction_only");
        assert_eq!(
            summary["privacy_filter_contract"],
            "text_only_normalized_input"
        );
        assert_eq!(summary["ready_for_text_pii_eval"], true);
        assert_eq!(summary["network_api_called"], false);
        assert!(summary["non_goals"]
            .as_array()
            .expect("non_goals should be an array")
            .iter()
            .any(|non_goal| non_goal == "visual_redaction"));

        let serialized_summary = serde_json::to_string(&summary).expect("summary should serialize");
        for allowed_input_raw_field in ["extracted_text", "normalized_text"] {
            assert!(
                handoff.get(allowed_input_raw_field).is_some(),
                "official fixture should include top-level {allowed_input_raw_field} as input"
            );
        }

        for forbidden in [
            "Jane Example",
            "MRN-12345",
            "jane@example.com",
            "555-123-4567",
            "extracted_text",
            "normalized_text",
            "bbox",
        ] {
            assert!(
                !serialized_summary.contains(forbidden),
                "summary must not contain {forbidden}"
            );
        }
    }

    #[tokio::test]
    async fn ocr_handoff_summary_rejects_raw_text_and_incompatible_markers_without_echoing_phi() {
        let mut handoff = valid_ocr_handoff_fixture();
        handoff["spans"] = json!([{ "preview": "Jane Example MRN-12345" }]);
        handoff["line_count"] = json!("Jane Example");
        let body = post_ocr_handoff_summary(handoff, StatusCode::BAD_REQUEST).await;

        assert!(body.contains("invalid_ocr_handoff_summary_request"));
        for forbidden in [
            "Jane Example",
            "MRN-12345",
            "jane@example.com",
            "555-123-4567",
        ] {
            assert!(
                !body.contains(forbidden),
                "error body must not echo {forbidden}"
            );
        }
    }

    #[tokio::test]
    async fn ocr_handoff_summary_rejects_network_api_called_true_without_echoing_phi() {
        for mutation in [
            json!({"network_api_called": true}),
            json!({"metadata": {"network_api_called": true, "note": "Jane Example MRN-12345"}}),
        ] {
            let mut handoff = valid_ocr_handoff_fixture();
            let mutation = mutation
                .as_object()
                .expect("mutation should be an object")
                .clone();
            handoff
                .as_object_mut()
                .expect("handoff should be an object")
                .extend(mutation);

            let body = post_ocr_handoff_summary(handoff, StatusCode::BAD_REQUEST).await;
            assert!(body.contains("invalid_ocr_handoff_summary_request"));
            for forbidden in [
                "Jane Example",
                "MRN-12345",
                "jane@example.com",
                "555-123-4567",
            ] {
                assert!(
                    !body.contains(forbidden),
                    "error body must not echo {forbidden}"
                );
            }
        }
    }

    #[tokio::test]
    async fn ocr_handoff_summary_rejects_unsafe_non_contract_fields_recursively() {
        for mutation in [
            json!({"raw_text": "Jane Example MRN-12345"}),
            json!({"metadata": {"file_path": "/tmp/Jane Example.pdf"}}),
            json!({"metadata": {"source": "Jane Example.pdf"}}),
            json!({"metadata": [{"ocr_output": "jane@example.com"}]}),
            json!({"previews": [{"text": "Jane Example MRN-12345"}]}),
            json!({"metadata": {"bbox": [1, 2, 3, 4], "note": "jane@example.com"}}),
            json!({"metadata": [{"image": "555-123-4567"}]}),
            json!({"visual_redaction": "Jane Example overlay MRN-12345"}),
        ] {
            let mut handoff = valid_ocr_handoff_fixture();
            let mutation = mutation
                .as_object()
                .expect("mutation should be an object")
                .clone();
            handoff
                .as_object_mut()
                .expect("handoff should be an object")
                .extend(mutation);

            let body = post_ocr_handoff_summary(handoff, StatusCode::BAD_REQUEST).await;
            assert!(body.contains("invalid_ocr_handoff_summary_request"));
            for forbidden in ["Jane Example", "MRN-12345", "jane@example.com"] {
                assert!(
                    !body.contains(forbidden),
                    "error body must not echo {forbidden}"
                );
            }
        }
    }

    #[tokio::test]
    async fn ocr_handoff_summary_rejects_not_ready_or_wrong_contract() {
        for mutation in [
            ("ready_for_text_pii_eval", json!(false)),
            ("privacy_filter_contract", json!("visual_redaction")),
            ("scope", json!("full_pdf_ocr")),
            ("line_count", json!("Jane Example")),
        ] {
            let mut handoff = valid_ocr_handoff_fixture();
            handoff[mutation.0] = mutation.1;
            let body = post_ocr_handoff_summary(handoff, StatusCode::BAD_REQUEST).await;
            assert!(body.contains("invalid_ocr_handoff_summary_request"));
            for forbidden in [
                "Jane Example",
                "MRN-12345",
                "jane@example.com",
                "555-123-4567",
            ] {
                assert!(
                    !body.contains(forbidden),
                    "error body must not echo {forbidden}"
                );
            }
        }
    }

    fn valid_ocr_handoff_fixture() -> Value {
        serde_json::from_str::<Value>(include_str!(
            "../../../scripts/ocr_eval/fixtures/ocr_handoff_expected_shape.json"
        ))
        .expect("fixture should be valid JSON")
    }

    async fn post_ocr_to_privacy_filter_summary(
        handoff: Value,
        expected_status: StatusCode,
    ) -> String {
        let response = build_default_router()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/ocr-to-privacy-filter/summary")
                    .header("content-type", "application/json")
                    .body(Body::from(json!({ "handoff": handoff }).to_string()))
                    .unwrap(),
            )
            .await
            .expect("router should respond");
        assert_eq!(response.status(), expected_status);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        String::from_utf8(body.to_vec()).unwrap()
    }

    async fn post_ocr_handoff_summary(handoff: Value, expected_status: StatusCode) -> String {
        let response = build_default_router()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/ocr-handoff/summary")
                    .header("content-type", "application/json")
                    .body(Body::from(json!({ "handoff": handoff }).to_string()))
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(response.status(), expected_status);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should buffer");
        String::from_utf8(body.to_vec()).expect("body should be UTF-8")
    }

    #[test]
    fn classifies_parse_errors_as_invalid_dicom() {
        let error = ApplicationError::DicomAdapter(
            DicomAdapter::new(DicomPrivateTagPolicy::Remove)
                .extract(b"not-a-dicom-payload", "broken.dcm")
                .expect_err("garbage bytes should fail DICOM parse"),
        );

        assert_eq!(
            map_application_error(&error).0,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }

    #[test]
    fn classifies_rewrite_meta_errors_as_internal_error() {
        let error = ApplicationError::DicomAdapter(invalid_meta_error().into());

        assert!(matches!(
            error,
            ApplicationError::DicomAdapter(DicomAdapterError::Meta(_))
        ));
        assert_eq!(
            map_application_error(&error).0,
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn classifies_rewrite_write_errors_as_internal_error() {
        let error = ApplicationError::DicomAdapter(invalid_write_error().into());

        assert!(matches!(
            error,
            ApplicationError::DicomAdapter(DicomAdapterError::Write(_))
        ));
        assert_eq!(
            map_application_error(&error).0,
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    fn invalid_meta_error() -> dicom_object::WithMetaError {
        dicom_object::WithMetaError::BuildMetaTable {
            source: dicom_object::meta::Error::MissingElement {
                alias: "Media Storage SOP Class UID",
                backtrace: Backtrace::capture(),
            },
        }
    }

    fn invalid_write_error() -> dicom_object::WriteError {
        dicom_object::WriteError::WritePreamble {
            backtrace: Backtrace::capture(),
            source: std::io::Error::other("simulated write failure"),
        }
    }
}
