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
            "json" => Ok(Self {
                mode: DesktopWorkflowMode::MediaMetadataJson,
                payload: std::str::from_utf8(bytes)
                    .map_err(|_| DesktopFileImportError::InvalidCsvUtf8)?
                    .to_string(),
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
    MediaMetadataJson,
}

impl DesktopWorkflowMode {
    pub const ALL: [Self; 5] = [
        Self::CsvText,
        Self::XlsxBase64,
        Self::PdfBase64Review,
        Self::DicomBase64,
        Self::MediaMetadataJson,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::CsvText => "CSV text",
            Self::XlsxBase64 => "XLSX base64",
            Self::PdfBase64Review => "PDF base64 review",
            Self::DicomBase64 => "DICOM base64",
            Self::MediaMetadataJson => "Media metadata JSON",
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
            Self::MediaMetadataJson => {
                "Paste media metadata JSON for local media review request preparation"
            }
        }
    }

    pub fn disclosure(self) -> &'static str {
        match self {
            Self::CsvText => "CSV text de-identification uses the bounded local runtime route /tabular/deidentify; no generalized workflow orchestrator is included.",
            Self::XlsxBase64 => "XLSX base64 de-identification uses the bounded local runtime route /tabular/deidentify/xlsx; no generalized workflow orchestrator is included.",
            Self::PdfBase64Review => "PDF base64 review uses the bounded local runtime route /pdf/deidentify; no generalized workflow orchestrator and no OCR/PDF rewrite are included.",
            Self::DicomBase64 => "DICOM base64 de-identification uses the bounded local runtime route /dicom/deidentify for tag-level DICOM de-identification; no generalized workflow orchestrator is included.",
            Self::MediaMetadataJson => "Media metadata JSON review uses the bounded local runtime route /media/conservative/deidentify with metadata-only JSON; it does not upload media bytes and performs no OCR.",
        }
    }

    pub fn route(self) -> &'static str {
        match self {
            Self::CsvText => "/tabular/deidentify",
            Self::XlsxBase64 => "/tabular/deidentify/xlsx",
            Self::PdfBase64Review => "/pdf/deidentify",
            Self::DicomBase64 => "/dicom/deidentify",
            Self::MediaMetadataJson => "/media/conservative/deidentify",
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

pub const DESKTOP_VAULT_WORKBENCH_COPY: &str = "Bounded desktop vault workbench: prepares request envelopes for existing localhost runtime vault routes, including explicit decode and read-only audit browsing. It does not persist passphrases, browse vault contents directly, transfer portable artifacts, and does not add controller, agent, or orchestration behavior.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopPortableMode {
    VaultExport,
    InspectArtifact,
    ImportArtifact,
}

impl DesktopPortableMode {
    pub const ALL: [Self; 3] = [
        Self::VaultExport,
        Self::InspectArtifact,
        Self::ImportArtifact,
    ];

    pub fn route(self) -> &'static str {
        match self {
            Self::VaultExport => "/vault/export",
            Self::InspectArtifact => "/portable-artifacts/inspect",
            Self::ImportArtifact => "/portable-artifacts/import",
        }
    }

    pub fn disclosure(self) -> &'static str {
        match self {
            Self::VaultExport => "bounded desktop portable export request preparation for the existing local /vault/export runtime route; no generalized workflow orchestration behavior is included.",
            Self::InspectArtifact => "bounded desktop portable artifact inspection request preparation for the existing local /portable-artifacts/inspect runtime route; no generalized workflow orchestration behavior is included.",
            Self::ImportArtifact => "bounded desktop portable artifact import request preparation for the existing local /portable-artifacts/import runtime route; no generalized workflow orchestration behavior is included.",
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct DesktopPortableRequestState {
    pub mode: DesktopPortableMode,
    pub vault_path: String,
    pub vault_passphrase: String,
    pub record_ids_json: String,
    pub export_passphrase: String,
    pub export_context: String,
    pub artifact_json: String,
    pub portable_passphrase: String,
    pub destination_vault_path: String,
    pub destination_vault_passphrase: String,
    pub import_context: String,
    pub requested_by: String,
}

impl std::fmt::Debug for DesktopPortableRequestState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DesktopPortableRequestState")
            .field("mode", &self.mode)
            .field("vault_path", &self.vault_path)
            .field("vault_passphrase", &"<redacted>")
            .field("record_ids_json", &self.record_ids_json)
            .field("export_passphrase", &"<redacted>")
            .field("export_context", &self.export_context)
            .field("artifact_json", &"<redacted>")
            .field("portable_passphrase", &"<redacted>")
            .field("destination_vault_path", &self.destination_vault_path)
            .field("destination_vault_passphrase", &"<redacted>")
            .field("import_context", &self.import_context)
            .field("requested_by", &self.requested_by)
            .finish()
    }
}

impl Default for DesktopPortableRequestState {
    fn default() -> Self {
        Self {
            mode: DesktopPortableMode::VaultExport,
            vault_path: String::new(),
            vault_passphrase: String::new(),
            record_ids_json: "[]".to_string(),
            export_passphrase: String::new(),
            export_context: "desktop portable export".to_string(),
            artifact_json: String::new(),
            portable_passphrase: String::new(),
            destination_vault_path: String::new(),
            destination_vault_passphrase: String::new(),
            import_context: "desktop portable import".to_string(),
            requested_by: "desktop".to_string(),
        }
    }
}

