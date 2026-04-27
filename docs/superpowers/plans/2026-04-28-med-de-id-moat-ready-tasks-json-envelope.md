# Moat Ready Tasks JSON Envelope Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add parseable JSON output to `mdid-cli moat ready-tasks` so external autonomous controllers can route all currently ready tasks without scraping text rows.

**Architecture:** Reuse the existing bounded read-only ready-task selection path and add an output-format switch at the CLI boundary only. Text output remains backward compatible; JSON output emits a deterministic envelope with selected round metadata and the same filtered task list, without mutating history or launching agents.

**Tech Stack:** Rust workspace, `mdid-cli`, `serde_json`, Cargo integration tests, local JSON moat-history fixtures.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `output_format: MoatOutputFormat` to `MoatReadyTasksCommand`.
  - Parse `--format text|json` for `moat ready-tasks` with duplicate, missing, and invalid-value errors.
  - Render `--format json` as a pretty deterministic envelope.
  - Update usage text for `moat ready-tasks`.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add RED/GREEN integration tests for JSON output and invalid format handling.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Truth-sync the shipped ready-tasks CLI contract.
- Modify: `README.md`
  - Truth-sync moat-loop CLI documentation and examples.
- Modify: `docs/superpowers/plans/2026-04-28-med-de-id-moat-ready-tasks-json-envelope.md`
  - Mark completion evidence if the implementer keeps plan task checkboxes current.

### Task 1: Add JSON output to `moat ready-tasks`

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
- Modify: `README.md`

- [x] **Step 1: Write the failing JSON output test**

Add this test near the existing `ready-tasks` integration tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn moat_ready_tasks_json_prints_parseable_filtered_envelope() {
    let history_path = unique_history_path("ready-tasks-json-envelope");
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
            "--role",
            "planner",
            "--limit",
            "1",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run mdid-cli moat ready-tasks with json format");

    assert!(
        output.status.success(),
        "ready-tasks json failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value =
        serde_json::from_str(&stdout).expect("ready-tasks json should be parseable");
    assert_eq!(value["type"], "moat_ready_tasks");
    assert_eq!(value["history_path"], history_path_arg);
    assert_eq!(value["ready_task_entries"], 1);
    assert_eq!(value["tasks"][0]["role"], "planner");
    assert_eq!(value["tasks"].as_array().expect("tasks should be an array").len(), 1);
    assert!(value["round_id"].as_str().expect("round id should be a string").starts_with("moat-round-"));
    assert!(value["tasks"][0]["node_id"].as_str().expect("node id should be a string").contains("market"));
    assert_eq!(value["tasks"][0]["spec_ref"], serde_json::Value::Null);
}
```

- [x] **Step 2: Run the JSON test to verify RED**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_ready_tasks_json_prints_parseable_filtered_envelope -- --nocapture`

Expected: FAIL with `unknown flag: --format` or non-JSON output parse failure.

- [x] **Step 3: Write failing parser guard tests**

Add these tests near the new JSON output test in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn moat_ready_tasks_rejects_missing_format_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "ready-tasks", "--format"])
        .output()
        .expect("failed to run mdid-cli moat ready-tasks with missing format value");

    assert!(!output.status.success(), "missing format should fail");
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("missing value for moat ready-tasks --format"));
}

#[test]
fn moat_ready_tasks_rejects_duplicate_format_filter() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "ready-tasks", "--format", "json", "--format", "text"])
        .output()
        .expect("failed to run mdid-cli moat ready-tasks with duplicate format flag");

    assert!(!output.status.success(), "duplicate format should fail");
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("duplicate moat ready-tasks --format"));
}

#[test]
fn moat_ready_tasks_rejects_unknown_format_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "ready-tasks", "--format", "yaml"])
        .output()
        .expect("failed to run mdid-cli moat ready-tasks with unknown format value");

    assert!(!output.status.success(), "unknown format should fail");
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("unknown moat ready-tasks format: yaml"));
}
```

- [x] **Step 4: Run parser tests to verify RED**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_ready_tasks_rejects_ -- --nocapture`

Expected: FAIL because `--format` is not implemented for `ready-tasks`.

- [x] **Step 5: Implement minimal parser and renderer**

In `crates/mdid-cli/src/main.rs`, add `output_format: MoatOutputFormat` to `MoatReadyTasksCommand`, default it to `MoatOutputFormat::Text`, parse `--format text|json`, reject duplicate/missing/unknown values with the exact messages from tests, and branch `run_moat_ready_tasks` output:

```rust
#[derive(serde::Serialize)]
struct ReadyTaskJsonRow {
    role: String,
    kind: String,
    node_id: String,
    title: String,
    spec_ref: Option<String>,
}

#[derive(serde::Serialize)]
struct ReadyTasksJsonEnvelope {
    #[serde(rename = "type")]
    envelope_type: &'static str,
    round_id: String,
    history_path: String,
    ready_task_entries: usize,
    tasks: Vec<ReadyTaskJsonRow>,
}
```

Use `serde_json::to_string_pretty(&envelope)` and print it. Preserve the existing text output exactly when no `--format` is supplied or `--format text` is supplied.

- [x] **Step 6: Run targeted GREEN tests**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_ready_tasks_json -- --nocapture`

Expected: PASS.

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_ready_tasks_rejects_ -- --nocapture`

Expected: PASS.

- [x] **Step 7: Run broader ready-tasks regression tests**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli ready_tasks -- --nocapture`

Expected: PASS.

- [x] **Step 8: Truth-sync docs**

Update the ready-tasks bullet in `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md` so the command includes `[--format text|json]` and documents the JSON envelope fields: `type`, `round_id`, `history_path`, `ready_task_entries`, and `tasks[]` with `role`, `kind`, `node_id`, `title`, and nullable `spec_ref`.

Update `README.md` Moat Loop Foundation section to show:

```bash
cargo run -p mdid-cli -- moat ready-tasks --history-path .mdid/moat-history.json --format json
```

and state that `ready-tasks --format json` is read-only and intended for external controllers.

- [x] **Step 9: Verify docs and tests**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli ready_tasks -- --nocapture`

Expected: PASS.

Run: `git diff -- README.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs`

Expected: Diff contains only ready-tasks JSON output changes and docs truth-sync.

- [x] **Step 10: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs README.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-28-med-de-id-moat-ready-tasks-json-envelope.md
git commit -m "feat(cli): emit moat ready tasks as json"
```

## Self-Review

- Spec coverage: The plan implements parseable ready-task routing output for external controllers, preserving read-only bounded semantics and text compatibility.
- Placeholder scan: No TBD, TODO, implement-later, or unspecified test steps remain.
- Type consistency: `MoatOutputFormat`, `ReadyTasksJsonEnvelope`, `ReadyTaskJsonRow`, `ready_task_entries`, and `tasks` names are used consistently.
