use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn moat_controller_plan_text_exports_multiple_ready_packets_read_only() {
    let history_path = unique_history_path("controller-plan-text");
    write_history_fixture(&history_path);
    let before = fs::read_to_string(&history_path).expect("failed to read seeded history");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "controller-plan",
            "--history-path",
            history_path.to_str().expect("history path utf8"),
            "--limit",
            "2",
        ])
        .output()
        .expect("failed to run moat controller-plan text");

    assert!(
        output.status.success(),
        "controller-plan text failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("controller-plan text stdout utf8");
    assert!(stdout.contains("controller_plan_packets=2"));
    assert!(stdout.contains("work_packet=market_scan|planner|market_scan|ready"));
    assert!(stdout.contains("work_packet=competitor_analysis|planner|competitor_analysis|ready"));
    assert!(stdout.contains("acceptance_criteria=Read-only controller packet export only"));
    assert!(!stdout.contains("complete_command"));
    assert!(!stdout.contains("complete-task"));

    let after = fs::read_to_string(&history_path).expect("failed to read history after command");
    assert_eq!(after, before, "controller-plan text mutated history");
    cleanup_history_path(&history_path);
}

#[test]
fn moat_controller_plan_json_exports_multiple_ready_packets_read_only() {
    let history_path = unique_history_path("controller-plan-json");
    write_history_fixture(&history_path);
    let before = fs::read_to_string(&history_path).expect("failed to read seeded history");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "controller-plan",
            "--history-path",
            history_path.to_str().expect("history path utf8"),
            "--limit",
            "2",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run moat controller-plan json");

    assert!(
        output.status.success(),
        "controller-plan json failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("controller-plan stdout json");
    assert_eq!(json["type"], "moat_controller_plan");
    assert_eq!(json["read_only"], true);
    assert_eq!(json["packet_count"], 2);
    assert_eq!(json["packets"][0]["node_id"], "market_scan");
    assert!(json["packets"][0].get("complete_command").is_none());
    assert_eq!(
        json["packets"][0]["acceptance_criteria"],
        json!(["Read-only controller packet export only; do not mutate moat history or advertise write-side completion commands."])
    );
    assert_eq!(json["constraints"]["local_only"], true);
    assert_eq!(json["constraints"]["read_only"], true);
    assert_eq!(json["constraints"]["no_code_writes"], true);
    assert_eq!(json["constraints"]["no_artifact_writes"], true);

    let serialized = json.to_string();
    assert!(!serialized.contains("complete_command"));
    assert!(!serialized.contains("complete-task"));

    let after = fs::read_to_string(&history_path).expect("failed to read history after command");
    assert_eq!(after, before, "controller-plan json mutated history");
    cleanup_history_path(&history_path);
}

#[test]
fn moat_controller_plan_requires_history_path() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "controller-plan"])
        .output()
        .expect("failed to run moat controller-plan without history path");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("missing --history-path for moat controller-plan"));
}

#[test]
fn moat_controller_plan_rejects_conflicting_dependency_filters() {
    let history_path = unique_history_path("controller-plan-conflicting-dependency-flags");
    write_history_fixture(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "controller-plan",
            "--history-path",
            history_path.to_str().expect("history path utf8"),
            "--depends-on",
            "market_scan",
            "--no-dependencies",
        ])
        .output()
        .expect("failed to run moat controller-plan with conflicting dependency flags");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("moat controller-plan cannot combine --depends-on and --no-dependencies"));
    cleanup_history_path(&history_path);
}

#[test]
fn moat_controller_plan_reports_unknown_round_id() {
    let history_path = unique_history_path("controller-plan-unknown-round-id");
    write_history_fixture(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "controller-plan",
            "--history-path",
            history_path.to_str().expect("history path utf8"),
            "--round-id",
            "missing-round",
        ])
        .output()
        .expect("failed to run moat controller-plan with unknown round id");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unknown moat round-id: missing-round")
    );
    cleanup_history_path(&history_path);
}

