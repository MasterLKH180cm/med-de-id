# Moat Complete Task JSON Envelope Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add machine-readable JSON output to `mdid-cli moat complete-task` so external autonomous controllers can complete a claimed task, record an artifact handoff, and route newly ready downstream tasks without scraping text rows.

**Architecture:** Extend the existing bounded `complete-task` mutation with `--format text|json` at the CLI boundary only. Preserve the current text output as the default. JSON output should be a deterministic envelope over the same persisted state transition and downstream ready-task calculation that text output already uses. The command still opens only local history, reloads before mutation, completes exactly one in-progress task, optionally records a paired artifact, and never launches agents, appends rounds, schedules work, crawls data, opens PRs, creates cron jobs, or writes artifact files.

**Tech Stack:** Rust workspace, `mdid-cli`, local JSON moat history fixtures, `serde_json`, Cargo integration tests, TDD.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `format: MoatOutputFormat` to `MoatCompleteTaskCommand`.
  - Parse `--format text|json` for `moat complete-task` with duplicate, missing, and invalid-value errors.
  - Render `--format json` as a pretty deterministic envelope after the same mutation and downstream ready-task derivation used by text output.
  - Update `moat complete-task` usage text.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add RED/GREEN integration tests for JSON completion output, downstream ready-task JSON rows, artifact metadata, and invalid format handling.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Truth-sync the shipped `complete-task` CLI contract after implementation.
- Modify: `README.md`
  - Document the controller-facing JSON completion envelope.
- Modify: `docs/superpowers/plans/2026-04-28-med-de-id-moat-complete-task-json-envelope.md`
  - Keep task checkboxes and completion evidence current during implementation.

### Task 1: Add JSON output to `moat complete-task`

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
- Modify: `README.md`
- Modify: `docs/superpowers/plans/2026-04-28-med-de-id-moat-complete-task-json-envelope.md`

- [x] **Step 1: Write the failing JSON completion envelope test**

Add this test near the existing `complete-task` integration tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn moat_complete_task_json_prints_artifact_and_downstream_ready_envelope() {
    let temp = tempdir().expect("failed to create tempdir");
    let history_path = temp.path().join("moat-history.json");
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
    assert!(!json["round_id"].as_str().expect("round id string").is_empty());
    assert_eq!(json["node_id"], "spec-workflow-audit");
    assert_eq!(json["previous_state"], "in_progress");
    assert_eq!(json["new_state"], "completed");
    assert_eq!(json["artifact_recorded"], true);
    assert_eq!(json["artifact"]["ref"], "docs/superpowers/specs/workflow-audit.md");
    assert_eq!(json["artifact"]["summary"], "Workflow audit spec drafted");
    assert_eq!(json["next_ready_task_entries"], json["next_ready_tasks"].as_array().expect("next ready array").len());
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
}
```

- [x] **Step 2: Run the focused test to verify RED**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_complete_task_json_prints_artifact_and_downstream_ready_envelope -- --nocapture`

Expected: FAIL with `unknown option for moat complete-task: --format`, `unknown flag: --format`, or non-JSON stdout.

- [x] **Step 3: Add parser guard tests**

Add these tests near the new JSON completion test:

```rust
#[test]
fn moat_complete_task_rejects_missing_format_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "complete-task", "--history-path", "history.json", "--node-id", "node-1", "--format"])
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
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("duplicate moat complete-task --format"));
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
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("unknown moat complete-task format: yaml"));
}
```

- [x] **Step 4: Run parser tests to verify RED**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_complete_task_rejects_ -- --nocapture`

Expected: FAIL because `--format` is not implemented for `complete-task` yet.

- [x] **Step 5: Implement minimal parser and JSON renderer**

In `crates/mdid-cli/src/main.rs`:

1. Reuse the existing `MoatOutputFormat` enum if already present for other moat commands; otherwise add one local enum shared by the moat command parsers:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MoatOutputFormat {
    Text,
    Json,
}
```

2. Add `format: MoatOutputFormat` to `MoatCompleteTaskCommand` and default it to `MoatOutputFormat::Text`.
3. Parse `--format text|json` in the `complete-task` parser. Reject duplicate values with `duplicate moat complete-task --format`, missing values with `missing value for moat complete-task --format`, and unknown values with `unknown moat complete-task format: VALUE`.
4. Preserve the existing text output exactly when no `--format` is supplied or `--format text` is supplied.
5. For `--format json`, perform the same mutation as text output, reload/derive the same downstream ready tasks, then print a pretty deterministic envelope:

