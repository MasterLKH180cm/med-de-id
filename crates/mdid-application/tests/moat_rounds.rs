use mdid_application::{
    build_moat_spec_handoff_ids, evaluate_moat_round, select_top_strategies,
    MoatImprovementThreshold,
};
use mdid_domain::{
    CompetitorProfile, ContinueDecision, LockInReport, MarketMoatSnapshot, MoatStrategy, MoatType,
};
use uuid::Uuid;

#[test]
fn select_top_strategies_chooses_the_highest_expected_gain_within_budget() {
    let selected = select_top_strategies(
        vec![
            MoatStrategy {
                strategy_id: "a".into(),
                title: "A".into(),
                target_moat_type: MoatType::ComplianceMoat,
                implementation_cost: 2,
                expected_moat_gain: 5,
                ..MoatStrategy::default()
            },
            MoatStrategy {
                strategy_id: "c".into(),
                title: "C".into(),
                target_moat_type: MoatType::WorkflowLockIn,
                implementation_cost: 3,
                expected_moat_gain: 8,
                ..MoatStrategy::default()
            },
            MoatStrategy {
                strategy_id: "b".into(),
                title: "B".into(),
                target_moat_type: MoatType::WorkflowLockIn,
                implementation_cost: 1,
                expected_moat_gain: 8,
                ..MoatStrategy::default()
            },
        ],
        2,
    );

    assert_eq!(selected.len(), 2);
    assert_eq!(selected[0].strategy_id, "b");
    assert_eq!(selected[1].strategy_id, "c");
}

#[test]
fn build_moat_spec_handoff_ids_uses_selected_order_and_spec_budget() {
    let spec_ids = build_moat_spec_handoff_ids(
        &[
            MoatStrategy {
                strategy_id: "workflow-audit".into(),
                ..MoatStrategy::default()
            },
            MoatStrategy {
                strategy_id: "compliance-ledger".into(),
                ..MoatStrategy::default()
            },
            MoatStrategy {
                strategy_id: "vault-portability".into(),
                ..MoatStrategy::default()
            },
        ],
        2,
    );

    assert_eq!(
        spec_ids,
        vec![
            "moat-spec/workflow-audit".to_string(),
            "moat-spec/compliance-ledger".to_string(),
        ]
    );
}

#[test]
fn build_moat_spec_handoff_ids_normalizes_strategy_ids_and_skips_empty_results() {
    let spec_ids = build_moat_spec_handoff_ids(
        &[
            MoatStrategy {
                strategy_id: " Workflow Audit / 2026 ".into(),
                ..MoatStrategy::default()
            },
            MoatStrategy {
                strategy_id: "***".into(),
                ..MoatStrategy::default()
            },
            MoatStrategy {
                strategy_id: "Compliance Ledger".into(),
                ..MoatStrategy::default()
            },
            MoatStrategy {
                strategy_id: "   ".into(),
                ..MoatStrategy::default()
            },
        ],
        4,
    );

    assert_eq!(
        spec_ids,
        vec![
            "moat-spec/workflow-audit-2026".to_string(),
            "moat-spec/compliance-ledger".to_string(),
        ]
    );
}

#[test]
fn evaluate_moat_round_stops_when_improvement_is_below_threshold() {
    let summary = evaluate_moat_round(
        Uuid::nil(),
        &MarketMoatSnapshot {
            moat_score: 50,
            ..MarketMoatSnapshot::default()
        },
        &CompetitorProfile {
            threat_score: 60,
            ..CompetitorProfile::default()
        },
        &LockInReport {
            lockin_score: 52,
            workflow_dependency_strength: 55,
            ..LockInReport::default()
        },
        &[],
        0,
        true,
        MoatImprovementThreshold(3),
    );

    assert_eq!(summary.continue_decision, ContinueDecision::Stop);
    assert_eq!(summary.moat_score_before, 72);
    assert_eq!(summary.moat_score_after, 72);
    assert_eq!(
        summary.stop_reason.as_deref(),
        Some("moat improvement below threshold")
    );
}

#[test]
fn evaluate_moat_round_continues_when_tests_pass_and_score_improves() {
    let summary = evaluate_moat_round(
        Uuid::nil(),
        &MarketMoatSnapshot {
            moat_score: 40,
            ..MarketMoatSnapshot::default()
        },
        &CompetitorProfile {
            threat_score: 35,
            ..CompetitorProfile::default()
        },
        &LockInReport {
            lockin_score: 60,
            workflow_dependency_strength: 70,
            ..LockInReport::default()
        },
        &[MoatStrategy {
            strategy_id: "workflow-audit".into(),
            title: "Workflow audit moat".into(),
            expected_moat_gain: 7,
            implementation_cost: 2,
            target_moat_type: MoatType::WorkflowLockIn,
            ..MoatStrategy::default()
        }],
        1,
        true,
        MoatImprovementThreshold(3),
    );

    assert_eq!(summary.continue_decision, ContinueDecision::Continue);
    assert_eq!(summary.moat_score_before, 83);
    assert_eq!(summary.moat_score_after, 90);
    assert_eq!(summary.stop_reason, None);
}

#[test]
fn evaluate_moat_round_populates_implemented_specs_from_selected_strategies() {
    let summary = evaluate_moat_round(
        Uuid::nil(),
        &MarketMoatSnapshot {
            moat_score: 40,
            ..MarketMoatSnapshot::default()
        },
        &CompetitorProfile {
            threat_score: 35,
            ..CompetitorProfile::default()
        },
        &LockInReport {
            lockin_score: 60,
            workflow_dependency_strength: 70,
            ..LockInReport::default()
        },
        &[
            MoatStrategy {
                strategy_id: "workflow-audit".into(),
                title: "Workflow audit moat".into(),
                expected_moat_gain: 7,
                implementation_cost: 2,
                target_moat_type: MoatType::WorkflowLockIn,
                ..MoatStrategy::default()
            },
            MoatStrategy {
                strategy_id: "compliance-ledger".into(),
                title: "Compliance ledger moat".into(),
                expected_moat_gain: 5,
                implementation_cost: 1,
                target_moat_type: MoatType::ComplianceMoat,
                ..MoatStrategy::default()
            },
        ],
        1,
        true,
        MoatImprovementThreshold(3),
    );

    assert_eq!(
        summary.implemented_specs,
        vec!["moat-spec/workflow-audit".to_string()]
    );
}

#[test]
fn evaluate_moat_round_stops_for_test_failures_with_test_failure_reason() {
    let summary = evaluate_moat_round(
        Uuid::nil(),
        &MarketMoatSnapshot {
            moat_score: 40,
            ..MarketMoatSnapshot::default()
        },
        &CompetitorProfile {
            threat_score: 35,
            ..CompetitorProfile::default()
        },
        &LockInReport {
            lockin_score: 60,
            workflow_dependency_strength: 70,
            ..LockInReport::default()
        },
        &[MoatStrategy {
            strategy_id: "workflow-audit".into(),
            title: "Workflow audit moat".into(),
            expected_moat_gain: 7,
            implementation_cost: 2,
            target_moat_type: MoatType::WorkflowLockIn,
            ..MoatStrategy::default()
        }],
        1,
        false,
        MoatImprovementThreshold(3),
    );

    assert_eq!(summary.continue_decision, ContinueDecision::Stop);
    assert_eq!(summary.moat_score_before, 83);
    assert_eq!(summary.moat_score_after, 83);
    assert_eq!(summary.stop_reason.as_deref(), Some("tests failed"));
}
