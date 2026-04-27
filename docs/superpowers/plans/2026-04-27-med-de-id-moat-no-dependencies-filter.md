# Moat No Dependencies Filter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `--no-dependencies` filters to `mdid-cli moat task-graph` and `mdid-cli moat assignments` so operators can inspect root tasks with no upstream dependencies.

**Architecture:** Extend the existing CLI command structs with a boolean filter parsed as a presence-only flag. Reuse the latest persisted moat history and task graph dependency data already used by `--depends-on`; task-graph filters nodes directly and assignments filters by the assigned node's task graph metadata.

**Tech Stack:** Rust, Cargo workspace, `mdid-cli` integration tests, local JSON moat history store.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `no_dependencies: bool` to `MoatAssignmentsCommand` and `MoatTaskGraphCommand`.
  - Update `USAGE` to advertise `[--no-dependencies]` for both subcommands.
  - Parse `--no-dependencies` as a duplicate-rejected presence-only flag in both parsers.
  - Filter task graph nodes where `node.depends_on.is_empty()` when enabled.
  - Filter assignments by looking up the assignment node and requiring `node.depends_on.is_empty()` when enabled.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Update the test-local `USAGE` string.
  - Add one task-graph integration test for `--no-dependencies`.
  - Add one assignments integration test for `--no-dependencies`.
  - Add duplicate flag rejection tests for both subcommands.

### Task 1: Add task graph `--no-dependencies` filter

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/moat_cli.rs`

- [ ] **Step 1: Write the failing task graph filter test**

Add this test immediately after `moat_task_graph_filters_nodes_by_dependency` in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn moat_task_graph_filters_nodes_with_no_dependencies() {
    let history_path = unique_history_path("task-graph-no-dependencies");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed task graph history for no-dependencies filter");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--no-dependencies",
        ])
        .output()
        .expect("failed to inspect task graph by no-dependencies filter");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("node=planner|market_scan|Market Scan|market_scan|completed|<none>|<none>\n"));
    assert!(!stdout.contains("node=planner|competitor_analysis|Competitor Analysis|competitor_analysis"));
    assert!(!stdout.contains("node=coder|implementation|Implementation|implementation"));

    cleanup_history_path(&history_path);
}
```

- [ ] **Step 2: Run the focused test to verify RED**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_task_graph_filters_nodes_with_no_dependencies -- --exact --nocapture
```

Expected: FAIL because `--no-dependencies` is currently reported as an unknown moat task-graph flag.

- [ ] **Step 3: Implement minimal task graph support**

In `crates/mdid-cli/src/main.rs`, add `no_dependencies: bool` to `MoatTaskGraphCommand`, initialize it to `false` in `parse_moat_task_graph_command`, parse `--no-dependencies` as a duplicate-rejected presence flag, include it in the returned struct, and add this filter after the existing `--depends-on` filter:

```rust
        .filter(|node| {
            if command.no_dependencies {
                node.depends_on.is_empty()
            } else {
                true
            }
        })
```

Also update both `USAGE` strings to show:

```text
moat task-graph --history-path PATH [--role planner|coder|reviewer] [--state pending|ready|in_progress|completed|blocked] [--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation] [--node-id NODE_ID] [--depends-on NODE_ID] [--no-dependencies] [--title-contains TEXT] [--spec-ref SPEC_REF] [--contains TEXT] [--limit N]
```

- [ ] **Step 4: Run the focused test to verify GREEN**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_task_graph_filters_nodes_with_no_dependencies -- --exact --nocapture
```

Expected: PASS.

- [ ] **Step 5: Add duplicate flag rejection test**

Add this test immediately after `moat_task_graph_rejects_duplicate_depends_on_filter`:

```rust
#[test]
fn moat_task_graph_rejects_duplicate_no_dependencies_filter() {
    let history_path = unique_history_path("task-graph-no-dependencies-duplicate");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--no-dependencies",
            "--no-dependencies",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with duplicate no-dependencies filter");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("duplicate flag: --no-dependencies"));
    assert!(!history_path.exists());
}
```

