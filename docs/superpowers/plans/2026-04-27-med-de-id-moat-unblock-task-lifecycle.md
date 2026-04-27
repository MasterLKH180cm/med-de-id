# med-de-id Moat Unblock Task Lifecycle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded `mdid-cli moat unblock-task` local coordination mutation that returns a blocked task to ready so external autonomous controllers can recover and continue the task graph.

**Architecture:** Extend the existing moat task lifecycle surfaces (`claim-task`, `complete-task`, `block-task`) with the symmetric blocked-to-ready transition. Keep mutation in `LocalMoatHistoryStore` so the CLI remains thin and all state persistence uses the same lock/reload/write path as current lifecycle operations.

**Tech Stack:** Rust workspace, `mdid-runtime` local JSON history store, `mdid-cli` binary integration tests, Cargo targeted test execution with `CARGO_INCREMENTAL=0`.

---

## File Structure

- Modify: `crates/mdid-runtime/src/moat_history.rs`
  - Add `unblock_blocked_task(&self, round_id: Option<&str>, node_id: &str) -> Result<MoatHistoryEntry, String>`.
  - Add a generic task transition helper that can require a specified previous state so block/complete continue requiring `in_progress` while unblock requires `blocked`.
- Modify: `crates/mdid-cli/src/main.rs`
  - Add `MoatUnblockTaskCommand`, parser route `moat unblock-task`, usage text, runner `run_moat_unblock_task`, and stable output lines.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add integration tests for successful unblock of a blocked task and rejection when the task is not blocked.
  - Update the duplicated usage string fixture to include `unblock-task`.

### Task 1: CLI + runtime unblock-task lifecycle

**Files:**
- Modify: `crates/mdid-runtime/src/moat_history.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/moat_cli.rs`

- [ ] **Step 1: Write the failing successful-unblock test**

Add this test near the existing block-task tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn cli_unblocks_blocked_moat_task_to_ready() {
    let history_path = unique_history_path("unblock-task-blocked");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed unblock-task history");
    assert!(
        seed_output.status.success(),
        "expected seed round success, stderr was: {}",
        String::from_utf8_lossy(&seed_output.stderr)
    );

    let store = LocalMoatHistoryStore::open(&history_path).expect("history store should open");
    let round_id = store
        .summary()
        .latest_round_id
        .expect("seeded history should have latest round id");

    let claim_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "claim-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
            "--round-id",
            &round_id,
        ])
        .output()
        .expect("failed to claim task before blocking");
    assert!(
        claim_output.status.success(),
        "expected claim success, stderr was: {}",
        String::from_utf8_lossy(&claim_output.stderr)
    );

    let block_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "block-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
            "--round-id",
            &round_id,
        ])
        .output()
        .expect("failed to block task before unblocking");
    assert!(
        block_output.status.success(),
        "expected block success, stderr was: {}",
        String::from_utf8_lossy(&block_output.stderr)
    );

    let unblock_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "unblock-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
            "--round-id",
            &round_id,
        ])
        .output()
        .expect("failed to run mdid-cli moat unblock-task");

    assert!(
        unblock_output.status.success(),
        "expected unblock success, stderr was: {}",
        String::from_utf8_lossy(&unblock_output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&unblock_output.stdout),
        format!(
            "moat task unblocked\nround_id={round_id}\nnode_id=review\nprevious_state=blocked\nnew_state=ready\nhistory_path={history_path_arg}\n"
        )
    );

    let graph_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--round-id",
            &round_id,
            "--node-id",
            "review",
        ])
        .output()
        .expect("failed to inspect task graph after unblock");
    assert!(
        graph_output.status.success(),
        "expected task graph success, stderr was: {}",
        String::from_utf8_lossy(&graph_output.stderr)
    );
    assert!(String::from_utf8_lossy(&graph_output.stdout)
        .contains("node=reviewer|review|Review|review|ready|implementation|<none>\n"));

    cleanup_history_path(&history_path);
}
```

- [ ] **Step 2: Run the successful-unblock test to verify RED**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_unblocks_blocked_moat_task_to_ready -- --exact --nocapture
```

Expected: FAIL because `moat unblock-task` is not recognized and usage is printed.

- [ ] **Step 3: Write the failing non-blocked rejection test**

Add this test next to the successful unblock test:

```rust
#[test]
fn cli_rejects_unblock_task_when_task_is_not_blocked() {
    let history_path = unique_history_path("unblock-task-ready");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed unblock-task history");
    assert!(
        seed_output.status.success(),
        "expected seed round success, stderr was: {}",
        String::from_utf8_lossy(&seed_output.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "unblock-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
        ])
        .output()
        .expect("failed to run mdid-cli moat unblock-task for ready node");

    assert!(
        !output.status.success(),
        "expected unblock failure for non-blocked task"
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "error: node 'review' is ready, expected blocked\n"
    );

    cleanup_history_path(&history_path);
}
```

