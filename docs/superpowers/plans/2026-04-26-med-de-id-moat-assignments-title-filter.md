# med-de-id Moat Assignments Title Filter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a read-only `--title-contains` filter to `mdid-cli moat assignments` so operators can narrow latest persisted Planner/Coder/Reviewer assignment projections by task title text.

**Architecture:** Extend only the existing CLI assignments inspection command with one optional conjunctive string filter. The command continues to open existing history read-only, inspect only the latest round, preserve deterministic output, and never append history, schedule work, launch agents, crawl data, open PRs, or create cron jobs.

**Tech Stack:** Rust workspace, `mdid-cli`, `mdid-runtime::moat_history::LocalMoatHistoryStore`, Cargo integration tests.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Extend `USAGE` text for `moat assignments` to include `[--title-contains TEXT]`.
  - Add `title_contains: Option<String>` to `MoatAssignmentsCommand`.
  - Parse `--title-contains` with required non-flag value, duplicate flag rejection, and normal unknown-flag behavior.
  - Apply the filter conjunctively in `run_moat_assignments` using case-sensitive `assignment.title.contains(expected_title)`.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Update mirrored `USAGE` constant.
  - Add focused tests for matching, zero-match, conjunctive role behavior, missing value, duplicate flag, and read-only behavior.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Document the new assignments title filter in the shipped foundation slice.
- Create: `docs/superpowers/plans/2026-04-26-med-de-id-moat-assignments-title-filter.md`
  - This implementation plan.

## Task 1: Add `--title-contains` filtering to `mdid-cli moat assignments`

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
- Create: `docs/superpowers/plans/2026-04-26-med-de-id-moat-assignments-title-filter.md`

- [x] **Step 1: Write failing tests**

Add these tests near the existing assignments filter tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn assignments_title_filter_matches_latest_assignment_titles() {
    let history_path = unique_history_path("assignments-title-filter");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    seed_successful_moat_history(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--title-contains",
            "Strategy",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with title filter");

    assert!(output.status.success(), "stderr was: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "moat assignments\n",
            "assignment_entries=1\n",
            "assignment=planner|strategy_generation|Strategy Generation|strategy_generation|<none>\n",
        )
    );

    cleanup_history_path(&history_path);
}

#[test]
fn assignments_title_filter_returns_zero_entries_when_no_title_matches() {
    let history_path = unique_history_path("assignments-title-filter-empty");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    seed_successful_moat_history(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--title-contains",
            "No Such Assignment Title",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with unmatched title filter");

    assert!(output.status.success(), "stderr was: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "moat assignments\nassignment_entries=0\n");

    cleanup_history_path(&history_path);
}

#[test]
fn assignments_title_filter_combines_with_role_filter() {
    let history_path = unique_history_path("assignments-title-role-filter");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    seed_successful_moat_history(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--role",
            "coder",
            "--title-contains",
            "Strategy",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with role and title filters");

    assert!(output.status.success(), "stderr was: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "moat assignments\nassignment_entries=0\n");

    cleanup_history_path(&history_path);
}

#[test]
fn assignments_title_filter_requires_a_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            "/tmp/mdid-unused-history.json",
            "--title-contains",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with missing title filter value");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("missing value for --title-contains\n{}\n", USAGE)
    );
}

#[test]
fn assignments_title_filter_rejects_duplicate_flag() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            "/tmp/mdid-unused-history.json",
            "--title-contains",
            "Strategy",
            "--title-contains",
            "Review",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with duplicate title filter");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("duplicate flag: --title-contains\n{}\n", USAGE)
    );
}

#[test]
fn assignments_title_filter_does_not_append_history() {
    let history_path = unique_history_path("assignments-title-filter-read-only");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    seed_successful_moat_history(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--title-contains",
            "Strategy",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with title filter read-only check");
    assert!(output.status.success(), "stderr was: {}", String::from_utf8_lossy(&output.stderr));

    let history = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "history", "--history-path", history_path_arg])
        .output()
        .expect("failed to inspect moat history after assignments title filter");
    assert!(history.status.success(), "stderr was: {}", String::from_utf8_lossy(&history.stderr));
    assert!(String::from_utf8_lossy(&history.stdout).contains("entries=1\n"));

    cleanup_history_path(&history_path);
}
```

- [x] **Step 2: Run tests to verify RED**

Run: `cargo test -p mdid-cli --test moat_cli assignments_title_filter -- --nocapture`

Expected: FAIL to compile or run because `moat assignments` does not yet recognize `--title-contains`, and/or the mirrored usage string does not include it.

- [x] **Step 3: Implement minimal CLI support**

In `crates/mdid-cli/src/main.rs`:

```rust
struct MoatAssignmentsCommand {
    history_path: String,
    role: Option<AgentRole>,
    node_id: Option<String>,
    title_contains: Option<String>,
}
```

Parse the flag inside `parse_moat_assignments_command`:

```rust
"--title-contains" => {
    let value = required_flag_value(args, index, "--title-contains", true)?;
    if title_contains.is_some() {
        return Err(duplicate_flag_error("--title-contains"));
    }
    title_contains = Some(value.clone());
}
```

Return it:

```rust
Ok(MoatAssignmentsCommand {
    history_path: history_path
        .ok_or_else(|| "missing required flag: --history-path".to_string())?,
    role,
    node_id,
    title_contains,
})
```

Filter it in `run_moat_assignments` after the node-id filter:

```rust
.filter(|assignment| {
    command
        .title_contains
        .as_deref()
        .map(|expected_title| assignment.title.contains(expected_title))
        .unwrap_or(true)
})
```

Update both production and test `USAGE` strings so `moat assignments` reads:

```text
moat assignments --history-path PATH [--role planner|coder|reviewer] [--node-id NODE_ID] [--title-contains TEXT]
```

- [x] **Step 4: Update the design spec**

In `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`, update the assignments shipped-slice bullet so it says `mdid-cli moat assignments --history-path PATH [--role planner|coder|reviewer] [--node-id NODE_ID] [--title-contains TEXT]` and documents that `--title-contains` performs a case-sensitive substring match over persisted assignment titles, combines conjunctively with role/node-id, returns zero entries without error on no match, and remains read-only.

- [x] **Step 5: Run targeted tests to verify GREEN**

Run: `cargo test -p mdid-cli --test moat_cli assignments_title_filter -- --nocapture`

Expected: PASS.

- [x] **Step 6: Run broader relevant CLI tests**

Run: `cargo test -p mdid-cli --test moat_cli assignments -- --nocapture`

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-26-med-de-id-moat-assignments-title-filter.md
git commit -m "feat: filter moat assignments by title"
```

## Self-Review

- Spec coverage: The plan adds read-only assignment title filtering, exact command parsing behavior, zero-match behavior, conjunctive filtering, docs, and verification.
- Placeholder scan: No TBD/TODO/implement-later placeholders are present.
- Type consistency: `title_contains: Option<String>` is consistently used in the command struct, parser, and runner.
