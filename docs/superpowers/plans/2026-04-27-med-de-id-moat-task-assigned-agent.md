# Moat Task Assigned Agent Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Persist and inspect which autonomous worker claimed a moat task so multi-agent dispatch has durable task ownership instead of output-only attribution.

**Architecture:** Extend the domain task-node record with an optional `assigned_agent_id` field that defaults to `None` for existing history files. Runtime claim operations set or clear this field through a new agent-aware claim path; CLI dispatch/claim commands expose `--agent-id` and inspection commands print the persisted owner without launching agents.

**Tech Stack:** Rust workspace, serde JSON persistence, `mdid-domain`, `mdid-runtime`, `mdid-cli`, Cargo integration tests.

---

## File Structure

- Modify `crates/mdid-domain/src/lib.rs`: add `assigned_agent_id: Option<String>` to `MoatTaskNode` with `#[serde(default)]` so old history JSON remains readable.
- Modify `crates/mdid-runtime/src/moat_history.rs`: add `claim_ready_task_with_agent(round_id, node_id, agent_id)` and keep `claim_ready_task` as a compatibility wrapper.
- Modify `crates/mdid-runtime/tests/moat_history.rs`: add focused persistence tests for agent-aware claims and legacy deserialization.
- Modify `crates/mdid-cli/src/main.rs`: add `--agent-id` to `moat claim-task`, pass dispatch `--agent-id` into the runtime claim path, include `assigned_agent_id` in dispatch output and `moat task-graph` inspection output.
- Modify `crates/mdid-cli/tests/moat_cli.rs`: add CLI integration tests for persisted `assigned_agent_id` after dispatch and claim.
- Modify `README.md`: document that dispatch/claim can persist a local worker ID and that this is still bounded local orchestration, not automatic agent launching.
- Modify `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`: truth-sync command signatures and task graph output with persisted ownership metadata.

### Task 1: Runtime task ownership persistence

**Files:**
- Modify: `crates/mdid-domain/src/lib.rs:502-512`
- Modify: `crates/mdid-runtime/src/moat_history.rs:204-251`
- Test: `crates/mdid-runtime/tests/moat_history.rs`

- [ ] **Step 1: Write the failing runtime tests**

Add tests that call `claim_ready_task_with_agent(None, "implementation", Some("coder-7"))`, reopen the history store, and assert the claimed node has `state == MoatTaskNodeState::InProgress` and `assigned_agent_id.as_deref() == Some("coder-7")`. Add a second test that deserializes a minimal `MoatTaskNode` JSON object without `assigned_agent_id` and asserts the field defaults to `None`.

- [ ] **Step 2: Run test to verify RED**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-runtime claim_ready_task_with_agent_persists_assigned_agent_id -- --nocapture`
Expected: FAIL because `claim_ready_task_with_agent` and/or `assigned_agent_id` are not defined.

- [ ] **Step 3: Implement minimal runtime support**

Add `#[serde(default)] pub assigned_agent_id: Option<String>,` to `MoatTaskNode`. Implement `claim_ready_task_with_agent(&mut self, round_id: Option<&str>, node_id: &str, agent_id: Option<&str>) -> Result<(), ClaimReadyTaskError>` by sharing the existing claim logic and setting `node.assigned_agent_id = agent_id.map(str::to_string)` before persisting. Keep `claim_ready_task` delegating to `claim_ready_task_with_agent(round_id, node_id, None)`.

- [ ] **Step 4: Run runtime verification**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-runtime claim_ready_task -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit runtime slice**

Run: `git add crates/mdid-domain/src/lib.rs crates/mdid-runtime/src/moat_history.rs crates/mdid-runtime/tests/moat_history.rs && git commit -m "feat(runtime): persist moat task assigned agent"`

### Task 2: CLI ownership surfaces and docs

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`

- [ ] **Step 1: Write failing CLI tests**

Add one test where `mdid-cli moat dispatch-next --history-path <path> --agent-id coder-7` claims a ready task, then `mdid-cli moat task-graph --history-path <path>` prints `assigned_agent_id=coder-7` (or a documented task-graph row field carrying `coder-7`) for the claimed node. Add one test where `mdid-cli moat claim-task --history-path <path> --node-id implementation --agent-id planner-2` persists and prints `assigned_agent_id=planner-2`.

- [ ] **Step 2: Run tests to verify RED**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli dispatch_next_persists_assigned_agent_id claim_task_persists_assigned_agent_id -- --nocapture`
Expected: FAIL because claim-task does not accept/persist `--agent-id` and inspection output does not expose `assigned_agent_id`.

- [ ] **Step 3: Implement minimal CLI support**

Add `agent_id: Option<String>` to `MoatClaimTaskCommand`; parse `--agent-id` with duplicate/missing-value validation; pass `command.agent_id.as_deref()` to `claim_ready_task_with_agent`. Change dispatch non-dry-run claim to pass `command.agent_id.as_deref()`. Print `assigned_agent_id=<none>|<escaped-agent>` in claim-task, dispatch text/JSON, and `moat task-graph` output.

- [ ] **Step 4: Update README**

Document `--agent-id` for dispatch-next and claim-task. State clearly that it persists local task ownership only and does not spawn, schedule, or supervise external AI agents. Truth-sync the moat-loop design spec with the same command signatures and the `moat task-graph` assigned-agent field.

- [ ] **Step 5: Run CLI verification**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli dispatch_next_persists_assigned_agent_id claim_task_persists_assigned_agent_id -- --nocapture`
Expected: PASS.

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli`
Expected: PASS.

- [ ] **Step 6: Commit CLI/docs slice**

Run: `git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs README.md && git commit -m "feat(cli): persist moat assigned agent ids"`

## Self-Review

- Spec coverage: durable task ownership for a multi-agent moat loop is covered in runtime persistence, CLI dispatch/claim, inspection output, and README docs.
- Placeholder scan: no TBD/TODO/implement-later placeholders are present.
- Type consistency: `assigned_agent_id: Option<String>` is used consistently across domain, runtime, CLI output, and tests.
