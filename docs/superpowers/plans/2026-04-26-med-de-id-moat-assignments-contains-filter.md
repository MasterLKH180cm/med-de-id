# Moat Assignments Contains Filter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a read-only `--contains TEXT` filter to `mdid-cli moat assignments` so operators can drill into latest persisted assignment rows by persisted assignment text without mutating history.

**Architecture:** Extend the existing assignment inspection command parser and in-memory row filter only. The command must continue opening an existing history file, inspecting only the latest persisted `agent_assignments`, and rendering deterministic escaped pipe-delimited output.

**Tech Stack:** Rust workspace, `mdid-cli`, integration tests in `crates/mdid-cli/tests/moat_cli.rs`, docs in README and moat-loop design spec.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `contains: Option<String>` to `MoatAssignmentsCommand`.
  - Parse `--contains TEXT` with `required_flag_value(args, index, "--contains", true)?`.
  - Reject duplicate `--contains` with `duplicate flag: --contains`.
  - Filter raw persisted `assignment.node_id`, `assignment.title`, or `assignment.spec_ref` before escaping.
  - Update assignments usage text to include `[--contains TEXT]`.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Update the `USAGE` constant expected output.
  - Add RED/GREEN integration tests for positive match, zero-match, role+contains conjunction, missing value, flag-like missing value, duplicate flag, and read-only/no-append behavior.
- Modify: `README.md`
  - Document `--contains TEXT` for `moat assignments` and raw-content semantics.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Document the new read-only assignments filter and parser behavior.
- Modify: `docs/superpowers/plans/2026-04-26-med-de-id-moat-assignments-contains-filter.md`
  - Check off steps after implementation/review.

### Task 1: Add `moat assignments --contains TEXT`

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
- Modify: `docs/superpowers/plans/2026-04-26-med-de-id-moat-assignments-contains-filter.md`

- [x] **Step 1: Write failing tests for assignments contains filtering**

Add tests to `crates/mdid-cli/tests/moat_cli.rs` following the existing `moat assignments` integration-test style:

```rust
#[test]
fn assignments_filters_latest_assignments_by_contains_text() {
    let history_path = unique_history_path("moat-assignments-contains");
    run_cli(["moat", "round", "--history-path", history_path.to_str().unwrap()]);

    let output = run_cli([
        "moat",
        "assignments",
        "--history-path",
        history_path.to_str().unwrap(),
        "--contains",
        "Strategy",
    ]);

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("moat assignments\n"));
    assert!(stdout.contains("assignment_entries=1\n"));
    assert!(stdout.contains("assignment=planner|strategy_generation|Strategy Generation|strategy_generation|<none>\n"));
    assert!(!stdout.contains("assignment=reviewer|review|"));
}

#[test]
fn assignments_contains_filter_returns_zero_matches_without_error() {
    let history_path = unique_history_path("moat-assignments-contains-zero");
    run_cli(["moat", "round", "--history-path", history_path.to_str().unwrap()]);

    let output = run_cli([
        "moat",
        "assignments",
        "--history-path",
        history_path.to_str().unwrap(),
        "--contains",
        "No Such Assignment Text",
    ]);

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "moat assignments\nassignment_entries=0\n");
}

#[test]
fn assignments_role_and_contains_filters_are_conjunctive() {
    let history_path = unique_history_path("moat-assignments-role-contains");
    run_cli(["moat", "round", "--history-path", history_path.to_str().unwrap()]);

    let output = run_cli([
        "moat",
        "assignments",
        "--history-path",
        history_path.to_str().unwrap(),
        "--role",
        "reviewer",
        "--contains",
        "Strategy",
    ]);

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "moat assignments\nassignment_entries=0\n");
}

#[test]
fn assignments_rejects_missing_contains_value() {
    let history_path = unique_history_path("moat-assignments-missing-contains");

    let output = run_cli([
        "moat",
        "assignments",
        "--history-path",
        history_path.to_str().unwrap(),
        "--contains",
    ]);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing value for --contains"));
    assert!(!history_path.exists());
}

#[test]
fn assignments_rejects_flag_like_contains_value() {
    let history_path = unique_history_path("moat-assignments-flaglike-contains");

    let output = run_cli([
        "moat",
        "assignments",
        "--history-path",
        history_path.to_str().unwrap(),
        "--contains",
        "--role",
        "planner",
    ]);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing value for --contains"));
    assert!(!history_path.exists());
}

#[test]
fn assignments_rejects_duplicate_contains_filter() {
    let history_path = unique_history_path("moat-assignments-duplicate-contains");

    let output = run_cli([
        "moat",
        "assignments",
        "--history-path",
        history_path.to_str().unwrap(),
        "--contains",
        "Strategy",
        "--contains",
        "Review",
    ]);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("duplicate flag: --contains"));
    assert!(!history_path.exists());
}

#[test]
fn assignments_contains_filter_does_not_append_history() {
    let history_path = unique_history_path("moat-assignments-contains-readonly");
    run_cli(["moat", "round", "--history-path", history_path.to_str().unwrap()]);

    let output = run_cli([
        "moat",
        "assignments",
        "--history-path",
        history_path.to_str().unwrap(),
        "--contains",
        "Strategy",
    ]);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));

    let history_output = run_cli(["moat", "history", "--history-path", history_path.to_str().unwrap()]);
    assert!(history_output.status.success(), "stderr: {}", String::from_utf8_lossy(&history_output.stderr));
    assert!(String::from_utf8_lossy(&history_output.stdout).contains("entries=1\n"));
}
```

