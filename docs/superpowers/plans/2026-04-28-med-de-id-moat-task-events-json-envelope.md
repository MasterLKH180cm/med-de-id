# Moat Task Events JSON Envelope Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add machine-readable JSON output to `mdid-cli moat task-events` so external autonomous controllers can consume lifecycle events without scraping text.

**Architecture:** Extend the existing read-only `task-events` command with `--format text|json`, preserving current text output as the default. JSON output is a deterministic envelope containing `type`, selected `round_id`, `history_path`, `task_event_entries`, and an `events` array after all existing filters and limits are applied.

**Tech Stack:** Rust workspace, `mdid-cli`, local JSON moat history, Cargo integration tests, serde_json in tests.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs` — parse the optional format flag, render JSON events, update usage text.
- Modify: `crates/mdid-cli/tests/moat_cli.rs` — add integration coverage for JSON task-event output and invalid/duplicate format errors.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md` — truth-sync shipped task-events surface.
- Modify: `README.md` — document the JSON envelope for controller handoff.
- Modify: `AGENTS.md` — keep local controller docs aligned with landed behavior.

### Task 1: Add JSON output to `moat task-events`

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
- Modify: `README.md`
- Modify: `AGENTS.md`

- [ ] **Step 1: Write the failing JSON envelope test**

Add this test near existing `task_events_*` tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn task_events_json_format_emits_filtered_event_envelope() {
    let temp = tempdir().expect("failed to create tempdir");
    let history_path = temp.path().join("moat-history.json");
    let history_path_arg = history_path.to_str().expect("utf8 path");

    Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history");

    Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
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
        .expect("failed to claim task");

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
        .expect("failed to run task-events json output");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(json["type"], "moat_task_events");
    assert_eq!(json["history_path"], history_path_arg);
    assert_eq!(json["task_event_entries"], 1);
    assert_eq!(json["events"][0]["node_id"], "spec-workflow-audit");
    assert_eq!(json["events"][0]["action"], "claim");
    assert_eq!(json["events"][0]["agent_id"], "planner-json");
    assert_eq!(json["events"].as_array().expect("events array").len(), 1);
}
```

- [ ] **Step 2: Run the focused test to verify RED**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli task_events_json_format_emits_filtered_event_envelope -- --nocapture`

Expected: FAIL with an error such as `unknown option for moat task-events: --format` or non-JSON stdout.

- [ ] **Step 3: Add invalid format coverage**

Add these tests next to the JSON envelope test:

```rust
#[test]
fn task_events_rejects_unknown_format() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "task-events", "--history-path", "history.json", "--format", "yaml"])
        .output()
        .expect("failed to run task-events with invalid format");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unknown moat task-events format: yaml"));
}

#[test]
fn task_events_rejects_duplicate_format() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-events",
            "--history-path",
            "history.json",
            "--format",
            "json",
            "--format",
            "text",
        ])
        .output()
        .expect("failed to run task-events with duplicate format");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("duplicate moat task-events --format"));
}
```

- [ ] **Step 4: Run invalid format tests to verify RED**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli task_events_rejects_ -- --nocapture`

Expected: FAIL until `--format` parsing exists.

- [ ] **Step 5: Implement minimal JSON/text format support**

In `crates/mdid-cli/src/main.rs`:

1. Add a local enum:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MoatOutputFormat {
    Text,
    Json,
}
```

2. Add `format: MoatOutputFormat` to `MoatTaskEventsCommand`.
3. Parse `--format text|json` in `parse_moat_task_events_command`, rejecting duplicates with `duplicate moat task-events --format`, missing values with `missing value for moat task-events --format`, and unknown values with `unknown moat task-events format: VALUE`.
4. In `run_moat_task_events`, collect filtered events first. If `format == Text`, preserve the exact existing output. If `format == Json`, print this envelope with `serde_json::json!` and `serde_json::to_string_pretty`:

```json
{
  "type": "moat_task_events",
  "round_id": "...",
  "history_path": "...",
  "task_event_entries": 1,
  "events": [
    {
      "recorded_at": "...",
      "node_id": "spec-workflow-audit",
      "action": "claim",
      "previous_state": "ready",
      "new_state": "in_progress",
      "agent_id": "planner-json",
      "lease_expires_at": "...",
      "artifact_ref": null,
      "artifact_summary": null,
      "reason": null
    }
  ]
}
```

Use existing state/action formatting helpers and nullable JSON fields for absent optional event metadata.

- [ ] **Step 6: Run focused GREEN tests**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli task_events_json_format_emits_filtered_event_envelope task_events_rejects_ -- --nocapture`

Expected: PASS.

- [ ] **Step 7: Run broader task-events regression tests**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli task_events -- --nocapture`

Expected: PASS.

- [ ] **Step 8: Truth-sync docs**

Update `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`, `README.md`, and `AGENTS.md` to state that `mdid-cli moat task-events` supports `--format text|json`, defaults to text, and JSON emits a deterministic read-only controller envelope after filters and limits.

- [ ] **Step 9: Run docs/code targeted verification**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli task_events -- --nocapture`

Expected: PASS.

- [ ] **Step 10: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md README.md AGENTS.md docs/superpowers/plans/2026-04-28-med-de-id-moat-task-events-json-envelope.md
git commit -m "feat(cli): emit moat task events as json"
```

## Self-Review

- Spec coverage: Adds machine-readable lifecycle event inspection for autonomous controllers without launching agents or mutating history.
- Placeholder scan: No TBD/TODO/fill-in placeholders are present.
- Type consistency: `MoatOutputFormat`, `--format text|json`, and `moat_task_events` names are used consistently.
