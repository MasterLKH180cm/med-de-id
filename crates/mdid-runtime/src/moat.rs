use mdid_application::{evaluate_moat_round, select_top_strategies, MoatImprovementThreshold};
use mdid_domain::{
    CompetitorProfile, ContinueDecision, LockInReport, MarketMoatSnapshot, MoatRoundSummary,
    MoatStrategy, ResourceBudget,
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
    pub summary: MoatRoundSummary,
    pub executed_tasks: Vec<&'static str>,
    pub stop_reason: Option<String>,
}

pub fn run_bounded_round(input: MoatRoundInput) -> MoatRoundReport {
    let mut executed_tasks = vec![MARKET_SCAN, COMPETITOR_ANALYSIS, LOCKIN_ANALYSIS];

    if input.budget.max_strategy_candidates == 0 {
        return stop_report(
            executed_tasks,
            Vec::new(),
            &input,
            "strategy budget exhausted",
        );
    }

    executed_tasks.push(STRATEGY_SELECTION);

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

    executed_tasks.push(SPEC_PLAN_HANDOFF);
    executed_tasks.push(IMPLEMENTATION_GATE);
    executed_tasks.push(EVALUATION);

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
    executed_tasks: Vec<&'static str>,
    selected_strategies: Vec<MoatStrategy>,
    input: &MoatRoundInput,
    reason: &str,
) -> MoatRoundReport {
    let moat_score_before = ((input.market.moat_score as i16 + input.lock_in.lockin_score as i16)
        - (input.competitor.threat_score as i16 / 2))
        .max(0);
    let stop_reason = Some(reason.to_string());
    let summary = MoatRoundSummary {
        round_id: Uuid::nil(),
        selected_strategies: selected_strategies
            .iter()
            .map(|strategy| strategy.strategy_id.clone())
            .collect(),
        implemented_specs: Vec::new(),
        tests_passed: input.tests_passed,
        moat_score_before,
        moat_score_after: moat_score_before,
        continue_decision: ContinueDecision::Stop,
        stop_reason: stop_reason.clone(),
        pivot_reason: None,
    };

    MoatRoundReport {
        summary,
        executed_tasks,
        stop_reason,
    }
}
