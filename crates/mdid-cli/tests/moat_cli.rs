use std::process::Command;

#[test]
fn cli_runs_moat_round_and_prints_deterministic_report() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round"])
        .output()
        .expect("failed to run mdid-cli moat round");

    assert!(
        output.status.success(),
        "expected success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "moat round complete\n",
            "continue_decision=Continue\n",
            "executed_tasks=market_scan,competitor_analysis,lockin_analysis,strategy_generation,spec_planning,implementation,review,evaluation\n",
            "moat_score_before=90\n",
            "moat_score_after=98\n",
            "stop_reason=<none>\n",
        )
    );
}

#[test]
fn cli_runs_moat_round_with_review_budget_override_and_reports_stop_reason() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--review-loops", "0"])
        .output()
        .expect("failed to run mdid-cli moat round with review override");

    assert!(
        output.status.success(),
        "expected success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "moat round complete\n",
            "continue_decision=Stop\n",
            "executed_tasks=market_scan,competitor_analysis,lockin_analysis,strategy_generation,spec_planning,implementation\n",
            "moat_score_before=90\n",
            "moat_score_after=90\n",
            "stop_reason=review budget exhausted\n",
        )
    );
}

#[test]
fn cli_runs_moat_control_plane_with_strategy_budget_override() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "control-plane", "--strategy-candidates", "0"])
        .output()
        .expect("failed to run mdid-cli moat control-plane with strategy override");

    assert!(
        output.status.success(),
        "expected success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "moat control plane snapshot\n",
            "ready_nodes=strategy_generation\n",
            "latest_decision_summary=planning stopped before implementation\n",
            "improvement_delta=0\n",
            "task_states=market_scan:completed,competitor_analysis:completed,lockin_analysis:completed,strategy_generation:ready,spec_planning:pending,implementation:pending,review:pending,evaluation:pending\n",
        )
    );
}

#[test]
fn cli_rejects_non_numeric_override_values() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--review-loops", "bogus"])
        .output()
        .expect("failed to run mdid-cli moat round with invalid override");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid value for --review-loops: bogus"));
    assert!(stderr.contains("usage: mdid-cli [status | moat round [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false] | moat control-plane [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false]]"));
}

#[test]
fn cli_reports_helpful_error_for_unknown_commands() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .arg("bogus")
        .output()
        .expect("failed to run mdid-cli bogus");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown command: bogus"));
    assert!(stderr.contains("usage: mdid-cli [status | moat round [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false] | moat control-plane [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false]]"));
}