#[test]
fn moat_controller_plan_filters_ready_packets_across_controller_flags() {
    let history_path = unique_history_path("controller-plan-filter-flags");
    write_history_fixture(&history_path);

    assert_packet_node_ids(
        &history_path,
        &["--role", "planner"],
        &["market_scan", "competitor_analysis"],
    );
    assert_packet_node_ids(&history_path, &["--kind", "market_scan"], &["market_scan"]);
    assert_packet_node_ids(
        &history_path,
        &["--node-id", "competitor_analysis"],
        &["competitor_analysis"],
    );
    assert_packet_node_ids(
        &history_path,
        &["--no-dependencies"],
        &["market_scan", "competitor_analysis"],
    );
    assert_packet_node_ids(
        &history_path,
        &["--title-contains", "incumbent clinic"],
        &["competitor_analysis"],
    );

    cleanup_history_path(&history_path);
}

#[test]
fn moat_controller_plan_filters_requires_artifacts_and_spec_ref() {
    let history_path = unique_history_path("controller-plan-artifacts-spec-ref");
    write_history_fixture_with_value(
        &history_path,
        json!({
            "entries": [
                {
                    "report": {
                        "summary": { "round_id": "round-123" },
                        "control_plane": {
                            "task_graph": {
                                "nodes": [
                                    {
                                        "node_id": "market_scan",
                                        "title": "Map the local workflow moat",
                                        "role": "planner",
                                        "kind": "market_scan",
                                        "state": "ready",
                                        "spec_ref": null,
                                        "depends_on": [],
                                        "artifacts": [{ "path": "reports/market-scan.md" }]
                                    },
                                    {
                                        "node_id": "competitor_analysis",
                                        "title": "Profile incumbent clinic alternatives",
                                        "role": "planner",
                                        "kind": "competitor_analysis",
                                        "state": "ready",
                                        "spec_ref": null,
                                        "depends_on": []
                                    },
                                    {
                                        "node_id": "artifact_ready_implementation",
                                        "title": "Implement the selected moat slice",
                                        "role": "coder",
                                        "kind": "implementation",
                                        "state": "ready",
                                        "spec_ref": "moat-spec/workflow-audit",
                                        "depends_on": ["market_scan"]
                                    },
                                    {
                                        "node_id": "artifact_missing_implementation",
                                        "title": "Implement the backup moat slice",
                                        "role": "coder",
                                        "kind": "implementation",
                                        "state": "ready",
                                        "spec_ref": "moat-spec/backup",
                                        "depends_on": ["competitor_analysis"]
                                    }
                                ]
                            }
                        }
                    }
                }
            ]
        }),
    );

    assert_packet_node_ids(
        &history_path,
        &["--requires-artifacts"],
        &[
            "market_scan",
            "competitor_analysis",
            "artifact_ready_implementation",
        ],
    );
    assert_packet_node_ids(
        &history_path,
        &["--spec-ref", "moat-spec/workflow-audit"],
        &["artifact_ready_implementation"],
    );

    cleanup_history_path(&history_path);
}