- [ ] **Step 6: Run the duplicate test**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_task_graph_rejects_duplicate_no_dependencies_filter -- --exact --nocapture
```

Expected: PASS.

- [ ] **Step 7: Commit task graph slice**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs docs/superpowers/plans/2026-04-27-med-de-id-moat-no-dependencies-filter.md
git commit -m "feat: filter moat task graph roots"
```

### Task 2: Add assignments `--no-dependencies` filter

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/moat_cli.rs`

- [x] **Step 1: Write the failing assignments filter test**

Add this test near the other moat assignment filter tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn moat_assignments_filters_entries_with_no_dependencies() {
    let history_path = unique_history_path("assignments-no-dependencies");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed assignments history for no-dependencies filter");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--no-dependencies",
        ])
        .output()
        .expect("failed to inspect assignments by no-dependencies filter");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("assignment=planner|market_scan|Market Scan|market_scan|<none>\n"));
    assert!(!stdout.contains("assignment=planner|competitor_analysis|Competitor Analysis|competitor_analysis"));
    assert!(!stdout.contains("assignment=coder|implementation|Implementation|implementation"));

    cleanup_history_path(&history_path);
}
```

- [x] **Step 2: Run the focused test to verify RED**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_assignments_filters_entries_with_no_dependencies -- --exact --nocapture
```

Expected: FAIL because `--no-dependencies` is currently reported as an unknown moat assignments flag.

- [x] **Step 3: Implement minimal assignments support**

In `crates/mdid-cli/src/main.rs`, add `no_dependencies: bool` to `MoatAssignmentsCommand`, initialize it to `false` in `parse_moat_assignments_command`, parse `--no-dependencies` as a duplicate-rejected presence flag, include it in the returned struct, and add this filter after the existing `--depends-on` assignments filter:

```rust
        .filter(|assignment| {
            if command.no_dependencies {
                latest
                    .report
                    .control_plane
                    .task_graph
                    .nodes
                    .iter()
                    .find(|node| node.node_id == assignment.node_id)
                    .map(|node| node.depends_on.is_empty())
                    .unwrap_or(false)
            } else {
                true
            }
        })
```

Also update both `USAGE` strings to show:

```text
moat assignments --history-path PATH [--role planner|coder|reviewer] [--state pending|ready|in_progress|completed|blocked] [--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation] [--node-id NODE_ID] [--depends-on NODE_ID] [--no-dependencies] [--title-contains TEXT] [--spec-ref SPEC_REF] [--contains TEXT] [--limit N]
```

- [x] **Step 4: Run the focused test to verify GREEN**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_assignments_filters_entries_with_no_dependencies -- --exact --nocapture
```

Expected: PASS.

- [x] **Step 5: Add duplicate flag rejection test**

Add this test near the assignments duplicate flag tests:

```rust
#[test]
fn moat_assignments_rejects_duplicate_no_dependencies_filter() {
    let history_path = unique_history_path("assignments-no-dependencies-duplicate");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--no-dependencies",
            "--no-dependencies",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with duplicate no-dependencies filter");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("duplicate flag: --no-dependencies"));
    assert!(!history_path.exists());
}
```

- [x] **Step 6: Run the duplicate test**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_assignments_rejects_duplicate_no_dependencies_filter -- --exact --nocapture
```

Expected: PASS.

- [x] **Step 7: Run relevant broader validation**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli no_dependencies -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_task_graph_filters_nodes_with_no_dependencies moat_assignments_filters_entries_with_no_dependencies -- --nocapture
```

Expected: PASS for all no-dependencies tests. If the second command does not accept two filters on this Cargo version, run each named test separately.

- [x] **Step 8: Commit assignments slice**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs docs/superpowers/plans/2026-04-27-med-de-id-moat-no-dependencies-filter.md
git commit -m "feat: filter moat assignments roots"
```

## Self-Review

- Spec coverage: The plan adds root/no-dependency inspection for both `moat task-graph` and `moat assignments`, including parser support, usage text, behavior tests, and duplicate flag rejection.
- Placeholder scan: No TBD/TODO/fill-in placeholders remain.
- Type consistency: Both command structs use the same field name, `no_dependencies: bool`, and both parsers reject duplicate `--no-dependencies` flags with the shared `duplicate_flag_error` helper.
