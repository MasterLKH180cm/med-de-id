use chrono::Utc;
use mdid_adapters::{
    sanitize_output_name, CsvTabularAdapter, DicomAdapter, DicomAdapterError, DicomRewritePlan,
    DicomTagReplacement, DicomUidReplacement, ExtractedTabularData, FieldPolicy,
    TabularAdapterError,
};
use mdid_domain::{
    AgentRole, BatchSummary, BurnedInAnnotationStatus, CompetitorProfile, ContinueDecision,
    DecisionLogEntry, DicomDeidentificationSummary, DicomPhiCandidate, DicomPrivateTagPolicy,
    LockInReport, MappingScope, MarketMoatSnapshot, MoatMemorySnapshot, MoatRoundSummary,
    MoatStrategy, MoatTaskGraph, MoatTaskNode, MoatTaskNodeKind, MoatTaskNodeState, PhiCandidate,
    PipelineDefinition, PipelineRun, PipelineRunState, SurfaceKind, TabularColumn,
};
use mdid_vault::{LocalVaultStore, NewMappingRecord, VaultError};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error("pipeline not found: {0}")]
    PipelineNotFound(Uuid),
    #[error(transparent)]
    DicomAdapter(#[from] DicomAdapterError),
    #[error(transparent)]
    TabularAdapter(#[from] TabularAdapterError),
    #[error(transparent)]
    Vault(#[from] VaultError),
    #[error("csv rewrite failure: {0}")]
    Csv(#[from] csv::Error),
    #[error("io failure: {0}")]
    Io(#[from] std::io::Error),
    #[error("utf8 conversion failure: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}

#[derive(Clone, Default)]
pub struct ApplicationService {
    pipelines: Arc<Mutex<HashMap<Uuid, PipelineDefinition>>>,
}

impl ApplicationService {
    pub fn register_pipeline(&self, name: String) -> PipelineDefinition {
        let pipeline = PipelineDefinition {
            id: Uuid::new_v4(),
            name,
            created_at: Utc::now(),
        };
        self.pipelines
            .lock()
            .expect("pipelines lock poisoned")
            .insert(pipeline.id, pipeline.clone());
        pipeline
    }

    pub fn start_run(
        &self,
        pipeline_id: Uuid,
        started_by: SurfaceKind,
    ) -> Result<PipelineRun, ApplicationError> {
        let has_pipeline = self
            .pipelines
            .lock()
            .expect("pipelines lock poisoned")
            .contains_key(&pipeline_id);

        if !has_pipeline {
            return Err(ApplicationError::PipelineNotFound(pipeline_id));
        }

        Ok(PipelineRun {
            id: Uuid::new_v4(),
            pipeline_id,
            state: PipelineRunState::Pending,
            started_by,
            created_at: Utc::now(),
        })
    }
}

#[derive(Clone)]
pub struct TabularDeidentificationOutput {
    pub csv: String,
    pub summary: BatchSummary,
    pub review_queue: Vec<PhiCandidate>,
}

impl fmt::Debug for TabularDeidentificationOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TabularDeidentificationOutput")
            .field("csv", &"[REDACTED]")
            .field("summary", &self.summary)
            .field("review_queue_len", &self.review_queue.len())
            .finish()
    }
}

#[derive(Clone)]
pub struct DicomDeidentificationOutput {
    pub bytes: Vec<u8>,
    pub summary: DicomDeidentificationSummary,
    pub review_queue: Vec<DicomPhiCandidate>,
    pub sanitized_file_name: String,
}

impl fmt::Debug for DicomDeidentificationOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DicomDeidentificationOutput")
            .field("bytes", &"[REDACTED]")
            .field("summary", &self.summary)
            .field("review_queue_len", &self.review_queue.len())
            .field("sanitized_file_name", &self.sanitized_file_name)
            .finish()
    }
}

#[derive(Clone, Default)]
pub struct TabularDeidentificationService;

#[derive(Clone, Default)]
pub struct DicomDeidentificationService;

impl DicomDeidentificationService {
    pub fn deidentify_bytes(
        &self,
        bytes: &[u8],
        source_name: &str,
        private_tag_policy: DicomPrivateTagPolicy,
        vault: &mut LocalVaultStore,
        actor: SurfaceKind,
    ) -> Result<DicomDeidentificationOutput, ApplicationError> {
        let adapter = DicomAdapter::new(private_tag_policy);
        let extracted = adapter.extract(bytes, source_name)?;
        let job_id = Uuid::new_v4();
        let artifact_id = Uuid::new_v4();
        let mut summary = DicomDeidentificationSummary {
            total_tags: extracted.candidates.len(),
            removed_private_tags: if private_tag_policy == DicomPrivateTagPolicy::Remove {
                extracted.private_tags.len()
            } else {
                0
            },
            burned_in_suspicions: match extracted.burned_in_annotation {
                BurnedInAnnotationStatus::Suspicious => 1,
                BurnedInAnnotationStatus::Clean => 0,
            },
            ..DicomDeidentificationSummary::default()
        };
        let mut review_queue = Vec::new();
        let mut tag_replacements = Vec::new();

        for candidate in extracted.candidates {
            if candidate.decision.requires_human_review() {
                summary.review_required_tags += 1;
                review_queue.push(candidate);
                continue;
            }

            if !candidate.decision.allows_encode() {
                continue;
            }

            let mapping = vault.ensure_mapping(
                NewMappingRecord {
                    scope: MappingScope::new(job_id, artifact_id, candidate.tag.field_path()),
                    phi_type: DICOM_COMMON_PHI_MAPPING_TYPE.into(),
                    original_value: candidate.value.clone(),
                },
                actor,
            )?;

            tag_replacements.push(DicomTagReplacement::new(candidate.tag, mapping.token));
            summary.encoded_tags += 1;
        }

        let mut uid_replacements = Vec::new();
        for uid in adapter.extract_uid_family(bytes)? {
            let mapping = vault.ensure_mapping(
                NewMappingRecord {
                    scope: MappingScope::new(job_id, artifact_id, uid.field_path().to_string()),
                    phi_type: DICOM_UID_MAPPING_TYPE.into(),
                    original_value: uid.value.clone(),
                },
                actor,
            )?;

            uid_replacements.push(DicomUidReplacement::new(uid.tag, uid.value, mapping.token));
        }
        summary.remapped_uids = uid_replacements.len();

        Ok(DicomDeidentificationOutput {
            bytes: adapter.rewrite(
                bytes,
                &DicomRewritePlan {
                    tag_replacements,
                    uid_replacements,
                },
            )?,
            summary,
            review_queue,
            sanitized_file_name: sanitize_output_name(source_name),
        })
    }
}

impl TabularDeidentificationService {
    pub fn deidentify_csv(
        &self,
        csv: &str,
        policies: &[FieldPolicy],
        vault: &mut LocalVaultStore,
        actor: SurfaceKind,
    ) -> Result<TabularDeidentificationOutput, ApplicationError> {
        let adapter = CsvTabularAdapter::new(policies.to_vec());
        let extracted = adapter.extract(csv.as_bytes())?;
        self.deidentify_extracted(extracted, vault, actor)
    }

    pub fn deidentify_extracted(
        &self,
        extracted: ExtractedTabularData,
        vault: &mut LocalVaultStore,
        actor: SurfaceKind,
    ) -> Result<TabularDeidentificationOutput, ApplicationError> {
        let job_id = Uuid::new_v4();
        let artifact_id = Uuid::new_v4();
        let mut summary = BatchSummary {
            total_rows: extracted.rows.len(),
            ..BatchSummary::default()
        };
        let mut review_queue = Vec::new();
        let mut rewritten_rows = extracted.rows.clone();
        let mut candidates_by_row = BTreeMap::<usize, Vec<PhiCandidate>>::new();
        let mut failed_rows = BTreeSet::new();

        for candidate in extracted.candidates {
            if candidate.cell.row_index >= summary.total_rows {
                continue;
            }

            if candidate.decision.requires_human_review() {
                summary.review_required_cells += 1;
                review_queue.push(candidate);
                continue;
            }

            if !candidate.decision.allows_encode() {
                continue;
            }

            candidates_by_row
                .entry(candidate.cell.row_index)
                .or_default()
                .push(candidate);
        }

        for (row_index, row_candidates) in candidates_by_row {
            let Some(row) = rewritten_rows.get_mut(row_index) else {
                failed_rows.insert(row_index);
                continue;
            };

            if row_candidates
                .iter()
                .any(|candidate| row.get(candidate.cell.column_index).is_none())
            {
                failed_rows.insert(row_index);
                continue;
            }

            for candidate in row_candidates {
                let mapping = vault.ensure_mapping(
                    NewMappingRecord {
                        scope: MappingScope::new(job_id, artifact_id, candidate.cell.field_path()),
                        phi_type: candidate.phi_type.clone(),
                        original_value: candidate.value.clone(),
                    },
                    actor,
                )?;

                row[candidate.cell.column_index] = mapping.token;
                summary.encoded_cells += 1;
            }
        }

        summary.failed_rows = failed_rows.len();

        Ok(TabularDeidentificationOutput {
            csv: write_csv(&extracted.columns, &rewritten_rows)?,
            summary,
            review_queue,
        })
    }
}

fn write_csv(columns: &[TabularColumn], rows: &[Vec<String>]) -> Result<String, ApplicationError> {
    let mut ordered_columns = columns.iter().collect::<Vec<_>>();
    ordered_columns.sort_by_key(|column| column.index);

    let mut writer = csv::WriterBuilder::new()
        .terminator(csv::Terminator::Any(b'\n'))
        .from_writer(Vec::new());
    writer.write_record(ordered_columns.iter().map(|column| column.name.as_str()))?;

    for row in rows {
        writer.write_record(row)?;
    }

    let bytes = writer.into_inner().map_err(|err| err.into_error())?;
    Ok(String::from_utf8(bytes)?)
}

const DICOM_COMMON_PHI_MAPPING_TYPE: &str = "dicom_common_phi";
const DICOM_UID_MAPPING_TYPE: &str = "dicom_uid";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoatImprovementThreshold(pub i16);

pub fn select_top_strategies(
    mut strategies: Vec<MoatStrategy>,
    max_strategy_candidates: usize,
) -> Vec<MoatStrategy> {
    strategies.sort_by(|left, right| {
        right
            .expected_moat_gain
            .cmp(&left.expected_moat_gain)
            .then_with(|| left.implementation_cost.cmp(&right.implementation_cost))
    });
    strategies.truncate(max_strategy_candidates);
    strategies
}

pub fn build_moat_spec_handoff_ids(
    selected_strategies: &[MoatStrategy],
    max_spec_generations: usize,
) -> Vec<String> {
    selected_strategies
        .iter()
        .take(max_spec_generations)
        .map(|strategy| format!("moat-spec/{}", strategy.strategy_id))
        .collect()
}

pub fn evaluate_moat_round(
    round_id: Uuid,
    market: &MarketMoatSnapshot,
    competitor: &CompetitorProfile,
    lock_in: &LockInReport,
    selected_strategies: &[MoatStrategy],
    max_spec_generations: usize,
    tests_passed: bool,
    threshold: MoatImprovementThreshold,
) -> MoatRoundSummary {
    let moat_score_before = ((market.moat_score as i16 + lock_in.lockin_score as i16)
        - (competitor.threat_score as i16 / 2))
        .max(0);
    let strategy_gain: i16 = selected_strategies
        .iter()
        .map(|strategy| strategy.expected_moat_gain)
        .sum();
    let moat_score_after = if tests_passed {
        moat_score_before + strategy_gain
    } else {
        moat_score_before
    };
    let continue_decision = if tests_passed && (moat_score_after - moat_score_before) >= threshold.0
    {
        ContinueDecision::Continue
    } else {
        ContinueDecision::Stop
    };

    MoatRoundSummary {
        round_id,
        selected_strategies: selected_strategies
            .iter()
            .map(|strategy| strategy.strategy_id.clone())
            .collect(),
        implemented_specs: build_moat_spec_handoff_ids(selected_strategies, max_spec_generations),
        tests_passed,
        moat_score_before,
        moat_score_after,
        continue_decision,
        stop_reason: if continue_decision == ContinueDecision::Stop {
            Some(
                if tests_passed {
                    "moat improvement below threshold"
                } else {
                    "tests failed"
                }
                .into(),
            )
        } else {
            None
        },
        pivot_reason: None,
    }
}

pub fn build_default_moat_task_graph(round_id: Uuid) -> MoatTaskGraph {
    MoatTaskGraph {
        round_id,
        nodes: vec![
            MoatTaskNode {
                node_id: "market_scan".into(),
                title: "Market Scan".into(),
                role: AgentRole::Planner,
                kind: MoatTaskNodeKind::MarketScan,
                state: MoatTaskNodeState::Pending,
                depends_on: vec![],
                spec_ref: None,
            },
            MoatTaskNode {
                node_id: "competitor_analysis".into(),
                title: "Competitor Analysis".into(),
                role: AgentRole::Planner,
                kind: MoatTaskNodeKind::CompetitorAnalysis,
                state: MoatTaskNodeState::Pending,
                depends_on: vec![],
                spec_ref: None,
            },
            MoatTaskNode {
                node_id: "lockin_analysis".into(),
                title: "Lock-In Analysis".into(),
                role: AgentRole::Planner,
                kind: MoatTaskNodeKind::LockInAnalysis,
                state: MoatTaskNodeState::Pending,
                depends_on: vec![],
                spec_ref: None,
            },
            MoatTaskNode {
                node_id: "strategy_generation".into(),
                title: "Strategy Generation".into(),
                role: AgentRole::Planner,
                kind: MoatTaskNodeKind::StrategyGeneration,
                state: MoatTaskNodeState::Pending,
                depends_on: vec![
                    "market_scan".into(),
                    "competitor_analysis".into(),
                    "lockin_analysis".into(),
                ],
                spec_ref: None,
            },
            MoatTaskNode {
                node_id: "spec_planning".into(),
                title: "Spec Planning".into(),
                role: AgentRole::Planner,
                kind: MoatTaskNodeKind::SpecPlanning,
                state: MoatTaskNodeState::Pending,
                depends_on: vec!["strategy_generation".into()],
                spec_ref: Some(
                    "docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md".into(),
                ),
            },
            MoatTaskNode {
                node_id: "implementation".into(),
                title: "Implementation".into(),
                role: AgentRole::Coder,
                kind: MoatTaskNodeKind::Implementation,
                state: MoatTaskNodeState::Pending,
                depends_on: vec!["spec_planning".into()],
                spec_ref: None,
            },
            MoatTaskNode {
                node_id: "review".into(),
                title: "Review".into(),
                role: AgentRole::Reviewer,
                kind: MoatTaskNodeKind::Review,
                state: MoatTaskNodeState::Pending,
                depends_on: vec!["implementation".into()],
                spec_ref: None,
            },
            MoatTaskNode {
                node_id: "evaluation".into(),
                title: "Evaluation".into(),
                role: AgentRole::Reviewer,
                kind: MoatTaskNodeKind::Evaluation,
                state: MoatTaskNodeState::Pending,
                depends_on: vec!["review".into()],
                spec_ref: None,
            },
        ],
    }
}

pub fn project_task_graph_progress(
    mut graph: MoatTaskGraph,
    executed_tasks: &[String],
) -> MoatTaskGraph {
    let executed_tasks = executed_tasks.iter().cloned().collect::<BTreeSet<_>>();

    for node in &mut graph.nodes {
        node.state = if executed_tasks.contains(&node.node_id) {
            MoatTaskNodeState::Completed
        } else {
            MoatTaskNodeState::Pending
        };
    }

    let completed_nodes = graph
        .nodes
        .iter()
        .filter(|node| node.state == MoatTaskNodeState::Completed)
        .map(|node| node.node_id.clone())
        .collect::<BTreeSet<_>>();

    for node in &mut graph.nodes {
        if node.state == MoatTaskNodeState::Pending
            && node
                .depends_on
                .iter()
                .all(|dependency| completed_nodes.contains(dependency))
        {
            node.state = MoatTaskNodeState::Ready;
        }
    }

    graph
}

pub fn summarize_round_memory(
    summary: &MoatRoundSummary,
    decisions: Vec<DecisionLogEntry>,
) -> MoatMemorySnapshot {
    MoatMemorySnapshot {
        round_id: summary.round_id,
        latest_score: summary.moat_score_after,
        improvement_delta: summary.improvement(),
        decisions,
    }
}
