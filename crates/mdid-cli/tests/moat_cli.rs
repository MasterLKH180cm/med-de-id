use mdid_runtime::moat_history::LocalMoatHistoryStore;
use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

const USAGE: &str = "usage: mdid-cli [status | moat round [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false] [--history-path PATH] | moat control-plane [--history-path PATH] [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false] | moat history --history-path PATH | moat decision-log --history-path PATH [--role planner|coder|reviewer] [--contains TEXT] | moat assignments --history-path PATH [--role planner|coder|reviewer] [--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation] [--node-id NODE_ID] [--title-contains TEXT] [--spec-ref SPEC_REF] | moat task-graph --history-path PATH [--role planner|coder|reviewer] [--state pending|ready|in_progress|completed|blocked] [--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation] [--node-id NODE_ID] [--title-contains TEXT] [--spec-ref SPEC_REF] | moat continue --history-path PATH [--improvement-threshold N] | moat schedule-next --history-path PATH [--improvement-threshold N] | moat export-specs --history-path PATH --output-dir DIR | moat export-plans --history-path PATH --output-dir DIR]";

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
fn task_graph_requires_history_path() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "task-graph"])
        .output()
        .expect("failed to run mdid-cli moat task-graph without history path");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("missing required flag: --history-path"));
    assert!(stderr.contains(USAGE));
}

#[test]
fn task_graph_rejects_unknown_role() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            "history.json",
            "--role",
            "analyst",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with unknown role");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown moat task-graph role: analyst"));
    assert!(stderr.contains(USAGE));
}

#[test]
fn task_graph_rejects_unknown_state() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            "history.json",
            "--state",
            "done",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with unknown state");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown moat task-graph state: done"));
    assert!(stderr.contains(USAGE));
}

#[test]
fn task_graph_prints_latest_persisted_graph() {
    let history_path = unique_history_path("task-graph-success");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed task graph history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "task-graph", "--history-path", history_path_arg])
        .output()
        .expect("failed to inspect task graph");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("moat task graph\n"));
    assert!(stdout
        .contains("node=planner|market_scan|Market Scan|market_scan|completed|<none>|<none>\n"));
    assert!(stdout
        .contains("node=planner|lockin_analysis|Lock-In Analysis|lock_in_analysis|completed|"));
    assert!(stdout.contains(
        "node=planner|spec_planning|Spec Planning|spec_planning|completed|strategy_generation|docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md\n"
    ));
    assert!(stdout.contains(
        "node=coder|implementation|Implementation|implementation|completed|spec_planning|<none>\n"
    ));
    assert!(stdout.contains("node=reviewer|review|Review|review|completed|implementation|<none>\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn task_graph_filters_latest_graph_by_role_and_state() {
    let history_path = unique_history_path("task-graph-filtered");
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
        .expect("failed to seed stopped task graph history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--role",
            "reviewer",
            "--state",
            "ready",
        ])
        .output()
        .expect("failed to inspect filtered task graph");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "moat task graph\n",
            "node=reviewer|review|Review|review|ready|implementation|<none>\n",
        )
    );

    cleanup_history_path(&history_path);
}

#[test]
fn task_graph_filters_latest_graph_by_node_id() {
    let history_path = unique_history_path("task-graph-node-id");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history for node-id task graph filter");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--node-id",
            "strategy_generation",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with node-id filter");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout
            .lines()
            .filter(|line| line.starts_with("node="))
            .count(),
        1
    );
    assert!(stdout
        .contains("node=planner|strategy_generation|Strategy Generation|strategy_generation|"));
    assert!(!stdout.contains("node=planner|market_scan|"));

    cleanup_history_path(&history_path);
}

#[test]
fn task_graph_node_id_filter_returns_empty_when_no_node_matches() {
    let history_path = unique_history_path("task-graph-node-id-empty");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history for empty node-id task graph filter");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--node-id",
            "not-a-persisted-node",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with unmatched node-id filter");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "moat task graph\n");

    cleanup_history_path(&history_path);
}

#[test]
fn task_graph_rejects_missing_node_id_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            "/tmp/mdid-unused-history.json",
            "--node-id",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with missing node-id value");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("missing value for --node-id\n{}\n", USAGE)
    );
}

