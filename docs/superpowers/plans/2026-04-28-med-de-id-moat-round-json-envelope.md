# Moat Round JSON Envelope Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `mdid-cli moat round --format text|json` so external autonomous controllers can consume a deterministic JSON envelope from the bounded local moat round runner without scraping text output.

**Architecture:** Keep `mdid-cli moat round` as the bounded local-only one-shot runner. Text remains the default and must preserve the existing line-oriented output exactly for callers that do not pass `--format`. Add a `MoatOutputFormat` field to the round command, parse `--format text|json`, and render the already-computed `MoatRoundReport` as a pretty deterministic `moat_round` JSON envelope after optional history persistence. The command must not launch agents, run as a daemon, schedule background work, crawl data, create cron jobs, open PRs, or write artifacts; the only persistence remains the existing append to `--history-path PATH`.

**Tech Stack:** Rust workspace, `mdid-cli`, `serde_json`, Cargo integration tests, markdown docs/spec truth-sync.

---

## File Structure

- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add integration coverage for `moat round --format json`, default text preservation, explicit text format, invalid format, missing format value, and duplicate format.
- Modify: `crates/mdid-cli/src/main.rs`
  - Add `format: MoatOutputFormat` to `MoatRoundCommand`.
  - Parse `--format text|json` in `parse_moat_round_command` while preserving existing round override/input/history parsing behavior.
  - Branch in `run_moat_round` to render either existing text output or a deterministic JSON envelope.
  - Update CLI usage text to include `[--format text|json]` for `moat round`.
- Modify: `README.md`
  - Document `moat round --format json` for external controllers and list the JSON envelope fields.
- Modify: `AGENTS.md`
  - Truth-sync the local-only constraints and default-text/JSON-envelope semantics for `moat round`.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Truth-sync the moat-loop design spec to include the round JSON envelope while keeping autonomous daemon/process execution out of scope.

### Task 1: Add `moat round --format json`

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `README.md`
- Modify: `AGENTS.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`

- [ ] **Step 1: Write the failing JSON envelope test**

Add this test near the existing `moat round` CLI tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn moat_round_json_emits_deterministic_controller_envelope() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--format", "json"])
        .output()
        .expect("failed to run mdid-cli moat round json");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("valid json envelope");

    assert_eq!(value["type"], "moat_round");
    assert_eq!(value["source"], "sample");
    assert_eq!(value["history_path"], serde_json::Value::Null);
    assert_eq!(value["input_path"], serde_json::Value::Null);
    assert!(!value["round_id"].as_str().expect("round_id string").is_empty());
    assert_eq!(value["continue_decision"], "continue");
    assert!(value["executed_tasks"].as_array().expect("executed tasks array").len() > 0);
    assert!(value["implemented_specs"].as_array().expect("implemented specs array").len() > 0);
    assert!(value["moat_score_before"].as_u64().expect("score before number") > 0);
    assert!(value["moat_score_after"].as_u64().expect("score after number") > 0);
    assert!(value["improvement_delta"].is_number());
    assert!(value["stop_reason"].is_null());
    assert!(value["ready_tasks"].as_array().expect("ready tasks array").len() > 0);
    assert!(value["assignments"].as_array().expect("assignments array").len() > 0);
    assert!(value["task_states"].as_array().expect("task states array").len() > 0);
    assert!(value["decision_summary"].as_str().expect("decision summary string").len() > 0);
    assert!(value["constraints"].as_array().expect("constraints array").iter().any(|item| item == "local_only"));
}
```

- [ ] **Step 2: Run the JSON envelope test to verify RED**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_round_json_emits_deterministic_controller_envelope -- --nocapture`

Expected: FAIL because `moat round` currently rejects `--format`, treats it as an unknown flag, or prints text that is not valid JSON.

- [ ] **Step 3: Write default/explicit text and parser validation tests**

Add these tests near the same `moat round` test group in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn moat_round_default_text_output_is_preserved() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round"])
        .output()
        .expect("failed to run mdid-cli moat round text");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert_eq!(
        stdout,
        concat!(
            "moat round complete\n",
            "continue_decision=continue\n",
            "executed_tasks=planner:strategy,planner:spec,coder:implementation,reviewer:review\n",
            "implemented_specs=spec:phi-redaction-cli\n",
            "moat_score_before=58\n",
            "moat_score_after=75\n",
            "stop_reason=<none>\n",
        )
    );
}

#[test]
fn moat_round_explicit_text_output_matches_default() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--format", "text"])
        .output()
        .expect("failed to run mdid-cli moat round explicit text");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.starts_with("moat round complete\n"));
    assert!(stdout.contains("continue_decision=continue\n"));
    assert!(stdout.contains("stop_reason=<none>\n"));
}

#[test]
fn moat_round_rejects_unknown_format() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--format", "yaml"])
        .output()
        .expect("failed to run mdid-cli moat round unknown format");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("unknown moat round format: yaml"));
}

#[test]
fn moat_round_rejects_missing_format_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--format"])
        .output()
        .expect("failed to run mdid-cli moat round missing format value");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("missing value for moat round --format"));
}

#[test]
fn moat_round_rejects_duplicate_format() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--format", "json", "--format", "text"])
        .output()
        .expect("failed to run mdid-cli moat round duplicate format");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("duplicate moat round --format"));
}
```

- [ ] **Step 4: Run focused parser/output tests to verify current behavior**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_round_ -- --nocapture`

Expected: the new JSON, explicit text, and parser validation tests FAIL until `--format` is implemented; existing default text tests PASS.

- [ ] **Step 5: Implement minimal CLI parsing**

In `crates/mdid-cli/src/main.rs`:

- Add `format: MoatOutputFormat` to `MoatRoundCommand` and default it to `MoatOutputFormat::Text`.
- Replace `parse_moat_round_command` with a parser that separates `--format` from the existing round flags, mirroring the safe control-plane pattern without changing `--input-path`, `--history-path`, or override behavior.
- Add `fn parse_moat_round_format(value: &str) -> Result<MoatOutputFormat, String>` that accepts only `text` and `json` and returns `unknown moat round format: VALUE` for any other value.
- Return `missing value for moat round --format` when the flag has no following non-flag value.
- Return `duplicate moat round --format` when the flag is supplied more than once.
- Update the `USAGE` constant to show `moat round [--input-path PATH] [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false] [--history-path PATH] [--format text|json]`.

- [ ] **Step 6: Implement deterministic text/json rendering branch**

In `run_moat_round` in `crates/mdid-cli/src/main.rs`:

- Keep `let report = run_bounded_round(round_input_for_command(command)?);` unchanged.
- Keep the existing `append_report_to_history` behavior before output so JSON truthfully reports `history_saved=true` only after append succeeds.
- Move the current `println!` block into `print_moat_round_text(command, &report)` without changing the emitted text.
- Add `print_moat_round_json(command, &report)` that uses `serde_json::json!` and `serde_json::to_string_pretty`.
- For the JSON envelope, emit these deterministic fields in this order by constructing the object in order:
  - `type: "moat_round"`
  - `source: "input"` when `--input-path` is supplied, otherwise `"sample"`
  - `history_path: string|null`
  - `input_path: string|null`
  - `history_saved: bool`
  - `round_id: string`
  - `continue_decision: "continue"|"stop"`
  - `executed_tasks: string[]`
  - `implemented_specs: string[]`
  - `moat_score_before: number`
  - `moat_score_after: number`
  - `improvement_delta: number`
  - `stop_reason: string|null`
  - `ready_tasks: string[]` from `report.control_plane.task_graph.ready_node_ids()`
  - `assignments: string[]` using the same deterministic `agent_id:role:task` style already used by control-plane JSON if present, otherwise the existing assignment formatter output split into stable strings
  - `task_states: string[]` using `format_task_states(&report.control_plane.task_graph.nodes)`
  - `decision_summary: string` from `report.control_plane.memory.latest_decision_summary().unwrap_or_else(|| "<none>".to_string())`
  - `constraints: ["local_only", "bounded", "one_shot", "no_agent_launch", "no_daemon", "no_background_work", "no_crawling", "no_pr_creation", "no_cron_creation", "no_artifact_writes"]`
- Branch on `command.format`: `Text` calls `print_moat_round_text`; `Json` calls `print_moat_round_json`.

- [ ] **Step 7: Add history/input-path JSON coverage**

Add this integration test to `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn moat_round_json_reports_input_and_history_paths_after_persisting() {
    let temp_dir = unique_temp_dir("moat-round-json-input-history");
    fs::create_dir_all(&temp_dir).expect("create temp dir");
    let input_path = temp_dir.join("round-input.json");
    let history_path = temp_dir.join("moat-history.json");
    fs::write(&input_path, local_moat_round_input_json()).expect("write moat input");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--input-path",
            input_path.to_str().expect("input path utf8"),
            "--history-path",
            history_path.to_str().expect("history path utf8"),
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run mdid-cli moat round json with input/history");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(history_path.exists(), "history file should be persisted before json output");
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("valid json envelope");

    assert_eq!(value["type"], "moat_round");
    assert_eq!(value["source"], "input");
    assert_eq!(value["input_path"], input_path.display().to_string());
    assert_eq!(value["history_path"], history_path.display().to_string());
    assert_eq!(value["history_saved"], true);
    assert!(!value["round_id"].as_str().expect("round_id string").is_empty());
}
```

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_round_json_ -- --nocapture`

Expected: PASS after implementation.

- [ ] **Step 8: Run targeted GREEN tests**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_round_ -- --nocapture`

Expected: PASS.

- [ ] **Step 9: Update README, AGENTS, and spec truth**

Update `README.md`:

- Add `mdid-cli moat round --format json` to the CLI examples for automation/external controllers.
- State text output is the default and remains stable for humans/scripts.
- Document the `moat_round` JSON envelope fields and local-only constraints.
- State that `--history-path PATH` remains the only write side effect and no artifacts, PRs, cron jobs, daemon processes, crawling, or agent launches occur.

Update `AGENTS.md`:

- Add a `Moat round JSON envelope` or equivalent section saying `mdid-cli moat round [--input-path PATH] [--history-path PATH] [--format text|json]` is a bounded local-only one-shot runner.
- Include the envelope type `moat_round`, default text behavior, history persistence rule, and the explicit prohibitions on launching agents, daemon/background work, crawling, cron jobs, PR creation, and artifact writes.

Update `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`:

- Truth-sync the CLI automation surface to list `moat round --format text|json` as landed or planned by this slice.
- Clarify that JSON envelope support is for external autonomous controllers, while full autonomous daemon/process execution remains out of scope/future.
- Keep the spec aligned with README and AGENTS wording for local-only and bounded behavior.

- [ ] **Step 10: Run broader verification**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli moat_ --test moat_cli --no-fail-fast`

Expected: PASS.

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --no-fail-fast`

Expected: PASS.

- [ ] **Step 11: Commit**

Run:

```bash
git add crates/mdid-cli/tests/moat_cli.rs crates/mdid-cli/src/main.rs README.md AGENTS.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-28-med-de-id-moat-round-json-envelope.md
git commit -m "feat(cli): emit moat round json envelope"
```
