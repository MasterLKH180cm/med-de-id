# med-de-id Moat Ready Tasks Node ID Filter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for completed tracking.

**Goal:** Add an exact `--node-id NODE_ID` filter to `mdid-cli moat ready-tasks` so autonomous controllers can route or claim a specific ready task without scanning unrelated rows.

**Architecture:** Keep the behavior bounded and local-first: parse the optional filter in the CLI, apply it after deriving ready nodes and before limit truncation, and preserve existing read-only semantics. The filter exact-matches persisted task node IDs and never mutates history or launches agents.

**Tech Stack:** Rust workspace, Cargo integration tests, `mdid-cli`, `mdid-runtime::moat_history`.

---

## File Structure

- Modified `crates/mdid-cli/src/main.rs`: extended `MoatReadyTasksCommand`, parsed `--node-id`, included it in ready-task filtering, and updated usage text.
- Modified `crates/mdid-cli/tests/moat_cli.rs`: added CLI integration tests for exact matching and no-match success output.
- Modified `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`: truth-synced shipped foundation description for the new read-only filter and documented the committed row order.

### Task 1: Ready Tasks Exact Node ID Filter

**Files:**
- Modified: `crates/mdid-cli/tests/moat_cli.rs`
- Modified: `crates/mdid-cli/src/main.rs`
- Modified: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`

- [x] **Step 1: Write the failing exact-match test**

Added `cli_filters_ready_tasks_by_exact_node_id` near the existing `ready-tasks` CLI tests. The committed test seeds a deterministic moat round, adjusts the persisted `lockin_analysis` task to `ready`, then filters by exact persisted node ID:

```rust
.args([
    "moat",
    "ready-tasks",
    "--history-path",
    history_path_arg,
    "--node-id",
    "lockin_analysis",
])
```

The fixture intentionally uses `node_id=lockin_analysis` with `kind=lock_in_analysis` so a mistaken kind-based filter would not satisfy the test. Expected output uses the implementation row order:

```text
moat ready tasks
ready_task_entries=1
ready_task=planner|lock_in_analysis|lockin_analysis|Lock-In Analysis|<none>
```

- [x] **Step 2: Write the failing no-match test**

Added `cli_ready_tasks_node_id_filter_succeeds_with_no_matches`, which filters by `--node-id missing-node` and asserts a successful empty result:

```text
moat ready tasks
ready_task_entries=0
```

- [x] **Step 3: Run tests to verify RED**

Ran the node-id-focused CLI tests before implementation during the feature slice; they failed until parsing/filtering was added.

- [x] **Step 4: Implement minimal CLI parsing and filtering**

Implemented `node_id: Option<String>` on `MoatReadyTasksCommand`, parsed `--node-id NODE_ID` with duplicate/missing-value errors, updated the `USAGE` synopsis, and applied exact persisted node ID matching before `--limit` truncation.

- [x] **Step 5: Truth-sync spec**

Updated `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md` so `mdid-cli moat ready-tasks` documents `[--node-id NODE_ID]` as a read-only exact persisted node ID filter that combines conjunctively with round/role/kind before `--limit`. The spec now matches the committed output row order: `ready_task=<role>|<kind>|<node_id>|<title>|<spec_ref>`.

- [x] **Step 6: Run targeted tests to verify GREEN**

Verified the strengthened exact-node-id test with:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_filters_ready_tasks_by_exact_node_id -- --nocapture
```

- [x] **Step 7: Run broader relevant verification**

Verified the ready-tasks integration coverage with:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli ready_tasks -- --nocapture
```

- [x] **Step 8: Commit**

Committed the quality-review fix with:

```bash
git commit -m "fix: tighten moat ready task node filter docs"
```

## Self-Review

- Spec coverage: The plan implements and documents a bounded read-only routing filter that advances the autonomous control-plane handoff path.
- Placeholder scan: No unresolved placeholders remain.
- Type consistency: `node_id` matches persisted field naming and the CLI flag naming used by related moat commands.
- Row-order consistency: Ready-task examples use `ready_task=<role>|<kind>|<node_id>|<title>|<spec_ref>`, matching implementation and tests.
