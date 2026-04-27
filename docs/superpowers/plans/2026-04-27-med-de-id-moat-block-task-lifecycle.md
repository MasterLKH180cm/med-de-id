# Med De ID Moat Block Task Lifecycle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `mdid-cli moat block-task` lifecycle command so autonomous workers can mark an in-progress moat task as blocked instead of only completing it.

**Architecture:** Reuse the existing persisted moat history control-plane pattern in `crates/mdid-cli/src/main.rs`: parse a CLI command, open `LocalMoatHistoryStore`, select the round, transition exactly one task node, persist history, and print a deterministic summary. Keep the slice narrow: only in-progress nodes may be blocked, dependencies are not advanced, and no new scheduler behavior is introduced.

**Tech Stack:** Rust workspace, Cargo integration tests in `crates/mdid-cli/tests/moat_cli.rs`, local JSONL moat history store via `mdid_runtime::moat_history`.

---

## File Structure

- Modify `crates/mdid-cli/src/main.rs`
  - Add `MoatBlockTaskCommand` with `history_path`, optional `round_id`, and `node_id`.
  - Add `CliCommand::MoatBlockTask` dispatch.
  - Add parser support for `mdid-cli moat block-task --history-path <path> --node-id <id> [--round-id <round>]`.
  - Add `run_moat_block_task` mirroring `run_moat_complete_task` but transitioning `InProgress -> Blocked` and never reporting next-ready tasks.
- Modify `crates/mdid-cli/tests/moat_cli.rs`
  - Add integration tests proving a claimed task can be blocked and persisted.
  - Add integration tests proving a ready/unclaimed task is rejected.

### Task 1: CLI block-task lifecycle

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/moat_cli.rs`

- [ ] **Step 1: Write the failing claimed-task integration test**

Add this test near the existing `complete-task` tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn cli_blocks_claimed_moat_task() {
    let history_path = unique_history_path("block-task-claimed");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--review-loops",
            "0",
            "--history-path",
            history_path_arg,
        ])
        .output()
        .expect("failed to seed block-task history");
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));

    let claim = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "claim-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
        ])
        .output()
        .expect("failed to claim task before blocking");
    assert!(claim.status.success(), "{}", String::from_utf8_lossy(&claim.stderr));

    let round_id = LocalMoatHistoryStore::open_existing(&history_path)
        .expect("history should reload")
        .entries()[0]
        .report
        .summary
        .round_id
        .to_string();

    let block_review = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "block-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
        ])
        .output()
        .expect("failed to block review task");
    assert!(block_review.status.success(), "{}", String::from_utf8_lossy(&block_review.stderr));

    assert_eq!(
        String::from_utf8_lossy(&block_review.stdout),
        format!(
            "moat task blocked\nround_id={round_id}\nnode_id=review\nprevious_state=in_progress\nnew_state=blocked\nhistory_path={history_path_arg}\n"
        )
    );

    let graph = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
        ])
        .output()
        .expect("failed to inspect task graph after blocking");
    assert!(graph.status.success(), "{}", String::from_utf8_lossy(&graph.stderr));
    assert!(String::from_utf8_lossy(&graph.stdout)
        .contains("node=reviewer|review|Review|review|blocked|implementation|<none>\n"));

    cleanup_history_path(&history_path);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_blocks_claimed_moat_task -- --nocapture`

Expected: FAIL because `block-task` is not recognized and stderr contains CLI usage/unknown command behavior.

- [ ] **Step 3: Write minimal implementation**

In `crates/mdid-cli/src/main.rs`:

1. Add this struct after `MoatCompleteTaskCommand`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
struct MoatBlockTaskCommand {
    history_path: String,
    round_id: Option<String>,
    node_id: String,
}
```

2. Add `MoatBlockTask(MoatBlockTaskCommand),` to `CliCommand` immediately after `MoatCompleteTask`.

3. Add dispatch in `main` immediately after complete-task dispatch:

```rust
        Ok(CliCommand::MoatBlockTask(command)) => {
            if let Err(error) = run_moat_block_task(&command) {
                exit_with_error(error);
            }
        }
```

4. Add parser branch after `complete-task`:

```rust
        [moat, block_task, rest @ ..] if moat == "moat" && block_task == "block-task" => {
            Ok(CliCommand::MoatBlockTask(parse_moat_block_task_command(rest)?))
        }
```

5. Add parser helper after `parse_moat_complete_task_command`:

```rust
fn parse_moat_block_task_command(args: &[String]) -> Result<MoatBlockTaskCommand, String> {
    let command = parse_moat_claim_task_command(args)?;
    Ok(MoatBlockTaskCommand {
        history_path: command.history_path,
        round_id: command.round_id,
        node_id: command.node_id,
    })
}
```

6. Add runner near `run_moat_complete_task`:

```rust
fn run_moat_block_task(command: &MoatBlockTaskCommand) -> Result<(), String> {
    let mut store = LocalMoatHistoryStore::open_existing(&command.history_path)?;
    let entry = select_moat_history_entry_mut(store.entries_mut(), command.round_id.as_deref())?;
    let round_id = entry.report.summary.round_id.to_string();
    let previous_state = transition_moat_task_node(
        &mut entry.report.task_graph,
        &command.node_id,
        MoatTaskNodeState::InProgress,
        MoatTaskNodeState::Blocked,
    )?;
    store.persist()?;

    println!("moat task blocked");
    println!("round_id={round_id}");
    println!("node_id={}", command.node_id);
    println!("previous_state={}", format_moat_task_state(previous_state));
    println!("new_state={}", format_moat_task_state(MoatTaskNodeState::Blocked));
    println!("history_path={}", command.history_path);

    Ok(())
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_blocks_claimed_moat_task -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Write failing rejection test**

Add this test near `cli_complete_task_rejects_unclaimed_ready_task`:

```rust
#[test]
fn cli_block_task_rejects_unclaimed_ready_task() {
    let history_path = unique_history_path("block-task-unclaimed");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--review-loops",
            "0",
            "--history-path",
            history_path_arg,
        ])
        .output()
        .expect("failed to seed block-task history");
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "block-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
        ])
        .output()
        .expect("failed to run mdid-cli moat block-task for ready node");

    assert!(!output.status.success(), "blocking ready node should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("moat task node is not in progress"), "{stderr}");
    assert!(stderr.contains("review"), "{stderr}");

    cleanup_history_path(&history_path);
}
```

- [ ] **Step 6: Run rejection test to verify it passes against implementation**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_block_task_rejects_unclaimed_ready_task -- --nocapture`

Expected: PASS because `transition_moat_task_node` already rejects non-`InProgress` nodes.

- [ ] **Step 7: Run focused lifecycle tests**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli complete_task block_task -- --nocapture`

Expected: PASS for complete-task and block-task lifecycle tests.

- [ ] **Step 8: Commit**

```bash
git add docs/superpowers/plans/2026-04-27-med-de-id-moat-block-task-lifecycle.md crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs
git commit -m "feat: add moat block-task lifecycle"
```

---

## Self-Review

Spec coverage: The plan adds a minimal, verifiable blocked-state transition for autonomous task lifecycle management, preserving existing task graph and history persistence behavior. Placeholder scan: no TBD/TODO/fill-later placeholders are present. Type consistency: all names match existing CLI patterns and the new `MoatBlockTaskCommand` is parsed via the existing claim-task parser shape.
