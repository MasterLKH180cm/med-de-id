use base64::Engine as _;

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

#[derive(Clone, PartialEq, Eq)]
pub enum DesktopFileImportTarget {
    Workflow(DesktopFileImportPayload),
    PortableArtifactInspect(DesktopPortableFileImportPayload),
}

impl std::fmt::Debug for DesktopFileImportTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Workflow(payload) => f.debug_tuple("Workflow").field(payload).finish(),
            Self::PortableArtifactInspect(payload) => f
                .debug_tuple("PortableArtifactInspect")
                .field(payload)
                .finish(),
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct DesktopPortableFileImportPayload {
    pub mode: DesktopPortableMode,
    pub artifact_json: String,
    pub source_name: String,
}

impl std::fmt::Debug for DesktopPortableFileImportPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DesktopPortableFileImportPayload")
            .field("mode", &self.mode)
            .field("artifact_json", &"<redacted>")
            .field("source_name", &"<redacted>")
            .finish()
    }
}

impl DesktopPortableFileImportPayload {
    pub fn from_bytes_for_mode(
        mode: DesktopPortableMode,
        source_name: impl Into<String>,
        bytes: &[u8],
    ) -> Result<Self, DesktopFileImportError> {
        let source_name = source_name.into();
        if !matches!(
            mode,
            DesktopPortableMode::InspectArtifact | DesktopPortableMode::ImportArtifact
        ) {
            return Err(DesktopFileImportError::UnsupportedFileType);
        }
        if !is_portable_artifact_json_filename(&source_name) {
            return Err(DesktopFileImportError::UnsupportedFileType);
        }
        if bytes.len() > DESKTOP_FILE_IMPORT_MAX_BYTES {
            return Err(DesktopFileImportError::FileTooLarge);
        }
        let artifact_json = std::str::from_utf8(bytes)
            .map_err(|_| DesktopFileImportError::InvalidCsvUtf8)?
            .to_string();

        Ok(Self {
            mode,
            artifact_json,
            source_name,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopFileImportError {
    UnsupportedFileType,
    FileTooLarge,
    InvalidCsvUtf8,
}

impl DesktopFileImportPayload {
    pub fn from_bytes_target(
        source_name: impl Into<String>,
        bytes: &[u8],
    ) -> Result<DesktopFileImportTarget, DesktopFileImportError> {
        let source_name = source_name.into();
        if is_portable_artifact_json_filename(&source_name) {
            if bytes.len() > DESKTOP_FILE_IMPORT_MAX_BYTES {
                return Err(DesktopFileImportError::FileTooLarge);
            }
            let artifact_json = std::str::from_utf8(bytes)
                .map_err(|_| DesktopFileImportError::InvalidCsvUtf8)?
                .to_string();
            return Ok(DesktopFileImportTarget::PortableArtifactInspect(
                DesktopPortableFileImportPayload {
                    mode: DesktopPortableMode::InspectArtifact,
                    artifact_json,
                    source_name,
                },
            ));
        }

        Self::from_bytes(source_name, bytes).map(DesktopFileImportTarget::Workflow)
    }

    pub fn from_bytes(
        source_name: impl Into<String>,
        bytes: &[u8],
    ) -> Result<Self, DesktopFileImportError> {
        if bytes.len() > DESKTOP_FILE_IMPORT_MAX_BYTES {
            return Err(DesktopFileImportError::FileTooLarge);
        }

        let source_name = source_name.into();
        if is_portable_artifact_json_filename(&source_name) {
            return Err(DesktopFileImportError::UnsupportedFileType);
        }
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
                source_name: Some(source_name),
            }),
            "xlsx" => Ok(Self {
                mode: DesktopWorkflowMode::XlsxBase64,
                payload: encode_base64(bytes),
                source_name: Some(source_name),
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

fn is_portable_artifact_json_filename(source_name: &str) -> bool {
    let filename = source_name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(source_name)
        .to_ascii_lowercase();

    filename == "mdid-browser-portable-artifact.json"
        || filename.ends_with(".mdid-portable.json")
        || filename.ends_with("-mdid-portable.json")
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

fn decode_base64(value: &str) -> Option<Vec<u8>> {
    base64::engine::general_purpose::STANDARD
        .decode(value.trim())
        .ok()
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
            Self::CsvText => "CSV text de-identification uses the bounded local runtime route /tabular/deidentify; it stays limited to this local de-identification request surface.",
            Self::XlsxBase64 => "XLSX base64 de-identification uses the bounded local runtime route /tabular/deidentify/xlsx; it processes the first non-empty worksheet only. Sheet selection is not supported in this desktop flow; it stays limited to this local de-identification request surface.",
            Self::PdfBase64Review => "PDF base64 review uses the bounded local runtime route /pdf/deidentify; it stays limited to this local review request surface and includes no OCR/PDF rewrite.",
            Self::DicomBase64 => "DICOM base64 de-identification uses the bounded local runtime route /dicom/deidentify for tag-level DICOM de-identification. DICOM pixel data was not inspected or redacted; burned-in annotations require separate visual review. It stays limited to this local de-identification request surface.",
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

pub const DESKTOP_VAULT_WORKBENCH_COPY: &str = "Bounded desktop vault workbench: prepares request envelopes for existing localhost runtime vault routes, including explicit decode and read-only audit browsing. It does not persist passphrases, browse vault contents directly, transfer portable artifacts, or add unrelated background workflow behavior.";

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
            Self::VaultExport => "bounded desktop portable export request preparation for the existing local /vault/export runtime route; no unrelated background workflow behavior is included.",
            Self::InspectArtifact => "bounded desktop portable artifact inspection request preparation for the existing local /portable-artifacts/inspect runtime route; no unrelated background workflow behavior is included.",
            Self::ImportArtifact => "bounded desktop portable artifact import request preparation for the existing local /portable-artifacts/import runtime route; no unrelated background workflow behavior is included.",
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
    let mut seen_record_ids = std::collections::HashSet::with_capacity(record_ids.len());
    for record_id in &record_ids {
        if record_id.trim().is_empty() {
            return Err(error("record id must not be blank".to_string()));
        }
        let record_id = uuid::Uuid::parse_str(record_id)
            .map_err(|parse_error| error(parse_error.to_string()))?;
        if !seen_record_ids.insert(record_id) {
            return Err(error("duplicate record id is not allowed".to_string()));
        }
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
    pub const ALL: [Self; 2] = [Self::Decode, Self::AuditEvents];

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
    pub audit_limit: Option<String>,
    pub audit_offset: Option<String>,
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
            .field("audit_limit", &self.audit_limit)
            .field("audit_offset", &self.audit_offset)
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
            audit_limit: None,
            audit_offset: None,
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
                let mut seen_record_ids =
                    std::collections::HashSet::with_capacity(record_ids.len());
                for record_id in &record_ids {
                    if !seen_record_ids.insert(*record_id) {
                        return Err(DesktopVaultValidationError::InvalidRecordIdsJson(
                            "duplicate record id is not allowed".to_string(),
                        ));
                    }
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
            DesktopVaultMode::AuditEvents => {
                let limit = parse_optional_positive_usize(
                    self.audit_limit.as_deref(),
                    DesktopVaultValidationError::InvalidAuditLimit,
                    DesktopVaultValidationError::ZeroAuditLimit,
                )?;
                let offset = parse_optional_non_negative_usize(
                    self.audit_offset.as_deref(),
                    DesktopVaultValidationError::InvalidAuditOffset,
                )?;
                let mut body = serde_json::json!({
                    "vault_path": vault_path,
                    "vault_passphrase": self.vault_passphrase.clone(),
                    "kind": lowercase_optional_filter(self.audit_kind.as_deref()),
                    "actor": lowercase_optional_filter(self.audit_actor.as_deref()),
                });
                if let Some(limit) = limit {
                    body["limit"] = serde_json::json!(limit);
                }
                if let Some(offset) = offset.filter(|offset| *offset > 0) {
                    body["offset"] = serde_json::json!(offset);
                }
                body
            }
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

fn parse_optional_positive_usize<E>(
    value: Option<&str>,
    invalid: fn(String) -> E,
    zero: E,
) -> Result<Option<usize>, E> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    let parsed = value
        .parse::<usize>()
        .map_err(|error| invalid(error.to_string()))?;
    if parsed == 0 {
        return Err(zero);
    }
    Ok(Some(parsed))
}

fn parse_optional_non_negative_usize<E>(
    value: Option<&str>,
    invalid: fn(String) -> E,
) -> Result<Option<usize>, E> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if value.starts_with('-') {
        return Err(invalid("negative values are not allowed".to_string()));
    }
    value
        .parse::<usize>()
        .map(Some)
        .map_err(|_| invalid("expected non-negative integer".to_string()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopVaultValidationError {
    BlankVaultPath,
    BlankVaultPassphrase,
    BlankOutputTarget,
    BlankJustification,
    EmptyRecordIds,
    InvalidRecordIdsJson(String),
    InvalidAuditLimit(String),
    ZeroAuditLimit,
    InvalidAuditOffset(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopVaultResponseMode {
    VaultDecode,
    VaultAudit,
    VaultExport,
    InspectArtifact,
    ImportArtifact,
}

impl DesktopVaultResponseMode {
    fn safe_export_label(self) -> &'static str {
        match self {
            Self::VaultDecode => "vault_decode",
            Self::VaultAudit => "vault_audit",
            Self::VaultExport => "vault_export",
            Self::InspectArtifact => "portable_artifact_inspect",
            Self::ImportArtifact => "portable_artifact_import",
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct DesktopVaultResponseState {
    pub banner: String,
    pub error: Option<String>,
    pub summary: String,
    pub artifact_notice: String,
    rendered_mode: Option<DesktopVaultResponseMode>,
    last_success_mode: Option<DesktopVaultResponseMode>,
    last_success_response: Option<serde_json::Value>,
}

impl Default for DesktopVaultResponseState {
    fn default() -> Self {
        Self {
            banner: "No bounded vault or portable response rendered yet.".to_string(),
            error: None,
            summary: String::new(),
            artifact_notice: String::new(),
            rendered_mode: None,
            last_success_mode: None,
            last_success_response: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopPortableArtifactSaveError {
    NotVaultExport,
    MissingArtifact,
    Io(String),
    InvalidJson(String),
}

impl std::fmt::Display for DesktopPortableArtifactSaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotVaultExport => write!(
                f,
                "portable artifact save is only available for vault export responses"
            ),
            Self::MissingArtifact => write!(
                f,
                "safe response report or portable artifact is unavailable"
            ),
            Self::Io(_) => write!(f, "portable artifact JSON could not be written"),
            Self::InvalidJson(_) => write!(f, "portable artifact JSON could not be prepared"),
        }
    }
}

impl std::error::Error for DesktopPortableArtifactSaveError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopDecodedValuesExportError {
    NotVaultDecode,
    MissingDecodedValues,
    Io(String),
    InvalidJson(String),
}

impl std::fmt::Display for DesktopDecodedValuesExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotVaultDecode => write!(
                f,
                "decoded values export is only available for successful vault decode responses"
            ),
            Self::MissingDecodedValues => write!(f, "decoded values are unavailable"),
            Self::Io(_) => write!(f, "decoded values JSON could not be written"),
            Self::InvalidJson(_) => write!(f, "decoded values JSON could not be prepared"),
        }
    }
}

impl std::error::Error for DesktopDecodedValuesExportError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopAuditEventsExportError {
    NotVaultAudit,
    MissingAuditEvents,
    Io(String),
    InvalidJson(String),
}

impl std::fmt::Display for DesktopAuditEventsExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotVaultAudit => write!(
                f,
                "audit events export is only available for successful vault audit responses"
            ),
            Self::MissingAuditEvents => write!(f, "audit events are unavailable"),
            Self::Io(_) => write!(f, "audit events JSON could not be written"),
            Self::InvalidJson(_) => write!(f, "audit events JSON could not be prepared"),
        }
    }
}

impl std::error::Error for DesktopAuditEventsExportError {}

#[derive(Clone, PartialEq, Eq)]
pub struct DesktopVaultResponseReportDownload {
    pub file_name: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopPortableReportSavePayload {
    pub suggested_file_name: String,
    pub mime_type: &'static str,
    pub contents: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopPortableReportSaveError {
    InvalidResponseJson,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopPdfReviewReportSave {
    pub file_name: String,
    pub contents: String,
    pub status: String,
}

impl std::fmt::Display for DesktopPortableReportSaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidResponseJson => write!(f, "portable response JSON must be an object"),
        }
    }
}

impl std::error::Error for DesktopPortableReportSaveError {}

pub fn build_desktop_pdf_review_report_save(
    response_json: &str,
    source_name: Option<&str>,
) -> Result<DesktopPdfReviewReportSave, String> {
    let response: serde_json::Value = serde_json::from_str(response_json)
        .map_err(|_| "PDF review report requires a JSON object response".to_string())?;
    let object = response
        .as_object()
        .ok_or_else(|| "PDF review report requires a JSON object response".to_string())?;

    let report = serde_json::json!({
        "mode": "pdf_review_report",
        "summary": sanitize_desktop_pdf_review_report_summary(object.get("summary")),
        "review_queue": sanitize_desktop_pdf_review_report_queue(object.get("review_queue")),
    });
    let contents = serde_json::to_string_pretty(&report)
        .map_err(|_| "PDF review report could not be prepared".to_string())?;
    let stem = source_name
        .and_then(portable_report_source_stem)
        .unwrap_or_else(|| "mdid-desktop-pdf".to_string());

    Ok(DesktopPdfReviewReportSave {
        file_name: format!("{stem}-pdf-review-report.json"),
        contents,
        status: "PDF review report ready to save; text content and PDF bytes are redacted from this report.".to_string(),
    })
}

fn sanitize_desktop_pdf_review_report_summary(
    value: Option<&serde_json::Value>,
) -> serde_json::Value {
    let Some(serde_json::Value::Object(object)) = value else {
        return serde_json::json!({});
    };
    let mut sanitized = serde_json::Map::new();
    for key in [
        "total_pages",
        "pages_with_text",
        "ocr_required_pages",
        "candidate_count",
        "requires_ocr",
        "status",
    ] {
        if let Some(value) = object.get(key).filter(|value| {
            matches!(
                value,
                serde_json::Value::Null
                    | serde_json::Value::Bool(_)
                    | serde_json::Value::Number(_)
                    | serde_json::Value::String(_)
            )
        }) {
            sanitized.insert(key.to_string(), value.clone());
        }
    }
    serde_json::Value::Object(sanitized)
}

fn sanitize_desktop_pdf_review_report_queue(
    value: Option<&serde_json::Value>,
) -> serde_json::Value {
    let Some(serde_json::Value::Array(items)) = value else {
        return serde_json::json!([]);
    };
    serde_json::Value::Array(
        items
            .iter()
            .filter_map(|item| {
                let object = item.as_object()?;
                let mut sanitized = serde_json::Map::new();
                if let Some(page) = object.get("page").filter(|value| is_json_primitive(value)) {
                    sanitized.insert("page".to_string(), page.clone());
                }
                if let Some(kind) = object.get("kind").filter(|value| is_json_primitive(value)) {
                    sanitized.insert("kind".to_string(), kind.clone());
                }
                if let Some(status) = object
                    .get("status")
                    .filter(|value| is_json_primitive(value))
                {
                    sanitized.insert("status".to_string(), status.clone());
                }
                Some(serde_json::Value::Object(sanitized))
            })
            .collect(),
    )
}

fn is_json_primitive(value: &serde_json::Value) -> bool {
    matches!(
        value,
        serde_json::Value::Null
            | serde_json::Value::Bool(_)
            | serde_json::Value::Number(_)
            | serde_json::Value::String(_)
    )
}

impl std::fmt::Debug for DesktopVaultResponseReportDownload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DesktopVaultResponseReportDownload")
            .field("file_name", &self.file_name)
            .field("bytes", &"<redacted>")
            .finish()
    }
}

pub fn build_desktop_portable_response_report_save(
    mode: DesktopPortableMode,
    imported_file_name: Option<&str>,
    response_json: &str,
) -> Result<DesktopPortableReportSavePayload, DesktopPortableReportSaveError> {
    let mut report: serde_json::Value = serde_json::from_str(response_json)
        .map_err(|_| DesktopPortableReportSaveError::InvalidResponseJson)?;
    let object = report
        .as_object_mut()
        .ok_or(DesktopPortableReportSaveError::InvalidResponseJson)?;
    object.insert(
        "mode".to_string(),
        serde_json::Value::String(portable_report_mode(mode).to_string()),
    );
    redact_portable_report_value(&mut report);

    let contents = serde_json::to_string_pretty(&report)
        .map_err(|_| DesktopPortableReportSaveError::InvalidResponseJson)?;
    let operation = portable_report_operation(mode);
    let suggested_file_name = imported_file_name
        .and_then(portable_report_source_stem)
        .filter(|stem| !stem.is_empty())
        .map(|stem| format!("{stem}-portable-artifact-{operation}-report.json"))
        .unwrap_or_else(|| "desktop-portable-artifact-report.json".to_string());

    Ok(DesktopPortableReportSavePayload {
        suggested_file_name,
        mime_type: "application/json",
        contents,
        status: portable_report_status(mode).to_string(),
    })
}

fn portable_report_mode(mode: DesktopPortableMode) -> &'static str {
    match mode {
        DesktopPortableMode::VaultExport => "portable_artifact_export",
        DesktopPortableMode::InspectArtifact => "portable_artifact_inspect",
        DesktopPortableMode::ImportArtifact => "portable_artifact_import",
    }
}

fn portable_report_operation(mode: DesktopPortableMode) -> &'static str {
    match mode {
        DesktopPortableMode::VaultExport => "export",
        DesktopPortableMode::InspectArtifact => "inspect",
        DesktopPortableMode::ImportArtifact => "import",
    }
}

fn portable_report_status(mode: DesktopPortableMode) -> &'static str {
    match mode {
        DesktopPortableMode::VaultExport => "Portable artifact export report ready to save; artifact and decoded values are redacted from this report.",
        DesktopPortableMode::InspectArtifact => "Portable artifact inspect report ready to save; artifact and decoded values are redacted from this report.",
        DesktopPortableMode::ImportArtifact => "Portable artifact import report ready to save; artifact and decoded values are redacted from this report.",
    }
}

fn redact_portable_report_value(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(object) => {
            for (key, value) in object.iter_mut() {
                if matches!(
                    key.as_str(),
                    "artifact" | "decoded_values" | "records" | "vault_passphrase"
                ) {
                    *value = serde_json::Value::String("redacted".to_string());
                } else {
                    redact_portable_report_value(value);
                }
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                redact_portable_report_value(value);
            }
        }
        _ => {}
    }
}

fn portable_report_source_stem(source_name: &str) -> Option<String> {
    let filename = source_name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(source_name)
        .trim();
    let mut stem = filename;
    for suffix in [".mdid-portable.json", "-mdid-portable.json"] {
        if let Some(stripped) = stem.strip_suffix(suffix) {
            stem = stripped;
            break;
        }
    }
    if stem == filename {
        stem = stem.rsplit_once('.').map_or(stem, |(stem, _)| stem);
    }

    let mut safe = String::new();
    let mut last_was_sep = false;
    for ch in stem.trim().chars() {
        if safe.len() >= 64 {
            break;
        }
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            safe.push(ch);
            last_was_sep = false;
        } else if !last_was_sep && !safe.is_empty() {
            safe.push('_');
            last_was_sep = true;
        }
    }
    while safe.ends_with(['_', '-']) {
        safe.pop();
    }
    (!safe.is_empty()).then_some(safe)
}

impl DesktopVaultResponseState {
    pub fn apply_success(&mut self, mode: DesktopVaultResponseMode, response: &serde_json::Value) {
        self.banner = vault_response_banner(mode).to_string();
        self.summary = vault_response_summary(mode, response);
        self.artifact_notice = vault_response_artifact_notice(response);
        self.error = None;
        self.rendered_mode = Some(mode);
        self.last_success_mode = Some(mode);
        self.last_success_response = Some(response.clone());
    }

    pub fn apply_error(&mut self, mode: DesktopVaultResponseMode, message: impl AsRef<str>) {
        self.banner = vault_response_banner(mode).to_string();
        self.error = Some(redact_desktop_vault_error(message.as_ref()));
        self.summary.clear();
        self.artifact_notice.clear();
        self.rendered_mode = Some(mode);
        self.last_success_mode = None;
        self.last_success_response = None;
    }

    pub fn has_safe_response_report(&self) -> bool {
        self.rendered_mode.is_some()
            && (!self.summary.is_empty()
                || !self.artifact_notice.is_empty()
                || self.error.is_some())
    }

    pub fn safe_response_report_mode(&self) -> Option<DesktopVaultResponseMode> {
        self.has_safe_response_report()
            .then_some(self.rendered_mode?)
    }

    pub fn safe_response_report_json(
        &self,
    ) -> Result<serde_json::Value, DesktopPortableArtifactSaveError> {
        if !self.has_safe_response_report() {
            return Err(DesktopPortableArtifactSaveError::MissingArtifact);
        }
        let mode = self
            .rendered_mode
            .ok_or(DesktopPortableArtifactSaveError::MissingArtifact)?;
        Ok(self.safe_export_json(mode))
    }

    pub fn safe_response_report_download_for_source(
        &self,
        source_name: Option<&str>,
    ) -> Result<DesktopVaultResponseReportDownload, DesktopPortableArtifactSaveError> {
        let mode = self
            .safe_response_report_mode()
            .ok_or(DesktopPortableArtifactSaveError::MissingArtifact)?;
        let json = serde_json::to_string_pretty(&self.safe_export_json(mode))
            .map_err(|error| DesktopPortableArtifactSaveError::InvalidJson(error.to_string()))?;
        let stem = source_name
            .and_then(safe_source_file_stem)
            .unwrap_or_else(|| "desktop".to_string());

        Ok(DesktopVaultResponseReportDownload {
            file_name: format!("{stem}-response-report.json"),
            bytes: json.into_bytes(),
        })
    }

    pub fn portable_artifact_download_json(
        &self,
        mode: DesktopVaultResponseMode,
    ) -> Result<String, DesktopPortableArtifactSaveError> {
        if mode != DesktopVaultResponseMode::VaultExport {
            return Err(DesktopPortableArtifactSaveError::NotVaultExport);
        }

        if self.last_success_mode != Some(DesktopVaultResponseMode::VaultExport) {
            return Err(DesktopPortableArtifactSaveError::NotVaultExport);
        }

        let artifact = self
            .last_success_response
            .as_ref()
            .and_then(|response| response.get("artifact"))
            .filter(|artifact| artifact.is_object())
            .ok_or(DesktopPortableArtifactSaveError::MissingArtifact)?;

        serde_json::to_string_pretty(artifact)
            .map_err(|error| DesktopPortableArtifactSaveError::InvalidJson(error.to_string()))
    }

    pub fn decode_values_export_json(
        &self,
    ) -> Result<serde_json::Value, DesktopDecodedValuesExportError> {
        if self.last_success_mode != Some(DesktopVaultResponseMode::VaultDecode) {
            return Err(DesktopDecodedValuesExportError::NotVaultDecode);
        }

        let response = self
            .last_success_response
            .as_ref()
            .ok_or(DesktopDecodedValuesExportError::NotVaultDecode)?;
        let decoded_values = response
            .get("decoded_values")
            .filter(|decoded_values| decoded_values.is_object())
            .ok_or(DesktopDecodedValuesExportError::MissingDecodedValues)?;
        let decoded_value_count = response
            .get("decoded_value_count")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_else(|| {
                decoded_values
                    .as_object()
                    .map_or(0, |values| values.len() as u64)
            });

        Ok(serde_json::json!({
            "mode": "vault_decode_values",
            "decoded_value_count": decoded_value_count,
            "disclosure": "high-risk decoded values; store only in an approved local workstation location",
            "decoded_values": decoded_values,
        }))
    }

    pub fn audit_events_export_json(
        &self,
    ) -> Result<serde_json::Value, DesktopAuditEventsExportError> {
        if self.last_success_mode != Some(DesktopVaultResponseMode::VaultAudit) {
            return Err(DesktopAuditEventsExportError::NotVaultAudit);
        }

        let response = self
            .last_success_response
            .as_ref()
            .ok_or(DesktopAuditEventsExportError::NotVaultAudit)?;
        let events = response
            .get("events")
            .filter(|events| events.is_array())
            .ok_or(DesktopAuditEventsExportError::MissingAuditEvents)?;
        let mut export = serde_json::json!({
            "mode": "vault_audit_events",
            "events": events,
        });
        if let Some(event_count) = response
            .get("event_count")
            .and_then(serde_json::Value::as_u64)
        {
            export["event_count"] = serde_json::Value::from(event_count);
        }
        if let Some(returned_event_count) = response
            .get("returned_event_count")
            .and_then(serde_json::Value::as_u64)
        {
            export["returned_event_count"] = serde_json::Value::from(returned_event_count);
        }
        if let Some(next_offset) = response
            .get("next_offset")
            .and_then(serde_json::Value::as_u64)
        {
            export["next_offset"] = serde_json::Value::from(next_offset);
        }
        Ok(export)
    }

    fn safe_metadata(&self) -> serde_json::Value {
        const ALLOWLISTED_KEYS: &[&str] = &[
            "artifact_record_count",
            "decoded_count",
            "decoded_value_count",
            "audit_event_id",
            "returned_event_count",
            "total_event_count",
            "offset",
            "limit",
            "imported_record_count",
            "skipped_record_count",
        ];

        let mut metadata = serde_json::Map::new();
        let Some(response) = self
            .last_success_response
            .as_ref()
            .and_then(|value| value.as_object())
        else {
            return serde_json::Value::Object(metadata);
        };

        for key in ALLOWLISTED_KEYS {
            if let Some(value) = response.get(*key).filter(|value| {
                matches!(
                    value,
                    serde_json::Value::Null
                        | serde_json::Value::Bool(_)
                        | serde_json::Value::Number(_)
                        | serde_json::Value::String(_)
                )
            }) {
                metadata.insert((*key).to_string(), value.clone());
            }
        }

        if !metadata.contains_key("total_event_count") {
            if let Some(value) = response.get("event_count").filter(|value| {
                matches!(
                    value,
                    serde_json::Value::Null | serde_json::Value::Number(_)
                )
            }) {
                metadata.insert("total_event_count".to_string(), value.clone());
            }
        }

        serde_json::Value::Object(metadata)
    }

    pub fn safe_export_json(&self, mode: DesktopVaultResponseMode) -> serde_json::Value {
        serde_json::json!({
            "mode": mode.safe_export_label(),
            "banner": self.banner,
            "summary": self.summary,
            "artifact_notice": self.artifact_notice,
            "error": self.error,
            "metadata": self.safe_metadata(),
        })
    }
}

pub fn write_portable_artifact_json(
    state: &DesktopVaultResponseState,
    path: impl AsRef<std::path::Path>,
) -> Result<std::path::PathBuf, DesktopPortableArtifactSaveError> {
    let artifact_json =
        state.portable_artifact_download_json(DesktopVaultResponseMode::VaultExport)?;
    let path = path.as_ref();
    std::fs::write(path, artifact_json)
        .map_err(|error| DesktopPortableArtifactSaveError::Io(error.to_string()))?;
    Ok(path.to_path_buf())
}

pub fn write_desktop_decode_values_json(
    state: &DesktopVaultResponseState,
    path: impl AsRef<std::path::Path>,
) -> Result<std::path::PathBuf, DesktopDecodedValuesExportError> {
    let decode_values_json = serde_json::to_string_pretty(&state.decode_values_export_json()?)
        .map_err(|error| DesktopDecodedValuesExportError::InvalidJson(error.to_string()))?;
    let path = path.as_ref();
    std::fs::write(path, decode_values_json)
        .map_err(|error| DesktopDecodedValuesExportError::Io(error.to_string()))?;
    Ok(path.to_path_buf())
}

pub fn write_desktop_audit_events_json(
    state: &DesktopVaultResponseState,
    path: impl AsRef<std::path::Path>,
) -> Result<std::path::PathBuf, DesktopAuditEventsExportError> {
    let audit_events_json = serde_json::to_string_pretty(&state.audit_events_export_json()?)
        .map_err(|error| DesktopAuditEventsExportError::InvalidJson(error.to_string()))?;
    let path = path.as_ref();
    std::fs::write(path, audit_events_json)
        .map_err(|error| DesktopAuditEventsExportError::Io(error.to_string()))?;
    Ok(path.to_path_buf())
}

pub fn write_safe_vault_response_json(
    state: &DesktopVaultResponseState,
    mode: DesktopVaultResponseMode,
    path: impl AsRef<std::path::Path>,
) -> Result<std::path::PathBuf, DesktopPortableArtifactSaveError> {
    if state.safe_response_report_mode() != Some(mode) {
        return Err(DesktopPortableArtifactSaveError::MissingArtifact);
    }

    let report_json = serde_json::to_string_pretty(&state.safe_export_json(mode))
        .map_err(|error| DesktopPortableArtifactSaveError::InvalidJson(error.to_string()))?;
    let path = path.as_ref();
    std::fs::write(path, report_json)
        .map_err(|error| DesktopPortableArtifactSaveError::Io(error.to_string()))?;
    Ok(path.to_path_buf())
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
        DesktopVaultResponseMode::VaultAudit => vault_audit_response_summary(response),
        DesktopVaultResponseMode::VaultExport | DesktopVaultResponseMode::InspectArtifact => {
            format!("records: {}", response_u64(response, "record_count"))
        }
        DesktopVaultResponseMode::ImportArtifact => format!(
            "imported records: {}",
            response_u64(response, "imported_record_count")
        ),
    }
}

fn vault_audit_response_summary(response: &serde_json::Value) -> String {
    let total = response_u64(response, "event_count");
    let returned = response
        .get("returned_event_count")
        .and_then(serde_json::Value::as_u64)
        .or_else(|| {
            response
                .get("events")
                .and_then(serde_json::Value::as_array)
                .map(|events| events.len() as u64)
        })
        .unwrap_or_default();
    let offset = response_u64(response, "offset");
    let limit = response.get("limit").and_then(serde_json::Value::as_u64);
    let page_status = if returned == 0 || total == 0 || offset >= total {
        format!("Audit events page: showing 0 of {total} from offset {offset}")
    } else {
        let first = offset.saturating_add(1);
        let last = offset.saturating_add(returned).min(total);
        format!("Audit events page: showing {first}-{last} of {total}")
    };
    let limit_status = limit
        .map(|limit| format!("; limit {limit}"))
        .unwrap_or_default();

    format!("events returned: {returned} / {total}; {page_status}{limit_status}")
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

                if media_metadata_json_contains_media_bytes(&body) {
                    return Err(DesktopWorkflowValidationError::MediaBytesNotAccepted);
                }

                Ok(DesktopWorkflowRequest {
                    route: self.mode.route(),
                    body,
                })
            }
        }
    }
}

