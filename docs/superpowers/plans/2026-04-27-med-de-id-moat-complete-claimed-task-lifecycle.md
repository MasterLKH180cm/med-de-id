# Med De-ID Moat Complete Claimed Task Lifecycle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a persisted `mdid-cli moat complete-task` lifecycle mutation so autonomous workers can finish a claimed task and unlock the next ready task.

**Architecture:** Mirror the existing local persisted coordination model used by `moat claim-task`: the CLI parses a bounded mutation command, the runtime history store reloads the latest on-disk history before mutation, validates the selected task state, persists the transition, and existing task-graph dependency logic exposes newly ready downstream tasks. This slice intentionally does not launch agents or store arbitrary artifacts; it adds the smallest workflow primitive needed after claim-task.

**Tech Stack:** Rust workspace, `mdid-cli`, `mdid-runtime::moat_history::LocalMoatHistoryStore`, `mdid-domain::MoatTaskNodeState`, Cargo integration tests.

---

## File Structure

- Modify: `crates/mdid-runtime/src/moat_history.rs`
  - Add a `complete_in_progress_task(round_id: Option<&str>, node_id: &str)` store mutation beside `claim_ready_task`.
  - Reload on-disk history immediately before mutation, select latest-or-exact round, require `InProgress`, persist `Completed`.
- Modify: `crates/mdid-runtime/tests/moat_history.rs`
  - Add persistence tests proving claim then complete survives reload and rejects non-claimed tasks.
- Modify: `crates/mdid-cli/src/main.rs`
  - Add `moat complete-task --history-path PATH --node-id NODE_ID [--round-id ROUND_ID]` usage, parser, command enum variant, and runner.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add CLI integration tests for completing a claimed task and rejecting an unclaimed task.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Truth-sync shipped surface with `moat complete-task` semantics.
- Modify: `docs/superpowers/plans/2026-04-27-med-de-id-moat-complete-claimed-task-lifecycle.md`
  - Mark implementation notes after verification.

### Task 1: Persist Complete-Task Runtime Mutation

**Files:**
- Modify: `crates/mdid-runtime/src/moat_history.rs`
- Test: `crates/mdid-runtime/tests/moat_history.rs`

- [ ] **Step 1: Write failing runtime tests**

Add tests near the existing `claim_ready_task` tests in `crates/mdid-runtime/tests/moat_history.rs`:

```rust
#[test]
fn claim_then_complete_task_persists_completed_state() {
    let temp = tempfile::tempdir().expect("tempdir");
    let history_path = temp.path().join("moat-history.jsonl");
    let store = LocalMoatHistoryStore::new(&history_path);

    let report = run_bounded_round(MoatLoopConfig {
        history_path: Some(history_path.clone()),
        ..MoatLoopConfig::default()
    })
    .expect("round should run");

    let ready_node = report
        .control_plane
        .task_graph
        .ready_node_ids()
        .into_iter()
        .next()
        .expect("round should expose a ready task");

    store
        .claim_ready_task(Some(&report.summary.round_id), &ready_node)
        .expect("claim should persist");
    store
        .complete_in_progress_task(Some(&report.summary.round_id), &ready_node)
        .expect("completion should persist");

    let entries = store.load().expect("history should reload");
    let node = entries[0]
        .report
        .control_plane
        .task_graph
        .node(&ready_node)
        .expect("completed node should still exist");

    assert_eq!(node.state, MoatTaskNodeState::Completed);
}

#[test]
fn complete_task_rejects_non_in_progress_node() {
    let temp = tempfile::tempdir().expect("tempdir");
    let history_path = temp.path().join("moat-history.jsonl");
    let store = LocalMoatHistoryStore::new(&history_path);

    let report = run_bounded_round(MoatLoopConfig {
        history_path: Some(history_path.clone()),
        ..MoatLoopConfig::default()
    })
    .expect("round should run");

    let ready_node = report
        .control_plane
        .task_graph
        .ready_node_ids()
        .into_iter()
        .next()
        .expect("round should expose a ready task");

    let err = store
        .complete_in_progress_task(Some(&report.summary.round_id), &ready_node)
        .expect_err("ready task cannot be completed before claim");

    assert!(
        err.to_string().contains("is not in_progress"),
        "unexpected error: {err}"
    );
}
```