```json
{
  "type": "moat_complete_task",
  "round_id": "moat-round-001",
  "history_path": ".mdid/moat-history.json",
  "node_id": "spec-workflow-audit",
  "previous_state": "in_progress",
  "new_state": "completed",
  "artifact_recorded": true,
  "artifact": {
    "ref": "docs/superpowers/specs/workflow-audit.md",
    "summary": "Workflow audit spec drafted"
  },
  "next_ready_task_entries": 1,
  "next_ready_tasks": [
    {
      "role": "coder",
      "kind": "implementation",
      "node_id": "implement-workflow-audit",
      "title": "Implement workflow audit moat strategy",
      "spec_ref": "moat-spec/workflow-audit"
    }
  ]
}
```

When no artifact is recorded, emit `"artifact_recorded": false` and `"artifact": null`. Emit `spec_ref: null` for downstream ready tasks without a spec reference. Use `serde_json::to_string_pretty` and ensure no text header or compatibility rows are mixed into JSON stdout.

- [x] **Step 6: Run focused GREEN tests**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_complete_task_json_prints_artifact_and_downstream_ready_envelope -- --nocapture`

Expected: PASS.

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_complete_task_rejects_ -- --nocapture`

Expected: PASS.

- [x] **Step 7: Run broader complete-task regression tests**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli complete_task -- --nocapture`

Expected: PASS.

- [x] **Step 8: Truth-sync docs**

Update the `complete-task` bullet in `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md` so the command includes `[--format text|json]` and documents that JSON emits a deterministic envelope with `type`, `round_id`, `history_path`, `node_id`, `previous_state`, `new_state`, `artifact_recorded`, nullable `artifact`, `next_ready_task_entries`, and `next_ready_tasks[]` fields.

Update `README.md` Moat Loop Foundation documentation to include this controller example:

```bash
cargo run -p mdid-cli -- moat complete-task \
  --history-path .mdid/moat-history.json \
  --node-id spec-workflow-audit \
  --artifact-ref docs/superpowers/specs/workflow-audit.md \
  --artifact-summary "Workflow audit spec drafted" \
  --format json
```

State that `complete-task --format json` is intended for external controllers that need parseable completion, artifact handoff, and downstream routing metadata from one local mutation.

- [x] **Step 9: Verify docs and tests**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli complete_task -- --nocapture`

Expected: PASS.

Run: `git diff -- README.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs docs/superpowers/plans/2026-04-28-med-de-id-moat-complete-task-json-envelope.md`

Expected: Diff contains only `complete-task --format text|json`, JSON envelope rendering/tests, and docs truth-sync.

- [x] **Step 10: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs README.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-28-med-de-id-moat-complete-task-json-envelope.md
git commit -m "feat(cli): emit moat complete task as json"
```

## Evidence

- RED focused JSON envelope test: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_complete_task_json_prints_artifact_and_downstream_ready_envelope -- --nocapture` failed before implementation because `moat complete-task` did not accept or emit `--format json`.
- RED parser guard tests: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_complete_task_rejects_ -- --nocapture` failed before parser support because `--format` was not implemented for `complete-task`.
- GREEN focused JSON envelope test: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_complete_task_json_prints_artifact_and_downstream_ready_envelope -- --nocapture` passed after implementation.
- GREEN parser guard tests: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_complete_task_rejects_ -- --nocapture` passed after implementation.
- GREEN broader regression: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli complete_task -- --nocapture` passed with 9 tests passed, 0 failed, 275 filtered out.
- Verification: `git diff --check` passed.

## Self-Review

- Spec coverage: This is the next strongest controller-automation slice after ready-task and task-event JSON because it makes the task completion/artifact/downstream-routing mutation machine-readable without introducing process execution, daemons, PR automation, or crawlers.
- Release size: One CLI flag and one envelope on an existing command; no new domain workflow is required.
- TDD quality: The plan includes concrete failing integration tests, exact commands for RED/GREEN verification, and broader regression coverage.
- Placeholder scan: No unresolved placeholder markers are present.
- Scope guard: The implementation is limited to CLI parsing/rendering, tests, and docs truth-sync; production behavior remains local-first and bounded.
