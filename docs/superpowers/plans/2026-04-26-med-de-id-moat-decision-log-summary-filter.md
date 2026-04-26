# Med De Id Moat Decision Log Summary Filter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a read-only `mdid-cli moat decision-log --summary-contains TEXT` filter so operators can drill into latest persisted decisions by summary text only.

**Architecture:** Extend the existing moat decision-log parser and latest-round renderer in `crates/mdid-cli/src/main.rs` with one optional case-sensitive substring filter applied only to `decision.summary`. Keep the existing `--contains` filter as summary-or-rationale search, apply all filters conjunctively, and update CLI tests plus README/spec documentation.

**Tech Stack:** Rust workspace, `mdid-cli`, `mdid-runtime::moat_history::LocalMoatHistoryStore`, std `Command` integration tests.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `summary_contains: Option<String>` to `MoatDecisionLogCommand`.
  - Parse `--summary-contains TEXT`, rejecting missing, flag-like, and duplicate values.
  - Apply `--role`, `--contains`, and `--summary-contains` conjunctively over latest persisted decisions.
  - Update usage text to include `[--summary-contains TEXT]`.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add integration tests for positive summary-only matching, rationale-only non-match, conjunctive role+summary behavior, parser errors, and no-append/read-only behavior.
  - Update the shared `USAGE` constant.
- Modify: `README.md`
  - Document the optional summary filter and read-only semantics.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Update the shipped decision-log bullet to include `[--summary-contains TEXT]` and latest-round read-only filtering semantics.
- Create: `docs/superpowers/plans/2026-04-26-med-de-id-moat-decision-log-summary-filter.md`
  - This implementation plan.

---

### Task 1: Decision-log summary filter CLI

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
- Test: `crates/mdid-cli/tests/moat_cli.rs`

- [x] **Step 1: Write failing tests**

Add tests near the existing `moat_decision_log` tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn decision_log_filters_latest_decisions_by_summary_text() {
    let history_path = unique_history_path("decision-log-summary-contains");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let round_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("moat round command should run");
    assert!(round_output.status.success(), "{}", String::from_utf8_lossy(&round_output.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "decision-log",
            "--history-path",
            history_path_arg,
            "--summary-contains",
            "review approved",
        ])
        .output()
        .expect("decision-log command should run");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("decision_log_entries=1\n"), "{stdout}");
    assert!(stdout.contains("decision=reviewer|review approved bounded moat round|review approved bounded moat round after evaluation cleared the improvement threshold\n"), "{stdout}");
}

#[test]
fn decision_log_summary_filter_ignores_rationale_only_matches() {
    let history_path = unique_history_path("decision-log-summary-contains-rationale");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let round_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("moat round command should run");
    assert!(round_output.status.success(), "{}", String::from_utf8_lossy(&round_output.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "decision-log",
            "--history-path",
            history_path_arg,
            "--summary-contains",
            "evaluation cleared",
        ])
        .output()
        .expect("decision-log command should run");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "decision_log_entries=0\n");
}

#[test]
fn decision_log_combines_role_and_summary_filters() {
    let history_path = unique_history_path("decision-log-summary-role");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let round_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("moat round command should run");
    assert!(round_output.status.success(), "{}", String::from_utf8_lossy(&round_output.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "decision-log",
            "--history-path",
            history_path_arg,
            "--role",
            "planner",
            "--summary-contains",
            "review approved",
        ])
        .output()
        .expect("decision-log command should run");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "decision_log_entries=0\n");
}

#[test]
fn decision_log_rejects_missing_summary_contains_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "decision-log", "--summary-contains"])
        .output()
        .expect("decision-log command should run");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing value for --summary-contains"));
}

#[test]
fn decision_log_rejects_flag_like_summary_contains_value() {
    let history_path = unique_history_path("decision-log-summary-flag-like");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "decision-log",
            "--history-path",
            history_path_arg,
            "--summary-contains",
            "--role",
            "reviewer",
        ])
        .output()
        .expect("decision-log command should run");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing value for --summary-contains"));
    assert!(!history_path.exists());
}

#[test]
fn decision_log_rejects_duplicate_summary_contains_filter() {
    let history_path = unique_history_path("decision-log-summary-duplicate");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "decision-log",
            "--history-path",
            history_path_arg,
            "--summary-contains",
            "review",
            "--summary-contains",
            "approved",
        ])
        .output()
        .expect("decision-log command should run");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("duplicate flag: --summary-contains"));
    assert!(!history_path.exists());
}
```

- [x] **Step 2: Run tests to verify RED**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli summary_contains -- --nocapture
```

Expected: FAIL because `--summary-contains` is not in the parser and the tests/usage updates are not implemented yet.

- [x] **Step 3: Implement minimal parser and filter**

Update `MoatDecisionLogCommand` in `crates/mdid-cli/src/main.rs` to include:

```rust
summary_contains: Option<String>,
```

Update `parse_moat_decision_log_command` to track `summary_contains`, parse it with:

```rust
"--summary-contains" => {
    let value = required_flag_value(args, index, "--summary-contains", true)?;
    if summary_contains.is_some() {
        return Err(duplicate_flag_error("--summary-contains"));
    }
    summary_contains = Some(value.clone());
}
```

and include `summary_contains` in the returned command. Update `run_moat_decision_log` filtering so each decision is retained only when all present filters match:

```rust
if let Some(summary_contains) = &command.summary_contains {
    if !decision.summary.contains(summary_contains) {
        return false;
    }
}
```

Update usage strings to show:

```text
mdid-cli moat decision-log --history-path PATH [--role planner|coder|reviewer] [--contains TEXT] [--summary-contains TEXT]
```

- [x] **Step 4: Update docs**

In `README.md` and `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`, document that `moat decision-log` supports `--summary-contains TEXT`, that it is case-sensitive, latest-round scoped, read-only, and conjunctive with `--role` and `--contains`.

- [x] **Step 5: Run targeted GREEN tests**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli summary_contains -- --nocapture
cargo test -p mdid-cli --test moat_cli decision_log -- --nocapture
```

Expected: PASS.

- [x] **Step 6: Run broader verification**

Run:

```bash
source "$HOME/.cargo/env"
cargo fmt --check
cargo test -p mdid-cli --test moat_cli moat -- --nocapture
cargo test -p mdid-cli
```

Expected: PASS.

- [x] **Step 7: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs README.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-26-med-de-id-moat-decision-log-summary-filter.md
git commit -m "feat: filter moat decision log by summary"
```

---

## Self-Review

- Spec coverage: The plan covers parser behavior, latest-round read-only filtering, conjunctive semantics, tests, docs, and verification.
- Placeholder scan: No TBD/TODO/fill-later placeholders are present.
- Type consistency: The new field is consistently named `summary_contains`; command examples use `--summary-contains` throughout.
