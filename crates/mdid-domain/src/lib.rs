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
}

impl AuditEventKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditEventKind::Encode => "encode",
            AuditEventKind::Decode => "decode",
            AuditEventKind::Export => "export",
        }
    }

    pub fn is_high_risk(&self) -> bool {
        matches!(self, AuditEventKind::Decode | AuditEventKind::Export)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoatType {
    ComplianceMoat,
    DataMoat,
    #[serde(rename = "workflow_lockin")]
    WorkflowLockIn,
    EcosystemMoat,
    DistributionMoat,
    NetworkEffectAdjacent,
    BrandTrustMoat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContinueDecision {
    Continue,
    Stop,
    Pivot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ResourceBudget {
    pub max_round_minutes: u32,
    pub max_parallel_tasks: u8,
    pub max_strategy_candidates: u8,
    pub max_spec_generations: u8,
    pub max_implementation_tasks: u8,
    pub max_review_loops: u8,
}

impl ResourceBudget {
    pub fn supports_parallelism(&self) -> bool {
        self.max_parallel_tasks > 1
    }

    pub fn is_zero(&self) -> bool {
        self.max_round_minutes == 0
            && self.max_parallel_tasks == 0
            && self.max_strategy_candidates == 0
            && self.max_spec_generations == 0
            && self.max_implementation_tasks == 0
            && self.max_review_loops == 0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MarketMoatSnapshot {
    pub market_id: String,
    pub industry_segment: String,
    pub market_snapshot_at: Option<DateTime<Utc>>,
    pub moat_score: u8,
    pub moat_type: Vec<MoatType>,
    pub confidence: f32,
    pub evidence: Vec<String>,
    pub assumptions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CompetitorProfile {
    pub competitor_id: String,
    pub name: String,
    pub category: String,
    pub pricing_summary: String,
    pub feature_summary: String,
    pub talent_signal_summary: String,
    pub suspected_moat_types: Vec<MoatType>,
    pub threat_score: u8,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct LockInReport {
    pub lockin_score: u8,
    pub lockin_vectors: Vec<String>,
    pub switching_cost_strength: u8,
    pub data_gravity_strength: u8,
    pub workflow_dependency_strength: u8,
    pub portability_risk: u8,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoatStrategy {
    pub strategy_id: String,
    pub title: String,
    pub rationale: String,
    pub target_moat_type: MoatType,
    pub implementation_cost: u8,
    pub expected_moat_gain: i16,
    pub risk_level: u8,
    pub dependencies: Vec<String>,
    pub testable_hypotheses: Vec<String>,
}

impl Default for MoatStrategy {
    fn default() -> Self {
        Self {
            strategy_id: String::new(),
            title: String::new(),
            rationale: String::new(),
            target_moat_type: MoatType::ComplianceMoat,
            implementation_cost: 0,
            expected_moat_gain: 0,
            risk_level: 0,
            dependencies: Vec::new(),
            testable_hypotheses: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoatRoundSummary {
    pub round_id: Uuid,
    pub selected_strategies: Vec<String>,
    pub implemented_specs: Vec<String>,
    pub tests_passed: bool,
    pub moat_score_before: i16,
    pub moat_score_after: i16,
    pub continue_decision: ContinueDecision,
    pub stop_reason: Option<String>,
    pub pivot_reason: Option<String>,
}

impl Default for MoatRoundSummary {
    fn default() -> Self {
        Self {
            round_id: Uuid::new_v4(),
            selected_strategies: Vec::new(),
            implemented_specs: Vec::new(),
            tests_passed: false,
            moat_score_before: 0,
            moat_score_after: 0,
            continue_decision: ContinueDecision::Stop,
            stop_reason: None,
            pivot_reason: None,
        }
    }
}

impl MoatRoundSummary {
    pub fn improvement(&self) -> i16 {
        self.moat_score_after - self.moat_score_before
    }

    pub fn improved(&self) -> bool {
        self.improvement() > 0
    }
}
