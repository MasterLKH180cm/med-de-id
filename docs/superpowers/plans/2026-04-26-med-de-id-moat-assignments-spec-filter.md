# Moat Assignments Spec Filter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a read-only `--spec-ref SPEC_REF` filter to `mdid-cli moat assignments` so operators can drill into latest-round persisted assignments for a specific implementation handoff.

**Architecture:** Extend the existing assignments command parser and filter pipeline only. The command remains latest-round scoped, opens only existing history, does not mutate history, and applies `--spec-ref` conjunctively with `--role`, `--kind`, `--node-id`, and `--title-contains`.

**Tech Stack:** Rust workspace, `mdid-cli`, persisted moat history JSON, Cargo integration tests.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `spec_ref: Option<String>` to `MoatAssignmentsCommand`.
  - Parse `--spec-ref SPEC_REF` with strict missing/duplicate handling.
  - Filter assignments by exact persisted `assignment.spec_ref` match.
  - Update CLI usage string.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add RED/GREEN integration tests for positive match, zero-match, role+spec conjunction, missing value, duplicate flag, and read-only/no-append behavior.
  - Update usage constant/expectations if present.
- Modify: `README.md`
  - Document the new `--spec-ref SPEC_REF` assignments filter.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Truth-sync the moat assignments operator contract.
- Modify: `docs/superpowers/plans/2026-04-26-med-de-id-moat-assignments-spec-filter.md`
  - Mark steps complete after implementation/review.

### Task 1: Add `--spec-ref` to moat assignments

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
- Modify: `docs/superpowers/plans/2026-04-26-med-de-id-moat-assignments-spec-filter.md`

- [x] **Step 1: Write the failing tests**

Add tests in `crates/mdid-cli/tests/moat_cli.rs` following the existing `assignments` test style:

```rust
#[test]
fn assignments_filters_latest_assignments_by_spec_ref() {
    let history_path = unique_history_path("moat-assignments-spec-ref");
    run_cli_ok(&["moat", "round", "--history-path", history_path.to_str().unwrap()]);

    let output = run_cli_ok(&[
        "moat",
        "assignments",
        "--history-path",
        history_path.to_str().unwrap(),
        "--spec-ref",
        "moat-spec/workflow-audit",
    ]);

    assert!(output.stdout.contains("moat assignments\n"));
    assert!(output.stdout.contains("assignment_entries=1\n"));
    assert!(output.stdout.contains("assignment=coder|implementation|Implementation|implementation|moat-spec/workflow-audit\n"));
}

#[test]
fn assignments_spec_ref_filter_returns_zero_for_no_match() {
    let history_path = unique_history_path("moat-assignments-spec-ref-zero");
    run_cli_ok(&["moat", "round", "--history-path", history_path.to_str().unwrap()]);

    let output = run_cli_ok(&[
        "moat",
        "assignments",
        "--history-path",
        history_path.to_str().unwrap(),
        "--spec-ref",
        "moat-spec/not-present",
    ]);

    assert_eq!(output.stdout, "moat assignments\nassignment_entries=0\n");
}

#[test]
fn assignments_combines_role_and_spec_ref_filters() {
    let history_path = unique_history_path("moat-assignments-role-spec-ref");
    run_cli_ok(&["moat", "round", "--history-path", history_path.to_str().unwrap()]);

    let output = run_cli_ok(&[
        "moat",
        "assignments",
        "--history-path",
        history_path.to_str().unwrap(),
        "--role",
        "planner",
        "--spec-ref",
        "moat-spec/workflow-audit",
    ]);

    assert_eq!(output.stdout, "moat assignments\nassignment_entries=0\n");
}

#[test]
fn assignments_rejects_missing_spec_ref_value() {
    let output = run_cli_err(&["moat", "assignments", "--history-path", "/tmp/missing.json", "--spec-ref"]);
    assert!(output.stderr.contains("missing value for --spec-ref"));
}

#[test]
fn assignments_rejects_flag_like_spec_ref_value() {
    let output = run_cli_err(&[
        "moat",
        "assignments",
        "--history-path",
        "/tmp/missing.json",
        "--spec-ref",
        "--role",
        "reviewer",
    ]);
    assert!(output.stderr.contains("missing value for --spec-ref"));
}

#[test]
fn assignments_rejects_duplicate_spec_ref_filter() {
    let output = run_cli_err(&[
        "moat",
        "assignments",
        "--history-path",
        "/tmp/missing.json",
        "--spec-ref",
        "moat-spec/workflow-audit",
        "--spec-ref",
        "moat-spec/other",
    ]);
    assert!(output.stderr.contains("duplicate flag: --spec-ref"));
}

#[test]
fn assignments_spec_ref_filter_does_not_append_history() {
    let history_path = unique_history_path("moat-assignments-spec-ref-read-only");
    run_cli_ok(&["moat", "round", "--history-path", history_path.to_str().unwrap()]);

    let before = run_cli_ok(&["moat", "history", "--history-path", history_path.to_str().unwrap()]);
    assert!(before.stdout.contains("entries=1\n"));

    run_cli_ok(&[
        "moat",
        "assignments",
        "--history-path",
        history_path.to_str().unwrap(),
        "--spec-ref",
        "moat-spec/workflow-audit",
    ]);

    let after = run_cli_ok(&["moat", "history", "--history-path", history_path.to_str().unwrap()]);
    assert!(after.stdout.contains("entries=1\n"));
}
```

