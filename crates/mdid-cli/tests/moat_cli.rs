use serde_json::{json, Value};
use std::{
    fs,
    path::PathBuf,
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

fn assert_packet_node_ids(history_path: &PathBuf, extra_args: &[&str], expected_node_ids: &[&str]) {
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

fn assert_stderr_contains(history_path: &PathBuf, extra_args: &[&str], expected: &str) {
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

fn run_controller_plan(history_path: &PathBuf, extra_args: &[&str]) -> std::process::Output {
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
