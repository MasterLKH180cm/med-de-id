# Med De-ID Moat Claim Ready Task Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded `mdid-cli moat claim-task` control-plane action that atomically marks one persisted ready task as `in_progress` so multiple agents do not claim the same work.

**Architecture:** The runtime history store owns the safe persisted mutation for a selected round and task node. The CLI parses a small command, calls the runtime mutation, and prints machine-readable output; it does not launch agents, create rounds, schedule background work, or write code.

**Tech Stack:** Rust workspace, `mdid-runtime` local JSONL history store, `mdid-cli` integration tests, Cargo targeted tests with `CARGO_INCREMENTAL=0`.

---

## File Structure

- Modify: `crates/mdid-runtime/src/moat_history.rs`
  - Add a typed error enum and `LocalMoatHistoryStore::claim_ready_task(round_id: Option<&str>, node_id: &str)` that loads history, selects latest or exact round, verifies the node exists and is `Ready`, changes it to `InProgress`, and rewrites the same history file.
- Modify: `crates/mdid-runtime/tests/moat_history.rs`
  - Add tests proving successful claim persists, unknown/non-ready nodes fail without mutation, and exact `--round-id` selection is honored.
- Modify: `crates/mdid-cli/src/main.rs`
  - Add `MoatClaimTaskCommand`, parser arm for `moat claim-task`, runner output, and usage text.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add end-to-end CLI tests: successful claim removes node from `ready-tasks` and task graph shows `in_progress`; failures are non-mutating.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Truth-sync that `ready-tasks` is shipped and `claim-task` provides bounded persisted coordination without agent execution.

### Task 1: Runtime persisted ready-task claim

**Files:**
- Modify: `crates/mdid-runtime/src/moat_history.rs`
- Test: `crates/mdid-runtime/tests/moat_history.rs`

- [ ] **Step 1: Write failing runtime tests**

Append tests that create a history file with a ready node, call `LocalMoatHistoryStore::claim_ready_task`, reload history, and assert the target node is `InProgress`. Add negative tests for unknown nodes and non-ready nodes.

- [ ] **Step 2: Run runtime tests to verify RED**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-runtime claim_ready_task -- --nocapture`
Expected: FAIL because `claim_ready_task` does not exist.

- [ ] **Step 3: Implement minimal runtime mutation**

Add `ClaimReadyTaskError` and `LocalMoatHistoryStore::claim_ready_task` in `crates/mdid-runtime/src/moat_history.rs`. The method must not create missing history files, must select latest when `round_id` is `None`, must preserve all unrelated entries and fields, and must rewrite via the existing JSONL format.

- [ ] **Step 4: Run runtime tests to verify GREEN**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-runtime claim_ready_task -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit runtime slice**

Run: `git add crates/mdid-runtime/src/moat_history.rs crates/mdid-runtime/tests/moat_history.rs && git commit -m "feat: claim persisted ready moat tasks"`

### Task 2: CLI `moat claim-task`

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/moat_cli.rs`

- [ ] **Step 1: Write failing CLI tests**

Add tests for `mdid-cli moat claim-task --history-path PATH --node-id review`, asserting stdout includes `moat task claimed`, `node_id=review`, `previous_state=ready`, `new_state=in_progress`, and a subsequent `ready-tasks` call no longer lists `review`. Add failure coverage for an already completed node.

- [ ] **Step 2: Run CLI tests to verify RED**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli claim_task -- --nocapture`
Expected: FAIL because `claim-task` is unknown.

- [ ] **Step 3: Implement minimal CLI parser and runner**

Add command parsing for `moat claim-task --history-path PATH --node-id NODE_ID [--round-id ROUND_ID]`, call `LocalMoatHistoryStore::claim_ready_task`, map errors to clear stderr strings, and update usage.

- [ ] **Step 4: Run CLI tests to verify GREEN**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli claim_task -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit CLI slice**

Run: `git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs && git commit -m "feat: add moat claim-task cli"`

### Task 3: Spec truth-sync and verification

**Files:**
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
- Modify: `docs/superpowers/plans/2026-04-27-med-de-id-moat-claim-ready-task.md`

- [ ] **Step 1: Update spec shipped-status section**

Document that `ready-tasks` lists claimable nodes and `claim-task` atomically moves one ready node to `in_progress` for external autonomous controllers; neither command launches agents or schedules background work.

- [ ] **Step 2: Run focused verification**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-runtime claim_ready_task -- --nocapture`
Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli claim_task -- --nocapture`
Expected: PASS.

- [ ] **Step 3: Run broader moat CLI smoke verification**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli ready_tasks -- --nocapture`
Expected: PASS.

- [ ] **Step 4: Commit docs**

Run: `git add docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-27-med-de-id-moat-claim-ready-task.md && git commit -m "docs: plan moat claim-task coordination slice"`

## Self-Review

- Spec coverage: covers bounded autonomous coordination after ready-task discovery; leaves full agent spawning, daemon scheduling, and live market crawling out of scope intentionally.
- Placeholder scan: no TBD/TODO/fill-in placeholders remain.
- Type consistency: command is consistently named `claim-task`; persisted node state changes from `Ready` to `InProgress`.
