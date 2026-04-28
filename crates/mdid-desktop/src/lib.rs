pub const DESKTOP_FILE_IMPORT_MAX_BYTES: usize = 10 * 1024 * 1024;

#[derive(Clone, PartialEq, Eq)]
pub struct DesktopFileImportPayload {
    pub mode: DesktopWorkflowMode,
    pub payload: String,
    pub source_name: Option<String>,
}

impl std::fmt::Debug for DesktopFileImportPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DesktopFileImportPayload")
            .field("mode", &self.mode)
            .field("payload", &"<redacted>")
            .field("source_name", &"<redacted>")
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopFileImportError {
    UnsupportedFileType,
    FileTooLarge,
    InvalidCsvUtf8,
}

impl DesktopFileImportPayload {
    pub fn from_bytes(
        source_name: impl Into<String>,
        bytes: &[u8],
    ) -> Result<Self, DesktopFileImportError> {
        if bytes.len() > DESKTOP_FILE_IMPORT_MAX_BYTES {
            return Err(DesktopFileImportError::FileTooLarge);
        }

        let source_name = source_name.into();
        let extension = source_name
            .rsplit_once('.')
            .map(|(_, extension)| extension.to_ascii_lowercase())
            .ok_or(DesktopFileImportError::UnsupportedFileType)?;

        match extension.as_str() {
            "csv" => Ok(Self {
                mode: DesktopWorkflowMode::CsvText,
                payload: std::str::from_utf8(bytes)
                    .map_err(|_| DesktopFileImportError::InvalidCsvUtf8)?
                    .to_string(),
                source_name: None,
            }),
            "xlsx" => Ok(Self {
                mode: DesktopWorkflowMode::XlsxBase64,
                payload: encode_base64(bytes),
                source_name: None,
            }),
            "pdf" => Ok(Self {
                mode: DesktopWorkflowMode::PdfBase64Review,
                payload: encode_base64(bytes),
                source_name: Some(source_name),
            }),
            "dcm" | "dicom" => Ok(Self {
                mode: DesktopWorkflowMode::DicomBase64,
                payload: encode_base64(bytes),
                source_name: Some(source_name),
            }),
            _ => Err(DesktopFileImportError::UnsupportedFileType),
        }
    }
}

