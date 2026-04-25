use mdid_application::{
    build_default_moat_task_graph, build_moat_spec_handoff_ids, evaluate_moat_round,
    project_task_graph_progress, select_top_strategies, summarize_round_memory,
    MoatImprovementThreshold,
};
use mdid_domain::{
    AgentRole, CompetitorProfile, ContinueDecision, DecisionLogEntry, LockInReport,
    MarketMoatSnapshot, MoatMemorySnapshot, MoatRoundSummary, MoatStrategy, MoatTaskGraph,
    ResourceBudget,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const MARKET_SCAN: &str = "market_scan";
const COMPETITOR_ANALYSIS: &str = "competitor_analysis";
const LOCKIN_ANALYSIS: &str = "lockin_analysis";
const STRATEGY_GENERATION: &str = "strategy_generation";
const SPEC_PLANNING: &str = "spec_planning";
const IMPLEMENTATION: &str = "implementation";
const REVIEW: &str = "review";
const EVALUATION: &str = "evaluation";
const REVIEW_APPROVED_SUMMARY: &str = "review approved bounded moat round";
const REVIEW_STOPPED_SUMMARY: &str = "review stopped bounded moat round";
const PLANNING_STOPPED_SUMMARY: &str = "planning stopped before implementation";
const IMPLEMENTATION_STOPPED_SUMMARY: &str = "implementation stopped before review";
const REVIEW_RECORDED_AT: &str = "1970-01-01T00:00:00Z";

#[derive(Debug, Clone)]
pub struct MoatRoundInput {
    pub market: MarketMoatSnapshot,
    pub competitor: CompetitorProfile,
    pub lock_in: LockInReport,
    pub strategies: Vec<MoatStrategy>,
    pub budget: ResourceBudget,
    pub improvement_threshold: i16,
    pub tests_passed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoatControlPlaneReport {
    pub task_graph: MoatTaskGraph,
    pub memory: MoatMemorySnapshot,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoatRoundReport {
    pub summary: MoatRoundSummary,
    pub executed_tasks: Vec<String>,
    pub stop_reason: Option<String>,
    pub control_plane: MoatControlPlaneReport,
}

pub fn run_bounded_round(input: MoatRoundInput) -> MoatRoundReport {
    let round_id = Uuid::new_v4();
    let mut executed_tasks = vec![
        MARKET_SCAN.to_string(),
        COMPETITOR_ANALYSIS.to_string(),
        LOCKIN_ANALYSIS.to_string(),
    ];

    if input.budget.max_strategy_candidates == 0 {
        return stop_report(
            round_id,
            executed_tasks,
            Vec::new(),
            &input,
            "strategy budget exhausted",
        );
    }

    executed_tasks.push(STRATEGY_GENERATION.to_string());

    let selected_strategies = select_top_strategies(
        input.strategies.clone(),
        usize::from(input.budget.max_strategy_candidates),
    );

    if input.budget.max_spec_generations == 0 || input.budget.max_implementation_tasks == 0 {
        return stop_report(
            round_id,
            executed_tasks,
            selected_strategies,
            &input,
            "spec or implementation budget exhausted",
        );
    }

    executed_tasks.push(SPEC_PLANNING.to_string());
    executed_tasks.push(IMPLEMENTATION.to_string());

    if input.budget.max_review_loops == 0 {
        return stop_report(
            round_id,
            executed_tasks,
            selected_strategies,
            &input,
            "review budget exhausted",
        );
    }

    executed_tasks.push(REVIEW.to_string());
    executed_tasks.push(EVALUATION.to_string());

    let summary = evaluate_moat_round(
        round_id,
        &input.market,
        &input.competitor,
        &input.lock_in,
        &selected_strategies,
        usize::from(input.budget.max_spec_generations),
        input.tests_passed,
        MoatImprovementThreshold(input.improvement_threshold),
    );
    let stop_reason = summary.stop_reason.clone();

    build_report(summary, executed_tasks, stop_reason)
}

fn stop_report(
    round_id: Uuid,
    executed_tasks: Vec<String>,
    selected_strategies: Vec<MoatStrategy>,
    input: &MoatRoundInput,
    reason: &str,
) -> MoatRoundReport {
    let stop_reason = Some(reason.to_string());
    let reached_spec_planning = executed_tasks.iter().any(|task| task == SPEC_PLANNING);
    let mut summary = evaluate_moat_round(
        round_id,
        &input.market,
        &input.competitor,
        &input.lock_in,
        &[],
        0,
        input.tests_passed,
        MoatImprovementThreshold(input.improvement_threshold),
    );
    summary.selected_strategies = selected_strategies
        .iter()
        .map(|strategy| strategy.strategy_id.clone())
        .collect();
    summary.implemented_specs = if reached_spec_planning {
        build_moat_spec_handoff_ids(
            &selected_strategies,
            usize::from(input.budget.max_spec_generations),
        )
    } else {
        Vec::new()
    };
    summary.continue_decision = ContinueDecision::Stop;
    summary.stop_reason = stop_reason.clone();

    build_report(summary, executed_tasks, stop_reason)
}

fn build_report(
    summary: MoatRoundSummary,
    executed_tasks: Vec<String>,
    stop_reason: Option<String>,
) -> MoatRoundReport {
    let control_plane = build_control_plane_report(&summary, &executed_tasks);

    MoatRoundReport {
        summary,
        executed_tasks,
        stop_reason,
        control_plane,
    }
}

fn build_control_plane_report(
    summary: &MoatRoundSummary,
    executed_tasks: &[String],
) -> MoatControlPlaneReport {
    let task_graph = project_task_graph_progress(
        build_default_moat_task_graph(summary.round_id),
        executed_tasks,
    );
    let memory = summarize_round_memory(summary, vec![latest_decision(summary, executed_tasks)]);

    MoatControlPlaneReport { task_graph, memory }
}

fn latest_decision(summary: &MoatRoundSummary, executed_tasks: &[String]) -> DecisionLogEntry {
    let (author_role, decision_summary, rationale) = if executed_tasks
        .iter()
        .any(|task| task == REVIEW || task == EVALUATION)
    {
        let (decision_summary, rationale) = if summary.continue_decision
            == ContinueDecision::Continue
        {
            (
                REVIEW_APPROVED_SUMMARY,
                "review approved bounded moat round after evaluation cleared the improvement threshold",
            )
        } else {
            (
                REVIEW_STOPPED_SUMMARY,
                summary
                    .stop_reason
                    .as_deref()
                    .unwrap_or("review stopped bounded moat round"),
            )
        };

        (AgentRole::Reviewer, decision_summary, rationale)
    } else if executed_tasks.iter().any(|task| task == IMPLEMENTATION) {
        (
            AgentRole::Coder,
            IMPLEMENTATION_STOPPED_SUMMARY,
            summary
                .stop_reason
                .as_deref()
                .unwrap_or(IMPLEMENTATION_STOPPED_SUMMARY),
        )
    } else {
        (
            AgentRole::Planner,
            PLANNING_STOPPED_SUMMARY,
            summary
                .stop_reason
                .as_deref()
                .unwrap_or(PLANNING_STOPPED_SUMMARY),
        )
    };

    DecisionLogEntry {
        entry_id: Uuid::new_v4(),
        round_id: summary.round_id,
        author_role,
        summary: decision_summary.to_string(),
        rationale: rationale.to_string(),
        recorded_at: REVIEW_RECORDED_AT
            .parse()
            .expect("review decision timestamp should parse"),
    }
}
