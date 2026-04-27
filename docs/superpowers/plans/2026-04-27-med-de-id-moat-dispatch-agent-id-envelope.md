# Moat Dispatch Agent ID Envelope Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an explicit `--agent-id` dispatch envelope field so each bounded moat-loop task handoff can be attributed to the local worker/agent that claimed it.

**Architecture:** Extend the existing `mdid-cli moat dispatch-next` command only; do not add a daemon, background scheduler, crawler, or unrestricted autonomous loop. The parser accepts an optional bounded string, the text output includes `agent_id=...`, and the JSON output includes an `agent_id` property in the dispatch envelope. Documentation stays truthful that this is still a local handoff surface, not automatic agent execution.

**Tech Stack:** Rust workspace, `mdid-cli`, CLI integration tests in `crates/mdid-cli/tests/moat_cli.rs`, README/plan docs.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `agent_id: Option<String>` to `MoatDispatchNextCommand`.
  - Parse `--agent-id AGENT_ID` as an optional, duplicate-rejected flag.
  - Render `agent_id=<value>` in text dispatch output, using `<none>` when absent.
  - Render `agent_id` in JSON dispatch output as a string value or null.
  - Update usage text for `moat dispatch-next`.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add CLI tests that first fail because `--agent-id` is not parsed/rendered.
- Modify: `README.md`
  - Document `--agent-id` for `moat dispatch-next` as local attribution metadata only.

### Task 1: Dispatch-next agent attribution field

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `README.md`

- [ ] **Step 1: Write the failing text-output test**

Append this test near the existing `dispatch-next` tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn cli_dispatch_next_text_output_includes_agent_id_attribution() {
    let history_path = unique_history_path("dispatch-agent-id-text");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let round_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to create persisted moat round for dispatch agent id test");
    assert!(
        round_output.status.success(),
        "expected round success, stderr was: {}",
        String::from_utf8_lossy(&round_output.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path_arg,
            "--role",
            "coder",
            "--agent-id",
            "coder-7",
        ])
        .output()
        .expect("failed to run mdid-cli moat dispatch-next with agent id");

    assert!(
        output.status.success(),
        "expected dispatch success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("agent_id=coder-7\n"), "stdout was: {stdout}");

    cleanup_history_path(&history_path);
}
```

- [ ] **Step 2: Run the text-output test to verify RED**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_dispatch_next_text_output_includes_agent_id_attribution -- --nocapture
```

Expected: FAIL because `--agent-id` is currently an unexpected argument or no `agent_id=` line is rendered.

- [ ] **Step 3: Write the failing JSON-output test**

Append this test near the existing dispatch-next JSON tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn cli_dispatch_next_json_output_includes_agent_id_attribution() {
    let history_path = unique_history_path("dispatch-agent-id-json");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let round_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to create persisted moat round for dispatch agent id json test");
    assert!(
        round_output.status.success(),
        "expected round success, stderr was: {}",
        String::from_utf8_lossy(&round_output.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path_arg,
            "--role",
            "coder",
            "--agent-id",
            "coder-7",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run mdid-cli moat dispatch-next json with agent id");

    assert!(
        output.status.success(),
        "expected dispatch success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: Value = serde_json::from_slice(&output.stdout).expect("dispatch output should be json");
    assert_eq!(payload["agent_id"], "coder-7");

    cleanup_history_path(&history_path);
}
```

- [ ] **Step 4: Run the JSON-output test to verify RED**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_dispatch_next_json_output_includes_agent_id_attribution -- --nocapture
```

Expected: FAIL because `--agent-id` is currently an unexpected argument or JSON output has no `agent_id` property.

- [ ] **Step 5: Implement parser and renderer**

In `crates/mdid-cli/src/main.rs`:

1. Add `agent_id: Option<String>` to `MoatDispatchNextCommand`.
2. Initialize `let mut agent_id = None;` in `parse_moat_dispatch_next_command`.
3. Add a `--agent-id` match arm that uses `required_flag_value(args, index, "--agent-id", true)?`, rejects duplicates with `duplicate_flag_error("--agent-id")`, and stores `Some(value.to_string())`.
4. Include `agent_id` in the returned command.
5. In text output for `run_moat_dispatch_next`, print `agent_id={}` after `round_id`, using `command.agent_id.as_deref().unwrap_or("<none>")`.
6. In JSON output, include `"agent_id": command.agent_id` as a nullable string field.
7. Add `[--agent-id AGENT_ID]` to the `moat dispatch-next` usage section.

- [ ] **Step 6: Run targeted tests to verify GREEN**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_dispatch_next_text_output_includes_agent_id_attribution cli_dispatch_next_json_output_includes_agent_id_attribution -- --nocapture
```

Expected: PASS for both new tests.

- [ ] **Step 7: Run relevant broader CLI tests**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli dispatch_next -- --nocapture
```

Expected: PASS for the dispatch-next related CLI tests.

- [ ] **Step 8: Update README**

In `README.md`, update the `moat dispatch-next` documentation to include `--agent-id AGENT_ID` and state that it is attribution metadata for the local handoff envelope only. It must not imply the CLI launches an agent, opens PRs, schedules background work, or creates cron jobs.

- [ ] **Step 9: Run documentation smoke check**

Run:

```bash
git diff -- README.md crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs
```

Expected: README documents the new flag truthfully and does not use misleading autonomous-execution language.

- [ ] **Step 10: Commit**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs README.md docs/superpowers/plans/2026-04-27-med-de-id-moat-dispatch-agent-id-envelope.md
git commit -m "feat(cli): add moat dispatch agent attribution"
```

Expected: commit succeeds on `feature/moat-loop-autonomy`.

## Self-Review

- Spec coverage: The plan adds bounded local dispatch attribution for text and JSON output, parser support, usage text, tests, and README truth-sync.
- Placeholder scan: No TBD/TODO/fill-in placeholders remain.
- Type consistency: The new field is consistently named `agent_id` in the command struct, parser, text output, JSON property, tests, usage, and README.
