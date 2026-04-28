use mdid_runtime::moat_history::LocalMoatHistoryStore;
use serde_json::Value;
use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

const USAGE: &str = "usage: mdid-cli [status | moat round [--input-path PATH] [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false] [--history-path PATH] [--format text|json] | moat control-plane [--input-path PATH] [--history-path PATH] [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false] [--format text|json] | moat history --history-path PATH [--round-id ROUND_ID] [--decision Continue|Stop|Pivot] [--contains TEXT] [--stop-reason-contains TEXT] [--min-score N] [--tests-passed true|false] [--limit N] [--format text|json] | moat decision-log --history-path PATH [--round-id ROUND_ID] [--role planner|coder|reviewer] [--contains TEXT] [--summary-contains TEXT] [--rationale-contains TEXT] [--limit N] | moat assignments --history-path PATH [--round-id ROUND_ID] [--role planner|coder|reviewer] [--state pending|ready|in_progress|completed|blocked] [--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation] [--node-id NODE_ID] [--assigned-agent-id AGENT_ID] [--depends-on NODE_ID] [--no-dependencies] [--title-contains TEXT] [--spec-ref SPEC_REF] [--contains TEXT] [--limit N] | moat task-graph --history-path PATH [--round-id ROUND_ID] [--role planner|coder|reviewer] [--state pending|ready|in_progress|completed|blocked] [--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation] [--node-id NODE_ID] [--assigned-agent-id AGENT_ID] [--depends-on NODE_ID] [--no-dependencies] [--title-contains TEXT] [--spec-ref SPEC_REF] [--contains TEXT] [--limit N] | moat task-events --history-path PATH [--round-id ROUND_ID] [--node-id NODE_ID] [--action claim|heartbeat|reap|complete|release|block|unblock] [--agent-id AGENT_ID] [--contains TEXT] [--limit N] [--format text|json] | moat work-packet --history-path PATH --node-id NODE_ID [--round-id ROUND_ID] [--format text|json] | moat ready-tasks --history-path PATH [--round-id ROUND_ID] [--role planner|coder|reviewer] [--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation] [--node-id NODE_ID] [--depends-on NODE_ID] [--no-dependencies] [--requires-artifacts] [--title-contains TEXT] [--spec-ref SPEC_REF] [--limit N] [--format text|json] | moat artifacts --history-path PATH [--round-id ROUND_ID] [--role planner|coder|reviewer] [--state pending|ready|in_progress|completed|blocked] [--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation] [--node-id NODE_ID] [--contains TEXT] [--artifact-ref TEXT] [--artifact-summary TEXT] [--limit N] [--format text|json] | moat dispatch-next --history-path PATH [--round-id ROUND_ID] [--role planner|coder|reviewer] [--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation] [--node-id NODE_ID] [--depends-on NODE_ID] [--no-dependencies] [--requires-artifacts] [--title-contains TEXT] [--spec-ref SPEC_REF] [--agent-id AGENT_ID] [--lease-seconds N] [--dry-run] [--format text|json] | moat claim-task --history-path PATH --node-id NODE_ID [--round-id ROUND_ID] [--agent-id AGENT_ID] [--lease-seconds N] [--format text|json] | moat heartbeat-task --history-path PATH --node-id NODE_ID [--round-id ROUND_ID] [--agent-id AGENT_ID] [--lease-seconds N] [--format text|json] | moat reap-stale-tasks --history-path PATH [--round-id ROUND_ID] [--now RFC3339] [--format text|json] | moat complete-task --history-path PATH --node-id NODE_ID [--round-id ROUND_ID] [--agent-id AGENT_ID] [--artifact-ref TEXT --artifact-summary TEXT] [--format text|json] | moat release-task --history-path PATH --node-id NODE_ID [--round-id ROUND_ID] | moat block-task --history-path PATH --node-id NODE_ID [--round-id ROUND_ID] | moat unblock-task --history-path PATH --node-id NODE_ID [--round-id ROUND_ID] | moat continue --history-path PATH [--improvement-threshold N] | moat schedule-next --history-path PATH [--improvement-threshold N] [--format text|json] | moat export-specs --history-path PATH [--round-id ROUND_ID] --output-dir DIR | moat export-plans --history-path PATH [--round-id ROUND_ID] --output-dir DIR]";

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
fn moat_round_json_emits_deterministic_controller_envelope() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--format", "json"])
        .output()
        .expect("failed to run mdid-cli moat round json");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("valid json envelope");

    assert_eq!(value["type"], "moat_round");
    assert_eq!(value["source"], "sample");
    assert_eq!(value["history_path"], serde_json::Value::Null);
    assert_eq!(value["input_path"], serde_json::Value::Null);
    assert_eq!(value["history_saved"], false);
    assert!(value["round_id"]
        .as_str()
        .expect("round_id string")
        .starts_with("moat-round-"));
    assert_eq!(value["continue_decision"], "Continue");
    assert!(
        value["executed_tasks"]
            .as_array()
            .expect("executed tasks array")
            .len()
            > 0
    );
    assert!(
        value["implemented_specs"]
            .as_array()
            .expect("implemented specs array")
            .len()
            > 0
    );
    assert!(
        value["moat_score_before"]
            .as_u64()
            .expect("score before number")
            > 0
    );
    assert!(
        value["moat_score_after"]
            .as_u64()
            .expect("score after number")
            > 0
    );
    assert!(value["improvement_delta"].is_number());
    assert!(value["stop_reason"].is_null());
    assert!(value["ready_tasks"].as_array().is_some());
    assert!(value["assignments"].as_array().is_some());
    assert!(
        value["task_states"]
            .as_array()
            .expect("task states array")
            .len()
            > 0
    );
    assert!(
        value["decision_summary"]
            .as_str()
            .expect("decision summary string")
            .len()
            > 0
    );
    assert!(value["constraints"]
        .as_array()
        .expect("constraints array")
        .iter()
        .any(|item| item == "local_only"));
}

#[test]
fn moat_round_default_text_output_is_preserved() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round"])
        .output()
        .expect("failed to run mdid-cli moat round text");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert_eq!(
        stdout,
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
fn moat_round_explicit_text_output_matches_default() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--format", "text"])
        .output()
        .expect("failed to run mdid-cli moat round explicit text");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert_eq!(stdout, concat!(
        "moat round complete\n",
        "continue_decision=Continue\n",
        "executed_tasks=market_scan,competitor_analysis,lockin_analysis,strategy_generation,spec_planning,implementation,review,evaluation\n",
        "implemented_specs=moat-spec/workflow-audit\n",
        "moat_score_before=90\n",
        "moat_score_after=98\n",
        "stop_reason=<none>\n",
    ));
}

#[test]
fn moat_round_rejects_unknown_format() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--format", "yaml"])
        .output()
        .expect("failed to run mdid-cli moat round unknown format");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("unknown moat round format: yaml"));
}

#[test]
fn moat_round_rejects_missing_format_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--format"])
        .output()
        .expect("failed to run mdid-cli moat round missing format value");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("missing value for moat round --format"));
}

#[test]
fn moat_round_rejects_duplicate_format() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--format", "json", "--format", "text"])
        .output()
        .expect("failed to run mdid-cli moat round duplicate format");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("duplicate moat round --format"));
}

#[test]
fn moat_round_json_reports_input_and_history_paths_after_persisting() {
    let temp_dir = unique_history_directory_path("moat-round-json-input-history");
    fs::create_dir_all(&temp_dir).expect("create temp dir");
    let input_path = temp_dir.join("round-input.json");
    let history_path = temp_dir.join("moat-history.json");
    fs::write(&input_path, local_moat_round_input_json()).expect("write moat input");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--input-path",
            input_path.to_str().expect("input path utf8"),
            "--history-path",
            history_path.to_str().expect("history path utf8"),
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run mdid-cli moat round json with input/history");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        history_path.exists(),
        "history file should be persisted before json output"
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("valid json envelope");

    assert_eq!(value["type"], "moat_round");
    assert_eq!(value["source"], "input");
    assert_eq!(value["input_path"], input_path.display().to_string());
    assert_eq!(value["history_path"], history_path.display().to_string());
    assert_eq!(value["history_saved"], true);
    assert!(value["round_id"]
        .as_str()
        .expect("round_id string")
        .starts_with("moat-round-"));
}

fn local_moat_round_input_json() -> String {
    serde_json::json!({
        "market": {
            "market_id": "clinic-deid",
            "industry_segment": "local clinic de-identification",
            "market_snapshot_at": null,
            "moat_score": 20,
            "moat_type": ["compliance_moat"],
            "confidence": 0.8,
            "evidence": ["local workflow evidence"],
            "assumptions": ["clinic users value offline auditability"]
        },
        "competitor": {
            "competitor_id": "manual-competitor",
            "name": "Manual redaction workflow",
            "category": "manual_process",
            "pricing_summary": "staff time",
            "feature_summary": "spreadsheet tracking and manual review",
            "talent_signal_summary": "compliance operations",
            "suspected_moat_types": ["workflow_lockin"],
            "threat_score": 80,
            "evidence": ["manual workflows are entrenched"]
        },
        "lock_in": {
            "lockin_score": 90,
            "lockin_vectors": ["workflow_dependency"],
            "switching_cost_strength": 85,
            "data_gravity_strength": 70,
            "workflow_dependency_strength": 95,
            "portability_risk": 40,
            "evidence": ["audit evidence stays in local workflows"]
        },
        "strategies": [{
            "strategy_id": "clinic-workflow-lock",
            "title": "Clinic workflow lock",
            "rationale": "Use local audit trails to make repeat clinic workflows safer and stickier.",
            "target_moat_type": "workflow_lockin",
            "implementation_cost": 1,
            "expected_moat_gain": 12,
            "risk_level": 2,
            "dependencies": ["local-audit-foundation"],
            "testable_hypotheses": ["operators reuse generated review packets"]
        }],
        "budget": {
            "max_round_minutes": 30,
            "max_parallel_tasks": 3,
            "max_strategy_candidates": 2,
            "max_spec_generations": 1,
            "max_implementation_tasks": 1,
            "max_review_loops": 1
        },
        "improvement_threshold": 3,
        "tests_passed": true
    })
    .to_string()
}

#[test]
fn moat_round_uses_local_json_input_file() {
    let input_path = unique_history_path("round-input-file");
    let history_path = unique_history_path("round-input-history");
    let input_path_arg = input_path.to_string_lossy().to_string();
    let history_path_arg = history_path.to_string_lossy().to_string();

    fs::write(&input_path, local_moat_round_input_json())
        .expect("failed to write moat input fixture");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--input-path",
            &input_path_arg,
            "--history-path",
            &history_path_arg,
        ])
        .output()
        .expect("failed to run moat round with input path");

    assert!(
        output.status.success(),
        "round failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(&format!("input_path={input_path_arg}\n")));
    assert!(stdout.contains("implemented_specs=moat-spec/clinic-workflow-lock\n"));
    assert!(stdout.contains(&format!("history_saved_to={history_path_arg}\n")));

    cleanup_history_path(&input_path);
    cleanup_history_path(&history_path);
}

#[test]
fn moat_control_plane_uses_local_json_input_file_without_saving_history() {
    let input_path = unique_history_path("control-plane-input-file");
    let input_path_arg = input_path.to_string_lossy().to_string();

    fs::write(&input_path, local_moat_round_input_json())
        .expect("failed to write moat input fixture");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "control-plane", "--input-path", &input_path_arg])
        .output()
        .expect("failed to run moat control-plane with input path");

    assert!(
        output.status.success(),
        "control-plane failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("source=input\n"));
    assert!(stdout.contains(&format!("input_path={input_path_arg}\n")));
    assert!(stdout.contains("latest_implemented_specs=moat-spec/clinic-workflow-lock\n"));

    cleanup_history_path(&input_path);
}

#[test]
fn moat_round_rejects_missing_input_path_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--input-path"])
        .output()
        .expect("failed to run moat round with missing input path");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("missing value for moat round --input-path"));
}

#[test]
fn moat_round_rejects_duplicate_input_path() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--input-path",
            "one.json",
            "--input-path",
            "two.json",
        ])
        .output()
        .expect("failed to run moat round with duplicate input path");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("duplicate moat round --input-path"));
}

#[test]
fn moat_control_plane_rejects_missing_input_path_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "control-plane", "--input-path"])
        .output()
        .expect("failed to run moat control-plane with missing input path");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("missing value for moat control-plane --input-path"));
}

#[test]
fn moat_control_plane_rejects_duplicate_input_path() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "control-plane",
            "--input-path",
            "one.json",
            "--input-path",
            "two.json",
        ])
        .output()
        .expect("failed to run moat control-plane with duplicate input path");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("duplicate moat control-plane --input-path"));
}

#[test]
fn moat_round_rejects_invalid_input_json() {
    let input_path = unique_history_path("round-invalid-input-json");
    let input_path_arg = input_path.to_string_lossy().to_string();
    fs::write(&input_path, "{not-json").expect("failed to write invalid input fixture");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--input-path", &input_path_arg])
        .output()
        .expect("failed to run moat round with invalid input json");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("failed to parse moat round input"));

    cleanup_history_path(&input_path);
}

#[test]
fn moat_control_plane_rejects_invalid_input_json() {
    let input_path = unique_history_path("control-plane-invalid-input-json");
    let input_path_arg = input_path.to_string_lossy().to_string();
    fs::write(&input_path, "{not-json").expect("failed to write invalid input fixture");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "control-plane", "--input-path", &input_path_arg])
        .output()
        .expect("failed to run moat control-plane with invalid input json");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("failed to parse moat round input"));

    cleanup_history_path(&input_path);
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
fn cli_emits_moat_history_json_envelope() {
    let history_path = unique_history_path("history-json-envelope");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history");
    assert!(
        seed_output.status.success(),
        "expected seed success, stderr was: {}",
        String::from_utf8_lossy(&seed_output.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "history",
            "--history-path",
            history_path_arg,
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run mdid-cli moat history json");
    assert!(
        output.status.success(),
        "expected success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let envelope: Value = serde_json::from_slice(&output.stdout).expect("stdout should be json");
    assert_eq!(envelope["type"], "moat_history");
    assert_eq!(envelope["history_path"], history_path_arg);
    assert_eq!(envelope["history_rounds"], 1);
    assert_eq!(envelope["summary"]["total_rounds"], 1);
    assert_eq!(
        envelope["rounds"].as_array().expect("rounds array").len(),
        1
    );
    assert_eq!(envelope["rounds"][0]["decision"], "Continue");
    assert_eq!(envelope["rounds"][0]["moat_score_after"], 98);
    assert_eq!(envelope["rounds"][0]["tests_passed"], true);

    cleanup_history_path(&history_path);
}

#[test]
fn cli_moat_history_defaults_to_text_after_json_format_addition() {
    let history_path = unique_history_path("history-default-text");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history");
    assert!(seed_output.status.success());

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "history", "--history-path", history_path_arg])
        .output()
        .expect("failed to run mdid-cli moat history text");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("moat history summary\n"));
    assert!(serde_json::from_slice::<Value>(&output.stdout).is_err());

    cleanup_history_path(&history_path);
}

