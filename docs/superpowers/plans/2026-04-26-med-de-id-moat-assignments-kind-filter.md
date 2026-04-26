# Moat Assignments Kind Filter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a read-only `--kind` filter to `mdid-cli moat assignments` so operators can drill into latest persisted Planner/Coder/Reviewer assignment rows by persisted task-node kind.

**Architecture:** Extend the existing assignment-inspection command in `crates/mdid-cli/src/main.rs` by reusing the task-graph kind parser and `MoatTaskNodeKind` equality checks. Keep the surface read-only and latest-round scoped: it must only call `LocalMoatHistoryStore::open_existing`, inspect `latest.report.control_plane.agent_assignments`, and never append, schedule, launch agents, crawl, open PRs, or create cron jobs.

**Tech Stack:** Rust workspace; `mdid-cli` binary integration tests via `std::process::Command`; existing `mdid-domain::MoatTaskNodeKind`; existing Cargo test runner.

---

## File Structure

- Modify `crates/mdid-cli/src/main.rs`
  - Add `kind: Option<MoatTaskNodeKind>` to `MoatAssignmentsCommand`.
  - Parse `--kind KIND` in `parse_moat_assignments_command` with strict missing-value and duplicate-flag behavior.
  - Add `parse_moat_assignments_kind_filter` accepting the exact persisted wire values: `market_scan`, `competitor_analysis`, `lock_in_analysis`, `strategy_generation`, `spec_planning`, `implementation`, `review`, `evaluation`.
  - Filter `run_moat_assignments` rows by `assignment.kind` conjunctively with existing role/node/title filters.
  - Update `USAGE` text to show `[--kind ...]` in the assignments command.
- Modify `crates/mdid-cli/tests/moat_cli.rs`
  - Add failing tests for positive kind filtering, zero-match filtering, role+kind conjunction, unknown kind rejection before history touch, missing value, flag-like missing value, duplicate flag, and read-only/no-append behavior.
  - Update the `USAGE` constant to include the assignments `--kind` filter.
- Modify `README.md`
  - Document the new `mdid-cli moat assignments --kind ...` usage and exact accepted persisted wire values.
- Modify `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Truth-sync the shipped foundation bullet for `moat assignments` to include `--kind` semantics and non-mutation guarantees.
- Modify this plan file
  - Mark completed checkboxes if implementation materially differs from plan due existing code details.

---

### Task 1: Add `--kind` to persisted moat assignment inspection

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
- Modify: `docs/superpowers/plans/2026-04-26-med-de-id-moat-assignments-kind-filter.md`

- [x] **Step 1: Write failing CLI tests for assignments kind filtering**

Append these tests near the existing assignment filter tests in `crates/mdid-cli/tests/moat_cli.rs`, and update the `USAGE` constant so its assignments segment reads exactly:

```rust
"moat assignments --history-path PATH [--role planner|coder|reviewer] [--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation] [--node-id NODE_ID] [--title-contains TEXT]"
```

Add these tests:

```rust
#[test]
fn assignments_filters_latest_assignments_by_kind() {
    let history_path = unique_history_path("assignments-kind");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--history-path",
            history_path_arg,
            "--implementation-tasks",
            "0",
        ])
        .output()
        .expect("failed to seed moat history for assignments kind filter");
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--kind",
            "strategy_generation",
        ])
        .output()
        .expect("failed to inspect moat assignments by kind");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout,
        "moat assignments\nassignment_entries=1\nassignment=planner|strategy_generation|Strategy Generation|strategy_generation|<none>\n"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn assignments_kind_filter_returns_zero_when_no_assignment_matches() {
    let history_path = unique_history_path("assignments-kind-empty");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--history-path",
            history_path_arg,
            "--implementation-tasks",
            "0",
        ])
        .output()
        .expect("failed to seed moat history for empty assignments kind filter");
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--kind",
            "review",
        ])
        .output()
        .expect("failed to inspect moat assignments by unmatched kind");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "moat assignments\nassignment_entries=0\n"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn assignments_kind_filter_is_conjunctive_with_role_filter() {
    let history_path = unique_history_path("assignments-kind-role");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--history-path",
            history_path_arg,
            "--implementation-tasks",
            "0",
        ])
        .output()
        .expect("failed to seed moat history for assignment kind and role filter");
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--role",
            "reviewer",
            "--kind",
            "strategy_generation",
        ])
        .output()
        .expect("failed to inspect moat assignments by role and kind");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "moat assignments\nassignment_entries=0\n"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn assignments_rejects_unknown_kind_without_touching_history() {
    let history_path = unique_history_path("assignments-kind-unknown");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--kind",
            "lockin_analysis",
        ])
        .output()
        .expect("failed to reject unknown moat assignments kind");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("unknown moat assignments kind: lockin_analysis\n{}\n", USAGE)
    );
    assert!(!history_path.exists());
}

#[test]
fn assignments_rejects_missing_kind_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            "/tmp/mdid-unused-history.json",
            "--kind",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with missing kind value");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("missing value for --kind\n{}\n", USAGE)
    );
}

#[test]
fn assignments_rejects_flag_like_kind_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            "/tmp/mdid-unused-history.json",
            "--kind",
            "--role",
            "planner",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with flag-like kind value");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("missing value for --kind\n{}\n", USAGE)
    );
}

#[test]
fn assignments_rejects_duplicate_kind_filter() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            "/tmp/mdid-unused-history.json",
            "--kind",
            "strategy_generation",
            "--kind",
            "review",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with duplicate kind filter");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("duplicate flag: --kind\n{}\n", USAGE)
    );
}