fn media_metadata_json_contains_media_bytes(value: &serde_json::Value) -> bool {
    const MEDIA_BYTE_FIELDS: &[&str] =
        &["media_bytes_base64", "image_bytes", "file_bytes", "base64"];
    value.as_object().is_some_and(|object| {
        MEDIA_BYTE_FIELDS
            .iter()
            .any(|field| object.contains_key(*field))
    })
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
    match body {
        serde_json::Value::Object(object) if object.is_empty() => body.clone(),
        serde_json::Value::Null => body.clone(),
        _ => serde_json::Value::String("<redacted>".to_string()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopWorkflowValidationError {
    BlankPayload,
    BlankFieldPolicyJson,
    InvalidFieldPolicyJson(String),
    BlankSourceName,
    InvalidMediaMetadataJson,
    MediaBytesNotAccepted,
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
            Self::MediaBytesNotAccepted => {
                write!(f, "metadata-only media review does not accept media bytes")
            }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopRuntimeSubmissionMode {
    Workflow(DesktopWorkflowMode),
    Vault(DesktopVaultMode),
    Portable(DesktopPortableMode),
}

impl DesktopRuntimeSubmissionMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Workflow(mode) => mode.label(),
            Self::Vault(DesktopVaultMode::Decode) => "Vault decode",
            Self::Vault(DesktopVaultMode::AuditEvents) => "Vault audit events",
            Self::Portable(DesktopPortableMode::VaultExport) => "Portable vault export",
            Self::Portable(DesktopPortableMode::InspectArtifact) => "Portable artifact inspect",
            Self::Portable(DesktopPortableMode::ImportArtifact) => "Portable artifact import",
        }
    }

    pub fn route(self) -> &'static str {
        match self {
            Self::Workflow(mode) => mode.route(),
            Self::Vault(mode) => mode.route(),
            Self::Portable(mode) => mode.route(),
        }
    }

    pub fn vault_response_mode(self) -> Option<DesktopVaultResponseMode> {
        match self {
            Self::Workflow(_) => None,
            Self::Vault(DesktopVaultMode::Decode) => Some(DesktopVaultResponseMode::VaultDecode),
            Self::Vault(DesktopVaultMode::AuditEvents) => {
                Some(DesktopVaultResponseMode::VaultAudit)
            }
            Self::Portable(DesktopPortableMode::VaultExport) => {
                Some(DesktopVaultResponseMode::VaultExport)
            }
            Self::Portable(DesktopPortableMode::InspectArtifact) => {
                Some(DesktopVaultResponseMode::InspectArtifact)
            }
            Self::Portable(DesktopPortableMode::ImportArtifact) => {
                Some(DesktopVaultResponseMode::ImportArtifact)
            }
        }
    }
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

    pub fn started(mode: DesktopRuntimeSubmissionMode) -> Self {
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
            || DesktopVaultMode::ALL
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

fn sanitize_review_report_queue(value: Option<&serde_json::Value>) -> serde_json::Value {
    match value {
        Some(serde_json::Value::Array(items)) => serde_json::Value::Array(
            items
                .iter()
                .map(sanitize_review_report_queue_item)
                .collect::<Vec<_>>(),
        ),
        _ => serde_json::Value::Null,
    }
}

fn sanitize_review_report_summary(value: Option<&serde_json::Value>) -> serde_json::Value {
    value.map_or(
        serde_json::Value::Null,
        sanitize_review_report_summary_value,
    )
}

fn sanitize_review_report_summary_value(value: &serde_json::Value) -> serde_json::Value {
    let serde_json::Value::Object(object) = value else {
        return serde_json::Value::Null;
    };

    let mut sanitized = serde_json::Map::new();
    for (key, value) in object {
        if !is_allowed_review_report_summary_key(key) {
            continue;
        }
        if let Some(value) = sanitize_review_report_summary_field(value) {
            sanitized.insert(key.clone(), value);
        }
    }
    serde_json::Value::Object(sanitized)
}

fn sanitize_review_report_summary_field(value: &serde_json::Value) -> Option<serde_json::Value> {
    match value {
        serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::Number(_) => {
            Some(value.clone())
        }
        serde_json::Value::Array(_)
        | serde_json::Value::Object(_)
        | serde_json::Value::String(_) => None,
    }
}

fn is_allowed_review_report_summary_key(key: &str) -> bool {
    matches!(
        key,
        "pages"
            | "page_count"
            | "row_count"
            | "metadata_fields"
            | "metadata_field_count"
            | "metadata_entry_count"
            | "artifact_count"
            | "ocr_required"
            | "ocr_or_visual_review_required"
            | "review_required"
            | "review_required_count"
            | "reviewed_field_count"
            | "candidate_count"
            | "finding_count"
            | "unsupported_count"
            | "unsupported_payload_count"
            | "text_layer_page_count"
            | "total_items"
            | "metadata_only_items"
            | "visual_review_required_items"
            | "unsupported_items"
            | "review_required_candidates"
    )
}

fn sanitize_review_report_queue_item(value: &serde_json::Value) -> serde_json::Value {
    let serde_json::Value::Object(object) = value else {
        return serde_json::Value::Object(serde_json::Map::new());
    };

    let mut sanitized = serde_json::Map::new();
    for (key, value) in object {
        match key.as_str() {
            "page" if value.as_u64().is_some() => {
                sanitized.insert(key.clone(), value.clone());
            }
            "confidence" if value.is_number() => {
                sanitized.insert(key.clone(), value.clone());
            }
            "requires_review" if value.is_boolean() => {
                sanitized.insert(key.clone(), value.clone());
            }
            "status" if value.as_str().is_some_and(is_allowed_review_report_status) => {
                sanitized.insert(key.clone(), value.clone());
            }
            "kind" if value.as_str().is_some_and(is_allowed_review_report_kind) => {
                sanitized.insert(key.clone(), value.clone());
            }
            "format" if value.as_str().is_some_and(is_allowed_review_report_format) => {
                sanitized.insert(key.clone(), value.clone());
            }
            "phi_type"
                if value
                    .as_str()
                    .is_some_and(is_allowed_review_report_phi_type) =>
            {
                sanitized.insert(key.clone(), value.clone());
            }
            "action" if value.as_str().is_some_and(is_allowed_review_report_action) => {
                sanitized.insert(key.clone(), value.clone());
            }
            "field" => {
                sanitized.insert(
                    key.clone(),
                    serde_json::Value::String("redacted-field".into()),
                );
            }
            "field_ref"
                if value.as_object().is_some_and(|field_ref| {
                    field_ref.contains_key("artifact_label")
                        && field_ref.contains_key("metadata_key")
                }) =>
            {
                sanitized.insert(
                    key.clone(),
                    serde_json::json!({
                        "artifact_label": "redacted-artifact",
                        "metadata_key": "redacted-field"
                    }),
                );
            }
            "reason" | "message" => {
                sanitized.insert(
                    key.clone(),
                    serde_json::Value::String("redacted-review-note".into()),
                );
            }
            _ => {}
        }
    }
    serde_json::Value::Object(sanitized)
}

fn is_allowed_review_report_status(text: &str) -> bool {
    matches!(
        text,
        "review_required"
            | "reviewed"
            | "unsupported"
            | "ok"
            | "blocked"
            | "warning"
            | "ocr_or_visual_review_required"
    )
}

fn is_allowed_review_report_kind(text: &str) -> bool {
    matches!(
        text,
        "pdf_review" | "text_layer" | "metadata" | "conservative_media" | "dicom" | "tabular"
    )
}

fn is_allowed_review_report_phi_type(text: &str) -> bool {
    matches!(
        text,
        "Name"
            | "RecordId"
            | "Date"
            | "Location"
            | "Contact"
            | "FreeText"
            | "name"
            | "metadata_identifier"
    )
}

fn is_allowed_review_report_format(text: &str) -> bool {
    matches!(
        text,
        "image"
            | "video"
            | "audio"
            | "fcs"
            | "unknown"
            | "image/jpeg"
            | "image/png"
            | "image/gif"
            | "image/tiff"
            | "application/dicom"
            | "video/mp4"
            | "audio/mpeg"
            | "metadata/json"
    )
}

fn is_allowed_review_report_action(text: &str) -> bool {
    matches!(text, "encode" | "review" | "ignore" | "redact" | "remove")
}

#[derive(Clone, PartialEq, Eq)]
pub struct DesktopWorkflowOutputDownload {
    pub file_name: String,
    pub bytes: Vec<u8>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct DesktopWorkflowReviewReportDownload {
    pub file_name: String,
    pub bytes: Vec<u8>,
}

impl std::fmt::Debug for DesktopWorkflowReviewReportDownload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DesktopWorkflowReviewReportDownload")
            .field("file_name", &self.file_name)
            .field("bytes", &"<redacted>")
            .finish()
    }
}

impl std::fmt::Debug for DesktopWorkflowOutputDownload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DesktopWorkflowOutputDownload")
            .field("file_name", &self.file_name)
            .field("bytes", &"<redacted>")
            .finish()
    }
}

