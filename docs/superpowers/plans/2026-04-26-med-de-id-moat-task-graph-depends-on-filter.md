# Med De ID Moat Task Graph Depends-On Filter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `mdid-cli moat task-graph --depends-on <node-id>` filter so autonomous moat-loop operators can inspect graph nodes gated by a specific prerequisite.

**Architecture:** This is a narrow CLI/query slice over the existing local moat history store. The command parser stores an optional dependency node-id filter, and `run_moat_task_graph` applies it to the latest round's task graph before printing existing escaped node rows.

**Tech Stack:** Rust workspace, `mdid-cli`, `mdid-runtime` local history store, Cargo integration tests.

---

## File Structure

- Modify `crates/mdid-cli/src/main.rs`
  - Add `depends_on: Option<String>` to `MoatTaskGraphCommand`.
  - Parse a repeat-safe `--depends-on <node-id>` flag in `parse_moat_task_graph_command`.
  - Apply the filter in `run_moat_task_graph` by retaining nodes whose `depends_on` vector contains the requested dependency exactly.
- Modify `crates/mdid-cli/tests/moat_cli.rs`
  - Add one integration test proving `--depends-on implementation` returns only nodes depending on `implementation`.
  - Add one parser/error integration test proving duplicate `--depends-on` is rejected.

### Task 1: CLI task graph dependency filter

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/moat_cli.rs`

- [x] **Step 1: Write the failing dependency filter integration test**

Add this test near existing `moat task-graph` filter tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn moat_task_graph_filters_nodes_by_dependency() {
    let temp_dir = TempDir::new().expect("temp dir");
    let history_path = temp_dir.path().join("moat-history.json");

    let round_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--history-path",
            history_path.to_str().expect("history path"),
            "--strategy-candidates",
            "2",
            "--spec-generations",
            "1",
            "--implementation-tasks",
            "1",
            "--review-loops",
            "0",
            "--tests-passed",
            "false",
        ])
        .output()
        .expect("run moat round");
    assert!(round_output.status.success());

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path.to_str().expect("history path"),
            "--depends-on",
            "implementation",
        ])
        .output()
        .expect("run moat task graph");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("task_node=review|Review|reviewer|review|ready|implementation|<none>\n"));
    assert!(!stdout.contains("task_node=implementation|Implementation|coder|implementation"));
    assert!(!stdout.contains("task_node=evaluation|Evaluation|reviewer|evaluation"));
}
```

- [x] **Step 2: Run the new test and verify RED**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli moat_task_graph_filters_nodes_by_dependency --test moat_cli -- --nocapture
```

Expected: FAIL because `--depends-on` is an unknown flag.

- [x] **Step 3: Add duplicate flag coverage**

Add this test near parser error tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn moat_task_graph_rejects_duplicate_depends_on_filter() {
    let temp_dir = TempDir::new().expect("temp dir");
    let history_path = temp_dir.path().join("moat-history.json");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path.to_str().expect("history path"),
            "--depends-on",
            "implementation",
            "--depends-on",
            "review",
        ])
        .output()
        .expect("run moat task graph");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("duplicate flag: --depends-on"));
}
```

- [x] **Step 4: Run the duplicate flag test and verify RED**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli moat_task_graph_rejects_duplicate_depends_on_filter --test moat_cli -- --nocapture
```

Expected: FAIL because `--depends-on` is currently treated as an unknown flag rather than a duplicate-aware flag.

- [x] **Step 5: Implement the minimal parser and filter code**

Change `crates/mdid-cli/src/main.rs` so `MoatTaskGraphCommand` includes the new field:

```rust
struct MoatTaskGraphCommand {
    history_path: String,
    role: Option<AgentRole>,
    state: Option<MoatTaskNodeState>,
    kind: Option<MoatTaskNodeKind>,
    node_id: Option<String>,
    title_contains: Option<String>,
    spec_ref: Option<String>,
    depends_on: Option<String>,
    contains: Option<String>,
    limit: Option<usize>,
}
```

In `parse_moat_task_graph_command`, initialize and parse the flag:

```rust
let mut depends_on = None;
```

```rust
"--depends-on" => {
    let value = required_flag_value(args, index, "--depends-on", false)?;
    if depends_on.is_some() {
        return Err(duplicate_flag_error("--depends-on"));
    }
    depends_on = Some(value.clone());
}
```

Include it in the returned command:

```rust
depends_on,
```

In `run_moat_task_graph`, add this filter before the existing `contains` filter:

```rust
.filter(|node| {
    command
        .depends_on
        .as_deref()
        .map(|dependency| node.depends_on.iter().any(|candidate| candidate == dependency))
        .unwrap_or(true)
})
```

- [x] **Step 6: Run targeted tests and verify GREEN**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli moat_task_graph_filters_nodes_by_dependency --test moat_cli -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli moat_task_graph_rejects_duplicate_depends_on_filter --test moat_cli -- --nocapture
```

Expected: both tests PASS.

- [x] **Step 7: Run the relevant broader CLI filter test subset**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli moat_task_graph --test moat_cli -- --nocapture
```

Expected: all `moat_task_graph...` tests PASS.

- [x] **Step 8: Commit**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs docs/superpowers/plans/2026-04-26-med-de-id-moat-task-graph-depends-on-filter.md
git commit -m "feat: filter moat task graph by dependency"
```

Expected: commit succeeds on `feature/moat-loop-autonomy`.

---

## Self-Review

- Spec coverage: The plan covers parser state, duplicate handling, exact dependency matching, output preservation, and targeted verification.
- Placeholder scan: No TBD/TODO/implement-later placeholders remain.
- Type consistency: The new field is consistently named `depends_on`, and the CLI flag is consistently `--depends-on`.