#[test]
fn cli_rejects_unknown_moat_history_format() {
    let history_path = unique_history_path("history-unknown-format");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "history",
            "--history-path",
            history_path_arg,
            "--format",
            "xml",
        ])
        .output()
        .expect("failed to run mdid-cli moat history invalid format");
    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("error: unknown output format for --format: xml\n{USAGE}\n")
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
fn cli_emits_moat_schedule_next_json_envelope_when_gate_allows_scheduling() {
    let history_path = unique_history_path("schedule-next-json-continue");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let round_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("moat round should run");
    assert!(round_output.status.success());

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "schedule-next",
            "--history-path",
            history_path_arg,
            "--format",
            "json",
        ])
        .output()
        .expect("moat schedule-next should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let envelope: Value = serde_json::from_slice(&output.stdout).expect("stdout should be json");
    assert_eq!(envelope["type"], "moat_schedule_next");
    assert_eq!(envelope["history_path"], history_path_arg);
    assert_eq!(envelope["scheduled"], true);
    assert_eq!(envelope["reason"], "latest round cleared continuation gate");
    assert!(
        envelope["scheduled_round_id"]
            .as_str()
            .expect("scheduled round id should be a string")
            .len()
            > 0
    );
    assert_eq!(envelope["required_improvement_threshold"], 3);

    let store = LocalMoatHistoryStore::open_existing(&history_path).expect("history should exist");
    assert_eq!(store.summary().entry_count, 2);

    cleanup_history_path(&history_path);
}

#[test]
fn cli_emits_moat_schedule_next_json_envelope_when_gate_blocks_scheduling() {
    let history_path = unique_history_path("schedule-next-json-stop");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let round_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--history-path",
            history_path_arg,
            "--tests-passed",
            "false",
        ])
        .output()
        .expect("moat round should run");
    assert!(round_output.status.success());

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "schedule-next",
            "--history-path",
            history_path_arg,
            "--format",
            "json",
        ])
        .output()
        .expect("moat schedule-next should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let envelope: Value = serde_json::from_slice(&output.stdout).expect("stdout should be json");
    assert_eq!(envelope["type"], "moat_schedule_next");
    assert_eq!(envelope["history_path"], history_path_arg);
    assert_eq!(envelope["scheduled"], false);
    assert_eq!(envelope["reason"], "latest round tests failed");
    assert!(envelope["scheduled_round_id"].is_null());
    assert_eq!(envelope["required_improvement_threshold"], 3);

    let store = LocalMoatHistoryStore::open_existing(&history_path).expect("history should exist");
    assert_eq!(store.summary().entry_count, 1);

    cleanup_history_path(&history_path);
}

#[test]
fn cli_moat_schedule_next_defaults_to_text_after_json_format_addition() {
    let history_path = unique_history_path("schedule-next-default-text");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let round_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("moat round should run");
    assert!(round_output.status.success());

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "schedule-next", "--history-path", history_path_arg])
        .output()
        .expect("moat schedule-next should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("moat schedule next\n"));
    assert!(stdout.contains("scheduled=true\n"));
    assert!(stdout.contains("reason=latest round cleared continuation gate\n"));
    assert!(stdout.contains("scheduled_round_id="));
    assert!(stdout.contains(&format!("history_path={}\n", history_path.display())));

    cleanup_history_path(&history_path);
}

#[test]
fn cli_rejects_unknown_moat_schedule_next_format() {
    let history_path = unique_history_path("schedule-next-unknown-format");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "schedule-next",
            "--history-path",
            history_path_arg,
            "--format",
            "yaml",
        ])
        .output()
        .expect("moat schedule-next should run");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "error: unknown output format for --format: yaml\n"
    );
    assert!(!history_path.exists());
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
fn moat_control_plane_json_emits_deterministic_controller_snapshot() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "control-plane", "--format", "json"])
        .output()
        .expect("run moat control-plane json");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("valid json envelope");

    assert_eq!(value["type"], "moat_control_plane");
    assert_eq!(value["history_path"], serde_json::Value::Null);
    assert_eq!(value["source"], "sample");
    assert!(value["round_id"]
        .as_str()
        .expect("round_id string")
        .starts_with("moat-round-"));
    assert!(value["score"].as_u64().expect("score number") > 0);
    assert!(value["improvement_delta"].is_number());
    assert!(value["can_continue"].is_boolean());
    assert!(value["ready_tasks"]
        .as_array()
        .expect("ready tasks array")
        .is_empty());
    assert!(value["assignments"]
        .as_array()
        .expect("assignments array")
        .is_empty());
    assert!(
        value["task_states"]
            .as_array()
            .expect("task states array")
            .len()
            > 0
    );
    assert!(
        value["decision_summary"]
            .as_str()
            .expect("decision summary string")
            .len()
            > 0
    );
    assert!(value["constraints"]
        .as_array()
        .expect("constraints array")
        .iter()
        .any(|item| item == "local_only"));
}

#[test]
fn moat_control_plane_default_text_output_is_preserved() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "control-plane"])
        .output()
        .expect("run moat control-plane text");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("moat control plane"));
    assert!(stdout.contains("ready_nodes="));
    assert!(stdout.contains("task_states="));
}

#[test]
fn moat_control_plane_rejects_unknown_format() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "control-plane", "--format", "yaml"])
        .output()
        .expect("run moat control-plane unknown format");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("unknown moat control-plane format: yaml"));
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
    assert!(stderr.contains(
        "cannot combine --history-path with moat control-plane --input-path or override flags"
    ));
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
fn moat_ready_tasks_json_prints_parseable_filtered_envelope() {
    let history_path = unique_history_path("ready-tasks-json");
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
        "{}",
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
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run mdid-cli moat ready-tasks as json");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be json");
    assert_eq!(json["type"], "moat_ready_tasks");
    assert_eq!(json["history_path"], history_path_arg);
    assert_eq!(json["ready_task_entries"], 1);
    assert!(json["round_id"].as_str().is_some());
    let tasks = json["tasks"].as_array().expect("tasks should be array");
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0]["role"], "reviewer");
    assert_eq!(tasks[0]["kind"], "review");
    assert_eq!(tasks[0]["node_id"], "review");
    assert_eq!(tasks[0]["title"], "Review");
    assert!(tasks[0]["spec_ref"].is_null());

    cleanup_history_path(&history_path);
}

#[test]
fn moat_work_packet_json_exports_task_context_and_dependency_artifacts_read_only() {
    let history_path = unique_history_path("work-packet-json");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    assert!(Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--review-loops",
            "0",
            "--history-path",
            history_path_arg
        ])
        .output()
        .unwrap()
        .status
        .success());
    let mut history: Value =
        serde_json::from_str(&fs::read_to_string(&history_path).unwrap()).unwrap();
    let implementation = history[0]["report"]["control_plane"]["task_graph"]["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["node_id"] == "implementation")
        .unwrap();
    implementation["state"] = Value::String("completed".to_string());
    implementation["artifacts"] = serde_json::json!([{
        "artifact_ref": "plan://implementation-output",
        "summary": "Implemented deterministic moat workflow audit slice",
        "recorded_at": "2026-04-28T00:00:00Z"
    }]);
    let nodes = history[0]["report"]["control_plane"]["task_graph"]["nodes"]
        .as_array_mut()
        .unwrap();
    let competitor = nodes
        .iter_mut()
        .find(|node| node["node_id"] == "competitor_analysis")
        .unwrap();
    competitor["state"] = Value::String("pending".to_string());
    competitor["artifacts"] = serde_json::json!([{
        "artifact_ref": "plan://pending-competitor-output",
        "summary": "must not be exported before completion",
        "recorded_at": "2026-04-28T00:01:00Z"
    }]);
    let review = nodes
        .iter_mut()
        .find(|node| node["node_id"] == "review")
        .unwrap();
    review["depends_on"] = serde_json::json!(["implementation", "competitor_analysis"]);
    fs::write(
        &history_path,
        serde_json::to_string_pretty(&history).unwrap(),
    )
    .unwrap();
    let before = fs::read_to_string(&history_path).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "work-packet",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let packet: Value =
        serde_json::from_slice(&output.stdout).expect("packet should be valid json");
    assert_eq!(packet["type"], "moat_work_packet");
    assert_eq!(packet["history_path"], history_path_arg);
    assert_eq!(packet["node_id"], "review");
    assert_eq!(packet["role"], "reviewer");
    assert_eq!(packet["kind"], "review");
    assert_eq!(packet["state"], "ready");
    assert_eq!(packet["dependencies"][0], "implementation");
    assert_eq!(packet["dependencies"][1], "competitor_analysis");
    assert_eq!(
        packet["dependency_artifacts"].as_array().unwrap().len(),
        1,
        "only completed dependency node artifacts should be exported"
    );
    assert_eq!(
        packet["dependency_artifacts"][0]["node_id"],
        "implementation"
    );
    assert_eq!(
        packet["dependency_artifacts"][0]["artifact_ref"],
        "plan://implementation-output"
    );
    assert_eq!(
        packet["acceptance_criteria"][0],
        "Use SDD and TDD for any implementation work before completing this task."
    );
    assert!(packet["complete_command"]
        .as_str()
        .unwrap()
        .contains("moat complete-task"));
    assert_eq!(fs::read_to_string(&history_path).unwrap(), before);
    cleanup_history_path(&history_path);
}

#[test]
fn moat_work_packet_text_exports_controller_handoff_without_mutating_history() {
    let history_path = unique_history_path("work-packet-text");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    assert!(Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--review-loops",
            "0",
            "--history-path",
            history_path_arg
        ])
        .output()
        .unwrap()
        .status
        .success());
    update_history_task_node(&history_path, "implementation", |node| {
        node.insert("state".to_string(), Value::String("completed".to_string()));
        node.insert(
            "artifacts".to_string(),
            serde_json::json!([{
                "artifact_ref": "plan://impl|artifact",
                "summary": "line one|line two\nnext",
                "recorded_at": "2026-04-28T00:00:00Z"
            }]),
        );
    });
    let before = fs::read_to_string(&history_path).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "work-packet",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("moat work packet\n"));
    assert!(stdout.contains("node_id=review\n"));
    assert!(stdout.contains("role=reviewer\n"));
    assert!(stdout.contains("kind=review\n"));
    assert!(stdout.contains("state=ready\n"));
    assert!(stdout.contains("dependency=implementation\n"));
    assert!(stdout.contains(
        "dependency_artifact=implementation|plan://impl\\|artifact|line one\\|line two\\nnext\n"
    ));
    assert!(stdout.contains(
        "acceptance=Use SDD and TDD for any implementation work before completing this task.\n"
    ));
    assert!(stdout.contains("complete_command=mdid-cli moat complete-task"));
    assert_eq!(fs::read_to_string(&history_path).unwrap(), before);
    cleanup_history_path(&history_path);
}

#[test]
fn moat_work_packet_complete_command_targets_explicit_round_id() {
    let history_path = unique_history_path("work-packet-explicit-round-command");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    seed_successful_moat_history(&history_path);
    let selected_round_id = latest_history_round_id(&history_path);
    seed_successful_moat_history(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "work-packet",
            "--history-path",
            history_path_arg,
            "--round-id",
            &selected_round_id,
            "--node-id",
            "review",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run mdid-cli moat work-packet for explicit round");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let packet: Value =
        serde_json::from_slice(&output.stdout).expect("packet should be valid json");
    assert_eq!(packet["round_id"], selected_round_id);
    assert!(packet["complete_command"]
        .as_str()
        .unwrap()
        .contains(&format!("--round-id '{selected_round_id}'")));

    cleanup_history_path(&history_path);
}

#[test]
fn moat_work_packet_fails_for_missing_node() {
    let history_path = unique_history_path("work-packet-missing-node");
    let history_path_arg = history_path.to_str().unwrap();
    assert!(Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--review-loops",
            "0",
            "--history-path",
            history_path_arg
        ])
        .output()
        .unwrap()
        .status
        .success());
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "work-packet",
            "--history-path",
            history_path_arg,
            "--node-id",
            "not-a-node",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("moat work-packet node not found: not-a-node"));
    cleanup_history_path(&history_path);
}

#[test]
fn moat_work_packet_rejects_unknown_format() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "work-packet",
            "--history-path",
            "history.json",
            "--node-id",
            "review",
            "--format",
            "yaml",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unknown moat work-packet format: yaml")
    );
}

#[test]
fn claim_task_marks_latest_ready_node_in_progress() {
    let history_path = unique_history_path("claim-task-latest");
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
        .expect("failed to seed claim-task history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "claim-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
        ])
        .output()
        .expect("failed to run mdid-cli moat claim-task");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("moat task claimed\n"));
    assert!(stdout.contains("node_id=review\n"));
    assert!(stdout.contains("previous_state=ready\n"));
    assert!(stdout.contains("new_state=in_progress\n"));
    assert!(stdout.contains(&format!("history_path={history_path_arg}\n")));

    let ready = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "ready-tasks", "--history-path", history_path_arg])
        .output()
        .expect("failed to inspect ready tasks after claim");
    assert!(
        ready.status.success(),
        "{}",
        String::from_utf8_lossy(&ready.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&ready.stdout),
        concat!("moat ready tasks\n", "ready_task_entries=0\n")
    );

    let graph = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
        ])
        .output()
        .expect("failed to inspect task graph after claim");
    assert!(
        graph.status.success(),
        "{}",
        String::from_utf8_lossy(&graph.stderr)
    );
    assert!(String::from_utf8_lossy(&graph.stdout)
        .contains("node=reviewer|review|Review|review|in_progress|implementation|<none>\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn claim_task_json_prints_parseable_envelope_and_claims_ready_node() {
    let history_path = unique_history_path("claim-task-json");
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
        .expect("failed to seed claim-task json history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "claim-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
            "--agent-id",
            "planner-json",
            "--lease-seconds",
            "60",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run mdid-cli moat claim-task json");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json stdout");
    assert_eq!(json["type"], "moat_claim_task");
    assert_eq!(json["history_path"], history_path_arg);
    assert_eq!(json["node_id"], "review");
    assert_eq!(json["assigned_agent_id"], "planner-json");
    assert_eq!(json["lease_seconds"], 60);
    assert_eq!(json["previous_state"], "ready");
    assert_eq!(json["new_state"], "in_progress");
    assert!(json["lease_expires_at"].as_str().unwrap().contains('T'));
    cleanup_history_path(&history_path);
}

#[test]
fn claim_task_accepts_custom_positive_lease_seconds() {
    let history_path = unique_history_path("claim-task-custom-lease");
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
        .expect("failed to seed claim-task custom lease history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "claim-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
            "--agent-id",
            "agent-a",
            "--lease-seconds",
            "37",
        ])
        .output()
        .expect("failed to run mdid-cli moat claim-task with custom lease");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("moat task claimed\n"), "{stdout}");
    assert!(stdout.contains("assigned_agent_id=agent-a\n"), "{stdout}");
    assert!(stdout.contains("lease_seconds=37\n"), "{stdout}");

    cleanup_history_path(&history_path);
}

