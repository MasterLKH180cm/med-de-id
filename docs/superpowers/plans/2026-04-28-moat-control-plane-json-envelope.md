# Moat Control Plane JSON Envelope Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `mdid-cli moat control-plane --format json` so external controllers can consume the bounded local orchestration snapshot without scraping text.

**Architecture:** Keep the existing control-plane runtime path and text output intact, add an output-format flag to the CLI command, and serialize the already-computed snapshot into a deterministic JSON envelope. The command remains local-only/read-only unless the existing `moat round --history-path` path is used elsewhere; it must not launch agents, schedule background work, crawl data, open PRs, create cron jobs, or write artifact files.

**Tech Stack:** Rust workspace, `mdid-cli`, `serde_json`, Cargo integration tests, markdown docs.

---

## File Structure

- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add integration coverage for control-plane JSON output, default text preservation, and format validation.
- Modify: `crates/mdid-cli/src/main.rs`
  - Add `format: MoatOutputFormat` to `MoatControlPlaneCommand`.
  - Parse `--format text|json` in `parse_moat_control_plane_command`.
  - Render deterministic JSON from `run_moat_control_plane` after computing the existing snapshot.
  - Update usage text.
- Modify: `README.md`
  - Document `moat control-plane --format json` and current local-controller surfaces.
- Modify: `AGENTS.md`
  - Document the JSON control-plane envelope and local-only constraints.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Truth-sync the spec to say local external-controller coordination surfaces are landed while autonomous daemon/process execution remains future.

### Task 1: Add `moat control-plane --format json`

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `README.md`
- Modify: `AGENTS.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`

- [ ] **Step 1: Write the failing JSON integration test**

Add this test near existing `moat_control_plane_*` tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn moat_control_plane_json_emits_deterministic_controller_snapshot() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "control-plane", "--format", "json"])
        .output()
        .expect("run moat control-plane json");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("valid json envelope");

    assert_eq!(value["type"], "moat_control_plane");
    assert_eq!(value["history_path"], serde_json::Value::Null);
    assert_eq!(value["source"], "sample");
    assert!(value["round_id"].as_str().expect("round_id string").starts_with("moat-round-"));
    assert!(value["score"].as_u64().expect("score number") > 0);
    assert!(value["improvement_delta"].is_number());
    assert!(value["can_continue"].is_boolean());
    assert!(value["ready_tasks"].as_array().expect("ready tasks array").len() > 0);
    assert!(value["assignments"].as_array().expect("assignments array").len() > 0);
    assert!(value["task_states"].as_array().expect("task states array").len() > 0);
    assert!(value["decision_summary"].as_str().expect("decision summary string").len() > 0);
    assert!(value["constraints"].as_array().expect("constraints array").iter().any(|item| item == "local_only"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_control_plane_json_emits_deterministic_controller_snapshot -- --nocapture`

Expected: FAIL because `moat control-plane` rejects or ignores `--format json`, or stdout is not valid JSON.

- [ ] **Step 3: Write validation/default text tests**

Add these tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn moat_control_plane_default_text_output_is_preserved() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "control-plane"])
        .output()
        .expect("run moat control-plane text");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("moat control plane"));
    assert!(stdout.contains("ready_tasks="));
    assert!(stdout.contains("task_states="));
}

#[test]
fn moat_control_plane_rejects_unknown_format() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "control-plane", "--format", "yaml"])
        .output()
        .expect("run moat control-plane unknown format");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("unknown moat control-plane format: yaml"));
}
```

- [ ] **Step 4: Run validation/default tests to verify current behavior**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_control_plane_ -- --nocapture`

Expected: new JSON and unknown-format tests FAIL; default text test PASS.

- [ ] **Step 5: Implement minimal CLI parsing and rendering**

In `crates/mdid-cli/src/main.rs`:

- Add `format: MoatOutputFormat` to `MoatControlPlaneCommand` with default `MoatOutputFormat::Text`.
- Update `parse_moat_control_plane_command` to accept `--format text|json`, reject duplicates, and reject unknown values with `unknown moat control-plane format: VALUE`.
- After computing the existing snapshot in `run_moat_control_plane`, branch on `command.format`:
  - `Text`: call the existing text renderer unchanged.
  - `Json`: print pretty JSON with fields `type`, `history_path`, `source`, `round_id`, `score`, `improvement_delta`, `can_continue`, `decision_summary`, `ready_tasks`, `assignments`, `task_states`, and `constraints: ["local_only", "read_only", "no_agent_launch", "no_daemon", "no_pr_creation", "no_cron_creation"]`.

- [ ] **Step 6: Run targeted GREEN tests**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_control_plane_ -- --nocapture`

Expected: PASS.

- [ ] **Step 7: Update docs**

Update `README.md`, `AGENTS.md`, and `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md` to describe `moat control-plane --format json`, local-only/read-only semantics, and the fact that local external-controller coordination surfaces are landed while full autonomous daemon/process execution remains future.

- [ ] **Step 8: Run broader verification**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli moat_ --test moat_cli --no-fail-fast`

Expected: PASS.

- [ ] **Step 9: Commit**

Run:

```bash
git add crates/mdid-cli/tests/moat_cli.rs crates/mdid-cli/src/main.rs README.md AGENTS.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-28-moat-control-plane-json-envelope.md
git commit -m "feat(cli): emit moat control-plane json envelope"
```