#[test]
fn task_graph_node_id_filter_does_not_append_history() {
    let history_path = unique_history_path("task-graph-node-id-read-only");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history for node-id read-only check");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let inspect = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--node-id",
            "strategy_generation",
        ])
        .output()
        .expect("failed to inspect moat task graph by node id");
    assert!(
        inspect.status.success(),
        "{}",
        String::from_utf8_lossy(&inspect.stderr)
    );

    let history = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "history", "--history-path", history_path_arg])
        .output()
        .expect("failed to inspect moat history after node-id task graph filter");
    assert!(
        history.status.success(),
        "{}",
        String::from_utf8_lossy(&history.stderr)
    );
    assert!(String::from_utf8_lossy(&history.stdout).contains("entries=1\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn task_graph_filters_latest_graph_by_spec_ref() {
    let history_path = unique_history_path("task-graph-spec-ref");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history for spec-ref task graph filter");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--spec-ref",
            "docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with spec-ref filter");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("moat task graph\n"));
    assert_eq!(
        stdout
            .lines()
            .filter(|line| line.starts_with("node="))
            .count(),
        1
    );
    assert!(stdout.contains("node=planner|spec_planning|Spec Planning|spec_planning|completed|strategy_generation|docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md\n"));
    assert!(!stdout.contains("|<none>\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn task_graph_spec_ref_filter_returns_header_only_when_no_node_matches() {
    let history_path = unique_history_path("task-graph-spec-ref-empty");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history for empty spec-ref task graph filter");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--spec-ref",
            "<none>",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with unmatched spec-ref filter");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "moat task graph\n");

    cleanup_history_path(&history_path);
}

#[test]
fn task_graph_spec_ref_filter_conjoins_with_role_filter() {
    let history_path = unique_history_path("task-graph-spec-ref-role");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history for spec-ref plus role filter");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--role",
            "coder",
            "--spec-ref",
            "docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with spec-ref and role filters");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "moat task graph\n");

    cleanup_history_path(&history_path);
}

#[test]
fn task_graph_rejects_missing_spec_ref_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            "/tmp/mdid-unused-history.json",
            "--spec-ref",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with missing spec-ref value");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("missing value for --spec-ref\n{}\n", USAGE)
    );
}

#[test]
fn task_graph_rejects_flag_like_spec_ref_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            "/tmp/mdid-unused-history.json",
            "--spec-ref",
            "--role",
            "planner",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with flag-like spec-ref value");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("missing value for --spec-ref\n{}\n", USAGE)
    );
}

#[test]
fn task_graph_rejects_duplicate_spec_ref_filter() {
    let history_path = unique_history_path("task-graph-spec-ref-duplicate");
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--spec-ref",
            "docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md",
            "--spec-ref",
            "other",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with duplicate spec-ref filter");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("duplicate flag: --spec-ref\n{}\n", USAGE)
    );
    assert!(!history_path.exists());
}

#[test]
fn task_graph_spec_ref_filter_does_not_append_history() {
    let history_path = unique_history_path("task-graph-spec-ref-read-only");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history for spec-ref read-only check");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let inspect = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--spec-ref",
            "docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md",
        ])
        .output()
        .expect("failed to inspect moat task graph by spec ref");
    assert!(
        inspect.status.success(),
        "{}",
        String::from_utf8_lossy(&inspect.stderr)
    );

    let history = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "history", "--history-path", history_path_arg])
        .output()
        .expect("failed to inspect moat history after spec-ref task graph filter");
    assert!(
        history.status.success(),
        "{}",
        String::from_utf8_lossy(&history.stderr)
    );
    assert!(String::from_utf8_lossy(&history.stdout).contains("entries=1\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn task_graph_filters_latest_graph_by_kind() {
    let history_path = unique_history_path("task-graph-kind");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history for kind task graph filter");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--kind",
            "lock_in_analysis",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with kind filter");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("moat task graph\n"));
    assert!(stdout.contains(
        "node=planner|lockin_analysis|Lock-In Analysis|lock_in_analysis|completed|<none>|<none>\n"
    ));
    assert!(!stdout
        .contains("node=planner|strategy_generation|Strategy Generation|strategy_generation|"));

    cleanup_history_path(&history_path);
}

#[test]
fn task_graph_kind_filter_returns_empty_when_no_node_matches() {
    let history_path = unique_history_path("task-graph-kind-empty");
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
        .expect("failed to seed stopped moat history for kind filter");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--kind",
            "evaluation",
            "--state",
            "completed",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with unmatched kind filter");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "moat task graph\n");

    cleanup_history_path(&history_path);
}

#[test]
fn task_graph_kind_filter_conjoins_with_role_filter() {
    let history_path = unique_history_path("task-graph-kind-role");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history for kind plus role filter");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--role",
            "reviewer",
            "--kind",
            "strategy_generation",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with kind and role filters");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "moat task graph\n");

    cleanup_history_path(&history_path);
}

#[test]
fn task_graph_rejects_unknown_kind_filter_before_touching_history() {
    let history_path = unique_history_path("task-graph-kind-unknown");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--kind",
            "operator",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with unknown kind filter");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("unknown moat task-graph kind: operator\n{}\n", USAGE)
    );
    assert!(!history_path.exists());
}

