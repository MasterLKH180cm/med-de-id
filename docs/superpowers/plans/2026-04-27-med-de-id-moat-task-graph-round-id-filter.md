# Moat Task Graph Round ID Filter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `--round-id ROUND_ID` to `mdid-cli moat task-graph` so operators can inspect the persisted task graph for a specific moat loop round instead of only the latest round.

**Architecture:** Extend only the task graph CLI command with an optional exact round-id selector. Preserve existing default behavior by continuing to inspect the latest persisted round when the selector is absent, and return the standard header with no node rows when the requested round is not found.

**Tech Stack:** Rust, Cargo workspace, `mdid-cli` integration tests, local JSON moat history store.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `round_id: Option<String>` to `MoatTaskGraphCommand`.
  - Update `USAGE` to advertise `[--round-id ROUND_ID]` for `moat task-graph`.
  - Parse `--round-id` as a duplicate-rejected string flag in `parse_moat_task_graph_command`.
  - In `run_moat_task_graph`, select the latest entry by default or the exact matching `entry.report.summary.round_id` when provided.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Update the test-local `USAGE` string for `moat task-graph`.
  - Add exact round-id selection test.
  - Add unknown round-id empty-result test.
  - Add duplicate flag rejection test.
- Modify: `README.md`
  - Update the `moat task-graph` usage example with `[--round-id ROUND_ID]`.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Truth-sync task graph inspection docs to mention default-latest plus exact round-id selection.

### Task 1: Add task graph `--round-id` filter

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
- Test: `crates/mdid-cli/tests/moat_cli.rs`

- [x] **Step 1: Write the failing exact round-id filter test**

Add this test near the other moat task graph filter tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn moat_task_graph_filters_nodes_by_exact_round_id() {
    let history_path = unique_history_path("task-graph-round-id");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let first = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--history-path",
            history_path_arg,
            "--review-loops",
            "0",
        ])
        .output()
        .expect("failed to seed first task graph round");
    assert!(first.status.success(), "{}", String::from_utf8_lossy(&first.stderr));

    let second = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed second task graph round");
    assert!(second.status.success(), "{}", String::from_utf8_lossy(&second.stderr));

    let store = LocalMoatHistoryStore::open_existing(&history_path)
        .expect("history should exist after seeding rounds");
    let entries = store.entries();
    assert_eq!(entries.len(), 2);
    let first_round_id = entries[0].report.summary.round_id.to_string();

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

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("node=reviewer|review|Review|review|ready|implementation|<none>\n"));
    assert!(!stdout.contains("node=reviewer|review|Review|review|completed|implementation|<none>\n"));

    cleanup_history_path(&history_path);
}
```

- [x] **Step 2: Run the focused test to verify RED**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_task_graph_filters_nodes_by_exact_round_id -- --exact --nocapture
```

Expected: FAIL because `--round-id` is currently reported as an unknown moat task-graph flag.

- [x] **Step 3: Implement minimal exact round-id support**

In `crates/mdid-cli/src/main.rs`, add `round_id: Option<String>` to `MoatTaskGraphCommand`, initialize it to `None` in `parse_moat_task_graph_command`, parse `--round-id` with duplicate rejection, include it in the returned struct, and replace the `latest` lookup in `run_moat_task_graph` with:

```rust
    let maybe_entry = if let Some(round_id) = command.round_id.as_deref() {
        store
            .entries()
            .iter()
            .find(|entry| entry.report.summary.round_id.to_string() == round_id)
    } else {
        store.entries().last()
    };

    println!("moat task graph");
    let Some(latest) = maybe_entry else {
        return Ok(());
    };
```

Also update the `moat task-graph` usage line in both `crates/mdid-cli/src/main.rs` and `crates/mdid-cli/tests/moat_cli.rs` to include `[--round-id ROUND_ID]` immediately after `--history-path PATH`.

- [x] **Step 4: Run the focused test to verify GREEN**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_task_graph_filters_nodes_by_exact_round_id -- --exact --nocapture
```

Expected: PASS.

- [x] **Step 5: Add unknown round-id empty-result test**

Add this test near `moat_task_graph_filters_nodes_by_exact_round_id`:

```rust
#[test]
fn moat_task_graph_reports_no_nodes_for_unknown_round_id() {
    let history_path = unique_history_path("task-graph-round-id-unknown");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed task graph history for unknown round id filter");
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--round-id",
            "00000000-0000-4000-8000-000000000000",
        ])
        .output()
        .expect("failed to inspect task graph by unknown round id");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "moat task graph\n");

    cleanup_history_path(&history_path);
}
```

- [x] **Step 6: Run the unknown round-id test**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_task_graph_reports_no_nodes_for_unknown_round_id -- --exact --nocapture
```

Expected: PASS.

- [x] **Step 7: Add duplicate flag rejection test**

Add this parser test near the task graph duplicate flag tests:

```rust
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
            "round-a",
            "--round-id",
            "round-b",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with duplicate round id filter");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("duplicate flag: --round-id"));
    assert!(!history_path.exists());
}
```

- [x] **Step 8: Run the duplicate flag test**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_task_graph_rejects_duplicate_round_id_filter -- --exact --nocapture
```

Expected: PASS.

- [x] **Step 9: Truth-sync docs**

Update `README.md` and `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md` so the documented task-graph filters include `[--round-id ROUND_ID]` and explain that the command defaults to the latest persisted round when the flag is absent.

- [x] **Step 10: Run relevant broader validation**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli round_id -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli task_graph -- --nocapture
```

Expected: PASS for all selected tests. If the second command is too broad for local disk pressure, run the three new task graph round-id tests plus the existing task graph dependency/no-dependency tests individually.

- [x] **Step 11: Commit the slice**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs README.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-27-med-de-id-moat-task-graph-round-id-filter.md
git commit -m "feat: filter moat task graph by round id"
```

## Self-Review

- Spec coverage: The plan adds exact round-id task graph inspection, default-latest preservation, unknown round empty output, duplicate flag rejection, usage text, and docs truth-sync.
- Placeholder scan: No TBD/TODO/fill-in placeholders remain.
- Type consistency: The new field is consistently named `round_id: Option<String>` and uses existing persisted `entry.report.summary.round_id` values.
