use mdid_runtime::moat_history::LocalMoatHistoryStore;
use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

const USAGE: &str = "usage: mdid-cli [status | moat round [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false] [--history-path PATH] | moat control-plane [--history-path PATH] [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false] | moat history --history-path PATH [--round-id ROUND_ID] [--decision Continue|Stop|Pivot] [--contains TEXT] [--stop-reason-contains TEXT] [--min-score N] [--tests-passed true|false] [--limit N] | moat decision-log --history-path PATH [--round-id ROUND_ID] [--role planner|coder|reviewer] [--contains TEXT] [--summary-contains TEXT] [--rationale-contains TEXT] [--limit N] | moat assignments --history-path PATH [--round-id ROUND_ID] [--role planner|coder|reviewer] [--state pending|ready|in_progress|completed|blocked] [--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation] [--node-id NODE_ID] [--depends-on NODE_ID] [--no-dependencies] [--title-contains TEXT] [--spec-ref SPEC_REF] [--contains TEXT] [--limit N] | moat task-graph --history-path PATH [--round-id ROUND_ID] [--role planner|coder|reviewer] [--state pending|ready|in_progress|completed|blocked] [--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation] [--node-id NODE_ID] [--depends-on NODE_ID] [--no-dependencies] [--title-contains TEXT] [--spec-ref SPEC_REF] [--contains TEXT] [--limit N] | moat ready-tasks --history-path PATH [--round-id ROUND_ID] [--role planner|coder|reviewer] [--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation] [--limit N] | moat continue --history-path PATH [--improvement-threshold N] | moat schedule-next --history-path PATH [--improvement-threshold N] | moat export-specs --history-path PATH --output-dir DIR | moat export-plans --history-path PATH --output-dir DIR]";

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
fn cli_reports_limited_recent_moat_history_rounds() {
    let history_path = unique_history_path("history-limit");
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
        .args([
            "moat",
            "history",
            "--history-path",
            history_path_arg,
            "--limit",
            "1",
        ])
        .output()
        .expect("failed to run mdid-cli moat history with limit");

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
            "history_rounds=1\n",
            "round={latest_round_id}|Stop|90|review budget exhausted\n",
        )
        .replace("{latest_round_id}", &latest_round_id)
    );

    cleanup_history_path(&history_path);
}

#[test]
fn cli_filters_recent_moat_history_rounds_by_exact_round_id() {
    let history_path = unique_history_path("history-round-id-filter");
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
        .args([
            "moat",
            "history",
            "--history-path",
            history_path_arg,
            "--round-id",
            &latest_round_id,
        ])
        .output()
        .expect("failed to run mdid-cli moat history with round-id filter");

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
            "history_rounds=1\n",
            "round={latest_round_id}|Stop|90|review budget exhausted\n",
        )
        .replace("{latest_round_id}", &latest_round_id)
    );

    cleanup_history_path(&history_path);
}

#[test]
fn cli_reports_zero_recent_moat_history_rounds_for_unknown_round_id() {
    let history_path = unique_history_path("history-round-id-no-match");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let round_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to run mdid-cli moat round with history path");
    assert!(
        round_output.status.success(),
        "expected round success, stderr was: {}",
        String::from_utf8_lossy(&round_output.stderr)
    );

    let store = LocalMoatHistoryStore::open(&history_path).expect("history store should open");
    let summary = store.summary();
    let latest_round_id = summary
        .latest_round_id
        .clone()
        .expect("summary should expose latest round id");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "history",
            "--history-path",
            history_path_arg,
            "--round-id",
            "missing-round",
        ])
        .output()
        .expect("failed to run mdid-cli moat history with missing round-id filter");

    assert!(
        output.status.success(),
        "expected success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "moat history summary\n",
            "entries=1\n",
            "latest_round_id={latest_round_id}\n",
            "latest_continue_decision=Continue\n",
            "latest_stop_reason=<none>\n",
            "latest_decision_summary=review approved bounded moat round\n",
            "latest_implemented_specs=moat-spec/workflow-audit\n",
            "latest_moat_score_after=98\n",
            "best_moat_score_after=98\n",
            "improvement_deltas=8\n",
            "history_rounds=0\n",
        )
        .replace("{latest_round_id}", &latest_round_id)
    );

    let after_summary = LocalMoatHistoryStore::open(&history_path)
        .expect("history store should reopen")
        .summary();
    assert_eq!(
        after_summary.entry_count, 1,
        "read-only filter must not append or mutate history"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn cli_filters_recent_moat_history_rounds_by_continue_decision() {
    let history_path = unique_history_path("history-decision-filter");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let continue_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to run continuing mdid-cli moat round");
    assert!(
        continue_output.status.success(),
        "expected continuing round success, stderr was: {}",
        String::from_utf8_lossy(&continue_output.stderr)
    );

    let stop_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--review-loops",
            "0",
            "--history-path",
            history_path_arg,
        ])
        .output()
        .expect("failed to run stopping mdid-cli moat round");
    assert!(
        stop_output.status.success(),
        "expected stopping round success, stderr was: {}",
        String::from_utf8_lossy(&stop_output.stderr)
    );

    let store = LocalMoatHistoryStore::open(&history_path).expect("history store should open");
    let latest_round_id = store
        .summary()
        .latest_round_id
        .clone()
        .expect("summary should expose latest round id");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "history",
            "--history-path",
            history_path_arg,
            "--decision",
            "Stop",
            "--limit",
            "5",
        ])
        .output()
        .expect("failed to run mdid-cli moat history with decision filter");

    assert!(
        output.status.success(),
        "expected history filter success, stderr was: {}",
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
            "history_rounds=1\n",
            "round={latest_round_id}|Stop|90|review budget exhausted\n",
        )
        .replace("{latest_round_id}", &latest_round_id)
    );

    cleanup_history_path(&history_path);
}

#[test]
fn cli_filters_moat_history_rounds_by_stop_reason_text() {
    let history_path = unique_history_path("history-stop-reason-filter");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let continue_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to run continuing mdid-cli moat round");
    assert!(
        continue_output.status.success(),
        "expected continuing round success, stderr was: {}",
        String::from_utf8_lossy(&continue_output.stderr)
    );

    let stop_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--review-loops",
            "0",
            "--history-path",
            history_path_arg,
        ])
        .output()
        .expect("failed to run stopping mdid-cli moat round");
    assert!(
        stop_output.status.success(),
        "expected stopping round success, stderr was: {}",
        String::from_utf8_lossy(&stop_output.stderr)
    );

    let store = LocalMoatHistoryStore::open(&history_path).expect("history store should open");
    let latest_round_id = store
        .summary()
        .latest_round_id
        .clone()
        .expect("summary should expose latest round id");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "history",
            "--history-path",
            history_path_arg,
            "--stop-reason-contains",
            "budget",
            "--limit",
            "5",
        ])
        .output()
        .expect("failed to run mdid-cli moat history with stop reason filter");

    assert!(
        output.status.success(),
        "expected history stop reason filter success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "moat history summary\n",
            "entries=1\n",
            "latest_round_id={latest_round_id}\n",
            "latest_continue_decision=Stop\n",
            "latest_stop_reason=review budget exhausted\n",
            "latest_decision_summary=implementation stopped before review\n",
            "latest_implemented_specs=moat-spec/workflow-audit\n",
            "latest_moat_score_after=90\n",
            "best_moat_score_after=90\n",
            "improvement_deltas=0\n",
            "history_rounds=1\n",
            "round={latest_round_id}|Stop|90|review budget exhausted\n",
        )
        .replace("{latest_round_id}", &latest_round_id)
    );

    cleanup_history_path(&history_path);
}

