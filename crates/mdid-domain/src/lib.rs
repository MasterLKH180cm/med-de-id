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
}

impl AuditEventKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditEventKind::Encode => "encode",
            AuditEventKind::Decode => "decode",
        }
    }

    pub fn is_high_risk(&self) -> bool {
        matches!(self, AuditEventKind::Decode)
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingRecord {
    pub id: Uuid,
    pub scope: MappingScope,
    pub phi_type: String,
    pub token: String,
    pub original_value: String,
    pub created_at: DateTime<Utc>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodedValue {
    pub record_id: Uuid,
    pub token: String,
    pub original_value: String,
    pub scope: MappingScope,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodeResult {
    pub values: Vec<DecodedValue>,
    pub audit_event: AuditEvent,
}