- [x] **Step 2: Run RED tests and verify they fail for the missing flag**

Run:

```bash
source "$HOME/.cargo/env"
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli assignments_contains -- --nocapture
```

Expected: FAIL. The parser should reject `--contains` as `unknown flag: --contains` or the new tests should fail because no contains filtering exists yet.

- [x] **Step 3: Implement minimal parser and filter**

In `crates/mdid-cli/src/main.rs`, change `MoatAssignmentsCommand` from:

```rust
struct MoatAssignmentsCommand {
    history_path: PathBuf,
    role: Option<MoatAgentRole>,
    kind: Option<MoatTaskNodeKind>,
    node_id: Option<String>,
    title_contains: Option<String>,
    spec_ref: Option<String>,
}
```

to:

```rust
struct MoatAssignmentsCommand {
    history_path: PathBuf,
    role: Option<MoatAgentRole>,
    kind: Option<MoatTaskNodeKind>,
    node_id: Option<String>,
    title_contains: Option<String>,
    spec_ref: Option<String>,
    contains: Option<String>,
}
```

In `parse_moat_assignments_command`, add `let mut contains = None;`, parse the flag:

```rust
"--contains" => {
    let value = required_flag_value(args, index, "--contains", true)?;
    if contains.is_some() {
        return Err(duplicate_flag_error("--contains"));
    }
    contains = Some(value.to_string());
}
```

and include `contains` in the returned struct.

In `run_moat_assignments`, add the raw persisted filter before rendering:

```rust
.filter(|assignment| {
    command
        .contains
        .as_deref()
        .map(|needle| {
            assignment.node_id.contains(needle)
                || assignment.title.contains(needle)
                || assignment
                    .spec_ref
                    .as_deref()
                    .map(|spec_ref| spec_ref.contains(needle))
                    .unwrap_or(false)
        })
        .unwrap_or(true)
})
```

Update the assignments usage string in `crates/mdid-cli/src/main.rs` to:

```text
mdid-cli moat assignments --history-path PATH [--role planner|coder|reviewer] [--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation] [--node-id NODE_ID] [--title-contains TEXT] [--spec-ref SPEC_REF] [--contains TEXT]
```

- [x] **Step 4: Run targeted GREEN tests**

Run:

```bash
source "$HOME/.cargo/env"
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli assignments_contains -- --nocapture
```

Expected: PASS.

- [x] **Step 5: Update docs/spec/usage tests**

Update `crates/mdid-cli/tests/moat_cli.rs` `USAGE` constant to include `[--contains TEXT]` on the assignments line.

Update `README.md` assignment command text to include `[--contains TEXT]` and say:

```markdown
The optional `--contains TEXT` filter performs a case-sensitive substring match over raw persisted assignment `node_id`, `title`, or `spec_ref` before escaping; it is conjunctive with `--role`, `--kind`, `--node-id`, `--title-contains`, and `--spec-ref`.
```

Update `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md` assignment command text to include `[--contains TEXT]` and the same raw-content/conjunctive/read-only semantics.

Mark all checklist items in this plan complete only after tests and docs are green.

- [x] **Step 6: Run broader verification**

Run:

```bash
source "$HOME/.cargo/env"
CARGO_INCREMENTAL=0 cargo fmt --check
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli assignments -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli
```

Expected: PASS.

- [x] **Step 7: Commit**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs README.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-26-med-de-id-moat-assignments-contains-filter.md
git commit -m "feat: filter moat assignments by text"
```

Expected: commit succeeds on `feature/moat-loop-autonomy`.
