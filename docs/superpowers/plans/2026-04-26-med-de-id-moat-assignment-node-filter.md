# Moat Assignment Node Filter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add exact `--node-id NODE_ID` filtering to the read-only `mdid-cli moat assignments` inspection command.

**Architecture:** Extend the existing CLI-only assignment inspection surface without changing persisted history schemas or launching agents. The command continues to open existing moat history read-only, inspect only the latest round, and print deterministic line-oriented assignment rows after applying conjunctive filters.

**Tech Stack:** Rust workspace, `mdid-cli`, `mdid-runtime::moat_history::LocalMoatHistoryStore`, existing CLI integration tests in `crates/mdid-cli/tests/moat_cli.rs`.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - `MoatAssignmentsCommand`: add `node_id: Option<String>`.
  - `parse_moat_assignments_command`: parse `--node-id NODE_ID`, reject duplicate or missing values, and preserve existing `--history-path` / `--role` behavior.
  - `run_moat_assignments`: apply exact persisted `assignment.node_id == node_id` filtering after the role filter.
  - `usage`: document `moat assignments --history-path PATH [--role planner|coder|reviewer] [--node-id NODE_ID]`.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Update the shared `USAGE` string.
  - Add assignment node-id tests using the existing temp history helpers and `std::process::Command` style.
- Modify: `README.md`
  - Document `--node-id` on `moat assignments` as exact, read-only latest-round filtering.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Truth-sync the control-plane inspection contract with assignment `--node-id` filtering.

## Behavioral Contract

- `mdid-cli moat assignments --history-path PATH` remains unchanged.
- `--node-id NODE_ID` performs exact matching against persisted `assignment.node_id`; do not normalize underscores/hyphens and do not escape before comparing.
- `--role` and `--node-id` are conjunctive: both filters must match when both are present.
- If no assignment matches, command succeeds and prints:

```text
moat assignments
assignment_entries=0
```

- Missing `--node-id` value fails with `missing value for --node-id` plus usage.
- Duplicate `--node-id` fails with `duplicate flag: --node-id` plus usage.
- The command must not append history, schedule rounds, run new rounds, crawl data, launch agents, open PRs, create cron jobs, or mutate persisted history.

---

### Task 1: Add parser and output tests for assignment `--node-id`

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`

- [x] **Step 1: Write failing tests for exact node-id filtering**

Add these tests near the existing `moat_assignments` tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn assignments_filters_latest_assignments_by_node_id() {
    let history_path = unique_history_path("assignments-node-id");
    run_cli([
        "moat",
        "round",
        "--history-path",
        history_path.to_str().unwrap(),
    ]);

    let output = run_cli([
        "moat",
        "assignments",
        "--history-path",
        history_path.to_str().unwrap(),
        "--node-id",
        "strategy_generation",
    ]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("moat assignments\n"));
    assert!(stdout.contains("assignment_entries=1\n"));
    assert!(stdout.contains("assignment=planner|strategy_generation|Strategy Generation|strategy_generation|moat-spec/workflow-audit\n"));
    assert!(!stdout.contains("assignment=planner|market_scan|"));

    std::fs::remove_file(history_path).ok();
}

#[test]
fn assignments_node_id_filter_returns_zero_when_no_assignment_matches() {
    let history_path = unique_history_path("assignments-node-id-empty");
    run_cli([
        "moat",
        "round",
        "--history-path",
        history_path.to_str().unwrap(),
    ]);

    let output = run_cli([
        "moat",
        "assignments",
        "--history-path",
        history_path.to_str().unwrap(),
        "--node-id",
        "missing_node",
    ]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), "moat assignments\nassignment_entries=0\n");

    std::fs::remove_file(history_path).ok();
}

#[test]
fn assignments_node_id_filter_combines_with_role_filter() {
    let history_path = unique_history_path("assignments-node-id-role");
    run_cli([
        "moat",
        "round",
        "--history-path",
        history_path.to_str().unwrap(),
    ]);

    let output = run_cli([
        "moat",
        "assignments",
        "--history-path",
        history_path.to_str().unwrap(),
        "--role",
        "reviewer",
        "--node-id",
        "strategy_generation",
    ]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), "moat assignments\nassignment_entries=0\n");

    std::fs::remove_file(history_path).ok();
}
```