impl DesktopPortableRequestState {
    pub fn try_build_request(
        &self,
    ) -> Result<DesktopWorkflowRequest, DesktopPortableValidationError> {
        let body = match self.mode {
            DesktopPortableMode::VaultExport => {
                let vault_path = require_nonblank(
                    &self.vault_path,
                    DesktopPortableValidationError::BlankVaultPath,
                )?;
                if self.vault_passphrase.trim().is_empty() {
                    return Err(DesktopPortableValidationError::BlankVaultPassphrase);
                }
                let record_ids_json = require_nonblank(
                    &self.record_ids_json,
                    DesktopPortableValidationError::BlankRecordIdsJson,
                )?;
                if self.export_passphrase.trim().is_empty() {
                    return Err(DesktopPortableValidationError::BlankExportPassphrase);
                }
                let export_context = require_nonblank(
                    &self.export_context,
                    DesktopPortableValidationError::BlankExportContext,
                )?;
                let requested_by = require_nonblank(
                    &self.requested_by,
                    DesktopPortableValidationError::BlankRequestedBy,
                )?;
                let record_ids = parse_portable_record_ids_json(
                    record_ids_json,
                    DesktopPortableValidationError::InvalidRecordIdsJson,
                )?;
                serde_json::json!({
                    "vault_path": vault_path,
                    "vault_passphrase": self.vault_passphrase.clone(),
                    "record_ids": record_ids,
                    "export_passphrase": self.export_passphrase.clone(),
                    "context": export_context,
                    "requested_by": requested_by,
                })
            }
            DesktopPortableMode::InspectArtifact => {
                let artifact_json = require_nonblank(
                    &self.artifact_json,
                    DesktopPortableValidationError::BlankArtifactJson,
                )?;
                if self.portable_passphrase.trim().is_empty() {
                    return Err(DesktopPortableValidationError::BlankPortablePassphrase);
                }
                let artifact = parse_portable_json(
                    artifact_json,
                    DesktopPortableValidationError::InvalidArtifactJson,
                )?;
                serde_json::json!({
                    "artifact": artifact,
                    "portable_passphrase": self.portable_passphrase.clone(),
                })
            }
            DesktopPortableMode::ImportArtifact => {
                let vault_path = require_nonblank(
                    &self.destination_vault_path,
                    DesktopPortableValidationError::BlankDestinationVaultPath,
                )?;
                if self.destination_vault_passphrase.trim().is_empty() {
                    return Err(DesktopPortableValidationError::BlankDestinationVaultPassphrase);
                }
                let artifact_json = require_nonblank(
                    &self.artifact_json,
                    DesktopPortableValidationError::BlankArtifactJson,
                )?;
                if self.portable_passphrase.trim().is_empty() {
                    return Err(DesktopPortableValidationError::BlankPortablePassphrase);
                }
                let import_context = require_nonblank(
                    &self.import_context,
                    DesktopPortableValidationError::BlankImportContext,
                )?;
                let requested_by = require_nonblank(
                    &self.requested_by,
                    DesktopPortableValidationError::BlankRequestedBy,
                )?;
                let artifact = parse_portable_json(
                    artifact_json,
                    DesktopPortableValidationError::InvalidArtifactJson,
                )?;
                serde_json::json!({
                    "vault_path": vault_path,
                    "vault_passphrase": self.destination_vault_passphrase.clone(),
                    "artifact": artifact,
                    "portable_passphrase": self.portable_passphrase.clone(),
                    "context": import_context,
                    "requested_by": requested_by,
                })
            }
        };

        Ok(DesktopWorkflowRequest {
            route: self.mode.route(),
            body,
        })
    }
}

fn require_nonblank<E>(value: &str, error: E) -> Result<&str, E> {
    let value = value.trim();
    if value.is_empty() {
        Err(error)
    } else {
        Ok(value)
    }
}

fn parse_portable_json(
    value: &str,
    error: fn(String) -> DesktopPortableValidationError,
) -> Result<serde_json::Value, DesktopPortableValidationError> {
    serde_json::from_str(value).map_err(|parse_error| error(parse_error.to_string()))
}

