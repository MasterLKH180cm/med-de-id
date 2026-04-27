# Moat Dispatch Next Spec Ref Filter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded `--spec-ref SPEC_REF` filter to `mdid-cli moat dispatch-next` so autonomous controllers can dispatch the next ready task for a specific implementation/spec handoff.

**Architecture:** Keep the feature entirely in the CLI coordination surface. Extend `MoatDispatchNextCommand` parsing and selection with an exact persisted `node.spec_ref` match, preserve existing role/kind/node-id behavior, and update operator docs/spec text.

**Tech Stack:** Rust 2021, Cargo integration tests in `crates/mdid-cli/tests/moat_cli.rs`, CLI implementation in `crates/mdid-cli/src/main.rs`, markdown specs under `docs/superpowers/specs/`.

---

## File Structure

- Modify `crates/mdid-cli/src/main.rs`: add `spec_ref: Option<String>` to `MoatDispatchNextCommand`, parse `--spec-ref`, include it in `select_dispatch_next_node`, and update the usage string.
- Modify `crates/mdid-cli/tests/moat_cli.rs`: add integration tests proving dispatch-next filters by exact spec ref and rejects unmatched spec refs without mutating history.
- Modify `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`: truth-sync the shipped status bullet for `dispatch-next` to mention `--spec-ref`.

### Task 1: Dispatch-next exact spec-ref routing filter

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`

- [x] **Step 1: Write the failing test for exact spec-ref dispatch selection**

Add this test near the existing `dispatch-next` tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn cli_dispatch_next_filters_ready_task_by_exact_spec_ref() {
    let history_path = unique_history_path("dispatch-next-spec-ref");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let round_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to persist moat round");
    assert!(
        round_output.status.success(),
        "expected round success, stderr was: {}",
        String::from_utf8_lossy(&round_output.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path_arg,
            "--spec-ref",
            "moat-spec/workflow-audit",
            "--dry-run",
        ])
        .output()
        .expect("failed to run mdid-cli moat dispatch-next with spec-ref filter");

    assert!(
        output.status.success(),
        "expected dispatch-next success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!(
            concat!(
                "moat dispatch next\n",
                "dry_run=true\n",
                "claimed=false\n",
                "round_id={round_id}\n",
                "node_id=task-implementation\n",
                "role=coder\n",
                "kind=implementation\n",
                "title=Implement workflow audit surface\n",
                "dependencies=task-spec-planning\n",
                "spec_ref=moat-spec/workflow-audit\n",
                "complete_command=mdid-cli moat complete-task --history-path '{history_path}' --round-id '{round_id}' --node-id 'task-implementation' --artifact-ref '<artifact-ref>' --artifact-summary '<artifact-summary>'\n",
            ),
            history_path = history_path.display()
        )
        .replace("{round_id}", &latest_round_id(&history_path))
    );

    cleanup_history_path(&history_path);
}
```

- [x] **Step 2: Run the focused failing test**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_dispatch_next_filters_ready_task_by_exact_spec_ref -- --nocapture
```

Expected: FAIL because `moat dispatch-next --spec-ref` is currently an unknown flag.

- [x] **Step 3: Add the minimal implementation**

In `crates/mdid-cli/src/main.rs`, change `MoatDispatchNextCommand` to include:

```rust
    spec_ref: Option<String>,
```

Initialize it in `parse_moat_dispatch_next_command`:

```rust
    let mut spec_ref = None;
```

Add this parser arm before `--dry-run`:

```rust
            "--spec-ref" => {
                let value = required_flag_value(args, index, "--spec-ref", false)?;
                if spec_ref.is_some() {
                    return Err(duplicate_flag_error("--spec-ref"));
                }
                spec_ref = Some(value.to_string());
                index += 2;
            }
```

Populate the command:

```rust
        spec_ref,
