# med-de-id Moat Dispatch Next Agent Task Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded `mdid-cli moat dispatch-next` command that selects one ready moat-loop task, optionally claims it, and emits a deterministic dispatch envelope for an external Planner/Coder/Reviewer controller.

**Architecture:** Reuse the persisted local history/control-plane task graph as the source of truth, matching existing `ready-tasks` and `claim-task` semantics. The command remains local-first and bounded: dry-run is read-only, non-dry-run mutates only one selected ready node to `in_progress`, and no agent process, daemon, PR, crawler, or cron job is launched.

**Tech Stack:** Rust workspace, `mdid-cli`, `mdid-runtime::moat_history::LocalMoatHistoryStore`, `mdid-domain` moat task graph types, Cargo integration tests.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `CliCommand::MoatDispatchNext` and `MoatDispatchNextCommand`.
  - Parse `moat dispatch-next --history-path PATH [--round-id ROUND_ID] [--role planner|coder|reviewer] [--kind KIND] [--dry-run]`.
  - Select exactly one ready node in persisted order after filters.
  - For dry-run, print the dispatch envelope without modifying history.
  - For non-dry-run, reload latest history and transition exactly that selected node from `ready` to `in_progress`, then print the same envelope plus claim metadata.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add TDD coverage for dry-run read-only dispatch, mutating dispatch, role/kind filtering, no-ready-task error, duplicate dry-run rejection, and missing history non-creation.
  - Update `USAGE` constant.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Truth-sync shipped foundation status with bounded one-task dispatch envelope support.
- Create: no new production modules.

### Task 1: Parse and dry-run `moat dispatch-next`

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`

- [x] **Step 1: Write the failing dry-run test**

Add this test near the other moat CLI routing tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn moat_dispatch_next_dry_run_prints_first_ready_task_without_mutating_history() {
    let history_path = unique_history_path("dispatch-next-dry-run");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed dispatch-next history");
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));
    make_workflow_audit_spec_task_ready(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path_arg,
            "--dry-run",
        ])
        .output()
        .expect("failed to dry-run dispatch-next");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("moat dispatch next\n"));
    assert!(stdout.contains("dry_run=true\n"));
    assert!(stdout.contains("claimed=false\n"));
    assert!(stdout.contains("node_id=spec-workflow-audit\n"));
    assert!(stdout.contains("role=planner\n"));
    assert!(stdout.contains("kind=spec_planning\n"));
    assert!(stdout.contains("title=Create spec for workflow audit\n"));
    assert!(stdout.contains("dependencies=<none>\n"));
    assert!(stdout.contains("spec_ref=moat-spec/workflow-audit\n"));
    assert!(stdout.contains(&format!(
        "complete_command=mdid-cli moat complete-task --history-path {} --node-id spec-workflow-audit --artifact-ref <artifact-ref> --artifact-summary <artifact-summary>\n",
        history_path.display()
    )));

    let ready_after = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "ready-tasks", "--history-path", history_path_arg])
        .output()
        .expect("failed to inspect ready tasks after dry-run dispatch");
    assert!(ready_after.status.success(), "{}", String::from_utf8_lossy(&ready_after.stderr));
    assert!(String::from_utf8_lossy(&ready_after.stdout)
        .contains("ready_task=planner|spec_planning|spec-workflow-audit|Create spec for workflow audit|moat-spec/workflow-audit\n"));

    cleanup_history_path(&history_path);
}
```

- [x] **Step 2: Run test to verify it fails**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_dispatch_next_dry_run_prints_first_ready_task_without_mutating_history -- --nocapture`

Expected: FAIL with `unknown moat command` or `unknown command` for `dispatch-next`.

- [x] **Step 3: Write minimal parser and dry-run implementation**

In `crates/mdid-cli/src/main.rs`, add a new command variant and parser matching the existing `ready-tasks` parser style. Print exactly these lines for the selected ready task:

```text
moat dispatch next
dry_run=true
claimed=false
round_id=<round-id>
node_id=<node-id>
role=<planner|coder|reviewer>
kind=<kind>
title=<title>
dependencies=<none>|<comma-joined-dependencies>
spec_ref=<none>|<spec-ref>
complete_command=mdid-cli moat complete-task --history-path <path> --node-id <node-id> --artifact-ref <artifact-ref> --artifact-summary <artifact-summary>
```

Use `LocalMoatHistoryStore::open_existing`, latest round by default, `task_graph.ready_node_ids()`, and persisted task-node order. For Task 1, only `--history-path` and `--dry-run` must work.

- [x] **Step 4: Run test to verify it passes**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_dispatch_next_dry_run_prints_first_ready_task_without_mutating_history -- --nocapture`

Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs
git commit -m "feat: add dry-run moat task dispatch envelope"
```

### Task 2: Claim exactly one dispatched task and support filters

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`

- [x] **Step 1: Write failing mutating dispatch and filter tests**

Add tests that:

```rust
#[test]
fn moat_dispatch_next_claims_selected_ready_task() {
    let history_path = unique_history_path("dispatch-next-claim");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed dispatch-next claim history");
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));
    make_workflow_audit_spec_task_ready(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "dispatch-next", "--history-path", history_path_arg])
        .output()
        .expect("failed to dispatch and claim next task");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("dry_run=false\n"));
    assert!(stdout.contains("claimed=true\n"));
    assert!(stdout.contains("previous_state=ready\n"));
    assert!(stdout.contains("new_state=in_progress\n"));
    assert!(stdout.contains("node_id=spec-workflow-audit\n"));

    let ready_after = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "ready-tasks", "--history-path", history_path_arg])
        .output()
        .expect("failed to inspect ready tasks after dispatch claim");
    assert!(ready_after.status.success(), "{}", String::from_utf8_lossy(&ready_after.stderr));
    assert!(String::from_utf8_lossy(&ready_after.stdout).contains("ready_task_entries=0\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn moat_dispatch_next_filters_by_role_and_kind() {
    let history_path = unique_history_path("dispatch-next-filter");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed dispatch-next filter history");
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));
    make_workflow_audit_spec_task_ready(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path_arg,
            "--role",
            "planner",
            "--kind",
            "spec_planning",
            "--dry-run",
        ])
        .output()
        .expect("failed to dispatch next task with role/kind filters");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8_lossy(&output.stdout).contains("node_id=spec-workflow-audit\n"));

    cleanup_history_path(&history_path);
}
```

- [x] **Step 2: Run tests to verify they fail**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_dispatch_next -- --nocapture`

Expected: at least the mutating/filter tests FAIL because non-dry-run and filters are not complete yet.

- [x] **Step 3: Implement claim and filter behavior**

Extend `MoatDispatchNextCommand` with optional `role`, `kind`, and `dry_run`. Reuse `parse_moat_assignments_role_filter` and `parse_moat_assignments_kind_filter`. For non-dry-run, reload current history before mutation, verify the selected node is still `ready`, set it to `MoatTaskNodeState::InProgress`, persist with existing history store mutation pattern used by `claim-task`, and print the envelope with:

```text
dry_run=false
claimed=true
previous_state=ready
new_state=in_progress
```

- [x] **Step 4: Run tests to verify they pass**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_dispatch_next -- --nocapture`

Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs
git commit -m "feat: claim dispatched moat task"
```

### Task 3: Error handling, usage, and spec truth-sync

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
- Modify: `docs/superpowers/plans/2026-04-27-med-de-id-moat-dispatch-next-agent-task.md`

- [x] **Step 1: Write failing error tests**

Add tests for:

```rust
#[test]
fn moat_dispatch_next_fails_when_no_ready_task_matches() {
    let history_path = unique_history_path("dispatch-next-no-match");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed dispatch-next no-match history");
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "dispatch-next", "--history-path", history_path_arg, "--role", "coder"])
        .output()
        .expect("failed to run dispatch-next no-match case");

    assert!(!output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stderr), "no ready moat task matched dispatch filters\n");

    cleanup_history_path(&history_path);
}

#[test]
fn moat_dispatch_next_rejects_duplicate_dry_run_flag() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "dispatch-next", "--history-path", "history.json", "--dry-run", "--dry-run"])
        .output()
        .expect("failed to run dispatch-next with duplicate dry-run");

    assert!(!output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stderr), format!("duplicate flag: --dry-run\n{USAGE}\n"));
}
```

- [x] **Step 2: Run tests to verify they fail**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_dispatch_next -- --nocapture`

Expected: FAIL until errors/usage are complete.

- [x] **Step 3: Implement error handling and usage text**

Update `usage()` and test `USAGE` to include:

```text
| moat dispatch-next --history-path PATH [--round-id ROUND_ID] [--role planner|coder|reviewer] [--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation] [--dry-run]
```

Return `no ready moat task matched dispatch filters` when selection is empty. Reject duplicate `--dry-run`, duplicate filter flags, missing required values, unknown flags, unknown role, and unknown kind using existing parser helper conventions.

- [x] **Step 4: Update spec status**

In `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`, update the shipped foundation list to include bounded `dispatch-next` and keep full autonomous process execution as future work.

- [x] **Step 5: Run focused and broader verification**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_dispatch_next -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_ -- --nocapture
```

Expected: PASS.

- [x] **Step 6: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-27-med-de-id-moat-dispatch-next-agent-task.md
git commit -m "feat: harden moat dispatch-next command"
```

## Self-Review

- Spec coverage: The plan covers bounded one-task external-controller dispatch, dry-run read-only behavior, non-dry-run claim mutation, role/kind filters, deterministic output, missing-match behavior, usage, and spec truth-sync. Full daemon/agent process execution remains explicitly out of scope.
- Placeholder scan: No TBD/TODO/implement-later placeholders remain.
- Type consistency: Command and test names consistently use `dispatch-next`; states use existing `ready` and `in_progress`; task kind/role wire values match existing parser values.