#[test]
fn moat_controller_plan_rejects_invalid_format_limit_duplicate_and_unknown_flags() {
    let history_path = unique_history_path("controller-plan-invalid-flags");
    write_history_fixture(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "controller-plan",
            "--history-path",
            history_path.to_str().expect("history path utf8"),
            "--format",
            "yaml",
        ])
        .output()
        .expect("failed to run moat controller-plan");
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("unknown moat controller-plan format: yaml"),
        "stderr missing invalid format text: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_stderr_contains(
        &history_path,
        &["--limit", "abc"],
        "invalid moat controller-plan --limit: abc",
    );
    assert_stderr_contains(
        &history_path,
        &["--role", "planner", "--role", "coder"],
        "duplicate moat controller-plan --role",
    );
    assert_stderr_contains(
        &history_path,
        &["--role", "operator"],
        "invalid moat controller-plan --role: operator (expected planner|coder|reviewer)",
    );
    assert_stderr_contains(
        &history_path,
        &["--kind", "market_scan", "--kind", "implementation"],
        "duplicate moat controller-plan --kind",
    );
    assert_stderr_contains(
        &history_path,
        &[
            "--node-id",
            "market_scan",
            "--node-id",
            "competitor_analysis",
        ],
        "duplicate moat controller-plan --node-id",
    );
    assert_stderr_contains(
        &history_path,
        &["--no-dependencies", "--no-dependencies"],
        "duplicate moat controller-plan --no-dependencies",
    );
    assert_stderr_contains(
        &history_path,
        &["--requires-artifacts", "--requires-artifacts"],
        "duplicate moat controller-plan --requires-artifacts",
    );
    assert_stderr_contains(
        &history_path,
        &["--title-contains", "workflow", "--title-contains", "clinic"],
        "duplicate moat controller-plan --title-contains",
    );
    assert_stderr_contains(
        &history_path,
        &[
            "--spec-ref",
            "moat-spec/workflow-audit",
            "--spec-ref",
            "moat-spec/backup",
        ],
        "duplicate moat controller-plan --spec-ref",
    );
    assert_stderr_contains(
        &history_path,
        &["--mystery-flag"],
        "unknown option for moat controller-plan: --mystery-flag",
    );
    assert_stderr_contains(
        &history_path,
        &["--agent-id", "agent-1"],
        "unsupported option for moat controller-plan: --agent-id",
    );
    assert_stderr_contains(
        &history_path,
        &["--lease-seconds", "30"],
        "unsupported option for moat controller-plan: --lease-seconds",
    );
    assert_stderr_contains(
        &history_path,
        &["--dry-run"],
        "unsupported option for moat controller-plan: --dry-run",
    );

    cleanup_history_path(&history_path);
}

#[test]
fn moat_controller_plan_rejects_malformed_node_data() {
    let history_path = unique_history_path("controller-plan-malformed-node");
    write_history_fixture_with_value(
        &history_path,
        json!({
            "entries": [
                {
                    "report": {
                        "summary": { "round_id": "round-123" },
                        "control_plane": {
                            "task_graph": {
                                "nodes": [
                                    {
                                        "node_id": "market_scan",
                                        "title": "Map the local workflow moat",
                                        "role": "planner",
                                        "kind": "market_scan",
                                        "state": "ready",
                                        "depends_on": [123]
                                    }
                                ]
                            }
                        }
                    }
                }
            ]
        }),
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "controller-plan",
            "--history-path",
            history_path.to_str().expect("history path utf8"),
        ])
        .output()
        .expect("failed to run moat controller-plan with malformed node data");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains(
        "invalid moat history file: node market_scan field depends_on entries must be strings"
    ));
    cleanup_history_path(&history_path);
}

