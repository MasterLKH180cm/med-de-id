use base64::Engine;
use leptos::*;
use serde::{Deserialize, Serialize};
use std::fmt;

const DEFAULT_FIELD_POLICY_JSON: &str = "[\n  {\n    \"header\": \"patient_id\",\n    \"phi_type\": \"patient_id\",\n    \"action\": \"encode\"\n  },\n  {\n    \"header\": \"patient_name\",\n    \"phi_type\": \"patient_name\",\n    \"action\": \"review\"\n  }\n]";
const IDLE_SUMMARY: &str = "Awaiting submission.";
const IDLE_REVIEW_QUEUE: &str = "No review items yet.";
#[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
const MAX_BROWSER_IMPORT_BYTES: u64 = 10 * 1024 * 1024;
const BROWSER_FILE_IMPORT_COPY: &str = "Bounded browser file import: CSV files load as text; media metadata JSON files also load as text; XLSX and PDF files load as base64 payloads for existing localhost runtime routes; DICOM files also load as base64 payloads for the existing DICOM runtime route. Media metadata JSON sends metadata only, not media bytes. This does not add OCR, visual redaction, vault browsing, or auth/session.";
#[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
const MAX_IMPORT_DERIVED_EXPORT_STEM_CHARS: usize = 64;
#[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
const EXPORT_FILENAME_WARNING_COPY: &str = "Browser suggested download filenames for imported files are derived from imported filenames for local UX only. If an original filename contains PHI, rename the downloaded file locally before sharing or storing it.";
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
const FETCH_UNAVAILABLE_MESSAGE: &str =
    "Runtime submission is only available from a wasm32 browser build.";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InputMode {
    CsvText,
    XlsxBase64,
    PdfBase64,
    DicomBase64,
    MediaMetadataJson,
    VaultAuditEvents,
    VaultDecode,
    VaultExport,
    PortableArtifactInspect,
    PortableArtifactImport,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
enum BrowserFileReadMode {
    Text,
    DataUrlBase64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BrowserDownloadPayload {
    file_name: String,
    mime_type: &'static str,
    bytes: Vec<u8>,
    is_text: bool,
}

impl InputMode {
    fn redacts_runtime_error_details(self) -> bool {
        matches!(
            self,
            Self::VaultAuditEvents
                | Self::VaultDecode
                | Self::VaultExport
                | Self::PortableArtifactInspect
                | Self::PortableArtifactImport
        )
    }

    #[cfg_attr(not(test), allow(dead_code))]
    fn from_file_name(file_name: &str) -> Option<Self> {
        let file_name = file_name.to_lowercase();

        if file_name.ends_with(".csv") {
            Some(Self::CsvText)
        } else if file_name.ends_with(".xlsx") {
            Some(Self::XlsxBase64)
        } else if file_name.ends_with(".pdf") {
            Some(Self::PdfBase64)
        } else if file_name.ends_with(".dcm") || file_name.ends_with(".dicom") {
            Some(Self::DicomBase64)
        } else if file_name == "mdid-browser-portable-artifact.json"
            || file_name.ends_with(".mdid-portable.json")
            || file_name.ends_with("-mdid-portable.json")
        {
            Some(Self::PortableArtifactInspect)
        } else if file_name.ends_with(".json") {
            Some(Self::MediaMetadataJson)
        } else {
            None
        }
    }

    fn from_select_value(value: &str) -> Self {
        match value {
            "xlsx-base64" => Self::XlsxBase64,
            "pdf-base64" => Self::PdfBase64,
            "dicom-base64" => Self::DicomBase64,
            "media-metadata-json" => Self::MediaMetadataJson,
            "vault-audit-events" => Self::VaultAuditEvents,
            "vault-decode" => Self::VaultDecode,
            "vault-export" => Self::VaultExport,
            "portable-artifact-inspect" => Self::PortableArtifactInspect,
            "portable-artifact-import" => Self::PortableArtifactImport,
            _ => Self::CsvText,
        }
    }

    fn select_value(self) -> &'static str {
        match self {
            Self::CsvText => "csv-text",
            Self::XlsxBase64 => "xlsx-base64",
            Self::PdfBase64 => "pdf-base64",
            Self::DicomBase64 => "dicom-base64",
            Self::MediaMetadataJson => "media-metadata-json",
            Self::VaultAuditEvents => "vault-audit-events",
            Self::VaultDecode => "vault-decode",
            Self::VaultExport => "vault-export",
            Self::PortableArtifactInspect => "portable-artifact-inspect",
            Self::PortableArtifactImport => "portable-artifact-import",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::CsvText => "CSV text",
            Self::XlsxBase64 => "XLSX base64",
            Self::PdfBase64 => "PDF base64",
            Self::DicomBase64 => "DICOM base64",
            Self::MediaMetadataJson => "Media metadata JSON",
            Self::VaultAuditEvents => "Vault audit events",
            Self::VaultDecode => "Vault decode",
            Self::VaultExport => "Vault export",
            Self::PortableArtifactInspect => "Portable artifact inspect",
            Self::PortableArtifactImport => "Portable artifact import",
        }
    }

    fn safe_vault_report_mode_label(self) -> &'static str {
        match self {
            Self::VaultAuditEvents => "vault_audit",
            Self::VaultDecode => "vault_decode",
            Self::PortableArtifactInspect => "portable_artifact_inspect",
            Self::PortableArtifactImport => "portable_artifact_import",
            _ => self.label(),
        }
    }

    fn payload_hint(self) -> &'static str {
        match self {
            Self::CsvText => "Paste CSV rows here",
            Self::XlsxBase64 => "Paste base64-encoded XLSX content here",
            Self::PdfBase64 => "Paste base64-encoded PDF content here",
            Self::DicomBase64 => "Paste base64-encoded DICOM content here",
            Self::MediaMetadataJson => "Paste media metadata JSON here",
            Self::VaultAuditEvents => "Vault audit request fields are rendered by the browser form",
            Self::VaultDecode => "Vault decode request fields are rendered by the browser form",
            Self::VaultExport => "Vault export request fields are rendered by the browser form",
            Self::PortableArtifactInspect => "Paste portable artifact JSON here",
            Self::PortableArtifactImport => {
                "Portable artifact import request fields are rendered by the browser form"
            }
        }
    }

    fn disclosure_copy(self) -> Option<&'static str> {
        match self {
            Self::CsvText => None,
            Self::XlsxBase64 => Some(
                "XLSX mode only processes the first non-empty worksheet. Sheet selection is not supported in this browser flow.",
            ),
            Self::PdfBase64 => Some("PDF mode is review-only: it reports text-layer candidates and OCR-required pages, but does not perform OCR, visual redaction, handwriting handling, or PDF rewrite/export."),
            Self::DicomBase64 => Some("DICOM mode uses the existing local runtime tag-level de-identification route, removes private tags, and returns rewritten DICOM bytes as base64 text. DICOM pixel data was not inspected or redacted; burned-in annotations require separate visual review. It does not add pixel redaction, OCR, vault browsing, auth/session, or broader platform workflow semantics."),
            Self::MediaMetadataJson => Some("Media metadata JSON mode is metadata-only: it sends a JSON object to the local media review runtime route, does not perform OCR, does not upload media bytes, and does not perform visual redaction or media rewrite/export."),
            Self::VaultAuditEvents => Some("Vault audit events mode uses the existing read-only localhost runtime endpoint with bounded optional kind, actor, and limit filters. It does not decode, export, browse vault contents, or add auth/session semantics."),
            Self::VaultDecode => Some("Vault decode mode sends explicit record ids to the existing localhost runtime endpoint. It does not browse vault contents, does not export vault contents, does not add auth/session, and does not add broader workflow behavior."),
            Self::VaultExport | Self::PortableArtifactInspect | Self::PortableArtifactImport => Some("Bounded localhost portable artifact request surfaces only. This is not vault browsing, decoded-value display, generalized transfer workflow, auth/session, or broader platform workflow functionality."),
        }
    }

    fn endpoint(self) -> &'static str {
        match self {
            Self::CsvText => "/tabular/deidentify",
            Self::XlsxBase64 => "/tabular/deidentify/xlsx",
            Self::PdfBase64 => "/pdf/deidentify",
            Self::DicomBase64 => "/dicom/deidentify",
            Self::MediaMetadataJson => "/media/conservative/deidentify",
            Self::VaultAuditEvents => "/vault/audit/events",
            Self::VaultDecode => "/vault/decode",
            Self::VaultExport => "/vault/export",
            Self::PortableArtifactInspect => "/portable-artifacts/inspect",
            Self::PortableArtifactImport => "/portable-artifacts/import",
        }
    }

    fn source_name_label(self) -> &'static str {
        match self {
            Self::PdfBase64 => "PDF",
            Self::DicomBase64 => "DICOM base64",
            Self::MediaMetadataJson => "media metadata JSON",
            _ => self.label(),
        }
    }

    fn requires_source_name(self) -> bool {
        matches!(
            self,
            Self::PdfBase64 | Self::DicomBase64 | Self::MediaMetadataJson
        )
    }

    fn requires_field_policy(self) -> bool {
        matches!(self, Self::CsvText | Self::XlsxBase64)
    }

    #[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
    fn browser_file_read_mode(self) -> BrowserFileReadMode {
        match self {
            Self::CsvText
            | Self::MediaMetadataJson
            | Self::VaultAuditEvents
            | Self::VaultDecode
            | Self::VaultExport
            | Self::PortableArtifactInspect
            | Self::PortableArtifactImport => BrowserFileReadMode::Text,
            Self::XlsxBase64 | Self::PdfBase64 | Self::DicomBase64 => {
                BrowserFileReadMode::DataUrlBase64
            }
        }
    }
}

#[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
fn file_import_payload_from_data_url(data_url: &str) -> String {
    data_url
        .split_once(',')
        .map(|(_, payload)| payload)
        .unwrap_or(data_url)
        .to_string()
}

#[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
fn validate_browser_import_size(file_size_bytes: u64) -> Result<(), String> {
    if file_size_bytes > MAX_BROWSER_IMPORT_BYTES {
        Err("Browser import file is too large for the bounded local browser flow.".to_string())
    } else {
        Ok(())
    }
}

#[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
fn build_vault_audit_request_payload(
    vault_path: &str,
    vault_passphrase: &str,
    kind: &str,
    actor: &str,
    limit: &str,
    offset: &str,
) -> Result<serde_json::Value, String> {
    let vault_path = vault_path.trim();
    if vault_path.is_empty() {
        return Err("Vault path is required before submitting.".to_string());
    }

    let vault_passphrase = vault_passphrase.trim();
    if vault_passphrase.is_empty() {
        return Err("Vault passphrase is required before submitting.".to_string());
    }

    let mut payload = serde_json::json!({
        "vault_path": vault_path,
        "vault_passphrase": vault_passphrase,
    });

    if let Some(object) = payload.as_object_mut() {
        let kind = kind.trim();
        if !kind.is_empty() {
            object.insert("kind".to_string(), serde_json::json!(kind));
        }

        let actor = actor.trim();
        if !actor.is_empty() {
            object.insert("actor".to_string(), serde_json::json!(actor));
        }

        let limit = limit.trim();
        if !limit.is_empty() {
            let parsed_limit = limit
                .parse::<usize>()
                .map_err(|_| "Vault audit limit must be a positive integer.".to_string())?;
            if parsed_limit == 0 {
                return Err("Vault audit limit must be a positive integer.".to_string());
            }
            object.insert("limit".to_string(), serde_json::json!(parsed_limit));
        }

        let offset = offset.trim();
        if !offset.is_empty() {
            let parsed_offset = offset
                .parse::<usize>()
                .map_err(|_| "Vault audit offset must be a non-negative integer.".to_string())?;
            if parsed_offset > 0 {
                object.insert("offset".to_string(), serde_json::json!(parsed_offset));
            }
        }
    }

    Ok(payload)
}

#[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
fn build_vault_decode_request_payload(
    vault_path: &str,
    vault_passphrase: &str,
    record_ids_json: &str,
    output_target: &str,
    justification: &str,
) -> Result<serde_json::Value, String> {
    let vault_path = vault_path.trim();
    if vault_path.is_empty() {
        return Err("Vault path is required before submitting.".to_string());
    }

    let vault_passphrase = vault_passphrase.trim();
    if vault_passphrase.is_empty() {
        return Err("Vault passphrase is required before submitting.".to_string());
    }

    let record_ids = parse_required_uuid_array(record_ids_json, "Vault decode record ids")?;

    let output_target = output_target.trim();
    if output_target.is_empty() {
        return Err("Output target is required before submitting.".to_string());
    }

    let justification = justification.trim();
    if justification.is_empty() {
        return Err("Justification is required before submitting.".to_string());
    }

    Ok(serde_json::json!({
        "vault_path": vault_path,
        "vault_passphrase": vault_passphrase,
        "record_ids": record_ids,
        "output_target": output_target,
        "justification": justification,
        "requested_by": "browser",
    }))
}

fn parse_required_uuid_array(
    record_ids_json: &str,
    field_name: &str,
) -> Result<Vec<String>, String> {
    let record_ids_value: serde_json::Value = serde_json::from_str(record_ids_json.trim())
        .map_err(|error| format!("{field_name} must be a JSON array of UUID strings: {error}"))?;
    let record_ids_array = record_ids_value
        .as_array()
        .ok_or_else(|| format!("{field_name} must be a JSON array of UUID strings."))?;
    if record_ids_array.is_empty() {
        return Err(format!(
            "{field_name} must include at least one explicit record id."
        ));
    }

    let mut record_ids = Vec::with_capacity(record_ids_array.len());
    let mut seen_record_ids = std::collections::HashSet::with_capacity(record_ids_array.len());
    for record_id in record_ids_array {
        let record_id = record_id
            .as_str()
            .ok_or_else(|| format!("{field_name} must be UUID strings."))?
            .trim();
        let record_id = uuid::Uuid::parse_str(record_id)
            .map_err(|_| format!("{field_name} must be valid UUID strings."))?;
        if !seen_record_ids.insert(record_id) {
            return Err("duplicate record id is not allowed".to_string());
        }
        record_ids.push(record_id.to_string());
    }
    Ok(record_ids)
}

fn parse_required_artifact_object(artifact_json: &str) -> Result<serde_json::Value, String> {
    let artifact: serde_json::Value = serde_json::from_str(artifact_json.trim())
        .map_err(|error| format!("Portable artifact JSON must be a JSON object: {error}"))?;
    if !artifact.is_object() {
        return Err("Portable artifact JSON must be a JSON object.".to_string());
    }
    Ok(artifact)
}

#[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
fn build_vault_export_request_payload(
    vault_path: &str,
    vault_passphrase: &str,
    record_ids_json: &str,
    export_passphrase: &str,
    context: &str,
) -> Result<serde_json::Value, String> {
    let vault_path = vault_path.trim();
    if vault_path.is_empty() {
        return Err("Vault path is required before submitting.".to_string());
    }
    let vault_passphrase = vault_passphrase.trim();
    if vault_passphrase.is_empty() {
        return Err("Vault passphrase is required before submitting.".to_string());
    }
    let export_passphrase = export_passphrase.trim();
    if export_passphrase.is_empty() {
        return Err("Export passphrase is required before submitting.".to_string());
    }
    let context = context.trim();
    if context.is_empty() {
        return Err("Context is required before submitting.".to_string());
    }
    let record_ids = parse_required_uuid_array(record_ids_json, "Vault export record ids")?;
    Ok(serde_json::json!({
        "vault_path": vault_path,
        "vault_passphrase": vault_passphrase,
        "record_ids": record_ids,
        "export_passphrase": export_passphrase,
        "context": context,
        "requested_by": "browser",
    }))
}

#[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
fn build_portable_artifact_inspect_request_payload(
    artifact_json: &str,
    portable_passphrase: &str,
) -> Result<serde_json::Value, String> {
    let portable_passphrase = portable_passphrase.trim();
    if portable_passphrase.is_empty() {
        return Err("Portable artifact passphrase is required before submitting.".to_string());
    }
    Ok(serde_json::json!({
        "artifact": parse_required_artifact_object(artifact_json)?,
        "portable_passphrase": portable_passphrase,
    }))
}

#[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
fn build_portable_artifact_import_request_payload(
    vault_path: &str,
    vault_passphrase: &str,
    artifact_json: &str,
    portable_passphrase: &str,
    context: &str,
) -> Result<serde_json::Value, String> {
    let vault_path = vault_path.trim();
    if vault_path.is_empty() {
        return Err("Vault path is required before submitting.".to_string());
    }
    let vault_passphrase = vault_passphrase.trim();
    if vault_passphrase.is_empty() {
        return Err("Vault passphrase is required before submitting.".to_string());
    }
    let portable_passphrase = portable_passphrase.trim();
    if portable_passphrase.is_empty() {
        return Err("Portable artifact passphrase is required before submitting.".to_string());
    }
    let context = context.trim();
    if context.is_empty() {
        return Err("Context is required before submitting.".to_string());
    }
    Ok(serde_json::json!({
        "vault_path": vault_path,
        "vault_passphrase": vault_passphrase,
        "artifact": parse_required_artifact_object(artifact_json)?,
        "portable_passphrase": portable_passphrase,
        "context": context,
        "requested_by": "browser",
    }))
}

#[derive(Clone, Eq, PartialEq)]
struct BrowserFlowState {
    input_mode: InputMode,
    payload: String,
    source_name: String,
    vault_path: String,
    vault_passphrase: String,
    vault_audit_kind: String,
    vault_audit_actor: String,
    vault_audit_limit: String,
    vault_audit_offset: String,
    vault_decode_record_ids_json: String,
    vault_decode_output_target: String,
    vault_decode_justification: String,
    portable_record_ids_json: String,
    portable_passphrase: String,
    portable_context: String,
    imported_file_name: Option<String>,
    field_policy_json: String,
    result_output: String,
    decoded_values_output: Option<String>,
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
            .field("vault_path", &"<redacted>")
            .field("vault_passphrase", &"<redacted>")
            .field("vault_audit_kind", &"<redacted>")
            .field("vault_audit_actor", &"<redacted>")
            .field("vault_audit_limit", &"<redacted>")
            .field("vault_audit_offset", &"<redacted>")
            .field("vault_decode_record_ids_json", &"<redacted>")
            .field("vault_decode_output_target", &"<redacted>")
            .field("vault_decode_justification", &"<redacted>")
            .field("portable_record_ids_json", &"<redacted>")
            .field("portable_passphrase", &"<redacted>")
            .field("portable_context", &"<redacted>")
            .field(
                "imported_file_name",
                &self.imported_file_name.as_ref().map(|_| "<redacted>"),
            )
            .field("field_policy_json", &"<redacted>")
            .field("result_output", &"<redacted>")
            .field(
                "decoded_values_output",
                &self.decoded_values_output.as_ref().map(|_| "<redacted>"),
            )
            .field("summary", &"<redacted>")
            .field("review_queue", &"<redacted>")
            .field(
                "error_banner",
                &self.error_banner.as_ref().map(|_| "<redacted>"),
            )
            .field("is_submitting", &self.is_submitting)
            .field("state_revision", &self.state_revision)
            .field("next_submission_token", &self.next_submission_token)
            .field("active_submission_token", &self.active_submission_token)
            .finish()
    }
}

