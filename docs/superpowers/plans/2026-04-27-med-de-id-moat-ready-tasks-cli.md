# Med De Id Moat Ready Tasks CLI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a CLI surface that prints the immediately schedulable moat task graph nodes for a selected history round.

**Architecture:** Reuse the existing persisted moat history store and `MoatTaskGraph::ready_node_ids()` domain behavior instead of inventing a scheduler. The CLI command reads the latest round by default, optionally scopes by exact `--round-id`, and prints only nodes whose dependencies are complete so the autonomous controller can choose the next safe agent assignment.

**Tech Stack:** Rust workspace, `mdid-cli`, `mdid-runtime::moat_history::LocalMoatHistoryStore`, `mdid-domain::MoatTaskGraph`, Cargo integration tests.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `MoatReadyTasksCommand` with `history_path`, optional `round_id`, optional `role`, optional `kind`, and optional positive `limit`.
  - Add `CliCommand::MoatReadyTasks` parsing for `mdid-cli moat ready-tasks`.
  - Add `run_moat_ready_tasks()` that loads history, selects the requested round or latest round, intersects `ready_node_ids()` with requested filters, and prints deterministic rows.
  - Update the usage string to document the new command.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add integration tests proving ready tasks exclude dependency-blocked nodes and support role/kind/limit filters.

### Task 1: Add `moat ready-tasks` CLI command

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`

- [x] **Step 1: Write the failing integration test for latest ready tasks**

Append this test near the existing moat task graph CLI tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn cli_reports_ready_moat_tasks_for_latest_round() {
    let history_path = unique_history_path("ready-tasks-latest");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--review-loops", "0", "--history-path", history_path_arg])
        .output()
        .expect("failed to run mdid-cli moat round with history path");
    assert!(
        output.status.success(),
        "expected setup round success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "ready-tasks", "--history-path", history_path_arg])
        .output()
        .expect("failed to run mdid-cli moat ready-tasks");

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
            "ready_task=reviewer|review|review-loop|Run spec compliance and quality review|<none>\n",
        )
    );

    cleanup_history_path(&history_path);
}
```

- [x] **Step 2: Run the new test to verify it fails**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_reports_ready_moat_tasks_for_latest_round -- --exact --nocapture`

Expected: FAIL because `moat ready-tasks` is an unknown command and exits with usage.

- [x] **Step 3: Implement minimal command parsing and output**

In `crates/mdid-cli/src/main.rs`:

1. Add this struct after `MoatTaskGraphCommand`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
struct MoatReadyTasksCommand {
    history_path: String,
    round_id: Option<String>,
    role: Option<AgentRole>,
    kind: Option<MoatTaskNodeKind>,
    limit: Option<usize>,
}
```

2. Add `MoatReadyTasks(MoatReadyTasksCommand),` to `CliCommand` after `MoatTaskGraph(MoatTaskGraphCommand),`.

3. Add this branch to `main()` after the `MoatTaskGraph` branch:

```rust
        Ok(CliCommand::MoatReadyTasks(command)) => {
            if let Err(error) = run_moat_ready_tasks(&command) {
                exit_with_error(error);
            }
        }
```

4. Add this parse branch to `parse_command()` after the `task-graph` branch:

```rust
        [moat, ready_tasks, rest @ ..] if moat == "moat" && ready_tasks == "ready-tasks" => Ok(
            CliCommand::MoatReadyTasks(parse_moat_ready_tasks_command(rest)?),
        ),
```

5. Add this parser near the task graph parser helpers:

