#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopWorkflowMode {
    CsvText,
    XlsxBase64,
    PdfBase64Review,
}

impl DesktopWorkflowMode {
    pub const ALL: [Self; 3] = [Self::CsvText, Self::XlsxBase64, Self::PdfBase64Review];

    pub fn label(self) -> &'static str {
        match self {
            Self::CsvText => "CSV text",
            Self::XlsxBase64 => "XLSX base64",
            Self::PdfBase64Review => "PDF base64 review",
        }
    }

    pub fn payload_hint(self) -> &'static str {
        match self {
            Self::CsvText => "Paste CSV text for local request preparation",
            Self::XlsxBase64 => "Paste XLSX workbook bytes encoded as base64",
            Self::PdfBase64Review => {
                "Paste PDF bytes encoded as base64 for review request preparation"
            }
        }
    }

    pub fn disclosure(self) -> &'static str {
        match self {
            Self::CsvText => "CSV text de-identification uses the bounded local runtime route /tabular/deidentify; no generalized workflow orchestrator is included.",
            Self::XlsxBase64 => "XLSX base64 de-identification uses the bounded local runtime route /tabular/deidentify/xlsx; no generalized workflow orchestrator is included.",
            Self::PdfBase64Review => "PDF base64 review uses the bounded local runtime route /pdf/deidentify; no generalized workflow orchestrator and no OCR/PDF rewrite are included.",
        }
    }

    pub fn route(self) -> &'static str {
        match self {
            Self::CsvText => "/tabular/deidentify",
            Self::XlsxBase64 => "/tabular/deidentify/xlsx",
            Self::PdfBase64Review => "/pdf/deidentify",
        }
    }

    pub fn endpoint(self) -> &'static str {
        self.route()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct DesktopWorkflowRequestState {
    pub mode: DesktopWorkflowMode,
    pub payload: String,
    pub field_policy_json: String,
    pub source_name: String,
}

impl Default for DesktopWorkflowRequestState {
    fn default() -> Self {
        Self {
            mode: DesktopWorkflowMode::CsvText,
            payload: String::new(),
            field_policy_json: r#"[{"header":"patient_name","phi_type":"Name","action":"encode"},{"header":"patient_id","phi_type":"RecordId","action":"review"}]"#.to_string(),
            source_name: "local-workstation-review.pdf".to_string(),
        }
    }
}

impl DesktopWorkflowRequestState {
    pub fn status_message(&self) -> String {
        match self.try_build_request() {
            Ok(request) => format!(
                "Ready to submit to {}; this slice can render runtime-shaped responses locally, but desktop networking is not wired. This workstation preview performs no OCR, visual redaction, PDF rewrite/export, file picker upload/download UX, vault/decode/audit workflow, or controller workflow.",
                request.route
            ),
            Err(error) => format!("Not ready: {error:?}"),
        }
    }

    pub fn try_build_request(
        &self,
    ) -> Result<DesktopWorkflowRequest, DesktopWorkflowValidationError> {
        if self.payload.trim().is_empty() {
            return Err(DesktopWorkflowValidationError::BlankPayload);
        }

        match self.mode {
            DesktopWorkflowMode::CsvText | DesktopWorkflowMode::XlsxBase64 => {
                if self.field_policy_json.trim().is_empty() {
                    return Err(DesktopWorkflowValidationError::BlankFieldPolicyJson);
                }

                let field_policies = parse_field_policies(&self.field_policy_json)?;
                let payload = self.payload.trim();

                let body = match self.mode {
                    DesktopWorkflowMode::CsvText => serde_json::json!({
                        "csv": payload,
                        "policies": field_policies,
                    }),
                    DesktopWorkflowMode::XlsxBase64 => serde_json::json!({
                        "workbook_base64": payload,
                        "field_policies": field_policies,
                    }),
                    DesktopWorkflowMode::PdfBase64Review => unreachable!(),
                };

                Ok(DesktopWorkflowRequest {
                    route: self.mode.route(),
                    body,
                })
            }
            DesktopWorkflowMode::PdfBase64Review => {
                if self.source_name.trim().is_empty() {
                    return Err(DesktopWorkflowValidationError::BlankSourceName);
                }

                Ok(DesktopWorkflowRequest {
                    route: self.mode.route(),
                    body: serde_json::json!({
                        "pdf_bytes_base64": self.payload.trim(),
                        "source_name": self.source_name.trim(),
                    }),
                })
            }
        }
    }
}

