# Med De-ID Moat Complete Task Next Ready Summary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `mdid-cli moat complete-task` immediately report the downstream ready tasks unlocked by the completion mutation.

**Architecture:** Keep the existing persisted coordination model: completing a task mutates only the selected persisted task graph node from `in_progress` to `completed`, then the CLI reloads the selected round from disk and derives `ready_node_ids()` from the updated task graph. The new output is a bounded routing summary (`next_ready_task_entries` plus `next_ready_task=...` rows) and does not launch agents, schedule work, create PRs, append rounds, or write artifacts.

**Tech Stack:** Rust workspace, `mdid-cli`, `mdid-runtime::moat_history::LocalMoatHistoryStore`, Cargo integration tests.

---

## File Structure

- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add CLI integration tests proving `complete-task` reports newly unlocked downstream ready tasks after completing a claimed upstream task.
- Modify: `crates/mdid-cli/src/main.rs`
  - After `complete_in_progress_task`, reload the existing history store, select the same latest-or-exact round, derive ready nodes, and print bounded `next_ready_task_entries` / `next_ready_task` routing rows.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Truth-sync the shipped `complete-task` surface to mention downstream ready-task summary output.
- Modify: `docs/superpowers/plans/2026-04-27-med-de-id-moat-complete-task-next-ready-summary.md`
  - Record implementation notes and verification results.

### Task 1: Complete-Task Downstream Ready Summary

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
- Modify: `docs/superpowers/plans/2026-04-27-med-de-id-moat-complete-task-next-ready-summary.md`

- [x] **Step 1: Write failing CLI test**

Added `cli_complete_task_reports_newly_ready_downstream_tasks` to `crates/mdid-cli/tests/moat_cli.rs`. The test creates a persisted bounded round, claims `strategy_generation` in a stop-path scenario where `spec_planning` is blocked only by that node, completes it, and asserts:

```rust
assert!(stdout.contains("next_ready_task_entries=1\n"));
assert!(stdout.contains("next_ready_task=planner|spec_planning|Spec Planning|spec_planning|docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md\n"));
```

Also updated the existing `cli_completes_claimed_moat_task` assertion to include the newly reported `evaluation` ready row after completing `review`.

- [x] **Step 2: Run test to verify RED**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_complete_task_reports_newly_ready_downstream_tasks -- --nocapture
```

Result: FAIL as expected because `complete-task` did not yet print `next_ready_task_entries` or `next_ready_task` rows.

- [x] **Step 3: Implement minimal CLI output**

Updated `run_moat_complete_task` in `crates/mdid-cli/src/main.rs` so after successfully persisting completion it:

```rust
let updated_store = LocalMoatHistoryStore::open_existing(&command.history_path)
    .map_err(|error| format!("failed to reload moat history store: {error}"))?;
let updated_entry = updated_store
    .entries()
    .iter()
    .find(|entry| entry.report.summary.round_id.to_string() == selected_round_id)
    .ok_or_else(|| format!("moat round not found after completion: {selected_round_id}"))?;
let ready_ids = updated_entry.report.control_plane.task_graph.ready_node_ids();
let next_ready_nodes = updated_entry
    .report
    .control_plane
    .task_graph
    .nodes
    .iter()
    .filter(|node| ready_ids.iter().any(|ready_id| ready_id == &node.node_id))
    .collect::<Vec<_>>();

println!("next_ready_task_entries={}", next_ready_nodes.len());
for node in next_ready_nodes {
    println!(
        "next_ready_task={}|{}|{}|{}|{}",
        format_agent_role(node.role),
        escape_assignment_output_field(&node.node_id),
        escape_assignment_output_field(&node.title),
        format_moat_task_kind(node.kind),
        node.spec_ref
            .as_deref()
            .map(escape_assignment_output_field)
            .unwrap_or_else(|| "<none>".to_string())
    );
}
```

- [x] **Step 4: Run targeted tests to verify GREEN**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_complete_task_reports_newly_ready_downstream_tasks -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_completes_claimed_moat_task -- --nocapture
```

Result: PASS for both targeted tests.

- [x] **Step 5: Run relevant broader verification**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli complete_task -- --nocapture
```

Result: PASS; 2 tests passed, 0 failed.

- [x] **Step 6: Truth-sync docs**

Updated `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md` so the shipped `complete-task` surface documents `next_ready_task_entries` and `next_ready_task=<role>|<node_id>|<title>|<kind>|<spec_ref>` output and reiterates that it does not launch agents, schedule work, append rounds, open PRs, create cron jobs, crawl data, or write artifacts.

- [x] **Step 7: Commit slice**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-27-med-de-id-moat-complete-task-next-ready-summary.md
git commit -m "feat: report next ready moat tasks on completion"
```
