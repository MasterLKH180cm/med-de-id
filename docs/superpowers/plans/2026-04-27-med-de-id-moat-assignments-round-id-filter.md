# med-de-id Moat Assignments Round ID Filter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a read-only `--round-id ROUND_ID` filter to `mdid-cli moat assignments` so operators can inspect agent assignments from a specific persisted moat round instead of only the latest round.

**Architecture:** Extend the CLI command model/parser with an optional `round_id`, select the matching persisted `MoatHistoryEntry` before projecting assignments, and keep all existing assignment filters conjunctive and read-only. The implementation mirrors the existing `moat task-graph --round-id` selection behavior while preserving deterministic output and no-match success.

**Tech Stack:** Rust, Cargo workspace, `mdid-cli`, `mdid-runtime::moat_history::LocalMoatHistoryStore`, integration tests in `crates/mdid-cli/tests/moat_cli.rs`.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `round_id: Option<String>` to `MoatAssignmentsCommand`.
  - Parse `--round-id ROUND_ID` in `parse_moat_assignments_command`.
  - Include `--round-id` in usage text.
  - Select the requested history entry before rendering assignments.
  - Keep assignment rows read-only; never append/run/schedule.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Update `USAGE` constant.
  - Add focused integration tests for exact round selection and unknown round no-match behavior.
  - Update in-file parser unit expected struct literals with `round_id: None` where needed.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Truth-sync the shipped status bullet for `moat assignments` to mention `--round-id` exact-match behavior.

### Task 1: CLI assignments exact round-id inspection

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`

- [x] **Step 1: Write the failing integration test**

Add this test near the other `moat assignments` tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn cli_filters_moat_assignments_by_exact_round_id() {
    let history_path = unique_history_path("assignments-round-id-filter");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let first_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed first moat history round for assignments round-id filter");
    assert!(first_output.status.success(), "expected first round success, stderr was: {}", String::from_utf8_lossy(&first_output.stderr));

    let store = LocalMoatHistoryStore::open(&history_path).expect("history store should open");
    let first_round_id = store.summary().latest_round_id.expect("first round id should exist");

    let second_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--review-loops", "0", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed second moat history round for assignments round-id filter");
    assert!(second_output.status.success(), "expected second round success, stderr was: {}", String::from_utf8_lossy(&second_output.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--round-id",
            &first_round_id,
            "--role",
            "reviewer",
        ])
        .output()
        .expect("failed to inspect moat assignments by round id");

    assert!(output.status.success(), "expected success, stderr was: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "moat assignments\nassignment_entries=1\nassignment=reviewer|review|Review|review|<none>\n"
    );

    cleanup_history_path(&history_path);
}
```

- [x] **Step 2: Write the failing no-match test**

Add this test near the first test:

```rust
#[test]
fn cli_returns_empty_moat_assignments_for_unknown_round_id() {
    let history_path = unique_history_path("assignments-round-id-empty");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history for unknown assignments round-id filter");
    assert!(seed_output.status.success(), "expected seed success, stderr was: {}", String::from_utf8_lossy(&seed_output.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--round-id",
            "missing-round-id",
        ])
        .output()
        .expect("failed to inspect moat assignments by unknown round id");

    assert!(output.status.success(), "expected success, stderr was: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "moat assignments\nassignment_entries=0\n");

    cleanup_history_path(&history_path);
}
```

- [x] **Step 3: Run RED targeted tests**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_filters_moat_assignments_by_exact_round_id -- --exact
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_returns_empty_moat_assignments_for_unknown_round_id -- --exact
```

Expected: both fail because `moat assignments` does not parse `--round-id` yet and prints usage/unknown flag.

- [x] **Step 4: Implement parser and command model**

In `crates/mdid-cli/src/main.rs`:

```rust
struct MoatAssignmentsCommand {
    history_path: String,
    round_id: Option<String>,
    role: Option<AgentRole>,
    state: Option<MoatTaskNodeState>,
    kind: Option<MoatTaskNodeKind>,
    node_id: Option<String>,
    depends_on: Option<String>,
    no_dependencies: bool,
    title_contains: Option<String>,
    spec_ref: Option<String>,
    contains: Option<String>,
    limit: Option<usize>,
}
```

Inside `parse_moat_assignments_command`, initialize `let mut round_id = None;`, parse:

```rust
"--round-id" => {
    let value = required_flag_value(args, index, "--round-id")?.clone();
    if round_id.is_some() {
        return Err(duplicate_flag_error("--round-id"));
    }
    round_id = Some(value);
    index += 2;
}
```

and include `round_id` in the returned `MoatAssignmentsCommand`.

- [x] **Step 5: Implement exact round selection before assignment projection**

In `run_moat_assignments`, select the entry like task-graph does:

```rust
let maybe_entry = match command.round_id.as_deref() {
    Some(round_id) => store
        .entries()
        .iter()
        .find(|entry| entry.report.summary.round_id == round_id),
    None => store.entries().last(),
};

let Some(entry) = maybe_entry else {
    println!("moat assignments");
    println!("assignment_entries=0");
    return Ok(());
};
```

Then use that `entry` for the existing assignment filtering/rendering logic.

- [x] **Step 6: Update usage text and parser unit expected literals**

Update both `USAGE` strings to show:

```text
moat assignments --history-path PATH [--round-id ROUND_ID] [--role planner|coder|reviewer] ...
```

For every `MoatAssignmentsCommand { ... }` expected literal in `crates/mdid-cli/src/main.rs` unit tests, add:

```rust
round_id: None,
```

Add/adjust one parser unit for `--round-id` if parser tests exist near the assignments parser tests:

```rust
assert_eq!(
    parse_command(&args_from(["moat", "assignments", "--history-path", "history.json", "--round-id", "round-1"])),
    Ok(CliCommand::MoatAssignments(MoatAssignmentsCommand {
        history_path: "history.json".to_string(),
        round_id: Some("round-1".to_string()),
        role: None,
        state: None,
        kind: None,
        node_id: None,
        depends_on: None,
        no_dependencies: false,
        title_contains: None,
        spec_ref: None,
        contains: None,
        limit: None,
    }))
);
```

- [x] **Step 7: Update spec status**

In `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`, update the `moat assignments` shipped bullet to include:

```text
`--round-id ROUND_ID` exact-matches `entry.report.summary.round_id` and selects that persisted round before projecting assignments; when absent, the command preserves the prior latest-round behavior.
```

- [x] **Step 8: Run GREEN targeted tests**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_filters_moat_assignments_by_exact_round_id -- --exact
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_returns_empty_moat_assignments_for_unknown_round_id -- --exact
```

Expected: PASS.

- [x] **Step 9: Run package verification**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli
```

Expected: PASS.

- [x] **Step 10: Commit**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-27-med-de-id-moat-assignments-round-id-filter.md
git commit -m "feat: filter moat assignments by round id"
```