- [ ] **Step 2: Run tests to verify RED**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-runtime --test moat_history claim_then_complete_task_persists_completed_state -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-runtime --test moat_history complete_task_rejects_non_in_progress_node -- --nocapture
```

Expected: both fail to compile because `complete_in_progress_task` is not defined.

- [ ] **Step 3: Implement minimal runtime mutation**

In `crates/mdid-runtime/src/moat_history.rs`, add this method to `impl LocalMoatHistoryStore` beside `claim_ready_task`:

```rust
    pub fn complete_in_progress_task(
        &self,
        round_id: Option<&str>,
        node_id: &str,
    ) -> Result<(), MoatHistoryError> {
        let mut entries = self.load()?;
        let entry = select_mutable_history_entry(&mut entries, round_id)?;
        let node = entry
            .report
            .control_plane
            .task_graph
            .node_mut(node_id)
            .ok_or_else(|| MoatHistoryError::TaskNotFound(node_id.to_string()))?;

        if node.state != MoatTaskNodeState::InProgress {
            return Err(MoatHistoryError::InvalidTaskState {
                node_id: node_id.to_string(),
                expected: MoatTaskNodeState::InProgress,
                actual: node.state,
            });
        }

        node.state = MoatTaskNodeState::Completed;
        self.replace_all(&entries)
    }
```

If the exact helper/error names differ, mirror the names already used by `claim_ready_task`; do not introduce parallel error types.

- [ ] **Step 4: Run runtime tests to verify GREEN**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-runtime --test moat_history claim_then_complete_task_persists_completed_state -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-runtime --test moat_history complete_task_rejects_non_in_progress_node -- --nocapture
```

Expected: both pass.

- [ ] **Step 5: Commit runtime slice**

```bash
git add crates/mdid-runtime/src/moat_history.rs crates/mdid-runtime/tests/moat_history.rs
git commit -m "feat: persist moat task completion"
```

### Task 2: Add Complete-Task CLI Surface

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/moat_cli.rs`

- [ ] **Step 1: Write failing CLI tests**

Add tests near existing claim-task tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn cli_completes_claimed_moat_task() {
    let temp = tempfile::tempdir().expect("tempdir");
    let history_path = temp.path().join("moat-history.jsonl");
    let history_path_arg = history_path.to_str().expect("utf8 history path");

    let round_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to run moat round");
    assert_success(&round_output);

    let ready_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "ready-tasks", "--history-path", history_path_arg, "--limit", "1"])
        .output()
        .expect("failed to list ready tasks");
    assert_success(&ready_output);
    let ready_stdout = String::from_utf8(ready_output.stdout).expect("ready stdout utf8");
    let ready_node_id = ready_stdout
        .lines()
        .find_map(|line| line.strip_prefix("ready_task="))
        .and_then(|entry| entry.split('|').nth(2))
        .expect("ready task node id")
        .to_string();

    let claim_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "claim-task", "--history-path", history_path_arg, "--node-id", &ready_node_id])
        .output()
        .expect("failed to claim task");
    assert_success(&claim_output);

    let complete_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "complete-task", "--history-path", history_path_arg, "--node-id", &ready_node_id])
        .output()
        .expect("failed to complete task");
    assert_success(&complete_output);
    let stdout = String::from_utf8(complete_output.stdout).expect("complete stdout utf8");

    assert!(stdout.contains("moat task completed\n"), "stdout was {stdout}");
    assert!(stdout.contains(&format!("node_id={ready_node_id}\n")), "stdout was {stdout}");
    assert!(stdout.contains("previous_state=in_progress\n"), "stdout was {stdout}");
    assert!(stdout.contains("new_state=completed\n"), "stdout was {stdout}");
    assert!(stdout.contains(&format!("history_path={history_path_arg}\n")), "stdout was {stdout}");
}

#[test]
fn cli_complete_task_rejects_unclaimed_ready_task() {
    let temp = tempfile::tempdir().expect("tempdir");
    let history_path = temp.path().join("moat-history.jsonl");
    let history_path_arg = history_path.to_str().expect("utf8 history path");

    let round_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to run moat round");
    assert_success(&round_output);

    let ready_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "ready-tasks", "--history-path", history_path_arg, "--limit", "1"])
        .output()
        .expect("failed to list ready tasks");
    assert_success(&ready_output);
    let ready_stdout = String::from_utf8(ready_output.stdout).expect("ready stdout utf8");
    let ready_node_id = ready_stdout
        .lines()
        .find_map(|line| line.strip_prefix("ready_task="))
        .and_then(|entry| entry.split('|').nth(2))
        .expect("ready task node id")
        .to_string();

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "complete-task", "--history-path", history_path_arg, "--node-id", &ready_node_id])
        .output()
        .expect("failed to run complete-task");

    assert!(!output.status.success(), "complete-task unexpectedly succeeded");
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("is not in_progress"), "stderr was {stderr}");
}
```

