# Moat Assigned-Agent Filter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add read-only `--assigned-agent-id AGENT_ID` filters to moat task ownership inspection commands so external multi-agent controllers can deterministically find work owned by a specific worker.

**Architecture:** Extend the existing CLI filter structs for `moat task-graph` and `moat assignments` with an optional exact-match persisted owner filter. Filtering remains read-only and bounded: it only inspects already-persisted history, never claims tasks, launches agents, schedules work, opens PRs, or creates cron jobs.

**Tech Stack:** Rust workspace, `mdid-cli`, persisted moat history JSON, Cargo integration tests, README/spec truth-sync.

---

## File Structure

- Modify `crates/mdid-cli/src/main.rs`: parse `--assigned-agent-id AGENT_ID` for `moat task-graph` and `moat assignments`; apply exact matching against `node.assigned_agent_id.as_deref()`; update CLI usage text.
- Modify `crates/mdid-cli/tests/moat_cli.rs`: add integration tests that persist ownership through `dispatch-next --agent-id` and verify task-graph/assignments filters show matching owned tasks and hide non-matching owners.
- Modify `README.md`: document the new read-only owner filters in the moat-loop CLI section.
- Modify `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`: truth-sync shipped command signatures and bounded behavior for assigned-agent filters.

### Task 1: Task-graph assigned-agent filter

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`

- [ ] **Step 1: Write failing CLI tests**

Add two tests near the existing `moat_task_graph_*` tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn moat_task_graph_filters_by_assigned_agent_id() {
    let history_path = unique_history_path("task-graph-assigned-agent");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed task graph assigned-agent history");
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));

    let dispatch = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path_arg,
            "--node-id",
            "implementation",
            "--agent-id",
            "coder-7",
        ])
        .output()
        .expect("failed to dispatch owned implementation task");
    assert!(dispatch.status.success(), "{}", String::from_utf8_lossy(&dispatch.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--assigned-agent-id",
            "coder-7",
        ])
        .output()
        .expect("failed to inspect task graph by assigned agent");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("node=coder|implementation|"), "{stdout}");
    assert!(stdout.contains("assigned_agent_id=implementation|coder-7"), "{stdout}");
    assert!(!stdout.contains("node=planner|market_scan|"), "{stdout}");
}

#[test]
fn moat_task_graph_assigned_agent_filter_with_no_match_prints_no_nodes() {
    let history_path = unique_history_path("task-graph-assigned-agent-none");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed task graph assigned-agent non-match history");
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--assigned-agent-id",
            "coder-7",
        ])
        .output()
        .expect("failed to inspect task graph by non-matching assigned agent");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("moat task graph"), "{stdout}");
    assert!(!stdout.contains("node="), "{stdout}");
    assert!(!stdout.contains("assigned_agent_id="), "{stdout}");
}
```

- [ ] **Step 2: Run tests to verify RED**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_task_graph_filters_by_assigned_agent_id moat_task_graph_assigned_agent_filter_with_no_match_prints_no_nodes -- --nocapture`
Expected: FAIL with `unknown option for moat task-graph: --assigned-agent-id`.

- [ ] **Step 3: Implement minimal task-graph filter**

In `crates/mdid-cli/src/main.rs`, add `assigned_agent_id: Option<String>` to `MoatTaskGraphCommand`, parse `--assigned-agent-id`, reject duplicate and missing values using existing flag helpers, and include this predicate in task-graph rendering:

```rust
if let Some(expected_agent_id) = command.assigned_agent_id.as_deref() {
    if node.assigned_agent_id.as_deref() != Some(expected_agent_id) {
        continue;
    }
}
```

Update `usage()` so the `moat task-graph` command shows `[--assigned-agent-id AGENT_ID]`.

- [ ] **Step 4: Run task-graph verification**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_task_graph_filters_by_assigned_agent_id moat_task_graph_assigned_agent_filter_with_no_match_prints_no_nodes -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit task-graph filter**

Run: `git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs && git commit -m "feat(cli): filter moat task graph by assigned agent"`

### Task 2: Assignments assigned-agent filter and docs truth-sync

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`