fn parse_field_policies(
    field_policy_json: &str,
) -> Result<serde_json::Value, DesktopWorkflowValidationError> {
    let value: serde_json::Value = serde_json::from_str(field_policy_json).map_err(|error| {
        DesktopWorkflowValidationError::InvalidFieldPolicyJson(error.to_string())
    })?;

    let policies = value.as_array().ok_or_else(|| {
        DesktopWorkflowValidationError::InvalidFieldPolicyJson(
            "field policy JSON must be an array".to_string(),
        )
    })?;

    for (index, policy) in policies.iter().enumerate() {
        let object = policy.as_object().ok_or_else(|| {
            DesktopWorkflowValidationError::InvalidFieldPolicyJson(format!(
                "field policy at index {index} must be an object"
            ))
        })?;

        for field in ["header", "phi_type"] {
            if !object.get(field).is_some_and(serde_json::Value::is_string) {
                return Err(DesktopWorkflowValidationError::InvalidFieldPolicyJson(
                    format!("field policy at index {index} must include string {field}"),
                ));
            }
        }

        match object.get("action").and_then(serde_json::Value::as_str) {
            Some("encode" | "review" | "ignore") => {}
            _ => {
                return Err(DesktopWorkflowValidationError::InvalidFieldPolicyJson(
                    format!(
                        "field policy at index {index} must include action encode, review, or ignore"
                    ),
                ));
            }
        }
    }

    Ok(value)
}

#[derive(Clone, PartialEq)]
pub struct DesktopWorkflowRequest {
    pub route: &'static str,
    pub body: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopWorkflowValidationError {
    BlankPayload,
    BlankFieldPolicyJson,
    InvalidFieldPolicyJson(String),
    BlankSourceName,
}

#[derive(Clone, PartialEq, Eq)]
pub struct DesktopWorkflowResponseState {
    pub banner: String,
    pub output: String,
    pub summary: String,
    pub review_queue: String,
    pub error: Option<String>,
}

impl Default for DesktopWorkflowResponseState {
    fn default() -> Self {
        Self {
            banner: "No runtime response rendered yet.".to_string(),
            output: String::new(),
            summary: "No successful runtime summary rendered yet.".to_string(),
            review_queue: "No review queue rendered yet.".to_string(),
            error: None,
        }
    }
}

impl DesktopWorkflowResponseState {
    pub fn apply_success_json(&mut self, mode: DesktopWorkflowMode, envelope: serde_json::Value) {
        self.banner = match mode {
            DesktopWorkflowMode::CsvText => "CSV text runtime response rendered locally.".to_string(),
            DesktopWorkflowMode::XlsxBase64 => {
                "XLSX base64 runtime response rendered locally.".to_string()
            }
            DesktopWorkflowMode::PdfBase64Review => "PDF base64 review runtime response rendered locally; no PDF rewrite/export is available.".to_string(),
        };

        self.output = match mode {
            DesktopWorkflowMode::CsvText => envelope
                .get("csv")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string(),
            DesktopWorkflowMode::XlsxBase64 => envelope
                .get("rewritten_workbook_base64")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string(),
            DesktopWorkflowMode::PdfBase64Review => envelope
                .get("rewritten_pdf_bytes_base64")
                .and_then(serde_json::Value::as_str)
                .map(ToString::to_string)
                .unwrap_or_else(|| {
                    "No rewritten PDF bytes returned by the bounded review route.".to_string()
                }),
        };

        self.summary = pretty_json_field(&envelope, "summary");
        self.review_queue = pretty_json_field(&envelope, "review_queue");
        self.error = None;
    }