```rust
fn parse_moat_ready_tasks_command(args: &[String]) -> Result<MoatReadyTasksCommand, String> {
    let mut history_path = None;
    let mut round_id = None;
    let mut role = None;
    let mut kind = None;
    let mut limit = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--history-path" => {
                let value = required_history_path_value(args, index)?.clone();
                if history_path.is_some() {
                    return Err(duplicate_flag_error("--history-path"));
                }
                history_path = Some(value);
            }
            "--round-id" => {
                let value = required_flag_value(args, index, "--round-id", true)?;
                if round_id.is_some() {
                    return Err(duplicate_flag_error("--round-id"));
                }
                round_id = Some(value.to_string());
            }
            "--role" => {
                let value = required_flag_value(args, index, "--role", false)?;
                if role.is_some() {
                    return Err(duplicate_flag_error("--role"));
                }
                role = Some(parse_moat_task_graph_role_filter(value)?);
            }
            "--kind" => {
                let value = required_flag_value(args, index, "--kind", true)?;
                if kind.is_some() {
                    return Err(duplicate_flag_error("--kind"));
                }
                kind = Some(parse_moat_task_graph_kind_filter(value)?);
            }
            "--limit" => {
                let value = required_flag_value(args, index, "--limit", true)?;
                if limit.is_some() {
                    return Err(duplicate_flag_error("--limit"));
                }
                limit = Some(parse_task_graph_limit_value(value)?);
            }
            flag => return Err(format!("unknown flag: {flag}")),
        }

        index += 2;
    }

    Ok(MoatReadyTasksCommand {
        history_path: history_path
            .ok_or_else(|| "missing required flag: --history-path".to_string())?,
        round_id,
        role,
        kind,
        limit,
    })
}
```

6. Add this runner near `run_moat_task_graph()`:

```rust
fn run_moat_ready_tasks(command: &MoatReadyTasksCommand) -> Result<(), String> {
    let store = LocalMoatHistoryStore::open_existing(&command.history_path)
        .map_err(|error| format!("failed to open moat history store: {error}"))?;
    if store.entries().is_empty() {
        return Err(
            "moat history is empty; run `mdid-cli moat round --history-path <path>` first"
                .to_string(),
        );
    }

    let selected = if let Some(round_id) = command.round_id.as_deref() {
        store
            .entries()
            .iter()
            .find(|entry| entry.report.summary.round_id.to_string() == round_id)
    } else {
        Some(store.entries().last().ok_or_else(|| {
            "moat history is empty; run `mdid-cli moat round --history-path <path>` first"
                .to_string()
        })?)
    };

    let Some(entry) = selected else {
        println!("moat ready tasks");
        println!("ready_task_entries=0");
        return Ok(());
    };

    let ready_ids = entry.report.control_plane.task_graph.ready_node_ids();
    let mut ready_nodes = entry
        .report
        .control_plane
        .task_graph
        .nodes
        .iter()
        .filter(|node| ready_ids.iter().any(|ready_id| ready_id == &node.node_id))
        .filter(|node| command.role.map(|role| node.role == role).unwrap_or(true))
        .filter(|node| command.kind.map(|kind| node.kind == kind).unwrap_or(true))
        .collect::<Vec<_>>();

    if let Some(limit) = command.limit {
        ready_nodes.truncate(limit);
    }

    println!("moat ready tasks");
    println!("ready_task_entries={}", ready_nodes.len());
    for node in ready_nodes {
        println!(
            "ready_task={}|{}|{}|{}|{}",
            format_agent_role(node.role),
            format_moat_task_kind(node.kind),
            escape_assignment_output_field(&node.node_id),
            escape_assignment_output_field(&node.title),
            node.spec_ref
                .as_deref()
                .map(escape_assignment_output_field)
                .unwrap_or_else(|| "<none>".to_string())
        );
    }

    Ok(())
}
```

7. Update `USAGE` in `crates/mdid-cli/tests/moat_cli.rs` and `usage()` in `crates/mdid-cli/src/main.rs` to include:

```text
 | moat ready-tasks --history-path PATH [--round-id ROUND_ID] [--role planner|coder|reviewer] [--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation] [--limit N]
```

