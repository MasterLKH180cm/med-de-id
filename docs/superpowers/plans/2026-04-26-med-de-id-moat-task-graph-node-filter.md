# med-de-id Moat Task Graph Node Filter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a read-only `--node-id` filter to `mdid-cli moat task-graph` so operators and future SDD handoff tooling can inspect one persisted task graph node deterministically.

**Architecture:** Extend the existing latest-round task graph inspection command in `crates/mdid-cli/src/main.rs` with an optional node-id filter that is applied together with the existing role/state filters. Keep the command strictly read-only: it must open existing history only, inspect the latest persisted graph, print persisted fields faithfully with existing pipe-field escaping, and never mutate history or schedule work.

**Tech Stack:** Rust workspace, `mdid-cli`, `mdid-runtime::moat_history::LocalMoatHistoryStore`, standard library CLI tests using `std::process::Command`.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `node_id: Option<String>` to `MoatTaskGraphCommand`.
  - Parse `--node-id VALUE` in `parse_moat_task_graph_command`.
  - Apply exact persisted node ID matching in `run_moat_task_graph` after loading the latest persisted graph.
  - Update the usage string to document `--node-id NODE_ID`.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add TDD coverage for the new filter, missing value handling, no-match behavior, and read-only behavior.
  - Update the shared `USAGE` constant if needed.
- Modify: `README.md`
  - Document the operator-facing `--node-id` filter for task graph inspection.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Truth-sync the shipped foundation status to include the `--node-id` filter.
- Modify: `docs/superpowers/plans/2026-04-26-med-de-id-moat-task-graph-node-filter.md`
  - If implementation details differ from this plan, update this file before final review.

---

### Task 1: Add read-only `--node-id` filtering to `moat task-graph`

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
- Modify: `docs/superpowers/plans/2026-04-26-med-de-id-moat-task-graph-node-filter.md`

- [x] **Step 1: Write failing CLI tests**

Add these tests near the existing `task_graph_*` tests in `crates/mdid-cli/tests/moat_cli.rs`. Reuse existing helpers such as `cli_bin()`, `unique_history_path(...)`, and existing `moat round` setup style from nearby tests.

```rust
#[test]
fn task_graph_filters_latest_graph_by_node_id() {
    let history_path = unique_history_path("task-graph-node-id");
    let history_path_arg = history_path.to_string_lossy().to_string();

    let seed = Command::new(cli_bin())
        .args(["moat", "round", "--history-path", history_path_arg.as_str()])
        .output()
        .expect("failed to seed moat history for node-id task graph filter");
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));

    let output = Command::new(cli_bin())
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg.as_str(),
            "--node-id",
            "strategy_generation",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with node-id filter");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.lines().filter(|line| line.starts_with("node=")).count(), 1);
    assert!(stdout.contains("node=planner|strategy_generation|Strategy Generation|strategy_generation|"));
    assert!(!stdout.contains("node=planner|market_scan|"));
}

#[test]
fn task_graph_node_id_filter_returns_empty_when_no_node_matches() {
    let history_path = unique_history_path("task-graph-node-id-empty");
    let history_path_arg = history_path.to_string_lossy().to_string();

    let seed = Command::new(cli_bin())
        .args(["moat", "round", "--history-path", history_path_arg.as_str()])
        .output()
        .expect("failed to seed moat history for empty node-id task graph filter");
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));

    let output = Command::new(cli_bin())
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg.as_str(),
            "--node-id",
            "not-a-persisted-node",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with unmatched node-id filter");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "moat task graph\n");
}

#[test]
fn task_graph_rejects_missing_node_id_value() {
    let output = Command::new(cli_bin())
        .args(["moat", "task-graph", "--history-path", "/tmp/mdid-unused-history.json", "--node-id"])
        .output()
        .expect("failed to run mdid-cli moat task-graph with missing node-id value");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("missing value for --node-id\n{}\n", USAGE)
    );
}

#[test]
fn task_graph_node_id_filter_does_not_append_history() {
    let history_path = unique_history_path("task-graph-node-id-read-only");
    let history_path_arg = history_path.to_string_lossy().to_string();

    let seed = Command::new(cli_bin())
        .args(["moat", "round", "--history-path", history_path_arg.as_str()])
        .output()
        .expect("failed to seed moat history for node-id read-only check");
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));

    let inspect = Command::new(cli_bin())
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg.as_str(),
            "--node-id",
            "strategy_generation",
        ])
        .output()
        .expect("failed to inspect moat task graph by node id");
    assert!(inspect.status.success(), "{}", String::from_utf8_lossy(&inspect.stderr));

    let history = Command::new(cli_bin())
        .args(["moat", "history", "--history-path", history_path_arg.as_str()])
        .output()
        .expect("failed to inspect moat history after node-id task graph filter");
    assert!(history.status.success(), "{}", String::from_utf8_lossy(&history.stderr));
    assert!(String::from_utf8_lossy(&history.stdout).contains("entries=1\n"));
}
```