pub fn write_workflow_output_file(
    path: impl AsRef<std::path::Path>,
    download: &DesktopWorkflowOutputDownload,
) -> Result<(), String> {
    std::fs::write(path, &download.bytes)
        .map_err(|_| "workflow output save failed: unable to write output file".to_string())
}

#[derive(Clone, PartialEq, Eq)]
pub struct DesktopWorkflowResponseState {
    pub banner: String,
    pub output: String,
    pub summary: String,
    pub review_queue: String,
    pub error: Option<String>,
    last_success_mode: Option<DesktopWorkflowMode>,
    last_success_response: Option<serde_json::Value>,
}

impl Default for DesktopWorkflowResponseState {
    fn default() -> Self {
        Self {
            banner: "No runtime response rendered yet.".to_string(),
            output: String::new(),
            summary: "No successful runtime summary rendered yet.".to_string(),
            review_queue: "No review queue rendered yet.".to_string(),
            error: None,
            last_success_mode: None,
            last_success_response: None,
        }
    }
}

impl DesktopWorkflowResponseState {
    pub fn workflow_output_download(
        &self,
        mode: DesktopWorkflowMode,
    ) -> Option<DesktopWorkflowOutputDownload> {
        if self.error.is_some() || self.last_success_mode != Some(mode) {
            return None;
        }

        let response = self.last_success_response.as_ref()?;
        match mode {
            DesktopWorkflowMode::CsvText => {
                let csv = response.get("csv")?.as_str()?;
                Some(DesktopWorkflowOutputDownload {
                    file_name: "desktop-deidentified.csv".to_string(),
                    bytes: csv.as_bytes().to_vec(),
                })
            }
            DesktopWorkflowMode::XlsxBase64 => {
                let encoded = response.get("rewritten_workbook_base64")?.as_str()?;
                Some(DesktopWorkflowOutputDownload {
                    file_name: "desktop-deidentified.xlsx".to_string(),
                    bytes: decode_base64(encoded)?,
                })
            }
            DesktopWorkflowMode::DicomBase64 => {
                let encoded = response.get("rewritten_dicom_bytes_base64")?.as_str()?;
                Some(DesktopWorkflowOutputDownload {
                    file_name: "desktop-deidentified.dcm".to_string(),
                    bytes: decode_base64(encoded)?,
                })
            }
            DesktopWorkflowMode::PdfBase64Review | DesktopWorkflowMode::MediaMetadataJson => None,
        }
    }

    pub fn workflow_output_download_for_source(
        &self,
        mode: DesktopWorkflowMode,
        source_name: Option<&str>,
    ) -> Option<DesktopWorkflowOutputDownload> {
        let mut download = self.workflow_output_download(mode)?;
        let stem = source_name
            .filter(|name| name.trim() != "local-workstation-review.pdf")
            .and_then(safe_source_file_stem)?;
        let extension = match mode {
            DesktopWorkflowMode::CsvText => "csv",
            DesktopWorkflowMode::XlsxBase64 => "xlsx",
            DesktopWorkflowMode::DicomBase64 => "dcm",
            DesktopWorkflowMode::PdfBase64Review | DesktopWorkflowMode::MediaMetadataJson => {
                return None;
            }
        };
        download.file_name = format!("{stem}-deidentified.{extension}");
        Some(download)
    }

    pub fn review_report_download(
        &self,
        mode: DesktopWorkflowMode,
    ) -> Option<DesktopWorkflowReviewReportDownload> {
        if !matches!(
            mode,
            DesktopWorkflowMode::PdfBase64Review | DesktopWorkflowMode::MediaMetadataJson
        ) || self.error.is_some()
            || self.last_success_mode != Some(mode)
        {
            return None;
        }

        let response = self.last_success_response.as_ref()?;
        let report = serde_json::json!({
            "mode": match mode {
                DesktopWorkflowMode::PdfBase64Review => "pdf_review",
                DesktopWorkflowMode::MediaMetadataJson => "media_metadata_json",
                DesktopWorkflowMode::CsvText
                | DesktopWorkflowMode::XlsxBase64
                | DesktopWorkflowMode::DicomBase64 => return None,
            },
            "summary": sanitize_review_report_summary(response.get("summary")),
            "review_queue": sanitize_review_report_queue(response.get("review_queue")),
        });
        let json = serde_json::to_string_pretty(&report).ok()?;

        Some(DesktopWorkflowReviewReportDownload {
            file_name: match mode {
                DesktopWorkflowMode::PdfBase64Review => {
                    "desktop-pdf-review-report.json".to_string()
                }
                DesktopWorkflowMode::MediaMetadataJson => {
                    "desktop-media-review-report.json".to_string()
                }
                DesktopWorkflowMode::CsvText
                | DesktopWorkflowMode::XlsxBase64
                | DesktopWorkflowMode::DicomBase64 => return None,
            },
            bytes: json.into_bytes(),
        })
    }

    pub fn review_report_download_for_source(
        &self,
        mode: DesktopWorkflowMode,
        source_name: Option<&str>,
    ) -> Option<DesktopWorkflowReviewReportDownload> {
        let mut download = self.review_report_download(mode)?;
        let stem = source_name
            .and_then(safe_source_file_stem)
            .unwrap_or_else(|| "desktop".to_string());
        download.file_name = match mode {
            DesktopWorkflowMode::PdfBase64Review => format!("{stem}-pdf-review-report.json"),
            DesktopWorkflowMode::MediaMetadataJson => format!("{stem}-media-review-report.json"),
            DesktopWorkflowMode::CsvText
            | DesktopWorkflowMode::XlsxBase64
            | DesktopWorkflowMode::DicomBase64 => return None,
        };
        Some(download)
    }

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

