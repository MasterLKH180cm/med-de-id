use mdid_runtime::moat_history::LocalMoatHistoryStore;
use std::{
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

const USAGE: &str = "usage: mdid-cli [status | moat round [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false] [--history-path PATH] | moat control-plane [--history-path PATH] [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false] | moat history --history-path PATH | moat continue --history-path PATH [--improvement-threshold N] | moat schedule-next --history-path PATH [--improvement-threshold N] | moat export-specs --history-path PATH --output-dir DIR | moat export-plans --history-path PATH --output-dir DIR]";

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
            "implemented_specs=moat-spec/workflow-audit\n",
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
            "implemented_specs=moat-spec/workflow-audit\n",
            "moat_score_before=90\n",
            "moat_score_after=90\n",
            "stop_reason=review budget exhausted\n",
        )
    );
}

#[test]
fn cli_runs_moat_round_with_history_path_and_persists_report() {
    let history_path = unique_history_path("persisted-round");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
        ])
        .output()
        .expect("failed to run mdid-cli moat round with history path");

    assert!(
        output.status.success(),
        "expected success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!(
            concat!(
                "moat round complete\n",
                "continue_decision=Continue\n",
                "executed_tasks=market_scan,competitor_analysis,lockin_analysis,strategy_generation,spec_planning,implementation,review,evaluation\n",
                "implemented_specs=moat-spec/workflow-audit\n",
                "moat_score_before=90\n",
                "moat_score_after=98\n",
                "stop_reason=<none>\n",
                "history_saved_to={}\n",
            ),
            history_path.display()
        )
    );

    let store = LocalMoatHistoryStore::open(&history_path).expect("history store should open");
    let summary = store.summary();
    assert_eq!(summary.entry_count, 1);
    assert_eq!(summary.best_moat_score_after, Some(98));

    cleanup_history_path(&history_path);
}

#[test]
fn cli_reports_history_summary_for_two_persisted_rounds() {
    let history_path = unique_history_path("history-summary");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let first_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to run first mdid-cli moat round with history path");
    assert!(
        first_output.status.success(),
        "expected first round success, stderr was: {}",
        String::from_utf8_lossy(&first_output.stderr)
    );

    let second_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--review-loops",
            "0",
            "--history-path",
            history_path_arg,
        ])
        .output()
        .expect("failed to run second mdid-cli moat round with history path");
    assert!(
        second_output.status.success(),
        "expected second round success, stderr was: {}",
        String::from_utf8_lossy(&second_output.stderr)
    );

    let store = LocalMoatHistoryStore::open(&history_path).expect("history store should open");
    let summary = store.summary();
    let latest_round_id = summary
        .latest_round_id
        .clone()
        .expect("summary should expose latest round id");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "history", "--history-path", history_path_arg])
        .output()
        .expect("failed to run mdid-cli moat history");

    assert!(
        output.status.success(),
        "expected success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "moat history summary\n",
            "entries=2\n",
            "latest_round_id={latest_round_id}\n",
            "latest_continue_decision=Stop\n",
            "latest_stop_reason=review budget exhausted\n",
            "latest_decision_summary=implementation stopped before review\n",
            "latest_implemented_specs=moat-spec/workflow-audit\n",
            "latest_moat_score_after=90\n",
            "best_moat_score_after=98\n",
            "improvement_deltas=8,0\n",
        )
        .replace("{latest_round_id}", &latest_round_id)
    );

    cleanup_history_path(&history_path);
}