#[test]
fn moat_history_filters_rounds_by_text_fragment() {
    let history_path = unique_history_path("history-contains-filter");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let continue_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to run continuing mdid-cli moat round");
    assert!(
        continue_output.status.success(),
        "expected continuing round success, stderr was: {}",
        String::from_utf8_lossy(&continue_output.stderr)
    );

    let stop_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--tests-passed",
            "false",
            "--history-path",
            history_path_arg,
        ])
        .output()
        .expect("failed to run stopping mdid-cli moat round");
    assert!(
        stop_output.status.success(),
        "expected stopping round success, stderr was: {}",
        String::from_utf8_lossy(&stop_output.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "history",
            "--history-path",
            history_path_arg,
            "--contains",
            "Continue",
            "--limit",
            "5",
        ])
        .output()
        .expect("failed to run mdid-cli moat history with contains filter");

    assert!(
        output.status.success(),
        "expected history contains filter success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("entries=1\n"), "stdout was: {stdout}");
    assert!(stdout.contains("Continue"), "stdout was: {stdout}");
    assert!(!stdout.contains("decision=Stop"), "stdout was: {stdout}");
    assert!(!stdout.contains("|Stop|"), "stdout was: {stdout}");

    cleanup_history_path(&history_path);
}

#[test]
fn moat_history_contains_filter_can_return_empty_summary() {
    let history_path = unique_history_path("history-contains-empty");
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
            "history",
            "--history-path",
            history_path_arg,
            "--contains",
            "text that is absent from every moat round",
        ])
        .output()
        .expect("failed to run mdid-cli moat history with absent contains filter");

    assert!(
        output.status.success(),
        "expected history contains filter success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "moat history summary\nentries=0\nlatest_round_id=none\nlatest_decision=none\n"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn cli_filters_recent_moat_history_rounds_by_min_score() {
    let history_path = unique_history_path("history-min-score");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--review-loops",
            "0",
            "--history-path",
            history_path_arg,
        ])
        .output()
        .expect("failed to run stop round");
    Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to run continue round");

    let before = fs::read_to_string(&history_path).expect("history should exist before inspection");
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "history",
            "--history-path",
            history_path_arg,
            "--min-score",
            "95",
            "--limit",
            "5",
        ])
        .output()
        .expect("failed to run mdid-cli moat history with min score");

    assert!(
        output.status.success(),
        "stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("moat history summary\nentries=1\n"),
        "stdout was: {stdout}"
    );
    assert!(
        stdout.contains("history_rounds=1\n"),
        "stdout was: {stdout}"
    );
    assert!(
        stdout.contains("|Continue|98|<none>\n"),
        "stdout was: {stdout}"
    );
    assert!(
        !stdout.contains("|Stop|90|review budget exhausted\n"),
        "stdout was: {stdout}"
    );
    assert_eq!(fs::read_to_string(&history_path).unwrap(), before);

    cleanup_history_path(&history_path);
}

#[test]
fn cli_reports_empty_moat_history_summary_when_min_score_matches_no_rounds() {
    let history_path = unique_history_path("history-min-score-empty");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to run round");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "history",
            "--history-path",
            history_path_arg,
            "--min-score",
            "101",
            "--limit",
            "5",
        ])
        .output()
        .expect("failed to run mdid-cli moat history with impossible min score");

    assert!(
        output.status.success(),
        "stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "moat history summary\nentries=0\nlatest_round_id=none\nlatest_decision=none\nhistory_rounds=0\n"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn cli_filters_recent_moat_history_rounds_by_tests_passed() {
    let history_path = unique_history_path("history-tests-passed-filter");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed_success = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed successful history round");
    assert!(
        seed_success.status.success(),
        "{}",
        String::from_utf8_lossy(&seed_success.stderr)
    );
    let seed_failed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--history-path",
            history_path_arg,
            "--tests-passed",
            "false",
        ])
        .output()
        .expect("failed to seed failed history round");
    assert!(
        seed_failed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed_failed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "history",
            "--history-path",
            history_path_arg,
            "--tests-passed",
            "false",
            "--limit",
            "5",
        ])
        .output()
        .expect("failed to inspect history by tests-passed filter");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("moat history summary\n"));
    assert!(stdout.contains("entries=1\n"));
    assert!(stdout.contains("latest_continue_decision=Stop\n"));
    assert!(stdout.contains("latest_stop_reason=tests failed\n"));
    assert!(stdout.contains("latest_moat_score_after=90\n"));
    assert!(stdout.contains("history_rounds=1\n"));
    assert!(stdout.contains("|Stop|90|tests failed\n"));
    assert!(!stdout.contains("|Continue|98|<none>\n"));

    let verify = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "history",
            "--history-path",
            history_path_arg,
            "--limit",
            "5",
        ])
        .output()
        .expect("failed to verify history was not mutated");
    assert!(
        verify.status.success(),
        "{}",
        String::from_utf8_lossy(&verify.stderr)
    );
    assert!(String::from_utf8_lossy(&verify.stdout).contains("history_rounds=2\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn cli_reports_empty_moat_history_summary_when_tests_passed_filter_matches_no_rounds() {
    let history_path = unique_history_path("history-tests-passed-empty");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed successful history round");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "history",
            "--history-path",
            history_path_arg,
            "--tests-passed",
            "false",
            "--limit",
            "5",
        ])
        .output()
        .expect("failed to inspect history by unmatched tests-passed filter");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "moat history summary\nentries=0\nlatest_round_id=none\nlatest_decision=none\nhistory_rounds=0\n"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn cli_history_rejects_duplicate_tests_passed_filter() {
    let history_path = unique_history_path("history-tests-passed-duplicate");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "history",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--tests-passed",
            "true",
            "--tests-passed",
            "false",
        ])
        .output()
        .expect("failed to run mdid-cli moat history with duplicate tests-passed filter");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("duplicate flag: --tests-passed"));
    assert!(!history_path.exists());
}

#[test]
fn cli_history_rejects_duplicate_min_score() {
    let missing_path = unique_history_path("history-duplicate-min-score");
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "history",
            "--history-path",
            missing_path.to_str().expect("history path should be utf-8"),
            "--min-score",
            "90",
            "--min-score",
            "95",
        ])
        .output()
        .expect("failed to run mdid-cli moat history with duplicate min score");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("duplicate flag: --min-score\n{}\n", USAGE)
    );
    assert!(!missing_path.exists());
}