#[test]
fn task_graph_rejects_missing_kind_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            "/tmp/mdid-unused-history.json",
            "--kind",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with missing kind value");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("missing value for --kind\n{}\n", USAGE)
    );
}

#[test]
fn task_graph_rejects_flag_like_kind_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            "/tmp/mdid-unused-history.json",
            "--kind",
            "--role",
            "planner",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with flag-like kind value");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("missing value for --kind\n{}\n", USAGE)
    );
}

#[test]
fn task_graph_rejects_duplicate_kind_filter() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            "/tmp/mdid-unused-history.json",
            "--kind",
            "market_scan",
            "--kind",
            "review",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with duplicate kind filter");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("duplicate flag: --kind\n{}\n", USAGE)
    );
}

#[test]
fn task_graph_kind_filter_does_not_append_history() {
    let history_path = unique_history_path("task-graph-kind-read-only");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history for kind read-only check");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let inspect = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--kind",
            "implementation",
        ])
        .output()
        .expect("failed to inspect moat task graph by kind");
    assert!(
        inspect.status.success(),
        "{}",
        String::from_utf8_lossy(&inspect.stderr)
    );

    let history = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "history", "--history-path", history_path_arg])
        .output()
        .expect("failed to inspect moat history after kind task graph filter");
    assert!(
        history.status.success(),
        "{}",
        String::from_utf8_lossy(&history.stderr)
    );
    assert!(String::from_utf8_lossy(&history.stdout).contains("entries=1\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn task_graph_filters_latest_graph_by_title_contains() {
    let history_path = unique_history_path("task-graph-title-contains");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history for title filter");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--title-contains",
            "Strategy",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with title filter");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout
            .lines()
            .filter(|line| line.starts_with("node="))
            .count(),
        1
    );
    assert!(stdout
        .contains("node=planner|strategy_generation|Strategy Generation|strategy_generation|"));
    assert!(!stdout.contains("node=planner|market_scan|"));

    cleanup_history_path(&history_path);
}

#[test]
fn task_graph_title_filter_returns_header_only_when_no_title_matches() {
    let history_path = unique_history_path("task-graph-title-empty");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history for empty title filter");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--title-contains",
            "not in any persisted title",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with unmatched title filter");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "moat task graph\n");

    cleanup_history_path(&history_path);
}

#[test]
fn task_graph_title_filter_is_conjunctive_with_role_filter() {
    let history_path = unique_history_path("task-graph-title-role");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history for role and title filter");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--role",
            "reviewer",
            "--title-contains",
            "Strategy",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with conjunctive filters");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "moat task graph\n");

    cleanup_history_path(&history_path);
}

#[test]
fn task_graph_rejects_missing_title_contains_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            "/tmp/mdid-unused-history.json",
            "--title-contains",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with missing title value");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("missing value for --title-contains\n{}\n", USAGE)
    );
}

#[test]
fn task_graph_rejects_flag_like_title_contains_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            "/tmp/mdid-unused-history.json",
            "--title-contains",
            "--role",
            "planner",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with flag-like title value");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("missing value for --title-contains\n{}\n", USAGE)
    );
}

#[test]
fn task_graph_rejects_duplicate_title_contains_filter() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            "/tmp/mdid-unused-history.json",
            "--title-contains",
            "Strategy",
            "--title-contains",
            "Review",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with duplicate title filter");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("duplicate flag: --title-contains\n{}\n", USAGE)
    );
}