fn parse_media_summary_report(summary: &str) -> serde_json::Value {
    let mut object = serde_json::Map::new();
    for line in summary.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        if !matches!(
            key,
            "total_items"
                | "metadata_only_items"
                | "visual_review_required_items"
                | "unsupported_items"
                | "review_required_candidates"
                | "rewritten_media_bytes_base64"
        ) {
            continue;
        }
        if key == "rewritten_media_bytes_base64" {
            object.insert(key.to_string(), serde_json::Value::Null);
            continue;
        }

        let value = value.trim();
        let parsed_value = value
            .parse::<u64>()
            .map_or(serde_json::Value::Null, serde_json::Value::from);
        object.insert(key.to_string(), parsed_value);
    }
    serde_json::Value::Object(object)
}

fn parse_media_review_queue_report(review_queue: &str) -> serde_json::Value {
    serde_json::Value::Array(
        review_queue
            .lines()
            .filter_map(parse_media_review_queue_line)
            .collect(),
    )
}

fn parse_media_review_queue_line(line: &str) -> Option<serde_json::Value> {
    let line = line.trim().strip_prefix("- ").unwrap_or(line.trim());
    let parts: Vec<&str> = line.split(" / ").map(str::trim).collect();
    if parts.len() < 5 {
        return None;
    }

    let format = allowlisted_media_format(parts.get(1).copied().unwrap_or_default());
    let phi_type = allowlisted_media_phi_type(parts.get(2).copied().unwrap_or_default());
    let confidence = parts
        .iter()
        .find_map(|part| part.strip_prefix("confidence "))
        .and_then(|value| value.trim().parse::<f64>().ok())
        .unwrap_or(0.0);

    Some(serde_json::json!({
        "metadata_key": "redacted-field",
        "format": format,
        "phi_type": phi_type,
        "confidence": confidence,
        "value": "redacted",
    }))
}

fn allowlisted_media_format(format: &str) -> &'static str {
    match format {
        "image" => "image",
        "video" => "video",
        "fcs" => "fcs",
        _ => "unknown",
    }
}

fn allowlisted_media_phi_type(phi_type: &str) -> &'static str {
    match phi_type {
        "Name" => "Name",
        "Date" => "Date",
        "Location" => "Location",
        "Identifier" => "Identifier",
        "Contact" => "Contact",
        "Age" => "Age",
        "metadata_identifier" => "metadata_identifier",
        _ => "Other",
    }
}

