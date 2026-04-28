# Moat Schedule Next JSON Envelope Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add deterministic `--format json` output to `mdid-cli moat schedule-next` so external controllers can consume one-shot scheduling results without parsing text.

**Architecture:** Keep `moat schedule-next` bounded and local: it may append at most one deterministic sample round when the continuation gate allows it, and otherwise leaves history unchanged. Extend the existing CLI parser with `MoatOutputFormat`, then branch output rendering inside `run_moat_schedule_next` without changing scheduling decisions.

**Tech Stack:** Rust workspace, `mdid-cli`, `mdid-runtime`, serde/serde_json, Cargo integration tests.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs` — add `format: MoatOutputFormat` to `CliCommand::MoatScheduleNext`, parse `--format text|json`, pass the format into `run_moat_schedule_next`, and print a deterministic JSON envelope for JSON mode.
- Modify: `crates/mdid-cli/tests/moat_cli.rs` — add RED integration tests for JSON output, default text preservation, and unknown-format rejection.
- Modify: `README.md` — document `moat schedule-next --format json`, envelope fields, and bounded local limitations.
- Modify: `AGENTS.md` — sync controller-facing schedule-next behavior for future agents.

### Task 1: Schedule-next JSON output

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/moat_cli.rs`
- Docs: `README.md`, `AGENTS.md`

- [ ] **Step 1: Write failing integration tests**

Add tests near existing `schedule-next` tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn cli_emits_moat_schedule_next_json_envelope_when_gate_allows_scheduling() {
    let history_path = unique_history_path("schedule-next-json-continue");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let round_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("moat round should run");
    assert!(round_output.status.success());

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "schedule-next",
            "--history-path",
            history_path_arg,
            "--format",
            "json",
        ])
        .output()
        .expect("moat schedule-next should run");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let envelope: Value = serde_json::from_slice(&output.stdout).expect("stdout should be json");
    assert_eq!(envelope["type"], "moat_schedule_next");
    assert_eq!(envelope["history_path"], history_path_arg);
    assert_eq!(envelope["scheduled"], true);
    assert_eq!(envelope["reason"], "latest round passed tests and improved moat score by 5");
    assert_eq!(envelope["scheduled_round_id"], "moat-round-001");
    assert_eq!(envelope["required_improvement_threshold"], 3);

    let store = LocalMoatHistoryStore::open_existing(history_path_arg).expect("history should exist");
    assert_eq!(store.entries().len(), 2);
}

#[test]
fn cli_emits_moat_schedule_next_json_envelope_when_gate_blocks_scheduling() {
    let history_path = unique_history_path("schedule-next-json-stop");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let round_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--history-path",
            history_path_arg,
            "--tests-passed",
            "false",
        ])
        .output()
        .expect("moat round should run");
    assert!(round_output.status.success());

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "schedule-next",
            "--history-path",
            history_path_arg,
            "--format",
            "json",
        ])
        .output()
        .expect("moat schedule-next should run");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let envelope: Value = serde_json::from_slice(&output.stdout).expect("stdout should be json");
    assert_eq!(envelope["type"], "moat_schedule_next");
    assert_eq!(envelope["history_path"], history_path_arg);
    assert_eq!(envelope["scheduled"], false);
    assert_eq!(envelope["reason"], "latest round did not complete tests");
    assert!(envelope["scheduled_round_id"].is_null());
    assert_eq!(envelope["required_improvement_threshold"], 3);

    let store = LocalMoatHistoryStore::open_existing(history_path_arg).expect("history should exist");
    assert_eq!(store.entries().len(), 1);
}

#[test]
fn cli_moat_schedule_next_defaults_to_text_after_json_format_addition() {
    let history_path = unique_history_path("schedule-next-default-text");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let round_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("moat round should run");
    assert!(round_output.status.success());

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "schedule-next", "--history-path", history_path_arg])
        .output()
        .expect("moat schedule-next should run");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("moat schedule next\n"));
    assert!(stdout.contains("scheduled=true\n"));
    assert!(stdout.contains("scheduled_round_id=moat-round-001\n"));
}

#[test]
fn cli_rejects_unknown_moat_schedule_next_format() {
    let history_path = unique_history_path("schedule-next-unknown-format");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "schedule-next",
            "--history-path",
            history_path_arg,
            "--format",
            "yaml",
        ])
        .output()
        .expect("moat schedule-next should run");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "error: unknown output format for --format: yaml\n"
    );
}
```

- [ ] **Step 2: Run tests to verify RED**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli schedule_next -- --nocapture`
Expected: FAIL because `--format` is rejected by `moat schedule-next`.

- [ ] **Step 3: Implement minimal code**

Update `CliCommand::MoatScheduleNext` to include `format: MoatOutputFormat`, parse `--format` via `parse_moat_history_format`, pass it from `main`, and make `run_moat_schedule_next(history_path, improvement_threshold, format)` print this JSON envelope when `format == MoatOutputFormat::Json`:

```json
{
  "type": "moat_schedule_next",
  "history_path": "PATH",
  "scheduled": true,
  "reason": "latest round passed tests and improved moat score by 5",
  "scheduled_round_id": "moat-round-001",
  "required_improvement_threshold": 3
}
```

Use `serde_json::json!` and pretty deterministic output with `serde_json::to_string_pretty`.

- [ ] **Step 4: Run tests to verify GREEN**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli schedule_next -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Run broader package verification**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli`
Expected: PASS.

- [ ] **Step 6: Update docs**

Update `README.md` and `AGENTS.md` with exact `--format json` usage, envelope fields, and the statement that `schedule-next` remains one-shot/local and does not launch agents, crawl data, open PRs, create cron jobs, or run as a daemon.

- [ ] **Step 7: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs README.md AGENTS.md docs/superpowers/plans/2026-04-28-med-de-id-moat-schedule-next-json-envelope.md
git commit -m "feat(cli): emit schedule-next json envelope"
```

## Self-Review

- Spec coverage: plan adds controller-readable JSON for one-shot scheduling while preserving bounded local behavior and default text output.
- Placeholder scan: no TBD/TODO/fill-in placeholders remain.
- Type consistency: uses existing `MoatOutputFormat`, `LocalMoatHistoryStore`, and `Value` names already present in the CLI tests.
