# med-de-id Moat Dispatch Next Node ID Filter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded `--node-id NODE_ID` filter to `mdid-cli moat dispatch-next` so external controllers can dispatch a specific ready moat-loop task deterministically.

**Architecture:** Reuse the existing `dispatch-next` ready-node selection pipeline and add one exact-match persisted node-id filter before the single selected task is chosen. The command remains local-first and bounded: dry-run is read-only, non-dry-run mutates only the selected ready node to `in_progress`, and it never launches agents, daemons, PRs, crawlers, or cron jobs.

**Tech Stack:** Rust workspace, `mdid-cli`, `mdid-runtime::moat_history::LocalMoatHistoryStore`, Cargo integration tests.

---

## File Structure

- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add coverage that `moat dispatch-next --node-id NODE_ID --dry-run` selects only the exact ready node.
  - Add coverage that a non-matching `--node-id` exits nonzero without mutating history.
  - Update the `USAGE` constant to include `[--node-id NODE_ID]` for `dispatch-next`.
- Modify: `crates/mdid-cli/src/main.rs`
  - Extend `MoatDispatchNextCommand` with `node_id: Option<String>`.
  - Parse `--node-id NODE_ID`, rejecting missing and duplicate values.
  - Apply exact persisted node-id matching in the dispatch-next selection path before role/kind filters choose the first matching ready node.
  - Update `usage()` with the new flag.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Truth-sync shipped foundation status for the new `dispatch-next --node-id NODE_ID` routing filter.

### Task 1: Add exact node-id routing to `moat dispatch-next`

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`

- [ ] **Step 1: Write the failing node-id dry-run test**

Add this test near the existing `moat_dispatch_next_*` tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn moat_dispatch_next_filters_by_exact_node_id() {
    let history_path = unique_history_path("dispatch-next-node-id");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed dispatch-next node-id history");
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));
    make_workflow_audit_spec_task_ready(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path_arg,
            "--node-id",
            "spec-workflow-audit",
            "--dry-run",
        ])
        .output()
        .expect("failed to dispatch next task with node-id filter");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("dry_run=true\n"));
    assert!(stdout.contains("claimed=false\n"));
    assert!(stdout.contains("node_id=spec-workflow-audit\n"));
    assert!(stdout.contains("spec_ref=moat-spec/workflow-audit\n"));

    cleanup_history_path(&history_path);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_dispatch_next_filters_by_exact_node_id -- --nocapture`

Expected: FAIL with an error such as `unknown option for moat dispatch-next: --node-id`.

- [ ] **Step 3: Write the failing non-match and duplicate/missing validation tests**

Add these tests near the same dispatch-next tests:

```rust
#[test]
fn moat_dispatch_next_node_id_non_match_does_not_claim_ready_task() {
    let history_path = unique_history_path("dispatch-next-node-id-non-match");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed dispatch-next node-id non-match history");
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));
    make_workflow_audit_spec_task_ready(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path_arg,
            "--node-id",
            "missing-node",
        ])
        .output()
        .expect("failed to dispatch next task with non-matching node-id");

    assert!(!output.status.success(), "dispatch-next unexpectedly succeeded");
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("no ready moat task matched dispatch filters"));

    let ready_after = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "ready-tasks", "--history-path", history_path_arg])
        .output()
        .expect("failed to inspect ready tasks after non-matching dispatch");
    assert!(ready_after.status.success(), "{}", String::from_utf8_lossy(&ready_after.stderr));
    assert!(String::from_utf8_lossy(&ready_after.stdout)
        .contains("ready_task=planner|spec_planning|spec-workflow-audit|Create spec for workflow audit|moat-spec/workflow-audit\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn moat_dispatch_next_rejects_missing_node_id_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "dispatch-next", "--node-id"])
        .output()
        .expect("failed to run mdid-cli moat dispatch-next with missing node-id value");

    assert!(!output.status.success(), "dispatch-next unexpectedly succeeded");
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("missing value for moat dispatch-next --node-id"));
}

#[test]
fn moat_dispatch_next_rejects_duplicate_node_id_filter() {
    let history_path = unique_history_path("dispatch-next-node-id-duplicate");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--node-id",
            "first-node",
            "--node-id",
            "second-node",
        ])
        .output()
        .expect("failed to run mdid-cli moat dispatch-next with duplicate node-id filter");

    assert!(!output.status.success(), "dispatch-next unexpectedly succeeded");
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("duplicate moat dispatch-next --node-id"));
}
```

- [ ] **Step 4: Run tests to verify they fail**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_dispatch_next_node_id -- --nocapture`

Expected: FAIL because `--node-id` is not parsed yet.

- [ ] **Step 5: Implement minimal parser and filter support**

In `crates/mdid-cli/src/main.rs`, extend the dispatch-next command struct:

```rust
struct MoatDispatchNextCommand {
    history_path: PathBuf,
    round_id: Option<String>,
    role: Option<MoatAgentRole>,
    kind: Option<MoatTaskKind>,
    node_id: Option<String>,
    dry_run: bool,
}
```

In `parse_moat_dispatch_next_command`, add handling for `--node-id`:

```rust
"--node-id" => {
    if command.node_id.is_some() {
        return Err("duplicate moat dispatch-next --node-id".to_string());
    }
    let Some(value) = iter.next() else {
        return Err("missing value for moat dispatch-next --node-id".to_string());
    };
    command.node_id = Some(value.to_string());
}
```

In the function that chooses the dispatch node, add exact matching before role/kind acceptance:

```rust
if let Some(expected_node_id) = command.node_id.as_deref() {
    if node.node_id != expected_node_id {
        continue;
    }
}
```

Update `usage()` so the dispatch-next segment reads:

```text
moat dispatch-next --history-path PATH [--round-id ROUND_ID] [--role planner|coder|reviewer] [--kind KIND] [--node-id NODE_ID] [--dry-run]
```

Update the test `USAGE` constant to match the production usage string exactly.

- [ ] **Step 6: Run targeted tests to verify they pass**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_dispatch_next_node_id -- --nocapture`

Expected: PASS for the node-id filter, non-match, missing value, and duplicate filter tests.

- [ ] **Step 7: Run existing dispatch-next regression tests**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_dispatch_next -- --nocapture`

Expected: PASS for all existing dispatch-next tests plus the new node-id tests.

- [ ] **Step 8: Truth-sync the moat-loop spec**

In `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`, update the shipped foundation bullet for `mdid-cli moat dispatch-next` so it includes `[--node-id NODE_ID]` and states that `--node-id` exact-matches persisted ready node IDs before selection.

- [ ] **Step 9: Run final relevant verification**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_dispatch_next -- --nocapture`

Expected: PASS.

Run: `git diff --check`

Expected: no whitespace errors.

- [ ] **Step 10: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-27-med-de-id-moat-dispatch-next-node-id-filter.md
git commit -m "feat: filter moat dispatch by node id"
```

## Self-Review

- Spec coverage: the plan implements a bounded exact node-id filter for `dispatch-next`, including dry-run, mutating non-match safety, parser validation, usage, and spec truth-sync.
- Placeholder scan: no TBD/TODO/fill-in placeholders remain; every code/test step includes exact content or exact insertion behavior.
- Type consistency: the plan uses `node_id: Option<String>`, persisted `node.node_id`, and the CLI flag `--node-id NODE_ID` consistently across tests, parser, usage, and docs.