impl Default for BrowserFlowState {
    fn default() -> Self {
        Self {
            input_mode: InputMode::CsvText,
            payload: String::new(),
            source_name: "local-review.pdf".to_string(),
            vault_path: String::new(),
            vault_passphrase: String::new(),
            vault_audit_kind: String::new(),
            vault_audit_actor: String::new(),
            vault_audit_limit: String::new(),
            vault_audit_offset: String::new(),
            vault_decode_record_ids_json: "[]".to_string(),
            vault_decode_output_target: String::new(),
            vault_decode_justification: String::new(),
            portable_record_ids_json: "[]".to_string(),
            portable_passphrase: String::new(),
            portable_context: String::new(),
            imported_file_name: None,
            field_policy_json: DEFAULT_FIELD_POLICY_JSON.to_string(),
            result_output: String::new(),
            decoded_values_output: None,
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

fn sanitized_import_stem(file_name: &str) -> String {
    let file_name = file_name.rsplit(['/', '\\']).next().unwrap_or(file_name);
    let stem = file_name
        .rsplit_once('.')
        .map_or(file_name, |(stem, _)| stem);

    let mut sanitized = String::new();
    let mut needs_separator = false;
    for ch in stem.chars() {
        if ch.is_ascii_alphanumeric() {
            if needs_separator && !sanitized.is_empty() {
                sanitized.push('-');
            }
            sanitized.push(ch.to_ascii_lowercase());
            needs_separator = false;
        } else {
            needs_separator = !sanitized.is_empty();
        }
    }

    if sanitized.len() > MAX_IMPORT_DERIVED_EXPORT_STEM_CHARS {
        sanitized.truncate(MAX_IMPORT_DERIVED_EXPORT_STEM_CHARS);
        while sanitized.ends_with('-') {
            sanitized.pop();
        }
    }

    if sanitized.is_empty() {
        "mdid-browser-output".to_string()
    } else {
        sanitized
    }
}

fn sanitized_source_stem_preserving_case(file_name: &str) -> String {
    let file_name = file_name.rsplit(['/', '\\']).next().unwrap_or(file_name);
    let stem = file_name
        .rsplit_once('.')
        .map_or(file_name, |(stem, _)| stem);

    let mut sanitized = String::new();
    let mut needs_separator = false;
    for ch in stem.chars() {
        if ch.is_ascii_alphanumeric() {
            if needs_separator && !sanitized.is_empty() {
                sanitized.push('-');
            }
            sanitized.push(ch);
            needs_separator = false;
        } else {
            needs_separator = !sanitized.is_empty();
        }
    }

    if sanitized.len() > MAX_IMPORT_DERIVED_EXPORT_STEM_CHARS {
        sanitized.truncate(MAX_IMPORT_DERIVED_EXPORT_STEM_CHARS);
        while sanitized.ends_with('-') {
            sanitized.pop();
        }
    }

    if sanitized.is_empty() {
        "mdid-browser-output".to_string()
    } else {
        sanitized
    }
}

fn portable_report_source_stem(file_name: &str) -> String {
    let file_name = file_name.rsplit(['/', '\\']).next().unwrap_or(file_name);
    let stem = if let Some(stem) = file_name.strip_suffix(".mdid-portable.json") {
        stem
    } else if let Some(stem) = file_name.strip_suffix("-mdid-portable.json") {
        stem
    } else {
        file_name.rsplit_once('.').map_or(file_name, |(stem, _)| stem)
    };

    let mut sanitized = String::new();
    let mut needs_separator = false;
    for ch in stem.chars() {
        if ch.is_ascii_alphanumeric() {
            if needs_separator && !sanitized.is_empty() {
                sanitized.push('_');
            }
            sanitized.push(ch);
            needs_separator = false;
        } else {
            needs_separator = !sanitized.is_empty();
        }
    }

    if sanitized.len() > MAX_IMPORT_DERIVED_EXPORT_STEM_CHARS {
        sanitized.truncate(MAX_IMPORT_DERIVED_EXPORT_STEM_CHARS);
        while sanitized.ends_with('_') {
            sanitized.pop();
        }
    }

    sanitized
}

fn build_portable_response_report_download(
    mode: InputMode,
    imported_file_name: Option<&str>,
    response_json: &str,
) -> Result<BrowserDownloadPayload, String> {
    const UNAVAILABLE: &str =
        "Portable response report download is only available for portable artifact modes.";
    const INVALID_RESPONSE: &str =
        "Portable response report download requires a valid portable response JSON object.";

    let mode_label = match mode {
        InputMode::VaultExport => "vault_export",
        InputMode::PortableArtifactInspect => "portable_artifact_inspect",
        InputMode::PortableArtifactImport => "portable_artifact_import",
        _ => return Err(UNAVAILABLE.to_string()),
    };

    let mut report = serde_json::from_str::<serde_json::Value>(response_json)
        .map_err(|_| INVALID_RESPONSE.to_string())?;
    let object = report
        .as_object_mut()
        .ok_or_else(|| INVALID_RESPONSE.to_string())?;

    for field in ["artifact", "decoded_values", "records", "vault_passphrase"] {
        if object.contains_key(field) {
            object.insert(field.to_string(), serde_json::Value::String("redacted".to_string()));
        }
    }
    object.insert(
        "mode".to_string(),
        serde_json::Value::String(mode_label.to_string()),
    );

    let report_kind = mode_label
        .strip_prefix("portable_artifact_")
        .unwrap_or(mode_label)
        .replace('_', "-");
    let file_name = imported_file_name
        .map(portable_report_source_stem)
        .filter(|stem| !stem.is_empty() && stem != "mdid_browser_portable_artifact")
        .map(|stem| format!("{stem}-portable-artifact-{report_kind}-report.json"))
        .unwrap_or_else(|| "mdid-browser-portable-artifact-report.json".to_string());

    Ok(BrowserDownloadPayload {
        file_name,
        mime_type: "application/json",
        bytes: serde_json::to_vec_pretty(&report)
            .map_err(|_| "Portable response report download could not encode JSON.".to_string())?,
        is_text: true,
    })
}

fn sanitized_vault_export_stem(file_name: &str) -> String {
    let file_name = file_name.rsplit(['/', '\\']).next().unwrap_or(file_name);
    let stem = file_name
        .rsplit_once('.')
        .map_or(file_name, |(stem, _)| stem);

    let mut sanitized = String::new();
    let mut needs_separator = false;
    for ch in stem.chars() {
        if ch.is_ascii_alphanumeric() {
            if needs_separator && !sanitized.is_empty() {
                sanitized.push('_');
            }
            sanitized.push(ch);
            needs_separator = false;
        } else {
            needs_separator = !sanitized.is_empty();
        }
    }

    if sanitized.len() > MAX_IMPORT_DERIVED_EXPORT_STEM_CHARS {
        sanitized.truncate(MAX_IMPORT_DERIVED_EXPORT_STEM_CHARS);
        while sanitized.ends_with('_') {
            sanitized.pop();
        }
    }

    if sanitized.is_empty() {
        "mdid-browser-output".to_string()
    } else {
        sanitized
    }
}

impl BrowserFlowState {
    #[cfg_attr(not(test), allow(dead_code))]
    fn mode_for_imported_file(&self, detected_mode: InputMode) -> InputMode {
        if self.input_mode == InputMode::PortableArtifactImport
            && detected_mode == InputMode::PortableArtifactInspect
        {
            InputMode::PortableArtifactImport
        } else {
            detected_mode
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    fn apply_imported_file(&mut self, file_name: &str, payload: &str, mode: InputMode) {
        self.input_mode = mode;
        self.source_name = file_name.to_string();
        self.imported_file_name = Some(file_name.to_string());
        self.invalidate_generated_state();
        self.payload = payload.to_string();
    }

    fn reject_imported_file(&mut self, file_name: &str) {
        self.imported_file_name = Some(file_name.to_string());
        self.invalidate_generated_state();
        self.error_banner = Some(
            "Unsupported browser import file type. Use .csv, .xlsx, .pdf, .dcm, .dicom, or .json."
                .to_string(),
        );
    }

    #[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
    fn apply_import_read_error(&mut self, message: &str) {
        self.invalidate_generated_state();
        self.error_banner = Some(message.to_string());
    }

    #[cfg_attr(not(test), allow(dead_code))]
    fn suggested_export_file_name(&self) -> String {
        if self.input_mode == InputMode::MediaMetadataJson {
            if !self.source_name.trim().is_empty() {
                let stem = sanitized_import_stem(&self.source_name);
                if stem != "mdid-browser-output" && stem != "local-review" {
                    return format!("{stem}-media-review-report.json");
                }
            }
            return "mdid-browser-media-review-report.json".to_string();
        }

        if let Some(imported_file_name) = &self.imported_file_name {
            let stem = sanitized_import_stem(imported_file_name);
            match self.input_mode {
                InputMode::CsvText => return format!("{stem}-deidentified.csv"),
                InputMode::XlsxBase64 => return format!("{stem}-deidentified.xlsx"),
                InputMode::PdfBase64 => return format!("{stem}-review-report.json"),
                InputMode::DicomBase64 => return format!("{stem}-deidentified.dcm"),
                InputMode::MediaMetadataJson => unreachable!(
                    "media metadata JSON filenames are handled before imported filename fallback"
                ),
                InputMode::PortableArtifactInspect => {
                    return format!("{stem}-portable-artifact-inspect.json");
                }
                InputMode::PortableArtifactImport => {
                    return format!("{stem}-portable-artifact-import.json");
                }
                InputMode::VaultAuditEvents => {
                    return format!("{stem}-vault-audit-events.json");
                }
                InputMode::VaultDecode => {
                    return format!("{stem}-vault-decode-response.json");
                }
                InputMode::VaultExport => {
                    let stem = sanitized_vault_export_stem(imported_file_name);
                    if stem != "mdid-browser-output" && stem != "mdid_browser_output" {
                        return format!("{stem}-portable-artifact.json");
                    }
                }
            }
        }

        if self.input_mode == InputMode::PdfBase64 && !self.source_name.trim().is_empty() {
            let stem = sanitized_import_stem(&self.source_name);
            if stem != "mdid-browser-output" {
                return format!("{stem}-review-report.json");
            }
        }

        if self.input_mode == InputMode::DicomBase64 && !self.source_name.trim().is_empty() {
            let stem = sanitized_source_stem_preserving_case(&self.source_name);
            if stem != "mdid-browser-output" && stem != "local-review" {
                return format!("{stem}-deidentified.dcm");
            }
        }

        match self.input_mode {
            InputMode::CsvText => "mdid-browser-output.csv",
            InputMode::XlsxBase64 => "mdid-browser-output.xlsx",
            InputMode::PdfBase64 => "mdid-browser-review-report.json",
            InputMode::DicomBase64 => "mdid-browser-output.dcm",
            InputMode::MediaMetadataJson => "mdid-browser-media-review-report.json",
            InputMode::VaultAuditEvents => "mdid-browser-vault-audit-events.json",
            InputMode::VaultDecode => "mdid-browser-vault-decode-response.json",
            InputMode::VaultExport => "mdid-browser-portable-artifact.json",
            InputMode::PortableArtifactInspect => "mdid-browser-portable-artifact-inspect.json",
            InputMode::PortableArtifactImport => "mdid-browser-portable-artifact-import.json",
        }
        .to_string()
    }

    #[cfg_attr(not(test), allow(dead_code))]
    fn vault_audit_pagination_status(&self) -> Option<String> {
        if self.input_mode != InputMode::VaultAuditEvents {
            return None;
        }

        let requested_offset = if self.vault_audit_offset.trim().is_empty() {
            0
        } else {
            self.vault_audit_offset.trim().parse::<u64>().ok()?
        };

        let summary = [&self.summary, &self.result_output]
            .into_iter()
            .filter_map(|candidate| serde_json::from_str::<serde_json::Value>(candidate).ok())
            .find(|candidate| {
                candidate
                    .get("event_count")
                    .and_then(serde_json::Value::as_u64)
                    .is_some()
            })?;
        let total_event_count = summary.get("event_count")?.as_u64()?;
        let returned_event_count = summary
            .get("returned_event_count")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(total_event_count);

        let suffix = match summary
            .get("next_offset")
            .and_then(serde_json::Value::as_u64)
        {
            Some(next_offset) => {
                format!("More events may be available from offset {next_offset}.")
            }
            None => "No next audit page was reported.".to_string(),
        };

        if returned_event_count == 0 {
            return Some(format!(
                "No audit events were returned for this page. {suffix}"
            ));
        }

        let start = requested_offset.saturating_add(1);
        let end = requested_offset.saturating_add(returned_event_count);
        let total = if summary.get("returned_event_count").is_some() {
            format!(" of {total_event_count}")
        } else {
            String::new()
        };

        Some(format!(
            "Showing audit events {start}-{end}{total}. {suffix}"
        ))
    }

    #[cfg_attr(not(test), allow(dead_code))]
    fn can_export_output(&self) -> bool {
        !self.result_output.trim().is_empty()
    }

    fn is_tabular_mode(&self) -> bool {
        matches!(self.input_mode, InputMode::CsvText | InputMode::XlsxBase64)
    }

    fn suggested_tabular_report_file_name(&self) -> String {
        fn safe_report_stem(file_name: &str) -> String {
            let file_name = file_name.rsplit(['/', '\\']).next().unwrap_or(file_name);
            let stem = file_name
                .rsplit_once('.')
                .map_or(file_name, |(stem, _)| stem);

            let mut sanitized = String::new();
            let mut needs_separator = false;
            for ch in stem.chars() {
                if ch.is_ascii_alphanumeric() {
                    if needs_separator && !sanitized.is_empty() {
                        sanitized.push('_');
                    }
                    sanitized.push(ch.to_ascii_lowercase());
                    needs_separator = false;
                } else {
                    needs_separator = !sanitized.is_empty();
                }
            }

            if sanitized.len() > MAX_IMPORT_DERIVED_EXPORT_STEM_CHARS {
                sanitized.truncate(MAX_IMPORT_DERIVED_EXPORT_STEM_CHARS);
                while sanitized.ends_with('_') {
                    sanitized.pop();
                }
            }

            if sanitized.is_empty() {
                "mdid-browser-output".to_string()
            } else {
                sanitized
            }
        }

        self.imported_file_name
            .as_deref()
            .map(safe_report_stem)
            .filter(|stem| stem != "mdid-browser-output")
            .map(|stem| format!("{stem}-tabular-report.json"))
            .unwrap_or_else(|| "mdid-browser-tabular-report.json".to_string())
    }

    fn tabular_report_download_json(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec_pretty(&serde_json::json!({
            "mode": "tabular_report",
            "input_mode": self.input_mode.select_value(),
            "summary": self.summary,
            "review_queue": self.review_queue,
        }))
        .map_err(|_| "Browser tabular report download could not encode JSON.".to_string())
    }

    fn can_export_tabular_report(&self) -> bool {
        self.is_tabular_mode() && !self.result_output.trim().is_empty()
    }

    fn prepared_tabular_report_download_payload(&self) -> Result<BrowserDownloadPayload, String> {
        if !self.can_export_tabular_report() {
            return Err(
                "Tabular report download is only available after a successful CSV/XLSX response."
                    .to_string(),
            );
        }

        Ok(BrowserDownloadPayload {
            file_name: self.suggested_tabular_report_file_name(),
            mime_type: "application/json;charset=utf-8",
            bytes: self.tabular_report_download_json()?,
            is_text: true,
        })
    }

    fn suggested_decoded_values_file_name(&self) -> String {
        self.imported_file_name
            .as_deref()
            .map(sanitized_import_stem)
            .filter(|stem| stem != "mdid-browser-output")
            .map(|stem| format!("{stem}-decoded-values.json"))
            .unwrap_or_else(|| "mdid-browser-decoded-values.json".to_string())
    }

    fn decoded_values_payload(&self) -> Result<serde_json::Value, String> {
        const ERROR: &str = "Decoded values download is only available after a successful vault decode response with decoded values.";

        if self.input_mode != InputMode::VaultDecode {
            return Err(ERROR.to_string());
        }

        let decoded_values_output = self
            .decoded_values_output
            .as_ref()
            .ok_or_else(|| ERROR.to_string())?;
        let response: serde_json::Value =
            serde_json::from_str(decoded_values_output).map_err(|_| ERROR.to_string())?;
        let decoded_values = response
            .get("decoded_values")
            .filter(|value| value.is_array() || value.is_object())
            .cloned()
            .ok_or_else(|| ERROR.to_string())?;

        Ok(serde_json::json!({
            "mode": "vault_decode_values",
            "decoded_values": decoded_values,
        }))
    }

    fn can_export_decoded_values(&self) -> bool {
        self.decoded_values_payload().is_ok()
    }

    fn prepared_decoded_values_download_payload(&self) -> Result<BrowserDownloadPayload, String> {
        let bytes = serde_json::to_vec_pretty(&self.decoded_values_payload()?)
            .map_err(|_| "Browser decoded values download could not encode JSON.".to_string())?;

        Ok(BrowserDownloadPayload {
            file_name: self.suggested_decoded_values_file_name(),
            mime_type: "application/json;charset=utf-8",
            bytes,
            is_text: true,
        })
    }

    fn review_report_download_json(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec_pretty(&serde_json::json!({
            "mode": self.input_mode.label(),
            "summary": self.summary,
            "review_queue": self.review_queue,
            "output": self.result_output,
        }))
        .map_err(|_| "Browser output download could not encode review report JSON.".to_string())
    }

    fn safe_vault_response_download_json(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec_pretty(&serde_json::json!({
            "mode": self.input_mode.safe_vault_report_mode_label(),
            "summary": self.summary,
            "review_queue": self.review_queue,
        }))
        .map_err(|_| {
            "Browser output download could not encode safe vault response JSON.".to_string()
        })
    }

    fn media_review_report_download_json(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec_pretty(&serde_json::json!({
            "mode": "media_metadata_review",
            "summary": parse_media_summary_report(&self.summary),
            "review_queue": parse_media_review_queue_report(&self.review_queue),
        }))
        .map_err(|_| {
            "Browser output download could not encode media review report JSON.".to_string()
        })
    }

    fn prepared_download_payload(&self) -> Result<BrowserDownloadPayload, String> {
        let file_name = self.suggested_export_file_name();
        match self.input_mode {
            InputMode::XlsxBase64 => {
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(self.result_output.trim())
                    .map_err(|_| {
                        "Browser output download could not decode rewritten XLSX base64 bytes."
                            .to_string()
                    })?;
                Ok(BrowserDownloadPayload {
                    file_name,
                    mime_type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                    bytes,
                    is_text: false,
                })
            }
            InputMode::DicomBase64 => {
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(self.result_output.trim())
                    .map_err(|_| {
                        "Browser output download could not decode rewritten DICOM base64 bytes."
                            .to_string()
                    })?;
                Ok(BrowserDownloadPayload {
                    file_name,
                    mime_type: "application/dicom",
                    bytes,
                    is_text: false,
                })
            }
            InputMode::MediaMetadataJson => Ok(BrowserDownloadPayload {
                file_name,
                mime_type: "application/json;charset=utf-8",
                bytes: self.media_review_report_download_json()?,
                is_text: true,
            }),
            InputMode::PdfBase64 => Ok(BrowserDownloadPayload {
                file_name,
                mime_type: "application/json;charset=utf-8",
                bytes: self.review_report_download_json()?,
                is_text: true,
            }),
            InputMode::VaultAuditEvents
            | InputMode::VaultDecode
            | InputMode::PortableArtifactInspect
            | InputMode::PortableArtifactImport => Ok(BrowserDownloadPayload {
                file_name,
                mime_type: "application/json;charset=utf-8",
                bytes: self.safe_vault_response_download_json()?,
                is_text: true,
            }),
            _ => Ok(BrowserDownloadPayload {
                file_name,
                mime_type: "text/plain;charset=utf-8",
                bytes: self.result_output.as_bytes().to_vec(),
                is_text: true,
            }),
        }
    }

    fn clear_generated_state(&mut self) {
        self.result_output.clear();
        self.decoded_values_output = None;
        self.summary = IDLE_SUMMARY.to_string();
        self.review_queue = IDLE_REVIEW_QUEUE.to_string();
        self.error_banner = None;
    }

    fn invalidate_generated_state(&mut self) {
        self.state_revision += 1;
        self.clear_generated_state();
    }

    fn validate_submission(&self) -> Result<RuntimeSubmitRequest, String> {
        if self.input_mode == InputMode::VaultAuditEvents {
            let body_json = serde_json::to_string(&build_vault_audit_request_payload(
                &self.vault_path,
                &self.vault_passphrase,
                &self.vault_audit_kind,
                &self.vault_audit_actor,
                &self.vault_audit_limit,
                &self.vault_audit_offset,
            )?)
            .map_err(|error| format!("Failed to serialize runtime request: {error}"))?;

            return Ok(RuntimeSubmitRequest {
                endpoint: self.input_mode.endpoint(),
                input_mode: self.input_mode,
                body_json,
            });
        }

        if self.input_mode == InputMode::VaultDecode {
            let body_json = serde_json::to_string(&build_vault_decode_request_payload(
                &self.vault_path,
                &self.vault_passphrase,
                &self.vault_decode_record_ids_json,
                &self.vault_decode_output_target,
                &self.vault_decode_justification,
            )?)
            .map_err(|error| format!("Failed to serialize runtime request: {error}"))?;

            return Ok(RuntimeSubmitRequest {
                endpoint: self.input_mode.endpoint(),
                input_mode: self.input_mode,
                body_json,
            });
        }

        if self.input_mode == InputMode::VaultExport {
            let body_json = serde_json::to_string(&build_vault_export_request_payload(
                &self.vault_path,
                &self.vault_passphrase,
                &self.portable_record_ids_json,
                &self.portable_passphrase,
                &self.portable_context,
            )?)
            .map_err(|error| format!("Failed to serialize runtime request: {error}"))?;
            return Ok(RuntimeSubmitRequest {
                endpoint: self.input_mode.endpoint(),
                input_mode: self.input_mode,
                body_json,
            });
        }

        if self.input_mode == InputMode::PortableArtifactInspect {
            let body_json =
                serde_json::to_string(&build_portable_artifact_inspect_request_payload(
                    &self.payload,
                    &self.portable_passphrase,
                )?)
                .map_err(|error| format!("Failed to serialize runtime request: {error}"))?;
            return Ok(RuntimeSubmitRequest {
                endpoint: self.input_mode.endpoint(),
                input_mode: self.input_mode,
                body_json,
            });
        }

        if self.input_mode == InputMode::PortableArtifactImport {
            let body_json = serde_json::to_string(&build_portable_artifact_import_request_payload(
                &self.vault_path,
                &self.vault_passphrase,
                &self.payload,
                &self.portable_passphrase,
                &self.portable_context,
            )?)
            .map_err(|error| format!("Failed to serialize runtime request: {error}"))?;
            return Ok(RuntimeSubmitRequest {
                endpoint: self.input_mode.endpoint(),
                input_mode: self.input_mode,
                body_json,
            });
        }

        if self.payload.trim().is_empty() {
            return Err(format!(
                "{} payload is required before submitting.",
                self.input_mode.label()
            ));
        }

        if self.input_mode.requires_source_name() && self.source_name.trim().is_empty() {
            return Err(format!(
                "{} source name is required before submitting.",
                self.input_mode.source_name_label()
            ));
        }

        if self.input_mode.requires_field_policy() && self.field_policy_json.trim().is_empty() {
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
        self.decoded_values_output = None;
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
        self.decoded_values_output = response.decoded_values_output;
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
        self.decoded_values_output = None;
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

#[derive(Clone, Eq, PartialEq, Serialize)]
struct DicomSubmitRequest {
    dicom_bytes_base64: String,
    source_name: String,
    private_tag_policy: &'static str,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct MediaRuntimeSuccessResponse {
    summary: MediaRuntimeSummary,
    review_queue: Vec<MediaReviewCandidate>,
    #[allow(dead_code)]
    rewritten_media_bytes_base64: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
struct MediaRuntimeSummary {
    total_items: usize,
    metadata_only_items: usize,
    visual_review_required_items: usize,
    unsupported_items: usize,
    review_required_candidates: usize,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct MediaReviewCandidate {
    field_ref: MediaReviewFieldRef,
    format: String,
    phi_type: String,
    #[allow(dead_code)]
    source_value: String,
    confidence: f32,
    status: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
struct MediaReviewFieldRef {
    artifact_label: String,
    metadata_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RuntimeSubmitRequest {
    endpoint: &'static str,
    input_mode: InputMode,
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
    worksheet_disclosure: Option<XlsxWorksheetDisclosure>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
struct XlsxWorksheetDisclosure {
    selected_sheet_name: String,
    selected_sheet_index: usize,
    total_sheet_count: usize,
    disclosure: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
struct DicomRuntimeSuccessResponse {
    summary: DicomRuntimeSummary,
    review_queue: Vec<DicomReviewCandidate>,
    #[allow(dead_code)]
    sanitized_file_name: Option<String>,
    rewritten_dicom_bytes_base64: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
struct DicomRuntimeSummary {
    total_tags: usize,
    encoded_tags: usize,
    review_required_tags: usize,
    removed_private_tags: usize,
    remapped_uids: usize,
    burned_in_suspicions: usize,
    pixel_redaction_performed: bool,
    burned_in_review_required: bool,
    burned_in_annotation_notice: String,
    #[serde(default = "default_dicom_burned_in_disclosure")]
    burned_in_disclosure: String,
}

fn default_dicom_burned_in_disclosure() -> String {
    "DICOM pixel data was not inspected or redacted; burned-in annotations require separate visual review.".to_string()
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
struct DicomReviewCandidate {
    tag: DicomTagRef,
    phi_type: String,
    #[allow(dead_code)]
    value: String,
    decision: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
struct DicomTagRef {
    group: u16,
    element: u16,
    keyword: String,
}

#[derive(Clone, PartialEq, Deserialize)]
struct PdfRuntimeSuccessResponse {
    summary: PdfExtractionSummary,
    page_statuses: Vec<PdfPageStatusResponse>,
    review_queue: Vec<PdfReviewCandidate>,
    rewrite_status: String,
    no_rewritten_pdf: bool,
    review_only: bool,
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
    rewrite_status: String,
    no_rewritten_pdf: bool,
    review_only: bool,
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

#[derive(Deserialize)]
struct VaultDecodeRuntimeSuccessResponse {
    values: Vec<VaultDecodeValueResponse>,
    audit_event: VaultDecodeAuditEventResponse,
}

#[derive(Deserialize, Serialize)]
struct VaultDecodeValueResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    record_id: Option<String>,
    #[allow(dead_code)]
    original_value: String,
    #[allow(dead_code)]
    token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    scope: Option<String>,
}

#[derive(Deserialize)]
struct VaultDecodeAuditEventResponse {
    kind: String,
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
    decoded_values_output: Option<String>,
    summary: String,
    review_queue: String,
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

fn build_submit_request(
    input_mode: InputMode,
    payload: &str,
    source_name: &str,
    field_policy_json: &str,
) -> Result<RuntimeSubmitRequest, String> {
    if input_mode == InputMode::PdfBase64 {
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
            input_mode,
            body_json,
        });
    }

    if input_mode == InputMode::DicomBase64 {
        if source_name.trim().is_empty() {
            return Err("DICOM base64 source name is required before submitting.".to_string());
        }

        let body_json = serde_json::to_string(&DicomSubmitRequest {
            dicom_bytes_base64: payload.trim().to_string(),
            source_name: source_name.trim().to_string(),
            private_tag_policy: "remove",
        })
        .map_err(|error| format!("Failed to serialize runtime request: {error}"))?;

        return Ok(RuntimeSubmitRequest {
            endpoint: input_mode.endpoint(),
            input_mode,
            body_json,
        });
    }

    if input_mode == InputMode::MediaMetadataJson {
        let value: serde_json::Value = serde_json::from_str(payload.trim()).map_err(|_| {
            "Media metadata JSON must be a JSON object accepted by the local media review runtime route."
                .to_string()
        })?;

        if !value.is_object() {
            return Err("Media metadata JSON must be a JSON object accepted by the local media review runtime route.".to_string());
        }

        if media_metadata_json_contains_media_bytes(&value) {
            return Err("metadata-only media review does not accept media bytes".to_string());
        }

        let body_json = serde_json::to_string(&value)
            .map_err(|error| format!("Failed to serialize runtime request: {error}"))?;

        return Ok(RuntimeSubmitRequest {
            endpoint: input_mode.endpoint(),
            input_mode,
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
        InputMode::DicomBase64 => unreachable!("DICOM requests are handled before policy parsing"),
        InputMode::MediaMetadataJson => {
            unreachable!("Media metadata JSON requests are handled before policy parsing")
        }
        InputMode::VaultAuditEvents => {
            unreachable!("Vault audit events requests are handled before policy parsing")
        }
        InputMode::VaultDecode => {
            unreachable!("Vault decode requests are handled before policy parsing")
        }
        InputMode::VaultExport => {
            unreachable!("Vault export requests are handled before policy parsing")
        }
        InputMode::PortableArtifactInspect => {
            unreachable!("Portable artifact inspect requests are handled before policy parsing")
        }
        InputMode::PortableArtifactImport => {
            unreachable!("Portable artifact import requests are handled before policy parsing")
        }
    }
    .map_err(|error| format!("Failed to serialize runtime request: {error}"))?;

    Ok(RuntimeSubmitRequest {
        endpoint: input_mode.endpoint(),
        input_mode,
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
                decoded_values_output: None,
                summary: format_summary(&parsed.summary),
                review_queue: format_review_queue(&parsed.review_queue),
            })
        }
        InputMode::XlsxBase64 => {
            let parsed: XlsxRuntimeSuccessResponse = serde_json::from_str(response_body)
                .map_err(|error| format!("Failed to parse runtime success response: {error}"))?;
            Ok(RuntimeResponseEnvelope {
                rewritten_output: parsed.rewritten_workbook_base64,
                decoded_values_output: None,
                summary: format_xlsx_summary(&parsed.summary, parsed.worksheet_disclosure.as_ref()),
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
                decoded_values_output: None,
                summary: format_pdf_summary(
                    &parsed.summary,
                    &parsed.page_statuses,
                    &parsed.rewrite_status,
                    parsed.no_rewritten_pdf,
                    parsed.review_only,
                ),
                review_queue: format_pdf_review_queue(&parsed.review_queue),
            })
        }
        InputMode::DicomBase64 => {
            let parsed: DicomRuntimeSuccessResponse = serde_json::from_str(response_body)
                .map_err(|error| format!("Failed to parse runtime success response: {error}"))?;
            Ok(RuntimeResponseEnvelope {
                rewritten_output: parsed.rewritten_dicom_bytes_base64,
                decoded_values_output: None,
                summary: format_dicom_summary(&parsed.summary),
                review_queue: format_dicom_review_queue(&parsed.review_queue),
            })
        }
        InputMode::MediaMetadataJson => {
            let parsed: MediaRuntimeSuccessResponse = serde_json::from_str(response_body)
                .map_err(|error| format!("Failed to parse runtime success response: {error}"))?;
            Ok(RuntimeResponseEnvelope {
                rewritten_output: "Media rewrite/export unavailable: runtime returned metadata-only conservative review.".to_string(),
                decoded_values_output: None,
                summary: format_media_summary(&parsed.summary),
                review_queue: format_media_review_queue(&parsed.review_queue),
            })
        }
        InputMode::VaultAuditEvents => Ok(RuntimeResponseEnvelope {
            rewritten_output: response_body.trim().to_string(),
            decoded_values_output: None,
            summary: "Vault audit events returned by read-only runtime endpoint.".to_string(),
            review_queue: "No review items returned.".to_string(),
        }),
        InputMode::VaultDecode => {
            let parsed: VaultDecodeRuntimeSuccessResponse = serde_json::from_str(response_body)
                .map_err(|error| format!("Failed to parse runtime success response: {error}"))?;
            let value_count = parsed.values.len();
            let decoded_values_output = serde_json::to_string(&serde_json::json!({
                "decoded_values": parsed.values,
            }))
            .map_err(|error| format!("Failed to serialize decoded values response: {error}"))?;
            Ok(RuntimeResponseEnvelope {
                rewritten_output: format!(
                    "Decoded values hidden for PHI safety. {value_count} value(s) decoded by bounded runtime endpoint."
                ),
                decoded_values_output: Some(decoded_values_output),
                summary: format!("Vault decode completed for {value_count} value(s)."),
                review_queue: format!("- {}", parsed.audit_event.kind),
            })
        }
        InputMode::VaultExport => {
            let parsed: serde_json::Value = serde_json::from_str(response_body)
                .map_err(|_| "Failed to parse runtime success response.".to_string())?;
            let artifact = parsed
                .get("artifact")
                .filter(|artifact| artifact.is_object())
                .ok_or("Vault export response missing artifact object.".to_string())?;
            if !artifact
                .get("salt_b64")
                .is_some_and(serde_json::Value::is_string)
                || !artifact
                    .get("nonce_b64")
                    .is_some_and(serde_json::Value::is_string)
                || !artifact
                    .get("ciphertext_b64")
                    .is_some_and(serde_json::Value::is_string)
            {
                return Err(
                    "Vault export response missing encrypted portable artifact fields.".to_string(),
                );
            }
            let rewritten_output = serde_json::to_string_pretty(artifact)
                .map_err(|error| format!("Failed to render portable artifact: {error}"))?;
            Ok(RuntimeResponseEnvelope {
                rewritten_output,
                decoded_values_output: None,
                summary: "Portable artifact created and available for local download.".to_string(),
                review_queue: "encrypted portable artifact available. Decoded PHI is not rendered."
                    .to_string(),
            })
        }
        InputMode::PortableArtifactInspect => {
            let parsed: serde_json::Value = serde_json::from_str(response_body)
                .map_err(|error| format!("Failed to parse runtime success response: {error}"))?;
            let record_count = parsed
                .get("record_count")
                .and_then(serde_json::Value::as_u64)
                .ok_or("Portable artifact inspect response missing record_count.".to_string())?;
            Ok(RuntimeResponseEnvelope {
                rewritten_output: "Portable artifact inspect completed. Artifact records and values are hidden for PHI safety.".to_string(),
                decoded_values_output: None,
                summary: format!("{record_count} portable record(s) inspected."),
                review_queue: "No record details rendered.".to_string(),
            })
        }
        InputMode::PortableArtifactImport => {
            let parsed: serde_json::Value = serde_json::from_str(response_body)
                .map_err(|error| format!("Failed to parse runtime success response: {error}"))?;
            let imported = parsed
                .get("imported_record_count")
                .and_then(serde_json::Value::as_u64)
                .ok_or(
                    "Portable artifact import response missing imported_record_count.".to_string(),
                )?;
            let duplicates = parsed
                .get("duplicate_record_count")
                .and_then(serde_json::Value::as_u64)
                .ok_or(
                    "Portable artifact import response missing duplicate_record_count.".to_string(),
                )?;
            Ok(RuntimeResponseEnvelope {
                rewritten_output: "Portable artifact import completed. Audit detail and artifact contents are hidden for PHI safety.".to_string(),
                decoded_values_output: None,
                summary: format!("{imported} imported portable record(s)."),
                review_queue: format!("{duplicates} duplicate portable record(s). Generic audit notice recorded."),
            })
        }
    }
}

#[cfg_attr(not(test), allow(dead_code))]
fn render_runtime_response(
    input_mode: InputMode,
    response_body: &str,
) -> Result<RuntimeResponseEnvelope, String> {
    parse_runtime_success(input_mode, response_body)
}

#[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
fn parse_runtime_error(input_mode: InputMode, status: u16, response_body: &str) -> String {
    const MAX_MESSAGE_LEN: usize = 240;

    if input_mode.redacts_runtime_error_details() {
        return format!(
            "Runtime request failed. Details hidden for PHI and secret safety. Status: {status}."
        );
    }

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

fn format_xlsx_summary(
    summary: &RuntimeSummary,
    disclosure: Option<&XlsxWorksheetDisclosure>,
) -> String {
    let mut formatted = format_summary(summary);
    if let Some(disclosure) = disclosure {
        formatted.push_str(&format!(
            "\nworksheet_disclosure:\nselected_sheet_name: {}\nselected_sheet_index: {}\ntotal_sheet_count: {}\ndisclosure: {}",
            disclosure.selected_sheet_name,
            disclosure.selected_sheet_index,
            disclosure.total_sheet_count,
            disclosure.disclosure
        ));
    }
    formatted
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

fn format_dicom_summary(summary: &DicomRuntimeSummary) -> String {
    format!(
        "total_tags: {}\nencoded_tags: {}\nreview_required_tags: {}\nremoved_private_tags: {}\nremapped_uids: {}\nburned_in_suspicions: {}\npixel_redaction_performed: {}\nburned_in_review_required: {}\nburned_in_annotation_notice: {}\nburned_in_disclosure: {}",
        summary.total_tags,
        summary.encoded_tags,
        summary.review_required_tags,
        summary.removed_private_tags,
        summary.remapped_uids,
        summary.burned_in_suspicions,
        summary.pixel_redaction_performed,
        summary.burned_in_review_required,
        summary.burned_in_annotation_notice,
        summary.burned_in_disclosure
    )
}

fn format_dicom_review_queue(review_queue: &[DicomReviewCandidate]) -> String {
    if review_queue.is_empty() {
        return "No review items returned.".to_string();
    }

    review_queue
        .iter()
        .map(|candidate| {
            format!(
                "- tag ({:04X},{:04X}) {} / {} / {} / value: <redacted>",
                candidate.tag.group,
                candidate.tag.element,
                candidate.tag.keyword,
                candidate.phi_type,
                candidate.decision
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_media_summary(summary: &MediaRuntimeSummary) -> String {
    format!(
        "total_items: {}\nmetadata_only_items: {}\nvisual_review_required_items: {}\nunsupported_items: {}\nreview_required_candidates: {}\nrewritten_media_bytes_base64: null",
        summary.total_items,
        summary.metadata_only_items,
        summary.visual_review_required_items,
        summary.unsupported_items,
        summary.review_required_candidates
    )
}

fn format_media_review_queue(review_queue: &[MediaReviewCandidate]) -> String {
    if review_queue.is_empty() {
        return "No review items returned.".to_string();
    }

    review_queue
        .iter()
        .map(|candidate| {
            format!(
                "- {} / {} / {} / confidence {} / value: <redacted>",
                candidate.field_ref.metadata_key,
                candidate.format,
                candidate.phi_type,
                candidate.confidence
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_pdf_summary(
    summary: &PdfExtractionSummary,
    page_statuses: &[PdfPageStatusResponse],
    rewrite_status: &str,
    no_rewritten_pdf: bool,
    review_only: bool,
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
        format!("rewrite_status: {rewrite_status}"),
        format!("no_rewritten_pdf: {no_rewritten_pdf}"),
        format!("review_only: {review_only}"),
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
        .map_err(|error| parse_runtime_error(request.input_mode, 0, &error.to_string()))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("Failed to read runtime response: {error}"))?;

    if (200..300).contains(&status) {
        Ok(body)
    } else {
        Err(parse_runtime_error(request.input_mode, status, &body))
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn perform_runtime_request(_request: RuntimeSubmitRequest) -> Result<String, String> {
    Err(FETCH_UNAVAILABLE_MESSAGE.to_string())
}

#[cfg(target_arch = "wasm32")]
fn trigger_browser_text_download(file_name: &str, text: &str) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};

    let parts = js_sys::Array::new();
    parts.push(&wasm_bindgen::JsValue::from_str(text));

    let options = BlobPropertyBag::new();
    options.set_type("text/plain;charset=utf-8");

    let blob = Blob::new_with_str_sequence_and_options(&parts, &options)
        .map_err(|_| "Failed to create browser export blob.".to_string())?;
    let url = Url::create_object_url_with_blob(&blob)
        .map_err(|_| "Failed to create browser export URL.".to_string())?;

    let window =
        web_sys::window().ok_or("Browser window is unavailable for export.".to_string())?;
    let document = window
        .document()
        .ok_or("Browser document is unavailable for export.".to_string())?;
    let anchor = document
        .create_element("a")
        .map_err(|_| "Failed to create browser export anchor.".to_string())?
        .dyn_into::<HtmlAnchorElement>()
        .map_err(|_| "Failed to prepare browser export anchor.".to_string())?;

    anchor.set_href(&url);
    anchor.set_download(file_name);
    anchor.click();
    Url::revoke_object_url(&url).ok();
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn trigger_browser_text_download(_file_name: &str, _text: &str) -> Result<(), String> {
    Err("Browser export is only available from a wasm32 browser build.".to_string())
}

#[cfg(target_arch = "wasm32")]
fn trigger_browser_binary_download(
    file_name: &str,
    bytes: &[u8],
    mime_type: &'static str,
) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};

    let parts = js_sys::Array::new();
    parts.push(&js_sys::Uint8Array::from(bytes));

    let options = BlobPropertyBag::new();
    options.set_type(mime_type);

    let blob = Blob::new_with_u8_array_sequence_and_options(&parts, &options)
        .map_err(|_| "Failed to create browser export blob.".to_string())?;
    let url = Url::create_object_url_with_blob(&blob)
        .map_err(|_| "Failed to create browser export URL.".to_string())?;

    let window =
        web_sys::window().ok_or("Browser window is unavailable for export.".to_string())?;
    let document = window
        .document()
        .ok_or("Browser document is unavailable for export.".to_string())?;
    let anchor = document
        .create_element("a")
        .map_err(|_| "Failed to create browser export anchor.".to_string())?
        .dyn_into::<HtmlAnchorElement>()
        .map_err(|_| "Failed to prepare browser export anchor.".to_string())?;

    anchor.set_href(&url);
    anchor.set_download(file_name);
    anchor.click();
    Url::revoke_object_url(&url).ok();
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn trigger_browser_download(payload: &BrowserDownloadPayload) -> Result<(), String> {
    if payload.is_text {
        let text = std::str::from_utf8(&payload.bytes)
            .map_err(|_| "Browser text export payload was not valid UTF-8.".to_string())?;
        return trigger_browser_text_download(&payload.file_name, text);
    }
    trigger_browser_binary_download(&payload.file_name, &payload.bytes, payload.mime_type)
}

#[cfg(not(target_arch = "wasm32"))]
fn trigger_browser_download(_payload: &BrowserDownloadPayload) -> Result<(), String> {
    Err(FETCH_UNAVAILABLE_MESSAGE.to_string())
}

#[cfg(target_arch = "wasm32")]
fn read_browser_import_file(event: leptos::ev::Event, state: RwSignal<BrowserFlowState>) {
    use std::{cell::RefCell, rc::Rc};
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;
    use web_sys::{FileReader, HtmlInputElement};

    let Some(input) = event
        .target()
        .and_then(|target| target.dyn_into::<HtmlInputElement>().ok())
    else {
        state.update(|state| {
            state.error_banner = Some("Browser import input is unavailable.".to_string());
        });
        return;
    };

    let Some(file) = input.files().and_then(|files| files.get(0)) else {
        return;
    };

    let file_name = file.name();
    let Some(input_mode) = InputMode::from_file_name(&file_name) else {
        state.update(|state| state.reject_imported_file(&file_name));
        return;
    };

    if let Err(message) = validate_browser_import_size(file.size() as u64) {
        state.update(|state| {
            state.invalidate_generated_state();
            state.error_banner = Some(message);
        });
        return;
    }

    let Ok(reader) = FileReader::new() else {
        state.update(|state| {
            state.error_banner = Some("Failed to prepare browser file reader.".to_string());
        });
        return;
    };

    let onload_slot: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
    let onerror_slot: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));

    let load_state = state;
    let load_file_name = file_name.clone();
    let load_reader = reader.clone();
    let load_cleanup_reader = reader.clone();
    let load_onload_slot = Rc::clone(&onload_slot);
    let load_onerror_slot = Rc::clone(&onerror_slot);
    let onload = Closure::wrap(Box::new(move || {
        load_cleanup_reader.set_onload(None);
        load_cleanup_reader.set_onerror(None);

        let payload = load_reader
            .result()
            .ok()
            .and_then(|value| value.as_string())
            .map(|value| match input_mode.browser_file_read_mode() {
                BrowserFileReadMode::Text => value,
                BrowserFileReadMode::DataUrlBase64 => file_import_payload_from_data_url(&value),
            });

        match payload {
            Some(payload) => load_state.update(|state| {
                let effective_mode = state.mode_for_imported_file(input_mode);
                state.apply_imported_file(&load_file_name, &payload, effective_mode);
            }),
            None => load_state.update(|state| {
                state.apply_import_read_error("Failed to read browser import payload.");
            }),
        }

        load_onload_slot.borrow_mut().take();
        load_onerror_slot.borrow_mut().take();
    }) as Box<dyn FnMut()>);

    let error_state = state;
    let error_cleanup_reader = reader.clone();
    let error_onload_slot = Rc::clone(&onload_slot);
    let error_onerror_slot = Rc::clone(&onerror_slot);
    let onerror = Closure::wrap(Box::new(move || {
        error_cleanup_reader.set_onload(None);
        error_cleanup_reader.set_onerror(None);
        error_state.update(|state| {
            state.apply_import_read_error("Failed to read selected browser import file.");
        });
        error_onload_slot.borrow_mut().take();
        error_onerror_slot.borrow_mut().take();
    }) as Box<dyn FnMut()>);

    reader.set_onload(Some(onload.as_ref().unchecked_ref()));
    reader.set_onerror(Some(onerror.as_ref().unchecked_ref()));
    onload_slot.borrow_mut().replace(onload);
    onerror_slot.borrow_mut().replace(onerror);

    let read_result = match input_mode.browser_file_read_mode() {
        BrowserFileReadMode::Text => reader.read_as_text(&file),
        BrowserFileReadMode::DataUrlBase64 => reader.read_as_data_url(&file),
    };

    if read_result.is_err() {
        reader.set_onload(None);
        reader.set_onerror(None);
        onload_slot.borrow_mut().take();
        onerror_slot.borrow_mut().take();
        state.update(|state| {
            state.apply_import_read_error("Failed to start browser import file read.");
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn read_browser_import_file(event: leptos::ev::Event, state: RwSignal<BrowserFlowState>) {
    let raw_name = event_target_value(&event);
    let file_name = raw_name
        .rsplit(['/', '\\'])
        .next()
        .filter(|value| !value.is_empty())
        .unwrap_or(raw_name.as_str())
        .to_string();

    if file_name.is_empty() {
        return;
    }

    if InputMode::from_file_name(&file_name).is_none() {
        state.update(|state| state.reject_imported_file(&file_name));
    }
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

    let on_vault_path_input = move |event| {
        let next_value = event_target_value(&event);
        state.update(|state| {
            state.vault_path = next_value;
            state.invalidate_generated_state();
        });
    };

    let on_vault_passphrase_input = move |event| {
        let next_value = event_target_value(&event);
        state.update(|state| {
            state.vault_passphrase = next_value;
            state.invalidate_generated_state();
        });
    };

    let on_vault_kind_input = move |event| {
        let next_value = event_target_value(&event);
        state.update(|state| {
            state.vault_audit_kind = next_value;
            state.invalidate_generated_state();
        });
    };

    let on_vault_actor_input = move |event| {
        let next_value = event_target_value(&event);
        state.update(|state| {
            state.vault_audit_actor = next_value;
            state.invalidate_generated_state();
        });
    };

    let on_vault_limit_input = move |event| {
        let next_value = event_target_value(&event);
        state.update(|state| {
            state.vault_audit_limit = next_value;
            state.invalidate_generated_state();
        });
    };

    let on_vault_offset_input = move |event| {
        let next_value = event_target_value(&event);
        state.update(|state| {
            state.vault_audit_offset = next_value;
            state.invalidate_generated_state();
        });
    };

    let on_vault_decode_record_ids_input = move |event| {
        let next_value = event_target_value(&event);
        state.update(|state| {
            state.vault_decode_record_ids_json = next_value;
            state.invalidate_generated_state();
        });
    };

    let on_vault_decode_output_target_input = move |event| {
        let next_value = event_target_value(&event);
        state.update(|state| {
            state.vault_decode_output_target = next_value;
            state.invalidate_generated_state();
        });
    };

    let on_vault_decode_justification_input = move |event| {
        let next_value = event_target_value(&event);
        state.update(|state| {
            state.vault_decode_justification = next_value;
            state.invalidate_generated_state();
        });
    };

    let on_portable_record_ids_input = move |event| {
        let next_value = event_target_value(&event);
        state.update(|state| {
            state.portable_record_ids_json = next_value;
            state.invalidate_generated_state();
        });
    };

    let on_portable_passphrase_input = move |event| {
        let next_value = event_target_value(&event);
        state.update(|state| {
            state.portable_passphrase = next_value;
            state.invalidate_generated_state();
        });
    };

    let on_portable_context_input = move |event| {
        let next_value = event_target_value(&event);
        state.update(|state| {
            state.portable_context = next_value;
            state.invalidate_generated_state();
        });
    };

    let on_file_import_change = move |event| read_browser_import_file(event, state);

    let export_file_name = move || state.get().suggested_export_file_name();
    let tabular_report_file_name = move || state.get().suggested_tabular_report_file_name();
    let can_export_output = move || state.get().can_export_output();
    let can_export_tabular_report = move || state.get().can_export_tabular_report();
    let can_export_decoded_values = move || state.get().can_export_decoded_values();

    let on_export = move |_| {
        let export = state.with_untracked(|state| {
            if state.can_export_output() {
                Some(state.prepared_download_payload())
            } else {
                None
            }
        });

        if let Some(export) = export {
            match export.and_then(|payload| trigger_browser_download(&payload)) {
                Ok(()) => {}
                Err(message) => state.update(|state| state.error_banner = Some(message)),
            }
        }
    };

    let on_export_decoded_values = move |_| {
        let export = state.with_untracked(|state| {
            if state.can_export_decoded_values() {
                Some(state.prepared_decoded_values_download_payload())
            } else {
                None
            }
        });

        if let Some(export) = export {
            match export.and_then(|payload| trigger_browser_download(&payload)) {
                Ok(()) => {}
                Err(message) => state.update(|state| state.error_banner = Some(message)),
            }
        }
    };

    let on_export_tabular_report = move |_| {
        let export = state.with_untracked(|state| {
            if state.can_export_tabular_report() {
                Some(state.prepared_tabular_report_download_payload())
            } else {
                None
            }
        });

        if let Some(export) = export {
            match export.and_then(|payload| trigger_browser_download(&payload)) {
                Ok(()) => {}
                Err(message) => state.update(|state| state.error_banner = Some(message)),
            }
        }
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
                <p class="input-disclosure">{BROWSER_FILE_IMPORT_COPY}</p>
                <label>
                    "Import local CSV/XLSX/PDF/DICOM/media metadata JSON payload"
                    <input
                        accept=".csv,.xlsx,.pdf,.dcm,.dicom,.json"
                        on:change=on_file_import_change
                        type="file"
                    />
                </label>
                <p class="input-disclosure">
                    "This bounded control validates CSV/XLSX/PDF/DICOM/media metadata JSON selection for the existing payload box. CSV content remains text; XLSX/PDF/DICOM payloads remain base64 text for localhost runtime routes. JSON payloads remain metadata-only and do not include media bytes."
                </p>
                <label>
                    "Input mode"
                    <select on:change=on_mode_change prop:value=move || state.get().input_mode.select_value()>
                        <option value="csv-text">"CSV text"</option>
                        <option value="xlsx-base64">"XLSX base64"</option>
                        <option value="pdf-base64">"PDF base64"</option>
                        <option value="dicom-base64">"DICOM base64"</option>
                        <option value="media-metadata-json">"Media metadata JSON"</option>
                        <option value="vault-audit-events">"Vault audit events"</option>
                        <option value="vault-decode">"Vault decode"</option>
                        <option value="vault-export">"Vault export"</option>
                        <option value="portable-artifact-inspect">"Portable artifact inspect"</option>
                        <option value="portable-artifact-import">"Portable artifact import"</option>
                    </select>
                </label>

                <Show when=move || !matches!(state.get().input_mode, InputMode::VaultAuditEvents | InputMode::VaultDecode | InputMode::VaultExport | InputMode::PortableArtifactImport)>
                    <label>
                        "Payload"
                        <textarea
                            on:input=on_payload_input
                            prop:value=move || state.get().payload
                            placeholder=move || state.get().input_mode.payload_hint()
                            rows="12"
                        />
                    </label>
                </Show>

                <Show when=move || state.get().input_mode == InputMode::VaultAuditEvents>
                    <div class="vault-audit-fields">
                        <label>
                            "Vault path"
                            <input on:input=on_vault_path_input prop:value=move || state.get().vault_path type="text" />
                        </label>
                        <label>
                            "Vault passphrase"
                            <input on:input=on_vault_passphrase_input prop:value=move || state.get().vault_passphrase type="password" />
                        </label>
                        <label>
                            "Kind filter (optional)"
                            <input on:input=on_vault_kind_input prop:value=move || state.get().vault_audit_kind type="text" />
                        </label>
                        <label>
                            "Actor filter (optional)"
                            <input on:input=on_vault_actor_input prop:value=move || state.get().vault_audit_actor type="text" />
                        </label>
                        <label>
                            "Limit (optional)"
                            <input on:input=on_vault_limit_input prop:value=move || state.get().vault_audit_limit type="text" />
                        </label>
                        <label>
                            "Offset (optional)"
                            <input on:input=on_vault_offset_input prop:value=move || state.get().vault_audit_offset type="text" />
                        </label>
                    </div>
                </Show>

                <Show when=move || state.get().input_mode == InputMode::VaultDecode>
                    <div class="vault-decode-fields">
                        <label>
                            "Vault path"
                            <input on:input=on_vault_path_input prop:value=move || state.get().vault_path type="text" />
                        </label>
                        <label>
                            "Vault passphrase"
                            <input on:input=on_vault_passphrase_input prop:value=move || state.get().vault_passphrase type="password" />
                        </label>
                        <label>
                            "Record ids JSON"
                            <textarea on:input=on_vault_decode_record_ids_input prop:value=move || state.get().vault_decode_record_ids_json rows="6" />
                        </label>
                        <label>
                            "Output target"
                            <input on:input=on_vault_decode_output_target_input prop:value=move || state.get().vault_decode_output_target type="text" />
                        </label>
                        <label>
                            "Justification"
                            <textarea on:input=on_vault_decode_justification_input prop:value=move || state.get().vault_decode_justification rows="4" />
                        </label>
                    </div>
                </Show>

                <Show when=move || matches!(state.get().input_mode, InputMode::VaultExport | InputMode::PortableArtifactImport)>
                    <div class="portable-artifact-fields">
                        <label>
                            "Vault path"
                            <input on:input=on_vault_path_input prop:value=move || state.get().vault_path type="text" />
                        </label>
                        <label>
                            "Vault passphrase"
                            <input on:input=on_vault_passphrase_input prop:value=move || state.get().vault_passphrase type="password" />
                        </label>
                    </div>
                </Show>

                <Show when=move || state.get().input_mode == InputMode::VaultExport>
                    <label>
                        "Record ids JSON"
                        <textarea on:input=on_portable_record_ids_input prop:value=move || state.get().portable_record_ids_json rows="6" />
                    </label>
                </Show>

                <Show when=move || state.get().input_mode == InputMode::PortableArtifactImport>
                    <label>
                        "Portable artifact JSON"
                        <textarea on:input=on_payload_input prop:value=move || state.get().payload rows="8" />
                    </label>
                </Show>

                <Show when=move || matches!(state.get().input_mode, InputMode::VaultExport | InputMode::PortableArtifactInspect | InputMode::PortableArtifactImport)>
                    <label>
                        "Portable passphrase"
                        <input on:input=on_portable_passphrase_input prop:value=move || state.get().portable_passphrase type="password" />
                    </label>
                </Show>

                <Show when=move || matches!(state.get().input_mode, InputMode::VaultExport | InputMode::PortableArtifactImport)>
                    <label>
                        "Context"
                        <textarea on:input=on_portable_context_input prop:value=move || state.get().portable_context rows="4" />
                    </label>
                </Show>

                <Show when=move || state.get().input_mode.disclosure_copy().is_some()>
                    <p class="input-disclosure">
                        {move || state.get().input_mode.disclosure_copy().unwrap_or_default()}
                    </p>
                </Show>

                <Show when=move || state.get().input_mode.requires_source_name()>
                    <label>
                        "Source name"
                        <input
                            on:input=on_source_name_input
                            prop:value=move || state.get().source_name
                            type="text"
                        />
                    </label>
                </Show>

                <Show when=move || state.get().input_mode.requires_field_policy()>
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
                <p>
                    "Suggested export file: "
                    <code>{export_file_name}</code>
                </p>
                <p class="export-filename-warning">
                    {EXPORT_FILENAME_WARNING_COPY}
                </p>
                <button disabled=move || !can_export_output() on:click=on_export type="button">
                    "Export current result output text"
                </button>
                <Show when=move || state.get().is_tabular_mode()>
                    <div class="tabular-report-export">
                        <p>
                            "Suggested structured report file: "
                            <code>{tabular_report_file_name}</code>
                        </p>
                        <button disabled=move || !can_export_tabular_report() on:click=on_export_tabular_report type="button">
                            "Download tabular structured report JSON"
                        </button>
                    </div>
                </Show>
                <Show when=move || state.get().input_mode == InputMode::VaultDecode>
                    <div class="decoded-values-export-warning">
                        <p>
                            "Decoded values contain high-risk PHI. Download only on a trusted local workstation and handle the JSON according to your vault access controls."
                        </p>
                        <button disabled=move || !can_export_decoded_values() on:click=on_export_decoded_values type="button">
                            "Download decoded values JSON"
                        </button>
                    </div>
                </Show>
                <pre>{move || state.get().result_output}</pre>
            </section>

            <section>
                <h2>"Summary"</h2>
                <Show when=move || state.get().vault_audit_pagination_status().is_some()>
                    <p class="input-disclosure">
                        {move || state.get().vault_audit_pagination_status().unwrap_or_default()}
                    </p>
                </Show>
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
    use base64::Engine;

    use super::{
        build_portable_artifact_import_request_payload, build_portable_response_report_download,
        build_portable_artifact_inspect_request_payload, build_submit_request,
        build_vault_audit_request_payload, build_vault_decode_request_payload,
        build_vault_export_request_payload, file_import_payload_from_data_url, format_review_queue,
        format_summary, parse_runtime_error, parse_runtime_success, render_runtime_response,
        validate_browser_import_size, BrowserFileReadMode, BrowserFlowState, InputMode,
        RuntimeReviewCandidate, RuntimeSummary, BROWSER_FILE_IMPORT_COPY,
        DEFAULT_FIELD_POLICY_JSON, EXPORT_FILENAME_WARNING_COPY, FETCH_UNAVAILABLE_MESSAGE,
        IDLE_REVIEW_QUEUE, IDLE_SUMMARY, MAX_BROWSER_IMPORT_BYTES,
    };
    use serde_json::json;

    type BrowserAppState = BrowserFlowState;

    #[test]
    fn portable_response_report_download_uses_safe_source_name_and_redacts_artifact() {
        let payload = build_portable_response_report_download(
            InputMode::PortableArtifactImport,
            Some("Patient Alice bundle.mdid-portable.json"),
            r#"{"artifact":{"records":[{"id":"phi-1"}]},"imported_record_count":1,"audit_event_count":2}"#,
        )
        .expect("portable import response should produce report download");

        assert_eq!(
            payload.file_name,
            "Patient_Alice_bundle-portable-artifact-import-report.json"
        );
        assert_eq!(payload.mime_type, "application/json");
        assert!(payload.is_text);
        let report: serde_json::Value = serde_json::from_slice(&payload.bytes).unwrap();
        assert_eq!(report["mode"], "portable_artifact_import");
        assert_eq!(report["imported_record_count"], 1);
        assert_eq!(report["audit_event_count"], 2);
        assert_eq!(report["artifact"], "redacted");
        assert!(!String::from_utf8(payload.bytes).unwrap().contains("phi-1"));
    }

    #[test]
    fn portable_response_report_download_rejects_non_portable_modes() {
        let error = build_portable_response_report_download(
            InputMode::CsvText,
            Some("rows.csv"),
            r#"{"summary":"ok"}"#,
        )
        .unwrap_err();

        assert_eq!(error, "Portable response report download is only available for portable artifact modes.");
    }

    #[test]
    fn tabular_report_download_payload_uses_safe_source_name_for_csv() {
        let mut state = BrowserFlowState::default();
        state.apply_imported_file("patient roster.csv", "name\nAlice", InputMode::CsvText);
        state.summary = "total_rows: 1\nencoded_cells: 1".to_string();
        state.review_queue = "No review items returned.".to_string();
        state.result_output = "name\nTOKEN_1".to_string();

        assert!(state.can_export_tabular_report());
        let payload = state.prepared_tabular_report_download_payload().unwrap();

        assert_eq!(payload.file_name, "patient_roster-tabular-report.json");
        assert_eq!(payload.mime_type, "application/json;charset=utf-8");
        assert!(payload.is_text);
        let report: serde_json::Value = serde_json::from_slice(&payload.bytes).unwrap();
        assert_eq!(report["mode"], "tabular_report");
        assert_eq!(report["input_mode"], "csv-text");
        assert_eq!(report["summary"], "total_rows: 1\nencoded_cells: 1");
        assert_eq!(report["review_queue"], "No review items returned.");
        assert!(report.get("rewritten_output").is_none());
    }

    #[test]
    fn tabular_report_download_payload_supports_xlsx_without_rewritten_bytes() {
        let mut state = BrowserFlowState::default();
        state.apply_imported_file("workbook.xlsx", "UEsDBAo=", InputMode::XlsxBase64);
        state.summary = "total_rows: 2\nencoded_cells: 2".to_string();
        state.review_queue = "- row 2 needs review".to_string();
        state.result_output = "UEsDBAo=".to_string();

        let payload = state.prepared_tabular_report_download_payload().unwrap();
        let report: serde_json::Value = serde_json::from_slice(&payload.bytes).unwrap();

        assert_eq!(payload.file_name, "workbook-tabular-report.json");
        assert_eq!(report["mode"], "tabular_report");
        assert_eq!(report["input_mode"], "xlsx-base64");
        assert_eq!(report["review_queue"], "- row 2 needs review");
        assert!(report.get("rewritten_output").is_none());
    }

    #[test]
    fn tabular_report_download_rejects_non_tabular_or_empty_output() {
        let mut state = BrowserFlowState::default();
        state.input_mode = InputMode::PdfBase64;
        state.result_output = "PDF rewrite/export unavailable".to_string();
        assert!(!state.can_export_tabular_report());
        assert!(state.prepared_tabular_report_download_payload().is_err());

        state.input_mode = InputMode::CsvText;
        state.result_output.clear();
        assert!(!state.can_export_tabular_report());
    }

    #[test]
    fn browser_decode_values_download_exports_only_decoded_values() {
        let state = BrowserFlowState {
            input_mode: InputMode::VaultDecode,
            imported_file_name: Some("Clinic Vault 2026.vault".to_string()),
            decoded_values_output: Some(
                serde_json::json!({
                    "decoded_values": {"patient-1": {"name": "Alice Example"}},
                    "vault_path": "/phi/vault",
                    "passphrase": "secret",
                    "audit_event": {"kind": "decode"}
                })
                .to_string(),
            ),
            ..BrowserFlowState::default()
        };

        assert!(state.can_export_decoded_values());
        let payload = state
            .prepared_decoded_values_download_payload()
            .expect("decoded values payload");
        let json: serde_json::Value = serde_json::from_slice(&payload.bytes).expect("json");
        let text = String::from_utf8(payload.bytes).expect("utf8");

        assert_eq!(payload.file_name, "clinic-vault-2026-decoded-values.json");
        assert_eq!(payload.mime_type, "application/json;charset=utf-8");
        assert!(payload.is_text);
        assert_eq!(json["mode"], "vault_decode_values");
        assert_eq!(json["decoded_values"]["patient-1"]["name"], "Alice Example");
        assert!(json.get("audit_event").is_none());
        assert!(!text.contains("/phi/vault"));
        assert!(!text.contains("secret"));
    }

    #[test]
    fn browser_decode_values_download_available_after_vault_decode_runtime_success() {
        let mut state = BrowserFlowState {
            input_mode: InputMode::VaultDecode,
            imported_file_name: Some("Clinic Vault 2026.vault".to_string()),
            active_submission_token: Some(7),
            is_submitting: true,
            ..BrowserFlowState::default()
        };
        let response = parse_runtime_success(
            InputMode::VaultDecode,
            "{\"values\":[{\"record_id\":\"patient-1\",\"original_value\":\"Alice Example\",\"token\":\"tok_patient_1\",\"scope\":\"demographics\"}],\"audit_event\":{\"kind\":\"vault.decode\"}}",
        )
        .expect("runtime response");

        state.apply_runtime_success(7, 0, response);

        assert!(state.can_export_decoded_values());
        assert_eq!(
            state.result_output,
            "Decoded values hidden for PHI safety. 1 value(s) decoded by bounded runtime endpoint."
        );
        let safe_payload = state.prepared_download_payload().expect("safe payload");
        let safe_text = String::from_utf8(safe_payload.bytes).expect("utf8");
        assert!(!safe_text.contains("Alice Example"));
        assert!(!safe_text.contains("tok_patient_1"));

        let decoded_payload = state
            .prepared_decoded_values_download_payload()
            .expect("decoded payload");
        let decoded_json: serde_json::Value =
            serde_json::from_slice(&decoded_payload.bytes).expect("decoded json");
        assert_eq!(decoded_json["mode"], "vault_decode_values");
        assert_eq!(
            decoded_json["decoded_values"][0]["original_value"],
            "Alice Example"
        );
        assert_eq!(decoded_json["decoded_values"][0]["token"], "tok_patient_1");
        assert_eq!(decoded_json["decoded_values"][0]["record_id"], "patient-1");
        assert_eq!(decoded_json["decoded_values"][0]["scope"], "demographics");
        assert!(decoded_json.get("audit_event").is_none());
        assert!(decoded_json.get("passphrase").is_none());
        assert!(decoded_json.get("vault_path").is_none());
    }

    #[test]
    fn decoded_values_download_filename_falls_back_without_source_file() {
        let state = BrowserFlowState {
            input_mode: InputMode::VaultDecode,
            decoded_values_output: Some(
                serde_json::json!({
                    "decoded_values": [{"original_value":"Alice Example","token":"tok_patient_1"}]
                })
                .to_string(),
            ),
            ..BrowserFlowState::default()
        };

        let payload = state
            .prepared_decoded_values_download_payload()
            .expect("decoded values payload");

        assert_eq!(payload.file_name, "mdid-browser-decoded-values.json");
    }

    #[test]
    fn browser_import_read_error_clears_decoded_values_export_state() {
        let mut state = BrowserFlowState {
            input_mode: InputMode::VaultDecode,
            decoded_values_output: Some(
                serde_json::json!({
                    "decoded_values": [{"original_value":"Alice Example","token":"tok_patient_1"}]
                })
                .to_string(),
            ),
            ..BrowserFlowState::default()
        };
        assert!(state.can_export_decoded_values());

        state.apply_import_read_error("Failed to read browser import payload.");

        assert!(!state.can_export_decoded_values());
        assert_eq!(
            state.error_banner.as_deref(),
            Some("Failed to read browser import payload.")
        );
    }

    #[test]
    fn browser_decode_values_download_is_unavailable_without_decoded_values() {
        let mut state = BrowserFlowState {
            input_mode: InputMode::VaultDecode,
            ..BrowserFlowState::default()
        };

        assert!(!state.can_export_decoded_values());
        assert_eq!(
            state.prepared_decoded_values_download_payload().unwrap_err(),
            "Decoded values download is only available after a successful vault decode response with decoded values."
        );

        state.input_mode = InputMode::VaultAuditEvents;
        state.decoded_values_output = Some(
            serde_json::json!({
                "decoded_values": {"patient-1": {"name": "Alice Example"}}
            })
            .to_string(),
        );

        assert!(!state.can_export_decoded_values());
    }

    #[test]
    fn vault_audit_pagination_status_reports_requested_window_and_next_page() {
        let mut state = BrowserAppState::default();
        state.input_mode = InputMode::VaultAuditEvents;
        state.vault_audit_limit = "25".to_string();
        state.vault_audit_offset = "50".to_string();
        state.summary = r#"{"event_count":25,"next_offset":75}"#.to_string();

        assert_eq!(
            state.vault_audit_pagination_status(),
            Some(
                "Showing audit events 51-75. More events may be available from offset 75."
                    .to_string()
            )
        );
    }

    #[test]
    fn vault_audit_pagination_status_omits_next_page_when_response_has_no_next_offset() {
        let mut state = BrowserAppState::default();
        state.input_mode = InputMode::VaultAuditEvents;
        state.vault_audit_limit = "10".to_string();
        state.vault_audit_offset = String::new();
        state.summary = r#"{"event_count":3}"#.to_string();

        assert_eq!(
            state.vault_audit_pagination_status(),
            Some("Showing audit events 1-3. No next audit page was reported.".to_string())
        );
    }

    #[test]
    fn vault_audit_pagination_status_uses_returned_event_count_for_page_window() {
        let mut state = BrowserAppState::default();
        state.input_mode = InputMode::VaultAuditEvents;
        state.vault_audit_offset = "100".to_string();
        state.summary =
            r#"{"event_count":150,"returned_event_count":50,"next_offset":150}"#.to_string();

        assert_eq!(
            state.vault_audit_pagination_status(),
            Some(
                "Showing audit events 101-150 of 150. More events may be available from offset 150."
                    .to_string()
            )
        );
    }

    #[test]
    fn vault_audit_pagination_status_reports_empty_page_without_invalid_range() {
        let mut state = BrowserAppState::default();
        state.input_mode = InputMode::VaultAuditEvents;
        state.vault_audit_offset = "150".to_string();
        state.summary = r#"{"event_count":150,"returned_event_count":0}"#.to_string();

        assert_eq!(
            state.vault_audit_pagination_status(),
            Some(
                "No audit events were returned for this page. No next audit page was reported."
                    .to_string()
            )
        );
    }

    #[test]
    fn vault_audit_pagination_status_reads_actual_runtime_output_after_success() {
        let mut state = BrowserAppState::default();
        state.input_mode = InputMode::VaultAuditEvents;
        state.vault_audit_offset = "100".to_string();
        let response = parse_runtime_success(
            InputMode::VaultAuditEvents,
            r#"{"event_count":150,"returned_event_count":50,"next_offset":150,"events":[]}"#,
        )
        .unwrap();
        state.result_output = response.rewritten_output;
        state.summary = response.summary;

        assert_eq!(
            state.vault_audit_pagination_status(),
            Some(
                "Showing audit events 101-150 of 150. More events may be available from offset 150."
                    .to_string()
            )
        );
    }

    #[test]
    fn vault_audit_pagination_status_uses_runtime_output_when_summary_json_lacks_counts() {
        let mut state = BrowserAppState::default();
        state.input_mode = InputMode::VaultAuditEvents;
        state.vault_audit_offset = "100".to_string();
        state.summary = r#"{"status":"ok"}"#.to_string();
        state.result_output =
            r#"{"event_count":150,"returned_event_count":50,"next_offset":150,"events":[]}"#
                .to_string();

        assert_eq!(
            state.vault_audit_pagination_status(),
            Some(
                "Showing audit events 101-150 of 150. More events may be available from offset 150."
                    .to_string()
            )
        );
    }

    #[test]
    fn vault_audit_pagination_status_is_absent_outside_vault_audit_mode() {
        let mut state = BrowserAppState::default();
        state.input_mode = InputMode::CsvText;
        state.vault_audit_limit = "25".to_string();
        state.vault_audit_offset = "50".to_string();
        state.summary = r#"{\"event_count\":25,\"next_offset\":75}"#.to_string();

        assert_eq!(state.vault_audit_pagination_status(), None);
    }

    #[test]
    fn xlsx_runtime_success_appends_safe_worksheet_disclosure_summary() {
        let body = json!({
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
            }
        });

        let parsed = parse_runtime_success(InputMode::XlsxBase64, &body.to_string()).unwrap();

        assert!(parsed.summary.contains("worksheet_disclosure:"));
        assert!(parsed.summary.contains("selected_sheet_name: Patients"));
        assert!(parsed.summary.contains("selected_sheet_index: 1"));
        assert!(parsed.summary.contains("total_sheet_count: 3"));
        assert!(parsed.summary.contains("first non-empty worksheet"));
        assert!(!parsed.summary.contains("Alice Patient"));
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn xlsx_output_download_decodes_base64_to_binary_payload() {
        let mut state = BrowserFlowState::default();
        state.input_mode = InputMode::XlsxBase64;
        state.result_output = base64::engine::general_purpose::STANDARD.encode(b"workbook-bytes");

        let payload = state.prepared_download_payload().expect("xlsx payload");

        assert_eq!(payload.file_name, "mdid-browser-output.xlsx");
        assert_eq!(
            payload.mime_type,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
        );
        assert_eq!(payload.bytes, b"workbook-bytes");
        assert!(!payload.is_text);
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn dicom_output_download_decodes_base64_to_binary_payload() {
        let mut state = BrowserFlowState::default();
        state.input_mode = InputMode::DicomBase64;
        state.imported_file_name = Some("CT Head.dcm".to_string());
        state.result_output = base64::engine::general_purpose::STANDARD.encode(b"dicom-bytes");

        let payload = state.prepared_download_payload().expect("dicom payload");

        assert_eq!(payload.file_name, "ct-head-deidentified.dcm");
        assert_eq!(payload.mime_type, "application/dicom");
        assert_eq!(payload.bytes, b"dicom-bytes");
        assert!(!payload.is_text);
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn csv_output_download_keeps_text_payload() {
        let mut state = BrowserFlowState::default();
        state.input_mode = InputMode::CsvText;
        state.result_output = "patient_id\nMDID-1\n".to_string();

        let payload = state.prepared_download_payload().expect("csv payload");

        assert_eq!(payload.file_name, "mdid-browser-output.csv");
        assert_eq!(payload.mime_type, "text/plain;charset=utf-8");
        assert_eq!(payload.bytes, b"patient_id\nMDID-1\n");
        assert!(payload.is_text);
    }

    #[test]
    fn pdf_review_download_exports_structured_json_report() {
        let mut state = BrowserFlowState {
            input_mode: InputMode::PdfBase64,
            result_output:
                "PDF rewrite/export unavailable: runtime returned review-only PDF analysis."
                    .to_string(),
            summary: "total_pages: 1\nocr_required_pages: 0".to_string(),
            review_queue: "- page 1 / patient_name / confidence 20 / review: <redacted>"
                .to_string(),
            ..BrowserFlowState::default()
        };
        state.imported_file_name = Some("Patient Doe.pdf".to_string());

        let payload = state.prepared_download_payload().expect("download payload");
        let json: serde_json::Value = serde_json::from_slice(&payload.bytes).expect("json report");

        assert_eq!(payload.file_name, "patient-doe-review-report.json");
        assert_eq!(payload.mime_type, "application/json;charset=utf-8");
        assert!(payload.is_text);
        assert_eq!(json["mode"], "PDF base64");
        assert_eq!(json["summary"], "total_pages: 1\nocr_required_pages: 0");
        assert!(json["output"]
            .as_str()
            .unwrap()
            .contains("review-only PDF analysis"));
    }

    #[test]
    fn pdf_review_download_uses_safe_source_name_when_no_imported_file_exists() {
        let mut state = BrowserFlowState {
            input_mode: InputMode::PdfBase64,
            source_name: "C:/records/Patient Jane MRI Scan.pdf".to_string(),
            result_output: "review only".to_string(),
            summary: "PDF review summary".to_string(),
            review_queue: "review queue".to_string(),
            ..BrowserFlowState::default()
        };
        state.imported_file_name = None;

        let payload = state.prepared_download_payload().expect("download payload");

        assert_eq!(
            payload.file_name,
            "patient-jane-mri-scan-review-report.json"
        );
        assert_eq!(payload.mime_type, "application/json;charset=utf-8");
        assert!(payload.is_text);
    }

    #[test]
    fn media_review_download_uses_safe_source_name_when_no_imported_file_exists() {
        let state = BrowserFlowState {
            input_mode: InputMode::MediaMetadataJson,
            source_name: "C:/incoming/Patient Face Photo.JPG".to_string(),
            imported_file_name: None,
            result_output: "metadata-only review".to_string(),
            summary: "Media review summary".to_string(),
            review_queue: "No review items returned.".to_string(),
            ..BrowserFlowState::default()
        };

        let payload = state
            .prepared_download_payload()
            .expect("media report download payload");

        assert_eq!(
            payload.file_name,
            "patient-face-photo-media-review-report.json"
        );
        assert_eq!(payload.mime_type, "application/json;charset=utf-8");
        assert!(payload.is_text);
    }

    #[test]
    fn media_review_download_prefers_visible_source_name_over_stale_imported_file() {
        let state = BrowserFlowState {
            input_mode: InputMode::MediaMetadataJson,
            source_name: "Patient Face Photo.JPG".to_string(),
            imported_file_name: Some("old-patient.csv".to_string()),
            result_output: "metadata-only review".to_string(),
            summary: "Media review summary".to_string(),
            review_queue: "No review items returned.".to_string(),
            ..BrowserFlowState::default()
        };

        let payload = state
            .prepared_download_payload()
            .expect("media report download payload");

        assert_eq!(
            payload.file_name,
            "patient-face-photo-media-review-report.json"
        );
    }

    #[test]
    fn media_review_download_ignores_stale_imported_file_for_placeholder_source_name() {
        let state = BrowserFlowState {
            input_mode: InputMode::MediaMetadataJson,
            source_name: "local-review.pdf".to_string(),
            imported_file_name: Some("old-patient.csv".to_string()),
            result_output: "metadata-only review".to_string(),
            summary: "Media review summary".to_string(),
            review_queue: "No review items returned.".to_string(),
            ..BrowserFlowState::default()
        };

        let payload = state
            .prepared_download_payload()
            .expect("media report download payload");

        assert_eq!(payload.file_name, "mdid-browser-media-review-report.json");
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn media_review_download_is_structured_and_phi_safe() {
        let mut state = BrowserFlowState::default();
        state.input_mode = InputMode::MediaMetadataJson;
        state.result_output =
            "Media rewrite/export unavailable: runtime returned metadata-only conservative review."
                .to_string();
        state.summary = "total_items: 1\nmetadata_only_items: 1\nvisual_review_required_items: 1\nunsupported_items: 0\nreview_required_candidates: 1\nrewritten_media_bytes_base64: null\noperator_notes: Jane Patient\ntotal_items_label: one\nmetadata_only_items: not-a-number".to_string();
        state.review_queue = "- PatientName / image / metadata_identifier / confidence 0.97 / value: <redacted>\n- FlowCytometryId / fcs / metadata_identifier / confidence 0.91 / value: <redacted>\n- VoiceNote / audio / metadata_identifier / confidence 0.88 / value: <redacted>".to_string();

        let payload = state.prepared_download_payload().expect("download payload");
        let report: serde_json::Value =
            serde_json::from_slice(&payload.bytes).expect("report json");
        let report_text = String::from_utf8(payload.bytes).expect("report utf8");

        assert_eq!(payload.file_name, "mdid-browser-media-review-report.json");
        assert_eq!(payload.mime_type, "application/json;charset=utf-8");
        assert!(payload.is_text);
        assert_eq!(report["mode"], "media_metadata_review");
        assert_eq!(report["summary"]["total_items"], 1);
        assert_eq!(
            report["summary"]["metadata_only_items"],
            serde_json::Value::Null
        );
        assert_eq!(report["summary"]["visual_review_required_items"], 1);
        assert_eq!(report["summary"]["unsupported_items"], 0);
        assert_eq!(report["summary"]["review_required_candidates"], 1);
        assert_eq!(
            report["summary"]["rewritten_media_bytes_base64"],
            serde_json::Value::Null
        );
        assert_eq!(report["review_queue"][0]["metadata_key"], "redacted-field");
        assert_eq!(report["review_queue"][0]["format"], "image");
        assert_eq!(report["review_queue"][1]["format"], "fcs");
        assert_eq!(report["review_queue"][2]["format"], "unknown");
        assert_eq!(report["review_queue"][0]["phi_type"], "metadata_identifier");
        assert!(report["summary"].get("operator_notes").is_none());
        assert!(report["summary"].get("total_items_label").is_none());
        assert_eq!(report["review_queue"][0]["confidence"], 0.97);
        assert_eq!(report["review_queue"][0]["value"], "redacted");
        assert!(!report_text.contains("Jane Patient"));
        assert!(!report_text.contains("PatientName"));
        assert!(!report_text.contains("source_value"));
    }

    #[test]
    fn browser_vault_response_download_is_structured_and_phi_safe() {
        let state = BrowserFlowState {
            input_mode: InputMode::VaultDecode,
            summary: "Decoded 2 requested records; values hidden in browser response.".to_string(),
            review_queue: "Review queue: no browser-visible decoded values.".to_string(),
            result_output: serde_json::json!({
                "decoded_values": {"patient-1": {"name": "Alice Example"}},
                "vault_path": "/phi/vault",
                "passphrase": "secret",
                "token": "MDID-123",
                "audit_event": {"kind": "decode", "record_ids": ["patient-1"]}
            })
            .to_string(),
            ..BrowserFlowState::default()
        };

        let payload = state.prepared_download_payload().expect("download payload");
        let report: serde_json::Value =
            serde_json::from_slice(&payload.bytes).expect("json report");

        assert_eq!(payload.file_name, "mdid-browser-vault-decode-response.json");
        assert_eq!(payload.mime_type, "application/json;charset=utf-8");
        assert_eq!(report["mode"], "vault_decode");
        assert_eq!(report["summary"], state.summary);
        assert_eq!(report["review_queue"], state.review_queue);
        assert!(report.get("output").is_none());
        let serialized = serde_json::to_string(&report).expect("serialized report");
        assert!(!serialized.contains("Alice Example"));
        assert!(!serialized.contains("/phi/vault"));
        assert!(!serialized.contains("secret"));
        assert!(!serialized.contains("MDID-123"));
        assert!(!serialized.contains("\"token\""));
        assert!(!serialized.contains("decoded_values"));
        assert!(!serialized.contains("audit_event"));
    }

    #[test]
    fn portable_review_download_exports_json_without_raw_runtime_body() {
        let state = BrowserFlowState {
            input_mode: InputMode::PortableArtifactInspect,
            result_output:
                "Portable artifact contains 2 record(s). Artifact contents are hidden.".to_string(),
            summary: "2 portable record(s) available for import.".to_string(),
            review_queue:
                "Portable artifact inspection completed without rendering original values or tokens."
                    .to_string(),
            ..BrowserFlowState::default()
        };

        let payload = state.prepared_download_payload().expect("download payload");
        let text = std::str::from_utf8(&payload.bytes).expect("utf8 json");
        let json: serde_json::Value = serde_json::from_str(text).expect("json report");

        assert_eq!(
            payload.file_name,
            "mdid-browser-portable-artifact-inspect.json"
        );
        assert_eq!(payload.mime_type, "application/json;charset=utf-8");
        assert!(payload.is_text);
        assert_eq!(json["mode"], "portable_artifact_inspect");
        assert_eq!(
            json["review_queue"],
            "Portable artifact inspection completed without rendering original values or tokens."
        );
        assert!(!text.contains("artifact_json"));
        assert!(!text.contains("original_value"));
    }

    #[test]
    fn browser_portable_response_downloads_use_safe_source_filenames() {
        let mut inspect_state = BrowserFlowState {
            input_mode: InputMode::PortableArtifactInspect,
            summary: "2 portable record(s) available for import.".to_string(),
            review_queue: "Portable artifact preview: values hidden in browser report.".to_string(),
            result_output: "Portable artifact contains 2 record(s). Artifact contents are hidden."
                .to_string(),
            ..BrowserFlowState::default()
        };
        inspect_state.imported_file_name = Some("../Clinic Export 2026.JSON".to_string());

        let inspect_payload = inspect_state
            .prepared_download_payload()
            .expect("inspect payload");

        assert_eq!(
            inspect_payload.file_name,
            "clinic-export-2026-portable-artifact-inspect.json"
        );
        assert_eq!(inspect_payload.mime_type, "application/json;charset=utf-8");
        assert!(inspect_payload.is_text);

        let mut import_state = BrowserFlowState {
            input_mode: InputMode::PortableArtifactImport,
            summary: "Imported 2 portable record(s); skipped 0 duplicate(s).".to_string(),
            review_queue: "Portable import response: artifact contents hidden.".to_string(),
            result_output: "Portable import completed; raw artifact payload hidden.".to_string(),
            ..BrowserFlowState::default()
        };
        import_state.imported_file_name = Some("Patient Bundle!!.json".to_string());

        let import_payload = import_state
            .prepared_download_payload()
            .expect("import payload");

        assert_eq!(
            import_payload.file_name,
            "patient-bundle-portable-artifact-import.json"
        );
        assert_eq!(import_payload.mime_type, "application/json;charset=utf-8");
        assert!(import_payload.is_text);
    }

    #[test]
    fn browser_vault_export_download_uses_safe_source_filename() {
        let state = BrowserFlowState {
            input_mode: InputMode::VaultExport,
            imported_file_name: Some("Clinic Vault Backup 2026.vault".to_string()),
            ..BrowserFlowState::default()
        };

        assert_eq!(
            state.suggested_export_file_name(),
            "Clinic_Vault_Backup_2026-portable-artifact.json"
        );
    }

    #[test]
    fn browser_vault_export_download_falls_back_when_source_stem_is_default() {
        let state = BrowserFlowState {
            input_mode: InputMode::VaultExport,
            imported_file_name: Some("***.vault".to_string()),
            ..BrowserFlowState::default()
        };

        assert_eq!(
            state.suggested_export_file_name(),
            "mdid-browser-portable-artifact.json"
        );
    }

    #[test]
    fn browser_vault_export_download_falls_back_when_source_stem_matches_default() {
        let state = BrowserFlowState {
            input_mode: InputMode::VaultExport,
            imported_file_name: Some("mdid-browser-output.vault".to_string()),
            ..BrowserFlowState::default()
        };

        assert_eq!(
            state.suggested_export_file_name(),
            "mdid-browser-portable-artifact.json"
        );
    }

    #[test]
    fn browser_vault_response_downloads_use_safe_source_filenames() {
        let mut audit_state = BrowserFlowState {
            input_mode: InputMode::VaultAuditEvents,
            imported_file_name: Some("Clinic Vault Backup 2026.vault".to_string()),
            ..BrowserFlowState::default()
        };
        audit_state.summary = "events returned: 2 / 2".to_string();
        audit_state.review_queue = "audit event summaries available".to_string();
        audit_state.result_output = "safe summary".to_string();

        let audit_payload = audit_state
            .prepared_download_payload()
            .expect("vault audit payload should be prepared");
        assert_eq!(
            audit_payload.file_name,
            "clinic-vault-backup-2026-vault-audit-events.json"
        );
        assert_eq!(audit_payload.mime_type, "application/json;charset=utf-8");
        let audit_json = String::from_utf8(audit_payload.bytes).expect("audit json utf8");
        assert!(audit_json.contains("\"mode\": \"vault_audit\""));
        assert!(audit_json.contains("events returned: 2 / 2"));
        assert!(!audit_json.contains("safe summary"));

        let decode_state = BrowserFlowState {
            input_mode: InputMode::VaultDecode,
            imported_file_name: Some("Clinic Vault Backup 2026.vault".to_string()),
            summary: "decoded count: 1".to_string(),
            review_queue: "decoded PHI is not included in the safe report".to_string(),
            result_output: "Jane Doe".to_string(),
            ..BrowserFlowState::default()
        };

        let decode_payload = decode_state
            .prepared_download_payload()
            .expect("vault decode payload should be prepared");
        assert_eq!(
            decode_payload.file_name,
            "clinic-vault-backup-2026-vault-decode-response.json"
        );
        assert_eq!(decode_payload.mime_type, "application/json;charset=utf-8");
        let decode_json = String::from_utf8(decode_payload.bytes).expect("decode json utf8");
        assert!(decode_json.contains("\"mode\": \"vault_decode\""));
        assert!(decode_json.contains("decoded count: 1"));
        assert!(!decode_json.contains("Jane Doe"));
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn binary_output_download_rejects_invalid_base64() {
        let mut state = BrowserFlowState::default();
        state.input_mode = InputMode::XlsxBase64;
        state.result_output = "not valid base64".to_string();

        let error = state
            .prepared_download_payload()
            .expect_err("invalid base64 should fail");

        assert_eq!(
            error,
            "Browser output download could not decode rewritten XLSX base64 bytes."
        );
    }

    #[test]
    fn media_metadata_json_requires_source_name_with_media_specific_label() {
        assert!(InputMode::MediaMetadataJson.requires_source_name());
        assert_eq!(
            InputMode::MediaMetadataJson.source_name_label(),
            "media metadata JSON"
        );
    }

    #[test]
    fn imported_csv_suggests_sanitized_deidentified_export_name() {
        let mut state = BrowserFlowState::default();
        state.apply_imported_file("Clinic Patient List.csv", "name\nAda", InputMode::CsvText);

        assert_eq!(
            state.suggested_export_file_name(),
            "clinic-patient-list-deidentified.csv"
        );
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn media_metadata_default_placeholder_source_name_uses_static_export_name() {
        let mut state = BrowserFlowState::default();
        state.input_mode = InputMode::MediaMetadataJson;

        assert_eq!(
            state.suggested_export_file_name(),
            "mdid-browser-media-review-report.json"
        );
        assert_ne!(
            state.suggested_export_file_name(),
            "local-review-media-review-report.json"
        );
    }

    #[test]
    fn imported_pdf_suggests_sanitized_review_report_name_without_phi() {
        let mut state = BrowserFlowState::default();
        state.apply_imported_file(
            "../Patient #42 Intake.PDF",
            "JVBERi0=",
            InputMode::PdfBase64,
        );

        assert_eq!(
            state.suggested_export_file_name(),
            "patient-42-intake-review-report.json"
        );
    }

    #[test]
    fn manual_default_csv_keeps_static_browser_output_name() {
        let state = BrowserFlowState::default();

        assert_eq!(
            state.suggested_export_file_name(),
            "mdid-browser-output.csv"
        );
    }

    #[test]
    fn imported_vault_export_suggests_safe_source_portable_artifact_name() {
        let mut state = BrowserFlowState::default();
        state.apply_imported_file(
            "Patient Portable Artifact.json",
            r#"{"record_count":1}"#,
            InputMode::VaultExport,
        );

        assert_eq!(
            state.suggested_export_file_name(),
            "Patient_Portable_Artifact-portable-artifact.json"
        );
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn dicom_download_uses_safe_source_name_when_no_imported_file_exists() {
        let mut state = BrowserFlowState::default();
        state.input_mode = InputMode::DicomBase64;
        state.source_name = r"C:\incoming\CT Series 01.dcm".to_string();
        state.imported_file_name = None;

        assert_eq!(
            state.suggested_export_file_name(),
            "CT-Series-01-deidentified.dcm"
        );
    }

    #[test]
    fn imported_dicom_and_media_metadata_suggest_mode_specific_export_names() {
        let mut dicom_state = BrowserFlowState::default();
        dicom_state.apply_imported_file(
            "C:\\scans\\Patient MRI 2026.dicom",
            "ZGljb20=",
            InputMode::DicomBase64,
        );

        assert_eq!(
            dicom_state.suggested_export_file_name(),
            "patient-mri-2026-deidentified.dcm"
        );

        let mut media_state = BrowserFlowState::default();
        media_state.apply_imported_file(
            "../Clinic Video Metadata.JSON",
            r#"{"objects":[]}"#,
            InputMode::MediaMetadataJson,
        );

        assert_eq!(
            media_state.suggested_export_file_name(),
            "clinic-video-metadata-media-review-report.json"
        );
    }

    #[test]
    fn imported_filename_stem_is_bounded_without_trailing_hyphen() {
        let mut state = BrowserFlowState::default();
        state.apply_imported_file(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-Patient Name.csv",
            "name\nAda",
            InputMode::CsvText,
        );

        assert_eq!(
            state.suggested_export_file_name(),
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-deidentified.csv"
        );
    }

    #[test]
    fn export_filename_phi_warning_copy_is_static_and_bounded() {
        assert!(EXPORT_FILENAME_WARNING_COPY.contains("suggested download filenames"));
        assert!(EXPORT_FILENAME_WARNING_COPY.contains("imported filenames"));
        assert!(EXPORT_FILENAME_WARNING_COPY.contains("rename"));
        assert!(!EXPORT_FILENAME_WARNING_COPY.contains("vault passphrase"));
        assert!(!EXPORT_FILENAME_WARNING_COPY.contains("decoded"));
    }

    #[test]
    fn vault_export_mode_uses_existing_runtime_endpoint() {
        assert_eq!(
            InputMode::from_select_value("vault-export"),
            InputMode::VaultExport
        );
        assert_eq!(InputMode::VaultExport.select_value(), "vault-export");
        assert_eq!(InputMode::VaultExport.endpoint(), "/vault/export");
        assert!(!InputMode::VaultExport.requires_field_policy());
        assert!(!InputMode::VaultExport.requires_source_name());
    }

    #[test]
    fn vault_export_payload_maps_form_to_runtime_contract() {
        let payload = build_vault_export_request_payload(
            " /tmp/vault.json ",
            " passphrase ",
            r#"["11111111-1111-1111-1111-111111111111"]"#,
            " portable secret ",
            " export for local review ",
        )
        .expect("payload");

        assert_eq!(payload["vault_path"], "/tmp/vault.json");
        assert_eq!(payload["vault_passphrase"], "passphrase");
        assert_eq!(
            payload["record_ids"][0],
            "11111111-1111-1111-1111-111111111111"
        );
        assert_eq!(payload["export_passphrase"], "portable secret");
        assert_eq!(payload["context"], "export for local review");
        assert_eq!(payload["requested_by"], "browser");
    }

    #[test]
    fn browser_portable_export_payload_rejects_duplicate_record_ids() {
        let err = build_vault_export_request_payload(
            "vault",
            "pw",
            r#"["550e8400-e29b-41d4-a716-446655440000","550e8400-e29b-41d4-a716-446655440000"]"#,
            "portable passphrase",
            "case handoff",
        )
        .expect_err("browser must reject duplicate export record ids");
        assert!(err.contains("duplicate record id"));
        assert!(!err.contains("550e8400"));
    }

    #[test]
    fn portable_artifact_modes_use_existing_runtime_endpoints() {
        assert_eq!(
            InputMode::from_select_value("portable-artifact-inspect"),
            InputMode::PortableArtifactInspect
        );
        assert_eq!(
            InputMode::PortableArtifactInspect.select_value(),
            "portable-artifact-inspect"
        );
        assert_eq!(
            InputMode::PortableArtifactInspect.endpoint(),
            "/portable-artifacts/inspect"
        );
        assert!(!InputMode::PortableArtifactInspect.requires_field_policy());
        assert!(!InputMode::PortableArtifactInspect.requires_source_name());

        assert_eq!(
            InputMode::from_select_value("portable-artifact-import"),
            InputMode::PortableArtifactImport
        );
        assert_eq!(
            InputMode::PortableArtifactImport.select_value(),
            "portable-artifact-import"
        );
        assert_eq!(
            InputMode::PortableArtifactImport.endpoint(),
            "/portable-artifacts/import"
        );
        assert!(!InputMode::PortableArtifactImport.requires_field_policy());
        assert!(!InputMode::PortableArtifactImport.requires_source_name());
    }

    #[test]
    fn imported_portable_artifact_preserves_selected_import_mode() {
        let mut state = BrowserFlowState {
            input_mode: InputMode::PortableArtifactImport,
            ..BrowserFlowState::default()
        };

        let detected_mode = InputMode::from_file_name("clinic-mapping.mdid-portable.json")
            .expect("portable artifact filename should be recognized");
        let resolved_mode = state.mode_for_imported_file(detected_mode);
        state.apply_imported_file(
            "clinic-mapping.mdid-portable.json",
            r#"{\"version\":1}"#,
            resolved_mode,
        );

        assert_eq!(state.input_mode, InputMode::PortableArtifactImport);
        assert_eq!(state.payload, r#"{\"version\":1}"#);
        assert_eq!(
            state.suggested_export_file_name(),
            "clinic-mapping-mdid-portable-portable-artifact-import.json"
        );
    }

    #[test]
    fn imported_portable_artifact_defaults_to_inspect_outside_import_mode() {
        let mut state = BrowserFlowState {
            input_mode: InputMode::CsvText,
            ..BrowserFlowState::default()
        };

        let detected_mode = InputMode::from_file_name("clinic-mapping.mdid-portable.json")
            .expect("portable artifact filename should be recognized");
        let resolved_mode = state.mode_for_imported_file(detected_mode);
        state.apply_imported_file(
            "clinic-mapping.mdid-portable.json",
            r#"{\"version\":1}"#,
            resolved_mode,
        );

        assert_eq!(state.input_mode, InputMode::PortableArtifactInspect);
        assert_eq!(state.payload, r#"{\"version\":1}"#);
        assert_eq!(
            state.suggested_export_file_name(),
            "clinic-mapping-mdid-portable-portable-artifact-inspect.json"
        );
    }

    #[test]
    fn portable_artifact_payloads_map_form_to_runtime_contract() {
        let artifact_json = r#"{"kdf":{"algorithm":"argon2id","version":19,"memory_cost_kib":19456,"iterations":2,"parallelism":1,"output_len":32},"verifier_b64":"dmVyaWZpZXI=","salt_b64":"c2FsdA==","nonce_b64":"bm9uY2U=","ciphertext_b64":"ZW5jcnlwdGVkLXBvcnRhYmxlLWFydGlmYWN0"}"#;
        let inspect =
            build_portable_artifact_inspect_request_payload(artifact_json, " portable secret ")
                .expect("inspect payload");
        assert_eq!(
            inspect["artifact"]["ciphertext_b64"],
            "ZW5jcnlwdGVkLXBvcnRhYmxlLWFydGlmYWN0"
        );
        assert_eq!(inspect["portable_passphrase"], "portable secret");

        let import = build_portable_artifact_import_request_payload(
            " /tmp/vault.json ",
            " vault secret ",
            artifact_json,
            " portable secret ",
            " import for local review ",
        )
        .expect("import payload");
        assert_eq!(import["vault_path"], "/tmp/vault.json");
        assert_eq!(import["vault_passphrase"], "vault secret");
        assert_eq!(
            import["artifact"]["ciphertext_b64"],
            "ZW5jcnlwdGVkLXBvcnRhYmxlLWFydGlmYWN0"
        );
        assert_eq!(import["portable_passphrase"], "portable secret");
        assert_eq!(import["context"], "import for local review");
        assert_eq!(import["requested_by"], "browser");
    }

    #[test]
    fn portable_artifact_payloads_reject_blank_required_fields_and_bad_uuid() {
        assert!(build_vault_export_request_payload("", "pw", "[]", "portable", "context").is_err());
        assert!(
            build_vault_export_request_payload("vault", "pw", "[]", "portable", "context").is_err()
        );
        assert!(build_vault_export_request_payload(
            "vault",
            "pw",
            r#"["not-a-uuid"]"#,
            "portable",
            "context"
        )
        .is_err());
        assert!(build_portable_artifact_inspect_request_payload("{}", "").is_err());
        assert!(build_portable_artifact_inspect_request_payload("[]", "portable").is_err());
        assert!(build_portable_artifact_inspect_request_payload("{", "portable").is_err());
        assert!(build_portable_artifact_import_request_payload(
            "vault", "pw", "{}", "portable", ""
        )
        .is_err());
    }

    #[test]
    fn portable_artifact_runtime_success_hides_artifact_values_and_raw_audit_detail() {
        let export = json!({
            "artifact": {"kdf":{"algorithm":"argon2id","version":19,"memory_cost_kib":19456,"iterations":2,"parallelism":1,"output_len":32}, "verifier_b64":"dmVyaWZpZXI=", "salt_b64":"c2FsdC1ieXRlcw==", "nonce_b64":"bm9uY2UtYnl0ZXM=", "ciphertext_b64":"bW9jay1hZXMtZ2NtLWNpcGhlcnRleHQtYnl0ZXM="}
        });
        let rendered_export = parse_runtime_success(InputMode::VaultExport, &export.to_string())
            .expect("export render");
        assert!(rendered_export
            .summary
            .contains("Portable artifact created"));
        assert!(rendered_export
            .review_queue
            .contains("encrypted portable artifact"));
        assert!(rendered_export.rewritten_output.contains("ciphertext_b64"));
        assert!(rendered_export
            .rewritten_output
            .contains("bW9jay1hZXMtZ2NtLWNpcGhlcnRleHQtYnl0ZXM="));
        assert!(!rendered_export.summary.contains("secret salt"));
        assert!(!rendered_export.review_queue.contains("secret nonce"));

        let inspect = json!({
            "record_count": 1,
            "records": [{"original_value": "Jane Patient", "token": "TOKEN-1"}]
        });
        let rendered_inspect =
            parse_runtime_success(InputMode::PortableArtifactInspect, &inspect.to_string())
                .expect("inspect render");
        assert!(rendered_inspect.summary.contains("1 portable record"));
        assert!(!rendered_inspect.rewritten_output.contains("Jane Patient"));
        assert!(!rendered_inspect.rewritten_output.contains("TOKEN-1"));

        let import = json!({
            "imported_record_count": 1,
            "duplicate_record_count": 2,
            "audit_event": {"kind": "import", "detail": "imported MRN 123", "actor": "browser"}
        });
        let rendered_import =
            parse_runtime_success(InputMode::PortableArtifactImport, &import.to_string())
                .expect("import render");
        assert!(rendered_import.summary.contains("1 imported"));
        assert!(rendered_import.review_queue.contains("2 duplicate"));
        assert!(!rendered_import.rewritten_output.contains("MRN 123"));
    }

    #[test]
    fn vault_export_runtime_success_renders_downloadable_encrypted_artifact_json() {
        let export = json!({
            "artifact": {
                "kdf": {
                    "algorithm": "argon2id",
                    "version": 19,
                    "memory_cost_kib": 19456,
                    "iterations": 2,
                    "parallelism": 1,
                    "output_len": 32
                },
                "verifier_b64": "dmVyaWZpZXI=",
                "salt_b64": "c2FsdA==",
                "nonce_b64": "bm9uY2U=",
                "ciphertext_b64": "ZW5jcnlwdGVkLXBvcnRhYmxlLWFydGlmYWN0"
            }
        });

        let rendered = parse_runtime_success(InputMode::VaultExport, &export.to_string())
            .expect("valid vault export success renders artifact JSON");

        assert!(rendered.summary.contains("Portable artifact created"));
        assert!(rendered
            .review_queue
            .contains("encrypted portable artifact"));
        assert!(rendered.rewritten_output.contains("\"ciphertext_b64\""));
        assert!(rendered
            .rewritten_output
            .contains("ZW5jcnlwdGVkLXBvcnRhYmxlLWFydGlmYWN0"));
        assert!(!rendered
            .summary
            .contains("ZW5jcnlwdGVkLXBvcnRhYmxlLWFydGlmYWN0"));
        assert!(!rendered.review_queue.contains("MDID-1"));
    }

    #[test]
    fn vault_export_runtime_success_rejects_malformed_contract() {
        let malformed = json!({
            "message": "exported record 11111111-1111-1111-1111-111111111111 for Jane Patient",
            "artifact": "not an artifact object"
        });

        let error = parse_runtime_success(InputMode::VaultExport, &malformed.to_string())
            .expect_err("malformed vault export success should fail closed");

        assert!(error.contains("Vault export response missing artifact object"));
        assert!(!error.contains("Jane Patient"));
        assert!(!error.contains("11111111-1111-1111-1111-111111111111"));
    }

    #[test]
    fn vault_export_runtime_success_rejects_object_artifact_without_encrypted_fields() {
        let malformed = json!({
            "artifact": {
                "original_value": "Jane Patient",
                "token": "MDID-SECRET-TOKEN"
            }
        });

        let error = parse_runtime_success(InputMode::VaultExport, &malformed.to_string())
            .expect_err("object-shaped PHI artifact should fail closed");

        assert!(error.contains("missing encrypted portable artifact fields"));
        assert!(!error.contains("Jane Patient"));
        assert!(!error.contains("MDID-SECRET-TOKEN"));
    }

    #[test]
    fn vault_decode_mode_uses_existing_runtime_endpoint() {
        let mode = InputMode::from_select_value("vault-decode");

        assert_eq!(mode, InputMode::VaultDecode);
        assert_eq!(mode.select_value(), "vault-decode");
        assert_eq!(mode.label(), "Vault decode");
        assert_eq!(mode.endpoint(), "/vault/decode");
        assert!(!mode.requires_field_policy());
        assert!(!mode.requires_source_name());
        assert_eq!(mode.browser_file_read_mode(), BrowserFileReadMode::Text);
        let disclosure = mode
            .disclosure_copy()
            .expect("vault decode mode has bounded disclosure");
        assert!(disclosure.contains("explicit record ids"));
        assert!(disclosure.contains("does not browse vault contents"));
        assert!(disclosure.contains("does not export"));
        assert!(disclosure.contains("does not add auth/session"));
        assert!(disclosure.contains("broader workflow behavior"));
    }

    #[test]
    fn vault_decode_payload_maps_form_to_runtime_contract() {
        let payload = build_vault_decode_request_payload(
            " /tmp/local-vault ",
            " passphrase kept local ",
            r#"["550e8400-e29b-41d4-a716-446655440000","550e8400-e29b-41d4-a716-446655440001"]"#,
            " local-output.json ",
            " bounded local decode for named records ",
        )
        .expect("valid bounded decode payload");

        assert_eq!(payload["vault_path"], "/tmp/local-vault");
        assert_eq!(payload["vault_passphrase"], "passphrase kept local");
        assert_eq!(
            payload["record_ids"],
            json!([
                "550e8400-e29b-41d4-a716-446655440000",
                "550e8400-e29b-41d4-a716-446655440001"
            ])
        );
        assert_eq!(payload["output_target"], "local-output.json");
        assert_eq!(
            payload["justification"],
            "bounded local decode for named records"
        );
        assert_eq!(payload["requested_by"], "browser");

        let state = BrowserFlowState {
            input_mode: InputMode::VaultDecode,
            vault_path: " /tmp/local-vault ".to_string(),
            vault_passphrase: " passphrase kept local ".to_string(),
            vault_decode_record_ids_json: r#"["550e8400-e29b-41d4-a716-446655440000"]"#.to_string(),
            vault_decode_output_target: " local-output.json ".to_string(),
            vault_decode_justification: " bounded local decode ".to_string(),
            field_policy_json: String::new(),
            source_name: String::new(),
            payload: String::new(),
            ..BrowserFlowState::default()
        };

        let request = state.validate_submission().expect("valid decode request");
        assert_eq!(request.endpoint, "/vault/decode");
        let body: serde_json::Value = serde_json::from_str(&request.body_json).unwrap();
        assert_eq!(
            body["record_ids"],
            json!(["550e8400-e29b-41d4-a716-446655440000"])
        );
        assert!(body.get("policies").is_none());
        assert!(body.get("field_policies").is_none());
        assert!(body.get("source_name").is_none());
    }

    #[test]
    fn browser_vault_decode_payload_rejects_duplicate_record_ids() {
        let err = build_vault_decode_request_payload(
            "vault",
            "pw",
            r#"["550e8400-e29b-41d4-a716-446655440000","550e8400-e29b-41d4-a716-446655440000"]"#,
            "desktop",
            "case review",
        )
        .expect_err("browser must reject duplicate decode record ids");
        assert!(err.contains("duplicate record id"));
        assert!(!err.contains("550e8400"));
    }

    #[test]
    fn parse_vault_decode_runtime_success_hides_decoded_values_and_audit_detail() {
        let decode_token = concat!("MDID", "-", "123");
        let response = parse_runtime_success(
            InputMode::VaultDecode,
            &json!({
                "values": [
                    {
                        "record_id": "550e8400-e29b-41d4-a716-446655440000",
                        "phi_type": "patient_name",
                        "original_value": "Jane Patient",
                        "token": decode_token
                    }
                ],
                "audit_event": {
                    "kind": "vault_decode",
                    "detail": "decoded Jane Patient for oncology board"
                },
                "output_target": "local-output.json",
                "justification": "oncology board"
            })
            .to_string(),
        )
        .unwrap();

        assert_eq!(response.summary, "Vault decode completed for 1 value(s).");
        assert_eq!(response.review_queue, "- vault_decode");
        assert!(response.rewritten_output.contains("Decoded values hidden"));

        let rendered = format!(
            "{}\n{}\n{}",
            response.summary, response.review_queue, response.rewritten_output
        );
        assert!(!rendered.contains("Jane Patient"));
        assert!(!rendered.contains("MDID-123"));
        assert!(!rendered.contains("oncology"));
    }

    #[test]
    fn vault_decode_payload_rejects_missing_scope_and_blank_fields() {
        for (
            vault_path,
            vault_passphrase,
            record_ids_json,
            output_target,
            justification,
            expected,
        ) in [
            (
                "",
                "passphrase",
                r#"["550e8400-e29b-41d4-a716-446655440000"]"#,
                "target",
                "why",
                "Vault path",
            ),
            (
                "vault",
                "",
                r#"["550e8400-e29b-41d4-a716-446655440000"]"#,
                "target",
                "why",
                "Vault passphrase",
            ),
            ("vault", "passphrase", "[]", "target", "why", "record ids"),
            (
                "vault",
                "passphrase",
                r#"["not-a-uuid"]"#,
                "target",
                "why",
                "UUID",
            ),
            (
                "vault",
                "passphrase",
                r#"["550e8400-e29b-41d4-a716-446655440000"]"#,
                "",
                "why",
                "Output target",
            ),
            (
                "vault",
                "passphrase",
                r#"["550e8400-e29b-41d4-a716-446655440000"]"#,
                "target",
                "",
                "Justification",
            ),
        ] {
            let error = build_vault_decode_request_payload(
                vault_path,
                vault_passphrase,
                record_ids_json,
                output_target,
                justification,
            )
            .expect_err("invalid decode payload must be rejected before localhost submission");

            assert!(
                error.contains(expected),
                "expected error {error:?} to mention {expected:?}"
            );
        }
    }

    #[test]
    fn vault_audit_events_mode_uses_existing_read_only_runtime_endpoint() {
        let mode = InputMode::from_select_value("vault-audit-events");

        assert_eq!(mode, InputMode::VaultAuditEvents);
        assert_eq!(mode.select_value(), "vault-audit-events");
        assert_eq!(mode.endpoint(), "/vault/audit/events");
        assert!(!mode.requires_field_policy());
        assert!(!mode.requires_source_name());
        assert_eq!(mode.browser_file_read_mode(), BrowserFileReadMode::Text);
        assert!(mode
            .disclosure_copy()
            .expect("vault audit mode has bounded disclosure")
            .contains("read-only"));
    }

    #[test]
    fn vault_audit_payload_maps_text_form_to_bounded_runtime_contract() {
        let payload = build_vault_audit_request_payload(
            "/tmp/local-vault",
            "passphrase kept local",
            "decode",
            "browser",
            "25",
            "10",
        )
        .expect("valid bounded audit payload");

        assert_eq!(payload["vault_path"], "/tmp/local-vault");
        assert_eq!(payload["vault_passphrase"], "passphrase kept local");
        assert_eq!(payload["kind"], "decode");
        assert_eq!(payload["actor"], "browser");
        assert_eq!(payload["limit"], 25);
        assert_eq!(payload["offset"], 10);
    }

    #[test]
    fn vault_audit_payload_omits_blank_optional_filters() {
        let payload = build_vault_audit_request_payload(
            "/tmp/local-vault",
            "passphrase kept local",
            " ",
            "",
            "",
            "",
        )
        .expect("blank optional filters are valid");

        assert_eq!(payload["vault_path"], "/tmp/local-vault");
        assert_eq!(payload["vault_passphrase"], "passphrase kept local");
        assert!(payload.get("kind").is_none());
        assert!(payload.get("actor").is_none());
        assert!(payload.get("limit").is_none());
        assert!(payload.get("offset").is_none());
    }

    #[test]
    fn vault_audit_payload_omits_zero_offset() {
        let payload = build_vault_audit_request_payload(
            "/tmp/local-vault",
            "passphrase kept local",
            "",
            "",
            "",
            "0",
        )
        .expect("zero offset is the runtime default");

        assert!(payload.get("offset").is_none());
    }

    #[test]
    fn vault_audit_payload_rejects_invalid_limit() {
        let error = build_vault_audit_request_payload(
            "/tmp/local-vault",
            "passphrase kept local",
            "decode",
            "browser",
            "not-a-number",
            "",
        )
        .expect_err("invalid limit must be rejected before localhost submission");

        assert!(error.contains("limit"));
    }

    #[test]
    fn vault_audit_payload_rejects_zero_limit() {
        let error = build_vault_audit_request_payload(
            "/tmp/local-vault",
            "passphrase kept local",
            "decode",
            "browser",
            "0",
            "",
        )
        .expect_err("zero limit must be rejected before localhost submission");

        assert!(error.contains("positive"));
    }

    #[test]
    fn vault_audit_payload_rejects_invalid_offset() {
        let error = build_vault_audit_request_payload(
            "/tmp/local-vault",
            "passphrase kept local",
            "decode",
            "browser",
            "",
            "not-a-number",
        )
        .expect_err("invalid offset must be rejected before localhost submission");

        assert!(error.contains("offset"));
    }

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
            error_banner: Some(
                "Runtime fallback included response body for Jane Patient".to_string(),
            ),
            ..BrowserFlowState::default()
        };

        let debug_output = format!("{state:?}");

        assert!(!debug_output.contains("Jane Patient"));
        assert!(debug_output.contains("error_banner"));
    }

    #[test]
    fn browser_flow_state_debug_redacts_summary_text() {
        let state = BrowserFlowState {
            summary: "Jane Patient page label".to_string(),
            ..BrowserFlowState::default()
        };

        let debug_output = format!("{state:?}");

        assert!(!debug_output.contains("Jane Patient page label"));
        assert!(debug_output.contains("summary: \"<redacted>\""));
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
        assert_eq!(
            InputMode::from_file_name("patients.csv"),
            Some(InputMode::CsvText)
        );
        assert_eq!(
            InputMode::from_file_name("workbook.XLSX"),
            Some(InputMode::XlsxBase64)
        );
        assert_eq!(
            InputMode::from_file_name("scan.PDF"),
            Some(InputMode::PdfBase64)
        );
        assert_eq!(InputMode::from_file_name("archive.zip"), None);
    }

    #[test]
    fn portable_artifact_json_filenames_select_inspect_mode() {
        assert_eq!(
            InputMode::from_file_name("mdid-browser-portable-artifact.json"),
            Some(InputMode::PortableArtifactInspect)
        );
        assert_eq!(
            InputMode::from_file_name("clinic-export.MDID-PORTABLE.JSON"),
            Some(InputMode::PortableArtifactInspect)
        );
        assert_eq!(
            InputMode::from_file_name("clinic.export.mdid-portable.json"),
            Some(InputMode::PortableArtifactInspect)
        );
    }

    #[test]
    fn ordinary_json_filenames_still_select_media_metadata_mode() {
        assert_eq!(
            InputMode::from_file_name("media-metadata.json"),
            Some(InputMode::MediaMetadataJson)
        );
        assert_eq!(
            InputMode::from_file_name("portable-not-artifact.json"),
            Some(InputMode::MediaMetadataJson)
        );
        assert_eq!(
            InputMode::from_file_name("not-mdid-browser-portable-artifact.json"),
            Some(InputMode::MediaMetadataJson)
        );
    }

    #[test]
    fn imported_dicom_file_selects_dicom_mode_and_base64_read() {
        assert_eq!(
            InputMode::from_file_name("scan.dcm"),
            Some(InputMode::DicomBase64)
        );
        assert_eq!(
            InputMode::from_file_name("SCAN.DICOM"),
            Some(InputMode::DicomBase64)
        );
        assert_eq!(
            InputMode::DicomBase64.browser_file_read_mode(),
            BrowserFileReadMode::DataUrlBase64
        );
    }

    #[test]
    fn dicom_mode_builds_bounded_runtime_request() {
        let state = BrowserFlowState {
            input_mode: InputMode::DicomBase64,
            payload: "ZGljb20=".to_string(),
            source_name: "local-scan.dcm".to_string(),
            ..BrowserFlowState::default()
        };

        let request = state.validate_submission().expect("valid DICOM request");

        assert_eq!(request.endpoint, "/dicom/deidentify");
        let body: serde_json::Value = serde_json::from_str(&request.body_json).unwrap();
        assert_eq!(body["dicom_bytes_base64"], "ZGljb20=");
        assert_eq!(body["source_name"], "local-scan.dcm");
        assert_eq!(body["private_tag_policy"], "remove");
    }

    #[test]
    fn dicom_response_renders_actual_runtime_summary_review_queue_and_rewritten_bytes() {
        let response = r#"{
            "summary": {
                "total_tags": 7,
                "encoded_tags": 2,
                "review_required_tags": 1,
                "removed_private_tags": 3,
                "remapped_uids": 4,
                "burned_in_suspicions": 1,
                "pixel_redaction_performed": false,
                "burned_in_review_required": true,
                "burned_in_annotation_notice": "DICOM pixel data was not inspected or redacted; burned-in annotations require separate visual review.",
                "burned_in_disclosure": "DICOM pixel data was not inspected or redacted; burned-in annotations require separate visual review."
            },
            "review_queue": [
                {"tag": {"group": 16, "element": 16, "keyword": "PatientName"}, "phi_type": "patient_name", "value": "Jane Patient", "decision": "Review"}
            ],
            "sanitized_file_name": "scan-deidentified.dcm",
            "rewritten_dicom_bytes_base64": "cmV3cml0dGVu"
        }"#;

        let rendered = render_runtime_response(InputMode::DicomBase64, response)
            .expect("DICOM response renders");

        assert!(rendered.summary.contains("total_tags: 7"));
        assert!(rendered.summary.contains("encoded_tags: 2"));
        assert!(rendered.summary.contains("review_required_tags: 1"));
        assert!(rendered.summary.contains("removed_private_tags: 3"));
        assert!(rendered.summary.contains("remapped_uids: 4"));
        assert!(rendered.summary.contains("burned_in_suspicions: 1"));
        assert!(rendered
            .summary
            .contains("pixel_redaction_performed: false"));
        assert!(rendered.summary.contains("burned_in_review_required: true"));
        assert!(rendered
            .summary
            .contains("DICOM pixel data was not inspected or redacted; burned-in annotations require separate visual review."));
        assert!(rendered.summary.contains("burned_in_disclosure: DICOM pixel data was not inspected or redacted; burned-in annotations require separate visual review."));
        assert_eq!(
            rendered.review_queue,
            "- tag (0010,0010) PatientName / patient_name / Review / value: <redacted>"
        );
        assert!(!rendered.review_queue.contains("Jane Patient"));
        assert!(rendered.rewritten_output.contains("cmV3cml0dGVu"));
    }

    #[test]
    fn dicom_mode_discloses_bounded_browser_limits() {
        assert!(InputMode::DicomBase64.disclosure_copy().unwrap().contains(
            "DICOM mode uses the existing local runtime tag-level de-identification route"
        ));
        assert!(InputMode::DicomBase64
            .disclosure_copy()
            .unwrap()
            .contains("DICOM pixel data was not inspected or redacted; burned-in annotations require separate visual review."));
        assert!(InputMode::DicomBase64
            .disclosure_copy()
            .unwrap()
            .contains("does not add pixel redaction"));
        assert_eq!(
            BrowserFlowState {
                input_mode: InputMode::DicomBase64,
                ..BrowserFlowState::default()
            }
            .suggested_export_file_name(),
            "mdid-browser-output.dcm"
        );
    }

    #[test]
    fn media_metadata_mode_uses_json_text_and_bounded_runtime_route() {
        assert_eq!(
            InputMode::from_select_value("media-metadata-json"),
            InputMode::MediaMetadataJson
        );
        assert_eq!(
            InputMode::MediaMetadataJson.select_value(),
            "media-metadata-json"
        );
        assert_eq!(
            InputMode::MediaMetadataJson.endpoint(),
            "/media/conservative/deidentify"
        );
        assert_eq!(
            InputMode::MediaMetadataJson.browser_file_read_mode(),
            BrowserFileReadMode::Text
        );
        assert_eq!(
            InputMode::from_file_name("metadata.JSON"),
            Some(InputMode::MediaMetadataJson)
        );
    }

    #[test]
    fn media_metadata_mode_builds_runtime_request_without_field_policies() {
        let request = build_submit_request(
            InputMode::MediaMetadataJson,
            r#"{"artifact_label":"local-media-metadata.json","format":"image","metadata":[{"key":"PatientName","value":"Jane Patient"}],"ocr_or_visual_review_required":true}"#,
            "local-media-metadata.json",
            "",
        )
        .unwrap();

        assert_eq!(request.endpoint, "/media/conservative/deidentify");
        let body: serde_json::Value = serde_json::from_str(&request.body_json).unwrap();
        assert_eq!(body["artifact_label"], "local-media-metadata.json");
        assert_eq!(body["format"], "image");
        assert_eq!(body["metadata"][0]["key"], "PatientName");
        assert_eq!(body["metadata"][0]["value"], "Jane Patient");
        assert_eq!(body["ocr_or_visual_review_required"], true);
        assert!(body.get("policies").is_none());
        assert!(body.get("field_policies").is_none());
    }

    #[test]
    fn media_metadata_mode_rejects_non_object_payload() {
        let error = build_submit_request(InputMode::MediaMetadataJson, "[]", "metadata.json", "")
            .unwrap_err();
        assert_eq!(error, "Media metadata JSON must be a JSON object accepted by the local media review runtime route.");
    }

    #[test]
    fn media_metadata_json_request_rejects_media_byte_payload_fields_phi_safely() {
        let raw_media_value = "SmFuZSBQYXRpZW50IE1STi0wMDE=";

        for field in ["media_bytes_base64", "image_bytes", "file_bytes", "base64"] {
            let payload = serde_json::json!({
                "artifact_label": "patient-jane-image.png",
                "format": "image",
                "metadata": [{"key": "CameraOwner", "value": "Jane Patient"}],
                field: raw_media_value,
            })
            .to_string();

            let error =
                build_submit_request(InputMode::MediaMetadataJson, &payload, "metadata.json", "")
                    .unwrap_err();
            assert_eq!(
                error,
                "metadata-only media review does not accept media bytes"
            );
            assert!(!error.contains(raw_media_value));
            assert!(!error.contains("Jane Patient"));
        }
    }

    #[test]
    fn parse_media_review_success_renders_phi_safe_summary_and_redacted_queue() {
        let response = parse_runtime_success(
            InputMode::MediaMetadataJson,
            &json!({
                "summary": {
                    "total_items": 1,
                    "metadata_only_items": 0,
                    "visual_review_required_items": 1,
                    "unsupported_items": 0,
                    "review_required_candidates": 1
                },
                "review_queue": [
                    {
                        "field_ref": {"artifact_label": "patient-jane-face.jpg", "metadata_key": "PatientName"},
                        "format": "image",
                        "phi_type": "metadata_identifier",
                        "source_value": "Jane Patient",
                        "confidence": 0.92,
                        "status": "ocr_or_visual_review_required"
                    }
                ],
                "rewritten_media_bytes_base64": "PHI_MEDIA_BYTES"
            })
            .to_string(),
        )
        .unwrap();

        assert_eq!(
            response.rewritten_output,
            "Media rewrite/export unavailable: runtime returned metadata-only conservative review."
        );
        assert!(response.summary.contains("total_items: 1"));
        assert!(response.summary.contains("visual_review_required_items: 1"));
        assert!(response.summary.contains("review_required_candidates: 1"));
        assert!(response
            .summary
            .contains("rewritten_media_bytes_base64: null"));
        assert!(!response.summary.contains("PHI_MEDIA_BYTES"));
        assert_eq!(
            response.review_queue,
            "- PatientName / image / metadata_identifier / confidence 0.92 / value: <redacted>"
        );
        assert!(!response.review_queue.contains("Jane Patient"));
    }

    #[test]
    fn media_metadata_mode_discloses_no_ocr_or_rewrite_claims() {
        let copy = InputMode::MediaMetadataJson.disclosure_copy().unwrap();

        assert!(copy.contains("metadata-only"));
        assert!(copy.contains("does not perform OCR"));
        assert!(copy.contains("visual redaction"));
        assert_eq!(
            BrowserFlowState {
                input_mode: InputMode::MediaMetadataJson,
                ..BrowserFlowState::default()
            }
            .suggested_export_file_name(),
            "mdid-browser-media-review-report.json"
        );
    }

    #[test]
    fn file_import_read_mode_matches_input_mode() {
        assert_eq!(
            InputMode::CsvText.browser_file_read_mode(),
            BrowserFileReadMode::Text
        );
        assert_eq!(
            InputMode::XlsxBase64.browser_file_read_mode(),
            BrowserFileReadMode::DataUrlBase64
        );
        assert_eq!(
            InputMode::PdfBase64.browser_file_read_mode(),
            BrowserFileReadMode::DataUrlBase64
        );
    }

    #[test]
    fn file_import_payload_from_data_url_strips_base64_prefix() {
        assert_eq!(
            file_import_payload_from_data_url(
                "data:application/vnd.openxmlformats-officedocument.spreadsheetml.sheet;base64,UEsDBA=="
            ),
            "UEsDBA=="
        );
        assert_eq!(
            file_import_payload_from_data_url("data:application/pdf;base64,JVBERi0x"),
            "JVBERi0x"
        );
        assert_eq!(
            file_import_payload_from_data_url("already-base64"),
            "already-base64"
        );
    }

    #[test]
    fn browser_import_size_bound_rejects_oversized_files_without_phi() {
        let error = validate_browser_import_size(MAX_BROWSER_IMPORT_BYTES + 1).unwrap_err();

        assert_eq!(
            error,
            "Browser import file is too large for the bounded local browser flow."
        );
        assert!(validate_browser_import_size(MAX_BROWSER_IMPORT_BYTES).is_ok());
    }

    #[test]
    fn import_export_copy_discloses_bounded_browser_file_limits() {
        assert!(BROWSER_FILE_IMPORT_COPY.contains("CSV files load as text"));
        assert!(BROWSER_FILE_IMPORT_COPY.contains("media metadata JSON files also load as text"));
        assert!(BROWSER_FILE_IMPORT_COPY
            .contains("Media metadata JSON sends metadata only, not media bytes"));
        assert!(BROWSER_FILE_IMPORT_COPY.contains("XLSX and PDF files load as base64 payloads"));
        assert!(BROWSER_FILE_IMPORT_COPY
            .contains("does not add OCR, visual redaction, vault browsing, or auth/session"));
    }

    #[test]
    fn browser_file_import_controls_expose_media_metadata_json() {
        let source = include_str!("app.rs");

        assert!(source
            .contains("<option value=\"media-metadata-json\">\"Media metadata JSON\"</option>"));
        assert!(source.contains("accept=\".csv,.xlsx,.pdf,.dcm,.dicom,.json\""));
        assert!(source.contains("Import local CSV/XLSX/PDF/DICOM/media metadata JSON payload"));
        assert!(source.contains("validates CSV/XLSX/PDF/DICOM/media metadata JSON selection"));
        assert!(
            source.contains("JSON payloads remain metadata-only and do not include media bytes")
        );
    }

    #[test]
    fn unsupported_import_extension_error_is_honest() {
        let mut state = BrowserFlowState::default();
        state.reject_imported_file("notes.txt");

        assert_eq!(
            state.error_banner.as_deref(),
            Some("Unsupported browser import file type. Use .csv, .xlsx, .pdf, .dcm, .dicom, or .json.")
        );
        assert_eq!(state.summary, IDLE_SUMMARY);
        assert_eq!(state.review_queue, IDLE_REVIEW_QUEUE);
    }

    #[test]
    fn export_filename_is_safe_and_mode_specific() {
        let mut state = BrowserFlowState {
            imported_file_name: Some("Jane Patient.csv".to_string()),
            ..BrowserFlowState::default()
        };
        assert_eq!(
            state.suggested_export_file_name(),
            "jane-patient-deidentified.csv"
        );

        state.input_mode = InputMode::XlsxBase64;
        state.imported_file_name = Some("clinic workbook.xlsx".to_string());
        assert_eq!(
            state.suggested_export_file_name(),
            "clinic-workbook-deidentified.xlsx"
        );

        state.input_mode = InputMode::PdfBase64;
        state.imported_file_name = Some("scan.pdf".to_string());
        assert_eq!(
            state.suggested_export_file_name(),
            "scan-review-report.json"
        );

        state.input_mode = InputMode::VaultExport;
        assert_eq!(
            state.suggested_export_file_name(),
            "scan-portable-artifact.json"
        );
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
                    "review_required_candidates": 1,
                    "rewrite_status": "review_only_no_rewritten_pdf",
                    "no_rewritten_pdf": true,
                    "review_only": true
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
                "rewrite_status": "review_only_no_rewritten_pdf",
                "no_rewritten_pdf": true,
                "review_only": true,
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
        assert!(response
            .summary
            .contains("rewrite_status: review_only_no_rewritten_pdf"));
        assert!(response.summary.contains("no_rewritten_pdf: true"));
        assert!(response.summary.contains("review_only: true"));
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
            InputMode::CsvText,
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
    fn parse_runtime_error_redacts_sensitive_portable_and_vault_modes() {
        let body = json!({
            "error": {
                "code": "portable_artifact_failure",
                "message": "failed passphrase portable secret for MRN 123 Jane Patient token TOKEN-1 at /tmp/vault.json record 11111111-1111-1111-1111-111111111111"
            },
            "artifact": {"ciphertext_b64": "artifact JSON secret"},
            "audit_event": {"detail": "audit detail with vault path"}
        })
        .to_string();

        for mode in [
            InputMode::VaultAuditEvents,
            InputMode::VaultDecode,
            InputMode::VaultExport,
            InputMode::PortableArtifactInspect,
            InputMode::PortableArtifactImport,
        ] {
            let error = parse_runtime_error(mode, 422, &body);

            assert_eq!(
                error,
                "Runtime request failed. Details hidden for PHI and secret safety. Status: 422."
            );
            for forbidden in [
                "portable_artifact_failure",
                "passphrase",
                "portable secret",
                "MRN 123",
                "Jane Patient",
                "TOKEN-1",
                "/tmp/vault.json",
                "11111111-1111-1111-1111-111111111111",
                "artifact JSON secret",
                "audit detail",
            ] {
                assert!(
                    !error.contains(forbidden),
                    "leaked {forbidden} for {mode:?}"
                );
            }
        }
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