- [ ] **Step 4: Run the non-blocked rejection test to verify RED**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_rejects_unblock_task_when_task_is_not_blocked -- --exact --nocapture
```

Expected: FAIL because `moat unblock-task` is not recognized and usage is printed instead of the expected blocked-state error.

- [ ] **Step 5: Implement the minimal runtime transition**

In `crates/mdid-runtime/src/moat_history.rs`, replace `complete_in_progress_task`, `block_in_progress_task`, and the helper signature with this implementation:

```rust
    pub fn complete_in_progress_task(
        &self,
        round_id: Option<&str>,
        node_id: &str,
    ) -> Result<MoatHistoryEntry, String> {
        self.transition_task(
            round_id,
            node_id,
            MoatTaskNodeState::InProgress,
            MoatTaskNodeState::Completed,
        )
    }

    pub fn block_in_progress_task(
        &self,
        round_id: Option<&str>,
        node_id: &str,
    ) -> Result<MoatHistoryEntry, String> {
        self.transition_task(
            round_id,
            node_id,
            MoatTaskNodeState::InProgress,
            MoatTaskNodeState::Blocked,
        )
    }

    pub fn unblock_blocked_task(
        &self,
        round_id: Option<&str>,
        node_id: &str,
    ) -> Result<MoatHistoryEntry, String> {
        self.transition_task(
            round_id,
            node_id,
            MoatTaskNodeState::Blocked,
            MoatTaskNodeState::Ready,
        )
    }

    fn transition_task(
        &self,
        round_id: Option<&str>,
        node_id: &str,
        expected_state: MoatTaskNodeState,
        next_state: MoatTaskNodeState,
    ) -> Result<MoatHistoryEntry, String> {
        let _claim_lock = ClaimReadyTaskLock::acquire(&self.path)?;
        let current_entries = read_entries(&self.path)?;
        let selected_index = select_entry_index(&current_entries, round_id)?;
        let mut updated_entries = current_entries.clone();
        let entry = updated_entries
            .get_mut(selected_index)
            .expect("selected index should reference an entry");
        let selected_round_id = entry.report.summary.round_id.clone();
        let node = entry
            .report
            .task_graph
            .nodes
            .iter_mut()
            .find(|candidate| candidate.node_id == node_id)
            .ok_or_else(|| {
                format!(
                    "node '{node_id}' was not found in round '{selected_round_id}'"
                )
            })?;

        if node.state != expected_state {
            return Err(format!(
                "node '{node_id}' is {}, expected {}",
                task_state_label(&node.state),
                task_state_label(&expected_state)
            ));
        }

        node.state = next_state;
        write_entries(&self.path, &updated_entries)?;
        Ok(updated_entries[selected_index].clone())
    }
```

- [ ] **Step 6: Implement the minimal CLI parser and runner**

In `crates/mdid-cli/src/main.rs`:

1. Add this struct after `MoatBlockTaskCommand`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
struct MoatUnblockTaskCommand {
    history_path: String,
    round_id: Option<String>,
    node_id: String,
}
```

2. Add a `main` match arm immediately after `MoatBlockTask`:

```rust
        Ok(CliCommand::MoatUnblockTask(command)) => {
            if let Err(error) = run_moat_unblock_task(&command) {
                exit_with_error(error);
            }
        }
```

3. Add the enum variant immediately after `MoatBlockTask`:

```rust
    MoatUnblockTask(MoatUnblockTaskCommand),
```

4. Add a parser route immediately after `block-task`:

```rust
        [moat, unblock_task, rest @ ..] if moat == "moat" && unblock_task == "unblock-task" => Ok(
            CliCommand::MoatUnblockTask(parse_moat_unblock_task_command(rest)?),
        ),
```

5. Add this parser helper after `parse_moat_block_task_command`:

```rust
fn parse_moat_unblock_task_command(args: &[String]) -> Result<MoatUnblockTaskCommand, String> {
    let command = parse_moat_claim_task_command(args)?;
    Ok(MoatUnblockTaskCommand {
        history_path: command.history_path,
        round_id: command.round_id,
        node_id: command.node_id,
    })
}
```

6. Add this runner after `run_moat_block_task`:

```rust
fn run_moat_unblock_task(command: &MoatUnblockTaskCommand) -> Result<(), String> {
    let store = LocalMoatHistoryStore::open_existing(&command.history_path)?;
    let entry = store.unblock_blocked_task(command.round_id.as_deref(), &command.node_id)?;
    println!("moat task unblocked");
    println!("round_id={}", entry.report.summary.round_id);
    println!("node_id={}", command.node_id);
    println!("previous_state=blocked");
    println!("new_state=ready");
    println!("history_path={}", command.history_path);
    Ok(())
}
```

7. Update both CLI usage strings (`USAGE` in `main.rs` and the duplicated test fixture in `moat_cli.rs`) so the moat command list includes:

```text
| moat unblock-task --history-path PATH --node-id NODE_ID [--round-id ROUND_ID]
```

- [ ] **Step 7: Run the targeted unblock tests to verify GREEN**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_unblocks_blocked_moat_task_to_ready cli_rejects_unblock_task_when_task_is_not_blocked -- --nocapture
```

Expected: PASS for both tests.

- [ ] **Step 8: Run lifecycle regression tests**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli claim_task complete_task block_task unblock_task -- --nocapture
```

Expected: all matching lifecycle tests pass.

- [ ] **Step 9: Run broader relevant CLI tests**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli -- --nocapture
```

Expected: all `moat_cli` integration tests pass.

- [ ] **Step 10: Commit**

Run:

```bash
git add docs/superpowers/plans/2026-04-27-med-de-id-moat-unblock-task-lifecycle.md crates/mdid-runtime/src/moat_history.rs crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs
git commit -m "feat: add moat unblock-task lifecycle"
```