fn encode_base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);

    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);

        encoded.push(TABLE[(b0 >> 2) as usize] as char);
        encoded.push(TABLE[(((b0 & 0b0000_0011) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            encoded.push(TABLE[(((b1 & 0b0000_1111) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            encoded.push('=');
        }
        if chunk.len() > 2 {
            encoded.push(TABLE[(b2 & 0b0011_1111) as usize] as char);
        } else {
            encoded.push('=');
        }
    }

    encoded
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopWorkflowMode {
    CsvText,
    XlsxBase64,
    PdfBase64Review,
    DicomBase64,
}

impl DesktopWorkflowMode {
    pub const ALL: [Self; 4] = [
        Self::CsvText,
        Self::XlsxBase64,
        Self::PdfBase64Review,
        Self::DicomBase64,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::CsvText => "CSV text",
            Self::XlsxBase64 => "XLSX base64",
            Self::PdfBase64Review => "PDF base64 review",
            Self::DicomBase64 => "DICOM base64",
        }
    }

    pub fn payload_hint(self) -> &'static str {
        match self {
            Self::CsvText => "Paste CSV text for local request preparation",
            Self::XlsxBase64 => "Paste XLSX workbook bytes encoded as base64",
            Self::PdfBase64Review => {
                "Paste PDF bytes encoded as base64 for review request preparation"
            }
            Self::DicomBase64 => "Paste DICOM bytes encoded as base64",
        }
    }

    pub fn disclosure(self) -> &'static str {
        match self {
            Self::CsvText => "CSV text de-identification uses the bounded local runtime route /tabular/deidentify; no generalized workflow orchestrator is included.",
            Self::XlsxBase64 => "XLSX base64 de-identification uses the bounded local runtime route /tabular/deidentify/xlsx; no generalized workflow orchestrator is included.",
            Self::PdfBase64Review => "PDF base64 review uses the bounded local runtime route /pdf/deidentify; no generalized workflow orchestrator and no OCR/PDF rewrite are included.",
            Self::DicomBase64 => "DICOM base64 de-identification uses the bounded local runtime route /dicom/deidentify for tag-level DICOM de-identification; no generalized workflow orchestrator is included.",
        }
    }

    pub fn route(self) -> &'static str {
        match self {
            Self::CsvText => "/tabular/deidentify",
            Self::XlsxBase64 => "/tabular/deidentify/xlsx",
            Self::PdfBase64Review => "/pdf/deidentify",
            Self::DicomBase64 => "/dicom/deidentify",
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

pub const DESKTOP_VAULT_WORKBENCH_COPY: &str = "Bounded desktop vault workbench: prepares request envelopes for existing localhost runtime vault routes, including explicit decode and read-only audit browsing. It does not store passphrases, browse vault contents directly, transfer portable artifacts, and does not add controller, agent, or orchestration behavior.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopVaultMode {
    Decode,
    AuditEvents,
}

impl DesktopVaultMode {
    pub fn route(self) -> &'static str {
        match self {
            Self::Decode => "/vault/decode",
            Self::AuditEvents => "/vault/audit/events",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopVaultRequestState {
    pub mode: DesktopVaultMode,
    pub vault_path: String,
    pub vault_passphrase: String,
    pub record_ids_json: String,
    pub output_target: String,
    pub audit_kind: Option<String>,
    pub audit_actor: Option<String>,
}

impl Default for DesktopVaultRequestState {
    fn default() -> Self {
        Self {
            mode: DesktopVaultMode::Decode,
            vault_path: String::new(),
            vault_passphrase: String::new(),
            record_ids_json: "[]".to_string(),
            output_target: "desktop-workbench".to_string(),
            audit_kind: None,
            audit_actor: None,
        }
    }
}

impl DesktopVaultRequestState {
    pub fn try_build_request(&self) -> Result<DesktopWorkflowRequest, DesktopVaultValidationError> {
        let vault_path = self.vault_path.trim();
        if vault_path.is_empty() {
            return Err(DesktopVaultValidationError::BlankVaultPath);
        }

        let vault_passphrase = self.vault_passphrase.trim();
        if vault_passphrase.is_empty() {
            return Err(DesktopVaultValidationError::BlankVaultPassphrase);
        }

        let body = match self.mode {
            DesktopVaultMode::Decode => {
                let output_target = self.output_target.trim();
                if output_target.is_empty() {
                    return Err(DesktopVaultValidationError::BlankOutputTarget);
                }

                let record_ids: Vec<uuid::Uuid> = serde_json::from_str(&self.record_ids_json)
                    .map_err(|error| {
                        DesktopVaultValidationError::InvalidRecordIdsJson(error.to_string())
                    })?;
                if record_ids.is_empty() {
                    return Err(DesktopVaultValidationError::EmptyRecordIds);
                }

                serde_json::json!({
                    "vault_path": vault_path,
                    "vault_passphrase": vault_passphrase,
                    "record_ids": record_ids,
                    "output_target": output_target,
                })
            }
            DesktopVaultMode::AuditEvents => serde_json::json!({
                "vault_path": vault_path,
                "vault_passphrase": vault_passphrase,
                "kind": self.audit_kind.as_deref().map(str::trim).filter(|value| !value.is_empty()),
                "actor": self.audit_actor.as_deref().map(str::trim).filter(|value| !value.is_empty()),
            }),
        };

        Ok(DesktopWorkflowRequest {
            route: self.mode.route(),
            body,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopVaultValidationError {
    BlankVaultPath,
    BlankVaultPassphrase,
    BlankOutputTarget,
    EmptyRecordIds,
    InvalidRecordIdsJson(String),
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
                "Ready to submit to {}; this slice can submit prepared envelopes to a localhost runtime, use bounded file import/export helpers, and render runtime-shaped responses locally. This workstation preview performs no OCR, visual redaction, PDF rewrite/export, vault/decode/audit workflow, or full review workflow.",
                request.route
            ),
            Err(error) => format!("Not ready: {error:?}"),
        }
    }

    pub fn apply_imported_file(&mut self, imported: DesktopFileImportPayload) {
        self.mode = imported.mode;
        self.payload = imported.payload;
        if let Some(source_name) = imported.source_name {
            self.source_name = source_name;
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
                    DesktopWorkflowMode::PdfBase64Review | DesktopWorkflowMode::DicomBase64 => {
                        unreachable!()
                    }
                };

                Ok(DesktopWorkflowRequest {
                    route: self.mode.route(),
                    body,
                })
            }
            DesktopWorkflowMode::PdfBase64Review | DesktopWorkflowMode::DicomBase64 => {
                if self.source_name.trim().is_empty() {
                    return Err(DesktopWorkflowValidationError::BlankSourceName);
                }

                let body = match self.mode {
                    DesktopWorkflowMode::PdfBase64Review => serde_json::json!({
                        "pdf_bytes_base64": self.payload.trim(),
                        "source_name": self.source_name.trim(),
                    }),
                    DesktopWorkflowMode::DicomBase64 => serde_json::json!({
                        "dicom_bytes_base64": self.payload.trim(),
                        "source_name": self.source_name.trim(),
                        "private_tag_policy": "review_required",
                    }),
                    DesktopWorkflowMode::CsvText | DesktopWorkflowMode::XlsxBase64 => {
                        unreachable!()
                    }
                };

                Ok(DesktopWorkflowRequest {
                    route: self.mode.route(),
                    body,
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

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopRuntimeSubmitError {
    InvalidEndpoint(String),
    Io(String),
    InvalidHttpResponse(String),
    RuntimeHttpStatus { status: u16, body: String },
    InvalidJson(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopRuntimeSubmissionSnapshot {
    pub in_flight: bool,
    pub route: Option<&'static str>,
}

impl DesktopRuntimeSubmissionSnapshot {
    pub fn idle() -> Self {
        Self {
            in_flight: false,
            route: None,
        }
    }

    pub fn started(mode: DesktopWorkflowMode) -> Self {
        Self {
            in_flight: true,
            route: Some(mode.route()),
        }
    }

    pub fn submit_button_disabled(&self) -> bool {
        self.in_flight
    }

    pub fn submit_button_label(&self) -> &'static str {
        if self.in_flight {
            "Submitting to local runtime..."
        } else {
            "Submit to local runtime"
        }
    }

    pub fn progress_banner(&self) -> Option<String> {
        self.route
            .filter(|_| self.in_flight)
            .map(|route| format!("Submitting {route} to local runtime..."))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopRuntimeSettings {
    pub host: String,
    pub port_text: String,
}

impl Default for DesktopRuntimeSettings {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port_text: "8787".to_string(),
        }
    }
}

impl DesktopRuntimeSettings {
    pub fn parse_port(&self) -> Result<u16, DesktopRuntimeSubmitError> {
        const MESSAGE: &str = "desktop runtime port must be a number between 1 and 65535";
        let port = self
            .port_text
            .trim()
            .parse::<u16>()
            .map_err(|_| DesktopRuntimeSubmitError::InvalidEndpoint(MESSAGE.to_string()))?;
        if port == 0 {
            return Err(DesktopRuntimeSubmitError::InvalidEndpoint(
                MESSAGE.to_string(),
            ));
        }
        Ok(port)
    }

    pub fn client(&self) -> Result<DesktopRuntimeClient, DesktopRuntimeSubmitError> {
        DesktopRuntimeClient::new(self.host.trim(), self.parse_port()?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopRuntimeClient {
    host: String,
    port: u16,
}

const INVALID_DESKTOP_RUNTIME_ROUTE_MESSAGE: &str =
    "desktop runtime route is not one of the approved local workstation routes";

impl DesktopRuntimeClient {
    pub fn new(host: impl Into<String>, port: u16) -> Result<Self, DesktopRuntimeSubmitError> {
        let host = host.into();
        if !matches!(host.as_str(), "localhost" | "127.0.0.1") {
            return Err(DesktopRuntimeSubmitError::InvalidEndpoint(
                "desktop runtime client only supports localhost/127.0.0.1".to_string(),
            ));
        }
        if port == 0 {
            return Err(DesktopRuntimeSubmitError::InvalidEndpoint(
                "desktop runtime port must be greater than zero".to_string(),
            ));
        }

        Ok(Self { host, port })
    }

    pub fn build_http_request(
        &self,
        request: &DesktopWorkflowRequest,
    ) -> Result<String, DesktopRuntimeSubmitError> {
        Self::validate_runtime_route(request.route)?;

        let body = serde_json::to_string(&request.body)
            .map_err(|error| DesktopRuntimeSubmitError::InvalidJson(error.to_string()))?;

        Ok(format!(
            "POST {} HTTP/1.1\r\nHost: {}:{}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            request.route,
            self.host,
            self.port,
            body.len(),
            body
        ))
    }

    fn validate_runtime_route(route: &str) -> Result<(), DesktopRuntimeSubmitError> {
        let approved = DesktopWorkflowMode::ALL
            .iter()
            .any(|mode| route == mode.route());
        if !route.starts_with('/') || route.contains(['\r', '\n']) || !approved {
            return Err(DesktopRuntimeSubmitError::InvalidEndpoint(
                INVALID_DESKTOP_RUNTIME_ROUTE_MESSAGE.to_string(),
            ));
        }

        Ok(())
    }

    pub fn submit(
        &self,
        request: &DesktopWorkflowRequest,
    ) -> Result<serde_json::Value, DesktopRuntimeSubmitError> {
        use std::io::{Read, Write};
        use std::time::Duration;

        let http_request = self.build_http_request(request)?;
        let mut stream = std::net::TcpStream::connect((self.host.as_str(), self.port))
            .map_err(|error| DesktopRuntimeSubmitError::Io(error.to_string()))?;
        let timeout = Some(Duration::from_secs(10));
        stream
            .set_read_timeout(timeout)
            .map_err(|error| DesktopRuntimeSubmitError::Io(error.to_string()))?;
        stream
            .set_write_timeout(timeout)
            .map_err(|error| DesktopRuntimeSubmitError::Io(error.to_string()))?;
        stream
            .write_all(http_request.as_bytes())
            .map_err(|error| DesktopRuntimeSubmitError::Io(error.to_string()))?;

        let mut response = String::new();
        stream
            .read_to_string(&mut response)
            .map_err(|error| DesktopRuntimeSubmitError::Io(error.to_string()))?;

        Self::extract_json_body(&response)
    }

    pub fn extract_json_body(
        response: &str,
    ) -> Result<serde_json::Value, DesktopRuntimeSubmitError> {
        let (head, body) = response.split_once("\r\n\r\n").ok_or_else(|| {
            DesktopRuntimeSubmitError::InvalidHttpResponse(
                "HTTP response missing header/body separator".to_string(),
            )
        })?;
        let status_line = head.lines().next().ok_or_else(|| {
            DesktopRuntimeSubmitError::InvalidHttpResponse(
                "HTTP response missing status".to_string(),
            )
        })?;
        let status = status_line
            .split_whitespace()
            .nth(1)
            .ok_or_else(|| {
                DesktopRuntimeSubmitError::InvalidHttpResponse(
                    "HTTP response missing status code".to_string(),
                )
            })?
            .parse::<u16>()
            .map_err(|error| DesktopRuntimeSubmitError::InvalidHttpResponse(error.to_string()))?;

        if !(200..300).contains(&status) {
            return Err(DesktopRuntimeSubmitError::RuntimeHttpStatus {
                status,
                body: body.to_string(),
            });
        }

        serde_json::from_str(body)
            .map_err(|error| DesktopRuntimeSubmitError::InvalidJson(error.to_string()))
    }
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
    pub fn exportable_output(&self) -> Option<&str> {
        let output = self.output.trim();
        if output.is_empty()
            || output == "No rewritten PDF bytes returned by the bounded review route."
        {
            None
        } else {
            Some(self.output.as_str())
        }
    }

    pub fn suggested_export_file_name(&self, mode: DesktopWorkflowMode) -> Option<&'static str> {
        self.exportable_output()?;
        match mode {
            DesktopWorkflowMode::CsvText => Some("desktop-deidentified.csv"),
            DesktopWorkflowMode::XlsxBase64 => Some("desktop-deidentified.xlsx.base64.txt"),
            DesktopWorkflowMode::PdfBase64Review => None,
            DesktopWorkflowMode::DicomBase64 => Some("desktop-deidentified.dcm.base64.txt"),
        }
    }

    pub fn apply_success_json(&mut self, mode: DesktopWorkflowMode, envelope: serde_json::Value) {
        self.banner = match mode {
            DesktopWorkflowMode::CsvText => "CSV text runtime response rendered locally.".to_string(),
            DesktopWorkflowMode::XlsxBase64 => {
                "XLSX base64 runtime response rendered locally.".to_string()
            }
            DesktopWorkflowMode::PdfBase64Review => "PDF base64 review runtime response rendered locally; no PDF rewrite/export is available.".to_string(),
            DesktopWorkflowMode::DicomBase64 => {
                "DICOM base64 runtime response rendered locally.".to_string()
            }
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
            DesktopWorkflowMode::DicomBase64 => envelope
                .get("rewritten_dicom_bytes_base64")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string(),
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
    fn desktop_vault_decode_request_builds_existing_runtime_contract() {
        let state = DesktopVaultRequestState {
            mode: DesktopVaultMode::Decode,
            vault_path: "C:/vaults/local.mdid".to_string(),
            vault_passphrase: "correct horse battery staple".to_string(),
            record_ids_json: r#"["550e8400-e29b-41d4-a716-446655440000"]"#.to_string(),
            output_target: "review-workbench".to_string(),
            audit_kind: None,
            audit_actor: None,
        };

        let request = state
            .try_build_request()
            .expect("decode request should build");

        assert_eq!(request.route, "/vault/decode");
        assert_eq!(request.body["vault_path"], "C:/vaults/local.mdid");
        assert_eq!(
            request.body["vault_passphrase"],
            "correct horse battery staple"
        );
        assert_eq!(
            request.body["record_ids"][0],
            "550e8400-e29b-41d4-a716-446655440000"
        );
        assert_eq!(request.body["output_target"], "review-workbench");
    }

    #[test]
    fn desktop_vault_audit_request_builds_read_only_filter_contract() {
        let state = DesktopVaultRequestState {
            mode: DesktopVaultMode::AuditEvents,
            vault_path: "C:/vaults/local.mdid".to_string(),
            vault_passphrase: "correct horse battery staple".to_string(),
            record_ids_json: "[]".to_string(),
            output_target: "review-workbench".to_string(),
            audit_kind: Some("Decode".to_string()),
            audit_actor: Some("Desktop".to_string()),
        };

        let request = state
            .try_build_request()
            .expect("audit request should build");

        assert_eq!(request.route, "/vault/audit/events");
        assert_eq!(request.body["vault_path"], "C:/vaults/local.mdid");
        assert_eq!(
            request.body["vault_passphrase"],
            "correct horse battery staple"
        );
        assert_eq!(request.body["kind"], "Decode");
        assert_eq!(request.body["actor"], "Desktop");
        assert!(request.body.get("record_ids").is_none());
    }

    #[test]
    fn desktop_vault_request_validation_rejects_blank_sensitive_inputs() {
        let mut state = DesktopVaultRequestState::default();
        assert_eq!(
            state.try_build_request(),
            Err(DesktopVaultValidationError::BlankVaultPath)
        );

        state.vault_path = "C:/vaults/local.mdid".to_string();
        assert_eq!(
            state.try_build_request(),
            Err(DesktopVaultValidationError::BlankVaultPassphrase)
        );

        state.vault_passphrase = "correct horse battery staple".to_string();
        state.record_ids_json = "not json".to_string();
        assert!(matches!(
            state.try_build_request(),
            Err(DesktopVaultValidationError::InvalidRecordIdsJson(_))
        ));
    }

    #[test]
    fn desktop_vault_workbench_copy_is_bounded_and_non_orchestrating() {
        assert!(DESKTOP_VAULT_WORKBENCH_COPY.contains("existing localhost runtime vault routes"));
        assert!(DESKTOP_VAULT_WORKBENCH_COPY.contains("does not store passphrases"));
        assert!(DESKTOP_VAULT_WORKBENCH_COPY
            .contains("does not add controller, agent, or orchestration behavior"));
    }

    #[test]
    fn desktop_file_import_csv_bytes_map_to_csv_text_payload() {
        let imported =
            DesktopFileImportPayload::from_bytes("patients.csv", b"name\nAlice").unwrap();

        assert_eq!(imported.mode, DesktopWorkflowMode::CsvText);
        assert_eq!(imported.payload, "name\nAlice");
        assert_eq!(imported.source_name, None);
    }

    #[test]
    fn desktop_file_import_xlsx_bytes_map_to_xlsx_base64_payload() {
        let imported =
            DesktopFileImportPayload::from_bytes("patients.xlsx", b"PK\x03\x04").unwrap();

        assert_eq!(imported.mode, DesktopWorkflowMode::XlsxBase64);
        assert_eq!(imported.payload, "UEsDBA==");
        assert_eq!(imported.source_name, None);
    }

    #[test]
    fn desktop_file_import_pdf_bytes_map_to_pdf_base64_payload_with_source_name() {
        let imported = DesktopFileImportPayload::from_bytes("chart.pdf", b"%PDF-1").unwrap();

        assert_eq!(imported.mode, DesktopWorkflowMode::PdfBase64Review);
        assert_eq!(imported.payload, "JVBERi0x");
        assert_eq!(imported.source_name.as_deref(), Some("chart.pdf"));
    }

    #[test]
    fn desktop_file_import_dicom_bytes_map_to_dicom_base64_payload_with_source_name() {
        let imported = DesktopFileImportPayload::from_bytes("study.dcm", b"DICM\x00\x01").unwrap();

        assert_eq!(imported.mode, DesktopWorkflowMode::DicomBase64);
        assert_eq!(imported.payload, encode_base64(b"DICM\x00\x01"));
        assert_eq!(imported.source_name.as_deref(), Some("study.dcm"));

        let imported =
            DesktopFileImportPayload::from_bytes("study.DICOM", b"DICM\x00\x01").unwrap();
        assert_eq!(imported.mode, DesktopWorkflowMode::DicomBase64);
        assert_eq!(imported.source_name.as_deref(), Some("study.DICOM"));
    }

    #[test]
    fn desktop_file_import_payload_debug_redacts_sensitive_fields() {
        let imported = DesktopFileImportPayload::from_bytes(
            "secret-chart.pdf",
            b"Patient name: Alice Smith\nMRN: 12345",
        )
        .unwrap();

        let debug = format!("{imported:?}");

        assert!(debug.contains("payload: \"<redacted>\""));
        assert!(debug.contains("source_name: \"<redacted>\""));
        assert!(!debug.contains("Alice Smith"));
        assert!(!debug.contains("12345"));
        assert!(!debug.contains("secret-chart.pdf"));
        assert!(!debug.contains(&imported.payload));
    }

    #[test]
    fn desktop_file_import_rejects_unsupported_file_type() {
        let error = DesktopFileImportPayload::from_bytes("notes.txt", b"name\nAlice").unwrap_err();

        assert_eq!(error, DesktopFileImportError::UnsupportedFileType);
    }

    #[test]
    fn desktop_file_import_rejects_large_payloads() {
        let bytes = vec![b'a'; DESKTOP_FILE_IMPORT_MAX_BYTES + 1];
        let error = DesktopFileImportPayload::from_bytes("large.csv", &bytes).unwrap_err();

        assert_eq!(error, DesktopFileImportError::FileTooLarge);
    }

    #[test]
    fn desktop_file_import_rejects_non_utf8_csv() {
        let error = DesktopFileImportPayload::from_bytes("patients.csv", &[0xff]).unwrap_err();

        assert_eq!(error, DesktopFileImportError::InvalidCsvUtf8);
    }

    #[test]
    fn desktop_file_import_request_state_apply_updates_import_fields_and_preserves_policy_json() {
        let mut state = DesktopWorkflowRequestState {
            mode: DesktopWorkflowMode::CsvText,
            payload: "old".to_string(),
            field_policy_json: r#"[{"header":"keep","phi_type":"Name","action":"review"}]"#
                .to_string(),
            source_name: "keep.pdf".to_string(),
        };
        let imported = DesktopFileImportPayload::from_bytes("chart.pdf", b"%PDF-1").unwrap();

        state.apply_imported_file(imported);

        assert_eq!(state.mode, DesktopWorkflowMode::PdfBase64Review);
        assert_eq!(state.payload, "JVBERi0x");
        assert_eq!(state.source_name, "chart.pdf");
        assert_eq!(
            state.field_policy_json,
            r#"[{"header":"keep","phi_type":"Name","action":"review"}]"#
        );

        let imported =
            DesktopFileImportPayload::from_bytes("patients.csv", b"name\nAlice").unwrap();
        state.apply_imported_file(imported);

        assert_eq!(state.mode, DesktopWorkflowMode::CsvText);
        assert_eq!(state.payload, "name\nAlice");
        assert_eq!(state.source_name, "chart.pdf");
        assert_eq!(
            state.field_policy_json,
            r#"[{"header":"keep","phi_type":"Name","action":"review"}]"#
        );
    }

    #[test]
    fn desktop_runtime_settings_default_to_localhost() {
        let settings = DesktopRuntimeSettings::default();
        assert_eq!(settings.host, "127.0.0.1");
        assert_eq!(settings.port_text, "8787");
        assert_eq!(settings.parse_port(), Ok(8787));
    }

    #[test]
    fn desktop_runtime_settings_reject_blank_or_invalid_ports() {
        let settings = DesktopRuntimeSettings {
            port_text: "".to_string(),
            ..DesktopRuntimeSettings::default()
        };
        assert_eq!(
            settings.parse_port(),
            Err(DesktopRuntimeSubmitError::InvalidEndpoint(
                "desktop runtime port must be a number between 1 and 65535".to_string()
            ))
        );
        let settings = DesktopRuntimeSettings {
            port_text: "99999".to_string(),
            ..DesktopRuntimeSettings::default()
        };
        assert_eq!(
            settings.parse_port(),
            Err(DesktopRuntimeSubmitError::InvalidEndpoint(
                "desktop runtime port must be a number between 1 and 65535".to_string()
            ))
        );
    }

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
    fn dicom_base64_builds_runtime_compatible_dicom_request_body() {
        let state = DesktopWorkflowRequestState {
            mode: DesktopWorkflowMode::DicomBase64,
            payload: "  RElDTQAB  ".to_string(),
            field_policy_json: String::new(),
            source_name: "  study.dcm  ".to_string(),
        };

        let request = state.try_build_request().unwrap();

        assert_eq!(request.route, "/dicom/deidentify");
        assert_eq!(
            request.body,
            json!({"dicom_bytes_base64":"RElDTQAB","source_name":"study.dcm","private_tag_policy":"review_required"})
        );

        assert_eq!(state.mode.label(), "DICOM base64");
        assert!(state
            .mode
            .payload_hint()
            .contains("DICOM bytes encoded as base64"));
        let disclosure = state.mode.disclosure();
        assert!(disclosure.contains("bounded local runtime"));
        assert!(disclosure.contains("tag-level DICOM de-identification"));
        assert!(disclosure.contains("no generalized workflow orchestrator"));
    }

    #[test]
    fn dicom_submit_requires_source_name_before_runtime_request() {
        let state = DesktopWorkflowRequestState {
            mode: DesktopWorkflowMode::DicomBase64,
            payload: "RElDTQAB".to_string(),
            field_policy_json: String::new(),
            source_name: "  ".to_string(),
        };

        assert!(matches!(
            state.try_build_request(),
            Err(DesktopWorkflowValidationError::BlankSourceName)
        ));
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
    fn status_message_explains_localhost_runtime_submit_boundary() {
        let state = DesktopWorkflowRequestState {
            mode: DesktopWorkflowMode::PdfBase64Review,
            payload: "JVBERi0x".to_string(),
            field_policy_json: DEFAULT_POLICY_JSON.to_string(),
            source_name: "chart.pdf".to_string(),
        };

        let message = state.status_message();

        assert!(message.contains("Ready to submit to /pdf/deidentify"));
        assert!(message.contains("render runtime-shaped responses locally"));
        assert!(message.contains("submit prepared envelopes to a localhost runtime"));
        assert!(message.contains("no OCR, visual redaction, PDF rewrite/export"));
        assert!(message.contains("bounded file import/export helpers"));
        assert!(!message.contains("file picker upload/download UX"));
        assert!(message.contains("vault/decode/audit workflow"));
        assert!(message.contains("full review workflow"));
        assert!(!message.contains(&["control", "ler workflow"].concat()));
    }

    #[test]
    fn runtime_submission_snapshot_drives_button_enabled_state_and_label() {
        let idle = DesktopRuntimeSubmissionSnapshot::idle();
        assert!(!idle.submit_button_disabled());
        assert_eq!(idle.submit_button_label(), "Submit to local runtime");
        assert_eq!(idle.progress_banner(), None);

        let started =
            DesktopRuntimeSubmissionSnapshot::started(DesktopWorkflowMode::PdfBase64Review);
        assert!(started.submit_button_disabled());
        assert_eq!(
            started.submit_button_label(),
            "Submitting to local runtime..."
        );
        assert_eq!(
            started.progress_banner(),
            Some("Submitting /pdf/deidentify to local runtime...".to_string())
        );
    }

    #[test]
    fn desktop_runtime_client_builds_local_post_request() {
        let state = DesktopWorkflowRequestState {
            mode: DesktopWorkflowMode::CsvText,
            payload: "patient_name\nJane".to_string(),
            field_policy_json: r#"[{"header":"patient_name","phi_type":"Name","action":"encode"}]"#
                .to_string(),
            source_name: "unused.pdf".to_string(),
        };
        let request = state.try_build_request().expect("valid request");

        let client = DesktopRuntimeClient::new("127.0.0.1", 8787).expect("valid local client");
        let http = client.build_http_request(&request).expect("request bytes");

        assert!(http.starts_with("POST /tabular/deidentify HTTP/1.1\r\n"));
        assert!(http.contains("Host: 127.0.0.1:8787\r\n"));
        assert!(http.contains("Content-Type: application/json\r\n"));
        assert!(http.contains("Connection: close\r\n"));
        let body = http
            .split_once("\r\n\r\n")
            .expect("HTTP request has header/body separator")
            .1;
        let body_json: serde_json::Value = serde_json::from_str(body).expect("JSON body");
        assert_eq!(body_json, request.body);
        assert_eq!(
            http.lines()
                .find(|line| line.starts_with("Content-Length: "))
                .expect("content length header"),
            format!("Content-Length: {}", body.len())
        );
    }

    #[test]
    fn desktop_runtime_client_rejects_unapproved_route() {
        let request = DesktopWorkflowRequest {
            route: "/not-approved",
            body: serde_json::json!({}),
        };
        let error = DesktopRuntimeClient::new("127.0.0.1", 8787)
            .expect("valid local client")
            .build_http_request(&request)
            .expect_err("unapproved route rejected");

        assert_eq!(
            error,
            DesktopRuntimeSubmitError::InvalidEndpoint(
                "desktop runtime route is not one of the approved local workstation routes"
                    .to_string()
            )
        );
    }

    #[test]
    fn desktop_runtime_client_rejects_route_header_injection() {
        let request = DesktopWorkflowRequest {
            route: "/tabular/deidentify\r\nX-Bad: yes",
            body: serde_json::json!({}),
        };
        let error = DesktopRuntimeClient::new("127.0.0.1", 8787)
            .expect("valid local client")
            .build_http_request(&request)
            .expect_err("CRLF route rejected");

        assert_eq!(
            error,
            DesktopRuntimeSubmitError::InvalidEndpoint(
                "desktop runtime route is not one of the approved local workstation routes"
                    .to_string()
            )
        );
    }

    #[test]
    fn desktop_runtime_submit_error_variants_match_runtime_contract() {
        let _ = DesktopRuntimeSubmitError::InvalidEndpoint("bad endpoint".to_string());
        let _ = DesktopRuntimeSubmitError::Io("io".to_string());
        let _ = DesktopRuntimeSubmitError::InvalidHttpResponse("bad response".to_string());
        let _ = DesktopRuntimeSubmitError::RuntimeHttpStatus {
            status: 500,
            body: "fail".to_string(),
        };
        let _ = DesktopRuntimeSubmitError::InvalidJson("bad json".to_string());
    }

    #[test]
    fn desktop_runtime_client_rejects_non_local_hosts() {
        let error =
            DesktopRuntimeClient::new("example.com", 8787).expect_err("remote host rejected");
        assert_eq!(
            error,
            DesktopRuntimeSubmitError::InvalidEndpoint(
                "desktop runtime client only supports localhost/127.0.0.1".to_string()
            )
        );
    }

    #[test]
    fn desktop_runtime_client_rejects_zero_port() {
        let error = DesktopRuntimeClient::new("127.0.0.1", 0).expect_err("zero port rejected");
        assert_eq!(
            error,
            DesktopRuntimeSubmitError::InvalidEndpoint(
                "desktop runtime port must be greater than zero".to_string()
            )
        );
    }

    #[test]
    fn desktop_runtime_client_extracts_success_json_body() {
        let response = "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 15\r\n\r\n{\"csv\":\"ok\"}";

        let body = DesktopRuntimeClient::extract_json_body(response).expect("success body");

        assert_eq!(body, serde_json::json!({"csv":"ok"}));
    }

    #[test]
    fn desktop_runtime_client_reports_runtime_error_body() {
        let response = "HTTP/1.1 422 Unprocessable Entity\r\ncontent-type: application/json\r\n\r\n{\"error\":\"bad csv\"}";

        let error = DesktopRuntimeClient::extract_json_body(response).expect_err("runtime error");

        assert_eq!(
            error,
            DesktopRuntimeSubmitError::RuntimeHttpStatus {
                status: 422,
                body: "{\"error\":\"bad csv\"}".to_string(),
            }
        );
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
    fn response_state_default_copy_keeps_networking_and_workflow_limits_honest() {
        let response = DesktopWorkflowResponseState::default();

        assert_eq!(response.banner, "No runtime response rendered yet.");
        assert_eq!(
            response.summary,
            "No successful runtime summary rendered yet."
        );
        assert_eq!(response.review_queue, "No review queue rendered yet.");
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
    fn response_state_renders_dicom_runtime_success_envelope() {
        let mut response = DesktopWorkflowResponseState::default();

        response.apply_success_json(
            DesktopWorkflowMode::DicomBase64,
            json!({
                "rewritten_dicom_bytes_base64": "RElDTQAB",
                "summary": {"private_tag_policy": "review_required"},
                "review_queue": [{"tag":"0010,0010"}]
            }),
        );

        assert_eq!(
            response.banner,
            "DICOM base64 runtime response rendered locally."
        );
        assert_eq!(response.output, "RElDTQAB");
        assert!(response.summary.contains("private_tag_policy"));
        assert!(response.review_queue.contains("0010,0010"));
        assert!(response.error.is_none());
        assert_eq!(
            response.suggested_export_file_name(DesktopWorkflowMode::DicomBase64),
            Some("desktop-deidentified.dcm.base64.txt")
        );
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
    fn response_state_suggests_exports_only_when_output_bytes_exist() {
        let mut csv = DesktopWorkflowResponseState::default();
        csv.apply_success_json(
            DesktopWorkflowMode::CsvText,
            json!({"csv":"patient_name\n<NAME-1>","summary":{},"review_queue":[]}),
        );
        assert_eq!(
            csv.suggested_export_file_name(DesktopWorkflowMode::CsvText),
            Some("desktop-deidentified.csv")
        );
        assert_eq!(csv.exportable_output(), Some("patient_name\n<NAME-1>"));

        let mut pdf = DesktopWorkflowResponseState::default();
        pdf.apply_success_json(
            DesktopWorkflowMode::PdfBase64Review,
            json!({"rewritten_pdf_bytes_base64":null,"summary":{},"review_queue":[]}),
        );
        assert_eq!(
            pdf.suggested_export_file_name(DesktopWorkflowMode::PdfBase64Review),
            None
        );
        assert_eq!(pdf.exportable_output(), None);
    }

    #[test]
    fn response_state_suggests_xlsx_export_for_rewritten_workbook_base64() {
        let mut xlsx = DesktopWorkflowResponseState::default();
        xlsx.apply_success_json(
            DesktopWorkflowMode::XlsxBase64,
            json!({"rewritten_workbook_base64":"UEsDBAo=","summary":{},"review_queue":[]}),
        );
        assert_eq!(
            xlsx.suggested_export_file_name(DesktopWorkflowMode::XlsxBase64),
            Some("desktop-deidentified.xlsx.base64.txt")
        );
        assert_eq!(xlsx.exportable_output(), Some("UEsDBAo="));
    }

    #[test]
    fn response_state_records_runtime_error_without_stale_output() {
        let mut response = DesktopWorkflowResponseState::default();
        response.apply_success_json(
            DesktopWorkflowMode::CsvText,
            json!({"csv":"patient_name\n<NAME-1>","summary":{},"review_queue":[]}),
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
