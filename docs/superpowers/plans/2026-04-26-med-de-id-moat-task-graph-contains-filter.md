# Med De Id Moat Task Graph Contains Filter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a read-only `--contains TEXT` filter to `mdid-cli moat task-graph` so operators can search latest persisted task graph nodes across node id, title, state/kind labels, dependencies, and spec refs.

**Architecture:** Extend the existing CLI parser and task graph renderer only. The filter is an in-memory conjunction with existing role/state/kind/node/title/spec/limit filters and must not mutate moat history.

**Tech Stack:** Rust workspace, `mdid-cli`, `mdid-runtime::moat_history`, Cargo integration tests.

---

## File Structure

- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add integration coverage for task graph raw-content filtering and parser errors.
- Modify: `crates/mdid-cli/src/main.rs`
  - Add `contains: Option<String>` to `MoatTaskGraphCommand`.
  - Parse `--contains TEXT` with duplicate and missing-value validation.
  - Apply raw-content filtering in `run_moat_task_graph` without appending history.
  - Update usage string.
- Modify: `docs/superpowers/plans/2026-04-26-med-de-id-moat-task-graph-contains-filter.md`
  - Track completion checkboxes.

### Task 1: Add `moat task-graph --contains TEXT`

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `docs/superpowers/plans/2026-04-26-med-de-id-moat-task-graph-contains-filter.md`

- [x] **Step 1: Write failing tests for task graph contains filtering**

Add tests to `crates/mdid-cli/tests/moat_cli.rs` near existing task graph filter tests:

```rust
#[test]
fn task_graph_filters_latest_nodes_by_contains_text() {
    let history_path = unique_history_path("moat-task-graph-contains");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    persist_sample_round(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "task-graph", "--history-path", history_path_arg, "--contains", "workflow-audit"])
        .output()
        .expect("failed to run mdid-cli moat task-graph with contains filter");

    assert!(output.status.success(), "expected success, stderr was: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("moat task graph\n"));
    assert!(stdout.contains("node=planner|spec_planning|Spec Planning|spec_planning|completed|strategy_generation|moat-spec/workflow-audit\n"));
    assert!(!stdout.contains("node=planner|market_scan|"));

    cleanup_history_path(&history_path);
}

#[test]
fn task_graph_contains_filter_returns_zero_matches_without_error() {
    let history_path = unique_history_path("moat-task-graph-contains-zero");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    persist_sample_round(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "task-graph", "--history-path", history_path_arg, "--contains", "not-present"])
        .output()
        .expect("failed to run mdid-cli moat task-graph with contains filter");

    assert!(output.status.success(), "expected success, stderr was: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "moat task graph\n");

    cleanup_history_path(&history_path);
}

#[test]
fn task_graph_contains_filter_combines_with_role_filter() {
    let history_path = unique_history_path("moat-task-graph-role-contains");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    persist_sample_round(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "task-graph", "--history-path", history_path_arg, "--role", "coder", "--contains", "Implementation"])
        .output()
        .expect("failed to run mdid-cli moat task-graph with role and contains filters");

    assert!(output.status.success(), "expected success, stderr was: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "moat task graph\nnode=coder|implementation|Implementation|implementation|completed|spec_planning|moat-spec/workflow-audit\n"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn task_graph_contains_filter_rejects_missing_flag_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "task-graph", "--history-path", "ignored", "--contains"])
        .output()
        .expect("failed to run mdid-cli moat task-graph with missing contains value");

    assert!(!output.status.success(), "expected failure");
    assert_eq!(String::from_utf8_lossy(&output.stderr), format!("error: missing value for --contains\n{USAGE}\n"));
}

#[test]
fn task_graph_contains_filter_rejects_flag_like_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "task-graph", "--history-path", "ignored", "--contains", "--role"])
        .output()
        .expect("failed to run mdid-cli moat task-graph with flag-like contains value");

    assert!(!output.status.success(), "expected failure");
    assert_eq!(String::from_utf8_lossy(&output.stderr), format!("error: missing value for --contains\n{USAGE}\n"));
}

#[test]
fn task_graph_contains_filter_rejects_duplicate_flag() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "task-graph", "--history-path", "ignored", "--contains", "one", "--contains", "two"])
        .output()
        .expect("failed to run mdid-cli moat task-graph with duplicate contains filter");

    assert!(!output.status.success(), "expected failure");
    assert_eq!(String::from_utf8_lossy(&output.stderr), format!("error: duplicate flag: --contains\n{USAGE}\n"));
}
```

- [x] **Step 2: Run targeted RED test**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli task_graph_contains -- --nocapture`

Expected: FAIL because `--contains` is an unknown flag and/or usage does not include the task graph contains filter.

- [x] **Step 3: Implement minimal CLI support**

In `crates/mdid-cli/src/main.rs`:

- Add `[--contains TEXT]` to the `moat task-graph` usage segment.
- Add `contains: Option<String>` to `MoatTaskGraphCommand`.
- Initialize `let mut contains = None;` in `parse_moat_task_graph_command`.
- Parse duplicate-safe `--contains` with `required_flag_value(args, index, "--contains", true)?`.
- Include `contains` in the constructed command.
- In `run_moat_task_graph`, add a filter that matches when the needle is contained in the node id, title, formatted kind, formatted state, any dependency id, or spec ref.

- [x] **Step 4: Run targeted GREEN tests**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli task_graph_contains -- --nocapture`

Expected: PASS.

- [x] **Step 5: Run broader CLI tests**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli -- --nocapture`

Expected: PASS.

- [x] **Step 6: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs docs/superpowers/plans/2026-04-26-med-de-id-moat-task-graph-contains-filter.md
git commit -m "feat: filter moat task graph by raw text"
```