```

Extend `select_dispatch_next_node` predicate with exact raw persisted spec-ref matching:

```rust
                && command
                    .spec_ref
                    .as_deref()
                    .map(|spec_ref| node.spec_ref.as_deref() == Some(spec_ref))
                    .unwrap_or(true)
```

Update the usage string to show:

```text
moat dispatch-next --history-path PATH [--round-id ROUND_ID] [--role planner|coder|reviewer] [--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation] [--node-id NODE_ID] [--spec-ref SPEC_REF] [--dry-run]
```

- [x] **Step 4: Run the focused passing test**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_dispatch_next_filters_ready_task_by_exact_spec_ref -- --nocapture
```

Expected: PASS.

- [x] **Step 5: Write the failing non-mutation test for unmatched spec-ref**

Add this test near the first new test:

```rust
#[test]
fn cli_dispatch_next_unmatched_spec_ref_fails_without_claiming_task() {
    let history_path = unique_history_path("dispatch-next-spec-ref-miss");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let round_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to persist moat round");
    assert!(
        round_output.status.success(),
        "expected round success, stderr was: {}",
        String::from_utf8_lossy(&round_output.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path_arg,
            "--spec-ref",
            "moat-spec/missing",
        ])
        .output()
        .expect("failed to run mdid-cli moat dispatch-next with unmatched spec-ref filter");

    assert!(
        !output.status.success(),
        "expected dispatch-next failure, stdout was: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "error: no ready moat task matched dispatch filters\n"
    );

    let ready_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "ready-tasks",
            "--history-path",
            history_path_arg,
            "--spec-ref",
            "moat-spec/workflow-audit",
        ])
        .output()
        .expect("failed to inspect ready tasks after unmatched dispatch");
    assert!(
        ready_output.status.success(),
        "expected ready-tasks success, stderr was: {}",
        String::from_utf8_lossy(&ready_output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&ready_output.stdout)
            .contains("ready_task=coder|implementation|task-implementation|Implement workflow audit surface|moat-spec/workflow-audit\n"),
        "expected implementation task to remain ready, stdout was: {}",
        String::from_utf8_lossy(&ready_output.stdout)
    );

    cleanup_history_path(&history_path);
}
```

- [x] **Step 6: Run the second focused test to verify it passes with the same implementation**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_dispatch_next_unmatched_spec_ref_fails_without_claiming_task -- --nocapture
```

Expected: PASS.

- [x] **Step 7: Update spec/docs**

In `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`, update the `dispatch-next` shipped-status bullet to include the new flag and exact-match semantics:

```markdown
- `mdid-cli moat dispatch-next --history-path PATH [--round-id ROUND_ID] [--role planner|coder|reviewer] [--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation] [--node-id NODE_ID] [--spec-ref SPEC_REF] [--dry-run]` is a bounded one-task dispatch envelope for external Planner/Coder/Reviewer controllers. It opens only an existing history file, selects exactly one persisted ready node in task-graph order after optional round/role/kind/node-id/spec-ref filters, with `--node-id NODE_ID` exact-matching the persisted ready `node.node_id` before role/kind/spec-ref acceptance and `--spec-ref SPEC_REF` exact-matching raw persisted `node.spec_ref.as_deref()`, emits deterministic task metadata plus the matching `complete-task` handoff command, and never launches agents, daemons, PRs, crawlers, or cron jobs. `--dry-run` is read-only and reports `claimed=false`; without `--dry-run`, the command reloads current history and persists only the selected ready node's transition to `in_progress`, reporting `claimed=true`, `previous_state=ready`, and `new_state=in_progress`. If no ready node matches, it exits nonzero with `no ready moat task matched dispatch filters` and does not create or mutate history.
```

- [x] **Step 8: Run targeted and relevant broader tests**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli dispatch_next -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli -- --nocapture
```

Expected: PASS for all selected `mdid-cli` moat CLI integration tests.

- [x] **Step 9: Commit**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-27-med-de-id-moat-dispatch-next-spec-ref-filter.md
git commit -m "feat: filter moat dispatch by spec ref"
```
