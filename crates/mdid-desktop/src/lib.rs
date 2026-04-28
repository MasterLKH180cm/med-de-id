#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopWorkflowMode {
    CsvText,
    XlsxBase64,
    PdfBase64Review,
}

impl DesktopWorkflowMode {
    pub fn disclosure(self) -> &'static str {
        match self {
            Self::CsvText => "CSV text de-identification uses the bounded local runtime route /tabular/deidentify/csv; no generalized workflow orchestrator is included.",
            Self::XlsxBase64 => "XLSX base64 de-identification uses the bounded local runtime route /tabular/deidentify/xlsx; no generalized workflow orchestrator is included.",
            Self::PdfBase64Review => "PDF base64 review uses the bounded local runtime route /pdf/deidentify; no generalized workflow orchestrator and no OCR/PDF rewrite are included.",
        }
    }

    fn route(self) -> &'static str {
        match self {
            Self::CsvText => "/tabular/deidentify/csv",
            Self::XlsxBase64 => "/tabular/deidentify/xlsx",
            Self::PdfBase64Review => "/pdf/deidentify",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
            field_policy_json: r#"{"patient_name":"redact"}"#.to_string(),
            source_name: "local-workstation-review.pdf".to_string(),
        }
    }
}

impl DesktopWorkflowRequestState {
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

                let field_policies =
                    serde_json::from_str(&self.field_policy_json).map_err(|error| {
                        DesktopWorkflowValidationError::InvalidFieldPolicyJson(error.to_string())
                    })?;

                Ok(DesktopWorkflowRequest {
                    route: self.mode.route(),
                    payload: self.payload.clone(),
                    field_policies: Some(field_policies),
                    source_name: None,
                })
            }
            DesktopWorkflowMode::PdfBase64Review => {
                if self.source_name.trim().is_empty() {
                    return Err(DesktopWorkflowValidationError::BlankSourceName);
                }

                Ok(DesktopWorkflowRequest {
                    route: self.mode.route(),
                    payload: self.payload.clone(),
                    field_policies: None,
                    source_name: Some(self.source_name.clone()),
                })
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DesktopWorkflowRequest {
    pub route: &'static str,
    pub payload: String,
    pub field_policies: Option<serde_json::Value>,
    pub source_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopWorkflowValidationError {
    BlankPayload,
    BlankFieldPolicyJson,
    InvalidFieldPolicyJson(String),
    BlankSourceName,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn default_state_is_csv_with_bounded_local_disclosure_and_default_pdf_source() {
        let state = DesktopWorkflowRequestState::default();

        assert_eq!(state.mode, DesktopWorkflowMode::CsvText);
        assert_eq!(state.payload, "");
        assert_eq!(state.source_name, "local-workstation-review.pdf");
        assert_eq!(state.field_policy_json, r#"{"patient_name":"redact"}"#);

        let disclosure = state.mode.disclosure();
        assert!(disclosure.contains("bounded local runtime"));
        assert!(disclosure.contains("no generalized workflow orchestrator"));
    }

    #[test]
    fn csv_text_builds_tabular_csv_request_with_field_policies() {
        let state = DesktopWorkflowRequestState {
            mode: DesktopWorkflowMode::CsvText,
            payload: "name\nAlice".to_string(),
            field_policy_json: r#"{"patient_name":"mask"}"#.to_string(),
            source_name: "ignored.pdf".to_string(),
        };

        let request = state.try_build_request().unwrap();

        assert_eq!(request.route, "/tabular/deidentify/csv");
        assert_eq!(request.payload, "name\nAlice");
        assert_eq!(request.field_policies, Some(json!({"patient_name":"mask"})));
        assert_eq!(request.source_name, None);
    }

    #[test]
    fn xlsx_base64_builds_tabular_xlsx_request_with_field_policies() {
        let state = DesktopWorkflowRequestState {
            mode: DesktopWorkflowMode::XlsxBase64,
            payload: "UEsDBAo=".to_string(),
            field_policy_json: r#"{"patient_name":"redact"}"#.to_string(),
            source_name: "ignored.pdf".to_string(),
        };

        let request = state.try_build_request().unwrap();

        assert_eq!(request.route, "/tabular/deidentify/xlsx");
        assert_eq!(request.payload, "UEsDBAo=");
        assert_eq!(
            request.field_policies,
            Some(json!({"patient_name":"redact"}))
        );
        assert_eq!(request.source_name, None);
    }

    #[test]
    fn pdf_base64_review_builds_pdf_request_without_field_policies() {
        let state = DesktopWorkflowRequestState {
            mode: DesktopWorkflowMode::PdfBase64Review,
            payload: "JVBERi0x".to_string(),
            field_policy_json: r#"{"patient_name":"redact"}"#.to_string(),
            source_name: "chart.pdf".to_string(),
        };

        let request = state.try_build_request().unwrap();

        assert_eq!(request.route, "/pdf/deidentify");
        assert_eq!(request.payload, "JVBERi0x");
        assert_eq!(request.field_policies, None);
        assert_eq!(request.source_name, Some("chart.pdf".to_string()));

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
            field_policy_json: r#"{"patient_name":"redact"}"#.to_string(),
            source_name: "local-workstation-review.pdf".to_string(),
        };
        assert_eq!(
            blank_csv.try_build_request(),
            Err(DesktopWorkflowValidationError::BlankPayload)
        );

        let blank_policy = DesktopWorkflowRequestState {
            payload: "name\nAlice".to_string(),
            field_policy_json: "  ".to_string(),
            ..DesktopWorkflowRequestState::default()
        };
        assert_eq!(
            blank_policy.try_build_request(),
            Err(DesktopWorkflowValidationError::BlankFieldPolicyJson)
        );

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
            field_policy_json: r#"{"patient_name":"redact"}"#.to_string(),
            source_name: "  ".to_string(),
        };
        assert_eq!(
            blank_pdf_source.try_build_request(),
            Err(DesktopWorkflowValidationError::BlankSourceName)
        );
    }
}
