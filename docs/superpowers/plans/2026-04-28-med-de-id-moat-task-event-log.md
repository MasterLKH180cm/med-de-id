# Moat Task Event Log Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a persisted read-only task event log so the moat-loop external controller can audit claim/heartbeat/reap/complete/release/block/unblock lifecycle transitions end-to-end.

**Architecture:** Extend the existing local-first history JSON contract with append-only `MoatTaskEvent` records stored on each persisted task graph. Mutation commands append deterministic event rows when they change task state or lease metadata. Add `mdid-cli moat task-events` as a bounded inspection surface with round/node/kind/agent/action filters; it must never launch agents, schedule jobs, append rounds, or mutate history.

**Tech Stack:** Rust workspace; `mdid-domain` serde models; `mdid-runtime::moat_history::LocalMoatHistoryStore`; `mdid-cli` argument parser; integration tests in `crates/mdid-cli/tests/moat_cli.rs`; targeted `cargo test` with `CARGO_INCREMENTAL=0`.

---

## File Structure

- Modify `crates/mdid-domain/src/lib.rs`: add `MoatTaskEventAction`, `MoatTaskEvent`, and `events: Vec<MoatTaskEvent>` on `MoatTaskGraph` with serde default/backward compatibility.
- Modify `crates/mdid-domain/tests/moat_agent_memory.rs`: add tests for stable event action wire values and legacy graphs deserializing with empty events.
- Modify `crates/mdid-runtime/src/moat_history.rs`: append task lifecycle events in existing mutation methods that claim, complete, release, block, unblock, heartbeat, and reap tasks.
- Modify `crates/mdid-runtime/tests/moat_history.rs`: add runtime-level persistence tests proving events are appended and stale reap records release events.
- Modify `crates/mdid-cli/src/main.rs`: parse and run `moat task-events --history-path PATH [--round-id ROUND_ID] [--node-id NODE_ID] [--action claim|heartbeat|reap|complete|release|block|unblock] [--agent-id AGENT_ID] [--contains TEXT] [--limit N]`.
- Modify `crates/mdid-cli/tests/moat_cli.rs`: add CLI tests for mutation-generated events and filters.
- Modify `README.md`, `AGENTS.md`, and `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`: truth-sync the new audit surface and clarify it is local read-only inspection, not an agent launcher.

### Task 1: Domain event model

**Files:**
- Modify: `crates/mdid-domain/src/lib.rs`
- Test: `crates/mdid-domain/tests/moat_agent_memory.rs`

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn task_event_action_wire_values_are_stable() {
    assert_eq!(serde_json::to_string(&MoatTaskEventAction::Claim).unwrap(), "\"claim\"");
    assert_eq!(serde_json::to_string(&MoatTaskEventAction::Heartbeat).unwrap(), "\"heartbeat\"");
    assert_eq!(serde_json::to_string(&MoatTaskEventAction::Reap).unwrap(), "\"reap\"");
    assert_eq!(serde_json::to_string(&MoatTaskEventAction::Complete).unwrap(), "\"complete\"");
    assert_eq!(serde_json::to_string(&MoatTaskEventAction::Release).unwrap(), "\"release\"");
    assert_eq!(serde_json::to_string(&MoatTaskEventAction::Block).unwrap(), "\"block\"");
    assert_eq!(serde_json::to_string(&MoatTaskEventAction::Unblock).unwrap(), "\"unblock\"");
}

#[test]
fn task_graph_deserializes_legacy_graphs_without_events_as_empty() {
    let graph: MoatTaskGraph = serde_json::from_str(r#"{
        "round_id": "00000000-0000-0000-0000-000000000000",
        "nodes": []
    }"#).expect("legacy task graph should deserialize");

    assert!(graph.events.is_empty());
}
```

- [ ] **Step 2: Run test to verify RED**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-domain --test moat_agent_memory task_event -- --nocapture`
Expected: FAIL because `MoatTaskEventAction` and `MoatTaskGraph.events` do not exist.

- [ ] **Step 3: Add minimal domain types**

Add public serde types and `#[serde(default)] pub events: Vec<MoatTaskEvent>` to `MoatTaskGraph`. Define fields: `event_id: Uuid`, `round_id: Uuid`, `node_id: String`, `action: MoatTaskEventAction`, `agent_id: Option<String>`, `recorded_at: DateTime<Utc>`, `summary: String`.

