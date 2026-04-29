use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SurfaceKind {
    Cli,
    Browser,
    Desktop,
}

impl SurfaceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            SurfaceKind::Cli => "cli",
            SurfaceKind::Browser => "browser",
            SurfaceKind::Desktop => "desktop",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PipelineRunState {
    Pending,
    Scheduled,
    Running,
    WaitingForReview,
    WaitingForApproval,
    Retrying,
    Completed,
    PartiallyFailed,
    Failed,
    Cancelled,
}

impl PipelineRunState {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            PipelineRunState::Completed
                | PipelineRunState::PartiallyFailed
                | PipelineRunState::Failed
                | PipelineRunState::Cancelled
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewTaskState {
    Open,
    Claimed,
    Resolved,
    Rejected,
    Expired,
}

impl ReviewTaskState {
    pub fn is_open(&self) -> bool {
        matches!(self, ReviewTaskState::Open | ReviewTaskState::Claimed)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineDefinition {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineRun {
    pub id: Uuid,
    pub pipeline_id: Uuid,
    pub state: PipelineRunState,
    pub started_by: SurfaceKind,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditEventKind {
    Encode,
    Decode,
    Export,
    Import,
}

impl AuditEventKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditEventKind::Encode => "encode",
            AuditEventKind::Decode => "decode",
            AuditEventKind::Export => "export",
            AuditEventKind::Import => "import",
        }
    }

    pub fn is_high_risk(&self) -> bool {
        matches!(
            self,
            AuditEventKind::Decode | AuditEventKind::Export | AuditEventKind::Import
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MappingScope {
    pub job_id: Uuid,
    pub artifact_id: Uuid,
    pub field_path: String,
}

impl MappingScope {
    pub fn new(job_id: Uuid, artifact_id: Uuid, field_path: String) -> Self {
        Self {
            job_id,
            artifact_id,
            field_path,
        }
    }

    pub fn scope_key(&self) -> String {
        format!("{}/{}/{}", self.job_id, self.artifact_id, self.field_path)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MappingRecord {
    pub id: Uuid,
    pub scope: MappingScope,
    pub phi_type: String,
    pub token: String,
    pub original_value: String,
    pub created_at: DateTime<Utc>,
}

impl std::fmt::Debug for MappingRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MappingRecord")
            .field("id", &self.id)
            .field("scope", &self.scope)
            .field("phi_type", &self.phi_type)
            .field("token", &self.token)
            .field("original_value", &"<redacted>")
            .field("created_at", &self.created_at)
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: Uuid,
    pub kind: AuditEventKind,
    pub actor: SurfaceKind,
    pub detail: String,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "DecodeRequestSerde")]
pub struct DecodeRequest {
    record_ids: Vec<Uuid>,
    output_target: String,
    justification: String,
    requested_by: SurfaceKind,
}

impl DecodeRequest {
    pub fn new(
        record_ids: Vec<Uuid>,
        output_target: String,
        justification: String,
        requested_by: SurfaceKind,
    ) -> Result<Self, DecodeRequestError> {
        if record_ids.is_empty() {
            return Err(DecodeRequestError::EmptyScope);
        }

        if output_target.trim().is_empty() {
            return Err(DecodeRequestError::MissingOutputTarget);
        }

        if justification.trim().is_empty() {
            return Err(DecodeRequestError::MissingJustification);
        }

        Ok(Self {
            record_ids,
            output_target,
            justification,
            requested_by,
        })
    }

    pub fn record_ids(&self) -> &[Uuid] {
        &self.record_ids
    }

    pub fn output_target(&self) -> &str {
        &self.output_target
    }

    pub fn justification(&self) -> &str {
        &self.justification
    }

    pub fn requested_by(&self) -> SurfaceKind {
        self.requested_by
    }
}

#[derive(Debug, Deserialize)]
struct DecodeRequestSerde {
    record_ids: Vec<Uuid>,
    output_target: String,
    justification: String,
    requested_by: SurfaceKind,
}

impl TryFrom<DecodeRequestSerde> for DecodeRequest {
    type Error = DecodeRequestError;

    fn try_from(value: DecodeRequestSerde) -> Result<Self, Self::Error> {
        Self::new(
            value.record_ids,
            value.output_target,
            value.justification,
            value.requested_by,
        )
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DecodeRequestError {
    #[error("decode scope must include at least one mapping record")]
    EmptyScope,
    #[error("decode output target is required")]
    MissingOutputTarget,
    #[error("decode justification is required")]
    MissingJustification,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DecodedValue {
    pub record_id: Uuid,
    pub token: String,
    pub original_value: String,
    pub scope: MappingScope,
}

impl std::fmt::Debug for DecodedValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DecodedValue")
            .field("record_id", &self.record_id)
            .field("token", &self.token)
            .field("original_value", &"<redacted>")
            .field("scope", &self.scope)
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodeResult {
    pub values: Vec<DecodedValue>,
    pub audit_event: AuditEvent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TabularFormat {
    Csv,
    Xlsx,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TabularColumn {
    pub index: usize,
    pub name: String,
    pub inferred_kind: String,
}

impl TabularColumn {
    pub fn new(index: usize, name: String, inferred_kind: String) -> Self {
        Self {
            index,
            name,
            inferred_kind,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TabularCellRef {
    pub row_index: usize,
    pub column_index: usize,
    pub header: String,
}

impl TabularCellRef {
    pub fn new(row_index: usize, column_index: usize, header: String) -> Self {
        Self {
            row_index,
            column_index,
            header,
        }
    }

    pub fn field_path(&self) -> String {
        format!(
            "rows/{}/columns/{}/{}",
            self.row_index,
            self.column_index,
            self.header.replace('/', "_")
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewDecision {
    Approved,
    Rejected,
    NeedsReview,
}

impl ReviewDecision {
    pub fn allows_encode(&self) -> bool {
        matches!(self, ReviewDecision::Approved)
    }

    pub fn requires_human_review(&self) -> bool {
        matches!(self, ReviewDecision::NeedsReview)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PdfScanStatus {
    TextLayerPresent,
    OcrRequired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdfPageRef {
    pub page_number: usize,
    pub label: String,
}

impl PdfPageRef {
    pub fn new(page_number: usize, label: String) -> Self {
        Self { page_number, label }
    }

    pub fn field_path(&self) -> String {
        format!(
            "pdf/pages/{}/{}",
            self.page_number,
            self.label.replace('/', "_")
        )
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PdfPhiCandidate {
    pub page: PdfPageRef,
    pub phi_type: String,
    pub source_text: String,
    pub confidence: u8,
    pub decision: ReviewDecision,
}

impl std::fmt::Debug for PdfPhiCandidate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PdfPhiCandidate")
            .field("page", &self.page)
            .field("phi_type", &self.phi_type)
            .field("source_text", &"<redacted>")
            .field("confidence", &self.confidence)
            .field("decision", &self.decision)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PdfExtractionSummary {
    pub total_pages: usize,
    pub text_layer_pages: usize,
    pub ocr_required_pages: usize,
    pub extracted_candidates: usize,
    pub review_required_candidates: usize,
}

impl PdfExtractionSummary {
    pub fn requires_review(&self) -> bool {
        self.ocr_required_pages > 0 || self.review_required_candidates > 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConservativeMediaFormat {
    Image,
    Video,
    Fcs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConservativeMediaScanStatus {
    MetadataOnly,
    OcrOrVisualReviewRequired,
    UnsupportedPayload,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConservativeMediaRef {
    pub artifact_label: String,
    pub metadata_key: String,
}

impl std::fmt::Debug for ConservativeMediaRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConservativeMediaRef")
            .field("artifact_label", &"<redacted>")
            .field("metadata_key", &"<redacted>")
            .finish()
    }
}

impl ConservativeMediaRef {
    pub fn field_path(&self) -> String {
        format!(
            "media:{}:{}",
            sanitize_media_path_label(&self.artifact_label),
            sanitize_media_path_label(&self.metadata_key)
        )
    }
}

fn sanitize_media_path_label(label: &str) -> String {
    label.replace(['/', '\\'], "_")
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ConservativeMediaCandidate {
    pub field_ref: ConservativeMediaRef,
    pub format: ConservativeMediaFormat,
    pub phi_type: String,
    pub source_value: String,
    pub confidence: f32,
    pub status: ConservativeMediaScanStatus,
}

impl std::fmt::Debug for ConservativeMediaCandidate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConservativeMediaCandidate")
            .field("field_ref", &self.field_ref)
            .field("format", &self.format)
            .field("phi_type", &self.phi_type)
            .field("source_value", &"<redacted>")
            .field("confidence", &self.confidence)
            .field("status", &self.status)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ConservativeMediaSummary {
    pub total_items: usize,
    pub metadata_only_items: usize,
    pub visual_review_required_items: usize,
    pub unsupported_items: usize,
    pub review_required_candidates: usize,
}

impl ConservativeMediaSummary {
    pub fn requires_review(&self) -> bool {
        self.visual_review_required_items > 0 || self.review_required_candidates > 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DicomPrivateTagPolicy {
    Keep,
    Remove,
    ReviewRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BurnedInAnnotationStatus {
    Clean,
    Suspicious,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DicomTagRef {
    pub group: u16,
    pub element: u16,
    pub keyword: String,
}

impl DicomTagRef {
    pub fn new(group: u16, element: u16, keyword: String) -> Self {
        Self {
            group,
            element,
            keyword,
        }
    }

    pub fn field_path(&self) -> String {
        format!(
            "dicom/{:04x},{:04x}/{}",
            self.group, self.element, self.keyword
        )
    }

    pub fn is_private(&self) -> bool {
        self.group % 2 == 1
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DicomPhiCandidate {
    pub tag: DicomTagRef,
    pub phi_type: String,
    pub value: String,
    pub decision: ReviewDecision,
}

impl std::fmt::Debug for DicomPhiCandidate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DicomPhiCandidate")
            .field("tag", &self.tag)
            .field("phi_type", &self.phi_type)
            .field("value", &"<redacted>")
            .field("decision", &self.decision)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DicomDeidentificationSummary {
    pub total_tags: usize,
    pub encoded_tags: usize,
    pub review_required_tags: usize,
    pub removed_private_tags: usize,
    pub remapped_uids: usize,
    pub burned_in_suspicions: usize,
}

impl DicomDeidentificationSummary {
    pub fn requires_review(&self) -> bool {
        self.review_required_tags > 0 || self.burned_in_suspicions > 0
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PhiCandidate {
    pub format: TabularFormat,
    pub column: TabularColumn,
    pub cell: TabularCellRef,
    pub phi_type: String,
    pub value: String,
    pub confidence: u8,
    pub decision: ReviewDecision,
}

impl std::fmt::Debug for PhiCandidate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PhiCandidate")
            .field("format", &self.format)
            .field("column", &self.column)
            .field("cell", &self.cell)
            .field("phi_type", &self.phi_type)
            .field("value", &"<redacted>")
            .field("confidence", &self.confidence)
            .field("decision", &self.decision)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct BatchSummary {
    pub total_rows: usize,
    pub encoded_cells: usize,
    pub review_required_cells: usize,
    pub failed_rows: usize,
}

impl BatchSummary {
    pub fn is_partial_failure(&self) -> bool {
        self.failed_rows > 0
    }
}
