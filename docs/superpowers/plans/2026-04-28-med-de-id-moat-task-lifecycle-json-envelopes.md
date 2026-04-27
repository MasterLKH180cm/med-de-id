# Moat Task Lifecycle JSON Envelopes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add deterministic JSON envelopes to explicit local moat task lifecycle lease/recovery commands so external controllers can run the core claim-heartbeat-reap flow without scraping text.

**Architecture:** Keep the existing local-first CLI command model and preserve text output as the default. Add `--format text|json` parsing to `claim-task`, `heartbeat-task`, and `reap-stale-tasks`, then render pretty deterministic JSON envelopes from the existing mutation results. Do not launch agents, create daemons, crawl data, open PRs, create cron jobs, or write artifacts.

**Tech Stack:** Rust workspace, `mdid-cli`, `mdid-runtime::moat_history`, serde_json integration tests, Cargo with `CARGO_INCREMENTAL=0` for bounded local builds.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Extend `MoatClaimTaskCommand`, `MoatHeartbeatTaskCommand`, and `MoatReapStaleTasksCommand` with `format: MoatOutputFormat`.
  - Parse `--format text|json` with missing, duplicate, and invalid validation.
  - Render JSON envelopes in `run_moat_claim_task`, `run_moat_heartbeat_task`, and `run_moat_reap_stale_tasks`.
  - Update usage text.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Update the mirrored `USAGE` constant.
  - Add CLI integration tests for JSON envelopes and parser errors.
- Modify: `README.md`
  - Document the JSON lifecycle envelopes and current landed command surface without overclaiming autonomous agent launch.
- Modify: `AGENTS.md`
  - Clarify lifecycle commands are local history-file coordination only.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Truth-sync implementation status for `claim-task`, `heartbeat-task`, and `reap-stale-tasks` JSON support.

### Task 1: Claim-task JSON envelope

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/moat_cli.rs`

- [ ] **Step 1: Write failing test**

Add an integration test named `claim_task_json_prints_parseable_envelope_and_claims_ready_node` near the existing claim-task tests. It must seed a history file with `moat round --strategy-candidates 0 --history-path PATH`, run `moat claim-task --history-path PATH --node-id market_scan --agent-id planner-json --lease-seconds 60 --format json`, parse stdout as JSON, and assert:

```rust
assert_eq!(json["type"], "moat_claim_task");
assert_eq!(json["history_path"], history_path_arg);
assert_eq!(json["node_id"], "market_scan");
assert_eq!(json["assigned_agent_id"], "planner-json");
assert_eq!(json["lease_seconds"], 60);
assert_eq!(json["previous_state"], "ready");
assert_eq!(json["new_state"], "in_progress");
assert!(json["lease_expires_at"].as_str().unwrap().contains('T'));
```

- [ ] **Step 2: Verify RED**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli claim_task_json_prints_parseable_envelope_and_claims_ready_node -- --nocapture`
Expected: FAIL because `claim-task` rejects unknown `--format` or does not emit JSON.

- [ ] **Step 3: Implement minimal code**

Add `format: MoatOutputFormat` to `MoatClaimTaskCommand`, parse `--format`, and in `run_moat_claim_task` preserve existing text output for `Text` while emitting:

```json
{
  "type": "moat_claim_task",
  "round_id": "<selected round id>",
  "history_path": "<path>",
  "node_id": "<node id>",
  "assigned_agent_id": "<agent or null>",
  "lease_seconds": 60,
  "lease_expires_at": "<rfc3339>",
  "previous_state": "ready",
  "new_state": "in_progress"
}
```

- [ ] **Step 4: Verify GREEN**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli claim_task_json_prints_parseable_envelope_and_claims_ready_node -- --nocapture`
Expected: PASS.

### Task 2: Heartbeat-task JSON envelope

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/moat_cli.rs`

- [ ] **Step 1: Write failing test**