#[test]
fn claim_task_rejects_non_positive_lease_seconds() {
    let history_path = unique_history_path("claim-task-invalid-lease");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "claim-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
            "--lease-seconds",
            "0",
        ])
        .output()
        .expect("failed to run mdid-cli moat claim-task with invalid lease");

    assert!(
        !output.status.success(),
        "claim-task should reject zero lease seconds"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("invalid moat claim-task --lease-seconds"),
        "{stderr}"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn lifecycle_format_claim_task_rejects_missing_value() {
    let history_path = unique_history_path("lifecycle-format-missing");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "claim-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
            "--format",
        ])
        .output()
        .expect("failed to run mdid-cli moat claim-task with missing format");

    assert!(
        !output.status.success(),
        "claim-task should reject missing format"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("missing value for moat lifecycle --format"),
        "{stderr}"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn lifecycle_format_claim_task_rejects_duplicate_value() {
    let history_path = unique_history_path("lifecycle-format-duplicate");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "claim-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
            "--format",
            "text",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run mdid-cli moat claim-task with duplicate format");

    assert!(
        !output.status.success(),
        "claim-task should reject duplicate format"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("duplicate moat lifecycle --format"),
        "{stderr}"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn lifecycle_format_claim_task_rejects_unknown_value() {
    let history_path = unique_history_path("lifecycle-format-unknown");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "claim-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
            "--format",
            "yaml",
        ])
        .output()
        .expect("failed to run mdid-cli moat claim-task with unknown format");

    assert!(
        !output.status.success(),
        "claim-task should reject unknown format"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown moat lifecycle format: yaml"),
        "{stderr}"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn dispatch_next_accepts_custom_positive_lease_seconds() {
    let history_path = unique_history_path("dispatch-next-custom-lease");
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
        .expect("failed to seed dispatch-next custom lease history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path_arg,
            "--agent-id",
            "agent-a",
            "--lease-seconds",
            "41",
        ])
        .output()
        .expect("failed to run mdid-cli moat dispatch-next with custom lease");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("moat dispatch next\n"), "{stdout}");
    assert!(stdout.contains("node_id=review\n"), "{stdout}");
    assert!(stdout.contains("lease_seconds=41\n"), "{stdout}");

    cleanup_history_path(&history_path);
}

#[test]
fn dispatch_next_rejects_non_positive_lease_seconds() {
    let history_path = unique_history_path("dispatch-next-invalid-lease");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path_arg,
            "--lease-seconds",
            "-1",
        ])
        .output()
        .expect("failed to run mdid-cli moat dispatch-next with invalid lease");

    assert!(
        !output.status.success(),
        "dispatch-next should reject negative lease seconds"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("invalid moat dispatch-next --lease-seconds"),
        "{stderr}"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn heartbeat_task_extends_lease_for_claiming_agent_and_rejects_wrong_agent() {
    let history_path = unique_history_path("heartbeat-task-agent");
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
        .expect("failed to seed heartbeat-task history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );
    let claim = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "claim-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
            "--agent-id",
            "agent-a",
            "--lease-seconds",
            "10",
        ])
        .output()
        .expect("failed to claim task before heartbeat");
    assert!(
        claim.status.success(),
        "{}",
        String::from_utf8_lossy(&claim.stderr)
    );

    let wrong = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "heartbeat-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
            "--agent-id",
            "agent-b",
            "--lease-seconds",
            "20",
        ])
        .output()
        .expect("failed to run wrong-agent heartbeat");
    assert!(!wrong.status.success(), "wrong-agent heartbeat should fail");
    assert!(String::from_utf8_lossy(&wrong.stderr).contains("failed to heartbeat moat task"));

    let ok = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "heartbeat-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
            "--agent-id",
            "agent-a",
            "--lease-seconds",
            "20",
        ])
        .output()
        .expect("failed to run heartbeat");
    assert!(
        ok.status.success(),
        "{}",
        String::from_utf8_lossy(&ok.stderr)
    );
    let stdout = String::from_utf8_lossy(&ok.stdout);
    assert!(
        stdout.contains("moat task heartbeat recorded\n"),
        "{stdout}"
    );
    assert!(stdout.contains("node_id=review\n"), "{stdout}");
    assert!(stdout.contains("lease_expires_at="), "{stdout}");

    cleanup_history_path(&history_path);
}

#[test]
fn heartbeat_task_json_prints_parseable_envelope_and_extends_lease() {
    let history_path = unique_history_path("heartbeat-task-json");
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
        .expect("failed to seed heartbeat-task json history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );
    let claim = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "claim-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
            "--agent-id",
            "planner-json",
            "--lease-seconds",
            "60",
        ])
        .output()
        .expect("failed to claim before heartbeat json");
    assert!(
        claim.status.success(),
        "{}",
        String::from_utf8_lossy(&claim.stderr)
    );
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "heartbeat-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
            "--agent-id",
            "planner-json",
            "--lease-seconds",
            "120",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run heartbeat-task json");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json stdout");
    assert_eq!(json["type"], "moat_heartbeat_task");
    assert_eq!(json["history_path"], history_path_arg);
    assert_eq!(json["node_id"], "review");
    assert_eq!(json["agent_id"], "planner-json");
    assert_eq!(json["lease_seconds"], 120);
    assert_eq!(json["state"], "in_progress");
    assert!(json["lease_expires_at"].as_str().unwrap().contains('T'));
    cleanup_history_path(&history_path);
}

#[test]
fn reap_stale_tasks_rejects_duplicate_optional_flags() {
    let history_path = unique_history_path("reap-stale-tasks-duplicate-flags");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "reap-stale-tasks",
            "--history-path",
            history_path_arg,
            "--round-id",
            "round-a",
            "--round-id",
            "round-b",
        ])
        .output()
        .expect("failed to run reap-stale-tasks with duplicate round-id");

    assert!(
        !output.status.success(),
        "reap-stale-tasks should reject duplicate round-id"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("duplicate flag: --round-id\n{USAGE}\n")
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "reap-stale-tasks",
            "--history-path",
            history_path_arg,
            "--now",
            "2999-01-01T00:00:00Z",
            "--now",
            "2999-01-02T00:00:00Z",
        ])
        .output()
        .expect("failed to run reap-stale-tasks with duplicate now");

    assert!(
        !output.status.success(),
        "reap-stale-tasks should reject duplicate now"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("duplicate flag: --now\n{USAGE}\n")
    );

    cleanup_history_path(&history_path);
}

#[test]
fn reap_stale_tasks_releases_expired_claims() {
    let history_path = unique_history_path("reap-stale-tasks");
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
        .expect("failed to seed reap history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );
    let claim = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "claim-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
            "--agent-id",
            "agent-a",
            "--lease-seconds",
            "1",
        ])
        .output()
        .expect("failed to claim task before reap");
    assert!(
        claim.status.success(),
        "{}",
        String::from_utf8_lossy(&claim.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "reap-stale-tasks",
            "--history-path",
            history_path_arg,
            "--now",
            "2999-01-01T00:00:00Z",
        ])
        .output()
        .expect("failed to run reap-stale-tasks");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("moat stale tasks reaped\n"), "{stdout}");
    assert!(stdout.contains("reaped_count=1\n"), "{stdout}");
    assert!(stdout.contains("reaped_node_ids=review\n"), "{stdout}");

    let graph = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
        ])
        .output()
        .expect("failed to inspect task graph after reap");
    assert!(
        graph.status.success(),
        "{}",
        String::from_utf8_lossy(&graph.stderr)
    );
    assert!(String::from_utf8_lossy(&graph.stdout)
        .contains("node=reviewer|review|Review|review|ready|implementation|<none>\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn reap_stale_tasks_json_prints_parseable_envelope() {
    let history_path = unique_history_path("reap-stale-tasks-json");
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
        .expect("failed to seed reap json history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );
    let claim = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "claim-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
            "--agent-id",
            "planner-json",
            "--lease-seconds",
            "1",
        ])
        .output()
        .expect("failed to claim before reap json");
    assert!(
        claim.status.success(),
        "{}",
        String::from_utf8_lossy(&claim.stderr)
    );
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "reap-stale-tasks",
            "--history-path",
            history_path_arg,
            "--now",
            "2099-01-01T00:00:00Z",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run reap-stale-tasks json");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json stdout");
    assert_eq!(json["type"], "moat_reap_stale_tasks");
    assert_eq!(json["history_path"], history_path_arg);
    assert_eq!(json["reaped_count"], 1);
    assert_eq!(json["reaped_node_ids"].as_array().unwrap()[0], "review");
    cleanup_history_path(&history_path);
}

#[test]
fn claim_task_rejects_non_ready_node() {
    let history_path = unique_history_path("claim-task-non-ready");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed claim-task history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "claim-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "market_scan",
        ])
        .output()
        .expect("failed to run mdid-cli moat claim-task for non-ready node");

    assert!(
        !output.status.success(),
        "claiming completed node should fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("moat task node is not ready"), "{stderr}");
    assert!(stderr.contains("market_scan"), "{stderr}");

    cleanup_history_path(&history_path);
}

#[test]
fn cli_complete_task_reports_newly_ready_downstream_tasks() {
    let history_path = unique_history_path("complete-task-next-ready");
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
        .expect("failed to seed complete-task history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let claim = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "claim-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "strategy_generation",
        ])
        .output()
        .expect("failed to claim strategy_generation before completion");
    assert!(
        claim.status.success(),
        "{}",
        String::from_utf8_lossy(&claim.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "complete-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "strategy_generation",
        ])
        .output()
        .expect("failed to run mdid-cli moat complete-task");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("next_ready_task_entries=1\n"),
        "expected one newly ready downstream task, stdout was:\n{stdout}"
    );
    assert!(
        stdout.contains("next_ready_task=planner|spec_planning|Spec Planning|spec_planning|docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md\n"),
        "expected spec_planning to become ready, stdout was:\n{stdout}"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn moat_complete_task_json_prints_artifact_and_downstream_ready_envelope() {
    let history_path = unique_history_path("complete-task-json-envelope");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let round_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history");
    assert!(
        round_output.status.success(),
        "moat round failed: {}",
        String::from_utf8_lossy(&round_output.stderr)
    );
    make_workflow_audit_spec_task_ready(&history_path);
    update_history_task_node(&history_path, "implementation", |implementation_node| {
        implementation_node.insert(
            "node_id".to_string(),
            Value::String("implement-workflow-audit".to_string()),
        );
        implementation_node.insert(
            "depends_on".to_string(),
            Value::Array(vec![Value::String("spec-workflow-audit".to_string())]),
        );
        implementation_node.insert("state".to_string(), Value::String("pending".to_string()));
        implementation_node.insert(
            "spec_ref".to_string(),
            Value::String("moat-spec/workflow-audit".to_string()),
        );
    });

    let claim_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "claim-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "spec-workflow-audit",
            "--agent-id",
            "planner-complete-json",
        ])
        .output()
        .expect("failed to claim spec workflow audit task");
    assert!(
        claim_output.status.success(),
        "claim failed: {}",
        String::from_utf8_lossy(&claim_output.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "complete-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "spec-workflow-audit",
            "--artifact-ref",
            "docs/superpowers/specs/workflow-audit.md",
            "--artifact-summary",
            "Workflow audit spec drafted",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to complete task with json format");

    assert!(
        output.status.success(),
        "complete-task json failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("complete-task json should be parseable");
    assert_eq!(json["type"], "moat_complete_task");
    assert_eq!(json["history_path"], history_path_arg);
    assert!(json["round_id"]
        .as_str()
        .expect("round id string")
        .starts_with("moat-round-"));
    assert_eq!(json["node_id"], "spec-workflow-audit");
    assert_eq!(json["previous_state"], "in_progress");
    assert_eq!(json["new_state"], "completed");
    assert_eq!(json["artifact_recorded"], true);
    assert_eq!(
        json["artifact"]["ref"],
        "docs/superpowers/specs/workflow-audit.md"
    );
    assert_eq!(json["artifact"]["summary"], "Workflow audit spec drafted");
    assert_eq!(
        json["next_ready_task_entries"],
        json["next_ready_tasks"]
            .as_array()
            .expect("next ready array")
            .len()
    );
    assert!(
        json["next_ready_tasks"]
            .as_array()
            .expect("next ready tasks should be an array")
            .iter()
            .any(|task| task["node_id"] == "implement-workflow-audit"
                && task["role"] == "coder"
                && task["kind"] == "implementation"
                && task["spec_ref"] == "moat-spec/workflow-audit"),
        "expected implementation task to become ready: {json:#?}"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn moat_complete_task_rejects_missing_format_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "complete-task",
            "--history-path",
            "history.json",
            "--node-id",
            "node-1",
            "--format",
        ])
        .output()
        .expect("failed to run complete-task with missing format value");

    assert!(!output.status.success(), "missing format should fail");
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("missing value for moat complete-task --format"));
}

#[test]
fn moat_complete_task_rejects_duplicate_format() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "complete-task",
            "--history-path",
            "history.json",
            "--node-id",
            "node-1",
            "--format",
            "json",
            "--format",
            "text",
        ])
        .output()
        .expect("failed to run complete-task with duplicate format flag");

    assert!(!output.status.success(), "duplicate format should fail");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("duplicate moat complete-task --format")
    );
}

