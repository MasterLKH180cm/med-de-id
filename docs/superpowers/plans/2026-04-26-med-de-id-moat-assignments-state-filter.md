# Med De-ID Moat Assignments State Filter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `mdid-cli moat assignments --state <state>` filter so operators can list assigned moat-loop work by task state.

**Architecture:** Keep the CLI as the orchestration surface and reuse the existing task graph state parser and formatter. Assignment filtering will join each assignment to the latest control-plane task graph by `node_id` and include only assignments whose task node state matches the requested state; assignments without a matching task node are excluded when a state filter is supplied.

**Tech Stack:** Rust workspace, `mdid-cli`, `assert_cmd`, TDD with Cargo integration tests.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `state: Option<MoatTaskNodeState>` to `MoatAssignmentsCommand`.
  - Parse `--state` in `parse_moat_assignments_command` using the existing `parse_moat_task_graph_state_filter` helper.
  - Apply the state filter in `run_moat_assignments` by looking up the assignment's `node_id` in the latest task graph.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add an integration test that generates a history, runs `mdid-cli moat assignments --state ready`, and verifies only ready assignments are emitted.

### Task 1: Assignments State Filter

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/moat_cli.rs`

- [ ] **Step 1: Write the failing test**

Add this test to `crates/mdid-cli/tests/moat_cli.rs` near the other `moat assignments` filter tests:

```rust
#[test]
fn moat_assignments_filters_by_task_state() {
    let temp = tempdir().unwrap();
    let history_path = temp.path().join("moat-history.jsonl");

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "moat",
            "round",
            "--history-path",
            history_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path.to_str().unwrap(),
            "--state",
            "ready",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("assignment_entries=1"))
        .stdout(predicate::str::contains("assignment=strategy-agent|strategy-discovery|"))
        .stdout(predicate::str::contains("assignment=spec-agent|spec-authoring|").not())
        .stdout(predicate::str::contains("assignment=implementation-agent|implementation-slice|").not());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run with disk-conscious settings:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_assignments_filters_by_task_state -- --nocapture
```

Expected: FAIL with `unknown moat assignments flag: --state`.

- [ ] **Step 3: Write minimal implementation**

In `crates/mdid-cli/src/main.rs`, change `MoatAssignmentsCommand` to include state:

```rust
struct MoatAssignmentsCommand {
    history_path: String,
    role: Option<AgentRole>,
    state: Option<MoatTaskNodeState>,
    kind: Option<MoatTaskNodeKind>,
    node_id: Option<String>,
    title_contains: Option<String>,
    spec_ref: Option<String>,
    contains: Option<String>,
}
```

In `parse_moat_assignments_command`, add `let mut state = None;`, parse the flag:

```rust
"--state" => {
    let value = required_flag_value(args, index, "--state", false)?;
    if state.is_some() {
        return Err(duplicate_flag_error("--state"));
    }
    state = Some(parse_moat_task_graph_state_filter(value)?);
}
```

and include `state,` in the returned `MoatAssignmentsCommand`.

In `run_moat_assignments`, insert a state filter after the `node_id` filter:

```rust
.filter(|assignment| {
    command
        .state
        .map(|expected_state| {
            latest
                .report
                .control_plane
                .task_graph
                .nodes
                .iter()
                .find(|node| node.node_id == assignment.node_id)
                .map(|node| node.state == expected_state)
                .unwrap_or(false)
        })
        .unwrap_or(true)
})
```

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_assignments_filters_by_task_state -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Run related CLI tests**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_assignments -- --nocapture
```

Expected: all matching `moat_assignments*` tests PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs docs/superpowers/plans/2026-04-26-med-de-id-moat-assignments-state-filter.md
git commit -m "feat: filter moat assignments by state"
```

## Self-Review

- Spec coverage: Adds state-level filtering for the assignment projection, reusing existing task graph state vocabulary (`pending`, `ready`, `in_progress`, `completed`, `blocked`).
- Placeholder scan: No placeholders remain.
- Type consistency: `MoatTaskNodeState` is already imported and used by the task graph command; the assignments command now uses the same type and parser.