- [ ] **Step 4: Run test to verify GREEN**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-domain --test moat_agent_memory task_event -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit**

Run: `git add crates/mdid-domain/src/lib.rs crates/mdid-domain/tests/moat_agent_memory.rs && git commit -m "feat(domain): add moat task event model"`

### Task 2: Persist lifecycle events in runtime mutations

**Files:**
- Modify: `crates/mdid-runtime/src/moat_history.rs`
- Test: `crates/mdid-runtime/tests/moat_history.rs`

- [ ] **Step 1: Write failing tests**

Add tests that claim a ready task, heartbeat it, complete it with an artifact, and assert selected round `task_graph.events` contains ordered `claim`, `heartbeat`, `complete` events for that node. Add a stale-reap test that claims with an expired lease, calls reap, and asserts a `reap` event with the node ID and previous agent ID.

- [ ] **Step 2: Run test to verify RED**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-runtime --test moat_history task_event -- --nocapture`
Expected: FAIL because mutation methods do not append events.

- [ ] **Step 3: Append events in mutation methods**

Create a small internal helper that pushes `MoatTaskEvent` onto the selected graph after each successful lifecycle mutation. Use existing mutation timestamp values where available and `Uuid::new_v4()` for `event_id`. Summaries must be stable phrases like `task claimed`, `task heartbeat recorded`, `stale task reaped`, `task completed`, `task released`, `task blocked`, `task unblocked`.

- [ ] **Step 4: Run test to verify GREEN**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-runtime --test moat_history task_event -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit**

Run: `git add crates/mdid-runtime/src/moat_history.rs crates/mdid-runtime/tests/moat_history.rs && git commit -m "feat(runtime): persist moat task lifecycle events"`

### Task 3: CLI task-events inspection surface

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/moat_cli.rs`

- [ ] **Step 1: Write failing tests**

Add tests that seed history with `moat round --history-path`, claim/heartbeat/complete a node, then run `mdid-cli moat task-events --history-path PATH` and assert output contains `task_event_entries=3` and rows formatted as `task_event=<action>|<node_id>|<agent-or-<none>>|<summary>`. Add filter tests for `--node-id`, `--action heartbeat`, `--agent-id`, and `--limit 1`.

- [ ] **Step 2: Run test to verify RED**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli task_events -- --nocapture`
Expected: FAIL because `task-events` command does not exist.

- [ ] **Step 3: Implement parser and read-only renderer**

Add `MoatTaskEventsCommand`, `CliCommand::MoatTaskEvents`, parser branch, usage text, action parser, and `run_moat_task_events`. It must open history with `LocalMoatHistoryStore::open_existing`, select latest or exact `--round-id`, filter events conjunctively, apply `--limit` after filters, print `task_event_entries=N`, and render escaped pipe fields. Missing selected round prints `task_event_entries=0`.

- [ ] **Step 4: Run test to verify GREEN**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli task_events -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit**

Run: `git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs && git commit -m "feat(cli): inspect moat task lifecycle events"`

### Task 4: Documentation truth-sync and targeted verification

**Files:**
- Modify: `README.md`
- Modify: `AGENTS.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`

- [ ] **Step 1: Update docs with exact current behavior**

Document `mdid-cli moat task-events --history-path PATH [--round-id ROUND_ID] [--node-id NODE_ID] [--action claim|heartbeat|reap|complete|release|block|unblock] [--agent-id AGENT_ID] [--contains TEXT] [--limit N]`; say it reads existing history only and never runs agents, schedules work, opens PRs, appends rounds, or creates cron jobs.

- [ ] **Step 2: Run focused verification**

Run:
```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-domain --test moat_agent_memory task_event -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-runtime --test moat_history task_event -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli task_events -- --nocapture
```
Expected: all PASS.

- [ ] **Step 3: Run broader moat verification if disk allows**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_task -- --nocapture`
Expected: PASS.

- [ ] **Step 4: Commit docs**

Run: `git add README.md AGENTS.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-28-med-de-id-moat-task-event-log.md && git commit -m "docs: sync moat task event log surface"`