#[test]
fn task_graph_title_filter_does_not_append_history() {
    let history_path = unique_history_path("task-graph-title-read-only");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history for title read-only check");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let inspect = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--title-contains",
            "Strategy",
        ])
        .output()
        .expect("failed to inspect moat task graph by title");
    assert!(
        inspect.status.success(),
        "{}",
        String::from_utf8_lossy(&inspect.stderr)
    );

    let history = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "history", "--history-path", history_path_arg])
        .output()
        .expect("failed to inspect moat history after title task graph filter");
    assert!(
        history.status.success(),
        "{}",
        String::from_utf8_lossy(&history.stderr)
    );
    assert!(String::from_utf8_lossy(&history.stdout).contains("entries=1\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn task_graph_rejects_missing_history_file_without_creating_it() {
    let history_path = unique_history_path("task-graph-missing");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
        ])
        .output()
        .expect("failed to run task-graph with missing history");

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed to open moat history store"));
    assert!(stderr.contains("moat history file does not exist"));
    assert!(!history_path.exists());

    cleanup_history_path(&history_path);
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
fn moat_decision_log_requires_history_path() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "decision-log"])
        .output()
        .expect("failed to run mdid-cli moat decision-log without history path");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("missing required flag: --history-path"));
    assert!(stderr.contains(USAGE));
}

#[test]
fn moat_decision_log_inspects_latest_persisted_round_without_appending() {
    let history_path = unique_history_path("decision-log");
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
        .args(["moat", "decision-log", "--history-path", history_path_arg])
        .output()
        .expect("failed to run mdid-cli moat decision-log");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("decision_log_entries=1\n"));
    assert!(stdout.contains("decision=reviewer|review approved bounded moat round|review approved bounded moat round after evaluation cleared the improvement threshold\n"));

    let store = LocalMoatHistoryStore::open_existing(&history_path).expect("history should exist");
    assert_eq!(store.summary().entry_count, 1);

    cleanup_history_path(&history_path);
}

#[test]
fn moat_decision_log_rejects_missing_history_file_without_creating_it() {
    let history_path = unique_history_path("missing-decision-log");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "decision-log",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
        ])
        .output()
        .expect("failed to run decision-log with missing history");

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed to open moat history"));
    assert!(!history_path.exists());

    cleanup_history_path(&history_path);
}

#[test]
fn moat_decision_log_filters_by_role() {
    let history_path = unique_history_path("decision-log-role-filter");
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

    let reviewer_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "decision-log",
            "--history-path",
            history_path_arg,
            "--role",
            "reviewer",
        ])
        .output()
        .expect("failed to run reviewer-filtered decision log");

    assert!(
        reviewer_output.status.success(),
        "{}",
        String::from_utf8_lossy(&reviewer_output.stderr)
    );
    let reviewer_stdout = String::from_utf8_lossy(&reviewer_output.stdout);
    assert!(reviewer_stdout.contains("decision_log_entries=1\n"));
    assert!(reviewer_stdout.contains("decision=reviewer|"));
    assert!(!reviewer_stdout.contains("decision=planner|"));
    assert!(!reviewer_stdout.contains("decision=coder|"));

    let planner_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "decision-log",
            "--history-path",
            history_path_arg,
            "--role",
            "planner",
        ])
        .output()
        .expect("failed to run planner-filtered decision log");

    assert!(
        planner_output.status.success(),
        "{}",
        String::from_utf8_lossy(&planner_output.stderr)
    );
    let planner_stdout = String::from_utf8_lossy(&planner_output.stdout);
    assert!(planner_stdout.contains("decision_log_entries=0\n"));
    assert!(!planner_stdout.contains("decision="));

    cleanup_history_path(&history_path);
}

#[test]
fn decision_log_filters_latest_decisions_by_text() {
    let history_path = unique_history_path("decision-log-contains");
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
        .args([
            "moat",
            "decision-log",
            "--history-path",
            history_path_arg,
            "--contains",
            "approved bounded",
        ])
        .output()
        .expect("failed to run text-filtered decision log");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("decision_log_entries=1\n"));
    assert!(stdout.contains("decision=reviewer|review approved bounded moat round|review approved bounded moat round after evaluation cleared the improvement threshold\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn decision_log_text_filter_returns_zero_when_no_decision_matches() {
    let history_path = unique_history_path("decision-log-contains-zero");
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
        .args([
            "moat",
            "decision-log",
            "--history-path",
            history_path_arg,
            "--contains",
            "not present in any decision",
        ])
        .output()
        .expect("failed to run text-filtered decision log");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "decision_log_entries=0\n"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn decision_log_text_filter_combines_with_role_filter() {
    let history_path = unique_history_path("decision-log-contains-role");
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
        .args([
            "moat",
            "decision-log",
            "--history-path",
            history_path_arg,
            "--role",
            "planner",
            "--contains",
            "approved bounded",
        ])
        .output()
        .expect("failed to run role-and-text-filtered decision log");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "decision_log_entries=0\n"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn decision_log_rejects_missing_contains_value() {
    let history_path = unique_history_path("decision-log-missing-contains");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "decision-log",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--contains",
        ])
        .output()
        .expect("failed to run decision log with missing contains value");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing value for --contains"));
    assert!(!history_path.exists());
}

#[test]
fn decision_log_rejects_flag_like_contains_value() {
    let history_path = unique_history_path("decision-log-flag-like-contains");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "decision-log",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--contains",
            "--role",
            "reviewer",
        ])
        .output()
        .expect("failed to run decision log with flag-like contains value");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing value for --contains"));
    assert!(!history_path.exists());
}