#[test]
fn cli_history_rejects_invalid_min_score() {
    let missing_path = unique_history_path("history-invalid-min-score");
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "history",
            "--history-path",
            missing_path.to_str().expect("history path should be utf-8"),
            "--min-score",
            "not-a-number",
        ])
        .output()
        .expect("failed to run mdid-cli moat history with invalid min score");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!(
            "invalid value for --min-score: expected non-negative integer, got not-a-number\n{}\n",
            USAGE
        )
    );
    assert!(!missing_path.exists());
}

#[test]
fn moat_history_contains_requires_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "history",
            "--history-path",
            "history.jsonl",
            "--contains",
        ])
        .output()
        .expect("failed to run mdid-cli moat history with missing contains value");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--contains requires a value"),
        "stderr was: {stderr}"
    );
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
fn cli_rejects_invalid_moat_history_limit() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "history",
            "--history-path",
            "ignored-history.jsonl",
            "--limit",
            "0",
        ])
        .output()
        .expect("failed to run mdid-cli moat history with invalid limit");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("limit must be greater than zero\n{USAGE}\n")
    );
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
fn cli_history_rejects_duplicate_decision_flags() {
    let history_path = unique_history_path("duplicate-history-decision");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "history",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--decision",
            "Continue",
            "--decision",
            "Stop",
        ])
        .output()
        .expect("failed to run mdid-cli moat history with duplicate decisions");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("duplicate flag: --decision\n{USAGE}\n")
    );
    assert!(!history_path.exists());
}

#[test]
fn cli_history_accepts_pivot_decision_filter_before_opening_history() {
    let history_path = unique_history_path("pivot-history-decision");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "history",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--decision",
            "Pivot",
        ])
        .output()
        .expect("failed to run mdid-cli moat history with pivot decision");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!(
            "failed to open moat history store: moat history file does not exist: {}\n",
            history_path.display()
        )
    );
    assert!(!history_path.exists());
}

#[test]
fn cli_history_rejects_unknown_decision_value() {
    let history_path = unique_history_path("unknown-history-decision");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "history",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--decision",
            "Pause",
        ])
        .output()
        .expect("failed to run mdid-cli moat history with unknown decision");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("unknown moat history decision: Pause\n{USAGE}\n")
    );
    assert!(!history_path.exists());
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
fn moat_task_graph_rejects_empty_history_without_round_id() {
    let history_path = unique_history_path("task-graph-empty-history-without-round-id");
    LocalMoatHistoryStore::open(&history_path).expect("empty history should be created");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
        ])
        .output()
        .expect("failed to inspect empty task graph history without round-id");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("moat history is empty; run"));

    cleanup_history_path(&history_path);
}

#[test]
fn moat_task_graph_rejects_empty_history_with_round_id() {
    let history_path = unique_history_path("task-graph-empty-history-with-round-id");
    LocalMoatHistoryStore::open(&history_path).expect("empty history should be created");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--round-id",
            "00000000-0000-4000-8000-000000000000",
        ])
        .output()
        .expect("failed to inspect empty task graph history with round-id");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("moat history is empty; run"));

    cleanup_history_path(&history_path);
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
fn cli_reports_ready_moat_tasks_for_latest_round() {
    let history_path = unique_history_path("ready-tasks-latest");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--review-loops",
            "0",
            "--history-path",
            history_path_arg,
        ])
        .output()
        .expect("failed to run mdid-cli moat round with history path");
    assert!(
        output.status.success(),
        "expected setup round success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "ready-tasks", "--history-path", history_path_arg])
        .output()
        .expect("failed to run mdid-cli moat ready-tasks");

    assert!(
        output.status.success(),
        "expected success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "moat ready tasks\n",
            "ready_task_entries=1\n",
            "ready_task=reviewer|review|review|Review|<none>\n",
        )
    );

    cleanup_history_path(&history_path);
}

#[test]
fn cli_filters_ready_tasks_by_role_kind_and_limit() {
    let history_path = unique_history_path("ready-tasks-filters");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--review-loops",
            "0",
            "--history-path",
            history_path_arg,
        ])
        .output()
        .expect("failed to run mdid-cli moat round with history path");
    assert!(
        output.status.success(),
        "expected setup round success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "ready-tasks",
            "--history-path",
            history_path_arg,
            "--role",
            "reviewer",
            "--kind",
            "review",
            "--limit",
            "1",
        ])
        .output()
        .expect("failed to run mdid-cli moat ready-tasks with filters");

    assert!(
        output.status.success(),
        "expected filtered ready tasks success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "moat ready tasks\n",
            "ready_task_entries=1\n",
            "ready_task=reviewer|review|review|Review|<none>\n",
        )
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "ready-tasks",
            "--history-path",
            history_path_arg,
            "--role",
            "coder",
        ])
        .output()
        .expect("failed to run mdid-cli moat ready-tasks with non-matching role filter");

    assert!(
        output.status.success(),
        "expected non-matching filtered ready tasks success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!("moat ready tasks\n", "ready_task_entries=0\n")
    );

    cleanup_history_path(&history_path);
}

#[test]
fn cli_reports_zero_ready_tasks_for_unknown_round_id() {
    let history_path = unique_history_path("ready-tasks-unknown-round");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to run mdid-cli moat round with history path");
    assert!(
        output.status.success(),
        "expected setup round success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "ready-tasks",
            "--history-path",
            history_path_arg,
            "--round-id",
            "00000000-0000-0000-0000-000000000000",
        ])
        .output()
        .expect("failed to run mdid-cli moat ready-tasks with unknown round id");

    assert!(
        output.status.success(),
        "expected unknown round ready tasks success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!("moat ready tasks\n", "ready_task_entries=0\n")
    );

    cleanup_history_path(&history_path);
}

#[test]
fn moat_task_graph_filters_nodes_by_exact_round_id() {
    let history_path = unique_history_path("task-graph-round-id");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let first_seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--review-loops",
            "0",
            "--history-path",
            history_path_arg,
        ])
        .output()
        .expect("failed to seed first task graph history round");
    assert!(
        first_seed.status.success(),
        "{}",
        String::from_utf8_lossy(&first_seed.stderr)
    );

    let second_seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed second task graph history round");
    assert!(
        second_seed.status.success(),
        "{}",
        String::from_utf8_lossy(&second_seed.stderr)
    );

    let first_round_id = LocalMoatHistoryStore::open_existing(&history_path)
        .expect("history should exist")
        .entries()
        .first()
        .expect("first entry should exist")
        .report
        .summary
        .round_id
        .to_string();

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--round-id",
            &first_round_id,
        ])
        .output()
        .expect("failed to inspect task graph by round id");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("node=reviewer|review|Review|review|ready|implementation|<none>\n"));
    assert!(
        !stdout.contains("node=reviewer|review|Review|review|completed|implementation|<none>\n")
    );

    cleanup_history_path(&history_path);
}