#[test]
fn moat_complete_task_rejects_unknown_format_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "complete-task",
            "--history-path",
            "history.json",
            "--node-id",
            "node-1",
            "--format",
            "yaml",
        ])
        .output()
        .expect("failed to run complete-task with unknown format value");

    assert!(!output.status.success(), "unknown format should fail");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unknown moat complete-task format: yaml")
    );
}

#[test]
fn cli_completes_claimed_moat_task() {
    let history_path = unique_history_path("complete-task-claimed");
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
        .expect("failed to seed complete-task history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let claim = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "claim-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
        ])
        .output()
        .expect("failed to claim task before completion");
    assert!(
        claim.status.success(),
        "{}",
        String::from_utf8_lossy(&claim.stderr)
    );

    let round_id = LocalMoatHistoryStore::open_existing(&history_path)
        .expect("history should reload")
        .entries()[0]
        .report
        .summary
        .round_id
        .to_string();

    let complete_review = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "complete-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
        ])
        .output()
        .expect("failed to complete review task");
    assert!(
        complete_review.status.success(),
        "{}",
        String::from_utf8_lossy(&complete_review.stderr)
    );

    assert_eq!(
        String::from_utf8_lossy(&complete_review.stdout),
        format!(
            "moat task completed\nround_id={round_id}\nnode_id=review\nprevious_state=in_progress\nnew_state=completed\nhistory_path={history_path_arg}\nartifact_recorded=false\nartifact_ref=<none>\nartifact_summary=<none>\nnext_ready_task_entries=1\nnext_ready_task=reviewer|evaluation|Evaluation|evaluation|<none>\n"
        )
    );

    let graph = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
        ])
        .output()
        .expect("failed to inspect task graph after completion");
    assert!(
        graph.status.success(),
        "{}",
        String::from_utf8_lossy(&graph.stderr)
    );
    assert!(String::from_utf8_lossy(&graph.stdout)
        .contains("node=reviewer|review|Review|review|completed|implementation|<none>\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn complete_task_artifact_records_handoff_and_prints_deterministic_fields() {
    let history_path = unique_history_path("complete-task-artifact-success");
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
        .expect("failed to seed complete-task artifact history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let claim = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "claim-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
        ])
        .output()
        .expect("failed to claim review before artifact completion");
    assert!(
        claim.status.success(),
        "{}",
        String::from_utf8_lossy(&claim.stderr)
    );

    let round_id = LocalMoatHistoryStore::open_existing(&history_path)
        .expect("history should reload")
        .entries()[0]
        .report
        .summary
        .round_id
        .to_string();

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "complete-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
            "--artifact-ref",
            "docs/review handoff.md",
            "--artifact-summary",
            "Reviewed | approved\nready for evaluation",
        ])
        .output()
        .expect("failed to complete review with artifact handoff");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!(
            "moat task completed\nround_id={round_id}\nnode_id=review\nprevious_state=in_progress\nnew_state=completed\nhistory_path={history_path_arg}\nartifact_recorded=true\nartifact_ref=docs/review handoff.md\nartifact_summary=Reviewed \\| approved\\nready for evaluation\nnext_ready_task_entries=1\nnext_ready_task=reviewer|evaluation|Evaluation|evaluation|<none>\n"
        )
    );

    let store = LocalMoatHistoryStore::open_existing(&history_path).expect("history should reload");
    let review_node = store.entries()[0]
        .report
        .control_plane
        .task_graph
        .nodes
        .iter()
        .find(|node| node.node_id == "review")
        .expect("review node should exist");
    assert_eq!(review_node.artifacts.len(), 1);
    assert_eq!(
        review_node.artifacts[0].artifact_ref,
        "docs/review handoff.md"
    );
    assert_eq!(
        review_node.artifacts[0].summary,
        "Reviewed | approved\nready for evaluation"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn moat_artifacts_json_prints_parseable_filtered_envelope() {
    let history_path = unique_history_path("artifacts-json");
    let history_path_arg = history_path.to_str().unwrap();

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("run moat round");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );
    update_history_task_node(&history_path, "implementation", |node| {
        node.insert("state".to_string(), Value::String("ready".to_string()));
        node.insert("artifacts".to_string(), Value::Array(Vec::new()));
    });

    let dispatch = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path_arg,
            "--node-id",
            "implementation",
            "--agent-id",
            "coder-json",
        ])
        .output()
        .expect("dispatch implementation task");
    assert!(
        dispatch.status.success(),
        "{}",
        String::from_utf8_lossy(&dispatch.stderr)
    );

    let complete = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "complete-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "implementation",
            "--artifact-ref",
            "commit:abc123",
            "--artifact-summary",
            "Implemented deterministic JSON artifacts export",
        ])
        .output()
        .expect("complete task with artifact");
    assert!(
        complete.status.success(),
        "{}",
        String::from_utf8_lossy(&complete.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "artifacts",
            "--history-path",
            history_path_arg,
            "--node-id",
            "implementation",
            "--format",
            "json",
        ])
        .output()
        .expect("export artifacts json");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("parse artifacts json");
    assert_eq!(json["type"], "moat_artifacts");
    assert_eq!(json["round_id"], latest_history_round_id(&history_path));
    assert_eq!(json["history_path"], history_path_arg);
    assert_eq!(json["artifact_entries"], 1);
    assert_eq!(json["artifacts"][0]["node_id"], "implementation");
    assert_eq!(json["artifacts"][0]["artifact_ref"], "commit:abc123");
    assert_eq!(
        json["artifacts"][0]["artifact_summary"],
        "Implemented deterministic JSON artifacts export"
    );
    assert_eq!(json["artifacts"][0]["node_title"], "Implementation");
    assert_eq!(json["artifacts"][0]["node_role"], "coder");
    assert_eq!(json["artifacts"][0]["node_kind"], "implementation");

    cleanup_history_path(&history_path);
}

#[test]
fn moat_artifacts_rejects_duplicate_format() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "artifacts",
            "--history-path",
            "history.json",
            "--format",
            "json",
            "--format",
            "text",
        ])
        .output()
        .expect("failed to run duplicate format artifacts command");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("duplicate option: --format"));
}

#[test]
fn moat_artifacts_prints_completed_task_artifact_handoffs_read_only() {
    let history_path = unique_history_path("moat-artifacts-inspect");
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
        .expect("failed to seed artifact history");
    Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "claim-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
        ])
        .output()
        .expect("failed to claim review task");

    let round_id = LocalMoatHistoryStore::open_existing(&history_path)
        .expect("history should reload")
        .entries()[0]
        .report
        .summary
        .round_id
        .to_string();

    let complete = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "complete-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
            "--artifact-ref",
            "docs/review handoff.md",
            "--artifact-summary",
            "Reviewed | approved\nready for evaluation",
        ])
        .output()
        .expect("failed to complete review with artifact");
    assert!(
        complete.status.success(),
        "{}",
        String::from_utf8_lossy(&complete.stderr)
    );

    let before = fs::read_to_string(&history_path).expect("history should exist");
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "artifacts",
            "--history-path",
            history_path_arg,
            "--round-id",
            &round_id,
            "--node-id",
            "review",
            "--contains",
            "handoff",
            "--limit",
            "1",
        ])
        .output()
        .expect("failed to inspect moat artifacts");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!(
            "moat artifacts\nround_id={round_id}\nartifact_entries=1\nartifact={round_id}|review|docs/review handoff.md|Reviewed \\| approved\\nready for evaluation\n"
        )
    );
    assert_eq!(fs::read_to_string(&history_path).unwrap(), before);

    cleanup_history_path(&history_path);
}

#[test]
fn moat_artifacts_filters_by_artifact_ref_and_summary_read_only() {
    let history_path = unique_history_path("moat-artifacts-field-filters");
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
        .expect("failed to seed artifact filter history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let claim = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "claim-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
        ])
        .output()
        .expect("failed to claim review task");
    assert!(
        claim.status.success(),
        "{}",
        String::from_utf8_lossy(&claim.stderr)
    );

    let round_id = LocalMoatHistoryStore::open_existing(&history_path)
        .expect("history should reload")
        .entries()[0]
        .report
        .summary
        .round_id
        .to_string();

    let complete = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "complete-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
            "--artifact-ref",
            "docs/review handoff.md",
            "--artifact-summary",
            "Reviewer approved release candidate",
        ])
        .output()
        .expect("failed to complete review with artifact");
    assert!(
        complete.status.success(),
        "{}",
        String::from_utf8_lossy(&complete.stderr)
    );

    let before = fs::read_to_string(&history_path).expect("history should exist");
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "artifacts",
            "--history-path",
            history_path_arg,
            "--artifact-ref",
            "review handoff",
            "--artifact-summary",
            "approved release",
        ])
        .output()
        .expect("failed to inspect moat artifacts with matching field filters");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!(
            "moat artifacts\nround_id={round_id}\nartifact_entries=1\nartifact={round_id}|review|docs/review handoff.md|Reviewer approved release candidate\n"
        )
    );
    assert_eq!(fs::read_to_string(&history_path).unwrap(), before);

    let missing = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "artifacts",
            "--history-path",
            history_path_arg,
            "--artifact-ref",
            "missing handoff",
            "--artifact-summary",
            "approved release",
        ])
        .output()
        .expect("failed to inspect moat artifacts with missing field filter");

    assert!(
        missing.status.success(),
        "{}",
        String::from_utf8_lossy(&missing.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&missing.stdout),
        format!("moat artifacts\nround_id={round_id}\nartifact_entries=0\n")
    );
    assert_eq!(fs::read_to_string(&history_path).unwrap(), before);

    cleanup_history_path(&history_path);
}

#[test]
fn cli_filters_moat_artifacts_by_node_role_kind_and_state() {
    let history_path = unique_history_path("artifacts-node-routing-filters");
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

    let claim_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "claim-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
        ])
        .output()
        .expect("failed to claim review task");
    assert!(
        claim_output.status.success(),
        "expected claim success, stderr was: {}",
        String::from_utf8_lossy(&claim_output.stderr)
    );

    let complete_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "complete-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
            "--artifact-ref",
            "docs/superpowers/plans/review.md",
            "--artifact-summary",
            "Review handoff ready for evaluation",
        ])
        .output()
        .expect("failed to complete review task with artifact");
    assert!(
        complete_output.status.success(),
        "expected complete success, stderr was: {}",
        String::from_utf8_lossy(&complete_output.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "artifacts",
            "--history-path",
            history_path_arg,
            "--role",
            "reviewer",
            "--kind",
            "review",
            "--state",
            "completed",
        ])
        .output()
        .expect("failed to inspect artifacts with node routing filters");

    assert!(
        output.status.success(),
        "expected artifacts success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("moat artifacts\n"));
    assert!(stdout.contains("artifact_entries=1\n"));
    assert!(stdout.contains(
        "|review|docs/superpowers/plans/review.md|Review handoff ready for evaluation\n"
    ));

    cleanup_history_path(&history_path);
}

#[test]
fn cli_filters_moat_artifacts_conjunctively_by_node_metadata() {
    let history_path = unique_history_path("artifacts-node-filter-conjunction");
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

    let claim_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "claim-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
        ])
        .output()
        .expect("failed to claim review task");
    assert!(
        claim_output.status.success(),
        "expected claim success, stderr was: {}",
        String::from_utf8_lossy(&claim_output.stderr)
    );

    let complete_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "complete-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
            "--artifact-ref",
            "docs/superpowers/plans/review.md",
            "--artifact-summary",
            "Review handoff ready for evaluation",
        ])
        .output()
        .expect("failed to complete review task with artifact");
    assert!(
        complete_output.status.success(),
        "expected complete success, stderr was: {}",
        String::from_utf8_lossy(&complete_output.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "artifacts",
            "--history-path",
            history_path_arg,
            "--role",
            "coder",
            "--kind",
            "review",
            "--state",
            "completed",
        ])
        .output()
        .expect("failed to inspect artifacts with mismatched node routing filters");

    assert!(
        output.status.success(),
        "expected artifacts success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|line| line.starts_with("artifact_entries="))
            .collect::<Vec<_>>(),
        vec!["artifact_entries=0"]
    );

    cleanup_history_path(&history_path);
}

#[test]
fn complete_task_artifact_rejects_unpaired_flags() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "complete-task",
            "--history-path",
            "history.json",
            "--node-id",
            "review",
            "--artifact-ref",
            "docs/review.md",
        ])
        .output()
        .expect("failed to run complete-task with unpaired artifact flag");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--artifact-ref and --artifact-summary must be supplied together"));
    assert!(stderr.contains(USAGE));
}

#[test]
fn complete_task_artifact_rejects_duplicate_artifact_ref() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "complete-task",
            "--history-path",
            "history.json",
            "--node-id",
            "review",
            "--artifact-ref",
            "first",
            "--artifact-ref",
            "second",
            "--artifact-summary",
            "summary",
        ])
        .output()
        .expect("failed to run complete-task with duplicate artifact ref");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("duplicate flag: --artifact-ref"));
    assert!(stderr.contains(USAGE));
}

#[test]
fn cli_releases_claimed_moat_task() {
    let history_path = unique_history_path("release-task-claimed");
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
        .expect("failed to seed release-task history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let claim = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "claim-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
        ])
        .output()
        .expect("failed to claim task before release");
    assert!(
        claim.status.success(),
        "{}",
        String::from_utf8_lossy(&claim.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "release-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
        ])
        .output()
        .expect("failed to run mdid-cli moat release-task");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("moat task released\n"));
    assert!(stdout.contains("round_id="));
    assert!(stdout.contains("node_id=review\n"));

    let graph = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
        ])
        .output()
        .expect("failed to inspect task graph after release");
    assert!(
        graph.status.success(),
        "{}",
        String::from_utf8_lossy(&graph.stderr)
    );
    assert!(String::from_utf8_lossy(&graph.stdout)
        .contains("node=reviewer|review|Review|review|ready|implementation|<none>\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn release_task_rejects_ready_node() {
    let history_path = unique_history_path("release-task-ready");
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
        .expect("failed to seed release-task history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "release-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
        ])
        .output()
        .expect("failed to run mdid-cli moat release-task for ready node");

    assert!(!output.status.success(), "releasing ready node should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed to release moat task"));
    assert!(stderr.contains("expected in_progress"));

    cleanup_history_path(&history_path);
}