- [x] **Step 4: Run the latest ready tasks test to verify it passes**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_reports_ready_moat_tasks_for_latest_round -- --exact --nocapture`

Expected: PASS.

- [x] **Step 5: Write failing filter tests**

Append these tests near the first ready tasks test:

```rust
#[test]
fn cli_filters_ready_moat_tasks_by_role_kind_and_limit() {
    let history_path = unique_history_path("ready-tasks-filters");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--review-loops", "0", "--history-path", history_path_arg])
        .output()
        .expect("failed to run mdid-cli moat round with history path");
    assert!(
        output.status.success(),
        "expected setup round success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "ready-tasks",
            "--history-path",
            history_path_arg,
            "--role",
            "reviewer",
            "--kind",
            "review",
            "--limit",
            "1",
        ])
        .output()
        .expect("failed to run mdid-cli moat ready-tasks with filters");

    assert!(
        output.status.success(),
        "expected filtered ready tasks success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "moat ready tasks\n",
            "ready_task_entries=1\n",
            "ready_task=reviewer|review|review-loop|Run spec compliance and quality review|<none>\n",
        )
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "ready-tasks",
            "--history-path",
            history_path_arg,
            "--role",
            "coder",
        ])
        .output()
        .expect("failed to run mdid-cli moat ready-tasks with non-matching role filter");

    assert!(
        output.status.success(),
        "expected non-matching filtered ready tasks success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!("moat ready tasks\n", "ready_task_entries=0\n")
    );

    cleanup_history_path(&history_path);
}

#[test]
fn cli_reports_zero_ready_moat_tasks_for_unknown_round_id() {
    let history_path = unique_history_path("ready-tasks-unknown-round");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to run mdid-cli moat round with history path");
    assert!(
        output.status.success(),
        "expected setup round success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "ready-tasks",
            "--history-path",
            history_path_arg,
            "--round-id",
            "00000000-0000-0000-0000-000000000000",
        ])
        .output()
        .expect("failed to run mdid-cli moat ready-tasks with unknown round id");

    assert!(
        output.status.success(),
        "expected unknown round ready tasks success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!("moat ready tasks\n", "ready_task_entries=0\n")
    );

    cleanup_history_path(&history_path);
}
```

- [x] **Step 6: Run filter tests to verify they fail before any missing implementation is added**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_filters_ready_moat_tasks_by_role_kind_and_limit cli_reports_zero_ready_moat_tasks_for_unknown_round_id -- --nocapture`

Expected: If Step 3 did not yet include all filters, FAIL on missing/incorrect filter handling. If Step 3 already included the filters, these tests may PASS; record that Step 3's minimal implementation already covered the behavior.

- [x] **Step 7: Implement any missing filter behavior**

If Step 6 failed, update `parse_moat_ready_tasks_command()` and `run_moat_ready_tasks()` to match the exact code shown in Step 3. Do not add additional flags or output columns.

- [x] **Step 8: Run targeted tests**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli ready_tasks -- --nocapture`

Expected: PASS for all tests whose names contain `ready_tasks`.

- [x] **Step 9: Run package-level CLI tests**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli -- --nocapture`

Expected: PASS.

- [x] **Step 10: Run package-level tests for compile-time struct literal coverage**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli`

Expected: PASS.

- [x] **Step 11: Commit**

```bash
git add docs/superpowers/plans/2026-04-27-med-de-id-moat-ready-tasks-cli.md crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs
git commit -m "feat: report ready moat tasks from CLI"
```

## Self-Review

- Spec coverage: The plan adds a CLI command for ready autonomous task selection, includes latest-round default, exact round selection, role/kind/limit filters, deterministic output, usage documentation, and targeted/package verification.
- Placeholder scan: No TBD/TODO/fill-in placeholders remain.
- Type consistency: `MoatReadyTasksCommand`, `parse_moat_ready_tasks_command`, `run_moat_ready_tasks`, and `CliCommand::MoatReadyTasks` names are consistent throughout.