- [x] **Step 2: Run targeted RED test and verify it fails for the missing feature**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli task_graph_node_id -- --nocapture
```

Expected: FAIL because `--node-id` is not yet parsed; stderr should mention an unknown argument or the new assertions should fail before implementation.

- [x] **Step 3: Implement minimal CLI parsing and filtering**

In `crates/mdid-cli/src/main.rs`, update the task graph command struct:

```rust
struct MoatTaskGraphCommand {
    history_path: String,
    role: Option<AgentRole>,
    state: Option<MoatTaskNodeState>,
    node_id: Option<String>,
}
```

Update `parse_moat_task_graph_command` to parse the new flag and preserve existing errors:

```rust
"--node-id" => {
    let value = args
        .get(index + 1)
        .ok_or_else(|| "missing value for --node-id".to_string())?;
    node_id = Some(value.clone());
    index += 2;
}
```

Initialize `let mut node_id = None;` before the loop and include `node_id` in `Ok(MoatTaskGraphCommand { ... })`.

In `run_moat_task_graph`, apply exact persisted node ID matching together with existing filters:

```rust
if let Some(expected_node_id) = &command.node_id {
    if node.node_id != *expected_node_id {
        continue;
    }
}
```

Place the check beside the existing role/state checks before printing the `node=` line. Do not normalize underscores/hyphens, do not escape before comparing, and do not error when no node matches.

Update the usage text to include:

```text
moat task-graph --history-path PATH [--role planner|coder|reviewer] [--state pending|ready|in_progress|completed|blocked] [--node-id NODE_ID]
```

- [x] **Step 4: Run targeted GREEN test**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli task_graph_node_id -- --nocapture
```

Expected: PASS.

- [x] **Step 5: Update README/spec/plan docs**

Update `README.md` and `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md` so the task graph inspection surface is described as:

```text
mdid-cli moat task-graph --history-path PATH [--role planner|coder|reviewer] [--state pending|ready|in_progress|completed|blocked] [--node-id NODE_ID]
```

Document that `--node-id` performs exact matching against persisted task graph node IDs, returns no `node=` rows when no persisted node matches, and remains read-only.

If implementation details or expected output changed while coding, update this plan file before review so reviewers use the honest contract.

- [x] **Step 6: Run broader verification**

Run:

```bash
source "$HOME/.cargo/env"
cargo fmt --check
cargo test -p mdid-cli --test moat_cli task_graph -- --nocapture
cargo test -p mdid-cli --test moat_cli moat -- --nocapture
cargo test -p mdid-cli
```

Expected: all commands PASS. If any command fails with `No space left on device`, follow the disk-pressure cleanup rule from the moat-loop skill, then rerun the same command.

- [ ] **Step 7: Commit**

After spec compliance and code quality review are both approved, commit:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs README.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-26-med-de-id-moat-task-graph-node-filter.md
git commit -m "feat: filter moat task graph by node id"
```

---

## Self-Review

- Spec coverage: The plan implements the next conservative Autonomous Multi-Agent System support slice by making persisted task graph inspection more directly addressable for future SDD handoff tooling. It preserves read-only behavior and does not create a daemon, crawler, PR automation, or cron job.
- Placeholder scan: No `TBD`, `TODO`, `implement later`, or unspecified test requests remain.
- Type consistency: `MoatTaskGraphCommand.node_id: Option<String>` is used consistently by parser and runner; CLI flag spelling is consistently `--node-id`; output remains existing `node=` rows.
