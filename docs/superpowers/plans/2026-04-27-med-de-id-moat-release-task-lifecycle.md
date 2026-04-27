# Med De ID Moat Release Task Lifecycle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `mdid-cli moat release-task` command that returns a claimed `in_progress` moat task to `ready` so autonomous workers can safely release work without completing or blocking it.

**Architecture:** Reuse the existing local moat history task-state transition machinery in `mdid-runtime` and expose the narrow lifecycle transition through `mdid-cli`. Keep the slice intentionally small: one runtime transition method, one CLI command parser/runner, and integration/unit tests that prove the transition and rejection behavior.

**Tech Stack:** Rust workspace, `mdid-runtime` local JSON history store, `mdid-cli` command parser, Cargo integration tests.

---

## File Structure

- Modify: `crates/mdid-runtime/src/moat_history.rs`
  - Add `release_in_progress_task(round_id, node_id)` as an `InProgress -> Ready` transition using the existing `transition_task_state` helper.
- Modify: `crates/mdid-cli/src/main.rs`
  - Import/use the runtime transition through a new `MoatReleaseTaskCommand`.
  - Parse `mdid-cli moat release-task --history-path PATH [--round-id ROUND_ID] --node-id NODE_ID`.
  - Run the command and print `moat task released` and `round_id=<id>`.
  - Include the command in usage and parser unit tests.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add end-to-end integration tests for successful release of a claimed task and rejection of non-`in_progress` tasks.

## Task 1: Runtime release transition

**Files:**
- Modify: `crates/mdid-runtime/src/moat_history.rs`

- [ ] **Step 1: Write the failing runtime test**

Add this test to the existing `#[cfg(test)] mod tests` in `crates/mdid-runtime/src/moat_history.rs`:

```rust
#[test]
fn release_in_progress_task_returns_node_to_ready() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let history_path = temp_dir.path().join("moat-history.json");
    let round_id = Uuid::from_u128(0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa);
    let mut entry = sample_history_entry(round_id, true);
    entry.report.control_plane.task_graph.nodes[0].state = MoatTaskNodeState::InProgress;
    fs::write(
        &history_path,
        serde_json::to_vec_pretty(&vec![entry]).expect("failed to serialize history"),
    )
    .expect("failed to write history");

    let mut store = LocalMoatHistoryStore::new(history_path.clone()).expect("failed to open history");
    let selected_round_id = store
        .release_in_progress_task(None, "strategy_generation")
        .expect("failed to release task");

    assert_eq!(selected_round_id, round_id.to_string());
    let entries = load_entries(&history_path).expect("failed to reload entries");
    assert_eq!(
        entries[0].report.control_plane.task_graph.nodes[0].state,
        MoatTaskNodeState::Ready
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-runtime release_in_progress_task_returns_node_to_ready -- --nocapture
```

Expected: FAIL to compile with an error equivalent to `no method named release_in_progress_task found for struct LocalMoatHistoryStore`.

- [ ] **Step 3: Write minimal runtime implementation**

Add this method beside `block_in_progress_task` and `unblock_blocked_task` in `impl LocalMoatHistoryStore`:

```rust
pub fn release_in_progress_task(
    &mut self,
    round_id: Option<&str>,
    node_id: &str,
) -> Result<String, CompleteInProgressTaskError> {
    self.transition_task_state(
        round_id,
        node_id,
        MoatTaskNodeState::InProgress,
        MoatTaskNodeState::Ready,
    )
}
```

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-runtime release_in_progress_task_returns_node_to_ready -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-runtime/src/moat_history.rs docs/superpowers/plans/2026-04-27-med-de-id-moat-release-task-lifecycle.md
git commit -m "feat: add moat release task runtime transition"
```

## Task 2: CLI release-task command

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/moat_cli.rs`

- [ ] **Step 1: Write failing CLI integration test for successful release**

Add this test to `crates/mdid-cli/tests/moat_cli.rs` near the existing task lifecycle tests:

```rust
#[test]
fn cli_releases_claimed_moat_task() {
    let history_path = unique_history_path("release-task-claimed");
    let history_path_arg = history_path.to_str().expect("history path should be utf8");
    seed_history(&history_path, default_history_entry()).expect("failed to seed release-task history");

    let claim = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "claim-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "strategy_generation",
        ])
        .output()
        .expect("failed to claim task before release");
    assert!(claim.status.success(), "{}", String::from_utf8_lossy(&claim.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "release-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "strategy_generation",
        ])
        .output()
        .expect("failed to run mdid-cli moat release-task");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("moat task released\n"));
    assert!(stdout.contains("round_id="));

    let graph = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "task-graph", "--history-path", history_path_arg])
        .output()
        .expect("failed to inspect task graph after release");
    assert!(graph.status.success(), "{}", String::from_utf8_lossy(&graph.stderr));
    let graph_stdout = String::from_utf8_lossy(&graph.stdout);
    assert!(graph_stdout.contains("strategy_generation"));
    assert!(graph_stdout.contains("ready"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_releases_claimed_moat_task -- --nocapture
```

