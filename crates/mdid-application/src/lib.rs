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
    PipelineDefinition, PipelineRun, PipelineRunState, ResourceBudget, SurfaceKind, TabularColumn,
};
use mdid_vault::{LocalVaultStore, NewMappingRecord, VaultError};
use serde::{Deserialize, Serialize};
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
        .filter_map(|strategy| normalize_moat_strategy_handoff_id(&strategy.strategy_id))
        .take(max_spec_generations)
        .map(|strategy_id| format!("moat-spec/{strategy_id}"))
        .collect()
}

pub fn render_moat_spec_markdown(
    handoff_id: &str,
    summary: &MoatRoundSummary,
    selected_strategies: &[String],
) -> Result<String, String> {
    let slug = parse_safe_moat_spec_handoff_slug(handoff_id)?;
    let title = title_from_moat_spec_slug(slug);

    if !summary
        .implemented_specs
        .iter()
        .any(|spec| spec == handoff_id)
    {
        return Err(format!(
            "handoff id {handoff_id} not present in summary.implemented_specs: {:?}",
            summary.implemented_specs
        ));
    }

    let selected_strategy_ids = resolve_render_selected_strategies(summary, selected_strategies)?;
    let selected = if selected_strategy_ids.is_empty() {
        "<none>".to_string()
    } else {
        selected_strategy_ids.join(",")
    };
    let improvement_delta = summary.improvement();

    Ok(format!(
        concat!(
            "# {title} Moat Spec\n\n",
            "- handoff_id: `{handoff_id}`\n",
            "- source_round_id: `{round_id}`\n",
            "- source_selected_strategies: `{selected}`\n",
            "- moat_score_before: `{before}`\n",
            "- moat_score_after: `{after}`\n",
            "- improvement_delta: `{delta}`\n\n",
            "## Objective\n\n",
            "Ship the {slug} moat slice as a bounded engineering increment that preserves the moat gain identified by the latest round.\n\n",
            "## Required Deliverables\n\n",
            "- Persist a {slug} artifact inside the local-first med-de-id product surface.\n",
            "- Expose the artifact through a deterministic operator-facing workflow.\n",
            "- Add automated verification for the new {slug} behavior.\n\n",
            "## Acceptance Tests\n\n",
            "- `{handoff_id}` stays derivable from the selected strategy set `{selected}`.\n",
            "- Re-rendering the same round preserves handoff `{handoff_id}` and moat delta `{delta}`.\n"
        ),
        title = title,
        handoff_id = handoff_id,
        round_id = summary.round_id,
        selected = selected,
        before = summary.moat_score_before,
        after = summary.moat_score_after,
        delta = improvement_delta,
        slug = slug,
    ))
}

pub fn render_moat_plan_markdown(
    handoff_id: &str,
    summary: &MoatRoundSummary,
    selected_strategies: &[String],
) -> Result<String, String> {
    let slug = parse_safe_moat_spec_handoff_slug(handoff_id)?;
    let title = title_from_moat_spec_slug(slug);

    if !summary
        .implemented_specs
        .iter()
        .any(|spec| spec == handoff_id)
    {
        return Err(format!(
            "handoff id {handoff_id} not present in summary.implemented_specs: {:?}",
            summary.implemented_specs
        ));
    }

    let selected_strategy_ids = resolve_render_selected_strategies(summary, selected_strategies)?;
    let selected = if selected_strategy_ids.is_empty() {
        "<none>".to_string()
    } else {
        selected_strategy_ids.join(",")
    };

    Ok(format!(
        concat!(
            "# {title} Implementation Plan\n\n",
            "> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan with strict RED -> GREEN -> REFACTOR discipline.\n\n",
            "**Goal:** Ship the {slug} moat slice from handoff `{handoff_id}` as a bounded SDD/TDD implementation increment.\n\n",
            "**Architecture:** Start from the deterministic moat spec handoff generated by round `{round_id}`. Keep changes local-first, covered by tests, and limited to the product surfaces needed for `{slug}`. Selected strategies: `{selected}`.\n\n",
            "### Task 1: Persist {slug} artifact\n\n",
            "- [ ] RED: Add the smallest failing test that describes the {slug} moat behavior before writing production code.\n",
            "- [ ] Run targeted tests and confirm they fail for the expected reason: `cargo test -p mdid-application moat_rounds:: -- --nocapture`.\n",
            "- [ ] GREEN: Implement the minimal production code needed to satisfy the failing test without broad automation or background agents.\n",
            "- [ ] REFACTOR: Remove duplication while preserving deterministic behavior and local-first constraints.\n",
            "- [ ] Verify with targeted and relevant broader tests, including `cargo test -p mdid-application moat_rounds::`.\n",
            "- [ ] Commit with `git commit -m \"feat: add {slug} moat plan\"`.\n"
        ),
        title = title,
        slug = slug,
        handoff_id = handoff_id,
        round_id = summary.round_id,
        selected = selected,
    ))
}