fn parse_portable_record_ids_json(
    value: &str,
    error: fn(String) -> DesktopPortableValidationError,
) -> Result<serde_json::Value, DesktopPortableValidationError> {
    let record_ids: Vec<String> =
        serde_json::from_str(value).map_err(|parse_error| error(parse_error.to_string()))?;
    if record_ids.is_empty() {
        return Err(DesktopPortableValidationError::EmptyRecordIds);
    }
    for record_id in &record_ids {
        if record_id.trim().is_empty() {
            return Err(error("record id must not be blank".to_string()));
        }
        uuid::Uuid::parse_str(record_id).map_err(|parse_error| error(parse_error.to_string()))?;
    }
    Ok(serde_json::json!(record_ids))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopPortableValidationError {
    BlankVaultPath,
    BlankVaultPassphrase,
    BlankRecordIdsJson,
    BlankExportPassphrase,
    BlankExportContext,
    BlankArtifactJson,
    BlankPortablePassphrase,
    BlankDestinationVaultPath,
    BlankDestinationVaultPassphrase,
    BlankImportContext,
    BlankRequestedBy,
    EmptyRecordIds,
    InvalidRecordIdsJson(String),
    InvalidArtifactJson(String),
}

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

#[derive(Clone, PartialEq, Eq)]
pub struct DesktopVaultRequestState {
    pub mode: DesktopVaultMode,
    pub vault_path: String,
    pub vault_passphrase: String,
    pub record_ids_json: String,
    pub output_target: String,
    pub justification: String,
    pub requested_by: String,
    pub audit_kind: Option<String>,
    pub audit_actor: Option<String>,
}

impl std::fmt::Debug for DesktopVaultRequestState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DesktopVaultRequestState")
            .field("mode", &self.mode)
            .field("vault_path", &self.vault_path)
            .field("vault_passphrase", &"<redacted>")
            .field("record_ids_json", &self.record_ids_json)
            .field("output_target", &self.output_target)
            .field("justification", &self.justification)
            .field("requested_by", &self.requested_by)
            .field("audit_kind", &self.audit_kind)
            .field("audit_actor", &self.audit_actor)
            .finish()
    }
}

impl Default for DesktopVaultRequestState {
    fn default() -> Self {
        Self {
            mode: DesktopVaultMode::Decode,
            vault_path: String::new(),
            vault_passphrase: String::new(),
            record_ids_json: "[]".to_string(),
            output_target: "desktop-workbench".to_string(),
            justification: "desktop decode request".to_string(),
            requested_by: "desktop".to_string(),
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

        if self.vault_passphrase.trim().is_empty() {
            return Err(DesktopVaultValidationError::BlankVaultPassphrase);
        }

        let body = match self.mode {
            DesktopVaultMode::Decode => {
                let output_target = self.output_target.trim();
                if output_target.is_empty() {
                    return Err(DesktopVaultValidationError::BlankOutputTarget);
                }

                let justification = self.justification.trim();
                if justification.is_empty() {
                    return Err(DesktopVaultValidationError::BlankJustification);
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
                    "vault_passphrase": self.vault_passphrase.clone(),
                    "record_ids": record_ids,
                    "output_target": output_target,
                    "justification": justification,
                    "requested_by": self.requested_by.trim(),
                })
            }
            DesktopVaultMode::AuditEvents => serde_json::json!({
                "vault_path": vault_path,
                "vault_passphrase": self.vault_passphrase.clone(),
                "kind": lowercase_optional_filter(self.audit_kind.as_deref()),
                "actor": lowercase_optional_filter(self.audit_actor.as_deref()),
            }),
        };

        Ok(DesktopWorkflowRequest {
            route: self.mode.route(),
            body,
        })
    }
}

fn lowercase_optional_filter(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopVaultValidationError {
    BlankVaultPath,
    BlankVaultPassphrase,
    BlankOutputTarget,
    BlankJustification,
    EmptyRecordIds,
    InvalidRecordIdsJson(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopVaultResponseMode {
    VaultDecode,
    VaultAudit,
    VaultExport,
    InspectArtifact,
    ImportArtifact,
}

#[derive(Clone, PartialEq, Eq)]
pub struct DesktopVaultResponseState {
    pub banner: String,
    pub error: Option<String>,
    pub summary: String,
    pub artifact_notice: String,
}

impl Default for DesktopVaultResponseState {
    fn default() -> Self {
        Self {
            banner: "No bounded vault or portable response rendered yet.".to_string(),
            error: None,
            summary: String::new(),
            artifact_notice: String::new(),
        }
    }
}

impl DesktopVaultResponseState {
    pub fn apply_success(&mut self, mode: DesktopVaultResponseMode, response: &serde_json::Value) {
        self.banner = vault_response_banner(mode).to_string();
        self.summary = vault_response_summary(mode, response);
        self.artifact_notice = vault_response_artifact_notice(response);
        self.error = None;
    }

    pub fn apply_error(&mut self, mode: DesktopVaultResponseMode, message: impl AsRef<str>) {
        self.banner = vault_response_banner(mode).to_string();
        self.error = Some(redact_desktop_vault_error(message.as_ref()));
        self.summary.clear();
        self.artifact_notice.clear();
    }
}

fn vault_response_banner(mode: DesktopVaultResponseMode) -> &'static str {
    match mode {
        DesktopVaultResponseMode::VaultDecode => "bounded vault decode response rendered locally",
        DesktopVaultResponseMode::VaultAudit => "bounded vault audit response rendered locally",
        DesktopVaultResponseMode::VaultExport
        | DesktopVaultResponseMode::InspectArtifact
        | DesktopVaultResponseMode::ImportArtifact => {
            "bounded portable artifact response rendered locally"
        }
    }
}

fn vault_response_summary(mode: DesktopVaultResponseMode, response: &serde_json::Value) -> String {
    match mode {
        DesktopVaultResponseMode::VaultDecode => format!(
            "decoded values: {}",
            response_u64(response, "decoded_value_count")
        ),
        DesktopVaultResponseMode::VaultAudit => format!(
            "events returned: {} / {}",
            response_u64(response, "returned_event_count"),
            response_u64(response, "event_count")
        ),
        DesktopVaultResponseMode::VaultExport | DesktopVaultResponseMode::InspectArtifact => {
            format!("records: {}", response_u64(response, "record_count"))
        }
        DesktopVaultResponseMode::ImportArtifact => format!(
            "imported records: {}",
            response_u64(response, "imported_record_count")
        ),
    }
}

fn vault_response_artifact_notice(response: &serde_json::Value) -> String {
    if response
        .get("report_path")
        .or_else(|| response.get("artifact_path"))
        .and_then(serde_json::Value::as_str)
        .is_some()
    {
        "artifact path returned; full path hidden".to_string()
    } else {
        String::new()
    }
}

fn response_u64(response: &serde_json::Value, field: &str) -> u64 {
    response
        .get(field)
        .and_then(serde_json::Value::as_u64)
        .unwrap_or_default()
}

fn redact_desktop_vault_error(message: &str) -> String {
    if message.trim().is_empty() {
        "runtime failed".to_string()
    } else {
        "runtime failed; details redacted".to_string()
    }
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
            Err(error) => format!("Not ready: {error}"),
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
                    DesktopWorkflowMode::PdfBase64Review
                    | DesktopWorkflowMode::DicomBase64
                    | DesktopWorkflowMode::MediaMetadataJson => unreachable!(),
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
                    DesktopWorkflowMode::CsvText
                    | DesktopWorkflowMode::XlsxBase64
                    | DesktopWorkflowMode::MediaMetadataJson => unreachable!(),
                };

                Ok(DesktopWorkflowRequest {
                    route: self.mode.route(),
                    body,
                })
            }
            DesktopWorkflowMode::MediaMetadataJson => {
                let body: serde_json::Value = serde_json::from_str(self.payload.trim())
                    .map_err(|_| DesktopWorkflowValidationError::InvalidMediaMetadataJson)?;
                if !body.is_object() {
                    return Err(DesktopWorkflowValidationError::InvalidMediaMetadataJson);
                }

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

#[derive(Clone, PartialEq)]
pub struct DesktopWorkflowRequest {
    pub route: &'static str,
    pub body: serde_json::Value,
}

impl std::fmt::Debug for DesktopWorkflowRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DesktopWorkflowRequest")
            .field("route", &self.route)
            .field("body", &redact_sensitive_request_body_fields(&self.body))
            .finish()
    }
}

