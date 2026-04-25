use mdid_domain::{
    CompetitorProfile, ContinueDecision, LockInReport, MarketMoatSnapshot, MoatStrategy, MoatType,
    ResourceBudget,
};
use mdid_runtime::moat::{run_bounded_round, MoatRoundInput};

#[test]
fn bounded_round_returns_continue_when_gain_exceeds_threshold() {
    let report = run_bounded_round(MoatRoundInput {
        market: MarketMoatSnapshot {
            market_id: "healthcare-deid".into(),
            moat_score: 45,
            ..MarketMoatSnapshot::default()
        },
        competitor: CompetitorProfile {
            competitor_id: "comp-1".into(),
            threat_score: 30,
            ..CompetitorProfile::default()
        },
        lock_in: LockInReport {
            lockin_score: 60,
            workflow_dependency_strength: 72,
            ..LockInReport::default()
        },
        strategies: vec![MoatStrategy {
            strategy_id: "workflow-audit".into(),
            title: "Workflow audit moat".into(),
            target_moat_type: MoatType::WorkflowLockIn,
            implementation_cost: 2,
            expected_moat_gain: 8,
            ..MoatStrategy::default()
        }],
        budget: ResourceBudget {
            max_round_minutes: 30,
            max_parallel_tasks: 3,
            max_strategy_candidates: 2,
            max_spec_generations: 1,
            max_implementation_tasks: 1,
            max_review_loops: 1,
        },
        improvement_threshold: 3,
        tests_passed: true,
    });

    assert_eq!(report.summary.continue_decision, ContinueDecision::Continue);
    assert_eq!(
        report.executed_tasks,
        vec![
            "market_scan".to_string(),
            "competitor_analysis".to_string(),
            "lockin_analysis".to_string(),
            "strategy_selection".to_string(),
            "spec_plan_handoff".to_string(),
            "implementation_gate".to_string(),
            "evaluation".to_string(),
        ]
    );
    assert!(report.stop_reason.is_none());
}

#[test]
fn bounded_round_stops_when_budget_disallows_strategy_work() {
    let report = run_bounded_round(MoatRoundInput {
        market: MarketMoatSnapshot::default(),
        competitor: CompetitorProfile::default(),
        lock_in: LockInReport::default(),
        strategies: vec![MoatStrategy {
            strategy_id: "data-room".into(),
            title: "Data moat".into(),
            target_moat_type: MoatType::DataMoat,
            implementation_cost: 1,
            expected_moat_gain: 4,
            ..MoatStrategy::default()
        }],
        budget: ResourceBudget {
            max_round_minutes: 10,
            max_parallel_tasks: 1,
            max_strategy_candidates: 0,
            max_spec_generations: 0,
            max_implementation_tasks: 0,
            max_review_loops: 0,
        },
        improvement_threshold: 2,
        tests_passed: true,
    });

    assert_eq!(report.summary.continue_decision, ContinueDecision::Stop);
    assert_eq!(
        report.executed_tasks,
        vec![
            "market_scan".to_string(),
            "competitor_analysis".to_string(),
            "lockin_analysis".to_string(),
        ]
    );
    assert_eq!(
        report.stop_reason.as_deref(),
        Some("strategy budget exhausted")
    );
}

#[test]
fn bounded_round_stops_when_spec_or_implementation_budget_is_zero() {
    let report = run_bounded_round(MoatRoundInput {
        market: MarketMoatSnapshot {
            moat_score: 50,
            ..MarketMoatSnapshot::default()
        },
        competitor: CompetitorProfile {
            threat_score: 20,
            ..CompetitorProfile::default()
        },
        lock_in: LockInReport {
            lockin_score: 55,
            ..LockInReport::default()
        },
        strategies: vec![MoatStrategy {
            strategy_id: "compliance-playbook".into(),
            title: "Compliance playbook moat".into(),
            target_moat_type: MoatType::ComplianceMoat,
            implementation_cost: 2,
            expected_moat_gain: 6,
            ..MoatStrategy::default()
        }],
        budget: ResourceBudget {
            max_round_minutes: 20,
            max_parallel_tasks: 2,
            max_strategy_candidates: 2,
            max_spec_generations: 1,
            max_implementation_tasks: 0,
            max_review_loops: 1,
        },
        improvement_threshold: 3,
        tests_passed: true,
    });

    assert_eq!(report.summary.continue_decision, ContinueDecision::Stop);
    assert_eq!(
        report.executed_tasks,
        vec![
            "market_scan".to_string(),
            "competitor_analysis".to_string(),
            "lockin_analysis".to_string(),
            "strategy_selection".to_string(),
        ]
    );
    assert_eq!(
        report.stop_reason.as_deref(),
        Some("spec or implementation budget exhausted")
    );
}