#[test]
fn decision_log_rejects_duplicate_contains_filter() {
    let history_path = unique_history_path("decision-log-duplicate-contains");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "decision-log",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--contains",
            "approved",
            "--contains",
            "bounded",
        ])
        .output()
        .expect("failed to run decision log with duplicate contains filter");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("duplicate flag: --contains"));
    assert!(!history_path.exists());
}

#[test]
fn decision_log_text_filter_does_not_append_history() {
    let history_path = unique_history_path("decision-log-contains-read-only");
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

    let inspect = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "decision-log",
            "--history-path",
            history_path_arg,
            "--contains",
            "approved bounded",
        ])
        .output()
        .expect("failed to inspect decision log");
    assert!(
        inspect.status.success(),
        "{}",
        String::from_utf8_lossy(&inspect.stderr)
    );

    let history = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "history", "--history-path", history_path_arg])
        .output()
        .expect("failed to inspect moat history");
    assert!(
        history.status.success(),
        "{}",
        String::from_utf8_lossy(&history.stderr)
    );
    assert!(String::from_utf8_lossy(&history.stdout).contains("entries=1\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn cli_requires_history_path_for_moat_assignments() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "assignments"])
        .output()
        .expect("failed to run mdid-cli moat assignments without history path");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("missing required flag: --history-path\n{}\n", USAGE)
    );
}

#[test]
fn cli_rejects_unknown_moat_assignments_role() {
    let history_path = unique_history_path("assignments-unknown-role");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--role",
            "operator",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with unknown role");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("unknown moat assignments role: operator\n{}\n", USAGE)
    );
    assert!(!history_path.exists());
}

#[test]
fn cli_reports_latest_moat_assignments_from_persisted_history() {
    let history_path = unique_history_path("assignments-success");
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
        .expect("failed to seed moat history");
    assert!(
        seed.status.success(),
        "stderr was: {}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "assignments", "--history-path", history_path_arg])
        .output()
        .expect("failed to run mdid-cli moat assignments");

    assert!(
        output.status.success(),
        "stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "moat assignments\n",
            "assignment_entries=1\n",
            "assignment=reviewer|review|Review|review|<none>\n",
        )
    );

    let history = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "history", "--history-path", history_path_arg])
        .output()
        .expect("failed to inspect moat history after assignments");
    assert!(
        history.status.success(),
        "stderr was: {}",
        String::from_utf8_lossy(&history.stderr)
    );
    assert!(String::from_utf8_lossy(&history.stdout).contains("entries=1\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn cli_moat_assignments_escapes_pipe_delimited_fields() {
    let history_path = unique_history_path("assignments-escaped-fields");
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
        .expect("failed to seed moat history");
    assert!(
        seed.status.success(),
        "stderr was: {}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let persisted = fs::read_to_string(&history_path).expect("history should be readable");
    let persisted = persisted
        .replace("\"node_id\": \"review\"", "\"node_id\": \"review|node\"")
        .replace("\"title\": \"Review\"", "\"title\": \"Review\\nTitle\"")
        .replace(
            "\"spec_ref\": null",
            "\"spec_ref\": \"spec|ref\\rpath\\\\tail\"",
        );
    fs::write(&history_path, persisted).expect("history should be patchable");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "assignments", "--history-path", history_path_arg])
        .output()
        .expect("failed to run mdid-cli moat assignments");

    assert!(
        output.status.success(),
        "stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "moat assignments\n",
            "assignment_entries=1\n",
            "assignment=reviewer|review\\|node|Review\\nTitle|review|spec\\|ref\\rpath\\\\tail\n",
        )
    );

    cleanup_history_path(&history_path);
}

