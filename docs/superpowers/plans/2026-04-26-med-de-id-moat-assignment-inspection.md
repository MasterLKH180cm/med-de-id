# Moat Assignment Inspection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a read-only `mdid-cli moat assignments --history-path PATH [--role planner|coder|reviewer]` command that inspects latest persisted moat agent assignments without mutating history or launching agents.

**Architecture:** Reuse the existing persisted moat history store and `MoatControlPlaneReport.agent_assignments` projection. The CLI parser mirrors `moat decision-log --role` but prints assignment rows from the latest persisted round only. This is an inspection-only control-plane surface; it must not append rounds, schedule work, create cron jobs, crawl data, or launch Planner/Coder/Reviewer agents.

**Tech Stack:** Rust workspace, `mdid-cli`, `mdid-runtime::moat_history::LocalMoatHistoryStore`, existing `mdid_application::MoatAgentAssignment`, cargo integration tests.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `MoatAssignmentsCommand { history_path, role }`.
  - Parse `moat assignments --history-path PATH [--role planner|coder|reviewer]`.
  - Add `run_moat_assignments` using `LocalMoatHistoryStore::open_existing` and `entries().last()`.
  - Add deterministic line-oriented output.
  - Keep command read-only.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add CLI integration tests for success, role filtering, missing history non-creation, missing flag, unknown role, and read-only behavior.
  - Update the shared usage string.
- Modify: `README.md`
  - Document the new read-only assignment inspection command and explicitly say it does not launch agents or mutate history.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Add the new persisted assignment inspection surface to shipped/read-only control-plane capabilities.

### Task 1: CLI Assignment Inspection Command

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`

- [ ] **Step 1: Write failing parser/usage tests**

Add these tests to `crates/mdid-cli/tests/moat_cli.rs` near the existing `moat_decision_log` tests:

```rust
#[test]
fn cli_requires_history_path_for_moat_assignments() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "assignments"])
        .output()
        .expect("failed to run mdid-cli moat assignments without history path");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("missing required flag: --history-path\n{}\n", USAGE)
    );
}

#[test]
fn cli_rejects_unknown_moat_assignments_role() {
    let history_path = unique_history_path("assignments-unknown-role");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--role",
            "operator",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with unknown role");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("unknown moat assignments role: operator\n{}\n", USAGE)
    );
    assert!(!history_path.exists());
}
```

Update `USAGE` to include:

```text
moat assignments --history-path PATH [--role planner|coder|reviewer]
```

- [ ] **Step 2: Run tests to verify RED**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli moat_assignments -- --nocapture
```

Expected: FAIL because `moat assignments` is still an unknown command or the usage string does not include it.

- [ ] **Step 3: Add minimal parser support**

In `crates/mdid-cli/src/main.rs`, add:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
struct MoatAssignmentsCommand {
    history_path: String,
    role: Option<AgentRole>,
}
```

Add enum variant:

```rust
MoatAssignments(MoatAssignmentsCommand),
```

Add parse arm after `decision-log`:

```rust
[moat, assignments, rest @ ..] if moat == "moat" && assignments == "assignments" => Ok(
    CliCommand::MoatAssignments(parse_moat_assignments_command(rest)?),
),
```

Add main match arm:

```rust
Ok(CliCommand::MoatAssignments(command)) => {
    if let Err(error) = run_moat_assignments(&command) {
        exit_with_error(error);
    }
}
```

Add parser:

```rust
fn parse_moat_assignments_command(args: &[String]) -> Result<MoatAssignmentsCommand, String> {
    let mut history_path = None;
    let mut role = None;
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
            flag => return Err(format!("unknown flag: {flag}")),
        }

        index += 2;
    }

    Ok(MoatAssignmentsCommand {
        history_path: history_path
            .ok_or_else(|| "missing required flag: --history-path".to_string())?,
        role,
    })
}

