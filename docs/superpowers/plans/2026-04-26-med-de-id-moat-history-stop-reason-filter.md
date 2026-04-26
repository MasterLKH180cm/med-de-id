# Moat History Stop Reason Filter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `mdid-cli moat history --stop-reason-contains TEXT` so operators can isolate stopped moat rounds by the recorded stop reason.

**Architecture:** Extend the existing moat history CLI command parser and in-memory filtering pipeline. The flag composes with existing `--decision`, `--contains`, `--min-score`, and `--limit` filters and reuses the current filtered history summary and escaped round output.

**Tech Stack:** Rust, Cargo workspace, `mdid-cli` integration tests, existing `LocalMoatHistoryStore` persisted JSON history.

---

## File Structure

- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add one integration test that persists a continuing round and a stopped round, then verifies `--stop-reason-contains budget` returns only the stopped round and filtered summary.
- Modify: `crates/mdid-cli/src/main.rs`
  - Add `stop_reason_contains: Option<String>` to `MoatHistoryCommand`.
  - Parse `--stop-reason-contains TEXT` with duplicate and missing-value validation.
  - Include the new filter in `run_moat_history`.
  - Treat the new filter as a filtered-summary trigger.
  - Update usage text and parser unit test expected struct.

### Task 1: Add moat history stop-reason filter

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`

- [ ] **Step 1: Write the failing integration test**

Add this test near the existing moat history filter tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn cli_filters_moat_history_rounds_by_stop_reason_text() {
    let history_path = unique_history_path("history-stop-reason-filter");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let continuing_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to run continuing mdid-cli moat round with history path");
    assert!(
        continuing_output.status.success(),
        "expected continuing round success, stderr was: {}",
        String::from_utf8_lossy(&continuing_output.stderr)
    );

    let stopped_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--review-loops",
            "0",
            "--history-path",
            history_path_arg,
        ])
        .output()
        .expect("failed to run stopped mdid-cli moat round with history path");
    assert!(
        stopped_output.status.success(),
        "expected stopped round success, stderr was: {}",
        String::from_utf8_lossy(&stopped_output.stderr)
    );

    let store = LocalMoatHistoryStore::open(&history_path).expect("history store should open");
    let stopped_round_id = store.entries()[1].report.summary.round_id.to_string();

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
        "expected success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "moat history filtered summary\n",
            "entries=1\n",
            "latest_round_id={stopped_round_id}\n",
            "latest_continue_decision=Stop\n",
            "latest_stop_reason=review budget exhausted\n",
            "latest_decision_summary=implementation stopped before review\n",
            "latest_implemented_specs=moat-spec/workflow-audit\n",
            "latest_moat_score_after=90\n",
            "best_moat_score_after=90\n",
            "improvement_deltas=0\n",
            "history_rounds=1\n",
            "round={stopped_round_id}|Stop|90|review budget exhausted\n",
        )
        .replace("{stopped_round_id}", &stopped_round_id)
    );

    cleanup_history_path(&history_path);
}
```

- [ ] **Step 2: Run the targeted test to verify RED**

Run:

```bash
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/tmp/med-de-id-moat-loop-target cargo test -p mdid-cli --test moat_cli cli_filters_moat_history_rounds_by_stop_reason_text -- --nocapture
```

Expected: FAIL with `unknown moat history flag: --stop-reason-contains`.

- [ ] **Step 3: Implement the minimal parser and filter**

Make these exact production changes in `crates/mdid-cli/src/main.rs`:

```rust
struct MoatHistoryCommand {
    history_path: String,
    decision: Option<ContinueDecision>,
    contains: Option<String>,
    stop_reason_contains: Option<String>,
    min_score: Option<u32>,
    limit: Option<usize>,
}
```

In `parse_moat_history_command`, declare `let mut stop_reason_contains = None;`, add this match arm before `--limit`, and include `stop_reason_contains` in the returned struct:

```rust
"--stop-reason-contains" => {
    let value = args
        .get(index + 1)
        .filter(|value| !value.starts_with("--"))
        .ok_or_else(|| "--stop-reason-contains requires a value".to_string())?;
    if stop_reason_contains.is_some() {
        return Err(duplicate_flag_error("--stop-reason-contains"));
    }
    stop_reason_contains = Some(value.clone());
    index += 2;
}
```

In `run_moat_history`, add this filter after the existing `contains` filter:

```rust
.filter(|entry| {
    command
        .stop_reason_contains
        .as_ref()
        .map(|needle| {
            entry
                .report
                .stop_reason
                .as_deref()
                .map(|stop_reason| stop_reason.contains(needle))
                .unwrap_or(false)
        })
        .unwrap_or(true)
})
```

Change the filtered-summary trigger to:

```rust
if command.contains.is_some() || command.stop_reason_contains.is_some() || command.min_score.is_some() {
```

Update `usage()` so the history section reads:

```text
moat history --history-path PATH [--decision Continue|Stop] [--contains TEXT] [--stop-reason-contains TEXT] [--min-score N] [--limit N]
```

Update parser unit-test expected `MoatHistoryCommand` literals to include `stop_reason_contains: None,`.

- [ ] **Step 4: Run the targeted test to verify GREEN**

Run:

```bash
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/tmp/med-de-id-moat-loop-target cargo test -p mdid-cli --test moat_cli cli_filters_moat_history_rounds_by_stop_reason_text -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Run relevant broader tests**

Run:

```bash
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/tmp/med-de-id-moat-loop-target cargo test -p mdid-cli --test moat_cli moat_history -- --nocapture
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/tmp/med-de-id-moat-loop-target cargo test -p mdid-cli parse_moat_history_command -- --nocapture
```

Expected: PASS for both commands.

- [ ] **Step 6: Commit**

Run:

```bash
git add crates/mdid-cli/tests/moat_cli.rs crates/mdid-cli/src/main.rs docs/superpowers/plans/2026-04-26-med-de-id-moat-history-stop-reason-filter.md
git commit -m "feat: filter moat history by stop reason"
```

Expected: commit succeeds on `feature/moat-loop-autonomy`.

## Self-Review

- Spec coverage: This plan covers the new stop-reason history inspection capability in the autonomous moat-loop CLI without touching DICOM/product mainline code.
- Placeholder scan: No TBD/TODO/fill-in/similar placeholders are present.
- Type consistency: The new CLI flag, struct field, parser, runner filter, and tests all use `stop_reason_contains` / `--stop-reason-contains` consistently.