    pub fn apply_error(&mut self, message: impl Into<String>) {
        self.banner = "Runtime response error.".to_string();
        self.output.clear();
        self.summary = "No successful runtime summary rendered yet.".to_string();
        self.review_queue = "No review queue rendered yet.".to_string();
        self.error = Some(message.into());
    }
}

fn pretty_json_field(envelope: &serde_json::Value, field: &str) -> String {
    envelope
        .get(field)
        .and_then(|value| serde_json::to_string_pretty(value).ok())
        .unwrap_or_else(|| "null".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const DEFAULT_POLICY_JSON: &str = r#"[{"header":"patient_name","phi_type":"Name","action":"encode"},{"header":"patient_id","phi_type":"RecordId","action":"review"}]"#;

    #[test]
    fn default_state_is_csv_with_bounded_local_disclosure_and_default_pdf_source() {
        let state = DesktopWorkflowRequestState::default();

        assert_eq!(state.mode, DesktopWorkflowMode::CsvText);
        assert_eq!(state.payload, "");
        assert_eq!(state.source_name, "local-workstation-review.pdf");
        assert_eq!(state.field_policy_json, DEFAULT_POLICY_JSON);

        let disclosure = state.mode.disclosure();
        assert!(disclosure.contains("bounded local runtime"));
        assert!(disclosure.contains("no generalized workflow orchestrator"));
    }

    #[test]
    fn csv_text_builds_runtime_compatible_tabular_request_body() {
        let state = DesktopWorkflowRequestState {
            mode: DesktopWorkflowMode::CsvText,
            payload: "name\nAlice".to_string(),
            field_policy_json: r#"[{"header":"patient_name","phi_type":"Name","action":"encode"}]"#
                .to_string(),
            source_name: "ignored.pdf".to_string(),
        };

        let request = state.try_build_request().unwrap();

        assert_eq!(request.route, "/tabular/deidentify");
        assert_eq!(
            request.body,
            json!({"csv":"name\nAlice","policies":[{"header":"patient_name","phi_type":"Name","action":"encode"}]})
        );
    }

    #[test]
    fn xlsx_base64_builds_runtime_compatible_tabular_request_body() {
        let state = DesktopWorkflowRequestState {
            mode: DesktopWorkflowMode::XlsxBase64,
            payload: "UEsDBAo=".to_string(),
            field_policy_json: r#"[{"header":"patient_name","phi_type":"Name","action":"review"}]"#
                .to_string(),
            source_name: "ignored.pdf".to_string(),
        };

        let request = state.try_build_request().unwrap();

        assert_eq!(request.route, "/tabular/deidentify/xlsx");
        assert_eq!(
            request.body,
            json!({"workbook_base64":"UEsDBAo=","field_policies":[{"header":"patient_name","phi_type":"Name","action":"review"}]})
        );
    }

    #[test]
    fn pdf_base64_review_builds_runtime_compatible_pdf_request_body() {
        let state = DesktopWorkflowRequestState {
            mode: DesktopWorkflowMode::PdfBase64Review,
            payload: "JVBERi0x".to_string(),
            field_policy_json: DEFAULT_POLICY_JSON.to_string(),
            source_name: "chart.pdf".to_string(),
        };

        let request = state.try_build_request().unwrap();

        assert_eq!(request.route, "/pdf/deidentify");
        assert_eq!(
            request.body,
            json!({"pdf_bytes_base64":"JVBERi0x","source_name":"chart.pdf"})
        );

        let disclosure = state.mode.disclosure();
        assert!(disclosure.contains("bounded local runtime"));
        assert!(disclosure.contains("no generalized workflow orchestrator"));
        assert!(disclosure.contains("no OCR/PDF rewrite"));
    }

    #[test]
    fn validation_errors_cover_blank_payload_blank_policy_invalid_json_and_blank_pdf_source() {
        let blank_csv = DesktopWorkflowRequestState {
            mode: DesktopWorkflowMode::CsvText,
            payload: "  ".to_string(),
            field_policy_json: DEFAULT_POLICY_JSON.to_string(),
            source_name: "local-workstation-review.pdf".to_string(),
        };
        assert!(matches!(
            blank_csv.try_build_request(),
            Err(DesktopWorkflowValidationError::BlankPayload)
        ));

        let blank_policy = DesktopWorkflowRequestState {
            payload: "name\nAlice".to_string(),
            field_policy_json: "  ".to_string(),
            ..DesktopWorkflowRequestState::default()
        };
        assert!(matches!(
            blank_policy.try_build_request(),
            Err(DesktopWorkflowValidationError::BlankFieldPolicyJson)
        ));

        let invalid_policy = DesktopWorkflowRequestState {
            payload: "name\nAlice".to_string(),
            field_policy_json: "not json".to_string(),
            ..DesktopWorkflowRequestState::default()
        };
        assert!(matches!(
            invalid_policy.try_build_request(),
            Err(DesktopWorkflowValidationError::InvalidFieldPolicyJson(_))
        ));

        let blank_pdf_source = DesktopWorkflowRequestState {
            mode: DesktopWorkflowMode::PdfBase64Review,
            payload: "JVBERi0x".to_string(),
            field_policy_json: DEFAULT_POLICY_JSON.to_string(),
            source_name: "  ".to_string(),
        };
        assert!(matches!(
            blank_pdf_source.try_build_request(),
            Err(DesktopWorkflowValidationError::BlankSourceName)
        ));
    }

    #[test]
    fn field_policy_validation_rejects_non_array_and_bad_item_schema() {
        for field_policy_json in [
            r#"{"patient_name":"encode"}"#,
            r#"[{"phi_type":"Name","action":"encode"}]"#,
            r#"[{"header":7,"phi_type":"Name","action":"encode"}]"#,
            r#"[{"header":"patient_name","action":"encode"}]"#,
            r#"[{"header":"patient_name","phi_type":7,"action":"encode"}]"#,
            r#"[{"header":"patient_name","phi_type":"Name"}]"#,
            r#"[{"header":"patient_name","phi_type":"Name","action":7}]"#,
            r#"[{"header":"patient_name","phi_type":"Name","action":"Encode"}]"#,
            r#"[{"header":"patient_name","phi_type":"Name","action":"redact"}]"#,
        ] {
            let state = DesktopWorkflowRequestState {
                payload: "name\nAlice".to_string(),
                field_policy_json: field_policy_json.to_string(),
                ..DesktopWorkflowRequestState::default()
            };

            assert!(
                matches!(
                    state.try_build_request(),
                    Err(DesktopWorkflowValidationError::InvalidFieldPolicyJson(_))
                ),
                "policy should be rejected: {field_policy_json}"
            );
        }
    }

    #[test]
    fn status_message_explains_preview_only_runtime_submit_boundary() {
        let state = DesktopWorkflowRequestState {
            mode: DesktopWorkflowMode::PdfBase64Review,
            payload: "JVBERi0x".to_string(),
            field_policy_json: DEFAULT_POLICY_JSON.to_string(),
            source_name: "chart.pdf".to_string(),
        };

        let message = state.status_message();

        assert!(message.contains("Ready to submit to /pdf/deidentify"));
        assert!(message.contains("render runtime-shaped responses locally"));
        assert!(message.contains("desktop networking is not wired"));
        assert!(message.contains("no OCR, visual redaction, PDF rewrite/export"));
        assert!(message.contains("file picker upload/download UX"));
        assert!(message.contains("vault/decode/audit workflow"));
        assert!(message.contains("controller workflow"));
    }

    #[test]
    fn response_state_renders_csv_runtime_success_envelope() {
        let mut response = DesktopWorkflowResponseState::default();

        response.apply_success_json(
            DesktopWorkflowMode::CsvText,
            json!({
                "csv": "patient_name\n<NAME-1>",
                "summary": {"encoded_fields": 1, "review_required": 0},
                "review_queue": []
            }),
        );

        assert_eq!(
            response.banner,
            "CSV text runtime response rendered locally."
        );
        assert!(response.output.contains("<NAME-1>"));
        assert!(response.summary.contains("encoded_fields"));
        assert_eq!(response.review_queue, "[]");
        assert!(response.error.is_none());
    }

    #[test]
    fn response_state_renders_xlsx_runtime_success_envelope() {
        let mut response = DesktopWorkflowResponseState::default();

        response.apply_success_json(
            DesktopWorkflowMode::XlsxBase64,
            json!({
                "rewritten_workbook_base64": "UEsDBAo=",
                "summary": {"encoded_fields": 2},
                "review_queue": [{"header":"patient_id"}]
            }),
        );

        assert_eq!(
            response.banner,
            "XLSX base64 runtime response rendered locally."
        );
        assert_eq!(response.output, "UEsDBAo=");
        assert!(response.summary.contains("encoded_fields"));
        assert!(response.review_queue.contains("patient_id"));
        assert!(response.error.is_none());
    }

    #[test]
    fn response_state_renders_pdf_review_runtime_success_envelope_without_rewrite_claim() {
        let mut response = DesktopWorkflowResponseState::default();

        response.apply_success_json(
            DesktopWorkflowMode::PdfBase64Review,
            json!({
                "rewritten_pdf_bytes_base64": null,
                "summary": {"pages": 1, "ocr_required_pages": 1},
                "pages": [{"page_number": 1, "status": "ocr_required"}],
                "review_queue": [{"page_number": 1, "reason":"ocr_required"}]
            }),
        );

        assert_eq!(response.banner, "PDF base64 review runtime response rendered locally; no PDF rewrite/export is available.");
        assert_eq!(
            response.output,
            "No rewritten PDF bytes returned by the bounded review route."
        );
        assert!(response.summary.contains("ocr_required_pages"));
        assert!(response.review_queue.contains("ocr_required"));
        assert!(response.error.is_none());
    }

    #[test]
    fn response_state_records_runtime_error_without_stale_output() {
        let mut response = DesktopWorkflowResponseState::default();
        response.apply_success_json(
            DesktopWorkflowMode::CsvText,
            json!({"rewritten_csv":"patient_name\n<NAME-1>","summary":{},"review_queue":[]}),
        );

        response.apply_error("runtime rejected invalid payload");

        assert_eq!(response.banner, "Runtime response error.");
        assert_eq!(response.output, "");
        assert_eq!(
            response.summary,
            "No successful runtime summary rendered yet."
        );
        assert_eq!(response.review_queue, "No review queue rendered yet.");
        assert_eq!(
            response.error.as_deref(),
            Some("runtime rejected invalid payload")
        );
    }

    #[test]
    fn request_body_values_are_trimmed_before_insertion() {
        let csv = DesktopWorkflowRequestState {
            mode: DesktopWorkflowMode::CsvText,
            payload: "  name\nAlice  ".to_string(),
            field_policy_json: r#"[{"header":"patient_name","phi_type":"Name","action":"ignore"}]"#
                .to_string(),
            source_name: "ignored.pdf".to_string(),
        }
        .try_build_request()
        .unwrap();
        assert_eq!(csv.body["csv"], "name\nAlice");

        let xlsx = DesktopWorkflowRequestState {
            mode: DesktopWorkflowMode::XlsxBase64,
            payload: "  UEsDBAo=\n".to_string(),
            field_policy_json: r#"[{"header":"patient_name","phi_type":"Name","action":"encode"}]"#
                .to_string(),
            source_name: "ignored.pdf".to_string(),
        }
        .try_build_request()
        .unwrap();
        assert_eq!(xlsx.body["workbook_base64"], "UEsDBAo=");

        let pdf = DesktopWorkflowRequestState {
            mode: DesktopWorkflowMode::PdfBase64Review,
            payload: "\n JVBERi0x \t".to_string(),
            field_policy_json: String::new(),
            source_name: "  chart.pdf  ".to_string(),
        }
        .try_build_request()
        .unwrap();
        assert_eq!(pdf.body["pdf_bytes_base64"], "JVBERi0x");
        assert_eq!(pdf.body["source_name"], "chart.pdf");
    }
}
