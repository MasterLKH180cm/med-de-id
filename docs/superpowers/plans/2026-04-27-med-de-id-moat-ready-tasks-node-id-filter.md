# med-de-id Moat Ready Tasks Node ID Filter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an exact `--node-id NODE_ID` filter to `mdid-cli moat ready-tasks` so autonomous controllers can route or claim a specific ready task without scanning unrelated rows.

**Architecture:** Keep the behavior bounded and local-first: parse the optional filter in the CLI, apply it after deriving ready nodes and before limit truncation, and preserve existing read-only semantics. The filter exact-matches persisted task node IDs and never mutates history or launches agents.

**Tech Stack:** Rust workspace, Cargo integration tests, `mdid-cli`, `mdid-runtime::moat_history`.

---

## File Structure

- Modify `crates/mdid-cli/src/main.rs`: extend `MoatReadyTasksCommand`, parse `--node-id`, include it in ready-task filtering, and update usage text.
- Modify `crates/mdid-cli/tests/moat_cli.rs`: add CLI integration tests for exact matching and no-match success output.
- Modify `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`: truth-sync shipped foundation description for the new read-only filter.

### Task 1: Ready Tasks Exact Node ID Filter

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`

- [ ] **Step 1: Write the failing exact-match test**

Add this test near the existing `ready-tasks` CLI tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn cli_filters_ready_tasks_by_exact_node_id() {
    let history_path = unique_history_path("ready-tasks-node-id-filter");
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
            "competitor-analysis",
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
            "ready_task=Planner|competitor-analysis|Analyze de-identification competitors|competitor_analysis|moat-spec/competitor-map\n",
        )
    );

    cleanup_history_path(&history_path);
}
```

- [ ] **Step 2: Write the failing no-match test**

Add this adjacent test in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
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
```

- [ ] **Step 3: Run tests to verify RED**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli ready_tasks_node_id -- --nocapture
```

Expected: FAIL because `--node-id` is an unexpected argument or the command does not filter by node ID.

- [ ] **Step 4: Implement minimal CLI parsing and filtering**

In `crates/mdid-cli/src/main.rs`, add `node_id: Option<String>` to `MoatReadyTasksCommand`, parse `--node-id NODE_ID` with duplicate detection in `parse_moat_ready_tasks_command`, and apply this exact match in `ready_task_matches` before `--limit` truncation:

```rust
if let Some(expected_node_id) = command.node_id.as_deref() {
    if task.node_id != expected_node_id {
        return false;
    }
}
```

Also update the `USAGE` string so the `moat ready-tasks` synopsis includes `[--node-id NODE_ID]`.

- [ ] **Step 5: Truth-sync spec**

In `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`, update the shipped foundation bullet for `mdid-cli moat ready-tasks` so it documents `[--node-id NODE_ID]` as a read-only exact persisted node ID filter that combines conjunctively with round/role/kind and applies before `--limit`.

- [ ] **Step 6: Run targeted tests to verify GREEN**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli ready_tasks_node_id -- --nocapture
```

Expected: PASS for both new tests.

- [ ] **Step 7: Run broader relevant verification**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli ready_tasks -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli -- --nocapture
```

Expected: all selected `moat_cli` tests pass.

- [ ] **Step 8: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-27-med-de-id-moat-ready-tasks-node-id-filter.md
git commit -m "feat: filter moat ready tasks by node id"
```

## Self-Review

- Spec coverage: The plan implements a bounded read-only routing filter that directly advances the Autonomous Multi-Agent System control-plane handoff path.
- Placeholder scan: No TBD/TODO/fill-in placeholders remain.
- Type consistency: `node_id` matches the existing persisted field naming and the CLI flag naming used by task-graph, assignments, artifacts, claim, complete, release, block, and unblock commands.