#[test]
fn cli_filters_moat_assignments_by_role() {
    let history_path = unique_history_path("assignments-role-filter");
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
        .expect("failed to seed moat history");
    assert!(
        seed.status.success(),
        "stderr was: {}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let planner_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--role",
            "planner",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with planner filter");
    assert!(
        planner_output.status.success(),
        "stderr was: {}",
        String::from_utf8_lossy(&planner_output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&planner_output.stdout),
        concat!("moat assignments\n", "assignment_entries=0\n")
    );

    let reviewer_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--role",
            "reviewer",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with reviewer filter");
    assert!(
        reviewer_output.status.success(),
        "stderr was: {}",
        String::from_utf8_lossy(&reviewer_output.stderr)
    );
    assert!(String::from_utf8_lossy(&reviewer_output.stdout).contains("assignment_entries=1\n"));
    assert!(
        String::from_utf8_lossy(&reviewer_output.stdout).contains("assignment=reviewer|review|")
    );

    cleanup_history_path(&history_path);
}

#[test]
fn assignments_filters_latest_assignments_by_kind() {
    let history_path = unique_history_path("assignments-kind");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    seed_successful_moat_history(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--kind",
            "strategy_generation",
        ])
        .output()
        .expect("failed to inspect moat assignments by kind");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "moat assignments\nassignment_entries=1\nassignment=planner|strategy_generation|Strategy Generation|strategy_generation|<none>\n"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn assignments_kind_filter_returns_zero_when_no_assignment_matches() {
    let history_path = unique_history_path("assignments-kind-empty");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    seed_successful_moat_history(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--kind",
            "evaluation",
        ])
        .output()
        .expect("failed to inspect moat assignments by unmatched kind");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "moat assignments\nassignment_entries=0\n"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn assignments_kind_filter_is_conjunctive_with_role_filter() {
    let history_path = unique_history_path("assignments-kind-role");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    seed_successful_moat_history(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--role",
            "coder",
            "--kind",
            "strategy_generation",
        ])
        .output()
        .expect("failed to inspect moat assignments by role and kind");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "moat assignments\nassignment_entries=0\n"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn assignments_rejects_unknown_kind_without_touching_history() {
    let history_path = unique_history_path("assignments-kind-unknown");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--kind",
            "lockin_analysis",
        ])
        .output()
        .expect("failed to reject unknown moat assignments kind");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!(
            "unknown moat assignments kind: lockin_analysis\n{}\n",
            USAGE
        )
    );
    assert!(!history_path.exists());
}

#[test]
fn assignments_rejects_missing_kind_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            "/tmp/mdid-unused-history.json",
            "--kind",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with missing kind value");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("missing value for --kind\n{}\n", USAGE)
    );
}

#[test]
fn assignments_rejects_flag_like_kind_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            "/tmp/mdid-unused-history.json",
            "--kind",
            "--role",
            "planner",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with flag-like kind value");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("missing value for --kind\n{}\n", USAGE)
    );
}

#[test]
fn assignments_rejects_duplicate_kind_filter() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            "/tmp/mdid-unused-history.json",
            "--kind",
            "review",
            "--kind",
            "evaluation",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with duplicate kind filter");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("duplicate flag: --kind\n{}\n", USAGE)
    );
}

#[test]
fn assignments_kind_filter_does_not_append_history() {
    let history_path = unique_history_path("assignments-kind-read-only");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    seed_successful_moat_history(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--kind",
            "strategy_generation",
        ])
        .output()
        .expect("failed to inspect moat assignments by kind");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let history_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "history", "--history-path", history_path_arg])
        .output()
        .expect("failed to inspect moat history after kind assignments filter");
    assert!(
        history_output.status.success(),
        "{}",
        String::from_utf8_lossy(&history_output.stderr)
    );
    assert!(String::from_utf8_lossy(&history_output.stdout).contains("entries=1\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn assignments_filters_latest_assignments_by_node_id() {
    let history_path = unique_history_path("assignments-node-id");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--strategy-candidates",
            "0",
            "--history-path",
            history_path_arg,
        ])
        .output()
        .expect("failed to seed moat history");
    assert!(
        seed.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--node-id",
            "strategy_generation",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with node-id filter");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("moat assignments\n"));
    assert!(stdout.contains("assignment_entries=1\n"));
    assert!(stdout.contains(
        "assignment=planner|strategy_generation|Strategy Generation|strategy_generation|<none>\n"
    ));
    assert!(!stdout.contains("assignment=planner|market_scan|"));

    cleanup_history_path(&history_path);
}

