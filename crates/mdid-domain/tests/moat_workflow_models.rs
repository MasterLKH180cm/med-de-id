use mdid_domain::{
    CompetitorProfile, ContinueDecision, LockInReport, MarketMoatSnapshot, MoatRoundSummary,
    MoatStrategy, MoatType, ResourceBudget,
};

#[test]
fn moat_type_wire_values_are_stable() {
    assert_eq!(
        serde_json::to_string(&MoatType::ComplianceMoat).unwrap(),
        "\"compliance_moat\""
    );
    assert_eq!(
        serde_json::to_string(&MoatType::WorkflowLockIn).unwrap(),
        "\"workflow_lockin\""
    );
}

#[test]
fn resource_budget_reports_exhaustion() {
    let budget = ResourceBudget {
        max_round_minutes: 15,
        max_parallel_tasks: 4,
        max_strategy_candidates: 5,
        max_spec_generations: 2,
        max_implementation_tasks: 3,
        max_review_loops: 2,
    };

    assert!(budget.supports_parallelism());
    assert!(!budget.is_zero());
}

#[test]
fn moat_round_summary_calculates_improvement() {
    let summary = MoatRoundSummary {
        moat_score_before: 41,
        moat_score_after: 49,
        continue_decision: ContinueDecision::Continue,
        ..MoatRoundSummary::default()
    };

    assert_eq!(summary.improvement(), 8);
    assert!(summary.improved());
}

#[test]
fn market_and_lockin_models_store_core_scores() {
    let market = MarketMoatSnapshot {
        market_id: "healthcare-deid".into(),
        industry_segment: "medical de-identification".into(),
        moat_score: 62,
        moat_type: vec![MoatType::ComplianceMoat, MoatType::WorkflowLockIn],
        confidence: 0.75,
        evidence: vec!["HIPAA/GDPR burden".into()],
        assumptions: vec!["buyers value auditability".into()],
        ..MarketMoatSnapshot::default()
    };
    let lock_in = LockInReport {
        lockin_score: 58,
        switching_cost_strength: 61,
        data_gravity_strength: 44,
        workflow_dependency_strength: 72,
        evidence: vec!["review workflow embedded in operations".into()],
        ..LockInReport::default()
    };

    assert_eq!(market.moat_score, 62);
    assert_eq!(lock_in.workflow_dependency_strength, 72);
}

#[test]
fn competitor_and_strategy_models_capture_actionable_fields() {
    let competitor = CompetitorProfile {
        competitor_id: "comp-1".into(),
        name: "Acme DeID".into(),
        suspected_moat_types: vec![MoatType::DataMoat],
        threat_score: 55,
        ..CompetitorProfile::default()
    };
    let strategy = MoatStrategy {
        strategy_id: "strategy-1".into(),
        title: "Audit-driven workflow moat".into(),
        target_moat_type: MoatType::WorkflowLockIn,
        implementation_cost: 3,
        expected_moat_gain: 8,
        ..MoatStrategy::default()
    };

    assert_eq!(competitor.threat_score, 55);
    assert_eq!(strategy.expected_moat_gain, 8);
}