#[test]
fn moat_task_graph_filters_nodes_by_dependency() {
    let history_path = unique_history_path("task-graph-depends-on");
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
        .expect("failed to seed task graph history for dependency filter");
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
            "--depends-on",
            "implementation",
        ])
        .output()
        .expect("failed to inspect task graph by dependency");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("node=reviewer|review|Review|review|ready|implementation|<none>\n"));
    assert!(!stdout.contains("node=coder|implementation|Implementation|implementation"));
    assert!(!stdout.contains("node=reviewer|evaluation|Evaluation|evaluation"));

    cleanup_history_path(&history_path);
}

#[test]
fn moat_task_graph_filters_nodes_with_no_dependencies() {
    let history_path = unique_history_path("task-graph-no-dependencies");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed task graph history for no-dependencies filter");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let history = fs::read_to_string(&history_path)
        .expect("seeded history should be readable for no-dependencies regression setup");
    let mutated_history = history.replace(
        concat!(
            "            {\n",
            "              \"node_id\": \"market_scan\",\n",
            "              \"title\": \"Market Scan\",\n",
            "              \"role\": \"planner\",\n",
            "              \"kind\": \"market_scan\",\n",
            "              \"state\": \"completed\",\n",
            "              \"depends_on\": [],\n",
            "              \"spec_ref\": null\n",
            "            },\n"
        ),
        concat!(
            "            {\n",
            "              \"node_id\": \"market_scan\",\n",
            "              \"title\": \"Market Scan\",\n",
            "              \"role\": \"planner\",\n",
            "              \"kind\": \"market_scan\",\n",
            "              \"state\": \"completed\",\n",
            "              \"depends_on\": [],\n",
            "              \"spec_ref\": null\n",
            "            },\n",
            "            {\n",
            "              \"node_id\": \"independent_spec_planning\",\n",
            "              \"title\": \"Independent Spec Planning\",\n",
            "              \"role\": \"planner\",\n",
            "              \"kind\": \"spec_planning\",\n",
            "              \"state\": \"completed\",\n",
            "              \"depends_on\": [],\n",
            "              \"spec_ref\": null\n",
            "            },\n"
        ),
    );
    assert_ne!(
        history, mutated_history,
        "regression setup should add a second root node"
    );
    fs::write(&history_path, mutated_history)
        .expect("mutated no-dependencies regression history should be writable");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--no-dependencies",
        ])
        .output()
        .expect("failed to inspect task graph by no-dependencies filter");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout
        .contains("node=planner|market_scan|Market Scan|market_scan|completed|<none>|<none>\n"));
    assert!(stdout.contains(
        "node=planner|independent_spec_planning|Independent Spec Planning|spec_planning|completed|<none>|<none>\n"
    ));
    assert!(stdout.contains(
        "node=planner|competitor_analysis|Competitor Analysis|competitor_analysis|completed|<none>|<none>\n"
    ));
    assert!(!stdout.contains("node=coder|implementation|Implementation|implementation"));

    cleanup_history_path(&history_path);
}

#[test]
fn moat_task_graph_reports_no_nodes_for_unknown_round_id() {
    let history_path = unique_history_path("task-graph-round-id-unknown");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed task graph history for unknown round id");
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
            "--round-id",
            "unknown-round-id",
        ])
        .output()
        .expect("failed to inspect task graph by unknown round id");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "moat task graph\n");

    cleanup_history_path(&history_path);
}

#[test]
fn moat_task_graph_rejects_duplicate_round_id_filter() {
    let history_path = unique_history_path("task-graph-round-id-duplicate");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--round-id",
            "first",
            "--round-id",
            "second",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with duplicate round-id filter");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("duplicate flag: --round-id"));
    assert!(!history_path.exists());
}

#[test]
fn moat_task_graph_rejects_duplicate_depends_on_filter() {
    let history_path = unique_history_path("task-graph-depends-on-duplicate");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--depends-on",
            "implementation",
            "--depends-on",
            "review",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with duplicate depends-on filter");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("duplicate flag: --depends-on"));
    assert!(!history_path.exists());
}

#[test]
fn moat_task_graph_rejects_duplicate_no_dependencies_filter() {
    let history_path = unique_history_path("task-graph-no-dependencies-duplicate");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--no-dependencies",
            "--no-dependencies",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with duplicate no-dependencies filter");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("duplicate flag: --no-dependencies"));
    assert!(!history_path.exists());
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
fn task_graph_filters_latest_nodes_by_contains_text() {
    let history_path = unique_history_path("task-graph-contains");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history for contains filter");
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
            "--contains",
            "strategy_generation",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with contains filter");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("moat task graph\n"));
    assert!(stdout.contains(
        "node=planner|strategy_generation|Strategy Generation|strategy_generation|completed|"
    ));
    assert!(stdout.contains("node=planner|spec_planning|Spec Planning|spec_planning|completed|strategy_generation|docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md\n"));
    assert!(!stdout.contains("node=planner|market_scan|"));

    cleanup_history_path(&history_path);
}

#[test]
fn task_graph_contains_filter_returns_zero_matches_without_error() {
    let history_path = unique_history_path("task-graph-contains-zero");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history for empty contains filter");
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
            "--contains",
            "not-present-in-any-node",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with unmatched contains filter");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "moat task graph\n");

    cleanup_history_path(&history_path);
}

#[test]
fn task_graph_contains_filter_combines_with_role_filter() {
    let history_path = unique_history_path("task-graph-contains-role");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history for role and contains filter");
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
            "--contains",
            "Implementation",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with role and contains filters");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "moat task graph\nnode=coder|implementation|Implementation|implementation|completed|spec_planning|<none>\n"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn task_graph_contains_filter_rejects_missing_flag_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            "/tmp/mdid-unused-history.json",
            "--contains",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with missing contains value");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("missing value for --contains\n{}\n", USAGE)
    );
}

#[test]
fn task_graph_contains_filter_rejects_flag_like_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            "/tmp/mdid-unused-history.json",
            "--contains",
            "--role",
            "planner",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with flag-like contains value");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("missing value for --contains\n{}\n", USAGE)
    );
}

#[test]
fn task_graph_contains_filter_rejects_duplicate_flag() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            "/tmp/mdid-unused-history.json",
            "--contains",
            "one",
            "--contains",
            "two",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with duplicate contains filter");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("duplicate flag: --contains\n{}\n", USAGE)
    );
}

#[test]
fn task_graph_limit_bounds_rendered_nodes_after_filters() {
    let history_path = unique_history_path("task-graph-limit");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history for task graph limit");
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
            "planner",
            "--limit",
            "2",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with limit");

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
        2
    );
    assert!(stdout
        .contains("node=planner|market_scan|Market Scan|market_scan|completed|<none>|<none>\n"));
    assert!(stdout.contains("node=planner|competitor_analysis|Competitor Analysis|competitor_analysis|completed|<none>|<none>\n"));
    assert!(!stdout.contains("node=planner|lockin_analysis|Lock-In Analysis|lock_in_analysis|"));

    cleanup_history_path(&history_path);
}