Expected: FAIL because `moat release-task` is an unknown command.

- [ ] **Step 3: Implement parser, runner, and usage**

In `crates/mdid-cli/src/main.rs`, make these concrete edits:

1. Add a command struct near the other task lifecycle command structs:

```rust
#[derive(Debug, PartialEq, Eq)]
struct MoatReleaseTaskCommand {
    history_path: String,
    round_id: Option<String>,
    node_id: String,
}
```

2. Add a `CliCommand` variant:

```rust
MoatReleaseTask(MoatReleaseTaskCommand),
```

3. Add a dispatch arm in `main` beside block/unblock task dispatch:

```rust
Ok(CliCommand::MoatReleaseTask(command)) => {
    if let Err(error) = run_moat_release_task(&command) {
        eprintln!("error: {error}");
        process::exit(1);
    }
}
```

4. Add a parse arm beside `block-task`/`unblock-task`:

```rust
[moat, release_task, rest @ ..] if moat == "moat" && release_task == "release-task" => Ok(
    CliCommand::MoatReleaseTask(parse_moat_release_task_command(rest)?),
),
```

5. Add parser functions by reusing the complete/block task option parser shape:

```rust
fn parse_moat_release_task_command(args: &[String]) -> Result<MoatReleaseTaskCommand, String> {
    let command = parse_moat_complete_task_command(args)?;
    Ok(MoatReleaseTaskCommand {
        history_path: command.history_path,
        round_id: command.round_id,
        node_id: command.node_id,
    })
}
```

6. Add runner function beside `run_moat_block_task`/`run_moat_unblock_task`:

```rust
fn run_moat_release_task(command: &MoatReleaseTaskCommand) -> Result<(), String> {
    let history_path = PathBuf::from(&command.history_path);
    let mut store = LocalMoatHistoryStore::new(history_path)
        .map_err(|error| format!("failed to load moat history: {error}"))?;
    let round_id = store
        .release_in_progress_task(command.round_id.as_deref(), &command.node_id)
        .map_err(|error| format!("failed to release moat task: {error}"))?;

    println!("moat task released");
    println!("round_id={round_id}");
    println!("node_id={}", command.node_id);
    Ok(())
}
```

7. Update the `USAGE` string to include:

```text
| moat release-task --history-path PATH [--round-id ROUND_ID] --node-id NODE_ID
```

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_releases_claimed_moat_task -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Write failing CLI integration test for invalid state rejection**

Add this test to `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn release_task_rejects_ready_node() {
    let history_path = unique_history_path("release-task-ready");
    let history_path_arg = history_path.to_str().expect("history path should be utf8");
    seed_history(&history_path, default_history_entry()).expect("failed to seed release-task history");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "release-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "strategy_generation",
        ])
        .output()
        .expect("failed to run mdid-cli moat release-task for ready node");

    assert!(!output.status.success(), "releasing ready node should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed to release moat task"));
    assert!(stderr.contains("expected in_progress"));
}
```

- [ ] **Step 6: Run invalid-state test to verify it passes with existing implementation**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli release_task_rejects_ready_node -- --nocapture
```

Expected: PASS because the runtime transition helper rejects non-`in_progress` nodes.

- [ ] **Step 7: Add parser unit test**

Add this unit test to `crates/mdid-cli/src/main.rs` in the existing tests module:

```rust
#[test]
fn parses_moat_release_task_command() {
    let args = vec![
        "moat".to_string(),
        "release-task".to_string(),
        "--history-path".to_string(),
        "history.json".to_string(),
        "--round-id".to_string(),
        "round-1".to_string(),
        "--node-id".to_string(),
        "strategy_generation".to_string(),
    ];

    assert_eq!(
        parse_command(&args),
        Ok(CliCommand::MoatReleaseTask(MoatReleaseTaskCommand {
            history_path: "history.json".to_string(),
            round_id: Some("round-1".to_string()),
            node_id: "strategy_generation".to_string(),
        }))
    );
}
```

- [ ] **Step 8: Run parser unit test**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli parses_moat_release_task_command -- --nocapture
```

Expected: PASS.

- [ ] **Step 9: Run targeted broader package verification**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-runtime release_in_progress_task_returns_node_to_ready -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli release_task -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli parses_moat_release_task_command -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli -- --nocapture
```

Expected: all PASS.

- [ ] **Step 10: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs
git commit -m "feat: add moat release-task cli"
```

## Self-Review

**Spec coverage:** This plan covers the requested autonomous multi-agent moat-loop direction by improving task lifecycle safety: workers can claim, complete, block, unblock, and now release abandoned or intentionally yielded work. This is a releaseable, core-flow slice for the task graph/control-plane execution loop.

**Placeholder scan:** No placeholders, TBDs, or unspecified edge cases remain.

**Type consistency:** `release_in_progress_task`, `MoatReleaseTaskCommand`, `parse_moat_release_task_command`, and `run_moat_release_task` names are used consistently across runtime, CLI parser, runner, and tests.
