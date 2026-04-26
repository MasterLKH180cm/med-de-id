# Moat Decision Log Rationale Filter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a read-only `--rationale-contains` filter to `mdid-cli moat decision-log` so operators can drill into persisted latest-round decision rationale text without matching summaries.

**Architecture:** Extend only the CLI parsing and latest-round read-only filtering path for the existing decision-log inspection command. Preserve existing output format, existing `--contains` combined summary/rationale semantics, and all history immutability guarantees.

**Tech Stack:** Rust workspace, `mdid-cli` binary integration tests, local JSON moat history store, Cargo test runner.

---

## File Structure

- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add behavior tests for rationale-only matching, zero-match output, conjunctive role matching, missing/flag-like value errors, duplicate flag rejection, and read-only history behavior.
  - Update the test-local `USAGE` string to include `--rationale-contains TEXT`.
- Modify: `crates/mdid-cli/src/main.rs`
  - Add `rationale_contains: Option<String>` to `MoatDecisionLogCommand`.
  - Parse `--rationale-contains` with required non-flag value and duplicate rejection.
  - Apply rationale-only case-sensitive substring filtering conjunctively in `run_moat_decision_log`.
  - Update CLI usage string.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Document `--rationale-contains` as a read-only latest-round decision-log filter.
- Create: `docs/superpowers/plans/2026-04-26-med-de-id-moat-decision-log-rationale-filter.md`
  - This plan.

## Task 1: Add `--rationale-contains` filtering to `mdid-cli moat decision-log`

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
- Create: `docs/superpowers/plans/2026-04-26-med-de-id-moat-decision-log-rationale-filter.md`

- [x] **Step 1: Write failing tests**

Update the `USAGE` constant in `crates/mdid-cli/tests/moat_cli.rs` so the decision-log portion reads exactly:

```rust
"moat decision-log --history-path PATH [--role planner|coder|reviewer] [--contains TEXT] [--summary-contains TEXT] [--rationale-contains TEXT]"
```

Add these tests near the existing decision-log filter tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn decision_log_filters_latest_decisions_by_rationale_contains() {
    let history_path = unique_history_path("decision-log-rationale");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history for rationale filter");
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "decision-log",
            "--history-path",
            history_path_arg,
            "--rationale-contains",
            "evaluation completed",
        ])
        .output()
        .expect("failed to run mdid-cli moat decision-log with rationale filter");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("decision_log_entries=1\n"));
    assert!(stdout.contains("decision=reviewer|review approved bounded moat round|evaluation completed with passing tests\n"));
    assert!(!stdout.contains("decision=planner|selected workflow audit strategy|"));

    cleanup_history_path(&history_path);
}

#[test]
fn decision_log_rationale_filter_returns_zero_entries_when_no_rationale_matches() {
    let history_path = unique_history_path("decision-log-rationale-empty");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history for empty rationale filter");
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "decision-log",
            "--history-path",
            history_path_arg,
            "--rationale-contains",
            "not in persisted rationale",
        ])
        .output()
        .expect("failed to run mdid-cli moat decision-log with unmatched rationale filter");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "decision_log_entries=0\n");

    cleanup_history_path(&history_path);
}

#[test]
fn decision_log_rationale_filter_conjoins_with_role_filter() {
    let history_path = unique_history_path("decision-log-rationale-role");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history for rationale role filter");
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "decision-log",
            "--history-path",
            history_path_arg,
            "--role",
            "planner",
            "--rationale-contains",
            "evaluation completed",
        ])
        .output()
        .expect("failed to run mdid-cli moat decision-log with role and rationale filters");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "decision_log_entries=0\n");

    cleanup_history_path(&history_path);
}

#[test]
fn decision_log_rejects_missing_rationale_contains_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "decision-log",
            "--history-path",
            "/tmp/mdid-unused-history.json",
            "--rationale-contains",
        ])
        .output()
        .expect("failed to run mdid-cli moat decision-log with missing rationale value");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("missing value for --rationale-contains\n{}\n", USAGE)
    );
}