#[test]
fn task_graph_rejects_zero_limit() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            "/tmp/mdid-unused-history.json",
            "--limit",
            "0",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with zero limit");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!(
            "invalid value for --limit: expected positive integer, got 0\n{}\n",
            USAGE
        )
    );
}

#[test]
fn task_graph_rejects_duplicate_limit() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            "/tmp/mdid-unused-history.json",
            "--limit",
            "1",
            "--limit",
            "2",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with duplicate limit");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("duplicate flag: --limit\n{}\n", USAGE)
    );
}

#[test]
fn task_graph_rejects_missing_limit_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            "/tmp/mdid-unused-history.json",
            "--limit",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with missing limit value");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("missing value for --limit\n{}\n", USAGE)
    );
}

#[test]
fn task_graph_limit_does_not_append_history() {
    let history_path = unique_history_path("task-graph-limit-read-only");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history for task graph limit read-only check");
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
            "--limit",
            "1",
        ])
        .output()
        .expect("failed to inspect moat task graph with limit");
    assert!(
        inspect.status.success(),
        "{}",
        String::from_utf8_lossy(&inspect.stderr)
    );

    let history = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "history", "--history-path", history_path_arg])
        .output()
        .expect("failed to inspect moat history after task graph limit");
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

fn patch_latest_reviewer_decision_with_escaped_field_fixtures(history_path: &PathBuf) {
    let json = fs::read_to_string(history_path).expect("history json should be readable");
    let json = json
        .replace(
            "\"summary\": \"review approved bounded moat round\"",
            "\"summary\": \"review|approved\\nsummary\"",
        )
        .replace(
            "\"rationale\": \"review approved bounded moat round after evaluation cleared the improvement threshold\"",
            "\"rationale\": \"evaluation|completed\\rpath\\\\tail\"",
        );
    fs::write(history_path, json).expect("patched history json should be writable");
}

const ESCAPED_DECISION_LOG_OUTPUT: &str = concat!(
    "decision_log_entries=1\n",
    "decision=reviewer|review\\|approved\\nsummary|evaluation\\|completed\\rpath\\\\tail\n",
);

#[test]
fn cli_moat_decision_log_escapes_pipe_delimited_summary_and_rationale_fields() {
    let history_path = unique_history_path("decision-log-escaped-fields");
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
    patch_latest_reviewer_decision_with_escaped_field_fixtures(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "decision-log", "--history-path", history_path_arg])
        .output()
        .expect("failed to run mdid-cli moat decision-log");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        ESCAPED_DECISION_LOG_OUTPUT
    );

    cleanup_history_path(&history_path);
}

#[test]
fn decision_log_escaped_output_does_not_append_history() {
    let history_path = unique_history_path("decision-log-escaped-read-only");
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
    patch_latest_reviewer_decision_with_escaped_field_fixtures(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "decision-log", "--history-path", history_path_arg])
        .output()
        .expect("failed to run mdid-cli moat decision-log");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        ESCAPED_DECISION_LOG_OUTPUT
    );

    let history = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "history", "--history-path", history_path_arg])
        .output()
        .expect("failed to run mdid-cli moat history");
    assert!(
        history.status.success(),
        "{}",
        String::from_utf8_lossy(&history.stderr)
    );
    assert!(String::from_utf8_lossy(&history.stdout).contains("entries=1\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn decision_log_filters_before_escaping_output_fields() {
    let history_path = unique_history_path("decision-log-escaped-filters");
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
    patch_latest_reviewer_decision_with_escaped_field_fixtures(&history_path);

    for args in [
        vec!["--summary-contains", "review|approved"],
        vec!["--rationale-contains", "path\\tail"],
        vec!["--contains", "completed\rpath"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
            .args(["moat", "decision-log", "--history-path", history_path_arg])
            .args(args)
            .output()
            .expect("failed to run filtered mdid-cli moat decision-log");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            ESCAPED_DECISION_LOG_OUTPUT
        );
    }

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "decision-log",
            "--history-path",
            history_path_arg,
            "--summary-contains",
            "review\\|approved",
        ])
        .output()
        .expect("failed to run nonmatching filtered mdid-cli moat decision-log");
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
fn cli_filters_moat_decision_log_by_exact_round_id() {
    let history_path = unique_history_path("decision-log-round-id-filter");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let first_seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed first moat history round");
    assert!(
        first_seed.status.success(),
        "{}",
        String::from_utf8_lossy(&first_seed.stderr)
    );
    let second_seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--review-loops",
            "0",
            "--history-path",
            history_path_arg,
        ])
        .output()
        .expect("failed to seed second moat history round");
    assert!(
        second_seed.status.success(),
        "{}",
        String::from_utf8_lossy(&second_seed.stderr)
    );

    let store = LocalMoatHistoryStore::open_existing(&history_path).expect("history should exist");
    let entries = store.entries();
    assert_eq!(entries.len(), 2);
    let older_round_id = entries[0].report.summary.round_id.to_string();
    let latest_round_id = entries[1].report.summary.round_id.to_string();
    assert_ne!(older_round_id, latest_round_id);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "decision-log",
            "--history-path",
            history_path_arg,
            "--round-id",
            &older_round_id,
        ])
        .output()
        .expect("failed to run round-id-filtered mdid-cli moat decision-log");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("decision_log_entries=1\n"));
    assert!(stdout.contains("decision=reviewer|review approved bounded moat round|review approved bounded moat round after evaluation cleared the improvement threshold\n"));
    assert!(!stdout.contains("implementation stopped before review"));

    cleanup_history_path(&history_path);
}