#[test]
fn moat_controller_step_json_claims_ready_task_and_embeds_work_packet() {
    let history_path = unique_history_path("controller-step-json");
    write_history_fixture_with_value(&history_path, controller_step_history_fixture());

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "controller-step",
            "--history-path",
            history_path.to_str().expect("history path utf8"),
            "--agent-id",
            "reviewer-1",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run moat controller-step json");

    assert!(
        output.status.success(),
        "controller-step failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("controller-step stdout json");
    assert_eq!(json["type"], "moat_controller_step");
    assert_eq!(
        json["history_path"],
        history_path.to_string_lossy().as_ref()
    );
    assert_eq!(json["dry_run"], false);
    assert_eq!(json["claimed"], true);
    assert_eq!(json["agent_id"], "reviewer-1");
    assert_eq!(json["assigned_agent_id"], "reviewer-1");
    assert_eq!(json["round_id"], "round-controller-step");
    assert_eq!(json["node_id"], "review");
    assert_eq!(json["role"], "reviewer");
    assert_eq!(json["kind"], "review");
    assert_eq!(json["previous_state"], "ready");
    assert_eq!(json["new_state"], "in_progress");
    assert_eq!(json["lease_seconds"], 900);
    assert!(json.get("complete_command").is_none());
    let packet = &json["work_packet"];
    assert_eq!(packet["type"], "moat_work_packet");
    assert_eq!(packet["node_id"], "review");
    assert_eq!(packet["role"], "reviewer");
    assert!(packet.get("complete_command").is_none());
    assert!(packet["acceptance_criteria"]
        .as_array()
        .expect("acceptance array")
        .iter()
        .any(|value| value.as_str().unwrap().contains("Use SDD and TDD")));

    let history: Value = serde_json::from_str(
        &fs::read_to_string(&history_path).expect("failed to read claimed history"),
    )
    .expect("claimed history json");
    let review_node = history["entries"][0]["report"]["control_plane"]["task_graph"]["nodes"]
        .as_array()
        .expect("nodes array")
        .iter()
        .find(|node| node["node_id"] == "review")
        .expect("review node present");
    assert_eq!(review_node["state"], "in_progress");
    assert_eq!(review_node["assigned_agent_id"], "reviewer-1");
    assert_eq!(review_node["lease_seconds"], 900);
    cleanup_history_path(&history_path);
}

#[test]
fn moat_controller_step_dry_run_json_exports_packet_without_mutating_history() {
    let history_path = unique_history_path("controller-step-dry-run");
    write_history_fixture_with_value(&history_path, controller_step_history_fixture());
    let before = fs::read_to_string(&history_path).expect("failed to read seeded history");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "controller-step",
            "--history-path",
            history_path.to_str().expect("history path utf8"),
            "--dry-run",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run dry-run moat controller-step");

    assert!(
        output.status.success(),
        "controller-step dry-run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value =
        serde_json::from_slice(&output.stdout).expect("controller-step dry-run stdout json");
    assert_eq!(json["type"], "moat_controller_step");
    assert_eq!(json["dry_run"], true);
    assert_eq!(json["claimed"], false);
    assert_eq!(json["assigned_agent_id"], Value::Null);
    assert_eq!(json["node_id"], "review");
    let packet = &json["work_packet"];
    assert_eq!(packet["node_id"], "review");
    assert_eq!(packet["state"], "ready");
    assert_eq!(packet["spec_ref"], Value::Null);
    assert_eq!(packet["dependencies"][0], "implementation");
    assert_eq!(
        packet["dependency_artifacts"]
            .as_array()
            .expect("dependency artifacts")
            .len(),
        1
    );
    assert_eq!(
        packet["dependency_artifacts"][0]["node_id"],
        "implementation"
    );
    assert_eq!(
        packet["dependency_artifacts"][0]["artifact_ref"],
        "plan://implementation-controller-step-output"
    );
    assert_eq!(
        packet["dependency_artifacts"][0]["artifact_summary"],
        "Implemented upstream controller-step dependency"
    );

    let after = fs::read_to_string(&history_path).expect("failed to read dry-run history");
    assert_eq!(after, before, "dry-run controller-step mutated history");
    cleanup_history_path(&history_path);
}

#[test]
fn moat_controller_step_requires_artifacts_exports_top_level_dependency_artifact() {
    let history_path = unique_history_path("controller-step-top-level-artifact");
    let mut history = controller_step_history_fixture();
    let implementation =
        &mut history["entries"][0]["report"]["control_plane"]["task_graph"]["nodes"][0];
    implementation
        .as_object_mut()
        .expect("implementation object")
        .remove("artifacts");
    implementation["artifact_ref"] =
        Value::String("plan://implementation-top-level-output".to_string());
    implementation["artifact_summary"] =
        Value::String("Top-level implementation artifact".to_string());
    write_history_fixture_with_value(&history_path, history);

    let before = fs::read_to_string(&history_path).expect("failed to read seeded history");
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "controller-step",
            "--history-path",
            history_path.to_str().expect("history path utf8"),
            "--requires-artifacts",
            "--dry-run",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run controller-step with top-level dependency artifact");

    assert!(
        output.status.success(),
        "controller-step failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("controller-step stdout json");
    assert_eq!(
        json["work_packet"]["dependency_artifacts"][0]["node_id"],
        "implementation"
    );
    assert_eq!(
        json["work_packet"]["dependency_artifacts"][0]["artifact_ref"],
        "plan://implementation-top-level-output"
    );
    assert_eq!(
        json["work_packet"]["dependency_artifacts"][0]["artifact_summary"],
        "Top-level implementation artifact"
    );
    let after = fs::read_to_string(&history_path).expect("failed to read dry-run history");
    assert_eq!(after, before, "dry-run controller-step mutated history");
    cleanup_history_path(&history_path);
}

#[test]
fn moat_controller_step_requires_artifacts_ignores_top_level_summary_without_ref() {
    let history_path = unique_history_path("controller-step-summary-without-ref");
    let mut history = controller_step_history_fixture();
    let implementation =
        &mut history["entries"][0]["report"]["control_plane"]["task_graph"]["nodes"][0];
    implementation
        .as_object_mut()
        .expect("implementation object")
        .remove("artifacts");
    implementation["artifact_summary"] =
        Value::String("Summary without exportable artifact ref".to_string());
    write_history_fixture_with_value(&history_path, history);

    let before = fs::read_to_string(&history_path).expect("failed to read seeded history");
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "controller-step",
            "--history-path",
            history_path.to_str().expect("history path utf8"),
            "--requires-artifacts",
            "--dry-run",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run controller-step with summary-only dependency artifact");

    assert!(
        !output.status.success(),
        "summary-only artifact dependency should not match requires-artifacts: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("no ready moat task matched dispatch filters"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let after = fs::read_to_string(&history_path).expect("failed to read dry-run history");
    assert_eq!(after, before, "dry-run controller-step mutated history");
    cleanup_history_path(&history_path);
}

#[test]
fn moat_controller_step_lock_rejects_non_dry_run_but_dry_run_ignores_stale_lock() {
    let history_path = unique_history_path("controller-step-lock");
    write_history_fixture_with_value(&history_path, controller_step_history_fixture());
    let before = fs::read_to_string(&history_path).expect("failed to read seeded history");
    let lock_path = history_path.with_file_name(format!(
        ".{}.lock",
        history_path
            .file_name()
            .expect("history filename")
            .to_string_lossy()
    ));
    fs::write(&lock_path, "stale lock").expect("write stale lock");

    let rejected = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "controller-step",
            "--history-path",
            history_path.to_str().expect("history path utf8"),
            "--agent-id",
            "reviewer-locked",
        ])
        .output()
        .expect("failed to run locked controller-step");
    assert!(
        !rejected.status.success(),
        "locked controller-step unexpectedly succeeded"
    );
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("moat history lock already exists"));
    let after_rejected =
        fs::read_to_string(&history_path).expect("failed to read rejected history");
    assert_eq!(
        after_rejected, before,
        "locked controller-step mutated history"
    );

    let dry_run = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "controller-step",
            "--history-path",
            history_path.to_str().expect("history path utf8"),
            "--dry-run",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run dry-run locked controller-step");
    assert!(
        dry_run.status.success(),
        "dry-run should ignore stale lock: {}",
        String::from_utf8_lossy(&dry_run.stderr)
    );
    let after_dry_run = fs::read_to_string(&history_path).expect("failed to read dry-run history");
    assert_eq!(after_dry_run, before, "dry-run with lock mutated history");
    cleanup_history_path(&history_path);
    let _ = fs::remove_file(lock_path);
}

#[test]
fn moat_controller_step_text_prints_bounded_handoff() {
    let history_path = unique_history_path("controller-step-text");
    write_history_fixture_with_value(&history_path, controller_step_history_fixture());

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "controller-step",
            "--history-path",
            history_path.to_str().expect("history path utf8"),
            "--agent-id",
            "reviewer-text",
        ])
        .output()
        .expect("failed to run moat controller-step text");

    assert!(
        output.status.success(),
        "controller-step text failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("controller-step text stdout utf8");
    assert!(stdout.contains("moat controller step"));
    assert!(stdout.contains("claimed=true"));
    assert!(stdout.contains("node_id=review"));
    assert!(stdout.contains("role=reviewer"));
    assert!(stdout.contains("kind=review"));
    assert!(stdout.contains("previous_state=ready"));
    assert!(stdout.contains("new_state=in_progress"));
    assert!(!stdout.contains("complete_command="));
    assert!(stdout.contains("acceptance=Use SDD and TDD"));
    cleanup_history_path(&history_path);
}

