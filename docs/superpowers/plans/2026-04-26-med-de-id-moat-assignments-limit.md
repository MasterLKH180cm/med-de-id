# Moat Assignments Limit Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a read-only `--limit N` filter to `mdid-cli moat assignments` so operators can bound persisted assignment rows after all other filters.

**Architecture:** Extend the existing `MoatAssignmentsCommand` parser and renderer path in `crates/mdid-cli/src/main.rs`. The command remains latest-round scoped and read-only; filters are applied first, then `--limit` keeps the first `N` assignments in deterministic persisted assignment order.

**Tech Stack:** Rust, Cargo, mdid-cli integration tests, local JSON moat history fixtures.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs` — add `limit: Option<usize>` to `MoatAssignmentsCommand`, parse `--limit`, apply `.take(limit)` after all assignment filters, update usage text.
- Modify: `crates/mdid-cli/tests/moat_cli.rs` — add failing tests for assignment limit behavior, parser errors, and read-only/no-append behavior; update duplicated `USAGE` string.
- Modify: `README.md` — document `mdid-cli moat assignments --limit N` as a read-only bounded assignment inspection option.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md` — document assignment limit semantics.
- Modify: `docs/superpowers/plans/2026-04-26-med-de-id-moat-assignments-limit.md` — keep this implementation plan checked and truth-synced.

### Task 1: Add assignment limit tests and implementation

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
- Modify: `docs/superpowers/plans/2026-04-26-med-de-id-moat-assignments-limit.md`

- [x] **Step 1: Write failing tests**

Add tests following existing `moat assignments` style:

```rust
#[test]
fn cli_assignments_limit_bounds_filtered_rows() {
    let history_path = unique_history_path("assignments-limit-bounds");
    seed_successful_moat_history(&history_path);

    let output = run_cli([
        "moat",
        "assignments",
        "--history-path",
        history_path.to_str().unwrap(),
        "--limit",
        "1",
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("assignment_entries=1"));
    assert_eq!(stdout.matches("assignment=").count(), 1);

    let history = run_cli(["moat", "history", "--history-path", history_path.to_str().unwrap()]);
    assert!(history.status.success());
    assert!(String::from_utf8(history.stdout).unwrap().contains("entries=1"));
}

#[test]
fn cli_assignments_limit_applies_after_role_filter() {
    let history_path = unique_history_path("assignments-limit-role");
    seed_successful_moat_history(&history_path);

    let output = run_cli([
        "moat",
        "assignments",
        "--history-path",
        history_path.to_str().unwrap(),
        "--role",
        "planner",
        "--limit",
        "1",
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("assignment_entries=1"));
    assert_eq!(stdout.matches("assignment=planner|").count(), 1);
    assert!(!stdout.contains("assignment=coder|"));
    assert!(!stdout.contains("assignment=reviewer|"));
}

#[test]
fn cli_rejects_zero_assignments_limit() {
    let history_path = unique_history_path("assignments-limit-zero");
    let output = run_cli([
        "moat",
        "assignments",
        "--history-path",
        history_path.to_str().unwrap(),
        "--limit",
        "0",
    ]);

    assert!(!output.status.success());
    assert!(String::from_utf8(output.stderr).unwrap().contains("invalid value for --limit: expected positive integer, got 0"));
    assert!(!history_path.exists());
}

#[test]
fn cli_rejects_duplicate_assignments_limit() {
    let history_path = unique_history_path("assignments-limit-duplicate");
    let output = run_cli([
        "moat",
        "assignments",
        "--history-path",
        history_path.to_str().unwrap(),
        "--limit",
        "1",
        "--limit",
        "2",
    ]);

    assert!(!output.status.success());
    assert!(String::from_utf8(output.stderr).unwrap().contains("duplicate flag: --limit"));
    assert!(!history_path.exists());
}
```

- [x] **Step 2: Run tests to verify RED**

Run: `source "$HOME/.cargo/env" && CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli assignments_limit -- --nocapture`

Expected: FAIL because `--limit` is currently an unknown flag or no tests exist yet.

- [x] **Step 3: Implement minimal parser and renderer changes**

In `crates/mdid-cli/src/main.rs`, add `limit: Option<usize>` to `MoatAssignmentsCommand`; initialize `let mut limit = None;`; parse:

```rust
"--limit" => {
    let value = required_flag_value(args, index, "--limit", true)?;
    if limit.is_some() {
        return Err(duplicate_flag_error("--limit"));
    }
    limit = Some(parse_positive_usize_flag("--limit", value)?);
}
```

Include `limit` in the constructed command. In `run_moat_assignments`, after all existing filters are applied and before collecting/rendering rows, apply:

```rust
if let Some(limit) = command.limit {
    assignments.truncate(limit);
}
```

If the implementation uses iterators rather than a mutable vector, use `.take(limit)` after filters. Update CLI usage text to include `[--limit N]`.

- [x] **Step 4: Update docs**

In `README.md` and `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`, document:

```text
mdid-cli moat assignments --history-path PATH [--role planner|coder|reviewer] [--state pending|ready|in_progress|completed|blocked] [--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation] [--node-id NODE_ID] [--title-contains TEXT] [--spec-ref SPEC_REF] [--contains TEXT] [--limit N]
```

State that `--limit N` is read-only, positive integer only, applied after all other filters, and keeps the first `N` rows in deterministic persisted assignment order.

- [x] **Step 5: Run targeted GREEN tests**

Run: `source "$HOME/.cargo/env" && CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli assignments_limit -- --nocapture`

Expected: PASS.

- [x] **Step 6: Run broader relevant tests**

Run:

```bash
source "$HOME/.cargo/env"
CARGO_INCREMENTAL=0 cargo fmt --check
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli assignments -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli
```

Expected: PASS.

- [x] **Step 7: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs README.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-26-med-de-id-moat-assignments-limit.md
git commit -m "feat: limit moat assignments output"
```

## Self-Review

- Spec coverage: The plan covers parser behavior, read-only semantics, filter ordering, docs, tests, and verification.
- Placeholder scan: No TBD/TODO placeholders are present.
- Type consistency: The plan uses existing `MoatAssignmentsCommand`, `MoatTaskNodeKind`, `MoatTaskNodeState`, and `parse_positive_usize_flag` names from the CLI implementation.