#[test]
fn cli_moat_decision_log_round_id_filter_reports_zero_for_missing_round() {
    let history_path = unique_history_path("decision-log-missing-round-id-filter");
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
            "--round-id",
            "missing-round-id",
        ])
        .output()
        .expect("failed to run missing-round filtered mdid-cli moat decision-log");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "decision_log_entries=0\n"
    );

    let store = LocalMoatHistoryStore::open_existing(&history_path).expect("history should exist");
    assert_eq!(store.summary().entry_count, 1);

    cleanup_history_path(&history_path);
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
fn moat_decision_log_rejects_flag_like_missing_round_id_value() {
    let history_path = unique_history_path("decision-log-missing-round-id-value");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "decision-log",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--round-id",
            "--role",
            "planner",
        ])
        .output()
        .expect("failed to run decision-log with missing round-id value");

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("missing value for --round-id"));
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
fn cli_decision_log_limits_filtered_rows_to_requested_count() {
    let history_path = unique_history_path("decision-log-limit");
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

    let mut store = LocalMoatHistoryStore::open(&history_path).expect("history store should open");
    let mut latest_report = store
        .entries()
        .last()
        .expect("seeded history should have latest entry")
        .report
        .clone();
    let mut second_decision = latest_report.control_plane.memory.decisions[0].clone();
    second_decision.summary = "follow-up bounded moat decision".to_string();
    second_decision.rationale =
        "second bounded decision should be truncated by --limit 1".to_string();
    latest_report
        .control_plane
        .memory
        .decisions
        .push(second_decision);
    assert_eq!(
        latest_report
            .control_plane
            .memory
            .decisions
            .iter()
            .filter(|decision| {
                decision.summary.contains("bounded") || decision.rationale.contains("bounded")
            })
            .count(),
        2,
        "latest seeded history entry should contain multiple matching decisions"
    );
    store
        .append(SystemTime::now().into(), latest_report)
        .expect("multi-decision history entry should append");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "decision-log",
            "--history-path",
            history_path_arg,
            "--contains",
            "bounded",
            "--limit",
            "1",
        ])
        .output()
        .expect("failed to run mdid-cli moat decision-log with limit");

    assert!(
        output.status.success(),
        "expected success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).lines().count(),
        2,
        "expected entry count plus one limited decision row"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("decision_log_entries=1\n"));
    assert!(
        stdout.contains("decision=reviewer|follow-up bounded moat decision|"),
        "--limit 1 should keep the newest matching decision, stdout was: {stdout}"
    );
    assert!(
        !stdout.contains("decision=reviewer|review approved bounded moat round|"),
        "--limit 1 should exclude the older matching decision, stdout was: {stdout}"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn cli_rejects_zero_decision_log_limit() {
    let history_path = unique_history_path("decision-log-zero-limit");
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
            "--limit",
            "0",
        ])
        .output()
        .expect("failed to run mdid-cli moat decision-log with zero limit");

    assert!(!output.status.success(), "expected command to fail");
    assert!(String::from_utf8_lossy(&output.stderr).contains("--limit must be greater than 0"));

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
fn decision_log_filters_latest_decisions_by_summary_text() {
    let history_path = unique_history_path("decision-log-summary-contains");
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
            "--summary-contains",
            "review approved",
        ])
        .output()
        .expect("failed to run summary-filtered decision log");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("decision_log_entries=1\n"), "{stdout}");
    assert!(stdout.contains("decision=reviewer|review approved bounded moat round|review approved bounded moat round after evaluation cleared the improvement threshold\n"), "{stdout}");

    cleanup_history_path(&history_path);
}

#[test]
fn decision_log_summary_filter_ignores_rationale_only_matches() {
    let history_path = unique_history_path("decision-log-summary-contains-rationale");
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
            "--summary-contains",
            "evaluation cleared",
        ])
        .output()
        .expect("failed to run summary-filtered decision log");

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
fn decision_log_combines_role_and_summary_filters() {
    let history_path = unique_history_path("decision-log-summary-role");
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
            "--summary-contains",
            "review approved",
        ])
        .output()
        .expect("failed to run role-and-summary-filtered decision log");

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
fn decision_log_rejects_missing_summary_contains_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "decision-log", "--summary-contains"])
        .output()
        .expect("failed to run decision log with missing summary contains value");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("missing value for --summary-contains")
    );
}

#[test]
fn decision_log_rejects_flag_like_summary_contains_value() {
    let history_path = unique_history_path("decision-log-summary-flag-like");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "decision-log",
            "--history-path",
            history_path_arg,
            "--summary-contains",
            "--role",
            "reviewer",
        ])
        .output()
        .expect("failed to run decision log with flag-like summary contains value");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("missing value for --summary-contains")
    );
    assert!(!history_path.exists());
}

#[test]
fn decision_log_rejects_duplicate_summary_contains_filter() {
    let history_path = unique_history_path("decision-log-summary-duplicate");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "decision-log",
            "--history-path",
            history_path_arg,
            "--summary-contains",
            "review",
            "--summary-contains",
            "approved",
        ])
        .output()
        .expect("failed to run decision log with duplicate summary contains filter");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("duplicate flag: --summary-contains"));
    assert!(!history_path.exists());
}

#[test]
fn decision_log_filters_latest_decisions_by_rationale_contains() {
    let history_path = unique_history_path("decision-log-rationale");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history for rationale filter");
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
            "--rationale-contains",
            "evaluation cleared",
        ])
        .output()
        .expect("failed to run mdid-cli moat decision-log with rationale filter");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("decision_log_entries=1\n"));
    assert!(stdout.contains("decision=reviewer|review approved bounded moat round|review approved bounded moat round after evaluation cleared the improvement threshold\n"));
    assert!(!stdout.contains("decision=planner|selected workflow audit strategy|"));

    cleanup_history_path(&history_path);
}

#[test]
fn decision_log_rationale_filter_returns_zero_entries_when_no_rationale_matches() {
    let history_path = unique_history_path("decision-log-rationale-empty");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history for empty rationale filter");
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
            "--rationale-contains",
            "not in persisted rationale",
        ])
        .output()
        .expect("failed to run mdid-cli moat decision-log with unmatched rationale filter");

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
fn decision_log_rationale_filter_conjoins_with_role_filter() {
    let history_path = unique_history_path("decision-log-rationale-role");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history for rationale role filter");
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
            "--rationale-contains",
            "evaluation cleared",
        ])
        .output()
        .expect("failed to run mdid-cli moat decision-log with role and rationale filters");

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
fn decision_log_rejects_missing_rationale_contains_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "decision-log",
            "--history-path",
            "/tmp/mdid-unused-history.json",
            "--rationale-contains",
        ])
        .output()
        .expect("failed to run mdid-cli moat decision-log with missing rationale value");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("missing value for --rationale-contains\n{}\n", USAGE)
    );
}

#[test]
fn decision_log_rejects_flag_like_rationale_contains_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "decision-log",
            "--history-path",
            "/tmp/mdid-unused-history.json",
            "--rationale-contains",
            "--role",
            "planner",
        ])
        .output()
        .expect("failed to run mdid-cli moat decision-log with flag-like rationale value");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("missing value for --rationale-contains\n{}\n", USAGE)
    );
}

#[test]
fn decision_log_rejects_duplicate_rationale_contains_filter() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "decision-log",
            "--history-path",
            "/tmp/mdid-unused-history.json",
            "--rationale-contains",
            "evaluation",
            "--rationale-contains",
            "tests",
        ])
        .output()
        .expect("failed to run mdid-cli moat decision-log with duplicate rationale filter");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("duplicate flag: --rationale-contains\n{}\n", USAGE)
    );
}

#[test]
fn decision_log_rationale_filter_does_not_append_history() {
    let history_path = unique_history_path("decision-log-rationale-read-only");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history for rationale read-only check");
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
            "--rationale-contains",
            "evaluation cleared",
        ])
        .output()
        .expect("failed to inspect moat decision log by rationale");
    assert!(
        inspect.status.success(),
        "{}",
        String::from_utf8_lossy(&inspect.stderr)
    );

    let history = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "history", "--history-path", history_path_arg])
        .output()
        .expect("failed to inspect moat history after rationale decision-log filter");
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
fn cli_rejects_unknown_moat_assignments_state() {
    let history_path = unique_history_path("assignments-unknown-state");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--state",
            "done",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with unknown state");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unknown moat assignments state: done"),
        "stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!history_path.exists());
}