#[test]
fn cli_blocks_claimed_moat_task() {
    let history_path = unique_history_path("block-task-claimed");
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
        .expect("failed to seed block-task history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let claim = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "claim-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
        ])
        .output()
        .expect("failed to claim task before blocking");
    assert!(
        claim.status.success(),
        "{}",
        String::from_utf8_lossy(&claim.stderr)
    );

    let round_id = LocalMoatHistoryStore::open_existing(&history_path)
        .expect("history should reload")
        .entries()[0]
        .report
        .summary
        .round_id
        .to_string();

    let block_review = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "block-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
        ])
        .output()
        .expect("failed to block review task");
    assert!(
        block_review.status.success(),
        "{}",
        String::from_utf8_lossy(&block_review.stderr)
    );

    assert_eq!(
        String::from_utf8_lossy(&block_review.stdout),
        format!(
            "moat task blocked\nround_id={round_id}\nnode_id=review\nprevious_state=in_progress\nnew_state=blocked\nhistory_path={history_path_arg}\n"
        )
    );

    let graph = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
        ])
        .output()
        .expect("failed to inspect task graph after blocking");
    assert!(
        graph.status.success(),
        "{}",
        String::from_utf8_lossy(&graph.stderr)
    );
    assert!(String::from_utf8_lossy(&graph.stdout)
        .contains("node=reviewer|review|Review|review|blocked|implementation|<none>\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn cli_unblocks_blocked_moat_task_to_ready() {
    let history_path = unique_history_path("unblock-task-blocked");
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
        .expect("failed to seed unblock-task history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let claim = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "claim-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
        ])
        .output()
        .expect("failed to claim task before unblocking");
    assert!(
        claim.status.success(),
        "{}",
        String::from_utf8_lossy(&claim.stderr)
    );

    let round_id = LocalMoatHistoryStore::open_existing(&history_path)
        .expect("history should reload")
        .entries()[0]
        .report
        .summary
        .round_id
        .to_string();

    let block = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "block-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
        ])
        .output()
        .expect("failed to block task before unblocking");
    assert!(
        block.status.success(),
        "{}",
        String::from_utf8_lossy(&block.stderr)
    );

    let unblock = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "unblock-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
        ])
        .output()
        .expect("failed to unblock review task");
    assert!(
        unblock.status.success(),
        "{}",
        String::from_utf8_lossy(&unblock.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&unblock.stdout),
        format!(
            "moat task unblocked\nround_id={round_id}\nnode_id=review\nprevious_state=blocked\nnew_state=ready\nhistory_path={history_path_arg}\n"
        )
    );

    let graph = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
        ])
        .output()
        .expect("failed to inspect task graph after unblocking");
    assert!(
        graph.status.success(),
        "{}",
        String::from_utf8_lossy(&graph.stderr)
    );
    assert!(String::from_utf8_lossy(&graph.stdout)
        .contains("node=reviewer|review|Review|review|ready|implementation|<none>\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn cli_rejects_unblock_task_when_task_is_not_blocked() {
    let history_path = unique_history_path("unblock-task-ready");
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
        .expect("failed to seed unblock-task history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "unblock-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
        ])
        .output()
        .expect("failed to run mdid-cli moat unblock-task for ready node");

    assert!(
        !output.status.success(),
        "unblocking ready node should fail"
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "error: node 'review' is ready, expected blocked\n"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn cli_block_task_rejects_unclaimed_ready_task() {
    let history_path = unique_history_path("block-task-unclaimed");
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
        .expect("failed to seed block-task history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "block-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
        ])
        .output()
        .expect("failed to run mdid-cli moat block-task for ready node");

    assert!(!output.status.success(), "blocking ready node should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("moat task node is not in progress"),
        "{stderr}"
    );
    assert!(stderr.contains("review"), "{stderr}");

    cleanup_history_path(&history_path);
}

#[test]
fn cli_complete_task_rejects_unclaimed_ready_task() {
    let history_path = unique_history_path("complete-task-ready-rejected");
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
        .expect("failed to seed complete-task history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "complete-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "market_scan",
        ])
        .output()
        .expect("failed to run mdid-cli moat complete-task for ready node");

    assert!(
        !output.status.success(),
        "completing ready node should fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("moat task node is not in progress"),
        "{stderr}"
    );
    assert!(stderr.contains("market_scan"), "{stderr}");

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
fn cli_ready_tasks_requires_completed_dependency_artifacts() {
    let history_path = write_history_with_artifact_routing_tasks();
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "ready-tasks",
            "--history-path",
            history_path_arg,
            "--requires-artifacts",
        ])
        .output()
        .expect("failed to run mdid-cli moat ready-tasks with artifact routing filter");

    assert!(
        output.status.success(),
        "ready-tasks failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("ready_with_artifact"),
        "stdout was: {stdout}"
    );
    assert!(
        !stdout.contains("ready_without_artifact"),
        "stdout was: {stdout}"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn moat_ready_tasks_rejects_duplicate_requires_artifacts_filter() {
    let history_path = unique_history_path("ready-tasks-requires-artifacts-duplicate");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "ready-tasks",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--requires-artifacts",
            "--requires-artifacts",
        ])
        .output()
        .expect("failed to run mdid-cli moat ready-tasks with duplicate requires-artifacts");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("duplicate flag: --requires-artifacts")
    );
    assert!(!history_path.exists());
}

#[test]
fn moat_ready_tasks_filters_by_dependency_node_id() {
    let history_path = unique_history_path("ready-tasks-depends-on");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let setup_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
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
        setup_output.status.success(),
        "setup round failed: {}",
        String::from_utf8_lossy(&setup_output.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "ready-tasks",
            "--history-path",
            history_path_arg,
            "--depends-on",
            "implementation",
        ])
        .output()
        .expect("failed to run mdid-cli moat ready-tasks with dependency filter");

    assert!(
        output.status.success(),
        "ready-tasks failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("moat ready tasks\n"));
    assert!(stdout.contains("ready_task=reviewer|review|review|Review|<none>\n"));
    assert!(!stdout.contains("ready_task=planner|market_scan|"));

    cleanup_history_path(&history_path);
}

#[test]
fn moat_ready_tasks_filters_to_nodes_without_dependencies() {
    let history_path = unique_history_path("ready-tasks-no-dependencies");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let setup_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
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
        setup_output.status.success(),
        "setup round failed: {}",
        String::from_utf8_lossy(&setup_output.stderr)
    );
    make_history_task_node_ready(&history_path, "market_scan");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "ready-tasks",
            "--history-path",
            history_path_arg,
            "--no-dependencies",
        ])
        .output()
        .expect("failed to run mdid-cli moat ready-tasks with no-dependencies filter");

    assert!(
        output.status.success(),
        "ready-tasks failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("moat ready tasks\n"));
    assert!(stdout.contains("ready_task=planner|market_scan|market_scan|Market Scan|<none>\n"));
    assert!(!stdout.contains("ready_task=reviewer|review|"));

    cleanup_history_path(&history_path);
}

#[test]
fn moat_ready_tasks_rejects_missing_format_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "ready-tasks", "--format"])
        .output()
        .expect("failed to run mdid-cli moat ready-tasks with missing format value");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("missing value for moat ready-tasks --format\n{}\n", USAGE)
    );
}

#[test]
fn moat_ready_tasks_rejects_duplicate_format() {
    let history_path = unique_history_path("ready-tasks-format-duplicate");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "ready-tasks",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--format",
            "json",
            "--format",
            "text",
        ])
        .output()
        .expect("failed to run mdid-cli moat ready-tasks with duplicate format");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("duplicate moat ready-tasks --format\n{}\n", USAGE)
    );
    assert!(!history_path.exists());
}

#[test]
fn moat_ready_tasks_rejects_unknown_format() {
    let history_path = unique_history_path("ready-tasks-format-unknown");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "ready-tasks",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--format",
            "yaml",
        ])
        .output()
        .expect("failed to run mdid-cli moat ready-tasks with unknown format");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("unknown moat ready-tasks format: yaml\n{}\n", USAGE)
    );
    assert!(!history_path.exists());
}

#[test]
fn moat_ready_tasks_rejects_missing_dependency_filter_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "ready-tasks", "--depends-on"])
        .output()
        .expect("failed to run mdid-cli moat ready-tasks with missing dependency value");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("missing value for --depends-on\n{}\n", USAGE)
    );
}

#[test]
fn moat_ready_tasks_rejects_duplicate_dependency_filter() {
    let history_path = unique_history_path("ready-tasks-depends-on-duplicate");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "ready-tasks",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--depends-on",
            "market_scan",
            "--depends-on",
            "competitor_analysis",
        ])
        .output()
        .expect("failed to run mdid-cli moat ready-tasks with duplicate dependency filter");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("duplicate flag: --depends-on"));
    assert!(!history_path.exists());
}

#[test]
fn moat_ready_tasks_rejects_duplicate_no_dependencies_filter() {
    let history_path = unique_history_path("ready-tasks-no-dependencies-duplicate");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "ready-tasks",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--no-dependencies",
            "--no-dependencies",
        ])
        .output()
        .expect("failed to run mdid-cli moat ready-tasks with duplicate no-dependencies filter");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("duplicate flag: --no-dependencies"));
    assert!(!history_path.exists());
}

#[test]
fn cli_filters_ready_tasks_by_exact_node_id() {
    let history_path = unique_history_path("ready-tasks-node-id-filter");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let round_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--strategy-candidates",
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

    make_history_task_node_ready(&history_path, "lockin_analysis");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "ready-tasks",
            "--history-path",
            history_path_arg,
            "--node-id",
            "lockin_analysis",
        ])
        .output()
        .expect("failed to run mdid-cli moat ready-tasks with node id filter");

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
            "ready_task=planner|lock_in_analysis|lockin_analysis|Lock-In Analysis|<none>\n",
        )
    );

    cleanup_history_path(&history_path);
}

#[test]
fn cli_filters_ready_tasks_by_title_contains() {
    let history_path = unique_history_path("ready-tasks-title-contains");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let round_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to run mdid-cli moat round with history path");
    assert!(
        round_output.status.success(),
        "moat round failed: {}",
        String::from_utf8_lossy(&round_output.stderr)
    );

    make_workflow_audit_spec_task_ready(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "ready-tasks",
            "--history-path",
            history_path_arg,
            "--title-contains",
            "workflow audit",
        ])
        .output()
        .expect("failed to run mdid-cli moat ready-tasks with title filter");

    assert!(
        output.status.success(),
        "ready-tasks failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "moat ready tasks\n",
            "ready_task_entries=1\n",
            "ready_task=planner|spec_planning|spec-workflow-audit|Create spec for workflow audit|moat-spec/workflow-audit\n",
        )
    );

    cleanup_history_path(&history_path);
}

#[test]
fn cli_ready_tasks_title_filter_succeeds_with_no_matches() {
    let history_path = unique_history_path("ready-tasks-title-no-match");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let round_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to run mdid-cli moat round with history path");
    assert!(
        round_output.status.success(),
        "moat round failed: {}",
        String::from_utf8_lossy(&round_output.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "ready-tasks",
            "--history-path",
            history_path_arg,
            "--title-contains",
            "nonexistent title substring",
        ])
        .output()
        .expect("failed to run mdid-cli moat ready-tasks with missing title filter");

    assert!(
        output.status.success(),
        "ready-tasks failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!("moat ready tasks\n", "ready_task_entries=0\n")
    );

    cleanup_history_path(&history_path);
}

#[test]
fn cli_dispatch_next_requires_completed_dependency_artifacts() {
    let history_path = write_history_with_artifact_routing_tasks();
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path_arg,
            "--requires-artifacts",
            "--dry-run",
        ])
        .output()
        .expect("failed to run mdid-cli moat dispatch-next with artifact routing filter");

    assert!(
        output.status.success(),
        "dispatch-next failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("ready_with_artifact"),
        "stdout was: {stdout}"
    );
    assert!(
        !stdout.contains("ready_without_artifact"),
        "stdout was: {stdout}"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn moat_dispatch_next_rejects_duplicate_requires_artifacts_filter() {
    let history_path = unique_history_path("dispatch-next-requires-artifacts-duplicate");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--requires-artifacts",
            "--requires-artifacts",
            "--dry-run",
        ])
        .output()
        .expect("failed to run mdid-cli moat dispatch-next with duplicate requires-artifacts");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("duplicate flag: --requires-artifacts")
    );
    assert!(!history_path.exists());
}

#[test]
fn moat_dispatch_next_filters_by_exact_node_id() {
    let history_path = unique_history_path("dispatch-next-node-id");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed dispatch-next node-id history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );
    make_workflow_audit_spec_task_ready(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path_arg,
            "--node-id",
            "spec-workflow-audit",
            "--dry-run",
        ])
        .output()
        .expect("failed to dry-run dispatch-next with node-id filter");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("dry_run=true\n"));
    assert!(stdout.contains("claimed=false\n"));
    assert!(stdout.contains("node_id=spec-workflow-audit\n"));
    assert!(stdout.contains("spec_ref=moat-spec/workflow-audit\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn cli_dispatch_next_text_output_includes_agent_id_attribution() {
    let history_path = unique_history_path("dispatch-agent-id-text");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let round_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to create persisted moat round for dispatch agent id test");
    assert!(
        round_output.status.success(),
        "expected round success, stderr was: {}",
        String::from_utf8_lossy(&round_output.stderr)
    );
    update_history_task_node(&history_path, "implementation", |implementation_node| {
        implementation_node.insert("state".to_string(), Value::String("ready".to_string()));
        implementation_node.insert("depends_on".to_string(), Value::Array(Vec::new()));
    });

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path_arg,
            "--role",
            "coder",
            "--agent-id",
            "coder-7",
        ])
        .output()
        .expect("failed to run mdid-cli moat dispatch-next with agent id");

    assert!(
        output.status.success(),
        "expected dispatch success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("agent_id=coder-7\n"),
        "stdout was: {stdout}"
    );
    assert!(
        stdout.contains("assigned_agent_id=coder-7\n"),
        "stdout was: {stdout}"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn cli_dispatch_next_json_output_includes_agent_id_attribution() {
    let history_path = unique_history_path("dispatch-agent-id-json");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let round_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to create persisted moat round for dispatch agent id json test");
    assert!(
        round_output.status.success(),
        "expected round success, stderr was: {}",
        String::from_utf8_lossy(&round_output.stderr)
    );
    update_history_task_node(&history_path, "implementation", |implementation_node| {
        implementation_node.insert("state".to_string(), Value::String("ready".to_string()));
        implementation_node.insert("depends_on".to_string(), Value::Array(Vec::new()));
    });

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path_arg,
            "--role",
            "coder",
            "--agent-id",
            "coder-7",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run mdid-cli moat dispatch-next json with agent id");

    assert!(
        output.status.success(),
        "expected dispatch success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("dispatch output should be json");
    assert_eq!(payload["agent_id"], "coder-7");
    assert_eq!(payload["assigned_agent_id"], "coder-7");

    cleanup_history_path(&history_path);
}

#[test]
fn dispatch_next_persists_assigned_agent_id() {
    let history_path = unique_history_path("dispatch-assigned-agent-id");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let round_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to create persisted moat round for dispatch assigned agent id test");
    assert!(
        round_output.status.success(),
        "{}",
        String::from_utf8_lossy(&round_output.stderr)
    );
    update_history_task_node(&history_path, "implementation", |implementation_node| {
        implementation_node.insert("state".to_string(), Value::String("ready".to_string()));
        implementation_node.insert("depends_on".to_string(), Value::Array(Vec::new()));
    });

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path_arg,
            "--role",
            "coder",
            "--agent-id",
            "coder-7",
        ])
        .output()
        .expect("failed to dispatch with assigned agent id");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let graph = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--node-id",
            "implementation",
        ])
        .output()
        .expect("failed to inspect graph after dispatch");
    assert!(
        graph.status.success(),
        "{}",
        String::from_utf8_lossy(&graph.stderr)
    );
    assert!(
        String::from_utf8_lossy(&graph.stdout).contains(
            "node=coder|implementation|Implementation|implementation|in_progress|<none>|<none>\n"
        ),
        "graph stdout was: {}",
        String::from_utf8_lossy(&graph.stdout)
    );
    assert!(
        String::from_utf8_lossy(&graph.stdout)
            .contains("assigned_agent_id=implementation|coder-7\n"),
        "graph stdout was: {}",
        String::from_utf8_lossy(&graph.stdout)
    );

    cleanup_history_path(&history_path);
}

