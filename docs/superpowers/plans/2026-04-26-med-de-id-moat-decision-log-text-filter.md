# Med De Id Moat Decision Log Text Filter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a read-only `mdid-cli moat decision-log --contains TEXT` filter so operators and future SDD handoff tooling can drill into the latest persisted decision log by decision text without mutating history.

**Architecture:** Extend the existing moat decision-log CLI parser and runner in `crates/mdid-cli/src/main.rs` with one optional exact substring filter applied to persisted decision summaries and rationales. Keep the surface latest-round scoped, deterministic, line-oriented, and read-only; update README/spec/docs to describe the shipped contract.

**Tech Stack:** Rust workspace, `mdid-cli`, `mdid-runtime::moat_history::LocalMoatHistoryStore`, std `Command` integration tests.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `contains: Option<String>` to `MoatDecisionLogCommand`.
  - Parse `--contains TEXT`, reject missing/duplicate values, and keep unknown flags rejected.
  - Filter latest persisted decisions where `decision.summary.contains(TEXT) || decision.rationale.contains(TEXT)`.
  - Update usage text to include `[--contains TEXT]`.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add tests proving matching and non-matching text filters, parser errors, and read-only behavior.
  - Update the shared `USAGE` constant.
- Modify: `README.md`
  - Document the optional text filter and read-only semantics.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Update the shipped decision-log bullet to include `[--contains TEXT]` and read-only filtering semantics.
- Create: `docs/superpowers/plans/2026-04-26-med-de-id-moat-decision-log-text-filter.md`
  - This implementation plan.

---

### Task 1: Decision-log text filter CLI

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
- Test: `crates/mdid-cli/tests/moat_cli.rs`

- [ ] **Step 1: Write failing tests**

Add these tests to `crates/mdid-cli/tests/moat_cli.rs` near the existing `moat_decision_log` tests:

```rust
#[test]
fn decision_log_filters_latest_decisions_by_text() {
    let history_path = unique_history_path("decision-log-contains");
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
            "--contains",
            "approved bounded",
        ])
        .output()
        .expect("decision-log command should run");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("decision_log_entries=1\n"), "{stdout}");
    assert!(stdout.contains("decision=reviewer|review approved bounded moat round|tests passed and moat score improved\n"), "{stdout}");
}

#[test]
fn decision_log_text_filter_returns_zero_when_no_decision_matches() {
    let history_path = unique_history_path("decision-log-contains-empty");
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
            "--contains",
            "not-present-in-latest-decision",
        ])
        .output()
        .expect("decision-log command should run");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "decision_log_entries=0\n");
}

#[test]
fn decision_log_text_filter_combines_with_role_filter() {
    let history_path = unique_history_path("decision-log-contains-role");
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
            "--contains",
            "approved bounded",
        ])
        .output()
        .expect("decision-log command should run");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "decision_log_entries=0\n");
}

#[test]
fn decision_log_rejects_missing_contains_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "decision-log",
            "--history-path",
            "/tmp/missing-history.json",
            "--contains",
        ])
        .output()
        .expect("decision-log command should run");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing value for --contains"));
}

#[test]
fn decision_log_rejects_duplicate_contains_filter() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "decision-log",
            "--history-path",
            "/tmp/missing-history.json",
            "--contains",
            "approved",
            "--contains",
            "tests",
        ])
        .output()
        .expect("decision-log command should run");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("duplicate flag: --contains"));
}

#[test]
fn decision_log_text_filter_does_not_append_history() {
    let history_path = unique_history_path("decision-log-contains-read-only");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let round_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("moat round command should run");
    assert!(round_output.status.success(), "{}", String::from_utf8_lossy(&round_output.stderr));

    let decision_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "decision-log",
            "--history-path",
            history_path_arg,
            "--contains",
            "approved",
        ])
        .output()
        .expect("decision-log command should run");
    assert!(decision_output.status.success(), "{}", String::from_utf8_lossy(&decision_output.stderr));

    let history_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "history", "--history-path", history_path_arg])
        .output()
        .expect("history command should run");
    assert!(history_output.status.success(), "{}", String::from_utf8_lossy(&history_output.stderr));
    assert!(String::from_utf8_lossy(&history_output.stdout).contains("entries=1\n"));
}
```

Update the `USAGE` constant so the decision-log portion reads:

```rust
"moat decision-log --history-path PATH [--role planner|coder|reviewer] [--contains TEXT]"
```

- [ ] **Step 2: Run targeted RED tests**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli decision_log -- --nocapture
```

Expected: FAIL because `--contains` is an unknown flag and usage still lacks the new option.

- [ ] **Step 3: Implement minimal CLI support**

In `crates/mdid-cli/src/main.rs`:

```rust
struct MoatDecisionLogCommand {
    history_path: String,
    role: Option<AgentRole>,
    contains: Option<String>,
}
```

In `parse_moat_decision_log_command`, add a `contains` variable, parse `--contains` with `required_flag_value(args, index, "--contains", false)?`, reject duplicates using `duplicate_flag_error("--contains")`, and return it in `MoatDecisionLogCommand`.

In `run_moat_decision_log`, after the role filter, add:

```rust
if let Some(needle) = &command.contains {
    decisions.retain(|decision| {
        decision.summary.contains(needle) || decision.rationale.contains(needle)
    });
}
```

Update `usage()` so the decision-log command includes `[--contains TEXT]`.

- [ ] **Step 4: Run targeted GREEN tests**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli decision_log -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Update docs/spec**

Update `README.md` decision-log documentation to show:

```bash
cargo run -p mdid-cli -- moat decision-log --history-path .mdid/moat-history.json --contains approved
```

and state that `--contains TEXT` filters latest persisted decisions by substring match over the decision summary or rationale, combines conjunctively with `--role`, and remains read-only.

Update `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md` shipped slice bullet to:

```markdown
- a read-only `mdid-cli moat decision-log --history-path PATH [--role planner|coder|reviewer] [--contains TEXT]` inspection surface that requires an already-persisted history file, reads the latest persisted round without running or appending a new one, optionally filters decisions by Planner/Coder/Reviewer role and/or substring match over decision summary/rationale, and prints each persisted decision as `decision=<role>|<summary>|<rationale>`
```

- [ ] **Step 6: Run broader verification**

Run:

```bash
source "$HOME/.cargo/env"
cargo fmt --check
cargo test -p mdid-cli --test moat_cli decision_log -- --nocapture
cargo test -p mdid-cli --test moat_cli moat -- --nocapture
cargo test -p mdid-cli
```

Expected: all PASS.

- [ ] **Step 7: Commit**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs README.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-26-med-de-id-moat-decision-log-text-filter.md
git commit -m "feat: filter moat decision log by text"
```