#[test]
fn cli_assignments_limit_bounds_filtered_rows_and_does_not_append_history() {
    let history_path = unique_history_path("assignments-limit-bounds");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    seed_moat_history_with_assignment_rows(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--limit",
            "1",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with limit");

    assert!(
        output.status.success(),
        "stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("assignment_entries=1\n"));
    assert_eq!(stdout.matches("assignment=").count(), 1);
    assert!(stdout.contains("assignment=planner|strategy_generation|"));

    let history = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "history", "--history-path", history_path_arg])
        .output()
        .expect("failed to inspect moat history after limited assignments");
    assert!(
        history.status.success(),
        "stderr was: {}",
        String::from_utf8_lossy(&history.stderr)
    );
    assert!(String::from_utf8_lossy(&history.stdout).contains("entries=1\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn cli_assignments_limit_applies_after_role_filter() {
    let history_path = unique_history_path("assignments-limit-role");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    seed_moat_history_with_assignment_rows(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--role",
            "planner",
            "--limit",
            "1",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with role and limit");

    assert!(
        output.status.success(),
        "stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("assignment_entries=1\n"));
    assert_eq!(stdout.matches("assignment=planner|").count(), 1);
    assert!(stdout.contains("assignment=planner|strategy_generation|"));
    assert!(!stdout.contains("assignment=coder|"));
    assert!(!stdout.contains("assignment=reviewer|"));

    cleanup_history_path(&history_path);
}

#[test]
fn cli_rejects_zero_assignments_limit_before_touching_missing_history() {
    let history_path = unique_history_path("assignments-limit-zero");
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--limit",
            "0",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with zero limit");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--limit must be greater than 0"),
        "stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!history_path.exists());
}

#[test]
fn cli_rejects_duplicate_assignments_limit_before_touching_missing_history() {
    let history_path = unique_history_path("assignments-limit-duplicate");
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--limit",
            "1",
            "--limit",
            "2",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with duplicate limit");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("duplicate flag: --limit"),
        "stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!history_path.exists());
}

#[test]
fn cli_rejects_missing_assignments_limit_before_touching_missing_history() {
    let history_path = unique_history_path("assignments-limit-missing");
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--limit",
            "--role",
            "planner",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with missing limit");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("missing value for --limit"),
        "stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!history_path.exists());
}

#[test]
fn cli_filters_moat_assignments_by_exact_round_id() {
    let history_path = unique_history_path("assignments-round-id-filter");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let first_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--review-loops",
            "0",
            "--history-path",
            history_path_arg,
        ])
        .output()
        .expect("failed to seed first moat history round for assignments round-id filter");
    assert!(
        first_output.status.success(),
        "expected first round success, stderr was: {}",
        String::from_utf8_lossy(&first_output.stderr)
    );

    let store = LocalMoatHistoryStore::open(&history_path).expect("history store should open");
    let first_round_id = store
        .summary()
        .latest_round_id
        .expect("first round id should exist");
    drop(store);

    let second_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed second moat history round for assignments round-id filter");
    assert!(
        second_output.status.success(),
        "expected second round success, stderr was: {}",
        String::from_utf8_lossy(&second_output.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--round-id",
            &first_round_id,
            "--role",
            "reviewer",
        ])
        .output()
        .expect("failed to inspect moat assignments by round id");

    assert!(
        output.status.success(),
        "expected success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "moat assignments\nassignment_entries=1\nassignment=reviewer|review|Review|review|<none>\n"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn cli_returns_empty_moat_assignments_for_unknown_round_id() {
    let history_path = unique_history_path("assignments-round-id-empty");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history for unknown assignments round-id filter");
    assert!(
        seed_output.status.success(),
        "expected seed success, stderr was: {}",
        String::from_utf8_lossy(&seed_output.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--round-id",
            "missing-round-id",
        ])
        .output()
        .expect("failed to inspect moat assignments by unknown round id");

    assert!(
        output.status.success(),
        "expected success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "moat assignments\nassignment_entries=0\n"
    );

    cleanup_history_path(&history_path);
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
fn cli_filters_moat_assignments_by_task_dependency() {
    let history_path = unique_history_path("assignments-depends-on-filter");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let round_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--review-loops",
            "0",
            "--history-path",
            history_path_arg,
        ])
        .output()
        .expect("failed to run mdid-cli moat round with history path");
    assert!(
        round_output.status.success(),
        "expected round success, stderr was: {}",
        String::from_utf8_lossy(&round_output.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--depends-on",
            "implementation",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with depends-on filter");

    assert!(
        output.status.success(),
        "expected assignments success, stderr was: {}",
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

    let after_summary = LocalMoatHistoryStore::open(&history_path)
        .expect("history store should reopen")
        .summary();
    assert_eq!(
        after_summary.entry_count, 1,
        "read-only assignments dependency filter must not append history"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn moat_assignments_filters_entries_with_no_dependencies() {
    let history_path = unique_history_path("assignments-no-dependencies");
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
        .expect("failed to seed assignments history for no-dependencies filter");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );
    let persisted = fs::read_to_string(&history_path).expect("history should be readable");
    let original = concat!(
        "\"agent_assignments\": [\n",
        "          {\n",
        "            \"role\": \"reviewer\",\n",
        "            \"node_id\": \"review\",\n",
        "            \"title\": \"Review\",\n",
        "            \"kind\": \"review\",\n",
        "            \"spec_ref\": null\n",
        "          }\n",
        "        ]"
    );
    let replacement = concat!(
        "\"agent_assignments\": [\n",
        "          {\n",
        "            \"role\": \"planner\",\n",
        "            \"node_id\": \"market_scan\",\n",
        "            \"title\": \"Market Scan\",\n",
        "            \"kind\": \"market_scan\",\n",
        "            \"spec_ref\": null\n",
        "          },\n",
        "          {\n",
        "            \"role\": \"planner\",\n",
        "            \"node_id\": \"competitor_analysis\",\n",
        "            \"title\": \"Competitor Analysis\",\n",
        "            \"kind\": \"competitor_analysis\",\n",
        "            \"spec_ref\": null\n",
        "          },\n",
        "          {\n",
        "            \"role\": \"coder\",\n",
        "            \"node_id\": \"implementation\",\n",
        "            \"title\": \"Implementation\",\n",
        "            \"kind\": \"implementation\",\n",
        "            \"spec_ref\": null\n",
        "          }\n",
        "        ]"
    );
    let patched = persisted.replacen(original, replacement, 1);
    assert_ne!(patched, persisted, "seed history assignment block changed");
    fs::write(&history_path, patched).expect("history should be patchable");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--no-dependencies",
        ])
        .output()
        .expect("failed to inspect assignments by no-dependencies filter");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("assignment=planner|market_scan|Market Scan|market_scan|<none>\n"));
    assert!(stdout.contains(
        "assignment=planner|competitor_analysis|Competitor Analysis|competitor_analysis|<none>\n"
    ));
    assert!(!stdout.contains("assignment=coder|implementation|Implementation|implementation"));

    cleanup_history_path(&history_path);
}

#[test]
fn cli_combines_moat_assignments_depends_on_with_role_filter() {
    let history_path = unique_history_path("assignments-depends-on-role-filter");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let round_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--review-loops",
            "0",
            "--history-path",
            history_path_arg,
        ])
        .output()
        .expect("failed to run mdid-cli moat round with history path");
    assert!(
        round_output.status.success(),
        "expected round success, stderr was: {}",
        String::from_utf8_lossy(&round_output.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--depends-on",
            "implementation",
            "--role",
            "coder",
        ])
        .output()
        .expect(
            "failed to run mdid-cli moat assignments with combined depends-on and role filters",
        );

    assert!(
        output.status.success(),
        "expected assignments success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!("moat assignments\n", "assignment_entries=0\n")
    );

    cleanup_history_path(&history_path);
}