#[test]
fn assignments_node_id_filter_returns_zero_when_no_assignment_matches() {
    let history_path = unique_history_path("assignments-node-id-empty");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--strategy-candidates",
            "0",
            "--history-path",
            history_path_arg,
        ])
        .output()
        .expect("failed to seed moat history");
    assert!(
        seed.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--node-id",
            "missing_node",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with unmatched node-id filter");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "moat assignments\nassignment_entries=0\n"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn assignments_node_id_filter_combines_with_role_filter() {
    let history_path = unique_history_path("assignments-node-id-role");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--strategy-candidates",
            "0",
            "--history-path",
            history_path_arg,
        ])
        .output()
        .expect("failed to seed moat history");
    assert!(
        seed.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--role",
            "reviewer",
            "--node-id",
            "strategy_generation",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with role and node-id filters");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "moat assignments\nassignment_entries=0\n"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn assignments_title_filter_matches_latest_assignment_titles() {
    let history_path = unique_history_path("assignments-title-filter");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    seed_successful_moat_history(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--title-contains",
            "Strategy",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with title filter");

    assert!(
        output.status.success(),
        "stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "moat assignments\n",
            "assignment_entries=1\n",
            "assignment=planner|strategy_generation|Strategy Generation|strategy_generation|<none>\n",
        )
    );

    cleanup_history_path(&history_path);
}