#[test]
fn claim_task_persists_assigned_agent_id() {
    let history_path = unique_history_path("claim-assigned-agent-id");
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
        .expect("failed to create persisted moat round for claim assigned agent id test");
    assert!(
        round_output.status.success(),
        "{}",
        String::from_utf8_lossy(&round_output.stderr)
    );
    update_history_task_node(&history_path, "implementation", |implementation_node| {
        implementation_node.insert("state".to_string(), Value::String("ready".to_string()));
        implementation_node.insert("depends_on".to_string(), Value::Array(Vec::new()));
    });

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "claim-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "implementation",
            "--agent-id",
            "planner-2",
        ])
        .output()
        .expect("failed to claim task with assigned agent id");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("assigned_agent_id=planner-2\n"));

    let graph = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--node-id",
            "implementation",
        ])
        .output()
        .expect("failed to inspect graph after claim");
    assert!(
        graph.status.success(),
        "{}",
        String::from_utf8_lossy(&graph.stderr)
    );
    assert!(
        String::from_utf8_lossy(&graph.stdout).contains(
            "node=coder|implementation|Implementation|implementation|in_progress|<none>|<none>\n"
        ),
        "graph stdout was: {}",
        String::from_utf8_lossy(&graph.stdout)
    );
    assert!(
        String::from_utf8_lossy(&graph.stdout)
            .contains("assigned_agent_id=implementation|planner-2\n"),
        "graph stdout was: {}",
        String::from_utf8_lossy(&graph.stdout)
    );

    cleanup_history_path(&history_path);
}

#[test]
fn moat_dispatch_next_json_dry_run_prints_parseable_envelope() {
    let history_path = unique_history_path("dispatch-next-json-dry-run");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed dispatch-next json history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );
    make_workflow_audit_spec_task_ready(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path_arg,
            "--dry-run",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to dry-run dispatch-next json");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be json");
    assert_eq!(json["type"], "moat_dispatch_next");
    assert_eq!(json["dry_run"], true);
    assert_eq!(json["claimed"], false);
    assert_eq!(json["node_id"], "spec-workflow-audit");
    assert_eq!(json["role"], "planner");
    assert_eq!(json["kind"], "spec_planning");
    assert_eq!(json["title"], "Create spec for workflow audit");
    assert_eq!(json["dependencies"].as_array().unwrap().len(), 0);
    assert_eq!(json["spec_ref"], "moat-spec/workflow-audit");
    assert!(json["complete_command"]
        .as_str()
        .unwrap()
        .contains("mdid-cli moat complete-task"));
    assert!(json.get("previous_state").is_none());
    assert!(json.get("new_state").is_none());

    cleanup_history_path(&history_path);
}

#[test]
fn moat_dispatch_next_json_claim_includes_state_transition() {
    let history_path = unique_history_path("dispatch-next-json-claim");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed dispatch-next json claim history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );
    make_workflow_audit_spec_task_ready(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path_arg,
            "--agent-id",
            "dispatcher json 'quoted'",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to claim dispatch-next json");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be json");
    assert_eq!(json["type"], "moat_dispatch_next");
    assert_eq!(json["dry_run"], false);
    assert_eq!(json["claimed"], true);
    assert_eq!(json["agent_id"], "dispatcher json 'quoted'");
    assert_eq!(json["assigned_agent_id"], "dispatcher json 'quoted'");
    assert_eq!(json["node_id"], "spec-workflow-audit");
    assert_eq!(json["role"], "planner");
    assert_eq!(json["kind"], "spec_planning");
    assert_eq!(json["title"], "Create spec for workflow audit");
    assert_eq!(json["dependencies"], serde_json::json!([]));
    assert_eq!(json["spec_ref"], "moat-spec/workflow-audit");
    assert_eq!(json["previous_state"], "ready");
    assert_eq!(json["new_state"], "in_progress");
    assert!(json["lease_seconds"].is_number());
    let complete_command = json["complete_command"]
        .as_str()
        .expect("complete_command should be string");
    assert!(complete_command.contains("mdid-cli moat complete-task"));
    assert!(complete_command.contains(r#"--agent-id 'dispatcher json '\''quoted'\'''"#));
    assert!(!complete_command.contains("--agent-id dispatcher json"));

    let complete_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "complete-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "spec-workflow-audit",
            "--artifact-ref",
            "artifact://workflow audit",
            "--artifact-summary",
            "completed by dispatcher json 'quoted'",
            "--agent-id",
            "dispatcher json 'quoted'",
        ])
        .output()
        .expect("failed to run emitted complete-task intent");
    assert!(
        complete_output.status.success(),
        "complete-task with dispatch agent id failed: {}",
        String::from_utf8_lossy(&complete_output.stderr)
    );

    let events_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-events",
            "--history-path",
            history_path_arg,
            "--node-id",
            "spec-workflow-audit",
            "--action",
            "complete",
            "--agent-id",
            "dispatcher json 'quoted'",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to inspect completion event");
    assert!(
        events_output.status.success(),
        "{}",
        String::from_utf8_lossy(&events_output.stderr)
    );
    let events_json: Value = serde_json::from_slice(&events_output.stdout).expect("events json");
    assert_eq!(events_json["task_event_entries"], 1);
    assert_eq!(
        events_json["events"][0]["agent_id"],
        "dispatcher json 'quoted'"
    );

    let artifacts_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "artifacts",
            "--history-path",
            history_path_arg,
            "--node-id",
            "spec-workflow-audit",
            "--artifact-ref",
            "artifact://workflow audit",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to inspect completion artifact");
    assert!(
        artifacts_output.status.success(),
        "{}",
        String::from_utf8_lossy(&artifacts_output.stderr)
    );
    let artifacts_json: Value =
        serde_json::from_slice(&artifacts_output.stdout).expect("artifacts json");
    assert_eq!(artifacts_json["artifact_entries"], 1);

    let ready_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "ready-tasks", "--history-path", history_path_arg])
        .output()
        .expect("failed to run ready-tasks after json claim");
    assert!(
        ready_output.status.success(),
        "{}",
        String::from_utf8_lossy(&ready_output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&ready_output.stdout).contains("ready_task_entries=0\n"),
        "stdout was: {}",
        String::from_utf8_lossy(&ready_output.stdout)
    );

    cleanup_history_path(&history_path);
}

#[test]
fn cli_dispatch_next_filters_ready_task_by_dependency() {
    let history_path = unique_history_path("dispatch-next-depends-on-filter");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let round_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history");
    assert!(
        round_output.status.success(),
        "expected seed round success, stderr was: {}",
        String::from_utf8_lossy(&round_output.stderr)
    );
    update_history_task_node(&history_path, "market_scan", |market_node| {
        market_node.insert(
            "node_id".to_string(),
            Value::String("market-scan".to_string()),
        );
        market_node.insert("state".to_string(), Value::String("completed".to_string()));
    });
    update_history_task_node(&history_path, "competitor_analysis", |competitor_node| {
        competitor_node.insert(
            "node_id".to_string(),
            Value::String("competitor-analysis".to_string()),
        );
        competitor_node.insert("state".to_string(), Value::String("ready".to_string()));
        competitor_node.insert(
            "depends_on".to_string(),
            Value::Array(vec![Value::String("market-scan".to_string())]),
        );
    });

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path_arg,
            "--depends-on",
            "market-scan",
            "--dry-run",
        ])
        .output()
        .expect("failed to run mdid-cli moat dispatch-next with dependency filter");

    assert!(
        output.status.success(),
        "expected success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("node_id=competitor-analysis\n"),
        "expected competitor-analysis dispatch, stdout was: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("dependencies=market-scan\n"),
        "expected persisted dependency output, stdout was: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    cleanup_history_path(&history_path);
}

#[test]
fn cli_dispatch_next_filters_ready_task_to_nodes_without_dependencies() {
    let history_path = unique_history_path("dispatch-next-no-dependencies-filter");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let round_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history");
    assert!(
        round_output.status.success(),
        "expected seed round success, stderr was: {}",
        String::from_utf8_lossy(&round_output.stderr)
    );
    update_history_task_node(&history_path, "market_scan", |market_node| {
        market_node.insert(
            "node_id".to_string(),
            Value::String("market-scan".to_string()),
        );
        market_node.insert("state".to_string(), Value::String("ready".to_string()));
        market_node.insert("depends_on".to_string(), Value::Array(Vec::new()));
    });
    update_history_task_node(&history_path, "competitor_analysis", |competitor_node| {
        competitor_node.insert(
            "node_id".to_string(),
            Value::String("competitor-analysis".to_string()),
        );
        competitor_node.insert("state".to_string(), Value::String("ready".to_string()));
        competitor_node.insert(
            "depends_on".to_string(),
            Value::Array(vec![Value::String("market-scan".to_string())]),
        );
    });

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path_arg,
            "--no-dependencies",
            "--dry-run",
        ])
        .output()
        .expect("failed to run mdid-cli moat dispatch-next with no-dependencies filter");

    assert!(
        output.status.success(),
        "expected success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("node_id=market-scan\n"),
        "expected market-scan dispatch, stdout was: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("dependencies=<none>\n"),
        "expected empty dependency output, stdout was: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    cleanup_history_path(&history_path);
}

#[test]
fn cli_dispatch_next_filters_ready_task_by_exact_spec_ref() {
    let history_path = unique_history_path("dispatch-next-spec-ref");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed dispatch-next spec-ref history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );
    make_workflow_audit_implementation_task_ready(&history_path);
    let round_id = latest_history_round_id(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path_arg,
            "--kind",
            "implementation",
            "--spec-ref",
            "moat-spec/workflow-audit",
            "--dry-run",
        ])
        .output()
        .expect("failed to dry-run dispatch-next with spec-ref filter");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!(
            "moat dispatch next\n\
dry_run=true\n\
claimed=false\n\
agent_id=<none>\n\
assigned_agent_id=<none>\n\
round_id={round_id}\n\
node_id=task-implementation\n\
role=coder\n\
kind=implementation\n\
title=Implement workflow audit\n\
dependencies=<none>\n\
spec_ref=moat-spec/workflow-audit\n\
complete_command=mdid-cli moat complete-task --history-path '{}' --round-id '{}' --node-id 'task-implementation' --artifact-ref '<artifact-ref>' --artifact-summary '<artifact-summary>'\n",
            history_path.display(),
            round_id
        )
    );

    cleanup_history_path(&history_path);
}

#[test]
fn cli_dispatch_next_filters_ready_task_by_title_substring() {
    let history_path = unique_history_path("dispatch-next-title-contains");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed dispatch-next title history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );
    make_workflow_audit_implementation_task_ready(&history_path);
    update_history_task_node(
        &history_path,
        "task-implementation",
        |implementation_node| {
            implementation_node.insert(
                "node_id".to_string(),
                Value::String("moat-implementation-workflow-audit".to_string()),
            );
        },
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path_arg,
            "--title-contains",
            "workflow audit",
            "--dry-run",
        ])
        .output()
        .expect("failed to dry-run dispatch-next with title filter");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("dry_run=true\n"));
    assert!(stdout.contains("claimed=false\n"));
    assert!(stdout.contains("node_id=moat-implementation-workflow-audit\n"));
    assert!(stdout.contains("title=Implement workflow audit\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn cli_dispatch_next_title_filter_reports_no_matching_ready_task() {
    let history_path = unique_history_path("dispatch-next-title-no-match");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed dispatch-next title no-match history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path_arg,
            "--title-contains",
            "nonexistent routing title",
            "--dry-run",
        ])
        .output()
        .expect("failed to dry-run dispatch-next with no matching title");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("no ready moat task matched dispatch filters"));

    cleanup_history_path(&history_path);
}