#[test]
fn cli_reports_continuation_gate_for_latest_successful_round() {
    let history_path = unique_history_path("continue-success");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history");
    assert!(
        seed.status.success(),
        "stderr was: {}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "continue", "--history-path", history_path_arg])
        .output()
        .expect("failed to run mdid-cli moat continue");

    assert!(
        output.status.success(),
        "stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("moat continuation gate\n"));
    assert!(String::from_utf8_lossy(&output.stdout).contains("can_continue=true\n"));
    assert!(String::from_utf8_lossy(&output.stdout)
        .contains("reason=latest round cleared continuation gate\n"));
    assert!(String::from_utf8_lossy(&output.stdout).contains("required_improvement_threshold=3\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn cli_reports_continuation_gate_for_pre_evaluation_stop_round() {
    let history_path = unique_history_path("continue-stop");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--review-loops",
            "0",
            "--history-path",
            history_path_arg,
        ])
        .output()
        .expect("failed to seed stopped moat history");
    assert!(
        seed.status.success(),
        "stderr was: {}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "continue", "--history-path", history_path_arg])
        .output()
        .expect("failed to run mdid-cli moat continue");

    assert!(
        output.status.success(),
        "stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("can_continue=false\n"));
    assert!(String::from_utf8_lossy(&output.stdout).contains("evaluation_completed=false\n"));
    assert!(String::from_utf8_lossy(&output.stdout)
        .contains("reason=latest round did not complete evaluation\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn cli_continue_applies_custom_improvement_threshold_to_gate_output() {
    let history_path = unique_history_path("continue-custom-threshold");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history");
    assert!(
        seed.status.success(),
        "stderr was: {}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "continue",
            "--history-path",
            history_path_arg,
            "--improvement-threshold",
            "9",
        ])
        .output()
        .expect("failed to run mdid-cli moat continue with custom threshold");

    assert!(
        output.status.success(),
        "stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("latest_improvement_delta=8\n"));
    assert!(stdout.contains("can_continue=false\n"));
    assert!(stdout.contains("reason=latest round improvement below threshold\n"));
    assert!(stdout.contains("required_improvement_threshold=9\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn moat_schedule_next_appends_one_round_when_gate_allows_continuation() {
    let history_path = unique_history_path("schedule-next-append");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "schedule-next", "--history-path", history_path_arg])
        .output()
        .expect("failed to schedule next moat round");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("moat schedule next\n"));
    assert!(stdout.contains("scheduled=true\n"));
    assert!(stdout.contains("reason=latest round cleared continuation gate\n"));
    assert!(stdout.contains("scheduled_round_id="));
    assert!(stdout.contains(&format!("history_path={}\n", history_path.display())));

    let store = LocalMoatHistoryStore::open_existing(&history_path).expect("history should exist");
    assert_eq!(store.summary().entry_count, 2);

    cleanup_history_path(&history_path);
}

#[test]
fn moat_schedule_next_does_not_append_when_gate_blocks_continuation() {
    let history_path = unique_history_path("schedule-next-noop");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--history-path",
            history_path_arg,
            "--tests-passed",
            "false",
        ])
        .output()
        .expect("failed to seed stopped moat history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "schedule-next", "--history-path", history_path_arg])
        .output()
        .expect("failed to inspect schedule next gate");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("moat schedule next\n"));
    assert!(stdout.contains("scheduled=false\n"));
    assert!(stdout.contains("reason=latest round tests failed\n"));
    assert!(stdout.contains("scheduled_round_id=<none>\n"));
    assert!(stdout.contains(&format!("history_path={}\n", history_path.display())));

    let store = LocalMoatHistoryStore::open_existing(&history_path).expect("history should exist");
    assert_eq!(store.summary().entry_count, 1);

    cleanup_history_path(&history_path);
}

#[test]
fn moat_schedule_next_rejects_missing_history_file_without_creating_it() {
    let history_path = unique_history_path("schedule-next-missing");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "schedule-next",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
        ])
        .output()
        .expect("failed to run schedule-next with missing history");

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed to open moat history store"));
    assert!(stderr.contains("moat history file does not exist"));
    assert!(!history_path.exists());

    cleanup_history_path(&history_path);
}

#[test]
fn cli_continue_rejects_invalid_improvement_threshold() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "continue",
            "--history-path",
            "history.json",
            "--improvement-threshold",
            "bogus",
        ])
        .output()
        .expect("failed to run mdid-cli moat continue with invalid threshold");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid value for --improvement-threshold: bogus"));
    assert!(stderr.contains(USAGE));
}

#[test]
fn cli_continue_rejects_negative_improvement_threshold() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "continue",
            "--history-path",
            "history.json",
            "--improvement-threshold",
            "-1",
        ])
        .output()
        .expect("failed to run mdid-cli moat continue with negative threshold");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid value for --improvement-threshold: -1"));
    assert!(stderr.contains(USAGE));
}