    pub fn suggested_export_file_name_for_source(
        &self,
        mode: DesktopWorkflowMode,
        source_name: Option<&str>,
    ) -> Option<String> {
        self.exportable_output()?;
        let stem = source_name.and_then(safe_source_file_stem);
        let stem = stem.as_deref().unwrap_or("desktop");

        match mode {
            DesktopWorkflowMode::CsvText => Some(format!("{stem}-deidentified.csv")),
            DesktopWorkflowMode::XlsxBase64 => Some(format!("{stem}-deidentified.xlsx.base64.txt")),
            DesktopWorkflowMode::PdfBase64Review => None,
            DesktopWorkflowMode::DicomBase64 => Some(format!("{stem}-deidentified.dcm.base64.txt")),
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

        self.summary = match mode {
            DesktopWorkflowMode::XlsxBase64 => pretty_xlsx_summary(&envelope),
            _ => pretty_json_field(&envelope, "summary"),
        };
        self.review_queue = pretty_json_field(&envelope, "review_queue");
        self.error = None;
        self.last_success_mode = Some(mode);
        self.last_success_response = Some(envelope);
    }

    pub fn apply_error(&mut self, message: impl Into<String>) {
        self.banner = "Runtime response error.".to_string();
        self.output.clear();
        self.summary = "No successful runtime summary rendered yet.".to_string();
        self.review_queue = "No review queue rendered yet.".to_string();
        self.error = Some(message.into());
        self.last_success_mode = None;
        self.last_success_response = None;
    }
}

fn safe_source_file_stem(source_name: &str) -> Option<String> {
    let filename = source_name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(source_name);
    let stem = filename
        .rsplit_once('.')
        .map_or(filename, |(stem, _)| stem)
        .trim();

    let mut safe = String::new();
    let mut last_was_dash = false;
    for ch in stem.chars() {
        if safe.len() >= 64 {
            break;
        }

        if ch.is_ascii_alphanumeric()
            || ((ch == '.' || ch == '-')
                && !safe.is_empty()
                && !safe.ends_with(['.', '-'])
                && !last_was_dash)
        {
            safe.push(ch);
            last_was_dash = false;
        } else if !last_was_dash && !safe.is_empty() {
            safe.push('-');
            last_was_dash = true;
        }
    }

    while safe.ends_with(['-', '.']) {
        safe.pop();
    }

    if safe.is_empty() {
        None
    } else {
        Some(safe)
    }
}

fn pretty_json_field(envelope: &serde_json::Value, field: &str) -> String {
    envelope
        .get(field)
        .and_then(|value| serde_json::to_string_pretty(value).ok())
        .unwrap_or_else(|| "null".to_string())
}

fn pretty_xlsx_summary(envelope: &serde_json::Value) -> String {
    let mut summary = pretty_json_field(envelope, "summary");
    if let Some(disclosure) = envelope.get("worksheet_disclosure") {
        if let Ok(disclosure) = serde_json::to_string_pretty(disclosure) {
            summary.push_str("\nworksheet_disclosure: ");
            summary.push_str(&disclosure);
        }
    }
    summary
}

#[cfg(test)]
mod tempfile {
    use std::path::{Path, PathBuf};

    pub struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        pub fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    pub fn tempdir() -> std::io::Result<TempDir> {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "mdid-desktop-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::create_dir(&path)?;
        Ok(TempDir { path })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tempfile;
    use serde_json::json;

    const DEFAULT_POLICY_JSON: &str = r#"[{"header":"patient_name","phi_type":"Name","action":"encode"},{"header":"patient_id","phi_type":"RecordId","action":"review"}]"#;

    #[test]
    fn desktop_product_copy_avoids_scope_drift_terms() {
        const FORBIDDEN_TERMS: [&str; 4] = ["controller", "agent", "orchestrator", "orchestration"];

        let mut copy = vec![DESKTOP_VAULT_WORKBENCH_COPY];
        copy.extend(
            DesktopWorkflowMode::ALL
                .iter()
                .map(|mode| mode.disclosure()),
        );
        copy.extend(
            DesktopWorkflowMode::ALL
                .iter()
                .map(|mode| mode.payload_hint()),
        );
        copy.extend(
            DesktopPortableMode::ALL
                .iter()
                .map(|mode| mode.disclosure()),
        );

        for text in copy {
            let normalized = text.to_ascii_lowercase();
            for forbidden in FORBIDDEN_TERMS {
                assert!(
                    !normalized.contains(forbidden),
                    "desktop product copy contains forbidden scope drift term {forbidden:?}: {text}"
                );
            }
        }
    }

    #[test]
    fn vault_and_portable_submission_modes_map_to_phi_safe_response_modes() {
        assert_eq!(
            DesktopRuntimeSubmissionMode::Vault(DesktopVaultMode::Decode).vault_response_mode(),
            Some(DesktopVaultResponseMode::VaultDecode)
        );
        assert_eq!(
            DesktopRuntimeSubmissionMode::Vault(DesktopVaultMode::AuditEvents)
                .vault_response_mode(),
            Some(DesktopVaultResponseMode::VaultAudit)
        );
        assert_eq!(
            DesktopRuntimeSubmissionMode::Portable(DesktopPortableMode::VaultExport)
                .vault_response_mode(),
            Some(DesktopVaultResponseMode::VaultExport)
        );
        assert_eq!(
            DesktopRuntimeSubmissionMode::Portable(DesktopPortableMode::InspectArtifact)
                .vault_response_mode(),
            Some(DesktopVaultResponseMode::InspectArtifact)
        );
        assert_eq!(
            DesktopRuntimeSubmissionMode::Portable(DesktopPortableMode::ImportArtifact)
                .vault_response_mode(),
            Some(DesktopVaultResponseMode::ImportArtifact)
        );
        assert_eq!(
            DesktopRuntimeSubmissionMode::Workflow(DesktopWorkflowMode::CsvText)
                .vault_response_mode(),
            None
        );
    }

    #[test]
    fn workflow_response_state_renders_xlsx_worksheet_disclosure_without_cells() {
        let mut state = DesktopWorkflowResponseState::default();
        state.apply_success_json(
            DesktopWorkflowMode::XlsxBase64,
            json!({
                "rewritten_workbook_base64": "d29ya2Jvb2s=",
                "summary": {
                    "total_rows": 2,
                    "encoded_cells": 1,
                    "review_required_cells": 0,
                    "failed_rows": 0
                },
                "review_queue": [],
                "worksheet_disclosure": {
                    "selected_sheet_name": "Patients",
                    "selected_sheet_index": 1,
                    "total_sheet_count": 3,
                    "disclosure": "XLSX processing used the first non-empty worksheet; other worksheets were not processed."
                },
                "patient_cell_value": "Alice Patient"
            }),
        );

        assert!(state.summary.contains("worksheet_disclosure"));
        assert!(state.summary.contains("selected_sheet_name"));
        assert!(state.summary.contains("Patients"));
        assert!(state.summary.contains("first non-empty worksheet"));
        assert!(!state.summary.contains("Alice Patient"));
    }

    #[test]
    fn vault_runtime_success_handoff_uses_safe_summary_not_raw_response_values() {
        let response = serde_json::json!({
            "decoded_value_count": 1,
            "values": [{"original_value": "Alice Patient", "token": "PATIENT_TOKEN"}],
            "audit_event": {"kind": "decode", "detail": "released to Dr Patient"}
        });
        let mut state = DesktopVaultResponseState::default();
        let mode = DesktopRuntimeSubmissionMode::Vault(DesktopVaultMode::Decode)
            .vault_response_mode()
            .expect("vault response mode");

        state.apply_success(mode, &response);

        assert!(state.summary.contains("decoded values: 1"));
        assert!(!state.summary.contains("Alice Patient"));
        assert!(!state.summary.contains("PATIENT_TOKEN"));
        assert!(!state.summary.contains("Dr Patient"));
    }

    #[test]
    fn desktop_portable_response_report_save_uses_safe_source_name_and_redacts_artifact() {
        let payload = build_desktop_portable_response_report_save(
            DesktopPortableMode::ImportArtifact,
            Some("Alice portable.mdid-portable.json"),
            r#"{"artifact":{"records":[{"id":"phi-1"}]},"nested":{"decoded_values":{"patient":"Alice"}},"imported_record_count":1,"audit_event_count":2}"#,
        )
        .expect("portable import response should produce report save payload");

        assert_eq!(
            payload.suggested_file_name,
            "Alice_portable-portable-artifact-import-report.json"
        );
        assert_eq!(payload.mime_type, "application/json");
        assert_eq!(payload.status, "Portable artifact import report ready to save; artifact and decoded values are redacted from this report.");
        let report: serde_json::Value = serde_json::from_str(&payload.contents).unwrap();
        assert_eq!(report["mode"], "portable_artifact_import");
        assert_eq!(report["imported_record_count"], 1);
        assert_eq!(report["audit_event_count"], 2);
        assert_eq!(report["artifact"], "redacted");
        assert_eq!(report["nested"]["decoded_values"], "redacted");
        assert!(!payload.contents.contains("phi-1"));
        assert!(!payload.contents.contains("Alice"));
    }

    #[test]
    fn desktop_portable_response_report_save_rejects_invalid_json() {
        let error = build_desktop_portable_response_report_save(
            DesktopPortableMode::InspectArtifact,
            Some("portable.mdid-portable.json"),
            "not-json",
        )
        .unwrap_err();

        assert_eq!(error, DesktopPortableReportSaveError::InvalidResponseJson);
    }

    #[test]
    fn desktop_portable_response_report_save_preserves_desktop_source_stem() {
        let payload = build_desktop_portable_response_report_save(
            DesktopPortableMode::InspectArtifact,
            Some("desktop.mdid-portable.json"),
            r#"{"artifact":{"records":[{"id":"phi-1"}]}}"#,
        )
        .expect("portable inspect response should produce report save payload");

        assert_eq!(
            payload.suggested_file_name,
            "desktop-portable-artifact-inspect-report.json"
        );
    }

    #[test]
    fn desktop_pdf_review_report_save_redacts_text_and_uses_source_stem() {
        let response = serde_json::json!({
            "summary": {
                "total_pages": 3,
                "pages_with_text": 2,
                "ocr_required_pages": 1,
                "sensitive_text": "Jane Roe DOB 1970"
            },
            "review_queue": [
                {
                    "page": 2,
                    "kind": "ocr_required",
                    "status": "needs_review",
                    "text": "Jane Roe DOB 1970"
                }
            ],
            "pdf_bytes_base64": "SHOULD_NOT_LEAK"
        });

        let save = build_desktop_pdf_review_report_save(
            &response.to_string(),
            Some("workstation scan.pdf"),
        )
        .expect("desktop pdf report save");

        assert_eq!(save.file_name, "workstation_scan-pdf-review-report.json");
        assert_eq!(save.status, "PDF review report ready to save; text content and PDF bytes are redacted from this report.");
        let report: serde_json::Value = serde_json::from_str(&save.contents).unwrap();
        assert_eq!(report["mode"], "pdf_review_report");
        assert_eq!(report["summary"]["total_pages"], 3);
        assert_eq!(report["review_queue"][0]["page"], 2);
        assert_eq!(report["review_queue"][0]["kind"], "ocr_required");
        let serialized = serde_json::to_string(&report).unwrap();
        assert!(!serialized.contains("Jane"));
        assert!(!serialized.contains("DOB"));
        assert!(!serialized.contains("SHOULD_NOT_LEAK"));
        assert!(!serialized.contains("pdf_bytes_base64"));
        assert!(!serialized.contains("\"text\""));
    }

    #[test]
    fn desktop_pdf_review_report_save_rejects_non_object_runtime_response() {
        let error = build_desktop_pdf_review_report_save("null", Some("scan.pdf")).unwrap_err();
        assert!(error.contains("PDF review report requires a JSON object response"));
    }

    #[test]
    fn desktop_pdf_review_report_save_defaults_missing_containers_and_preserves_primitive_queue_fields(
    ) {
        let missing_containers = build_desktop_pdf_review_report_save("{}", Some("scan.pdf"))
            .expect("missing summary and queue should still save");
        let missing_report: serde_json::Value =
            serde_json::from_str(&missing_containers.contents).unwrap();
        assert_eq!(missing_report["summary"], serde_json::json!({}));
        assert_eq!(missing_report["review_queue"], serde_json::json!([]));

        let non_container_response = serde_json::json!({
            "summary": "not an object",
            "review_queue": {"not": "an array"}
        });
        let non_containers = build_desktop_pdf_review_report_save(
            &non_container_response.to_string(),
            Some("scan.pdf"),
        )
        .expect("non-object summary and non-array queue should still save");
        let non_container_report: serde_json::Value =
            serde_json::from_str(&non_containers.contents).unwrap();
        assert_eq!(non_container_report["summary"], serde_json::json!({}));
        assert_eq!(non_container_report["review_queue"], serde_json::json!([]));

        let primitive_queue_response = serde_json::json!({
            "review_queue": [
                {
                    "page": null,
                    "kind": 42,
                    "status": false,
                    "text": "Jane Roe should not leak",
                    "details": {"text": "nested PHI"}
                },
                {
                    "page": "front matter",
                    "kind": null,
                    "status": 7
                }
            ]
        });
        let primitive_queue = build_desktop_pdf_review_report_save(
            &primitive_queue_response.to_string(),
            Some("scan.pdf"),
        )
        .expect("primitive/null queue field values should still save");
        let primitive_queue_report: serde_json::Value =
            serde_json::from_str(&primitive_queue.contents).unwrap();
        assert_eq!(
            primitive_queue_report["review_queue"],
            serde_json::json!([
                {"page": null, "kind": 42, "status": false},
                {"page": "front matter", "kind": null, "status": 7}
            ])
        );
        let serialized = serde_json::to_string(&primitive_queue_report).unwrap();
        assert!(!serialized.contains("Jane Roe"));
        assert!(!serialized.contains("nested PHI"));

        let mixed_queue_response = serde_json::json!({
            "review_queue": [42, "x", null, {"page": 1}]
        });
        let mixed_queue = build_desktop_pdf_review_report_save(
            &mixed_queue_response.to_string(),
            Some("scan.pdf"),
        )
        .expect("non-object queue entries should be skipped");
        let mixed_queue_report: serde_json::Value =
            serde_json::from_str(&mixed_queue.contents).unwrap();
        assert_eq!(
            mixed_queue_report["review_queue"],
            serde_json::json!([{ "page": 1 }])
        );
    }

    #[test]
    fn desktop_state_defaults_are_phi_safe() {
        let inspect_response = serde_json::json!({
            "record_count": 2,
            "records": [{
                "record_id": "inspect-raw-patient-123",
                "token": "inspect-raw-token-secret",
                "payload": "inspect-raw-phi-payload"
            }],
            "artifact_path": "C:\\vaults\\sensitive\\Inspect Raw Clinic Batch.mdid-portable.json"
        });
        let inspect_input_text =
            serde_json::to_string(&inspect_response).expect("fixture serializes");
        assert!(inspect_input_text.contains("Inspect Raw Clinic Batch"));
        assert!(inspect_input_text.contains("inspect-raw-patient-123"));
        assert!(inspect_input_text.contains("inspect-raw-token-secret"));
        assert!(inspect_input_text.contains("inspect-raw-phi-payload"));

        let mut inspect_state = DesktopVaultResponseState::default();
        inspect_state.apply_success(DesktopVaultResponseMode::InspectArtifact, &inspect_response);

        let inspect_report = inspect_state
            .safe_response_report_download_for_source(Some(
                "C:\\vaults\\sensitive\\Clinic Batch.mdid-portable.json",
            ))
            .expect("portable inspect response should create a safe report download");
        assert_eq!(
            inspect_report.file_name,
            "Clinic-Batch.mdid-portable-response-report.json"
        );
        let inspect_text = std::str::from_utf8(&inspect_report.bytes).expect("report is utf8 json");
        assert!(inspect_text.contains("bounded portable artifact response rendered locally"));
        assert!(inspect_text.contains("artifact path returned; full path hidden"));
        assert!(!inspect_text.contains("Inspect Raw Clinic Batch"));
        assert!(!inspect_text.contains("inspect-raw-patient-123"));
        assert!(!inspect_text.contains("inspect-raw-token-secret"));
        assert!(!inspect_text.contains("inspect-raw-phi-payload"));

        let import_response = serde_json::json!({
            "imported_record_count": 1,
            "duplicate_record_count": 1,
            "imported_records": [{
                "record_id": "import-raw-patient-456",
                "token": "import-raw-token-secret",
                "payload": "import-raw-phi-payload"
            }],
            "artifact_path": "/tmp/Import Raw Partner Export.mdid-portable.json"
        });
        let import_input_text =
            serde_json::to_string(&import_response).expect("fixture serializes");
        assert!(import_input_text.contains("Import Raw Partner Export"));
        assert!(import_input_text.contains("import-raw-patient-456"));
        assert!(import_input_text.contains("import-raw-token-secret"));
        assert!(import_input_text.contains("import-raw-phi-payload"));

        let mut import_state = DesktopVaultResponseState::default();
        import_state.apply_success(DesktopVaultResponseMode::ImportArtifact, &import_response);

        let import_report = import_state
            .safe_response_report_download_for_source(Some(
                "/tmp/Partner Export.mdid-portable.json",
            ))
            .expect("portable import response should create a safe report download");
        assert_eq!(
            import_report.file_name,
            "Partner-Export.mdid-portable-response-report.json"
        );
        let import_text = std::str::from_utf8(&import_report.bytes).expect("report is utf8 json");
        assert!(import_text.contains("bounded portable artifact response rendered locally"));
        assert!(import_text.contains("artifact path returned; full path hidden"));
        assert!(!import_text.contains("Import Raw Partner Export"));
        assert!(!import_text.contains("import-raw-patient-456"));
        assert!(!import_text.contains("import-raw-token-secret"));
        assert!(!import_text.contains("import-raw-phi-payload"));
    }

    #[test]
    fn safe_response_report_download_uses_source_stem_for_vault_decode() {
        let mut state = DesktopVaultResponseState::default();
        state.apply_success(
            DesktopVaultResponseMode::VaultDecode,
            &serde_json::json!({ "decoded_value_count": 2 }),
        );

        let download = state
            .safe_response_report_download_for_source(Some(
                "C:/Vault Exports/Patient Alpha.mdid-vault.json",
            ))
            .expect("decode report download should be available");

        assert_eq!(
            download.file_name,
            "Patient-Alpha.mdid-vault-response-report.json"
        );
        let report: serde_json::Value = serde_json::from_slice(&download.bytes).unwrap();
        assert_eq!(report["mode"], "vault_decode");
        assert_eq!(report["summary"], "decoded values: 2");
    }

    #[test]
    fn safe_response_report_download_uses_source_stem_for_vault_audit() {
        let mut state = DesktopVaultResponseState::default();
        state.apply_success(
            DesktopVaultResponseMode::VaultAudit,
            &serde_json::json!({ "returned_event_count": 3, "event_count": 8 }),
        );

        let download = state
            .safe_response_report_download_for_source(Some("audit export.json"))
            .expect("audit report download should be available");

        assert_eq!(download.file_name, "audit-export-response-report.json");
        let report: serde_json::Value = serde_json::from_slice(&download.bytes).unwrap();
        assert_eq!(report["mode"], "vault_audit");
        assert!(report["summary"]
            .as_str()
            .unwrap()
            .contains("events returned: 3 / 8"));
    }

    #[test]
    fn vault_audit_response_summary_includes_pagination_status() {
        let mut state = DesktopVaultResponseState::default();
        state.apply_success(
            DesktopVaultResponseMode::VaultAudit,
            &serde_json::json!({
                "events": [
                    {"id": "evt-1", "kind": "decode", "actor": "clinician-1"},
                    {"id": "evt-2", "kind": "decode", "actor": "clinician-2"}
                ],
                "event_count": 7,
                "returned_event_count": 2,
                "offset": 5,
                "limit": 2
            }),
        );

        assert!(state
            .summary
            .contains("Audit events page: showing 6-7 of 7"));
        assert!(state.summary.contains("limit 2"));
        assert!(!state.summary.contains("evt-1"));
        assert!(!state.summary.contains("decode"));
    }

    #[test]
    fn safe_response_report_download_uses_source_stem_for_vault_export() {
        let mut state = DesktopVaultResponseState::default();
        state.apply_success(
            DesktopVaultResponseMode::VaultExport,
            &serde_json::json!({ "record_count": 4, "artifact_path": "/sensitive/path/export.json" }),
        );

        let download = state
            .safe_response_report_download_for_source(Some("portable subset.mdid-portable.json"))
            .expect("export report download should be available");

        assert_eq!(
            download.file_name,
            "portable-subset.mdid-portable-response-report.json"
        );
        let report: serde_json::Value = serde_json::from_slice(&download.bytes).unwrap();
        assert_eq!(report["mode"], "vault_export");
        assert_eq!(report["summary"], "records: 4");
        assert_eq!(
            report["artifact_notice"],
            "artifact path returned; full path hidden"
        );
        assert!(report.get("artifact_path").is_none());
    }

    #[test]
    fn desktop_response_report_for_source_rejects_missing_rendered_report() {
        let state = DesktopVaultResponseState::default();

        assert_eq!(
            state.safe_response_report_download_for_source(Some("source.mdid-portable.json")),
            Err(DesktopPortableArtifactSaveError::MissingArtifact)
        );
    }

    #[test]
    fn vault_response_safe_export_omits_decoded_values_paths_and_raw_audit_detail() {
        let response = serde_json::json!({
            "decoded_value_count": 2,
            "report_path": "/sensitive/patient/alice-decode.json",
            "decoded_values": [
                {"record_id": "patient-1", "field": "name", "value": "Alice Example"}
            ],
            "audit_event": {"kind": "decode", "detail": "released Alice Example to oncology"}
        });
        let mut state = DesktopVaultResponseState::default();
        state.apply_success(DesktopVaultResponseMode::VaultDecode, &response);

        let exported = state.safe_export_json(DesktopVaultResponseMode::VaultDecode);
        let exported_text = serde_json::to_string(&exported).expect("safe export serializes");

        assert_eq!(exported["mode"], "vault_decode");
        assert_eq!(
            exported["banner"],
            "bounded vault decode response rendered locally"
        );
        assert_eq!(exported["summary"], "decoded values: 2");
        assert_eq!(
            exported["artifact_notice"],
            "artifact path returned; full path hidden"
        );
        assert_eq!(exported["error"], serde_json::Value::Null);
        assert!(!exported_text.contains("Alice Example"));
        assert!(!exported_text.contains("/sensitive/patient"));
        assert!(!exported_text.contains("released Alice"));
        assert!(!exported_text.contains("decoded_values"));
        assert!(!exported_text.contains("audit_event"));
    }

    #[test]
    fn vault_response_safe_export_keeps_runtime_errors_redacted() {
        let mut state = DesktopVaultResponseState::default();
        state.apply_error(
            DesktopVaultResponseMode::InspectArtifact,
            "failed to open /secret/artifact.json with passphrase hunter2",
        );

        let exported = state.safe_export_json(DesktopVaultResponseMode::InspectArtifact);
        let exported_text = serde_json::to_string(&exported).expect("safe export serializes");

        assert_eq!(exported["mode"], "portable_artifact_inspect");
        assert_eq!(
            exported["banner"],
            "bounded portable artifact response rendered locally"
        );
        assert_eq!(exported["summary"], "");
        assert_eq!(exported["artifact_notice"], "");
        assert_eq!(exported["error"], "runtime failed; details redacted");
        assert!(!exported_text.contains("/secret/artifact.json"));
        assert!(!exported_text.contains("hunter2"));
    }

    #[test]
    fn safe_vault_response_json_writer_rejects_default_state_without_creating_file() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let target = temp_dir.path().join("synthetic-vault-audit-report.json");
        let state = DesktopVaultResponseState::default();

        let result =
            write_safe_vault_response_json(&state, DesktopVaultResponseMode::VaultAudit, &target);

        assert!(matches!(
            result,
            Err(DesktopPortableArtifactSaveError::MissingArtifact)
        ));
        assert!(!target.exists());
    }