#[test]
fn cli_rejects_moat_assignments_depends_on_without_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            "history.json",
            "--depends-on",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with missing depends-on value");

    assert!(!output.status.success(), "expected command failure");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--depends-on requires a value"),
        "stderr should explain missing depends-on value, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn cli_rejects_duplicate_moat_assignments_depends_on_filter() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            "history.json",
            "--depends-on",
            "implementation",
            "--depends-on",
            "review",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with duplicate depends-on filter");

    assert!(!output.status.success(), "expected command failure");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("duplicate flag: --depends-on"),
        "stderr should explain duplicate depends-on filter, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn moat_assignments_rejects_duplicate_no_dependencies_filter() {
    let history_path = unique_history_path("assignments-no-dependencies-duplicate");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--no-dependencies",
            "--no-dependencies",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with duplicate no-dependencies filter");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("duplicate flag: --no-dependencies"));
    assert!(!history_path.exists());
}

#[test]
fn moat_assignments_filters_by_task_state() {
    let history_path = unique_history_path("assignments-state-ready");
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
        "stderr was: {}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--state",
            "ready",
        ])
        .output()
        .expect("failed to inspect moat assignments by task state");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("assignment_entries=1\n"));
    assert!(stdout.contains("assignment=planner|strategy_generation|"));
    assert!(!stdout.contains("assignment=coder|implementation|"));
    assert!(!stdout.contains("assignment=reviewer|review|"));

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
fn assignments_contains_filters_latest_assignments_by_raw_text() {
    let history_path = unique_history_path("assignments-contains");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    seed_successful_moat_history(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--contains",
            "Strategy",
        ])
        .output()
        .expect("failed to inspect moat assignments by contains text");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("assignment_entries=1\n"));
    assert!(stdout.contains(
        "assignment=planner|strategy_generation|Strategy Generation|strategy_generation|<none>\n"
    ));
    assert!(!stdout.contains("assignment=reviewer|review|"));

    cleanup_history_path(&history_path);
}

#[test]
fn assignments_contains_filter_returns_zero_when_no_assignment_matches() {
    let history_path = unique_history_path("assignments-contains-empty");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    seed_successful_moat_history(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--contains",
            "No Such Assignment Text",
        ])
        .output()
        .expect("failed to inspect moat assignments by unmatched contains text");

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
fn assignments_contains_filter_is_conjunctive_with_role_filter() {
    let history_path = unique_history_path("assignments-contains-role");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    seed_successful_moat_history(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--role",
            "reviewer",
            "--contains",
            "Strategy",
        ])
        .output()
        .expect("failed to inspect moat assignments by role and contains text");

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
fn assignments_contains_rejects_missing_value_without_touching_history() {
    let history_path = unique_history_path("assignments-contains-missing");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--contains",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with missing contains value");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing value for --contains"));
    assert!(!history_path.exists());
}

#[test]
fn assignments_contains_rejects_flag_like_value_without_touching_history() {
    let history_path = unique_history_path("assignments-contains-flag-like");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--contains",
            "--role",
            "planner",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with flag-like contains value");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing value for --contains"));
    assert!(!history_path.exists());
}

#[test]
fn assignments_contains_rejects_duplicate_filter_without_touching_history() {
    let history_path = unique_history_path("assignments-contains-duplicate");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--contains",
            "Strategy",
            "--contains",
            "Review",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with duplicate contains filter");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("duplicate flag: --contains"));
    assert!(!history_path.exists());
}

#[test]
fn assignments_contains_filter_does_not_append_history() {
    let history_path = unique_history_path("assignments-contains-read-only");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    seed_successful_moat_history(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--contains",
            "Strategy",
        ])
        .output()
        .expect("failed to inspect moat assignments by contains text");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let history_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "history", "--history-path", history_path_arg])
        .output()
        .expect("failed to inspect moat history after contains assignments filter");
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

fn seed_moat_history_with_assignment_rows(history_path: &PathBuf) {
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

    let persisted = fs::read_to_string(history_path).expect("history should be readable");
    let original = concat!(
        "\"agent_assignments\": [\n",
        "          {\n",
        "            \"role\": \"reviewer\",\n",
        "            \"node_id\": \"review\",\n",
        "            \"title\": \"Review\",\n",
        "            \"kind\": \"review\",\n",
        "            \"spec_ref\": null\n",
        "          }\n",
        "        ]"
    );
    let replacement = concat!(
        "\"agent_assignments\": [\n",
        "          {\n",
        "            \"role\": \"planner\",\n",
        "            \"node_id\": \"strategy_generation\",\n",
        "            \"title\": \"Strategy generation\",\n",
        "            \"kind\": \"strategy_generation\",\n",
        "            \"spec_ref\": null\n",
        "          },\n",
        "          {\n",
        "            \"role\": \"planner\",\n",
        "            \"node_id\": \"spec_planning\",\n",
        "            \"title\": \"Spec planning\",\n",
        "            \"kind\": \"spec_planning\",\n",
        "            \"spec_ref\": \"docs/superpowers/specs/example.md\"\n",
        "          },\n",
        "          {\n",
        "            \"role\": \"coder\",\n",
        "            \"node_id\": \"implementation\",\n",
        "            \"title\": \"Implementation\",\n",
        "            \"kind\": \"implementation\",\n",
        "            \"spec_ref\": \"docs/superpowers/specs/example.md\"\n",
        "          },\n",
        "          {\n",
        "            \"role\": \"reviewer\",\n",
        "            \"node_id\": \"review\",\n",
        "            \"title\": \"Review\",\n",
        "            \"kind\": \"review\",\n",
        "            \"spec_ref\": null\n",
        "          }\n",
        "        ]"
    );
    let patched = persisted.replacen(original, replacement, 1);
    assert_ne!(patched, persisted, "seed history assignment block changed");
    fs::write(history_path, patched).expect("history should be patchable");
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