#[test]
fn cli_runs_default_moat_control_plane_and_prints_graph_snapshot() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "control-plane"])
        .output()
        .expect("failed to run mdid-cli moat control-plane");

    assert!(
        output.status.success(),
        "expected success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "moat control plane snapshot\n",
            "source=sample\n",
            "ready_nodes=<none>\n",
            "latest_decision_summary=review approved bounded moat round\n",
            "improvement_delta=8\n",
            "agent_assignments=<none>\n",
            "task_states=market_scan:completed,competitor_analysis:completed,lockin_analysis:completed,strategy_generation:completed,spec_planning:completed,implementation:completed,review:completed,evaluation:completed\n",
        )
    );
}

#[test]
fn moat_control_plane_prints_planner_assignment_for_strategy_ready_node() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "control-plane", "--strategy-candidates", "0"])
        .output()
        .expect("failed to run mdid-cli moat control-plane with strategy override");

    assert!(
        output.status.success(),
        "expected success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout)
        .contains("agent_assignments=planner:strategy_generation\n"));
}

#[test]
fn moat_control_plane_prints_reviewer_assignment_for_review_ready_node() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "control-plane", "--review-loops", "0"])
        .output()
        .expect("failed to run mdid-cli moat control-plane with review override");

    assert!(
        output.status.success(),
        "expected success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("agent_assignments=reviewer:review\n"));
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
            "source=sample\n",
            "ready_nodes=strategy_generation\n",
            "latest_decision_summary=planning stopped before implementation\n",
            "improvement_delta=0\n",
            "agent_assignments=planner:strategy_generation\n",
            "task_states=market_scan:completed,competitor_analysis:completed,lockin_analysis:completed,strategy_generation:ready,spec_planning:pending,implementation:pending,review:pending,evaluation:pending\n",
        )
    );
}

#[test]
fn cli_runs_moat_control_plane_from_latest_persisted_history_round() {
    let history_path = unique_history_path("control-plane-history");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history");
    assert!(
        seed.status.success(),
        "stderr was: {}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let store = LocalMoatHistoryStore::open_existing(&history_path).expect("history should exist");
    let latest_round_id = store
        .summary()
        .latest_round_id
        .expect("seeded history should expose latest round id");
    drop(store);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "control-plane", "--history-path", history_path_arg])
        .output()
        .expect("failed to run persisted moat control-plane inspection");

    assert!(
        output.status.success(),
        "stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!(
            concat!(
                "moat control plane snapshot\n",
                "source=history\n",
                "latest_round_id={}\n",
                "history_path={}\n",
                "ready_nodes=<none>\n",
                "latest_decision_summary=review approved bounded moat round\n",
                "improvement_delta=8\n",
                "agent_assignments=<none>\n",
                "task_states=market_scan:completed,competitor_analysis:completed,lockin_analysis:completed,strategy_generation:completed,spec_planning:completed,implementation:completed,review:completed,evaluation:completed\n",
            ),
            latest_round_id,
            history_path.display()
        )
    );

    cleanup_history_path(&history_path);
}

#[test]
fn cli_control_plane_history_requires_existing_history_file() {
    let history_path = unique_history_path("control-plane-missing-history");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "control-plane", "--history-path", history_path_arg])
        .output()
        .expect("failed to run persisted moat control-plane inspection against missing history");

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed to open moat history store"));
    assert!(stderr.contains("moat history file does not exist"));
    assert!(!history_path.exists());

    cleanup_history_path(&history_path);
}

#[test]
fn cli_control_plane_rejects_empty_existing_history_file() {
    let history_path = unique_history_path("control-plane-empty-history");
    std::fs::write(&history_path, "[]").expect("empty history file should be writable");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "control-plane", "--history-path", history_path_arg])
        .output()
        .expect("failed to run persisted moat control-plane inspection against empty history");

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr
        .contains("moat history is empty; run `mdid-cli moat round --history-path <path>` first"));

    cleanup_history_path(&history_path);
}

#[test]
fn cli_control_plane_rejects_history_path_combined_with_override_flags() {
    let history_path = unique_history_path("control-plane-mixed-flags");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "control-plane",
            "--history-path",
            history_path_arg,
            "--review-loops",
            "0",
        ])
        .output()
        .expect("failed to run mdid-cli moat control-plane with mixed history and override flags");

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("cannot combine --history-path with control-plane override flags"));
    assert!(stderr.contains(USAGE));
    assert!(!history_path.exists());

    cleanup_history_path(&history_path);
}