Add `heartbeat_task_json_prints_parseable_envelope_and_extends_lease`. Seed history, claim `market_scan` for `planner-json`, then run `moat heartbeat-task --history-path PATH --node-id market_scan --agent-id planner-json --lease-seconds 120 --format json`. Parse JSON and assert:

```rust
assert_eq!(json["type"], "moat_heartbeat_task");
assert_eq!(json["history_path"], history_path_arg);
assert_eq!(json["node_id"], "market_scan");
assert_eq!(json["agent_id"], "planner-json");
assert_eq!(json["lease_seconds"], 120);
assert_eq!(json["state"], "in_progress");
assert!(json["lease_expires_at"].as_str().unwrap().contains('T'));
```

- [ ] **Step 2: Verify RED**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli heartbeat_task_json_prints_parseable_envelope_and_extends_lease -- --nocapture`
Expected: FAIL because `heartbeat-task` lacks `--format json`.

- [ ] **Step 3: Implement minimal code**

Add `format: MoatOutputFormat` to `MoatHeartbeatTaskCommand`, parse `--format`, and render the JSON fields above for `Json` while preserving existing text output for `Text`.

- [ ] **Step 4: Verify GREEN**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli heartbeat_task_json_prints_parseable_envelope_and_extends_lease -- --nocapture`
Expected: PASS.

### Task 3: Reap-stale-tasks JSON envelope and parser validation

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/moat_cli.rs`

- [ ] **Step 1: Write failing tests**

Add `reap_stale_tasks_json_prints_parseable_envelope` that seeds a history, claims `market_scan` with a one-second lease, runs `moat reap-stale-tasks --history-path PATH --now 2099-01-01T00:00:00Z --format json`, parses JSON, and asserts:

```rust
assert_eq!(json["type"], "moat_reap_stale_tasks");
assert_eq!(json["history_path"], history_path_arg);
assert_eq!(json["reaped_count"], 1);
assert_eq!(json["reaped_node_ids"].as_array().unwrap()[0], "market_scan");
```

Add parser tests for missing, duplicate, and invalid `--format` on one lifecycle command, expecting non-success and the existing validation-message style.

- [ ] **Step 2: Verify RED**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli reap_stale_tasks_json_prints_parseable_envelope -- --nocapture`
Expected: FAIL because `reap-stale-tasks` lacks `--format json`.

- [ ] **Step 3: Implement minimal code**

Add `format: MoatOutputFormat` to `MoatReapStaleTasksCommand`, parse `--format`, and render:

```json
{
  "type": "moat_reap_stale_tasks",
  "round_id": "<selected round id>",
  "history_path": "<path>",
  "reaped_count": 1,
  "reaped_node_ids": ["market_scan"]
}
```

- [ ] **Step 4: Verify GREEN and regressions**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli reap_stale_tasks_json_prints_parseable_envelope -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli lifecycle_format -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli moat_cli -- --nocapture
```

Expected: PASS.

### Task 4: Documentation truth-sync

**Files:**
- Modify: `README.md`
- Modify: `AGENTS.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`

- [ ] **Step 1: Update docs**

Document the local-only lifecycle JSON envelopes and command flags. Keep language bounded: these commands coordinate an external controller through a local history file; they do not launch agents, schedule background work, crawl data, open PRs, create cron jobs, or write artifacts.

- [ ] **Step 2: Verify docs mention landed commands**

Run:

```bash
grep -n "claim-task.*format" README.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md
grep -n "heartbeat-task.*format" README.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md
grep -n "reap-stale-tasks.*format" README.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md
```

Expected: matching lines in README and spec.

### Task 5: Final verification and commit

- [ ] **Step 1: Run targeted tests**

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli claim_task_json heartbeat_task_json reap_stale_tasks_json lifecycle_format -- --nocapture
```

Expected: PASS.

- [ ] **Step 2: Run broader CLI tests**

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli moat_cli -- --nocapture
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs README.md AGENTS.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-28-med-de-id-moat-task-lifecycle-json-envelopes.md
git commit -m "feat(cli): emit moat lifecycle json envelopes"
```
