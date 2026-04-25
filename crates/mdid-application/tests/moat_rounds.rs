use mdid_application::{
    build_moat_spec_handoff_ids, evaluate_moat_round, render_moat_spec_markdown,
    select_top_strategies, MoatImprovementThreshold,
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

#[test]
fn render_moat_spec_markdown_returns_deterministic_markdown_for_selected_handoff() {
    let summary = evaluate_moat_round(
        Uuid::nil(),
        &MarketMoatSnapshot {
            market_id: "healthcare-deid".into(),
            industry_segment: "Healthcare De-Identification".into(),
            moat_score: 40,
            ..MarketMoatSnapshot::default()
        },
        &CompetitorProfile {
            competitor_id: "comp-1".into(),
            name: "Incumbent PACS".into(),
            threat_score: 35,
            ..CompetitorProfile::default()
        },
        &LockInReport {
            lockin_score: 60,
            workflow_dependency_strength: 70,
            portability_risk: 20,
            ..LockInReport::default()
        },
        &[
            MoatStrategy {
                strategy_id: "workflow-audit".into(),
                title: "Workflow audit moat".into(),
                rationale: "Export auditable workflow evidence to raise switching costs.".into(),
                target_moat_type: MoatType::WorkflowLockIn,
                implementation_cost: 2,
                expected_moat_gain: 8,
                dependencies: vec!["dicom-runtime".into()],
                testable_hypotheses: vec![
                    "Operators complete audit export without spreadsheets".into(),
                    "Review evidence survives repeat runs".into(),
                ],
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
        2,
        true,
        MoatImprovementThreshold(3),
    );

    let markdown = render_moat_spec_markdown(
        "moat-spec/compliance-ledger",
        &summary,
        &summary.selected_strategies,
    )
    .expect("selected handoff should render");

    assert_eq!(
        markdown,
        concat!(
            "# Compliance Ledger Moat Spec\n\n",
            "- handoff_id: `moat-spec/compliance-ledger`\n",
            "- source_round_id: `00000000-0000-0000-0000-000000000000`\n",
            "- source_selected_strategies: `workflow-audit,compliance-ledger`\n",
            "- moat_score_before: `83`\n",
            "- moat_score_after: `96`\n",
            "- improvement_delta: `13`\n\n",
            "## Objective\n\n",
            "Ship the compliance-ledger moat slice as a bounded engineering increment that preserves the moat gain identified by the latest round.\n\n",
            "## Required Deliverables\n\n",
            "- Persist a compliance-ledger artifact inside the local-first med-de-id product surface.\n",
            "- Expose the artifact through a deterministic operator-facing workflow.\n",
            "- Add automated verification for the new compliance-ledger behavior.\n\n",
            "## Acceptance Tests\n\n",
            "- `moat-spec/compliance-ledger` stays derivable from the selected strategy set `workflow-audit,compliance-ledger`.\n",
            "- Re-rendering the same round preserves handoff `moat-spec/compliance-ledger` and moat delta `13`.\n"
        )
    );
}

#[test]
fn render_moat_spec_markdown_uses_summary_selected_strategies_when_argument_is_empty() {
    let summary = mdid_domain::MoatRoundSummary {
        round_id: Uuid::nil(),
        selected_strategies: vec!["workflow-audit".into()],
        implemented_specs: vec!["moat-spec/workflow-audit".into()],
        moat_score_before: 10,
        moat_score_after: 14,
        ..mdid_domain::MoatRoundSummary::default()
    };

    let markdown = render_moat_spec_markdown("moat-spec/workflow-audit", &summary, &[])
        .expect("empty argument should fall back to summary state");

    assert!(markdown.contains("source_selected_strategies: `workflow-audit`"));
}

#[test]
fn render_moat_spec_markdown_rejects_mismatched_selected_strategies() {
    let summary = mdid_domain::MoatRoundSummary {
        selected_strategies: vec!["workflow-audit".into()],
        implemented_specs: vec!["moat-spec/workflow-audit".into()],
        ..mdid_domain::MoatRoundSummary::default()
    };

    let error = render_moat_spec_markdown(
        "moat-spec/workflow-audit",
        &summary,
        &["compliance-ledger".into()],
    )
    .expect_err("mismatched selected strategies should fail");

    assert!(error.contains("selected strategy mismatch"));
}

#[test]
fn render_moat_spec_markdown_rejects_handoff_ids_not_implemented_in_summary() {
    let summary = mdid_domain::MoatRoundSummary {
        implemented_specs: vec!["moat-spec/workflow-audit".into()],
        ..mdid_domain::MoatRoundSummary::default()
    };

    let error = render_moat_spec_markdown("moat-spec/compliance-ledger", &summary, &[])
        .expect_err("handoff id outside implemented specs should fail");

    assert!(error.contains("implemented_specs"));
}

#[test]
fn render_moat_spec_markdown_rejects_non_handoff_ids() {
    let error = render_moat_spec_markdown(
        "workflow-audit",
        &mdid_domain::MoatRoundSummary::default(),
        &[],
    )
    .expect_err("invalid handoff id should fail");

    assert!(error.contains("expected moat-spec/ handoff id"));
}

#[test]
fn render_moat_spec_markdown_rejects_empty_handoff_slug() {
    let error =
        render_moat_spec_markdown("moat-spec/", &mdid_domain::MoatRoundSummary::default(), &[])
            .expect_err("empty moat spec slug should fail");

    assert!(error.contains("expected non-empty moat spec slug"));
}

#[test]
fn render_moat_plan_markdown_creates_sdd_tdd_plan_for_handoff() {
    let round_id = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000123").unwrap();
    let mut summary = sample_summary(round_id);
    summary.implemented_specs = vec!["moat-spec/workflow-audit".to_string()];
    summary.selected_strategies = vec!["workflow-audit".to_string()];

    let markdown = mdid_application::render_moat_plan_markdown(
        "moat-spec/workflow-audit",
        &summary,
        &summary.selected_strategies,
    )
    .expect("plan markdown should render");

    assert!(markdown.starts_with("# Workflow Audit Implementation Plan\n"));
    assert!(markdown.contains("REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development"));
    assert!(markdown.contains("**Goal:** Ship the workflow-audit moat slice"));
    assert!(markdown.contains("### Task 1: Persist workflow-audit artifact"));
    assert!(markdown.contains("cargo test -p mdid-application moat_rounds::"));
    assert!(markdown.contains("git commit -m \"feat: add workflow-audit moat plan\""));
}

#[test]
fn render_moat_plan_markdown_rejects_unknown_handoff() {
    let round_id = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000124").unwrap();
    let summary = sample_summary(round_id);

    let error = mdid_application::render_moat_plan_markdown(
        "moat-spec/missing",
        &summary,
        &summary.selected_strategies,
    )
    .expect_err("unknown handoff should fail");

    assert!(error.contains("handoff id moat-spec/missing not present"));
}

fn sample_summary(round_id: uuid::Uuid) -> mdid_domain::MoatRoundSummary {
    mdid_domain::MoatRoundSummary {
        round_id,
        selected_strategies: vec!["workflow-audit".to_string()],
        implemented_specs: vec!["moat-spec/workflow-audit".to_string()],
        moat_score_before: 90,
        moat_score_after: 98,
        tests_passed: true,
        ..mdid_domain::MoatRoundSummary::default()
    }
}