- [x] **Step 2: Write failing parser tests for missing and duplicate `--node-id`**

Add these tests near the existing assignment parser error tests:

```rust
#[test]
fn assignments_rejects_missing_node_id_value() {
    let history_path = unique_history_path("assignments-node-id-missing-value");

    let output = run_cli([
        "moat",
        "assignments",
        "--history-path",
        history_path.to_str().unwrap(),
        "--node-id",
    ]);

    assert!(!output.status.success());
    assert!(stderr(&output).contains("missing value for --node-id"));
    assert!(!history_path.exists());
}

#[test]
fn assignments_rejects_duplicate_node_id_filter() {
    let history_path = unique_history_path("assignments-node-id-duplicate");

    let output = run_cli([
        "moat",
        "assignments",
        "--history-path",
        history_path.to_str().unwrap(),
        "--node-id",
        "strategy_generation",
        "--node-id",
        "market_scan",
    ]);

    assert!(!output.status.success());
    assert!(stderr(&output).contains("duplicate flag: --node-id"));
    assert!(!history_path.exists());
}
```

- [x] **Step 3: Write failing read-only regression test**

Add this test near `assignment_inspection_does_not_append_history`:

```rust
#[test]
fn assignments_node_id_filter_does_not_append_history() {
    let history_path = unique_history_path("assignments-node-id-read-only");
    run_cli([
        "moat",
        "round",
        "--history-path",
        history_path.to_str().unwrap(),
    ]);

    let output = run_cli([
        "moat",
        "assignments",
        "--history-path",
        history_path.to_str().unwrap(),
        "--node-id",
        "strategy_generation",
    ]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));

    let history_output = run_cli([
        "moat",
        "history",
        "--history-path",
        history_path.to_str().unwrap(),
    ]);
    assert!(history_output.status.success(), "stderr: {}", stderr(&history_output));
    assert!(stdout(&history_output).contains("entries=1\n"));

    std::fs::remove_file(history_path).ok();
}
```

- [x] **Step 4: Update usage expectation to fail until implementation is added**

In the shared `USAGE` constant, change the assignments segment from:

```text
moat assignments --history-path PATH [--role planner|coder|reviewer]
```

to:

```text
moat assignments --history-path PATH [--role planner|coder|reviewer] [--node-id NODE_ID]
```

- [x] **Step 5: Run RED tests**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli assignments -- --nocapture
```

Expected: FAIL because `--node-id` is currently an unknown flag or not represented in usage. The broader `assignments` filter is intentional because the new assignment node-id tests use several descriptive names (`assignments_filters_latest_assignments_by_node_id`, `assignments_node_id_filter_returns_zero_when_no_assignment_matches`, `assignments_rejects_missing_node_id_value`) that do not all share one narrower substring.

---

### Task 2: Implement assignment `--node-id` parsing and filtering

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`

- [x] **Step 1: Add `node_id` to the command struct**