#[test]
fn cli_rejects_unknown_override_flags() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "bogus"])
        .output()
        .expect("failed to run mdid-cli moat round with unknown flag");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown flag: bogus"));
    assert!(stderr.contains(USAGE));
}

#[test]
fn cli_rejects_missing_override_values() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--review-loops"])
        .output()
        .expect("failed to run mdid-cli moat round with missing override value");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("missing value for --review-loops"));
    assert!(stderr.contains(USAGE));
}

#[test]
fn cli_rejects_flag_like_history_path_value_for_round_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", "--review-loops", "0"])
        .output()
        .expect("failed to run mdid-cli moat round with malformed history path");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("missing value for --history-path"));
    assert!(stderr.contains(USAGE));
}

#[test]
fn cli_round_rejects_duplicate_history_path_flags() {
    let first_history_path = unique_history_path("duplicate-round-history-first");
    let second_history_path = unique_history_path("duplicate-round-history-second");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--history-path",
            first_history_path
                .to_str()
                .expect("first history path should be utf-8"),
            "--history-path",
            second_history_path
                .to_str()
                .expect("second history path should be utf-8"),
        ])
        .output()
        .expect("failed to run mdid-cli moat round with duplicate history paths");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("duplicate flag: --history-path"));
    assert!(stderr.contains(USAGE));
    assert!(!first_history_path.exists());
    assert!(!second_history_path.exists());

    cleanup_history_path(&first_history_path);
    cleanup_history_path(&second_history_path);
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
    assert!(stderr.contains(USAGE));
}

#[test]
fn cli_requires_history_path_for_history_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "history"])
        .output()
        .expect("failed to run mdid-cli moat history without history path");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("missing required flag: --history-path"));
    assert!(stderr.contains(USAGE));
}

#[test]
fn cli_history_reports_missing_history_path_value_before_unknown_flags() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "history",
            "--history-path",
            "--review-loops",
            "0",
            "--bogus",
        ])
        .output()
        .expect("failed to run mdid-cli moat history with malformed history path");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("missing value for --history-path"));
    assert!(stderr.contains(USAGE));
}

#[test]
fn cli_history_rejects_duplicate_history_path_flags() {
    let first_history_path = unique_history_path("duplicate-history-first");
    let second_history_path = unique_history_path("duplicate-history-second");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "history",
            "--history-path",
            first_history_path
                .to_str()
                .expect("first history path should be utf-8"),
            "--history-path",
            second_history_path
                .to_str()
                .expect("second history path should be utf-8"),
        ])
        .output()
        .expect("failed to run mdid-cli moat history with duplicate history paths");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("duplicate flag: --history-path"));
    assert!(stderr.contains(USAGE));

    cleanup_history_path(&first_history_path);
    cleanup_history_path(&second_history_path);
}

#[test]
fn cli_exports_latest_handoff_specs_to_output_directory() {
    let history_path = unique_history_path("export-specs-success");
    let output_dir = unique_history_directory_path("exported-specs");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let output_dir_arg = output_dir.to_str().expect("output dir should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed history for export");
    assert!(
        seed.status.success(),
        "stderr was: {}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let latest_round_id = LocalMoatHistoryStore::open_existing(&history_path)
        .expect("history should exist")
        .entries()
        .last()
        .expect("latest entry should exist")
        .report
        .summary
        .round_id
        .to_string();

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "export-specs",
            "--history-path",
            history_path_arg,
            "--output-dir",
            output_dir_arg,
        ])
        .output()
        .expect("failed to run export-specs");

    assert!(
        output.status.success(),
        "stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "moat spec export complete\n",
            "round_id={latest_round_id}\n",
            "exported_specs=moat-spec/workflow-audit\n",
            "written_files=workflow-audit.md\n",
        )
        .replace("{latest_round_id}", &latest_round_id)
    );

    let exported = std::fs::read_to_string(output_dir.join("workflow-audit.md"))
        .expect("exported spec should exist");
    assert!(exported.contains("# Workflow Audit Moat Spec\n"));
    assert!(exported.contains("- handoff_id: `moat-spec/workflow-audit`\n"));

    cleanup_history_path(&history_path);
    cleanup_history_directory_path(&output_dir);
}