fn redact_sensitive_request_body_fields(body: &serde_json::Value) -> serde_json::Value {
    let mut redacted = body.clone();
    if let serde_json::Value::Object(object) = &mut redacted {
        for key in [
            "vault_passphrase",
            "export_passphrase",
            "portable_passphrase",
            "artifact",
        ] {
            if object.contains_key(key) {
                object.insert(
                    key.to_string(),
                    serde_json::Value::String("<redacted>".to_string()),
                );
            }
        }
    }
    redacted
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopWorkflowValidationError {
    BlankPayload,
    BlankFieldPolicyJson,
    InvalidFieldPolicyJson(String),
    BlankSourceName,
    InvalidMediaMetadataJson,
}

impl std::fmt::Display for DesktopWorkflowValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BlankPayload => write!(f, "Payload is required."),
            Self::BlankFieldPolicyJson => write!(f, "Field policy JSON is required."),
            Self::InvalidFieldPolicyJson(message) => write!(f, "Invalid field policy JSON: {message}"),
            Self::BlankSourceName => write!(f, "Source name is required."),
            Self::InvalidMediaMetadataJson => write!(
                f,
                "Media metadata JSON must be a JSON object accepted by the local media review runtime route."
            ),
        }
    }
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
            .any(|mode| route == mode.route())
            || DesktopPortableMode::ALL
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
            DesktopWorkflowMode::MediaMetadataJson => None,
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
            DesktopWorkflowMode::MediaMetadataJson => {
                "Media metadata JSON runtime response rendered locally; no media bytes were uploaded."
                    .to_string()
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
            DesktopWorkflowMode::MediaMetadataJson => {
                "No media rewrite/export is available for metadata-only review.".to_string()
            }
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
    fn media_metadata_mode_uses_bounded_runtime_route_and_copy() {
        assert!(DesktopWorkflowMode::ALL.contains(&DesktopWorkflowMode::MediaMetadataJson));
        assert_eq!(
            DesktopWorkflowMode::MediaMetadataJson.label(),
            "Media metadata JSON"
        );
        assert_eq!(
            DesktopWorkflowMode::MediaMetadataJson.route(),
            "/media/conservative/deidentify"
        );
        assert!(DesktopWorkflowMode::MediaMetadataJson
            .payload_hint()
            .contains("media metadata JSON"));
        assert!(DesktopWorkflowMode::MediaMetadataJson
            .disclosure()
            .contains("metadata-only"));
        assert!(DesktopWorkflowMode::MediaMetadataJson
            .disclosure()
            .contains("does not upload media bytes"));
        assert!(DesktopWorkflowMode::MediaMetadataJson
            .disclosure()
            .contains("no OCR"));
    }

    #[test]
    fn json_file_import_uses_media_metadata_mode_without_media_bytes() {
        let imported = DesktopFileImportPayload::from_bytes(
            "local-media-metadata.json",
            b"{\"artifact_label\":\"scan.png\",\"format\":\"image\",\"metadata\":[{\"key\":\"PatientName\",\"value\":\"Jane Patient\"}],\"ocr_or_visual_review_required\":true}",
        )
        .expect("json metadata imports should be accepted");

        assert_eq!(imported.mode, DesktopWorkflowMode::MediaMetadataJson);
        assert_eq!(
            imported.source_name.as_deref(),
            Some("local-media-metadata.json")
        );
        assert!(imported.payload.contains("PatientName"));
    }

    #[test]
    fn media_metadata_request_uses_raw_json_body_and_rejects_non_objects() {
        let valid = DesktopWorkflowRequestState {
            mode: DesktopWorkflowMode::MediaMetadataJson,
            payload: "{\"artifact_label\":\"scan.png\",\"format\":\"image\",\"metadata\":[],\"ocr_or_visual_review_required\":false}".to_string(),
            field_policy_json: "{\"PatientName\":\"redact\"}".to_string(),
            source_name: "local-media-metadata.json".to_string(),
        };

        let request = valid
            .try_build_request()
            .expect("valid metadata object should build");
        assert_eq!(request.route, "/media/conservative/deidentify");
        let body = serde_json::to_string(&request.body).expect("request body serializes");
        assert!(body.contains(r#""artifact_label":"scan.png""#));
        assert!(!body.contains("field_policies"));

        let invalid = DesktopWorkflowRequestState {
            mode: DesktopWorkflowMode::MediaMetadataJson,
            payload: "[]".to_string(),
            field_policy_json: "{}".to_string(),
            source_name: "local-media-metadata.json".to_string(),
        };

        assert_eq!(
            invalid.try_build_request(),
            Err(DesktopWorkflowValidationError::InvalidMediaMetadataJson)
        );
    }

    #[test]
    fn invalid_media_metadata_status_uses_exact_validation_message() {
        let state = DesktopWorkflowRequestState {
            mode: DesktopWorkflowMode::MediaMetadataJson,
            payload: "[]".to_string(),
            field_policy_json: "{}".to_string(),
            source_name: "local-media-metadata.json".to_string(),
        };

        assert_eq!(
            state.status_message(),
            "Not ready: Media metadata JSON must be a JSON object accepted by the local media review runtime route."
        );
    }

    #[test]
    fn portable_mode_routes_match_existing_runtime_routes() {
        assert_eq!(DesktopPortableMode::VaultExport.route(), "/vault/export");
        assert_eq!(
            DesktopPortableMode::InspectArtifact.route(),
            "/portable-artifacts/inspect"
        );
        assert_eq!(
            DesktopPortableMode::ImportArtifact.route(),
            "/portable-artifacts/import"
        );
        assert!(DesktopPortableMode::VaultExport
            .disclosure()
            .contains("bounded"));
        assert!(!DesktopPortableMode::VaultExport
            .disclosure()
            .contains("controller"));
    }

    #[test]
    fn portable_export_request_builds_runtime_envelope() {
        let state = DesktopPortableRequestState {
            mode: DesktopPortableMode::VaultExport,
            vault_path: "/safe/local.vault".to_string(),
            vault_passphrase: "vault-secret".to_string(),
            record_ids_json: "[\"550e8400-e29b-41d4-a716-446655440000\",\"550e8400-e29b-41d4-a716-446655440001\"]".to_string(),
            export_passphrase: "portable-secret".to_string(),
            export_context: "handoff to privacy office".to_string(),
            artifact_json: String::new(),
            portable_passphrase: String::new(),
            destination_vault_path: String::new(),
            destination_vault_passphrase: String::new(),
            import_context: String::new(),
            requested_by: "desktop".to_string(),
        };

        let request = state.try_build_request().unwrap();

        assert_eq!(request.route, "/vault/export");
        assert_eq!(request.body["vault_path"], "/safe/local.vault");
        assert_eq!(request.body["vault_passphrase"], "vault-secret");
        assert_eq!(
            request.body["record_ids"],
            json!([
                "550e8400-e29b-41d4-a716-446655440000",
                "550e8400-e29b-41d4-a716-446655440001"
            ])
        );
        assert_eq!(request.body["export_passphrase"], "portable-secret");
        assert_eq!(request.body["context"], "handoff to privacy office");
        assert_eq!(request.body["requested_by"], "desktop");
        assert!(request.body.get("export_context").is_none());
    }

    #[test]
    fn portable_inspect_request_builds_runtime_envelope() {
        let state = DesktopPortableRequestState {
            mode: DesktopPortableMode::InspectArtifact,
            artifact_json: "{\"version\":1}".to_string(),
            portable_passphrase: "portable-secret".to_string(),
            ..DesktopPortableRequestState::default()
        };

        let request = state.try_build_request().unwrap();

        assert_eq!(request.route, "/portable-artifacts/inspect");
        assert_eq!(
            request.body,
            json!({"artifact":{"version":1},"portable_passphrase":"portable-secret"})
        );
    }

    #[test]
    fn portable_import_request_builds_runtime_envelope() {
        let state = DesktopPortableRequestState {
            mode: DesktopPortableMode::ImportArtifact,
            destination_vault_path: "/safe/target.vault".to_string(),
            destination_vault_passphrase: "target-secret".to_string(),
            artifact_json: "{\"version\":1}".to_string(),
            portable_passphrase: "portable-secret".to_string(),
            import_context: "restore approved records".to_string(),
            ..DesktopPortableRequestState::default()
        };

        let request = state.try_build_request().unwrap();

        assert_eq!(request.route, "/portable-artifacts/import");
        assert_eq!(request.body["vault_path"], "/safe/target.vault");
        assert_eq!(request.body["vault_passphrase"], "target-secret");
        assert_eq!(request.body["artifact"], json!({"version":1}));
        assert_eq!(request.body["portable_passphrase"], "portable-secret");
        assert_eq!(request.body["context"], "restore approved records");
        assert_eq!(request.body["requested_by"], "desktop");
        assert!(request.body.get("import_context").is_none());
    }

    #[test]
    fn portable_export_validation_rejects_non_uuid_record_ids() {
        let state = DesktopPortableRequestState {
            mode: DesktopPortableMode::VaultExport,
            vault_path: "/safe/local.vault".to_string(),
            vault_passphrase: "vault-secret".to_string(),
            record_ids_json: "[\"record-1\"]".to_string(),
            export_passphrase: "portable-secret".to_string(),
            export_context: "handoff to privacy office".to_string(),
            ..DesktopPortableRequestState::default()
        };

        assert!(matches!(
            state.try_build_request(),
            Err(DesktopPortableValidationError::InvalidRecordIdsJson(_))
        ));
    }

    #[test]
    fn portable_export_validation_rejects_empty_record_ids() {
        let state = DesktopPortableRequestState {
            mode: DesktopPortableMode::VaultExport,
            vault_path: "/safe/local.vault".to_string(),
            vault_passphrase: "vault-secret".to_string(),
            record_ids_json: "[]".to_string(),
            export_passphrase: "portable-secret".to_string(),
            export_context: "handoff to privacy office".to_string(),
            requested_by: "desktop".to_string(),
            ..DesktopPortableRequestState::default()
        };

        assert_eq!(
            state.try_build_request(),
            Err(DesktopPortableValidationError::EmptyRecordIds)
        );
    }

    #[test]
    fn portable_request_validation_rejects_blank_required_fields() {
        let state = DesktopPortableRequestState::default();
        assert_eq!(
            state.try_build_request(),
            Err(DesktopPortableValidationError::BlankVaultPath)
        );

        let inspect = DesktopPortableRequestState {
            mode: DesktopPortableMode::InspectArtifact,
            artifact_json: "{\"version\":1}".to_string(),
            ..DesktopPortableRequestState::default()
        };
        assert_eq!(
            inspect.try_build_request(),
            Err(DesktopPortableValidationError::BlankPortablePassphrase)
        );

        let import = DesktopPortableRequestState {
            mode: DesktopPortableMode::ImportArtifact,
            destination_vault_path: "/safe/target.vault".to_string(),
            destination_vault_passphrase: "target-secret".to_string(),
            portable_passphrase: "portable-secret".to_string(),
            ..DesktopPortableRequestState::default()
        };
        assert_eq!(
            import.try_build_request(),
            Err(DesktopPortableValidationError::BlankArtifactJson)
        );
    }

    #[test]
    fn portable_request_debug_redacts_passphrases_and_artifact() {
        let state = DesktopPortableRequestState {
            vault_passphrase: "vault-secret".to_string(),
            export_passphrase: "portable-export-secret".to_string(),
            portable_passphrase: "portable-secret".to_string(),
            artifact_json: "{\"patient\":\"Alice\"}".to_string(),
            ..DesktopPortableRequestState::default()
        };

        let debug = format!("{state:?}");

        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("vault-secret"));
        assert!(!debug.contains("portable-export-secret"));
        assert!(!debug.contains("portable-secret"));
        assert!(!debug.contains("Alice"));
    }

    #[test]
    fn portable_workflow_request_debug_redacts_sensitive_request_body_fields() {
        let export = DesktopPortableRequestState {
            mode: DesktopPortableMode::VaultExport,
            vault_path: "/safe/local.vault".to_string(),
            vault_passphrase: "vault-secret".to_string(),
            record_ids_json: r#"["550e8400-e29b-41d4-a716-446655440000"]"#.to_string(),
            export_passphrase: "export-secret".to_string(),
            export_context: "handoff to privacy office".to_string(),
            ..DesktopPortableRequestState::default()
        }
        .try_build_request()
        .unwrap();
        let import = DesktopPortableRequestState {
            mode: DesktopPortableMode::ImportArtifact,
            destination_vault_path: "/safe/target.vault".to_string(),
            destination_vault_passphrase: "target-secret".to_string(),
            artifact_json: "{\"patient\":\"Alice\"}".to_string(),
            portable_passphrase: "portable-secret".to_string(),
            import_context: "restore approved records".to_string(),
            ..DesktopPortableRequestState::default()
        }
        .try_build_request()
        .unwrap();
        let inspect = DesktopPortableRequestState {
            mode: DesktopPortableMode::InspectArtifact,
            artifact_json: "{\"patient\":\"Bob\"}".to_string(),
            portable_passphrase: "inspect-secret".to_string(),
            ..DesktopPortableRequestState::default()
        }
        .try_build_request()
        .unwrap();

        for debug in [
            format!("{export:?}"),
            format!("{import:?}"),
            format!("{inspect:?}"),
        ] {
            assert!(debug.contains("<redacted>"));
            assert!(!debug.contains("export-secret"));
            assert!(!debug.contains("portable-secret"));
            assert!(!debug.contains("inspect-secret"));
            assert!(!debug.contains("vault-secret"));
            assert!(!debug.contains("target-secret"));
            assert!(!debug.contains("Alice"));
            assert!(!debug.contains("Bob"));
        }
    }

    #[test]
    fn vault_response_state_renders_decode_summary_without_decoded_values() {
        let mut state = DesktopVaultResponseState::default();
        let response = serde_json::json!({
            "decoded_value_count": 2,
            "report_path": "/tmp/Alice-Smith-decode-report.json",
            "audit_event": {"kind": "decode", "detail": "patient Alice decoded for oncology"},
            "decoded_values": [{"original_value": "Alice Smith", "token": format!("PHI-TOKEN-{}", 1)}]
        });

        state.apply_success(DesktopVaultResponseMode::VaultDecode, &response);

        assert!(state.banner.contains("bounded vault decode response"));
        assert!(state.summary.contains("decoded values: 2"));
        assert!(state
            .artifact_notice
            .contains("artifact path returned; full path hidden"));
        let rendered = format!(
            "{} {} {}",
            state.banner, state.summary, state.artifact_notice
        );
        assert!(!rendered.contains("Alice Smith"));
        assert!(!rendered.contains("Alice-Smith"));
        assert!(!rendered.contains("/tmp/Alice-Smith-decode-report.json"));
        assert!(!rendered.contains("patient Alice"));
        assert!(!rendered.contains("PHI-TOKEN-1"));
    }

    #[test]
    fn vault_response_state_renders_audit_counts_without_raw_details() {
        let mut state = DesktopVaultResponseState::default();
        let response = serde_json::json!({
            "event_count": 200,
            "returned_event_count": 100,
            "events": [
                {"kind": "decode", "detail": "patient Bob release"},
                {"kind": "encode", "detail": "encoded patient Carol"}
            ]
        });

        state.apply_success(DesktopVaultResponseMode::VaultAudit, &response);

        assert!(state.banner.contains("bounded vault audit response"));
        assert!(state.summary.contains("events returned: 100 / 200"));
        let rendered = format!(
            "{} {} {}",
            state.banner, state.summary, state.artifact_notice
        );
        assert!(!rendered.contains("patient Bob"));
        assert!(!rendered.contains("patient Carol"));
    }

    #[test]
    fn vault_response_state_renders_portable_artifact_without_raw_artifact_json() {
        let mut state = DesktopVaultResponseState::default();
        let response = serde_json::json!({
            "artifact_path": "/tmp/MRN-123-portable-artifact.json",
            "record_count": 3,
            "artifact_json": {"records": [{"original_value": "MRN-123"}]},
            "imported_record_count": 3
        });

        state.apply_success(DesktopVaultResponseMode::VaultExport, &response);
        assert!(state.banner.contains("bounded portable artifact response"));
        assert!(state.summary.contains("records: 3"));
        assert!(state
            .artifact_notice
            .contains("artifact path returned; full path hidden"));

        state.apply_success(DesktopVaultResponseMode::ImportArtifact, &response);
        assert!(state.summary.contains("imported records: 3"));

        let rendered = format!(
            "{} {} {}",
            state.banner, state.summary, state.artifact_notice
        );
        assert!(!rendered.contains("MRN-123"));
        assert!(!rendered.contains("/tmp/MRN-123-portable-artifact.json"));
    }

    #[test]
    fn vault_response_state_records_error_without_stale_phi() {
        let mut state = DesktopVaultResponseState::default();
        let response = serde_json::json!({"decoded_value_count": 1, "report_path": "/tmp/safe.json", "decoded_values": [{"original_value": "Alice Smith"}]});
        state.apply_success(DesktopVaultResponseMode::VaultDecode, &response);

        state.apply_error(
            DesktopVaultResponseMode::VaultDecode,
            "runtime failed for patient Alice Smith",
        );

        assert!(state.banner.contains("bounded vault decode response"));
        assert!(state
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("runtime failed"));
        assert!(!state
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("patient Alice Smith"));
        assert!(state.summary.is_empty());
        assert!(state.artifact_notice.is_empty());
    }

    #[test]
    fn vault_response_state_redacts_arbitrary_runtime_error_content() {
        let mut state = DesktopVaultResponseState::default();

        state.apply_error(
            DesktopVaultResponseMode::VaultAudit,
            "unable to process MRN-123 Alice Smith",
        );

        let error = state.error.as_deref().unwrap_or_default();
        assert!(error.contains("runtime failed"));
        assert!(!error.contains("MRN-123"));
        assert!(!error.contains("Alice Smith"));
    }

    #[test]
    fn desktop_vault_decode_request_builds_existing_runtime_contract() {
        let state = DesktopVaultRequestState {
            mode: DesktopVaultMode::Decode,
            vault_path: "C:/vaults/local.mdid".to_string(),
            vault_passphrase: " correct horse battery staple ".to_string(),
            record_ids_json: r#"["550e8400-e29b-41d4-a716-446655440000"]"#.to_string(),
            output_target: "review-workbench".to_string(),
            justification: "incident review".to_string(),
            requested_by: "desktop".to_string(),
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
            " correct horse battery staple "
        );
        assert_eq!(
            request.body["record_ids"][0],
            "550e8400-e29b-41d4-a716-446655440000"
        );
        assert_eq!(request.body["output_target"], "review-workbench");
        assert_eq!(request.body["justification"], "incident review");
        assert_eq!(request.body["requested_by"], "desktop");
    }

    #[test]
    fn desktop_vault_audit_request_builds_read_only_filter_contract() {
        let state = DesktopVaultRequestState {
            mode: DesktopVaultMode::AuditEvents,
            vault_path: "C:/vaults/local.mdid".to_string(),
            vault_passphrase: "correct horse battery staple".to_string(),
            record_ids_json: "[]".to_string(),
            output_target: "review-workbench".to_string(),
            justification: "desktop audit review".to_string(),
            requested_by: "desktop".to_string(),
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
        assert_eq!(request.body["kind"], "decode");
        assert_eq!(request.body["actor"], "desktop");
        assert!(request.body.get("record_ids").is_none());
    }

    #[test]
    fn desktop_vault_request_state_debug_redacts_passphrase() {
        let state = DesktopVaultRequestState {
            vault_passphrase: "correct horse battery staple".to_string(),
            ..DesktopVaultRequestState::default()
        };

        let debug = format!("{state:?}");

        assert!(debug.contains("vault_passphrase: \"<redacted>\""));
        assert!(!debug.contains("correct horse battery staple"));
    }

    #[test]
    fn desktop_vault_workflow_request_debug_redacts_passphrase_after_build() {
        let state = DesktopVaultRequestState {
            mode: DesktopVaultMode::Decode,
            vault_path: "C:/vaults/local.mdid".to_string(),
            vault_passphrase: "super secret passphrase".to_string(),
            record_ids_json: r#"["550e8400-e29b-41d4-a716-446655440000"]"#.to_string(),
            output_target: "review-workbench".to_string(),
            justification: "incident review".to_string(),
            requested_by: "desktop".to_string(),
            audit_kind: None,
            audit_actor: None,
        };

        let request = state
            .try_build_request()
            .expect("decode request should build");
        let debug = format!("{request:?}");

        assert!(debug.contains("/vault/decode"));
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("super secret passphrase"));
        assert_eq!(request.body["vault_passphrase"], "super secret passphrase");
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
        state.output_target = "   ".to_string();
        assert_eq!(
            state.try_build_request(),
            Err(DesktopVaultValidationError::BlankOutputTarget)
        );

        state.output_target = "review-workbench".to_string();
        assert_eq!(
            state.try_build_request(),
            Err(DesktopVaultValidationError::EmptyRecordIds)
        );

        state.record_ids_json = "not json".to_string();
        assert!(matches!(
            state.try_build_request(),
            Err(DesktopVaultValidationError::InvalidRecordIdsJson(_))
        ));
    }

    #[test]
    fn desktop_vault_decode_validation_rejects_blank_justification() {
        let state = DesktopVaultRequestState {
            mode: DesktopVaultMode::Decode,
            vault_path: "C:/vaults/local.mdid".to_string(),
            vault_passphrase: "correct horse battery staple".to_string(),
            record_ids_json: r#"["550e8400-e29b-41d4-a716-446655440000"]"#.to_string(),
            output_target: "review-workbench".to_string(),
            justification: "   ".to_string(),
            requested_by: "desktop".to_string(),
            audit_kind: None,
            audit_actor: None,
        };

        assert_eq!(
            state.try_build_request(),
            Err(DesktopVaultValidationError::BlankJustification)
        );
    }

    #[test]
    fn desktop_vault_workbench_copy_is_bounded_and_non_orchestrating() {
        assert!(DESKTOP_VAULT_WORKBENCH_COPY.contains("existing localhost runtime vault routes"));
        assert!(DESKTOP_VAULT_WORKBENCH_COPY.contains("does not persist passphrases"));
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
    fn desktop_runtime_client_accepts_portable_inspect_request_envelope() {
        let state = DesktopPortableRequestState {
            mode: DesktopPortableMode::InspectArtifact,
            artifact_json: "{\"version\":1}".to_string(),
            portable_passphrase: "portable-secret".to_string(),
            ..DesktopPortableRequestState::default()
        };
        let request = state.try_build_request().expect("valid portable request");

        let http = DesktopRuntimeClient::new("127.0.0.1", 8787)
            .expect("valid local client")
            .build_http_request(&request)
            .expect("portable inspect route accepted");

        assert!(http.starts_with("POST /portable-artifacts/inspect HTTP/1.1\r\n"));
        let body = http
            .split_once("\r\n\r\n")
            .expect("HTTP request has header/body separator")
            .1;
        let body_json: serde_json::Value = serde_json::from_str(body).expect("JSON body");
        assert_eq!(body_json, request.body);
    }

    #[test]
    fn desktop_runtime_route_validation_accepts_portable_routes() {
        for mode in [
            DesktopPortableMode::VaultExport,
            DesktopPortableMode::InspectArtifact,
            DesktopPortableMode::ImportArtifact,
        ] {
            DesktopRuntimeClient::validate_runtime_route(mode.route())
                .expect("portable route is approved for desktop runtime client");
        }
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