Change `MoatAssignmentsCommand` to:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
struct MoatAssignmentsCommand {
    history_path: String,
    role: Option<AgentRole>,
    node_id: Option<String>,
}
```

- [x] **Step 2: Parse `--node-id` exactly like task graph parsing**

Replace `parse_moat_assignments_command` with:

```rust
fn parse_moat_assignments_command(args: &[String]) -> Result<MoatAssignmentsCommand, String> {
    let mut history_path = None;
    let mut role = None;
    let mut node_id = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--history-path" => {
                let value = required_history_path_value(args, index)?.clone();
                if history_path.is_some() {
                    return Err(duplicate_flag_error("--history-path"));
                }
                history_path = Some(value);
            }
            "--role" => {
                let value = required_flag_value(args, index, "--role", false)?;
                if role.is_some() {
                    return Err(duplicate_flag_error("--role"));
                }
                role = Some(parse_moat_assignments_role_filter(value)?);
            }
            "--node-id" => {
                let value = required_flag_value(args, index, "--node-id", false)?;
                if node_id.is_some() {
                    return Err(duplicate_flag_error("--node-id"));
                }
                node_id = Some(value.clone());
            }
            flag => return Err(format!("unknown flag: {flag}")),
        }

        index += 2;
    }

    Ok(MoatAssignmentsCommand {
        history_path: history_path
            .ok_or_else(|| "missing required flag: --history-path".to_string())?,
        role,
        node_id,
    })
}
```

- [x] **Step 3: Apply exact node-id filtering in `run_moat_assignments`**

Change the assignment iterator to include a second filter:

```rust
    let assignments = latest
        .report
        .control_plane
        .agent_assignments
        .iter()
        .filter(|assignment| {
            command
                .role
                .map(|role| assignment.role == role)
                .unwrap_or(true)
        })
        .filter(|assignment| {
            command
                .node_id
                .as_ref()
                .map(|node_id| assignment.node_id == *node_id)
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();
```

- [x] **Step 4: Update CLI usage text**

In `usage()`, change the assignments usage segment to:

```text
moat assignments --history-path PATH [--role planner|coder|reviewer] [--node-id NODE_ID]
```

- [x] **Step 5: Run GREEN targeted tests**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli assignments -- --nocapture
```

Expected: PASS for the assignment node-id tests added in Task 1, plus the existing assignment inspection tests.

---

### Task 3: Document the read-only assignment node filter

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`

- [x] **Step 1: Update README assignment command description**

In the moat-loop CLI/control-plane section, ensure the assignments command is documented as:

```text
mdid-cli moat assignments --history-path PATH [--role planner|coder|reviewer] [--node-id NODE_ID]
```

Add or update the nearby prose to state:

```markdown
`moat assignments` is read-only and latest-round scoped. `--role` filters by bounded agent role, and `--node-id` performs an exact match against the persisted assignment node ID. Filters are conjunctive; if no assignment matches, the command prints `assignment_entries=0` and does not mutate history.
```

- [x] **Step 2: Update moat-loop design spec**

In `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`, update the assignment inspection contract to include:

```markdown
- `mdid-cli moat assignments --history-path PATH [--role planner|coder|reviewer] [--node-id NODE_ID]` inspects the latest persisted `agent_assignments` rows only. `--node-id` uses exact persisted node ID matching, combines conjunctively with `--role`, and returns `assignment_entries=0` without error when no assignment matches. It never appends history, schedules work, launches agents, or creates cron jobs.
```

- [x] **Step 3: Run docs-aware broader CLI tests**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli moat_assignments -- --nocapture
```

Expected: PASS for all assignment-focused CLI tests.

---

### Task 4: Final verification and commit

**Files:**
- Test: `crates/mdid-cli/tests/moat_cli.rs`
- Verify: `crates/mdid-cli/src/main.rs`, `README.md`, `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`, this plan file

- [x] **Step 1: Format check**

Run:

```bash
source "$HOME/.cargo/env"
cargo fmt --check
```

Expected: PASS.

- [x] **Step 2: Run focused assignment tests**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli moat_assignments -- --nocapture
```

Expected: PASS.

- [x] **Step 3: Run broader moat CLI tests**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli moat -- --nocapture
```

Expected: PASS.

- [x] **Step 4: Run crate verification**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli
```

Expected: PASS.

- [x] **Step 5: Commit**

Run:

```bash
git status --short
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs README.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-26-med-de-id-moat-assignment-node-filter.md
git commit -m "feat: filter moat assignments by node id"
```

Expected: commit succeeds on `feature/moat-loop-autonomy`.

---

## Self-Review

- Spec coverage: The plan covers parser behavior, exact filter semantics, combined role/node filtering, zero-match output, read-only guarantee, README/spec updates, and verification.
- Placeholder scan: No TBD/TODO/fill-in placeholders remain.
- Type consistency: `MoatAssignmentsCommand.node_id: Option<String>` and `assignment.node_id` are used consistently with the existing task graph filter pattern.