- [x] **Step 2: Run tests to verify RED**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-cli --test moat_cli spec_ref -- --nocapture`

Expected: FAIL because `--spec-ref` is currently an unknown assignments flag or the tests are not yet wired into the command.

- [x] **Step 3: Implement the minimal parser and filter**

In `crates/mdid-cli/src/main.rs`, update `MoatAssignmentsCommand`:

```rust
struct MoatAssignmentsCommand {
    history_path: String,
    role: Option<AgentRole>,
    kind: Option<MoatTaskNodeKind>,
    node_id: Option<String>,
    title_contains: Option<String>,
    spec_ref: Option<String>,
}
```

In `parse_moat_assignments_command`, add `let mut spec_ref = None;`, parse:

```rust
"--spec-ref" => {
    let value = required_flag_value(args, index, "--spec-ref", true)?;
    if spec_ref.is_some() {
        return Err(duplicate_flag_error("--spec-ref"));
    }
    spec_ref = Some(value.clone());
}
```

Return it in `MoatAssignmentsCommand { ... spec_ref }`.

In `run_moat_assignments`, add this filter after `title_contains`:

```rust
.filter(|assignment| {
    command
        .spec_ref
        .as_deref()
        .map(|expected_spec_ref| assignment.spec_ref.as_deref() == Some(expected_spec_ref))
        .unwrap_or(true)
})
```

Update the usage string to include `[--spec-ref SPEC_REF]` in the `moat assignments` clause.

- [x] **Step 4: Run targeted tests to verify GREEN**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-cli --test moat_cli spec_ref -- --nocapture`

Expected: PASS for all new `spec_ref` assignments tests.

- [x] **Step 5: Update docs/spec/plan truthfully**

Update README and moat-loop spec so the assignments command is documented as:

```text
mdid-cli moat assignments --history-path PATH [--role planner|coder|reviewer] [--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation] [--node-id NODE_ID] [--title-contains TEXT] [--spec-ref SPEC_REF]
```

State that `--spec-ref` is an exact match against persisted `assignment.spec_ref`, filters conjunctively, no matches return `assignment_entries=0`, and inspection remains read-only/latest-round scoped.

- [x] **Step 6: Run broader verification**

Run:

```bash
source "$HOME/.cargo/env"
cargo fmt --check
cargo test -p mdid-cli --test moat_cli assignments -- --nocapture
cargo test -p mdid-cli
```

Expected: all commands PASS.

- [x] **Step 7: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs README.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-26-med-de-id-moat-assignments-spec-filter.md
git commit -m "feat: filter moat assignments by spec ref"
```

## Self-Review

- Spec coverage: the plan covers parser behavior, exact persisted matching, conjunctive filters, no-match behavior, read-only history behavior, usage, README, and design spec updates.
- Placeholder scan: no TBD/TODO/fill-in placeholders are present.
- Type consistency: `spec_ref: Option<String>` is consistently used on `MoatAssignmentsCommand`, compared against `assignment.spec_ref.as_deref()`, and documented as `--spec-ref SPEC_REF`.