    #[test]
    fn safe_vault_response_json_writer_rejects_mismatched_rendered_mode_without_creating_file() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let target = temp_dir.path().join("mismatched-vault-audit-report.json");
        let mut state = DesktopVaultResponseState::default();
        let response = serde_json::json!({
            "decoded_value_count": 1,
            "decoded_values": [
                {"record_id": "patient-1", "field": "name", "value": "Alice Example"}
            ],
            "vault_path": "C:/vaults/alice.mdid",
            "audit_event": {"kind": "decode", "detail": "released Alice Example"}
        });
        state.apply_success(DesktopVaultResponseMode::VaultDecode, &response);

        let result =
            write_safe_vault_response_json(&state, DesktopVaultResponseMode::VaultAudit, &target);

        assert!(matches!(
            result,
            Err(DesktopPortableArtifactSaveError::MissingArtifact)
        ));
        assert!(!target.exists());
    }

    #[test]
    fn safe_vault_response_json_writer_persists_allowlisted_audit_summary_only() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let target = temp_dir.path().join("vault-audit-report.json");
        let mut state = DesktopVaultResponseState::default();
        let response = serde_json::json!({
            "event_count": 4,
            "returned_event_count": 2,
            "events": [
                {"kind": "decode", "detail": "patient Alice decoded for oncology"},
                {"kind": "export", "detail": "exported C:/vaults/alice.mdid"}
            ],
            "vault_path": "C:/vaults/alice.mdid",
            "vault_passphrase": "correct horse battery staple"
        });
        state.apply_success(DesktopVaultResponseMode::VaultAudit, &response);

        let written_path =
            write_safe_vault_response_json(&state, DesktopVaultResponseMode::VaultAudit, &target)
                .expect("safe vault response report should be written");