fn parse_safe_moat_spec_handoff_slug(handoff_id: &str) -> Result<&str, String> {
    let slug = handoff_id
        .strip_prefix("moat-spec/")
        .ok_or_else(|| format!("expected moat-spec/ handoff id, got {handoff_id}"))?;

    if slug.is_empty() {
        return Err(format!("invalid moat spec handoff slug in {handoff_id}"));
    }

    let mut previous_was_hyphen = false;
    for byte in slug.bytes() {
        match byte {
            b'a'..=b'z' | b'0'..=b'9' => previous_was_hyphen = false,
            b'-' if !previous_was_hyphen => previous_was_hyphen = true,
            _ => return Err(format!("invalid moat spec handoff slug in {handoff_id}")),
        }
    }

    if slug.starts_with('-') || slug.ends_with('-') {
        return Err(format!("invalid moat spec handoff slug in {handoff_id}"));
    }

    Ok(slug)
}

fn title_from_moat_spec_slug(slug: &str) -> String {
    slug.split('-')
        .map(|segment| {
            let mut chars = segment.chars();
            let Some(first) = chars.next() else {
                return String::new();
            };

            let mut word = first.to_ascii_uppercase().to_string();
            word.push_str(chars.as_str());
            word
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn resolve_render_selected_strategies<'a>(
    summary: &'a MoatRoundSummary,
    selected_strategies: &'a [String],
) -> Result<&'a [String], String> {
    if selected_strategies.is_empty() {
        return Ok(&summary.selected_strategies);
    }

    if selected_strategies != summary.selected_strategies.as_slice() {
        return Err(format!(
            "selected strategy mismatch: summary={:?}, argument={:?}",
            summary.selected_strategies, selected_strategies
        ));
    }

    Ok(selected_strategies)
}

fn normalize_moat_strategy_handoff_id(strategy_id: &str) -> Option<String> {
    let mut normalized = String::new();
    let mut last_was_separator = false;

    for character in strategy_id.chars() {
        if character.is_ascii_alphanumeric() {
            normalized.push(character.to_ascii_lowercase());
            last_was_separator = false;
        } else if !normalized.is_empty() && !last_was_separator {
            normalized.push('-');
            last_was_separator = true;
        }
    }

    while normalized.ends_with('-') {
        normalized.pop();
    }

    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
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
                assigned_agent_id: None,
                artifacts: Vec::new(),
            },
            MoatTaskNode {
                node_id: "competitor_analysis".into(),
                title: "Competitor Analysis".into(),
                role: AgentRole::Planner,
                kind: MoatTaskNodeKind::CompetitorAnalysis,
                state: MoatTaskNodeState::Pending,
                depends_on: vec![],
                spec_ref: None,
                assigned_agent_id: None,
                artifacts: Vec::new(),
            },
            MoatTaskNode {
                node_id: "lockin_analysis".into(),
                title: "Lock-In Analysis".into(),
                role: AgentRole::Planner,
                kind: MoatTaskNodeKind::LockInAnalysis,
                state: MoatTaskNodeState::Pending,
                depends_on: vec![],
                spec_ref: None,
                assigned_agent_id: None,
                artifacts: Vec::new(),
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
                assigned_agent_id: None,
                artifacts: Vec::new(),
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
                assigned_agent_id: None,
                artifacts: Vec::new(),
            },
            MoatTaskNode {
                node_id: "implementation".into(),
                title: "Implementation".into(),
                role: AgentRole::Coder,
                kind: MoatTaskNodeKind::Implementation,
                state: MoatTaskNodeState::Pending,
                depends_on: vec!["spec_planning".into()],
                spec_ref: None,
                assigned_agent_id: None,
                artifacts: Vec::new(),
            },
            MoatTaskNode {
                node_id: "review".into(),
                title: "Review".into(),
                role: AgentRole::Reviewer,
                kind: MoatTaskNodeKind::Review,
                state: MoatTaskNodeState::Pending,
                depends_on: vec!["implementation".into()],
                spec_ref: None,
                assigned_agent_id: None,
                artifacts: Vec::new(),
            },
            MoatTaskNode {
                node_id: "evaluation".into(),
                title: "Evaluation".into(),
                role: AgentRole::Reviewer,
                kind: MoatTaskNodeKind::Evaluation,
                state: MoatTaskNodeState::Pending,
                depends_on: vec!["review".into()],
                spec_ref: None,
                assigned_agent_id: None,
                artifacts: Vec::new(),
            },
        ],
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoatAgentAssignment {
    pub role: AgentRole,
    pub node_id: String,
    pub title: String,
    pub kind: MoatTaskNodeKind,
    pub spec_ref: Option<String>,
}

pub fn project_ready_moat_agent_assignments(
    graph: &MoatTaskGraph,
    budget: &ResourceBudget,
) -> Vec<MoatAgentAssignment> {
    if budget.max_parallel_tasks == 0 {
        return Vec::new();
    }

    let ready_node_ids = graph.ready_node_ids().into_iter().collect::<BTreeSet<_>>();

    graph
        .nodes
        .iter()
        .filter(|node| ready_node_ids.contains(&node.node_id))
        .take(budget.max_parallel_tasks as usize)
        .map(|node| MoatAgentAssignment {
            role: node.role,
            node_id: node.node_id.clone(),
            title: node.title.clone(),
            kind: node.kind,
            spec_ref: node.spec_ref.clone(),
        })
        .collect()
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