#[test]
fn moat_controller_step_filters_select_exact_ready_task() {
    let history_path = unique_history_path("controller-step-filters");
    write_history_fixture_with_value(&history_path, controller_step_history_fixture());

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "controller-step",
            "--history-path",
            history_path.to_str().expect("history path utf8"),
            "--role",
            "reviewer",
            "--kind",
            "review",
            "--node-id",
            "review",
            "--depends-on",
            "implementation",
            "--requires-artifacts",
            "--title-contains",
            "Review",
            "--dry-run",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run filtered moat controller-step");

    assert!(
        output.status.success(),
        "filtered controller-step failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("filtered stdout json");
    assert_eq!(json["node_id"], "review");
    assert_eq!(json["dry_run"], true);
    assert_eq!(json["claimed"], false);
    cleanup_history_path(&history_path);
}

#[test]
fn moat_controller_step_rejects_missing_history_invalid_format_and_non_positive_lease() {
    let history_path = unique_history_path("controller-step-missing-history");
    let _ = fs::remove_file(&history_path);
    let missing = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "controller-step",
            "--history-path",
            history_path.to_str().expect("history path utf8"),
            "--agent-id",
            "reviewer-missing",
        ])
        .output()
        .expect("failed to run moat controller-step with missing history");
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("failed to read moat history file"));
    assert!(
        !history_path.exists(),
        "controller-step created a missing history file"
    );

    let missing_without_agent = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "controller-step",
            "--history-path",
            history_path.to_str().expect("history path utf8"),
        ])
        .output()
        .expect("failed to run moat controller-step with missing history and no agent");
    assert!(!missing_without_agent.status.success());
    assert!(String::from_utf8_lossy(&missing_without_agent.stderr)
        .contains("failed to read moat history file"));

    let unknown_format = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "controller-step",
            "--history-path",
            "history.json",
            "--format",
            "yaml",
        ])
        .output()
        .expect("failed to run moat controller-step with unknown format");
    assert!(!unknown_format.status.success());
    assert!(String::from_utf8_lossy(&unknown_format.stderr)
        .contains("unknown moat controller-step format: yaml"));

    let bad_lease = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "controller-step",
            "--history-path",
            "history.json",
            "--lease-seconds",
            "0",
        ])
        .output()
        .expect("failed to run moat controller-step with bad lease");
    assert!(!bad_lease.status.success());
    assert!(String::from_utf8_lossy(&bad_lease.stderr)
        .contains("moat controller-step --lease-seconds must be positive"));

    let unsupported_limit = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "controller-step",
            "--history-path",
            "history.json",
            "--limit",
            "1",
        ])
        .output()
        .expect("failed to run moat controller-step with unsupported limit");
    assert!(!unsupported_limit.status.success());
    assert!(String::from_utf8_lossy(&unsupported_limit.stderr)
        .contains("unsupported option for moat controller-step: --limit"));
}