#[test]
fn moat_dispatch_next_dry_run_prints_first_ready_task_without_mutating_history() {
    let history_path = unique_history_path("dispatch-next-dry-run");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed dispatch-next history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );
    make_workflow_audit_spec_task_ready(&history_path);
    let round_id = latest_history_round_id(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path_arg,
            "--dry-run",
        ])
        .output()
        .expect("failed to dry-run dispatch-next");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("moat dispatch next\n"));
    assert!(stdout.contains("dry_run=true\n"));
    assert!(stdout.contains("claimed=false\n"));
    assert!(stdout.contains("node_id=spec-workflow-audit\n"));
    assert!(stdout.contains("role=planner\n"));
    assert!(stdout.contains("kind=spec_planning\n"));
    assert!(stdout.contains("title=Create spec for workflow audit\n"));
    assert!(stdout.contains("dependencies=<none>\n"));
    assert!(stdout.contains("spec_ref=moat-spec/workflow-audit\n"));
    assert!(stdout.contains(&format!("complete_command=mdid-cli moat complete-task --history-path '{}' --round-id '{}' --node-id 'spec-workflow-audit' --artifact-ref '<artifact-ref>' --artifact-summary '<artifact-summary>'\n", history_path.display(), round_id)));

    let ready_after = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "ready-tasks", "--history-path", history_path_arg])
        .output()
        .expect("failed to inspect ready tasks after dry-run dispatch");
    assert!(
        ready_after.status.success(),
        "{}",
        String::from_utf8_lossy(&ready_after.stderr)
    );
    assert!(String::from_utf8_lossy(&ready_after.stdout).contains("ready_task=planner|spec_planning|spec-workflow-audit|Create spec for workflow audit|moat-spec/workflow-audit\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn moat_dispatch_next_claims_selected_ready_task() {
    let history_path = unique_history_path("dispatch-next-claim");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed dispatch-next claim history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );
    make_workflow_audit_spec_task_ready(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "dispatch-next", "--history-path", history_path_arg])
        .output()
        .expect("failed to dispatch and claim next task");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("dry_run=false\n"));
    assert!(stdout.contains("claimed=true\n"));
    assert!(stdout.contains("previous_state=ready\n"));
    assert!(stdout.contains("new_state=in_progress\n"));
    assert!(stdout.contains("node_id=spec-workflow-audit\n"));

    let ready_after = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "ready-tasks", "--history-path", history_path_arg])
        .output()
        .expect("failed to inspect ready tasks after dispatch claim");
    assert!(
        ready_after.status.success(),
        "{}",
        String::from_utf8_lossy(&ready_after.stderr)
    );
    assert!(String::from_utf8_lossy(&ready_after.stdout).contains("ready_task_entries=0\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn moat_dispatch_next_complete_command_targets_explicit_round_id() {
    let history_path = unique_history_path("dispatch-next-explicit-round");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed dispatch-next explicit round history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );
    make_workflow_audit_spec_task_ready(&history_path);
    let round_id = latest_history_round_id(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path_arg,
            "--round-id",
            &round_id,
            "--dry-run",
        ])
        .output()
        .expect("failed to dispatch next task for explicit round");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(&format!("round_id={round_id}\n")));
    assert!(stdout.contains(&format!("complete_command=mdid-cli moat complete-task --history-path '{}' --round-id '{}' --node-id 'spec-workflow-audit' --artifact-ref '<artifact-ref>' --artifact-summary '<artifact-summary>'\n", history_path.display(), round_id)));

    cleanup_history_path(&history_path);
}

#[test]
fn moat_dispatch_next_filters_by_role_and_kind() {
    let history_path = unique_history_path("dispatch-next-filter");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed dispatch-next filter history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );
    make_workflow_audit_spec_task_ready(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path_arg,
            "--role",
            "planner",
            "--kind",
            "spec_planning",
            "--dry-run",
        ])
        .output()
        .expect("failed to dispatch next task with role/kind filters");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("node_id=spec-workflow-audit\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn moat_dispatch_next_fails_when_no_ready_task_matches() {
    let history_path = unique_history_path("dispatch-next-no-match");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed dispatch-next no-match history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path_arg,
            "--role",
            "coder",
        ])
        .output()
        .expect("failed to run dispatch-next no-match case");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "no ready moat task matched dispatch filters\n"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn moat_dispatch_next_node_id_non_match_does_not_claim_ready_task() {
    let history_path = unique_history_path("dispatch-next-node-id-no-match");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed dispatch-next node-id no-match history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );
    make_workflow_audit_spec_task_ready(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path_arg,
            "--node-id",
            "implementation-workflow-audit",
        ])
        .output()
        .expect("failed to run dispatch-next with non-matching node-id");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "no ready moat task matched dispatch filters\n"
    );
    let ready_after = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "ready-tasks", "--history-path", history_path_arg])
        .output()
        .expect("failed to inspect ready tasks after node-id no-match");
    assert!(
        ready_after.status.success(),
        "{}",
        String::from_utf8_lossy(&ready_after.stderr)
    );
    assert!(String::from_utf8_lossy(&ready_after.stdout).contains("ready_task=planner|spec_planning|spec-workflow-audit|Create spec for workflow audit|moat-spec/workflow-audit\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn moat_dispatch_next_spec_ref_non_match_does_not_claim_ready_task() {
    let history_path = unique_history_path("dispatch-next-spec-ref-no-match");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed dispatch-next spec-ref no-match history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );
    make_workflow_audit_implementation_task_ready(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path_arg,
            "--spec-ref",
            "moat-spec/missing",
        ])
        .output()
        .expect("failed to run dispatch-next with non-matching spec-ref");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "no ready moat task matched dispatch filters\n"
    );

    let ready_after = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "ready-tasks",
            "--history-path",
            history_path_arg,
            "--spec-ref",
            "moat-spec/workflow-audit",
        ])
        .output()
        .expect("failed to inspect ready tasks after spec-ref no-match");
    assert!(
        ready_after.status.success(),
        "{}",
        String::from_utf8_lossy(&ready_after.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&ready_after.stdout),
        concat!(
            "moat ready tasks\n",
            "ready_task_entries=1\n",
            "ready_task=coder|implementation|task-implementation|Implement workflow audit|moat-spec/workflow-audit\n",
        )
    );

    cleanup_history_path(&history_path);
}

#[test]
fn moat_dispatch_next_rejects_missing_node_id_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            "history.json",
            "--node-id",
        ])
        .output()
        .expect("failed to run dispatch-next with missing node-id value");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("missing value for moat dispatch-next --node-id\n{USAGE}\n")
    );
}

#[test]
fn moat_dispatch_next_rejects_duplicate_node_id_filter() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            "history.json",
            "--node-id",
            "spec-workflow-audit",
            "--node-id",
            "other-node",
        ])
        .output()
        .expect("failed to run dispatch-next with duplicate node-id");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("duplicate moat dispatch-next --node-id\n{USAGE}\n")
    );
}

#[test]
fn moat_dispatch_next_rejects_missing_format_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            "history.json",
            "--format",
        ])
        .output()
        .expect("failed to run dispatch-next with missing format value");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("missing value for --format\n{USAGE}\n")
    );
}

#[test]
fn moat_dispatch_next_rejects_duplicate_format_flag() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            "history.json",
            "--format",
            "json",
            "--format",
            "text",
        ])
        .output()
        .expect("failed to run dispatch-next with duplicate format");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("duplicate flag: --format\n{USAGE}\n")
    );
}

#[test]
fn moat_dispatch_next_rejects_invalid_format_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            "history.json",
            "--format",
            "yaml",
        ])
        .output()
        .expect("failed to run dispatch-next with invalid format");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("unknown moat dispatch-next format: yaml\n{USAGE}\n")
    );
}

#[test]
fn moat_dispatch_next_rejects_duplicate_dry_run_flag() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            "history.json",
            "--dry-run",
            "--dry-run",
        ])
        .output()
        .expect("failed to run dispatch-next with duplicate dry-run");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("duplicate flag: --dry-run\n{USAGE}\n")
    );
}

#[test]
fn moat_dispatch_next_rejects_missing_history_without_creating_it() {
    let history_path = unique_history_path("dispatch-next-missing");
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--dry-run",
        ])
        .output()
        .expect("failed to run dispatch-next with missing history");

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed to open moat history store"));
    assert!(stderr.contains("moat history file does not exist"));
    assert!(!history_path.exists());

    cleanup_history_path(&history_path);
}

#[test]
fn ready_tasks_rejects_missing_title_contains_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "ready-tasks",
            "--history-path",
            "history.json",
            "--title-contains",
        ])
        .output()
        .expect("failed to run mdid-cli moat ready-tasks with missing title value");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing value for --title-contains"));
}

#[test]
fn cli_filters_ready_tasks_by_exact_spec_ref() {
    let history_path = unique_history_path("ready-tasks-spec-ref");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let round_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to run mdid-cli moat round with history path");
    assert!(
        round_output.status.success(),
        "moat round failed: {}",
        String::from_utf8_lossy(&round_output.stderr)
    );

    make_workflow_audit_spec_task_ready(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "ready-tasks",
            "--history-path",
            history_path_arg,
            "--spec-ref",
            "moat-spec/workflow-audit",
        ])
        .output()
        .expect("failed to run mdid-cli moat ready-tasks with spec-ref filter");

    assert!(
        output.status.success(),
        "ready-tasks failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "moat ready tasks\n",
            "ready_task_entries=1\n",
            "ready_task=planner|spec_planning|spec-workflow-audit|Create spec for workflow audit|moat-spec/workflow-audit\n",
        )
    );

    cleanup_history_path(&history_path);
}

#[test]
fn cli_ready_tasks_spec_ref_filter_succeeds_with_no_matches() {
    let history_path = unique_history_path("ready-tasks-spec-ref-no-match");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let round_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to run mdid-cli moat round with history path");
    assert!(
        round_output.status.success(),
        "moat round failed: {}",
        String::from_utf8_lossy(&round_output.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "ready-tasks",
            "--history-path",
            history_path_arg,
            "--spec-ref",
            "moat-spec/does-not-exist",
        ])
        .output()
        .expect("failed to run mdid-cli moat ready-tasks with missing spec-ref filter");

    assert!(
        output.status.success(),
        "ready-tasks failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!("moat ready tasks\n", "ready_task_entries=0\n")
    );

    cleanup_history_path(&history_path);
}

#[test]
fn ready_tasks_rejects_missing_spec_ref_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "ready-tasks",
            "--history-path",
            "history.json",
            "--spec-ref",
        ])
        .output()
        .expect("failed to run mdid-cli moat ready-tasks with missing spec-ref value");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing value for --spec-ref"));
}

#[test]
fn cli_ready_tasks_node_id_filter_succeeds_with_no_matches() {
    let history_path = unique_history_path("ready-tasks-node-id-no-match");
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

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "ready-tasks",
            "--history-path",
            history_path_arg,
            "--node-id",
            "missing-node",
        ])
        .output()
        .expect("failed to run mdid-cli moat ready-tasks with missing node id filter");

    assert!(
        output.status.success(),
        "expected success, stderr was: {}",
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
fn moat_task_graph_filters_by_assigned_agent_id() {
    let history_path = unique_history_path("task-graph-assigned-agent");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed task graph assigned-agent history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );
    update_history_task_node(&history_path, "implementation", |implementation_node| {
        implementation_node.insert("state".to_string(), Value::String("ready".to_string()));
        implementation_node.insert("depends_on".to_string(), Value::Array(Vec::new()));
    });

    let dispatch = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path_arg,
            "--node-id",
            "implementation",
            "--agent-id",
            "coder-7",
        ])
        .output()
        .expect("failed to dispatch owned implementation task");
    assert!(
        dispatch.status.success(),
        "{}",
        String::from_utf8_lossy(&dispatch.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--assigned-agent-id",
            "coder-7",
        ])
        .output()
        .expect("failed to inspect task graph by assigned agent");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("node=coder|implementation|"), "{stdout}");
    assert!(
        stdout.contains("assigned_agent_id=implementation|coder-7"),
        "{stdout}"
    );
    assert!(!stdout.contains("node=planner|market_scan|"), "{stdout}");

    cleanup_history_path(&history_path);
}

#[test]
fn moat_task_graph_assigned_agent_filter_with_no_match_prints_no_nodes() {
    let history_path = unique_history_path("task-graph-assigned-agent-none");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed task graph assigned-agent non-match history");
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
            "--assigned-agent-id",
            "coder-7",
        ])
        .output()
        .expect("failed to inspect task graph by non-matching assigned agent");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("moat task graph"), "{stdout}");
    assert!(!stdout.contains("node="), "{stdout}");
    assert!(!stdout.contains("assigned_agent_id="), "{stdout}");

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
            "              \"spec_ref\": null,\n",
            "              \"assigned_agent_id\": null,\n",
            "              \"claimed_at\": null,\n",
            "              \"lease_expires_at\": null,\n",
            "              \"last_heartbeat_at\": null,\n",
            "              \"artifacts\": []\n",
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
            "              \"spec_ref\": null,\n",
            "              \"assigned_agent_id\": null,\n",
            "              \"claimed_at\": null,\n",
            "              \"lease_expires_at\": null,\n",
            "              \"last_heartbeat_at\": null,\n",
            "              \"artifacts\": []\n",
            "            },\n",
            "            {\n",
            "              \"node_id\": \"independent_spec_planning\",\n",
            "              \"title\": \"Independent Spec Planning\",\n",
            "              \"role\": \"planner\",\n",
            "              \"kind\": \"spec_planning\",\n",
            "              \"state\": \"completed\",\n",
            "              \"depends_on\": [],\n",
            "              \"spec_ref\": null,\n",
            "              \"assigned_agent_id\": null,\n",
            "              \"claimed_at\": null,\n",
            "              \"lease_expires_at\": null,\n",
            "              \"last_heartbeat_at\": null,\n",
            "              \"artifacts\": []\n",
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
            "assigned_agent_id=review|<none>\n",
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
        concat!(
            "moat task graph\n",
            "node=coder|implementation|Implementation|implementation|completed|spec_planning|<none>\n",
            "assigned_agent_id=implementation|<none>\n",
        )
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
        .contains("selected moat round does not contain implemented_specs handoffs"));
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
fn moat_assignments_filters_by_assigned_agent_id() {
    let history_path = unique_history_path("assignments-assigned-agent");
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
        .expect("failed to seed assignments assigned-agent history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );
    update_history_task_node(&history_path, "review", |review_node| {
        review_node.insert(
            "assigned_agent_id".to_string(),
            Value::String("reviewer-7".to_string()),
        );
    });

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--assigned-agent-id",
            "reviewer-7",
        ])
        .output()
        .expect("failed to inspect assignments by assigned agent");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("assignment=reviewer|review|"), "{stdout}");
    assert!(
        !stdout.contains("assignment=planner|market_scan|"),
        "{stdout}"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn moat_assignments_assigned_agent_filter_with_no_match_prints_zero_entries() {
    let history_path = unique_history_path("assignments-assigned-agent-none");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed assignments assigned-agent non-match history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--assigned-agent-id",
            "coder-7",
        ])
        .output()
        .expect("failed to inspect assignments by non-matching assigned agent");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("assignment_entries=0"), "{stdout}");
    assert!(!stdout.contains("assignment="), "{stdout}");

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
            "market_scan",
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

fn make_workflow_audit_spec_task_ready(history_path: &PathBuf) {
    update_history_task_node(history_path, "spec_planning", |spec_node| {
        spec_node.insert(
            "node_id".to_string(),
            Value::String("spec-workflow-audit".to_string()),
        );
        spec_node.insert(
            "title".to_string(),
            Value::String("Create spec for workflow audit".to_string()),
        );
        spec_node.insert("state".to_string(), Value::String("ready".to_string()));
        spec_node.insert("depends_on".to_string(), Value::Array(Vec::new()));
        spec_node.insert(
            "spec_ref".to_string(),
            Value::String("moat-spec/workflow-audit".to_string()),
        );
    });
}