#[test]
fn assignments_kind_filter_does_not_append_history() {
    let history_path = unique_history_path("assignments-kind-read-only");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--history-path",
            history_path_arg,
            "--implementation-tasks",
            "0",
        ])
        .output()
        .expect("failed to seed moat history for assignments kind read-only check");
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));

    let inspect = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--kind",
            "strategy_generation",
        ])
        .output()
        .expect("failed to inspect moat assignments by kind");
    assert!(inspect.status.success(), "{}", String::from_utf8_lossy(&inspect.stderr));

    let history = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "history", "--history-path", history_path_arg])
        .output()
        .expect("failed to inspect moat history after assignments kind filter");
    assert!(history.status.success(), "{}", String::from_utf8_lossy(&history.stderr));
    assert!(String::from_utf8_lossy(&history.stdout).contains("entries=1\n"));

    cleanup_history_path(&history_path);
}
```

- [ ] **Step 2: Run RED tests and verify failure**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli assignments_kind -- --nocapture
```

Expected: FAIL because `mdid-cli moat assignments --kind ...` currently returns `unknown flag: --kind`, and/or the USAGE string does not yet include the new option.

- [ ] **Step 3: Implement minimal CLI parser and filter**

In `crates/mdid-cli/src/main.rs`, make these exact structural changes:

```rust
struct MoatAssignmentsCommand {
    history_path: String,
    role: Option<AgentRole>,
    kind: Option<MoatTaskNodeKind>,
    node_id: Option<String>,
    title_contains: Option<String>,
}
```

Inside `parse_moat_assignments_command`, add `let mut kind = None;`, parse this match arm before `--node-id`, and include `kind` in the returned struct:

```rust
"--kind" => {
    let value = required_flag_value(args, index, "--kind", true)?;
    if kind.is_some() {
        return Err(duplicate_flag_error("--kind"));
    }
    kind = Some(parse_moat_assignments_kind_filter(value)?);
}
```

Add this parser near `parse_moat_assignments_role_filter`:

```rust
fn parse_moat_assignments_kind_filter(value: &str) -> Result<MoatTaskNodeKind, String> {
    match value {
        "market_scan" => Ok(MoatTaskNodeKind::MarketScan),
        "competitor_analysis" => Ok(MoatTaskNodeKind::CompetitorAnalysis),
        "lock_in_analysis" => Ok(MoatTaskNodeKind::LockInAnalysis),
        "strategy_generation" => Ok(MoatTaskNodeKind::StrategyGeneration),
        "spec_planning" => Ok(MoatTaskNodeKind::SpecPlanning),
        "implementation" => Ok(MoatTaskNodeKind::Implementation),
        "review" => Ok(MoatTaskNodeKind::Review),
        "evaluation" => Ok(MoatTaskNodeKind::Evaluation),
        other => Err(format!("unknown moat assignments kind: {other}")),
    }
}
```

In `run_moat_assignments`, add this filter after role filtering:

```rust
.filter(|assignment| {
    command
        .kind
        .map(|kind| assignment.kind == kind)
        .unwrap_or(true)
})
```

Update the CLI usage string in `usage()` to include the assignments `--kind` filter exactly as in Step 1.

- [ ] **Step 4: Run targeted GREEN tests**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli assignments_kind -- --nocapture
```

Expected: PASS for all assignment kind tests.

- [ ] **Step 5: Update README and moat-loop design spec**

In `README.md`, update the assignment inspection bullet to say:

```markdown
- `mdid-cli moat assignments --history-path PATH [--role planner|coder|reviewer] [--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation] [--node-id NODE_ID] [--title-contains TEXT]` inspects the latest persisted read-only Planner/Coder/Reviewer assignment projection and prints deterministic `assignment=<role>|<node_id>|<title>|<kind>|<spec_ref>` rows. Persisted `node_id`, `title`, and `spec_ref` fields are escaped for pipe-delimited output (`\\` as `\\\\`, `|` as `\\|`, newline as `\\n`, carriage return as `\\r`); bounded enum fields are not escaped. The optional `--kind` filter accepts only exact persisted task-node kind wire values and combines conjunctively with role, node-id, and title filters.
```

In `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`, update the shipped foundation `moat assignments` bullet to include `--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation`, mention that `--kind` accepts only exact persisted wire values, and preserve the read-only non-mutation sentence.

- [ ] **Step 6: Run broader verification**

Run:

```bash
source "$HOME/.cargo/env"
cargo fmt --check
cargo test -p mdid-cli --test moat_cli assignments -- --nocapture
cargo test -p mdid-cli --test moat_cli moat -- --nocapture
cargo test -p mdid-cli
```

Expected: all PASS.

- [ ] **Step 7: Commit the slice**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs README.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-26-med-de-id-moat-assignments-kind-filter.md
git commit -m "feat: filter moat assignments by kind"
```

Expected: commit succeeds on `feature/moat-loop-autonomy`.

---

## Self-Review

**Spec coverage:** This plan extends only the existing read-only assignment inspection surface. It covers parser behavior, filtering behavior, exact persisted kind wire values, no-match behavior, read-only behavior, README/spec docs, targeted tests, broader CLI tests, and commit.

**Placeholder scan:** No TBD/TODO/fill-in placeholders remain. All behavior changes include exact code snippets and exact commands.

**Type consistency:** The plan uses existing `MoatTaskNodeKind`, existing `MoatAssignmentsCommand`, existing `format_moat_task_kind`, existing `required_flag_value`, and existing assignment fields (`role`, `node_id`, `title`, `kind`, `spec_ref`) consistently.