- [ ] **Step 1: Write failing CLI tests**

Add these tests near the existing `moat_assignments_*` tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn moat_assignments_filters_by_assigned_agent_id() {
    let history_path = unique_history_path("assignments-assigned-agent");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed assignments assigned-agent history");
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));

    let dispatch = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path_arg,
            "--node-id",
            "implementation",
            "--agent-id",
            "coder-7",
        ])
        .output()
        .expect("failed to dispatch owned assignment task");
    assert!(dispatch.status.success(), "{}", String::from_utf8_lossy(&dispatch.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--assigned-agent-id",
            "coder-7",
        ])
        .output()
        .expect("failed to inspect assignments by assigned agent");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("assignment=coder|implementation|"), "{stdout}");
    assert!(!stdout.contains("assignment=planner|market_scan|"), "{stdout}");
}

#[test]
fn moat_assignments_assigned_agent_filter_with_no_match_prints_zero_entries() {
    let history_path = unique_history_path("assignments-assigned-agent-none");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed assignments assigned-agent non-match history");
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--assigned-agent-id",
            "coder-7",
        ])
        .output()
        .expect("failed to inspect assignments by non-matching assigned agent");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("assignment_entries=0"), "{stdout}");
    assert!(!stdout.contains("assignment="), "{stdout}");
}
```

- [ ] **Step 2: Run tests to verify RED**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_assignments_filters_by_assigned_agent_id moat_assignments_assigned_agent_filter_with_no_match_prints_zero_entries -- --nocapture`
Expected: FAIL with `unknown option for moat assignments: --assigned-agent-id`.

- [ ] **Step 3: Implement minimal assignments filter**

In `crates/mdid-cli/src/main.rs`, add `assigned_agent_id: Option<String>` to `MoatAssignmentsCommand`, parse `--assigned-agent-id`, reject duplicate and missing values, and include this predicate when projecting persisted assignment nodes:

```rust
if let Some(expected_agent_id) = command.assigned_agent_id.as_deref() {
    if node.assigned_agent_id.as_deref() != Some(expected_agent_id) {
        continue;
    }
}
```

Update `usage()` so the `moat assignments` command shows `[--assigned-agent-id AGENT_ID]`.

- [ ] **Step 4: Update README and spec**

In `README.md`, update the moat-loop CLI docs to mention:

```text
`moat task-graph` and `moat assignments` accept `--assigned-agent-id AGENT_ID` to inspect only tasks currently owned by that persisted local worker ID. This is read-only inspection; it does not spawn, schedule, or supervise agents.
```

In `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`, update the shipped status bullets for `moat task-graph` and `moat assignments` so their command signatures include `[--assigned-agent-id AGENT_ID]` and their filter descriptions state exact matching against persisted `assigned_agent_id`.

- [ ] **Step 5: Run CLI verification**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_assignments_filters_by_assigned_agent_id moat_assignments_assigned_agent_filter_with_no_match_prints_zero_entries -- --nocapture`
Expected: PASS.

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli assigned_agent -- --nocapture`
Expected: PASS for all assigned-agent CLI integration tests selected by the substring filter.

- [ ] **Step 6: Run broader package verification**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli`
Expected: PASS.

- [ ] **Step 7: Commit assignments/docs slice**

Run: `git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs README.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-27-med-de-id-moat-assigned-agent-filter.md && git commit -m "feat(cli): filter moat ownership by assigned agent"`

## Self-Review

- Spec coverage: The plan covers exact persisted owner filtering for both existing ownership inspection surfaces, command usage, README, and moat-loop design spec truth-sync.
- Placeholder scan: No TBD, TODO, implement-later, or vague edge-case placeholders remain.
- Type consistency: The flag is consistently named `--assigned-agent-id`; command fields are `assigned_agent_id: Option<String>`; predicate compares against `node.assigned_agent_id.as_deref()`.