fn write_history_fixture(path: &PathBuf) {
    write_history_fixture_with_value(path, sample_history_fixture());
}

fn write_history_fixture_with_value(path: &PathBuf, value: Value) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent dir");
    }
    fs::write(path, value.to_string()).expect("write history fixture");
}

fn sample_history_fixture() -> Value {
    json!({
        "entries": [
            {
                "report": {
                    "summary": {
                        "round_id": "round-123"
                    },
                    "control_plane": {
                        "task_graph": {
                            "nodes": [
                                {
                                    "node_id": "market_scan",
                                    "title": "Map the local workflow moat",
                                    "role": "planner",
                                    "kind": "market_scan",
                                    "state": "ready",
                                    "spec_ref": null,
                                    "depends_on": []
                                },
                                {
                                    "node_id": "competitor_analysis",
                                    "title": "Profile incumbent clinic alternatives",
                                    "role": "planner",
                                    "kind": "competitor_analysis",
                                    "state": "ready",
                                    "spec_ref": null,
                                    "depends_on": []
                                },
                                {
                                    "node_id": "implementation",
                                    "title": "Implement the selected moat slice",
                                    "role": "coder",
                                    "kind": "implementation",
                                    "state": "pending",
                                    "spec_ref": "moat-spec/workflow-audit",
                                    "depends_on": ["market_scan"]
                                }
                            ]
                        }
                    }
                }
            }
        ]
    })
}