- [ ] **Step 2: Run CLI tests to verify RED**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_completes_claimed_moat_task -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_complete_task_rejects_unclaimed_ready_task -- --nocapture
```

Expected: both fail because `moat complete-task` is not recognized.

- [ ] **Step 3: Implement minimal CLI command**

In `crates/mdid-cli/src/main.rs`:

1. Add usage line:

```rust
  moat complete-task --history-path PATH --node-id NODE_ID [--round-id ROUND_ID]
```

2. Add enum variant and struct mirroring claim-task:

```rust
    MoatCompleteTask(MoatCompleteTaskCommand),
```

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
struct MoatCompleteTaskCommand {
    history_path: String,
    round_id: Option<String>,
    node_id: String,
}
```

3. Add parse dispatch:

```rust
        ["moat", "complete-task", rest @ ..] => {
            parse_moat_complete_task_command(rest).map(CliCommand::MoatCompleteTask)
        }
```

4. Add parser by copying claim-task parser and changing names:

```rust
fn parse_moat_complete_task_command(args: &[&str]) -> Result<MoatCompleteTaskCommand, CliError> {
    let mut history_path = None;
    let mut round_id = None;
    let mut node_id = None;
    let mut iter = args.iter();

    while let Some(arg) = iter.next() {
        match *arg {
            "--history-path" => history_path = Some(required_value(&mut iter, "--history-path")?.to_string()),
            "--round-id" => round_id = Some(required_value(&mut iter, "--round-id")?.to_string()),
            "--node-id" => node_id = Some(required_value(&mut iter, "--node-id")?.to_string()),
            unknown => return Err(CliError::Message(format!("unknown moat complete-task argument: {unknown}"))),
        }
    }

    Ok(MoatCompleteTaskCommand {
        history_path: history_path.ok_or_else(|| CliError::Message("missing --history-path".to_string()))?,
        round_id,
        node_id: node_id.ok_or_else(|| CliError::Message("missing --node-id".to_string()))?,
    })
}
```

5. Add runner dispatch:

```rust
        CliCommand::MoatCompleteTask(command) => run_moat_complete_task(command),
```

6. Add runner mirroring claim-task:

```rust
fn run_moat_complete_task(command: MoatCompleteTaskCommand) -> Result<(), CliError> {
    let history_path = PathBuf::from(&command.history_path);
    let store = LocalMoatHistoryStore::new(&history_path);
    let before_entries = store.load()?;
    let selected = select_history_entry(&before_entries, command.round_id.as_deref())?;
    let node = selected
        .report
        .control_plane
        .task_graph
        .node(&command.node_id)
        .ok_or_else(|| CliError::Message(format!("moat task node not found: {}", command.node_id)))?;

    if node.state != MoatTaskNodeState::InProgress {
        return Err(CliError::Message(format!(
            "moat task node {} is not in_progress: {}",
            command.node_id,
            format_task_node_state(node.state)
        )));
    }

    store.complete_in_progress_task(command.round_id.as_deref(), &command.node_id)?;

    println!("moat task completed");
    println!("round_id={}", selected.report.summary.round_id);
    println!("node_id={}", command.node_id);
    println!("previous_state=in_progress");
    println!("new_state=completed");
    println!("history_path={}", history_path.display());

    Ok(())
}
```

If local helper names differ, mirror `run_moat_claim_task` exactly.

- [ ] **Step 4: Run CLI tests to verify GREEN**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_completes_claimed_moat_task -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_complete_task_rejects_unclaimed_ready_task -- --nocapture
```

Expected: both pass.

- [ ] **Step 5: Commit CLI slice**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs
git commit -m "feat: add moat complete-task cli"
```

### Task 3: Truth-Sync Spec and Verify Package

**Files:**
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
- Modify: `docs/superpowers/plans/2026-04-27-med-de-id-moat-complete-claimed-task-lifecycle.md`

- [ ] **Step 1: Update spec shipped surface**

In `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`, add a bullet after the `claim-task` bullet:

```markdown
- `mdid-cli moat complete-task --history-path PATH --node-id NODE_ID [--round-id ROUND_ID]` is a bounded local coordination mutation for external Planner/Coder/Reviewer controllers. It opens only an existing history file, selects the latest persisted round unless `--round-id` exact-matches a specific persisted round, reloads latest on-disk history before mutation, requires the selected node to be `in_progress`, persists only that task transition to `completed`, and leaves task execution/artifact generation to the external worker.
```

Also update the paragraph beginning `This shipped slice is intentionally narrower...` so it says `persisted ready-task claiming, claimed-task completion, and bounded markdown export`.

- [ ] **Step 2: Run relevant broader package tests**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-runtime --test moat_history -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test cli_smoke -- --nocapture
```

Expected: all pass. If Cargo's name filter misses tests, run `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli -- --nocapture`.

- [ ] **Step 3: Update plan completion notes**

Append this section to this plan:

```markdown
## Implementation Notes

- Implemented `mdid-cli moat complete-task` as a persisted local lifecycle mutation from `in_progress` to `completed`.
- Verified runtime persistence, CLI happy path, CLI rejection path, and relevant broader moat CLI smoke coverage with `CARGO_INCREMENTAL=0`.
```

- [ ] **Step 4: Commit docs and verification notes**

```bash
git add docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-27-med-de-id-moat-complete-claimed-task-lifecycle.md
git commit -m "docs: sync moat task completion lifecycle"
```

- [ ] **Step 5: Final gitflow merge gate**

Run:

```bash
git status --short
git checkout develop
git merge --no-ff feature/moat-loop-autonomy -m "merge: moat complete-task lifecycle"
```

Expected: merge succeeds only after tests are green and worktree is clean.

## Implementation Notes

- Implemented `mdid-cli moat complete-task` as a persisted local lifecycle mutation from `in_progress` to `completed`.
- Truth-synced the moat-loop design spec with the shipped bounded `complete-task` lifecycle surface and updated the shipped-slice summary to include claimed-task completion.
- Verified runtime persistence coverage with `CARGO_INCREMENTAL=0 cargo test -p mdid-runtime --test moat_history -- --nocapture`.
- Verified complete-task CLI happy/rejection paths with `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_completes_claimed_moat_task -- --nocapture` and `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_complete_task_rejects_unclaimed_ready_task -- --nocapture`.
- Verified CLI smoke coverage with `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test cli_smoke -- --nocapture`.
- Verified broader filtered moat CLI coverage with `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat -- --nocapture` after syncing the shared `moat_cli` usage fixture for `complete-task`.
