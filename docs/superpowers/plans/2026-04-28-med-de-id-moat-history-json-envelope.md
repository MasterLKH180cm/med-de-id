# Moat History JSON Envelope Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `--format json` to `mdid-cli moat history` so external autonomous controllers can consume deterministic round summaries without scraping text.

**Architecture:** Extend the existing `MoatHistoryCommand` with `MoatOutputFormat`, parse `--format text|json` consistently with other moat inspection commands, and branch rendering inside `run_moat_history`. Preserve text output as the default and keep the command read-only against existing history files.

**Tech Stack:** Rust 2021, Cargo integration tests in `crates/mdid-cli/tests/moat_cli.rs`, CLI implementation in `crates/mdid-cli/src/main.rs`, markdown docs/specs.

---

## File Structure

- Modify `crates/mdid-cli/src/main.rs`: add `format: MoatOutputFormat` to `MoatHistoryCommand`, parse `--format`, include it in usage, and emit deterministic pretty JSON envelopes from `run_moat_history`.
- Modify `crates/mdid-cli/tests/moat_cli.rs`: add integration tests for history JSON envelope output and default text compatibility.
- Modify `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`: truth-sync shipped CLI surface for `moat history --format text|json`.
- Modify `README.md` and `AGENTS.md` if needed: keep docs aligned with the landed read-only history behavior.

### Task 1: Moat history JSON envelope

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
- Modify: `README.md`
- Modify: `AGENTS.md`

- [ ] **Step 1: Write the failing JSON envelope test**

Add an integration test near existing `moat history` tests:

```rust
#[test]
fn cli_emits_moat_history_json_envelope() {
    let history_file = TempFile::new("moat-history-json", "json");
    let history_path_arg = history_file.path_arg();

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history for json history");
    assert_success(&seed);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "history",
            "--history-path",
            history_path_arg,
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run mdid-cli moat history as json");
    assert_success(&output);

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("history stdout should be json");
    assert_eq!(value["type"], "moat_history");
    assert_eq!(value["history_path"], history_path_arg);
    assert_eq!(value["history_rounds"], 1);
    assert_eq!(value["summary"]["total_rounds"], 1);
    assert_eq!(value["rounds"].as_array().expect("rounds should be array").len(), 1);
    assert_eq!(value["rounds"][0]["decision"], "Continue");
    assert_eq!(value["rounds"][0]["moat_score_after"], 90);
    assert_eq!(value["rounds"][0]["tests_passed"], true);
}
```

- [ ] **Step 2: Run the focused test and verify RED**

Run: `CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/tmp/med-de-id-moat-loop-target cargo test -p mdid-cli --test moat_cli cli_emits_moat_history_json_envelope -- --nocapture`

Expected: FAIL because `moat history --format` is currently an unknown flag.

- [ ] **Step 3: Implement parsing and JSON rendering**

In `crates/mdid-cli/src/main.rs`, add `format: MoatOutputFormat` to `MoatHistoryCommand`, parse `--format text|json` in `parse_moat_history_command`, default to text, and update `run_moat_history` so JSON mode prints only a pretty JSON object:

```json
{
  "type": "moat_history",
  "history_path": "PATH",
  "summary": {
    "total_rounds": 1,
    "latest_round_id": "...",
    "latest_decision": "Continue",
    "latest_implemented_specs": ["moat-spec/workflow-audit"],
    "latest_moat_score_after": 90,
    "best_moat_score_after": 90
  },
  "history_rounds": 1,
  "rounds": [
    {
      "round_id": "...",
      "decision": "Continue",
      "moat_score_after": 90,
      "stop_reason": null,
      "tests_passed": true,
      "implemented_specs": ["moat-spec/workflow-audit"]
    }
  ]
}
```

For filtered no-match output, emit the same envelope with `summary.total_rounds = 0`, nullable latest fields, `history_rounds = 0`, and `rounds = []`.

- [ ] **Step 4: Run focused JSON test and verify GREEN**

Run: `CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/tmp/med-de-id-moat-loop-target cargo test -p mdid-cli --test moat_cli cli_emits_moat_history_json_envelope -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Add compatibility and validation tests**

Add tests proving text remains the default and invalid format values fail:

```rust
#[test]
fn cli_moat_history_defaults_to_text_after_json_format_addition() {
    let history_file = TempFile::new("moat-history-text-default", "json");
    let history_path_arg = history_file.path_arg();
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history for text default");
    assert_success(&seed);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "history", "--history-path", history_path_arg])
        .output()
        .expect("failed to run mdid-cli moat history default text");
    assert_success(&output);
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.starts_with("moat history summary\n"));
    assert!(serde_json::from_str::<serde_json::Value>(&stdout).is_err());
}

#[test]
fn cli_rejects_unknown_moat_history_format() {
    let history_file = TempFile::new("moat-history-bad-format", "json");
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "history",
            "--history-path",
            history_file.path_arg(),
            "--format",
            "xml",
        ])
        .output()
        .expect("failed to run mdid-cli moat history with bad format");
    assert_failure(&output);
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert_eq!(stderr, "error: unknown output format for --format: xml\n");
}
```

- [ ] **Step 6: Run focused compatibility tests**

Run: `CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/tmp/med-de-id-moat-loop-target cargo test -p mdid-cli --test moat_cli moat_history -- --nocapture`

Expected: PASS.

- [ ] **Step 7: Truth-sync docs**

Update the moat-loop spec shipped foundation bullet for `mdid-cli moat history` to include `[--format text|json]`, default text compatibility, deterministic JSON envelope fields, read-only behavior, and no scheduling/agents/crawling/PR/cron side effects. Update README/AGENTS only if they mention moat history surfaces.

- [ ] **Step 8: Run broader verification**

Run: `CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/tmp/med-de-id-moat-loop-target cargo test -p mdid-cli --test moat_cli -- --nocapture`

Expected: PASS.

- [ ] **Step 9: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md README.md AGENTS.md docs/superpowers/plans/2026-04-28-med-de-id-moat-history-json-envelope.md
git commit -m "feat(cli): emit moat history json envelope"
```

## Self-Review

- Spec coverage: The plan adds a parseable controller-facing history envelope without changing default text or read-only semantics.
- Placeholder scan: No TBD/TODO/fill-later placeholders remain.
- Type consistency: The plan consistently uses `MoatOutputFormat`, `format`, `history_rounds`, and `rounds` field names.