#[test]
fn decision_log_rejects_flag_like_rationale_contains_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "decision-log",
            "--history-path",
            "/tmp/mdid-unused-history.json",
            "--rationale-contains",
            "--role",
            "planner",
        ])
        .output()
        .expect("failed to run mdid-cli moat decision-log with flag-like rationale value");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("missing value for --rationale-contains\n{}\n", USAGE)
    );
}

#[test]
fn decision_log_rejects_duplicate_rationale_contains_filter() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "decision-log",
            "--history-path",
            "/tmp/mdid-unused-history.json",
            "--rationale-contains",
            "evaluation",
            "--rationale-contains",
            "tests",
        ])
        .output()
        .expect("failed to run mdid-cli moat decision-log with duplicate rationale filter");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("duplicate flag: --rationale-contains\n{}\n", USAGE)
    );
}

#[test]
fn decision_log_rationale_filter_does_not_append_history() {
    let history_path = unique_history_path("decision-log-rationale-read-only");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history for rationale read-only check");
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));

    let inspect = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "decision-log",
            "--history-path",
            history_path_arg,
            "--rationale-contains",
            "evaluation completed",
        ])
        .output()
        .expect("failed to inspect moat decision log by rationale");
    assert!(inspect.status.success(), "{}", String::from_utf8_lossy(&inspect.stderr));

    let history = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "history", "--history-path", history_path_arg])
        .output()
        .expect("failed to inspect moat history after rationale decision-log filter");
    assert!(history.status.success(), "{}", String::from_utf8_lossy(&history.stderr));
    assert!(String::from_utf8_lossy(&history.stdout).contains("entries=1\n"));

    cleanup_history_path(&history_path);
}
```

- [x] **Step 2: Run tests to verify RED**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli rationale -- --nocapture
```

Expected: FAIL because `--rationale-contains` is currently an unknown flag or the usage string does not yet include it.

- [x] **Step 3: Implement minimal production code**

In `crates/mdid-cli/src/main.rs`, change `MoatDecisionLogCommand` to:

```rust
struct MoatDecisionLogCommand {
    history_path: String,
    role: Option<AgentRole>,
    contains: Option<String>,
    summary_contains: Option<String>,
    rationale_contains: Option<String>,
}
```

In `parse_moat_decision_log_command`, add `let mut rationale_contains = None;` and this match arm after `--summary-contains`:

```rust
"--rationale-contains" => {
    let value = required_flag_value(args, index, "--rationale-contains", true)?;
    if rationale_contains.is_some() {
        return Err(duplicate_flag_error("--rationale-contains"));
    }
    rationale_contains = Some(value.clone());
}
```

Include `rationale_contains` in the returned `MoatDecisionLogCommand`.

In `run_moat_decision_log`, add this filter after the summary filter:

```rust
.filter(|decision| {
    command
        .rationale_contains
        .as_ref()
        .map(|needle| decision.rationale.contains(needle))
        .unwrap_or(true)
})
```

Update the production `usage()` decision-log portion to:

```rust
moat decision-log --history-path PATH [--role planner|coder|reviewer] [--contains TEXT] [--summary-contains TEXT] [--rationale-contains TEXT]
```

- [x] **Step 4: Update spec documentation**

In `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`, update the shipped decision-log bullet so it includes:

```markdown
`--rationale-contains TEXT` performs a case-sensitive substring match over decision rationale only
```

and states that `--role`, `--contains`, `--summary-contains`, and `--rationale-contains` combine conjunctively.

- [x] **Step 5: Run targeted tests to verify GREEN**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli rationale -- --nocapture
```

Expected: PASS.

- [x] **Step 6: Run broader relevant tests**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli decision_log -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --bin mdid-cli -- --nocapture
```

Expected: PASS.

- [ ] **Step 7: Commit**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-26-med-de-id-moat-decision-log-rationale-filter.md
git commit -m "feat: filter moat decision log by rationale"
```

## Self-Review

- Spec coverage: The plan implements a read-only latest-round rationale-only decision-log filter, parse errors, conjunctive role behavior, docs, and immutability/read-only tests.
- Placeholder scan: No TODO/TBD/fill-in placeholders are present.
- Type consistency: The new command field is consistently named `rationale_contains`; the CLI flag is consistently `--rationale-contains`.