        assert_eq!(written_path, target);
        let persisted = std::fs::read_to_string(&written_path).expect("read report");
        assert!(persisted.contains("\"mode\": \"vault_audit\""));
        assert!(persisted.contains("events returned: 2 / 4"));
        assert!(!persisted.contains("patient Alice"));
        assert!(!persisted.contains("C:/vaults/alice.mdid"));
        assert!(!persisted.contains("correct horse battery staple"));
        assert!(!persisted.contains("\"events\""));
    }

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
    fn media_metadata_request_rejects_media_byte_payload_fields_phi_safely() {
        let raw_media_value = "SmFuZSBQYXRpZW50IE1STi0wMDE=";

        for field in ["media_bytes_base64", "image_bytes", "file_bytes", "base64"] {
            let state = DesktopWorkflowRequestState {
                mode: DesktopWorkflowMode::MediaMetadataJson,
                payload: serde_json::json!({
                    "artifact_label": "patient-jane-image.png",
                    "format": "image",
                    "metadata": [{"key": "CameraOwner", "value": "Jane Patient"}],
                    field: raw_media_value,
                })
                .to_string(),
                field_policy_json: "{}".to_string(),
                source_name: "local-media-metadata.json".to_string(),
            };

            let error = state.try_build_request().unwrap_err();
            assert_eq!(error, DesktopWorkflowValidationError::MediaBytesNotAccepted);
            let message = error.to_string();
            assert_eq!(
                message,
                "metadata-only media review does not accept media bytes"
            );
            assert!(!message.contains(raw_media_value));
            assert!(!message.contains("Jane Patient"));
        }
    }

    #[test]
    fn media_metadata_request_debug_redacts_phi_payload() {
        let state = DesktopWorkflowRequestState {
            mode: DesktopWorkflowMode::MediaMetadataJson,
            payload: "{\"artifact_label\":\"scan.png\",\"format\":\"image\",\"metadata\":[{\"key\":\"PatientName\",\"value\":\"Jane Patient\"}],\"ocr_or_visual_review_required\":true}".to_string(),
            field_policy_json: "{}".to_string(),
            source_name: "local-media-metadata.json".to_string(),
        };

        let request = state
            .try_build_request()
            .expect("valid metadata object should build");
        let debug = format!("{request:?}");

        assert!(debug.contains("/media/conservative/deidentify"));
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("Jane Patient"));
        assert!(!debug.contains("PatientName"));
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
        assert!(DesktopPortableMode::VaultExport
            .disclosure()
            .contains("existing local /vault/export runtime route"));
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
    fn desktop_portable_export_request_rejects_duplicate_record_ids() {
        let state = DesktopPortableRequestState {
            mode: DesktopPortableMode::VaultExport,
            vault_path: "/safe/local.vault".to_string(),
            vault_passphrase: "vault-secret".to_string(),
            record_ids_json: "[\"550e8400-e29b-41d4-a716-446655440000\",\"550e8400-e29b-41d4-a716-446655440000\"]".to_string(),
            export_passphrase: "portable-secret".to_string(),
            export_context: "handoff to privacy office".to_string(),
            artifact_json: String::new(),
            portable_passphrase: String::new(),
            destination_vault_path: String::new(),
            destination_vault_passphrase: String::new(),
            import_context: String::new(),
            requested_by: "desktop".to_string(),
        };

        let err = state
            .try_build_request()
            .expect_err("desktop must reject duplicate export record ids");
        let message = format!("{err:?}");
        assert!(message.contains("duplicate record id"));
        assert!(!message.contains("550e8400"));
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
    fn vault_export_download_json_contains_only_artifact_object() {
        let response = serde_json::json!({
            "artifact": {"version": 1, "ciphertext": "encrypted-payload", "nonce": "safe-nonce"},
            "record_count": 1,
            "vault_path": "/sensitive/Alice-vault.json",
            "vault_passphrase": "hunter2",
            "audit_event": {"detail": "exported Alice Example MRN 123"},
            "original_value": "Alice Example"
        });
        let mut state = DesktopVaultResponseState::default();
        state.apply_success(DesktopVaultResponseMode::VaultExport, &response);

        let artifact_json = state
            .portable_artifact_download_json(DesktopVaultResponseMode::VaultExport)
            .expect("valid artifact JSON should be available");

        assert!(artifact_json.contains("encrypted-payload"));
        assert!(artifact_json.contains("safe-nonce"));
        assert!(!artifact_json.contains("Alice Example"));
        assert!(!artifact_json.contains("/sensitive"));
        assert!(!artifact_json.contains("hunter2"));
        assert!(!artifact_json.contains("audit_event"));
        assert!(!artifact_json.contains("original_value"));
    }

    #[test]
    fn vault_export_download_json_fails_closed_for_malformed_or_non_export_responses() {
        let mut state = DesktopVaultResponseState::default();
        state.apply_success(
            DesktopVaultResponseMode::InspectArtifact,
            &serde_json::json!({"record_count": 3}),
        );
        assert_eq!(
            state.portable_artifact_download_json(DesktopVaultResponseMode::InspectArtifact),
            Err(DesktopPortableArtifactSaveError::NotVaultExport)
        );

        let mut inspect_state_with_artifact = DesktopVaultResponseState::default();
        inspect_state_with_artifact.apply_success(
            DesktopVaultResponseMode::InspectArtifact,
            &serde_json::json!({"artifact": {"version": 1, "ciphertext": "inspect-payload"}}),
        );
        assert_eq!(
            inspect_state_with_artifact
                .portable_artifact_download_json(DesktopVaultResponseMode::VaultExport),
            Err(DesktopPortableArtifactSaveError::NotVaultExport)
        );

        let mut export_state = DesktopVaultResponseState::default();
        export_state.apply_success(
            DesktopVaultResponseMode::VaultExport,
            &serde_json::json!({"artifact": "not an object"}),
        );
        assert_eq!(
            export_state.portable_artifact_download_json(DesktopVaultResponseMode::VaultExport),
            Err(DesktopPortableArtifactSaveError::MissingArtifact)
        );
    }

    #[test]
    fn write_portable_artifact_json_writes_pretty_artifact_without_sensitive_runtime_envelope() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("export.mdid-portable.json");
        let response = serde_json::json!({
            "artifact": {"version": 1, "ciphertext": "encrypted-payload"},
            "audit_event": {"detail": "patient Alice handoff"},
            "vault_path": "/secret/patient.vault"
        });
        let mut state = DesktopVaultResponseState::default();
        state.apply_success(DesktopVaultResponseMode::VaultExport, &response);

        let written = write_portable_artifact_json(&state, &path).expect("artifact write succeeds");
        let persisted = std::fs::read_to_string(&path).expect("artifact file exists");

        assert_eq!(written, path);
        assert_eq!(
            persisted,
            "{\n  \"ciphertext\": \"encrypted-payload\",\n  \"version\": 1\n}"
        );
        assert!(!persisted.contains("Alice"));
        assert!(!persisted.contains("/secret"));
        assert!(!persisted.contains("audit_event"));
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
    fn safe_response_report_includes_allowlisted_decode_metadata_without_sensitive_values() {
        let mut state = DesktopVaultResponseState::default();
        let response = serde_json::json!({
            "artifact_record_count": 9,
            "decoded_count": 4,
            "decoded_value_count": 2,
            "audit_event_id": "audit-123",
            "decoded_values": {"record-1": {"name": "Jane Doe"}},
            "events": [{"event_id": "evt-sensitive"}],
            "vault_path": "/secret/Jane.vault",
            "passphrase": "do-not-save",
            "nested": {"decoded_values": "Jane Doe"}
        });

        state.apply_success(DesktopVaultResponseMode::VaultDecode, &response);

        let report = state
            .safe_response_report_json()
            .expect("safe decode response report");
        let serialized = serde_json::to_string(&report).expect("serialize report");

        assert_eq!(report["mode"], "vault_decode");
        assert_eq!(report["metadata"]["artifact_record_count"], 9);
        assert_eq!(report["metadata"]["decoded_count"], 4);
        assert_eq!(report["metadata"]["decoded_value_count"], 2);
        assert_eq!(report["metadata"]["audit_event_id"], "audit-123");
        assert!(!serialized.contains("decoded_values"));
        assert!(!serialized.contains("\"events\""));
        assert!(!serialized.contains("vault_path"));
        assert!(!serialized.contains("passphrase"));
        assert!(!serialized.contains("Jane Doe"));
        assert!(!serialized.contains("/secret"));
        assert!(!serialized.contains("do-not-save"));
    }

    #[test]
    fn safe_response_report_includes_allowlisted_audit_metadata_without_sensitive_events() {
        let mut state = DesktopVaultResponseState::default();
        let response = serde_json::json!({
            "event_count": 200,
            "returned_event_count": 2,
            "offset": 10,
            "limit": null,
            "events": [
                {"event_id": "evt-1", "kind": "decode", "record_id": "record-1"},
                {"event_id": "evt-2", "kind": "encode", "record_id": "record-2"}
            ],
            "vault_path": "/secret/Alice.vault",
            "passphrase": "do-not-save"
        });

        state.apply_success(DesktopVaultResponseMode::VaultAudit, &response);

        let report = state
            .safe_response_report_json()
            .expect("safe audit response report");
        let serialized = serde_json::to_string(&report).expect("serialize report");

        assert_eq!(report["mode"], "vault_audit");
        assert_eq!(report["metadata"]["total_event_count"], 200);
        assert_eq!(report["metadata"]["returned_event_count"], 2);
        assert_eq!(report["metadata"]["offset"], 10);
        assert!(report["metadata"].get("limit").is_some());
        assert!(report["metadata"]["limit"].is_null());
        assert!(report["metadata"].get("event_count").is_none());
        assert!(!serialized.contains("\"events\""));
        assert!(!serialized.contains("vault_path"));
        assert!(!serialized.contains("passphrase"));
        assert!(!serialized.contains("evt-1"));
        assert!(!serialized.contains("/secret"));
        assert!(!serialized.contains("do-not-save"));
    }

    #[test]
    fn desktop_decode_values_export_contains_decoded_values_for_decode_response() {
        let mut state = DesktopVaultResponseState::default();
        let response = serde_json::json!({
            "decoded_value_count": 2,
            "decoded_values": {
                "record-1": {"name": "Jane Doe"},
                "record-2": {"mrn": "12345"}
            },
            "audit_event_id": "audit-1"
        });

        state.apply_success(DesktopVaultResponseMode::VaultDecode, &response);

        let json = state
            .decode_values_export_json()
            .expect("decode values export");

        assert_eq!(json["mode"], "vault_decode_values");
        assert_eq!(json["decoded_value_count"], 2);
        assert_eq!(json["decoded_values"]["record-1"]["name"], "Jane Doe");
        assert_eq!(json["decoded_values"]["record-2"]["mrn"], "12345");
        assert_eq!(
            json["disclosure"],
            "high-risk decoded values; store only in an approved local workstation location"
        );
    }

    #[test]
    fn desktop_decode_values_export_uses_decoded_values_len_when_count_missing() {
        let mut state = DesktopVaultResponseState::default();
        let response = serde_json::json!({
            "decoded_values": {
                "record-1": {"name": "Jane Doe"},
                "record-2": {"mrn": "12345"},
                "record-3": {"dob": "1970-01-01"}
            }
        });

        state.apply_success(DesktopVaultResponseMode::VaultDecode, &response);

        let json = state
            .decode_values_export_json()
            .expect("decode values export");

        assert_eq!(json["decoded_value_count"], 3);
        assert_eq!(json["decoded_values"]["record-3"]["dob"], "1970-01-01");
    }

    #[test]
    fn write_desktop_decode_values_export_json_persists_successful_decode_export() {
        let mut state = DesktopVaultResponseState::default();
        let response = serde_json::json!({
            "decoded_value_count": 1,
            "decoded_values": {
                "record-1": {"diagnosis": "asthma"}
            }
        });
        state.apply_success(DesktopVaultResponseMode::VaultDecode, &response);
        let path = std::env::temp_dir().join(format!(
            "mdid-desktop-decode-values-export-{}-persisted.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);

        let returned_path = write_desktop_decode_values_json(&state, &path).expect("write export");
        let persisted = std::fs::read_to_string(&path).expect("read persisted export");
        let persisted_json: serde_json::Value =
            serde_json::from_str(&persisted).expect("persisted JSON parses");
        let _ = std::fs::remove_file(&path);

        assert_eq!(returned_path, path);
        assert_eq!(persisted_json["mode"], "vault_decode_values");
        assert_eq!(persisted_json["decoded_value_count"], 1);
        assert_eq!(
            persisted_json["decoded_values"]["record-1"]["diagnosis"],
            "asthma"
        );
    }

    #[test]
    fn desktop_decode_values_export_is_cleared_after_decode_error() {
        let mut state = DesktopVaultResponseState::default();
        state.apply_success(
            DesktopVaultResponseMode::VaultDecode,
            &serde_json::json!({
                "decoded_value_count": 1,
                "decoded_values": {"record-1": {"name": "Jane Doe"}}
            }),
        );
        assert!(state.decode_values_export_json().is_ok());

        state.apply_error(
            DesktopVaultResponseMode::VaultDecode,
            "decode failed for Jane Doe",
        );

        let error = state
            .decode_values_export_json()
            .expect_err("stale decode values export cleared");
        assert_eq!(
            error.to_string(),
            "decoded values export is only available for successful vault decode responses"
        );
    }

    #[test]
    fn desktop_decode_values_export_rejects_non_decode_response() {
        let mut state = DesktopVaultResponseState::default();
        state.apply_success(
            DesktopVaultResponseMode::VaultAudit,
            &serde_json::json!({"event_count": 0, "events": []}),
        );

        let error = state.decode_values_export_json().expect_err("not decode");

        assert_eq!(
            error.to_string(),
            "decoded values export is only available for successful vault decode responses"
        );
    }

    #[test]
    fn desktop_decode_values_export_rejects_missing_decoded_values() {
        let mut state = DesktopVaultResponseState::default();
        state.apply_success(
            DesktopVaultResponseMode::VaultDecode,
            &serde_json::json!({"decoded_value_count": 0}),
        );

        let error = state
            .decode_values_export_json()
            .expect_err("missing values");

        assert_eq!(error.to_string(), "decoded values are unavailable");
    }

    #[test]
    fn audit_events_export_json_contains_events_and_metadata_without_request_secrets() {
        let mut state = DesktopVaultResponseState::default();
        state.apply_success(
            DesktopVaultResponseMode::VaultAudit,
            &serde_json::json!({
                "event_count": 200,
                "returned_event_count": 2,
                "next_offset": 2,
                "events": [
                    {"event_id": "evt-1", "kind": "decode", "record_id": "record-1"},
                    {"event_id": "evt-2", "kind": "encode", "record_id": "record-2"}
                ],
                "vault_path": "/secret/Alice.vault",
                "vault_passphrase": "do-not-save",
                "passphrase": "also-secret",
                "request": {"vault_path": "/secret/request.vault"}
            }),
        );

        let json = state
            .audit_events_export_json()
            .expect("audit events export");
        let serialized = serde_json::to_string(&json).expect("serialize export");

        assert_eq!(json["mode"], "vault_audit_events");
        assert_eq!(json["event_count"], 200);
        assert_eq!(json["returned_event_count"], 2);
        assert_eq!(json["next_offset"], 2);
        assert_eq!(json["events"][0]["event_id"], "evt-1");
        assert_eq!(json["events"][1]["kind"], "encode");
        assert!(!serialized.contains("passphrase"));
        assert!(!serialized.contains("vault_path"));
        assert!(!serialized.contains("request"));
        assert!(!serialized.contains("/secret"));
        assert!(!serialized.contains("do-not-save"));
    }

    #[test]
    fn audit_events_export_json_omits_absent_count_and_offset_metadata() {
        let mut state = DesktopVaultResponseState::default();
        state.apply_success(
            DesktopVaultResponseMode::VaultAudit,
            &serde_json::json!({
                "events": [
                    {"event_id": "evt-1", "kind": "decode", "record_id": "record-1"},
                    {"event_id": "evt-2", "kind": "encode", "record_id": "record-2"}
                ]
            }),
        );

        let json = state
            .audit_events_export_json()
            .expect("audit events export");

        assert_eq!(json["mode"], "vault_audit_events");
        assert_eq!(json["events"][0]["event_id"], "evt-1");
        assert_eq!(json["events"][1]["kind"], "encode");
        assert!(json.get("event_count").is_none());
        assert!(json.get("returned_event_count").is_none());
        assert!(json.get("next_offset").is_none());
    }

    #[test]
    fn write_desktop_audit_events_json_persists_successful_audit_export() {
        let mut state = DesktopVaultResponseState::default();
        state.apply_success(
            DesktopVaultResponseMode::VaultAudit,
            &serde_json::json!({
                "event_count": 1,
                "returned_event_count": 1,
                "events": [{"event_id": "evt-1", "kind": "decode"}],
                "vault_path": "/secret/Alice.vault"
            }),
        );
        let path = std::env::temp_dir().join(format!(
            "mdid-desktop-audit-events-export-{}-persisted.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);

        let returned_path = write_desktop_audit_events_json(&state, &path).expect("write export");
        let persisted = std::fs::read_to_string(&path).expect("read persisted export");
        let persisted_json: serde_json::Value =
            serde_json::from_str(&persisted).expect("persisted JSON parses");
        let _ = std::fs::remove_file(&path);

        assert_eq!(returned_path, path);
        assert_eq!(persisted_json["mode"], "vault_audit_events");
        assert_eq!(persisted_json["event_count"], 1);
        assert_eq!(persisted_json["events"][0]["event_id"], "evt-1");
        assert!(!persisted.contains("/secret"));
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
            audit_limit: None,
            audit_offset: None,
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
            audit_limit: None,
            audit_offset: None,
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
    fn desktop_vault_audit_request_includes_optional_positive_limit() {
        let state = DesktopVaultRequestState {
            mode: DesktopVaultMode::AuditEvents,
            vault_path: "C:/vaults/local.mdid".to_string(),
            vault_passphrase: "correct horse battery staple".to_string(),
            audit_limit: Some(" 25 ".to_string()),
            ..DesktopVaultRequestState::default()
        };

        let request = state
            .try_build_request()
            .expect("audit request with positive limit should build");

        assert_eq!(request.route, "/vault/audit/events");
        assert_eq!(request.body["limit"], 25);
    }

    #[test]
    fn desktop_vault_audit_request_omits_blank_limit() {
        let state = DesktopVaultRequestState {
            mode: DesktopVaultMode::AuditEvents,
            vault_path: "C:/vaults/local.mdid".to_string(),
            vault_passphrase: "correct horse battery staple".to_string(),
            audit_limit: Some("   ".to_string()),
            ..DesktopVaultRequestState::default()
        };

        let request = state
            .try_build_request()
            .expect("audit request with blank limit should build");

        assert!(request.body.get("limit").is_none());
    }

    #[test]
    fn desktop_vault_audit_request_rejects_invalid_limit() {
        let state = DesktopVaultRequestState {
            mode: DesktopVaultMode::AuditEvents,
            vault_path: "C:/vaults/local.mdid".to_string(),
            vault_passphrase: "correct horse battery staple".to_string(),
            audit_limit: Some("not-a-number".to_string()),
            ..DesktopVaultRequestState::default()
        };

        assert!(matches!(
            state.try_build_request(),
            Err(DesktopVaultValidationError::InvalidAuditLimit(_))
        ));
    }

    #[test]
    fn desktop_vault_audit_request_rejects_zero_limit() {
        let state = DesktopVaultRequestState {
            mode: DesktopVaultMode::AuditEvents,
            vault_path: "C:/vaults/local.mdid".to_string(),
            vault_passphrase: "correct horse battery staple".to_string(),
            audit_limit: Some("0".to_string()),
            ..DesktopVaultRequestState::default()
        };

        assert_eq!(
            state.try_build_request(),
            Err(DesktopVaultValidationError::ZeroAuditLimit)
        );
    }

    #[test]
    fn vault_audit_request_includes_positive_offset() {
        let state = DesktopVaultRequestState {
            mode: DesktopVaultMode::AuditEvents,
            vault_path: "C:/vaults/site.mdid".to_string(),
            vault_passphrase: "correct horse battery staple".to_string(),
            audit_limit: Some("25".to_string()),
            audit_offset: Some("50".to_string()),
            ..DesktopVaultRequestState::default()
        };

        let request = state
            .try_build_request()
            .expect("audit request should build");

        assert_eq!(request.route, "/vault/audit/events");
        assert_eq!(request.body["limit"], serde_json::json!(25));
        assert_eq!(request.body["offset"], serde_json::json!(50));
    }

    #[test]
    fn vault_audit_request_omits_blank_and_zero_offset() {
        let blank_state = DesktopVaultRequestState {
            mode: DesktopVaultMode::AuditEvents,
            vault_path: "C:/vaults/site.mdid".to_string(),
            vault_passphrase: "correct horse battery staple".to_string(),
            audit_offset: Some("   ".to_string()),
            ..DesktopVaultRequestState::default()
        };
        let blank_request = blank_state
            .try_build_request()
            .expect("blank offset is omitted");
        assert!(blank_request.body.get("offset").is_none());

        let zero_state = DesktopVaultRequestState {
            mode: DesktopVaultMode::AuditEvents,
            vault_path: "C:/vaults/site.mdid".to_string(),
            vault_passphrase: "correct horse battery staple".to_string(),
            audit_offset: Some("0".to_string()),
            ..DesktopVaultRequestState::default()
        };
        let zero_request = zero_state
            .try_build_request()
            .expect("zero offset is omitted");
        assert!(zero_request.body.get("offset").is_none());
    }

    #[test]
    fn vault_audit_request_rejects_negative_offset_without_echoing_input() {
        let state = DesktopVaultRequestState {
            mode: DesktopVaultMode::AuditEvents,
            vault_path: "C:/vaults/site.mdid".to_string(),
            vault_passphrase: "correct horse battery staple".to_string(),
            audit_offset: Some("-10".to_string()),
            ..DesktopVaultRequestState::default()
        };

        let error = state
            .try_build_request()
            .expect_err("negative offset must fail");

        assert!(matches!(
            error,
            DesktopVaultValidationError::InvalidAuditOffset(_)
        ));
        assert!(!format!("{error:?}").contains("-10"));
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
            audit_limit: None,
            audit_offset: None,
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
    fn desktop_vault_decode_request_rejects_duplicate_record_ids() {
        let state = DesktopVaultRequestState {
            vault_path: "C:/vaults/local.mdid".to_string(),
            vault_passphrase: "correct horse battery staple".to_string(),
            record_ids_json:
                r#"["550e8400-e29b-41d4-a716-446655440000","550e8400-e29b-41d4-a716-446655440000"]"#
                    .to_string(),
            output_target: "review-workbench".to_string(),
            justification: "case review".to_string(),
            ..Default::default()
        };

        let err = state
            .try_build_request()
            .expect_err("desktop must reject duplicate decode record ids");
        let message = format!("{err:?}");
        assert!(message.contains("duplicate record id"));
        assert!(!message.contains("550e8400"));
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
            audit_limit: None,
            audit_offset: None,
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
        assert!(DESKTOP_VAULT_WORKBENCH_COPY.contains("unrelated background workflow behavior"));
    }

    #[test]
    fn desktop_file_import_csv_bytes_map_to_csv_text_payload() {
        let imported =
            DesktopFileImportPayload::from_bytes("patients.csv", b"name\nAlice").unwrap();

        assert_eq!(imported.mode, DesktopWorkflowMode::CsvText);
        assert_eq!(imported.payload, "name\nAlice");
        assert_eq!(imported.source_name.as_deref(), Some("patients.csv"));
    }

    #[test]
    fn desktop_csv_file_import_preserves_source_name_for_save_suggestions() {
        let payload = DesktopFileImportPayload::from_bytes("clinic-export.csv", b"name\nAda\n")
            .expect("csv import should be accepted");

        assert_eq!(payload.mode, DesktopWorkflowMode::CsvText);
        assert_eq!(payload.source_name.as_deref(), Some("clinic-export.csv"));
    }

    #[test]
    fn desktop_file_import_xlsx_bytes_map_to_xlsx_base64_payload() {
        let imported =
            DesktopFileImportPayload::from_bytes("patients.xlsx", b"PK\x03\x04").unwrap();

        assert_eq!(imported.mode, DesktopWorkflowMode::XlsxBase64);
        assert_eq!(imported.payload, "UEsDBA==");
        assert_eq!(imported.source_name.as_deref(), Some("patients.xlsx"));
    }

    #[test]
    fn desktop_xlsx_file_import_preserves_source_name_for_save_suggestions() {
        let payload = DesktopFileImportPayload::from_bytes("clinic-export.xlsx", b"not-real-xlsx")
            .expect("xlsx helper import should accept bytes before runtime validation");

        assert_eq!(payload.mode, DesktopWorkflowMode::XlsxBase64);
        assert_eq!(payload.source_name.as_deref(), Some("clinic-export.xlsx"));
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
    fn portable_artifact_json_file_import_targets_inspect_mode() {
        let imported = DesktopFileImportPayload::from_bytes_target(
            "patient-123.mdid-portable.json",
            br#"{"version":1,"artifact":{"ciphertext":"secret-patient-ciphertext"}}"#,
        )
        .expect("portable artifact json imports should be accepted");

        match imported {
            DesktopFileImportTarget::PortableArtifactInspect(payload) => {
                assert_eq!(payload.mode, DesktopPortableMode::InspectArtifact);
                assert_eq!(
                    payload.artifact_json,
                    r#"{"version":1,"artifact":{"ciphertext":"secret-patient-ciphertext"}}"#
                );
                assert_eq!(payload.source_name, "patient-123.mdid-portable.json");
            }
            other => panic!("expected portable inspect import target, got {other:?}"),
        }
    }

    #[test]
    fn desktop_portable_file_import_payload_supports_import_mode() {
        let payload = DesktopPortableFileImportPayload::from_bytes_for_mode(
            DesktopPortableMode::ImportArtifact,
            "handoff.mdid-portable.json",
            br#"{\"version\":1,\"records\":[]}"#,
        )
        .expect("portable import handoff should accept artifact json");

        assert_eq!(payload.mode, DesktopPortableMode::ImportArtifact);
        assert_eq!(payload.artifact_json, r#"{\"version\":1,\"records\":[]}"#);
        assert_eq!(payload.source_name, "handoff.mdid-portable.json");
    }

    #[test]
    fn desktop_portable_file_import_payload_rejects_export_mode() {
        let error = DesktopPortableFileImportPayload::from_bytes_for_mode(
            DesktopPortableMode::VaultExport,
            "handoff.mdid-portable.json",
            br#"{\"version\":1}"#,
        )
        .expect_err("vault export is not an artifact-consuming mode");

        assert_eq!(error, DesktopFileImportError::UnsupportedFileType);
    }

    #[test]
    fn exact_browser_portable_artifact_filename_targets_inspect_mode() {
        let imported = DesktopFileImportPayload::from_bytes_target(
            "mdid-browser-portable-artifact.json",
            br#"{"artifact":{"ciphertext":"secret"}}"#,
        )
        .expect("browser portable artifact export names should be accepted");

        assert!(matches!(
            imported,
            DesktopFileImportTarget::PortableArtifactInspect(_)
        ));
    }

    #[test]
    fn generic_json_file_import_still_uses_media_metadata_mode() {
        let imported = DesktopFileImportPayload::from_bytes_target(
            "local-media-metadata.json",
            b"{\"artifact_label\":\"scan.png\",\"format\":\"image\",\"metadata\":[]}",
        )
        .expect("generic json metadata imports should still be accepted");

        match imported {
            DesktopFileImportTarget::Workflow(payload) => {
                assert_eq!(payload.mode, DesktopWorkflowMode::MediaMetadataJson);
                assert_eq!(
                    payload.source_name.as_deref(),
                    Some("local-media-metadata.json")
                );
            }
            other => panic!("expected workflow import target, got {other:?}"),
        }
    }

    #[test]
    fn portable_artifact_file_import_debug_redacts_artifact_contents() {
        let imported = DesktopFileImportPayload::from_bytes_target(
            "patient-123-mrn-456-m did-portable.json".replace(" ", ""),
            br#"{"artifact":{"ciphertext":"secret-patient-ciphertext"}}"#,
        )
        .expect("portable artifact json imports should be accepted");

        let debug = format!("{imported:?}");

        assert!(debug.contains("PortableArtifactInspect"));
        assert!(!debug.contains("secret-patient-ciphertext"));
        assert!(!debug.contains("patient-123"));
        assert!(!debug.contains("mrn-456"));
    }

    #[test]
    fn desktop_file_import_rejects_unsupported_file_type() {
        let error = DesktopFileImportPayload::from_bytes("notes.txt", b"name\nAlice").unwrap_err();

        assert_eq!(error, DesktopFileImportError::UnsupportedFileType);
    }

    #[test]
    fn workflow_only_file_import_rejects_portable_artifact_json_names() {
        let error = DesktopFileImportPayload::from_bytes(
            "patient-123.mdid-portable.json",
            br#"{"artifact":{"ciphertext":"secret"}}"#,
        )
        .unwrap_err();

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
        assert_eq!(state.source_name, "patients.csv");
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
        assert!(disclosure.contains("stays limited to this local"));
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
        let disclosure = state.mode.disclosure();
        assert!(disclosure.contains("first non-empty worksheet"));
        assert!(disclosure.contains("Sheet selection is not supported"));
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
        assert!(disclosure.contains("stays limited to this local"));
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
        assert!(disclosure.contains("stays limited to this local"));
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
    fn dicom_workflow_scope_note_discloses_no_pixel_redaction() {
        let disclosure = DesktopWorkflowMode::DicomBase64.disclosure();

        assert!(disclosure.contains("tag-level DICOM de-identification"));
        assert!(disclosure.contains("DICOM pixel data was not inspected or redacted; burned-in annotations require separate visual review."));
        assert!(!disclosure.contains("visual redaction is performed"));
    }

    #[test]
    fn runtime_submission_snapshot_drives_button_enabled_state_and_label() {
        let idle = DesktopRuntimeSubmissionSnapshot::idle();
        assert!(!idle.submit_button_disabled());
        assert_eq!(idle.submit_button_label(), "Submit to local runtime");
        assert_eq!(idle.progress_banner(), None);

        let started = DesktopRuntimeSubmissionSnapshot::started(
            DesktopRuntimeSubmissionMode::Workflow(DesktopWorkflowMode::PdfBase64Review),
        );
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
    fn pdf_review_report_download_exports_structured_json_without_pdf_bytes() {
        let mut state = DesktopWorkflowResponseState::default();
        state.apply_success_json(
            DesktopWorkflowMode::PdfBase64Review,
            json!({
                "summary": {"pages": 1, "ocr_required": false},
                "review_queue": [{"page": 1, "reason": "text-layer review"}],
                "rewritten_pdf_bytes_base64": null,
                "debug_raw_text": "Alice Patient"
            }),
        );

        let download = state
            .review_report_download(DesktopWorkflowMode::PdfBase64Review)
            .expect("pdf review success should create a structured review report download");

        assert_eq!(download.file_name, "desktop-pdf-review-report.json");
        let text = std::str::from_utf8(&download.bytes).expect("report is utf8 json");
        let report: serde_json::Value = serde_json::from_str(text).expect("report parses");
        assert_eq!(report["mode"], "pdf_review");
        assert_eq!(
            report["summary"],
            json!({"pages": 1, "ocr_required": false})
        );
        assert_eq!(
            report["review_queue"],
            json!([{"page": 1, "reason": "redacted-review-note"}])
        );
        assert!(report.get("rewritten_pdf_bytes_base64").is_none());
        assert!(report.get("debug_raw_text").is_none());
        assert!(!text.contains("Alice Patient"));
    }

    #[test]
    fn desktop_review_report_download_for_source_uses_safe_pdf_and_media_filenames() {
        let mut pdf_state = DesktopWorkflowResponseState::default();
        pdf_state.apply_success_json(
            DesktopWorkflowMode::PdfBase64Review,
            serde_json::json!({
                "summary": { "total_pages": 1, "pages_requiring_review": 1 },
                "review_queue": [{ "page_index": 0, "reason": "ocr_required" }],
                "rewritten_pdf_bytes_base64": null,
            }),
        );

        let pdf_report = pdf_state
            .review_report_download_for_source(
                DesktopWorkflowMode::PdfBase64Review,
                Some("C:\\clinic\\March intake.pdf"),
            )
            .expect("pdf review report should be exportable");
        assert_eq!(pdf_report.file_name, "March-intake-pdf-review-report.json");

        let mut media_state = DesktopWorkflowResponseState::default();
        media_state.apply_success_json(
            DesktopWorkflowMode::MediaMetadataJson,
            serde_json::json!({
                "summary": { "metadata_fields_reviewed": 2, "metadata_fields_requiring_review": 1 },
                "review_queue": [{ "field_index": 0, "reason": "metadata_identifier" }],
                "rewritten_media_bytes_base64": null,
            }),
        );

        let media_report = media_state
            .review_report_download_for_source(
                DesktopWorkflowMode::MediaMetadataJson,
                Some("/uploads/Camera Roll.metadata.json"),
            )
            .expect("media review report should be exportable");
        assert_eq!(
            media_report.file_name,
            "Camera-Roll.metadata-media-review-report.json"
        );
    }

    #[test]
    fn media_review_report_download_exports_structured_json_without_metadata_phi() {
        let mut state = DesktopWorkflowResponseState::default();
        state.apply_success_json(
            DesktopWorkflowMode::MediaMetadataJson,
            serde_json::json!({
                "summary": {
                    "total_items": 1,
                    "metadata_only_items": 1,
                    "visual_review_required_items": 1,
                    "unsupported_items": 0,
                    "review_required_candidates": 1,
                    "artifact_count": 1,
                    "metadata_entry_count": 2,
                    "candidate_count": 1,
                    "review_required_count": 1,
                    "unsupported_payload_count": 0,
                    "artifact_label": "Patient-Jane-Doe-face-photo.jpg",
                    "free_text_note": "MRN-12345"
                },
                "review_queue": [{
                    "kind": "conservative_media",
                    "format": "image",
                    "status": "ocr_or_visual_review_required",
                    "action": "review",
                    "phi_type": "metadata_identifier",
                    "metadata_key": "PatientName",
                    "artifact_label": "Patient-Jane-Doe-face-photo.jpg",
                    "field_ref": {
                        "artifact_label": "Patient-Jane-Doe-face-photo.jpg",
                        "metadata_key": "PatientName"
                    },
                    "source_value": "Jane Doe MRN-12345"
                }],
                "raw_body": {"patient": "Jane Doe"},
                "rewritten_media_bytes_base64": "SlBFRyBCWVRFUw=="
            }),
        );

        let download = state
            .review_report_download(DesktopWorkflowMode::MediaMetadataJson)
            .expect("media metadata review response should produce structured report");

        assert_eq!(download.file_name, "desktop-media-review-report.json");
        let rendered = std::str::from_utf8(&download.bytes).unwrap();
        let report: serde_json::Value = serde_json::from_str(rendered).unwrap();
        assert_eq!(report["mode"], "media_metadata_json");
        assert_eq!(report["summary"]["total_items"], 1);
        assert_eq!(report["summary"]["metadata_only_items"], 1);
        assert_eq!(report["summary"]["visual_review_required_items"], 1);
        assert_eq!(report["summary"]["unsupported_items"], 0);
        assert_eq!(report["summary"]["review_required_candidates"], 1);
        assert_eq!(report["summary"]["artifact_count"], 1);
        assert_eq!(report["summary"]["candidate_count"], 1);
        assert_eq!(report["review_queue"][0]["kind"], "conservative_media");
        assert_eq!(report["review_queue"][0]["format"], "image");
        assert_eq!(
            report["review_queue"][0]["status"],
            "ocr_or_visual_review_required"
        );
        assert_eq!(report["review_queue"][0]["phi_type"], "metadata_identifier");
        assert_eq!(
            report["review_queue"][0]["field_ref"]["artifact_label"],
            "redacted-artifact"
        );
        assert_eq!(
            report["review_queue"][0]["field_ref"]["metadata_key"],
            "redacted-field"
        );

        assert!(!rendered.contains("Jane Doe"));
        assert!(!rendered.contains("MRN-12345"));
        assert!(!rendered.contains("PatientName"));
        assert!(!rendered.contains("Patient-Jane-Doe"));
        assert!(rendered.contains("field_ref"));
        assert!(!rendered.contains("dicom.PatientName"));
        assert!(!rendered.contains("raw_body"));
        assert!(!rendered.contains("rewritten_media_bytes"));
    }

    #[test]
    fn pdf_review_report_download_sanitizes_review_queue_phi_fields() {
        let mut state = DesktopWorkflowResponseState::default();
        state.apply_success_json(
            DesktopWorkflowMode::PdfBase64Review,
            json!({
                "summary": {"pages": 1, "ocr_required": true},
                "review_queue": [{
                    "page": 1,
                    "field": "Alice Patient name field",
                    "reason": "possible Alice Patient DOB 1/2/1934",
                    "message": "call Alice Patient at 555-1212",
                    "status": "review_required",
                    "source_text": "Alice Patient",
                    "raw_text": "Alice Patient DOB 1/2/1934",
                    "value": "Alice Patient",
                    "metadata": {"capture_path": "/tmp/Alice Patient.pdf"},
                    "debug": {"text": "Alice Patient"},
                    "nested": {"message": "safe nested message", "source_text": "Alice Patient"}
                }],
                "rewritten_pdf_bytes_base64": null
            }),
        );

        let download = state
            .review_report_download(DesktopWorkflowMode::PdfBase64Review)
            .expect("pdf review report download");

        let text = std::str::from_utf8(&download.bytes).expect("report is utf8 json");
        assert!(!text.contains("Alice Patient"));
        assert!(!text.contains("555-1212"));
        assert!(!text.contains("source_text"));
        assert!(!text.contains("raw_text"));
        assert!(!text.contains("value"));
        assert!(!text.contains("metadata"));
        assert!(!text.contains("debug"));
        let report: serde_json::Value = serde_json::from_str(text).expect("report parses");
        assert_eq!(
            report["review_queue"],
            json!([{
                "field": "redacted-field",
                "message": "redacted-review-note",
                "page": 1,
                "reason": "redacted-review-note",
                "status": "review_required"
            }])
        );
    }

    #[test]
    fn pdf_review_report_download_redacts_summary_free_text() {
        let mut state = DesktopWorkflowResponseState::default();
        state.apply_success_json(
            DesktopWorkflowMode::PdfBase64Review,
            json!({
                "summary": {
                    "source_text": 1,
                    "metadata": {"pages": 1},
                    "status": "Alice-Patient",
                    "pages": 1,
                    "ocr_required": false,
                    "note": "Alice Patient reviewed on 1/2/1934",
                    "source": "Alice Patient intake.pdf",
                    "path": "/tmp/Alice Patient intake.pdf",
                    "nested": {"count": 1, "note": "Alice Patient nested note"}
                },
                "review_queue": [],
                "rewritten_pdf_bytes_base64": null
            }),
        );

        let download = state
            .review_report_download(DesktopWorkflowMode::PdfBase64Review)
            .expect("pdf review report download");

        let text = std::str::from_utf8(&download.bytes).expect("report is utf8 json");
        assert!(!text.contains("Alice"));
        assert!(!text.contains("source_text"));
        assert!(!text.contains("metadata"));
        assert!(!text.contains("status"));
        assert!(!text.contains("intake.pdf"));
        let report: serde_json::Value = serde_json::from_str(text).expect("report parses");
        assert_eq!(
            report["summary"],
            json!({
                "ocr_required": false,
                "pages": 1
            })
        );
    }

    #[test]
    fn pdf_review_report_download_drops_arbitrary_numeric_summary_phi_keys() {
        let mut state = DesktopWorkflowResponseState::default();
        state.apply_success_json(
            DesktopWorkflowMode::PdfBase64Review,
            json!({
                "summary": {
                    "pages": 2,
                    "page_count": 2,
                    "AlicePatient": 1,
                    "MRN12345": 12345,
                    "nested": {"AlicePatient": 1, "pages": 2}
                },
                "review_queue": [],
                "rewritten_pdf_bytes_base64": null
            }),
        );

        let download = state
            .review_report_download(DesktopWorkflowMode::PdfBase64Review)
            .expect("pdf review report download");

        let text = std::str::from_utf8(&download.bytes).expect("report is utf8 json");
        assert!(!text.contains("AlicePatient"));
        assert!(!text.contains("MRN12345"));
        assert!(!text.contains("nested"));
        let report: serde_json::Value = serde_json::from_str(text).expect("report parses");
        assert_eq!(report["summary"], json!({"page_count": 2, "pages": 2}));
    }

    #[test]
    fn pdf_review_report_download_drops_enum_like_phi_review_queue_values() {
        let mut state = DesktopWorkflowResponseState::default();
        state.apply_success_json(
            DesktopWorkflowMode::PdfBase64Review,
            json!({
                "summary": {"pages": 1},
                "review_queue": [{
                    "page": 1,
                    "status": "AlicePatient",
                    "kind": "MRN12345",
                    "action": "PatientDOB1934",
                    "phi_type": "Phone5551212",
                    "field": "Alice Patient",
                    "reason": "Alice Patient"
                }],
                "rewritten_pdf_bytes_base64": null
            }),
        );

        let download = state
            .review_report_download(DesktopWorkflowMode::PdfBase64Review)
            .expect("pdf review report download");

        let text = std::str::from_utf8(&download.bytes).expect("report is utf8 json");
        assert!(!text.contains("AlicePatient"));
        assert!(!text.contains("MRN12345"));
        assert!(!text.contains("PatientDOB1934"));
        assert!(!text.contains("Phone5551212"));
        let report: serde_json::Value = serde_json::from_str(text).expect("report parses");
        assert_eq!(
            report["review_queue"],
            json!([{"field": "redacted-field", "page": 1, "reason": "redacted-review-note"}])
        );
    }

    #[test]
    fn review_report_download_fails_closed_for_stale_error_and_non_review_modes() {
        let mut state = DesktopWorkflowResponseState::default();
        assert_eq!(
            state.review_report_download(DesktopWorkflowMode::PdfBase64Review),
            None
        );

        state.apply_success_json(
            DesktopWorkflowMode::CsvText,
            json!({"csv":"ok","summary":{},"review_queue":[]}),
        );
        assert_eq!(
            state.review_report_download(DesktopWorkflowMode::CsvText),
            None
        );
        assert_eq!(
            state.review_report_download(DesktopWorkflowMode::PdfBase64Review),
            None
        );

        state.apply_success_json(
            DesktopWorkflowMode::PdfBase64Review,
            json!({"summary":{},"review_queue":[]}),
        );
        state.apply_error("runtime failed");
        assert_eq!(
            state.review_report_download(DesktopWorkflowMode::PdfBase64Review),
            None
        );

        state.apply_success_json(
            DesktopWorkflowMode::MediaMetadataJson,
            json!({"summary":{"total_items":1},"review_queue":[]}),
        );
        assert_eq!(
            state.review_report_download(DesktopWorkflowMode::PdfBase64Review),
            None
        );
        state.apply_error("media runtime failed");
        assert_eq!(
            state.review_report_download(DesktopWorkflowMode::MediaMetadataJson),
            None
        );
    }

    #[test]
    fn workflow_output_download_extracts_csv_bytes_without_raw_envelope() {
        let mut response = DesktopWorkflowResponseState::default();
        response.apply_success_json(
            DesktopWorkflowMode::CsvText,
            json!({
                "csv": "patient_name\n<NAME-1>",
                "summary": {"encoded_fields": 1},
                "review_queue": []
            }),
        );

        let download = response
            .workflow_output_download(DesktopWorkflowMode::CsvText)
            .expect("csv output download");

        assert_eq!(download.file_name, "desktop-deidentified.csv");
        assert_eq!(download.bytes, b"patient_name\n<NAME-1>");
        assert!(!download.bytes.starts_with(b"{"));
        let debug = format!("{download:?}");
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("patient_name"));
        assert!(!debug.contains("NAME-1"));
    }

    #[test]
    fn write_workflow_output_file_writes_bytes_without_exposing_phi_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir
            .path()
            .join("patient-jane-doe-mrn-12345-deidentified.csv");
        let download = DesktopWorkflowOutputDownload {
            file_name: "desktop-deidentified.csv".to_string(),
            bytes: b"patient_name\n<NAME-1>\n".to_vec(),
        };

        write_workflow_output_file(&path, &download).expect("workflow output saved");

        assert_eq!(
            std::fs::read(&path).expect("saved bytes readable"),
            b"patient_name\n<NAME-1>\n"
        );
    }

    #[test]
    fn write_workflow_output_file_error_is_phi_safe() {
        let path = std::env::temp_dir()
            .join("missing-parent-jane-doe-mrn-12345")
            .join("output.csv");
        let download = DesktopWorkflowOutputDownload {
            file_name: "desktop-deidentified.csv".to_string(),
            bytes: b"patient_name\n<NAME-1>\n".to_vec(),
        };

        let error = write_workflow_output_file(&path, &download).expect_err("write fails");

        assert_eq!(
            error,
            "workflow output save failed: unable to write output file"
        );
        assert!(!error.contains("jane-doe"));
        assert!(!error.contains("12345"));
        assert!(!error.contains(path.to_string_lossy().as_ref()));
    }

    #[test]
    fn workflow_output_download_extracts_xlsx_and_dicom_base64_bytes() {
        let mut xlsx = DesktopWorkflowResponseState::default();
        xlsx.apply_success_json(
            DesktopWorkflowMode::XlsxBase64,
            json!({"rewritten_workbook_base64":"UEsDBAo=","summary":{},"review_queue":[]}),
        );
        let xlsx_download = xlsx
            .workflow_output_download(DesktopWorkflowMode::XlsxBase64)
            .expect("xlsx output download");
        assert_eq!(xlsx_download.file_name, "desktop-deidentified.xlsx");
        assert_eq!(xlsx_download.bytes, b"PK\x03\x04\n");

        let mut dicom = DesktopWorkflowResponseState::default();
        dicom.apply_success_json(
            DesktopWorkflowMode::DicomBase64,
            json!({"rewritten_dicom_bytes_base64":"RElDTQAB","summary":{},"review_queue":[]}),
        );
        let dicom_download = dicom
            .workflow_output_download(DesktopWorkflowMode::DicomBase64)
            .expect("dicom output download");
        assert_eq!(dicom_download.file_name, "desktop-deidentified.dcm");
        assert_eq!(dicom_download.bytes, b"DICM\x00\x01");
    }

    #[test]
    fn workflow_output_download_fails_closed_for_pdf_errors_malformed_and_mode_mismatch() {
        let mut pdf = DesktopWorkflowResponseState::default();
        pdf.apply_success_json(
            DesktopWorkflowMode::PdfBase64Review,
            json!({"rewritten_pdf_bytes_base64":"JVBERi0x","summary":{},"review_queue":[]}),
        );
        assert_eq!(
            pdf.workflow_output_download(DesktopWorkflowMode::PdfBase64Review),
            None
        );

        let mut errored = DesktopWorkflowResponseState::default();
        errored.apply_success_json(
            DesktopWorkflowMode::CsvText,
            json!({"csv":"ok","summary":{},"review_queue":[]}),
        );
        errored.apply_error("runtime failed");
        assert_eq!(
            errored.workflow_output_download(DesktopWorkflowMode::CsvText),
            None
        );

        let mut malformed = DesktopWorkflowResponseState::default();
        malformed.apply_success_json(
            DesktopWorkflowMode::XlsxBase64,
            json!({"rewritten_workbook_base64":"not valid base64!","summary":{},"review_queue":[]}),
        );
        assert_eq!(
            malformed.workflow_output_download(DesktopWorkflowMode::XlsxBase64),
            None
        );

        let mut non_canonical_padded = DesktopWorkflowResponseState::default();
        non_canonical_padded.apply_success_json(
            DesktopWorkflowMode::DicomBase64,
            json!({"rewritten_dicom_bytes_base64":"/x==","summary":{},"review_queue":[]}),
        );
        assert_eq!(
            non_canonical_padded.workflow_output_download(DesktopWorkflowMode::DicomBase64),
            None
        );

        let mut missing = DesktopWorkflowResponseState::default();
        missing.apply_success_json(
            DesktopWorkflowMode::CsvText,
            json!({"summary":{},"review_queue":[]}),
        );
        assert_eq!(
            missing.workflow_output_download(DesktopWorkflowMode::CsvText),
            None
        );

        let mut csv = DesktopWorkflowResponseState::default();
        csv.apply_success_json(
            DesktopWorkflowMode::CsvText,
            json!({"csv":"ok","summary":{},"review_queue":[]}),
        );
        assert_eq!(
            csv.workflow_output_download(DesktopWorkflowMode::XlsxBase64),
            None
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
    fn desktop_export_filename_uses_import_source_stem_for_csv_xlsx_and_dicom() {
        let state = DesktopWorkflowResponseState {
            output: "rewritten payload".to_string(),
            ..DesktopWorkflowResponseState::default()
        };

        assert_eq!(
            state.suggested_export_file_name_for_source(
                DesktopWorkflowMode::CsvText,
                Some("/clinic/intake/patient list.csv")
            ),
            Some("patient-list-deidentified.csv".to_string())
        );
        assert_eq!(
            state.suggested_export_file_name_for_source(
                DesktopWorkflowMode::XlsxBase64,
                Some("C:\\clinic\\April Census.xlsx")
            ),
            Some("April-Census-deidentified.xlsx.base64.txt".to_string())
        );
        assert_eq!(
            state.suggested_export_file_name_for_source(
                DesktopWorkflowMode::DicomBase64,
                Some("brain scan.dcm")
            ),
            Some("brain-scan-deidentified.dcm.base64.txt".to_string())
        );
    }

    #[test]
    fn desktop_export_filename_caps_source_stem_at_64_sanitized_chars() {
        let state = DesktopWorkflowResponseState {
            output: "rewritten payload".to_string(),
            ..DesktopWorkflowResponseState::default()
        };
        let source_name = format!("{}.csv", "a".repeat(80));
        let expected = format!("{}-deidentified.csv", "a".repeat(64));

        assert_eq!(
            state.suggested_export_file_name_for_source(
                DesktopWorkflowMode::CsvText,
                Some(&source_name)
            ),
            Some(expected)
        );
    }

    #[test]
    fn desktop_export_filename_falls_back_when_source_is_empty_or_unsafe() {
        let state = DesktopWorkflowResponseState {
            output: "rewritten payload".to_string(),
            ..DesktopWorkflowResponseState::default()
        };

        assert_eq!(
            state.suggested_export_file_name_for_source(DesktopWorkflowMode::CsvText, None),
            Some("desktop-deidentified.csv".to_string())
        );
        assert_eq!(
            state.suggested_export_file_name_for_source(
                DesktopWorkflowMode::CsvText,
                Some("///.csv")
            ),
            Some("desktop-deidentified.csv".to_string())
        );
        assert_eq!(
            state.suggested_export_file_name_for_source(
                DesktopWorkflowMode::PdfBase64Review,
                Some("report.pdf")
            ),
            None
        );
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
