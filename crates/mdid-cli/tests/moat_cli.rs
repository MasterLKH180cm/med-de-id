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
    assert!(stdout.contains("complete_command=mdid-cli moat complete-task"));

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
    assert!(json["packets"][0]["complete_command"]
        .as_str()
        .expect("complete command string")
        .contains("complete-task"));
    assert_eq!(json["constraints"]["local_only"], true);
    assert_eq!(json["constraints"]["read_only"], true);

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