fn parse_moat_assignments_role_filter(value: &str) -> Result<AgentRole, String> {
    match value {
        "planner" => Ok(AgentRole::Planner),
        "coder" => Ok(AgentRole::Coder),
        "reviewer" => Ok(AgentRole::Reviewer),
        other => Err(format!("unknown moat assignments role: {other}")),
    }
}
```

Add temporary minimal runner so parser tests compile and fail only where behavior is not implemented:

```rust
fn run_moat_assignments(command: &MoatAssignmentsCommand) -> Result<(), String> {
    let _ = command;
    Err("moat assignments inspection is not implemented".to_string())
}
```

Update `usage()` to include the new command.

- [ ] **Step 4: Run parser tests to verify GREEN**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli moat_assignments -- --nocapture
```

Expected: PASS for missing flag and unknown role tests if only those tests exist so far.

### Task 2: Read-Only Persisted Assignment Output

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`

- [ ] **Step 1: Write failing behavior tests**

Add these tests to `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn cli_reports_latest_moat_assignments_from_persisted_history() {
    let history_path = unique_history_path("assignments-success");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--review-loops", "0", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history");
    assert!(seed.status.success(), "stderr was: {}", String::from_utf8_lossy(&seed.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "assignments", "--history-path", history_path_arg])
        .output()
        .expect("failed to run mdid-cli moat assignments");

    assert!(output.status.success(), "stderr was: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "moat assignments\n",
            "assignment_entries=1\n",
            "assignment=reviewer|review|Review|review|<none>\n",
        )
    );

    let history = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "history", "--history-path", history_path_arg])
        .output()
        .expect("failed to inspect moat history after assignments");
    assert!(history.status.success(), "stderr was: {}", String::from_utf8_lossy(&history.stderr));
    assert!(String::from_utf8_lossy(&history.stdout).contains("entries=1\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn cli_filters_moat_assignments_by_role() {
    let history_path = unique_history_path("assignments-role-filter");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--review-loops", "0", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history");
    assert!(seed.status.success(), "stderr was: {}", String::from_utf8_lossy(&seed.stderr));

    let planner_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "assignments", "--history-path", history_path_arg, "--role", "planner"])
        .output()
        .expect("failed to run mdid-cli moat assignments with planner filter");
    assert!(planner_output.status.success(), "stderr was: {}", String::from_utf8_lossy(&planner_output.stderr));
    assert_eq!(
        String::from_utf8_lossy(&planner_output.stdout),
        concat!("moat assignments\n", "assignment_entries=0\n")
    );

    let reviewer_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "assignments", "--history-path", history_path_arg, "--role", "reviewer"])
        .output()
        .expect("failed to run mdid-cli moat assignments with reviewer filter");
    assert!(reviewer_output.status.success(), "stderr was: {}", String::from_utf8_lossy(&reviewer_output.stderr));
    assert!(String::from_utf8_lossy(&reviewer_output.stdout).contains("assignment_entries=1\n"));
    assert!(String::from_utf8_lossy(&reviewer_output.stdout).contains("assignment=reviewer|review|"));

    cleanup_history_path(&history_path);
}

#[test]
fn cli_moat_assignments_requires_existing_history_without_creating_file() {
    let history_path = unique_history_path("assignments-missing-history");
    assert!(!history_path.exists());

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with missing history");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("moat history file does not exist:"));
    assert!(!history_path.exists());
}
```

- [ ] **Step 2: Run tests to verify RED**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli moat_assignments -- --nocapture
```

Expected: FAIL because `run_moat_assignments` returns `moat assignments inspection is not implemented`.

- [ ] **Step 3: Implement read-only latest-history inspection**

Replace the temporary runner in `crates/mdid-cli/src/main.rs` with:

```rust
fn run_moat_assignments(command: &MoatAssignmentsCommand) -> Result<(), String> {
    let store = LocalMoatHistoryStore::open_existing(&command.history_path)
        .map_err(|error| error.to_string())?;
    let latest = store
        .entries()
        .last()
        .ok_or_else(|| "no persisted moat rounds to inspect; run moat round first".to_string())?;

    let assignments: Vec<&MoatAgentAssignment> = latest
        .report
        .control_plane
        .agent_assignments
        .iter()
        .filter(|assignment| command.role.is_none_or(|role| assignment.role == role))
        .collect();

    println!("moat assignments");
    println!("assignment_entries={}", assignments.len());
    for assignment in assignments {
        println!(
            "assignment={}|{}|{}|{}|{}",
            format_agent_role(assignment.role),
            assignment.node_id,
            assignment.title,
            format_moat_task_kind(assignment.kind),
            assignment.spec_ref.as_deref().unwrap_or("<none>")
        );
    }

    Ok(())
}
```

If `Option::is_none_or` is unavailable for the project compiler, use this equivalent filter:

```rust
.filter(|assignment| match command.role {
    Some(role) => assignment.role == role,
    None => true,
})
```

- [ ] **Step 4: Run tests to verify GREEN**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli moat_assignments -- --nocapture
```

Expected: PASS.

### Task 3: Documentation and Traceability

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
- Modify: `docs/superpowers/plans/2026-04-26-med-de-id-moat-assignment-inspection.md`

- [ ] **Step 1: Update README**

Add this bullet near the existing moat CLI/read-only control-plane commands:

```markdown
- `mdid-cli moat assignments --history-path PATH [--role planner|coder|reviewer]` inspects the latest persisted read-only Planner/Coder/Reviewer assignment projection and prints deterministic `assignment=<role>|<node_id>|<title>|<kind>|<spec_ref>` rows. Persisted `node_id`, `title`, and `spec_ref` fields are escaped for pipe-delimited output (`\\` as `\\\\`, `|` as `\\|`, newline as `\\n`, carriage return as `\\r`). It uses existing moat history only, never creates missing history files, never appends rounds, never schedules work, never launches agents, and never creates cron jobs.
```

- [ ] **Step 2: Update moat-loop design spec**

In `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`, add this capability statement near the control-plane/decision-log inspection section:

```markdown
- `mdid-cli moat assignments --history-path PATH [--role planner|coder|reviewer]` is a read-only latest-round inspection surface for persisted `agent_assignments`. It prints bounded assignment rows for operators and future SDD handoff tooling; persisted `node_id`, `title`, and `spec_ref` fields are escaped in pipe-delimited rows. It does not mutate history, schedule rounds, launch agents, crawl data, open PRs, or create cron jobs.
```

- [ ] **Step 3: Run formatting and focused tests**

Run:

```bash
source "$HOME/.cargo/env"
cargo fmt --check
cargo test -p mdid-cli --test moat_cli moat_assignments -- --nocapture
cargo test -p mdid-cli --test moat_cli
```

Expected: all PASS.

- [ ] **Step 4: Commit**

Run:

```bash
git status --short
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs README.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-26-med-de-id-moat-assignment-inspection.md crates/mdid-domain/tests/moat_agent_memory.rs
git commit -m "feat: inspect moat agent assignments"
```

Expected: commit succeeds on `feature/moat-loop-autonomy`.

## Self-Review

- Spec coverage: The plan covers a read-only persisted assignment inspection command, role filtering, error handling, non-mutating history behavior, docs/spec updates, and focused verification.
- Placeholder scan: No TBD/TODO/fill-in-later placeholders remain.
- Type consistency: The command type, parser, enum variant, runner, and tests consistently use `MoatAssignmentsCommand`, `moat assignments`, `assignment_entries`, and `assignment=` rows.
- 2026-04-26 quality follow-up: Assignment rows now escape persisted string fields (`node_id`, `title`, `spec_ref`) before pipe-delimited printing, with regression coverage for pipe, newline, carriage return, and backslash values.
