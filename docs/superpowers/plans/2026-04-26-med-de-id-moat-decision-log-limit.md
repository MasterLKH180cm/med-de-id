# Moat Decision Log Limit Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `--limit N` option to `mdid-cli moat decision-log` so autonomous moat-loop operators can inspect only the newest bounded set of persisted decisions.

**Architecture:** Keep the slice local to the CLI parser and decision-log renderer. Parse `--limit` as a positive integer and apply it after existing role/text filters so it limits the visible filtered rows without changing persisted history semantics.

**Tech Stack:** Rust 2021, Cargo workspace, `mdid-cli` binary integration tests, `mdid-runtime` history store.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `limit: Option<usize>` to `MoatDecisionLogCommand`.
  - Parse `--limit N` in `parse_moat_decision_log_command` with duplicate/missing/non-positive validation.
  - Include the new flag in usage text.
  - Apply the limit after existing `decision_log_matches` filtering, selecting the newest matching rows from the latest persisted round.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Update `USAGE` with the new flag.
  - Add CLI integration tests for filtered limiting and invalid zero values.

### Task 1: Decision log `--limit` filter

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/moat_cli.rs`

- [ ] **Step 1: Write the failing tests**

Add these tests near the existing `moat decision-log` tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn cli_decision_log_limits_filtered_rows_to_requested_count() {
    let history_path = unique_history_path("decision-log-limit");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    seed_moat_history_round(history_path_arg, &["--review-loops", "0"]);
    seed_moat_history_round(history_path_arg, &[]);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "decision-log",
            "--history-path",
            history_path_arg,
            "--contains",
            "workflow",
            "--limit",
            "1",
        ])
        .output()
        .expect("failed to run mdid-cli moat decision-log with limit");

    assert!(
        output.status.success(),
        "expected success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).lines().count(),
        2,
        "expected header plus one limited decision row"
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("moat decision log\n"));
    assert!(String::from_utf8_lossy(&output.stdout).contains("summary=continue moat loop\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn cli_rejects_zero_decision_log_limit() {
    let history_path = unique_history_path("decision-log-zero-limit");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    seed_moat_history_round(history_path_arg, &[]);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "decision-log",
            "--history-path",
            history_path_arg,
            "--limit",
            "0",
        ])
        .output()
        .expect("failed to run mdid-cli moat decision-log with zero limit");

    assert!(!output.status.success(), "expected command to fail");
    assert!(String::from_utf8_lossy(&output.stderr).contains("--limit must be greater than 0"));

    cleanup_history_path(&history_path);
}
```

- [ ] **Step 2: Run the tests to verify RED**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_decision_log_limits_filtered_rows_to_requested_count -- --exact
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_rejects_zero_decision_log_limit -- --exact
```

Expected: FAIL because `--limit` is currently rejected as an unknown flag.

- [ ] **Step 3: Implement minimal parser and renderer support**

In `crates/mdid-cli/src/main.rs`:

```rust
struct MoatDecisionLogCommand {
    history_path: String,
    role: Option<AgentRole>,
    contains: Option<String>,
    summary_contains: Option<String>,
    rationale_contains: Option<String>,
    limit: Option<usize>,
}
```

Initialize `let mut limit = None;`, parse `--limit` with duplicate checks, reject zero with `--limit must be greater than 0`, include `limit` in the returned struct, and apply it after filtering in `run_moat_decision_log` with `.take(command.limit.unwrap_or(usize::MAX))`.

Update the usage string in `crates/mdid-cli/tests/moat_cli.rs` and the production usage string to include `[--limit N]` after the decision-log filter flags.

- [ ] **Step 4: Run targeted tests to verify GREEN**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_decision_log_limits_filtered_rows_to_requested_count -- --exact
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_rejects_zero_decision_log_limit -- --exact
```

Expected: PASS.

- [ ] **Step 5: Run broader CLI moat test**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_decision -- --nocapture
```

Expected: PASS for all matching decision-log tests.

- [ ] **Step 6: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs docs/superpowers/plans/2026-04-26-med-de-id-moat-decision-log-limit.md
git commit -m "feat: limit moat decision log output"
```

## Self-Review

- Spec coverage: The plan adds bounded decision-log inspection for the autonomous moat-loop CLI and covers parser, usage, rendering, happy path, and invalid input.
- Placeholder scan: No TBD/TODO/fill-in placeholders remain.
- Type consistency: `limit` is consistently `Option<usize>` and command flag is consistently `--limit`.