fn controller_step_history_fixture() -> Value {
    json!({
        "entries": [
            {
                "report": {
                    "summary": {
                        "round_id": "round-controller-step"
                    },
                    "control_plane": {
                        "task_graph": {
                            "nodes": [
                                {
                                    "node_id": "implementation",
                                    "title": "Implement the selected moat slice",
                                    "role": "coder",
                                    "kind": "implementation",
                                    "state": "completed",
                                    "spec_ref": "moat-spec/workflow-audit",
                                    "depends_on": [],
                                    "artifacts": [
                                        {
                                            "artifact_ref": "plan://implementation-controller-step-output",
                                            "summary": "Implemented upstream controller-step dependency"
                                        }
                                    ]
                                },
                                {
                                    "node_id": "review",
                                    "title": "Review the implementation handoff",
                                    "role": "reviewer",
                                    "kind": "review",
                                    "state": "ready",
                                    "spec_ref": null,
                                    "depends_on": ["implementation"]
                                },
                                {
                                    "node_id": "follow_up_review",
                                    "title": "Review another branch",
                                    "role": "reviewer",
                                    "kind": "review",
                                    "state": "pending",
                                    "spec_ref": null,
                                    "depends_on": ["implementation"]
                                }
                            ]
                        }
                    }
                }
            }
        ]
    })
}

fn unique_history_path(label: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time ok")
        .as_nanos();
    path.push(format!("mdid-cli-{label}-{timestamp}.json"));
    path
}

fn cleanup_history_path(path: &PathBuf) {
    let _ = fs::remove_file(path);
}

fn assert_packet_node_ids(history_path: &Path, extra_args: &[&str], expected_node_ids: &[&str]) {
    let mut args = vec!["--format", "json"];
    args.extend_from_slice(extra_args);
    let output = run_controller_plan(history_path, &args);
    assert!(
        output.status.success(),
        "controller-plan filter command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: Value = serde_json::from_slice(&output.stdout).expect("controller-plan stdout json");
    let actual_node_ids = json["packets"]
        .as_array()
        .expect("packets array")
        .iter()
        .map(|packet| {
            packet["node_id"]
                .as_str()
                .expect("packet node id")
                .to_string()
        })
        .collect::<Vec<_>>();
    let expected_node_ids = expected_node_ids
        .iter()
        .map(|node_id| node_id.to_string())
        .collect::<Vec<_>>();

    assert_eq!(actual_node_ids, expected_node_ids);
    assert_eq!(json["packet_count"], expected_node_ids.len());
}

fn assert_stderr_contains(history_path: &Path, extra_args: &[&str], expected: &str) {
    let output = run_controller_plan(history_path, extra_args);
    assert!(
        !output.status.success(),
        "controller-plan unexpectedly succeeded"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(expected),
        "stderr missing expected text `{expected}`: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn run_controller_plan(history_path: &Path, extra_args: &[&str]) -> std::process::Output {
    let mut args = vec![
        "moat",
        "controller-plan",
        "--history-path",
        history_path.to_str().expect("history path utf8"),
    ];
    args.extend_from_slice(extra_args);

    Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(args)
        .output()
        .expect("failed to run moat controller-plan")
}