#[test]
fn assignments_title_filter_returns_zero_entries_when_no_title_matches() {
    let history_path = unique_history_path("assignments-title-filter-empty");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    seed_successful_moat_history(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--title-contains",
            "No Such Assignment Title",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with unmatched title filter");

    assert!(
        output.status.success(),
        "stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "moat assignments\nassignment_entries=0\n"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn assignments_title_filter_combines_with_role_filter() {
    let history_path = unique_history_path("assignments-title-role-filter");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    seed_successful_moat_history(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--role",
            "coder",
            "--title-contains",
            "Strategy",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with role and title filters");

    assert!(
        output.status.success(),
        "stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "moat assignments\nassignment_entries=0\n"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn assignments_title_filter_requires_a_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            "/tmp/mdid-unused-history.json",
            "--title-contains",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with missing title filter value");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("missing value for --title-contains\n{}\n", USAGE)
    );
}

#[test]
fn assignments_title_filter_rejects_duplicate_flag() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            "/tmp/mdid-unused-history.json",
            "--title-contains",
            "Strategy",
            "--title-contains",
            "Review",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with duplicate title filter");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("duplicate flag: --title-contains\n{}\n", USAGE)
    );
}

#[test]
fn assignments_title_filter_does_not_append_history() {
    let history_path = unique_history_path("assignments-title-filter-read-only");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    seed_successful_moat_history(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--title-contains",
            "Strategy",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with title filter read-only check");
    assert!(
        output.status.success(),
        "stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let history = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "history", "--history-path", history_path_arg])
        .output()
        .expect("failed to inspect moat history after assignments title filter");
    assert!(
        history.status.success(),
        "stderr was: {}",
        String::from_utf8_lossy(&history.stderr)
    );
    assert!(String::from_utf8_lossy(&history.stdout).contains("entries=1\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn spec_ref_filters_latest_assignments_by_exact_persisted_spec_ref() {
    let history_path = unique_history_path("assignments-spec-ref");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    seed_spec_ref_moat_history(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--spec-ref",
            "moat-spec/workflow-audit",
        ])
        .output()
        .expect("failed to inspect moat assignments by spec ref");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "moat assignments\nassignment_entries=1\nassignment=planner|strategy_generation|Strategy Generation|strategy_generation|moat-spec/workflow-audit\n"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn spec_ref_filter_returns_zero_when_no_assignment_matches() {
    let history_path = unique_history_path("assignments-spec-ref-empty");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    seed_successful_moat_history(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--spec-ref",
            "moat-spec/does-not-exist",
        ])
        .output()
        .expect("failed to inspect moat assignments by unmatched spec ref");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "moat assignments\nassignment_entries=0\n"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn spec_ref_filter_is_conjunctive_with_role_filter() {
    let history_path = unique_history_path("assignments-spec-ref-role");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    seed_spec_ref_moat_history(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--role",
            "coder",
            "--spec-ref",
            "moat-spec/workflow-audit",
        ])
        .output()
        .expect("failed to inspect moat assignments by role and spec ref");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "moat assignments\nassignment_entries=0\n"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn spec_ref_filter_rejects_missing_value() {
    let history_path = unique_history_path("assignments-spec-ref-missing-value");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--spec-ref",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with missing spec-ref value");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing value for --spec-ref"));
    assert!(!history_path.exists());
}

#[test]
fn spec_ref_filter_rejects_flag_like_missing_value() {
    let history_path = unique_history_path("assignments-spec-ref-flag-like");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--spec-ref",
            "--role",
            "planner",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with flag-like spec-ref value");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing value for --spec-ref"));
    assert!(!history_path.exists());
}

#[test]
fn spec_ref_filter_rejects_duplicate_flag() {
    let history_path = unique_history_path("assignments-spec-ref-duplicate");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--spec-ref",
            "moat-spec/workflow-audit",
            "--spec-ref",
            "moat-spec/other",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with duplicate spec-ref filter");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("duplicate flag: --spec-ref"));
    assert!(!history_path.exists());
}

#[test]
fn spec_ref_filter_does_not_append_history() {
    let history_path = unique_history_path("assignments-spec-ref-read-only");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    seed_spec_ref_moat_history(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--spec-ref",
            "moat-spec/workflow-audit",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with spec-ref read-only check");
    assert!(
        output.status.success(),
        "stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let history = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "history", "--history-path", history_path_arg])
        .output()
        .expect("failed to inspect moat history after assignments spec-ref filter");
    assert!(
        history.status.success(),
        "stderr was: {}",
        String::from_utf8_lossy(&history.stderr)
    );
    assert!(String::from_utf8_lossy(&history.stdout).contains("entries=1\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn assignments_rejects_missing_node_id_value() {
    let history_path = unique_history_path("assignments-node-id-missing-value");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--node-id",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with missing node-id value");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing value for --node-id"));
    assert!(!history_path.exists());
}

#[test]
fn assignments_rejects_duplicate_node_id_filter() {
    let history_path = unique_history_path("assignments-node-id-duplicate");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--node-id",
            "strategy_generation",
            "--node-id",
            "market_scan",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with duplicate node-id filter");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("duplicate flag: --node-id"));
    assert!(!history_path.exists());
}

#[test]
fn assignments_node_id_filter_does_not_append_history() {
    let history_path = unique_history_path("assignments-node-id-read-only");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--strategy-candidates",
            "0",
            "--history-path",
            history_path_arg,
        ])
        .output()
        .expect("failed to seed moat history");
    assert!(
        seed.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--node-id",
            "strategy_generation",
        ])
        .output()
        .expect("failed to inspect moat assignments by node id");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let history_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "history", "--history-path", history_path_arg])
        .output()
        .expect("failed to inspect moat history after node-id assignments filter");
    assert!(
        history_output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&history_output.stderr)
    );
    assert!(String::from_utf8_lossy(&history_output.stdout).contains("entries=1\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn cli_moat_assignments_requires_existing_history_without_creating_file() {
    let history_path = unique_history_path("assignments-missing-history");
    assert!(!history_path.exists());

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with missing history");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("moat history file does not exist:"));
    assert!(!history_path.exists());
}

#[test]
fn cli_moat_assignments_rejects_empty_history() {
    let history_path = unique_history_path("assignments-empty-history");
    let store =
        LocalMoatHistoryStore::open(&history_path).expect("empty history should be created");
    assert!(store.entries().is_empty());

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with empty history");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("moat history is empty; run `mdid-cli moat round --history-path <path>` first"));

    cleanup_history_path(&history_path);
}

#[test]
fn moat_decision_log_rejects_unknown_role_filter() {
    let history_path = unique_history_path("decision-log-unknown-role");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "decision-log",
            "--history-path",
            history_path.to_str().expect("utf8 path"),
            "--role",
            "operator",
        ])
        .output()
        .expect("failed to run decision-log with unknown role");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown moat decision-log role: operator"));

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

fn seed_successful_moat_history(history_path: &PathBuf) {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--strategy-candidates",
            "0",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
        ])
        .output()
        .expect("failed to seed successful moat history");
    assert!(
        output.status.success(),
        "stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn seed_spec_ref_moat_history(history_path: &PathBuf) {
    seed_successful_moat_history(history_path);
    let persisted = fs::read_to_string(history_path).expect("history should be readable");
    let persisted = persisted.replace(
        "\"spec_ref\": null",
        "\"spec_ref\": \"moat-spec/workflow-audit\"",
    );
    fs::write(history_path, persisted).expect("history should be patchable");
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