fn write_history_with_artifact_routing_tasks() -> PathBuf {
    let history_path = unique_history_path("artifact-routing-tasks");
    seed_successful_moat_history(&history_path);
    update_history_task_node(&history_path, "market_scan", |node| {
        node.insert("state".to_string(), Value::String("completed".to_string()));
        node.insert(
            "artifacts".to_string(),
            serde_json::json!([{
                "artifact_ref": "artifacts/market.md",
                "summary": "market evidence",
                "recorded_at": "2026-04-27T16:00:00Z"
            }]),
        );
    });
    update_history_task_node(&history_path, "competitor_analysis", |node| {
        node.insert("state".to_string(), Value::String("completed".to_string()));
        node.insert("artifacts".to_string(), Value::Array(Vec::new()));
    });
    update_history_task_node(&history_path, "strategy_generation", |node| {
        node.insert(
            "node_id".to_string(),
            Value::String("ready_with_artifact".to_string()),
        );
        node.insert(
            "title".to_string(),
            Value::String("Ready With Artifact".to_string()),
        );
        node.insert("state".to_string(), Value::String("ready".to_string()));
        node.insert(
            "depends_on".to_string(),
            Value::Array(vec![Value::String("market_scan".to_string())]),
        );
    });
    update_history_task_node(&history_path, "spec_planning", |node| {
        node.insert(
            "node_id".to_string(),
            Value::String("ready_without_artifact".to_string()),
        );
        node.insert(
            "title".to_string(),
            Value::String("Ready Without Artifact".to_string()),
        );
        node.insert("state".to_string(), Value::String("ready".to_string()));
        node.insert(
            "depends_on".to_string(),
            Value::Array(vec![Value::String("competitor_analysis".to_string())]),
        );
    });
    history_path
}

fn make_workflow_audit_implementation_task_ready(history_path: &PathBuf) {
    update_history_task_node(history_path, "implementation", |implementation_node| {
        implementation_node.insert(
            "node_id".to_string(),
            Value::String("task-implementation".to_string()),
        );
        implementation_node.insert(
            "title".to_string(),
            Value::String("Implement workflow audit".to_string()),
        );
        implementation_node.insert("state".to_string(), Value::String("ready".to_string()));
        implementation_node.insert("depends_on".to_string(), Value::Array(Vec::new()));
        implementation_node.insert(
            "spec_ref".to_string(),
            Value::String("moat-spec/workflow-audit".to_string()),
        );
    });
}

fn make_history_task_node_ready(history_path: &PathBuf, node_id: &str) {
    update_history_task_node(history_path, node_id, |node| {
        node.insert("state".to_string(), Value::String("ready".to_string()));
    });
}

fn update_history_task_node(
    history_path: &PathBuf,
    node_id: &str,
    update: impl FnOnce(&mut serde_json::Map<String, Value>),
) {
    let persisted = fs::read_to_string(history_path)
        .expect("seeded moat history should be readable for fixture adjustment");
    let mut history: Value =
        serde_json::from_str(&persisted).expect("seeded moat history should be valid JSON");
    let nodes = history
        .get_mut(0)
        .and_then(|entry| entry.get_mut("report"))
        .and_then(|report| report.get_mut("control_plane"))
        .and_then(|control_plane| control_plane.get_mut("task_graph"))
        .and_then(|task_graph| task_graph.get_mut("nodes"))
        .and_then(Value::as_array_mut)
        .expect("seeded moat history should contain task graph nodes");
    let task_node = nodes
        .iter_mut()
        .find(|node| node.get("node_id").and_then(Value::as_str) == Some(node_id))
        .and_then(Value::as_object_mut)
        .expect("seeded moat history should contain deterministic task graph node");

    update(task_node);

    let persisted = serde_json::to_string_pretty(&history)
        .expect("adjusted moat history should serialize as JSON");
    fs::write(history_path, persisted)
        .expect("seeded moat history should be writable for fixture adjustment");
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

fn latest_history_round_id(history_path: &PathBuf) -> String {
    let persisted = fs::read_to_string(history_path)
        .expect("seeded moat history should be readable for round id lookup");
    let history: Value = serde_json::from_str(&persisted)
        .expect("seeded moat history should be valid JSON for round id lookup");
    history
        .as_array()
        .and_then(|entries| entries.last())
        .and_then(|entry| entry.get("report"))
        .and_then(|report| report.get("summary"))
        .and_then(|summary| summary.get("round_id"))
        .and_then(Value::as_str)
        .expect("seeded moat history should include latest round id")
        .to_string()
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

#[test]
fn moat_export_specs_can_select_persisted_round_by_exact_round_id() {
    let history_path = unique_history_path("export-specs-round-id");
    let output_dir = unique_history_path("export-specs-round-id-output");
    if output_dir.exists() {
        std::fs::remove_file(&output_dir).expect("remove placeholder path");
    }
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let output_dir_arg = output_dir.to_str().expect("output dir should be utf-8");

    let first = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed first round");
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );

    let first_store =
        LocalMoatHistoryStore::open(&history_path).expect("history store should open");
    let first_round_id = first_store
        .summary()
        .latest_round_id
        .expect("first persisted round id should exist");

    let second = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--review-loops",
            "0",
            "--history-path",
            history_path_arg,
        ])
        .output()
        .expect("failed to seed second round");
    assert!(
        second.status.success(),
        "{}",
        String::from_utf8_lossy(&second.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "export-specs",
            "--history-path",
            history_path_arg,
            "--round-id",
            &first_round_id,
            "--output-dir",
            output_dir_arg,
        ])
        .output()
        .expect("failed to export moat specs for selected round");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("moat spec export complete\n"));
    assert!(stdout.contains(&format!("round_id={first_round_id}\n")));
    assert!(stdout.contains("exported_specs=moat-spec/workflow-audit\n"));
    assert!(output_dir.join("workflow-audit.md").exists());

    cleanup_history_path(&history_path);
    cleanup_history_path(&output_dir);
}

#[test]
fn moat_export_plans_can_select_persisted_round_by_exact_round_id() {
    let history_path = unique_history_path("export-plans-round-id");
    let output_dir = unique_history_path("export-plans-round-id-output");
    if output_dir.exists() {
        std::fs::remove_file(&output_dir).expect("remove placeholder path");
    }
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let output_dir_arg = output_dir.to_str().expect("output dir should be utf-8");

    let first = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed first round");
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );

    let first_store =
        LocalMoatHistoryStore::open(&history_path).expect("history store should open");
    let first_round_id = first_store
        .summary()
        .latest_round_id
        .expect("first persisted round id should exist");

    let second = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--review-loops",
            "0",
            "--history-path",
            history_path_arg,
        ])
        .output()
        .expect("failed to seed second round");
    assert!(
        second.status.success(),
        "{}",
        String::from_utf8_lossy(&second.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "export-plans",
            "--history-path",
            history_path_arg,
            "--round-id",
            &first_round_id,
            "--output-dir",
            output_dir_arg,
        ])
        .output()
        .expect("failed to export moat plans for selected round");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("moat plan export\n"));
    assert!(stdout.contains(&format!("round_id={first_round_id}\n")));
    assert!(stdout.contains("exported_plans=moat-spec/workflow-audit\n"));
    assert!(output_dir
        .join("workflow-audit-implementation-plan.md")
        .exists());

    cleanup_history_path(&history_path);
    cleanup_history_path(&output_dir);
}

#[test]
fn moat_exports_report_selected_round_without_handoffs_for_round_id_selection() {
    let history_path = unique_history_path("export-selected-empty-round-id");
    let specs_output_dir = unique_history_path("export-selected-empty-specs-output");
    let plans_output_dir = unique_history_path("export-selected-empty-plans-output");
    for path in [&specs_output_dir, &plans_output_dir] {
        if path.exists() {
            std::fs::remove_file(path).expect("remove placeholder path");
        }
    }
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let first = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--spec-generations",
            "0",
            "--history-path",
            history_path_arg,
        ])
        .output()
        .expect("failed to seed empty first round");
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );

    let first_round_id = LocalMoatHistoryStore::open(&history_path)
        .expect("history store should open")
        .summary()
        .latest_round_id
        .expect("first persisted round id should exist");

    let second = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed second round");
    assert!(
        second.status.success(),
        "{}",
        String::from_utf8_lossy(&second.stderr)
    );

    for (subcommand, output_dir) in [
        ("export-specs", &specs_output_dir),
        ("export-plans", &plans_output_dir),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
            .args([
                "moat",
                subcommand,
                "--history-path",
                history_path_arg,
                "--round-id",
                &first_round_id,
                "--output-dir",
                output_dir.to_str().expect("output dir should be utf-8"),
            ])
            .output()
            .expect("failed to run selected empty round export");

        assert!(!output.status.success(), "{subcommand} should fail");
        assert!(String::from_utf8_lossy(&output.stderr)
            .contains("selected moat round does not contain implemented_specs handoffs"));
        assert!(!output_dir.exists());
    }

    cleanup_history_path(&history_path);
    cleanup_history_path(&specs_output_dir);
    cleanup_history_path(&plans_output_dir);
}

#[test]
fn moat_export_specs_reports_error_when_round_id_does_not_match_history() {
    let history_path = unique_history_path("export-specs-missing-round-id");
    let output_dir = unique_history_path("export-specs-missing-round-id-output");
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
            "export-specs",
            "--history-path",
            history_path_arg,
            "--round-id",
            "00000000-0000-0000-0000-000000000999",
            "--output-dir",
            output_dir_arg,
        ])
        .output()
        .expect("failed to run moat spec export with missing round id");

    assert!(
        !output.status.success(),
        "export should fail for missing round id"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "error: no moat history entry matched round_id 00000000-0000-0000-0000-000000000999\n"
    );
    assert!(
        !output_dir.exists(),
        "failed export must not create output directory"
    );

    cleanup_history_path(&history_path);
    cleanup_history_path(&output_dir);
}

#[test]
fn task_events_reports_generated_lifecycle_events_for_latest_round() {
    let history_path = unique_history_path("task-events-generated");
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
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    for args in [
        vec![
            "moat",
            "claim-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
            "--agent-id",
            "agent-a",
        ],
        vec![
            "moat",
            "heartbeat-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
            "--agent-id",
            "agent-a",
        ],
        vec![
            "moat",
            "release-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
        ],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
            .args(args)
            .output()
            .expect("failed to mutate task lifecycle");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "task-events", "--history-path", history_path_arg])
        .output()
        .expect("failed to inspect task events");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "moat task events\n",
            "task_event_entries=3\n",
            "task_event=claim|review|agent-a|task claimed\n",
            "task_event=heartbeat|review|agent-a|task heartbeat recorded\n",
            "task_event=release|review|agent-a|task released\n",
        )
    );

    cleanup_history_path(&history_path);
}

#[test]
fn task_events_json_format_emits_filtered_event_envelope() {
    let history_path = unique_history_path("task-events-json");
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
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );
    make_workflow_audit_spec_task_ready(&history_path);

    let claim = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "claim-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "spec-workflow-audit",
            "--agent-id",
            "planner-json",
        ])
        .output()
        .expect("failed to claim moat task");
    assert!(
        claim.status.success(),
        "{}",
        String::from_utf8_lossy(&claim.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-events",
            "--history-path",
            history_path_arg,
            "--node-id",
            "spec-workflow-audit",
            "--action",
            "claim",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to inspect json task events");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be json");
    assert_eq!(json["type"], "moat_task_events");
    assert_eq!(json["history_path"], history_path_arg);
    assert_eq!(json["task_event_entries"], 1);
    let events = json["events"].as_array().expect("events should be array");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["node_id"], "spec-workflow-audit");
    assert_eq!(events[0]["action"], "claim");
    assert_eq!(events[0]["agent_id"], "planner-json");

    cleanup_history_path(&history_path);
}

#[test]
fn task_events_rejects_unknown_format() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-events",
            "--history-path",
            "unused.json",
            "--format",
            "yaml",
        ])
        .output()
        .expect("failed to run task-events with unknown format");

    assert!(
        !output.status.success(),
        "task-events should reject unknown format"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unknown moat task-events format: yaml")
    );
}

#[test]
fn task_events_rejects_duplicate_format() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-events",
            "--history-path",
            "unused.json",
            "--format",
            "json",
            "--format",
            "text",
        ])
        .output()
        .expect("failed to run task-events with duplicate format");

    assert!(
        !output.status.success(),
        "task-events should reject duplicate format"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("duplicate moat task-events --format"));
}

#[test]
fn task_events_filters_conjunctively_and_limits_after_filtering() {
    let history_path = unique_history_path("task-events-filters");
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
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );
    let first_round_id = LocalMoatHistoryStore::open(&history_path)
        .expect("history store should open")
        .summary()
        .latest_round_id
        .expect("first round id should exist");

    let second = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--review-loops",
            "0",
            "--history-path",
            history_path_arg,
        ])
        .output()
        .expect("failed to seed second moat round");
    assert!(
        second.status.success(),
        "{}",
        String::from_utf8_lossy(&second.stderr)
    );

    for args in [
        vec![
            "moat",
            "claim-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
            "--agent-id",
            "agent-b",
        ],
        vec![
            "moat",
            "heartbeat-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
            "--agent-id",
            "agent-b",
        ],
        vec![
            "moat",
            "complete-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
        ],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
            .args(args)
            .output()
            .expect("failed to mutate task lifecycle");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-events",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
            "--agent-id",
            "agent-b",
            "--contains",
            "task",
            "--limit",
            "2",
        ])
        .output()
        .expect("failed to inspect filtered task events");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "moat task events\n",
            "task_event_entries=2\n",
            "task_event=claim|review|agent-b|task claimed\n",
            "task_event=heartbeat|review|agent-b|task heartbeat recorded\n",
        )
    );

    let no_selected_round = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-events",
            "--history-path",
            history_path_arg,
            "--round-id",
            &first_round_id,
            "--action",
            "claim",
        ])
        .output()
        .expect("failed to inspect selected empty task event round");
    assert!(
        no_selected_round.status.success(),
        "{}",
        String::from_utf8_lossy(&no_selected_round.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&no_selected_round.stdout),
        "moat task events\ntask_event_entries=0\n"
    );

    cleanup_history_path(&history_path);
}
