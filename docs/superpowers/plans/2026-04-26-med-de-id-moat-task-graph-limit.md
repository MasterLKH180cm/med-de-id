# Moat Task Graph Limit Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a read-only `--limit N` option to `mdid-cli moat task-graph` so operators can inspect bounded slices of the latest autonomous task graph.

**Architecture:** The CLI already parses task-graph filters into `MoatTaskGraphCommand` and renders matching nodes in `run_moat_task_graph`. Extend that command with an optional positive `usize` limit, reuse existing flag parsing/duplicate/missing-value patterns, and apply the limit after all filters so it bounds final output without mutating history.

**Tech Stack:** Rust workspace, `mdid-cli` integration tests, Cargo targeted test execution with `CARGO_INCREMENTAL=0`.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `limit: Option<usize>` to `MoatTaskGraphCommand`.
  - Parse `--limit N` in `parse_moat_task_graph_command` using the same semantics as decision-log limit: positive integer only, duplicate rejected, missing/flag-like values rejected.
  - Include `[--limit N]` in usage text.
  - Apply `.take(limit.unwrap_or(usize::MAX))` after all task graph filters before output.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Update `USAGE` to document task graph `--limit N`.
  - Add behavior tests for limiting output, zero rejection, duplicate rejection, missing-value rejection, and read-only history behavior.

### Task 1: Task Graph Limit CLI Option

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/moat_cli.rs`

- [ ] **Step 1: Write failing tests**

Add these tests near the existing task graph filter tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn task_graph_limit_bounds_rendered_nodes_after_filters() {
    let history_path = unique_history_path("task-graph-limit");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history for task graph limit");
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--role",
            "planner",
            "--limit",
            "2",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with limit");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("moat task graph\n"));
    assert_eq!(stdout.lines().filter(|line| line.starts_with("node=")).count(), 2);
    assert!(stdout.contains("node=planner|market_scan|Market Scan|market_scan|completed|<none>|<none>\n"));
    assert!(stdout.contains("node=planner|competitor_analysis|Competitor Analysis|competitor_analysis|completed|market_scan|<none>\n"));
    assert!(!stdout.contains("node=planner|lockin_analysis|Lock-In Analysis|lock_in_analysis|"));

    cleanup_history_path(&history_path);
}

#[test]
fn task_graph_rejects_zero_limit() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            "/tmp/mdid-unused-history.json",
            "--limit",
            "0",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with zero limit");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("invalid value for --limit: expected positive integer, got 0\n{}\n", USAGE)
    );
}

#[test]
fn task_graph_rejects_duplicate_limit() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            "/tmp/mdid-unused-history.json",
            "--limit",
            "1",
            "--limit",
            "2",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with duplicate limit");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("duplicate flag: --limit\n{}\n", USAGE)
    );
}

#[test]
fn task_graph_rejects_missing_limit_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            "/tmp/mdid-unused-history.json",
            "--limit",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with missing limit value");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("missing value for --limit\n{}\n", USAGE)
    );
}

#[test]
fn task_graph_limit_does_not_append_history() {
    let history_path = unique_history_path("task-graph-limit-read-only");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history for task graph limit read-only check");
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));

    let inspect = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--limit",
            "1",
        ])
        .output()
        .expect("failed to inspect moat task graph with limit");
    assert!(inspect.status.success(), "{}", String::from_utf8_lossy(&inspect.stderr));

    let history = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "history", "--history-path", history_path_arg])
        .output()
        .expect("failed to inspect moat history after task graph limit");
    assert!(history.status.success(), "{}", String::from_utf8_lossy(&history.stderr));
    assert!(String::from_utf8_lossy(&history.stdout).contains("entries=1\n"));

    cleanup_history_path(&history_path);
}
```

- [ ] **Step 2: Run tests to verify RED**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli task_graph_limit -- --nocapture`

Expected: FAIL because `--limit` is currently reported as an unknown task-graph flag and usage lacks task-graph `--limit N`.

- [ ] **Step 3: Implement minimal code**

In `crates/mdid-cli/src/main.rs`:

```rust
struct MoatTaskGraphCommand {
    history_path: String,
    role: Option<AgentRole>,
    state: Option<MoatTaskNodeState>,
    kind: Option<MoatTaskNodeKind>,
    node_id: Option<String>,
    title_contains: Option<String>,
    spec_ref: Option<String>,
    limit: Option<usize>,
}
```

Parse `--limit` in `parse_moat_task_graph_command` with duplicate protection and positive integer validation, mirroring decision-log `parse_limit_value` behavior. Apply it in `run_moat_task_graph` after existing filters:

```rust
let limit = command.limit.unwrap_or(usize::MAX);
for node in task_graph
    .nodes
    .iter()
    .filter(|node| task_graph_node_matches_filters(node, command))
    .take(limit)
{
    // existing output rendering
}
```

Update the `USAGE` string in both production and test code so the task-graph command advertises `[--limit N]`.

- [ ] **Step 4: Run tests to verify GREEN**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli task_graph_limit -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Run broader relevant CLI tests**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli task_graph -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add docs/superpowers/plans/2026-04-26-med-de-id-moat-task-graph-limit.md crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs
git commit -m "feat: limit moat task graph output"
```