#[test]
fn cli_export_specs_rejects_latest_round_without_handoffs() {
    let history_path = unique_history_path("export-specs-empty");
    let output_dir = unique_history_directory_path("exported-specs-empty");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let output_dir_arg = output_dir.to_str().expect("output dir should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--spec-generations",
            "0",
            "--history-path",
            history_path_arg,
        ])
        .output()
        .expect("failed to seed no-handoff history");
    assert!(
        seed.status.success(),
        "stderr was: {}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "export-specs",
            "--history-path",
            history_path_arg,
            "--output-dir",
            output_dir_arg,
        ])
        .output()
        .expect("failed to run export-specs without handoffs");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("latest moat round does not contain implemented_specs handoffs"));
    assert!(!output_dir.join("workflow-audit.md").exists());

    cleanup_history_path(&history_path);
    cleanup_history_directory_path(&output_dir);
}

#[test]
fn moat_export_plans_writes_latest_handoff_plan_markdown() {
    let history_path = unique_history_path("export-plans");
    let output_dir = unique_history_path("export-plans-output");
    if output_dir.exists() {
        std::fs::remove_file(&output_dir).expect("remove placeholder path");
    }
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let output_dir_arg = output_dir.to_str().expect("output dir should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "export-plans",
            "--history-path",
            history_path_arg,
            "--output-dir",
            output_dir_arg,
        ])
        .output()
        .expect("failed to export moat plans");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("moat plan export\n"));
    assert!(stdout.contains("exported_plans=moat-spec/workflow-audit\n"));
    assert!(stdout.contains("written_files=workflow-audit-implementation-plan.md\n"));

    let markdown =
        std::fs::read_to_string(output_dir.join("workflow-audit-implementation-plan.md"))
            .expect("plan markdown should be written");
    assert!(markdown.contains("# Workflow Audit Implementation Plan"));
    assert!(markdown.contains("REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development"));

    cleanup_history_path(&history_path);
    cleanup_history_directory_path(&output_dir);
}

#[test]
fn moat_export_plans_requires_existing_history_file() {
    let history_path = unique_history_path("missing-export-plans");
    let output_dir = unique_history_path("missing-export-plans-output");
    if output_dir.exists() {
        std::fs::remove_file(&output_dir).expect("remove placeholder path");
    }
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "export-plans",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--output-dir",
            output_dir.to_str().expect("output dir should be utf-8"),
        ])
        .output()
        .expect("failed to run export-plans");

    assert!(
        !output.status.success(),
        "export-plans should fail for missing history"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed to open moat history"));

    cleanup_history_path(&history_path);
    cleanup_history_directory_path(&output_dir);
}

#[test]
fn cli_round_history_persistence_failure_does_not_print_success_report() {
    let history_path = unique_history_directory_path("round-history-write-error");
    std::fs::create_dir_all(&history_path).expect("history path directory should be creatable");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
        ])
        .output()
        .expect("failed to run mdid-cli moat round with invalid history path");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("moat round complete"));
    assert!(!stdout.contains("continue_decision="));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed to open moat history store"));

    cleanup_history_directory_path(&history_path);
}

#[test]
fn cli_history_rejects_missing_history_file_without_creating_it() {
    let history_path = unique_history_path("missing-history-summary");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "history",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
        ])
        .output()
        .expect("failed to run mdid-cli moat history with missing history path");

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed to open moat history store"));
    assert!(stderr.contains("moat history file does not exist"));
    assert!(!history_path.exists());

    cleanup_history_path(&history_path);
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
    assert!(stderr.contains(USAGE));
}

fn unique_history_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "mdid-cli-{label}-{}-{}.json",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos()
    ))
}

fn cleanup_history_path(path: &PathBuf) {
    let _ = std::fs::remove_file(path);
}

fn unique_history_directory_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "mdid-cli-{label}-dir-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos()
    ))
}

fn cleanup_history_directory_path(path: &PathBuf) {
    let _ = std::fs::remove_dir_all(path);
}
