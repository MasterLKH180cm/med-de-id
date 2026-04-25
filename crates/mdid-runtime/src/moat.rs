use mdid_application::{evaluate_moat_round, select_top_strategies, MoatImprovementThreshold};
use mdid_domain::{
    CompetitorProfile, ContinueDecision, LockInReport, MarketMoatSnapshot, MoatStrategy,
    ResourceBudget,
};
use uuid::Uuid;

const MARKET_SCAN: &str = "market_scan";
const COMPETITOR_ANALYSIS: &str = "competitor_analysis";
const LOCKIN_ANALYSIS: &str = "lockin_analysis";
const STRATEGY_SELECTION: &str = "strategy_selection";
const SPEC_PLAN_HANDOFF: &str = "spec_plan_handoff";
const IMPLEMENTATION_GATE: &str = "implementation_gate";
const EVALUATION: &str = "evaluation";

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

#[derive(Debug, Clone)]
pub struct MoatRoundReport {
    pub summary: mdid_domain::MoatRoundSummary,
    pub executed_tasks: Vec<String>,
    pub stop_reason: Option<String>,
}

pub fn run_bounded_round(input: MoatRoundInput) -> MoatRoundReport {
    let mut executed_tasks = vec![
        MARKET_SCAN.to_string(),
        COMPETITOR_ANALYSIS.to_string(),
        LOCKIN_ANALYSIS.to_string(),
    ];

    if input.budget.max_strategy_candidates == 0 {
        return stop_report(
            executed_tasks,
            Vec::new(),
            &input,
            "strategy budget exhausted",
        );
    }

    executed_tasks.push(STRATEGY_SELECTION.to_string());

    let selected_strategies = select_top_strategies(
        input.strategies.clone(),
        usize::from(input.budget.max_strategy_candidates),
    );

    if input.budget.max_spec_generations == 0 || input.budget.max_implementation_tasks == 0 {
        return stop_report(
            executed_tasks,
            selected_strategies,
            &input,
            "spec or implementation budget exhausted",
        );
    }

    executed_tasks.push(SPEC_PLAN_HANDOFF.to_string());
    executed_tasks.push(IMPLEMENTATION_GATE.to_string());
    executed_tasks.push(EVALUATION.to_string());

    let summary = evaluate_moat_round(
        Uuid::nil(),
        &input.market,
        &input.competitor,
        &input.lock_in,
        &selected_strategies,
        input.tests_passed,
        MoatImprovementThreshold(input.improvement_threshold),
    );
    let stop_reason = summary.stop_reason.clone();

    MoatRoundReport {
        summary,
        executed_tasks,
        stop_reason,
    }
}

fn stop_report(
    executed_tasks: Vec<String>,
    selected_strategies: Vec<MoatStrategy>,
    input: &MoatRoundInput,
    reason: &str,
) -> MoatRoundReport {
    let stop_reason = Some(reason.to_string());
    let mut summary = evaluate_moat_round(
        Uuid::nil(),
        &input.market,
        &input.competitor,
        &input.lock_in,
        &[],
        input.tests_passed,
        MoatImprovementThreshold(input.improvement_threshold),
    );
    summary.selected_strategies = selected_strategies
        .iter()
        .map(|strategy| strategy.strategy_id.clone())
        .collect();
    summary.continue_decision = ContinueDecision::Stop;
    summary.stop_reason = stop_reason.clone();

    MoatRoundReport {
        summary,
        executed_tasks,
        stop_reason,
    }
}
