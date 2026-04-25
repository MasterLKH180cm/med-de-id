# Moat Decision Log Role Filter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded read-only `--role` filter to `mdid-cli moat decision-log` so operators can inspect decisions by Planner/Coder/Reviewer role without appending moat history.

**Architecture:** Keep the feature inside the existing CLI decision-log inspection path. Replace the decision-log command payload with a small command struct containing `history_path` and optional `role`, parse role values with the same role names already rendered by `format_agent_role`, and filter only the in-memory latest decision list before printing deterministic output.

**Tech Stack:** Rust workspace, `mdid-cli`, `mdid-runtime` history store, Cargo integration tests with `assert_cmd` and `predicates`.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `MoatDecisionLogCommand { history_path, role }`, parse `--role planner|coder|reviewer`, and filter rendered decision rows.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add CLI coverage for role filtering and invalid role rejection.
- Modify: `README.md`
  - Document optional `--role` usage for read-only decision-log inspection.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Truth-sync shipped status with role-filtered decision-log inspection.

---

### Task 1: CLI role filter for decision logs

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`

- [ ] **Step 1: Write failing CLI tests**

Append these tests to `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn moat_decision_log_filters_by_role() {
    let temp = tempfile::tempdir().expect("tempdir");
    let history_path = temp.path().join("moat-history.json");

    Command::cargo_bin("mdid-cli")
        .expect("binary exists")
        .args([
            "moat",
            "round",
            "--history-path",
            history_path.to_str().expect("utf8 path"),
        ])
        .assert()
        .success();

    let assert = Command::cargo_bin("mdid-cli")
        .expect("binary exists")
        .args([
            "moat",
            "decision-log",
            "--history-path",
            history_path.to_str().expect("utf8 path"),
            "--role",
            "reviewer",
        ])
        .assert();

    assert
        .success()
        .stdout(predicate::str::contains("decision_log_entries=1"))
        .stdout(predicate::str::contains("decision=reviewer|"))
        .stdout(predicate::str::contains("decision=planner|").not())
        .stdout(predicate::str::contains("decision=coder|").not());
}

#[test]
fn moat_decision_log_rejects_unknown_role_filter() {
    let temp = tempfile::tempdir().expect("tempdir");
    let history_path = temp.path().join("moat-history.json");

    let assert = Command::cargo_bin("mdid-cli")
        .expect("binary exists")
        .args([
            "moat",
            "decision-log",
            "--history-path",
            history_path.to_str().expect("utf8 path"),
            "--role",
            "operator",
        ])
        .assert();

    assert
        .failure()
        .stderr(predicate::str::contains("unknown moat decision-log role: operator"));
}
```

- [ ] **Step 2: Run RED tests**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli decision_log -- --nocapture
```

Expected: FAIL because `--role` is currently treated as an unknown or unexpected flag.

- [ ] **Step 3: Implement minimal CLI role parser and filter**

In `crates/mdid-cli/src/main.rs`, add this command struct near other moat command structs:

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct MoatDecisionLogCommand {
    history_path: String,
    role: Option<AgentRole>,
}
```

Change `CliCommand::MoatDecisionLog(String)` to:

```rust
MoatDecisionLog(MoatDecisionLogCommand),
```

Change the main match arm to:

```rust
Ok(CliCommand::MoatDecisionLog(command)) => {
    if let Err(error) = run_moat_decision_log(&command) {
        exit_with_error(error);
    }
}
```

Replace the parser arm with:

```rust
[moat, decision_log, rest @ ..] if moat == "moat" && decision_log == "decision-log" => {
    Ok(CliCommand::MoatDecisionLog(parse_moat_decision_log_command(rest)?))
}
```

Add this parser near other moat parsers:

```rust
fn parse_moat_decision_log_command(args: &[String]) -> Result<MoatDecisionLogCommand, String> {
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
                role = Some(parse_agent_role_filter(value)?);
            }
            flag => return Err(format!("unknown flag: {flag}")),
        }

        index += 2;
    }

    Ok(MoatDecisionLogCommand {
        history_path: history_path
            .ok_or_else(|| "missing required flag: --history-path".to_string())?,
        role,
    })
}

fn parse_agent_role_filter(value: &str) -> Result<AgentRole, String> {
    match value {
        "planner" => Ok(AgentRole::Planner),
        "coder" => Ok(AgentRole::Coder),
        "reviewer" => Ok(AgentRole::Reviewer),
        other => Err(format!("unknown moat decision-log role: {other}")),
    }
}
```

Change the runner signature and body to filter before printing:

```rust
fn run_moat_decision_log(command: &MoatDecisionLogCommand) -> Result<(), String> {
    let store = LocalMoatHistoryStore::open_existing(&command.history_path)
        .map_err(|error| format!("failed to open moat history store: {error}"))?;
    let latest = store.entries().last().ok_or_else(|| {
        "moat history is empty; run `mdid-cli moat round --history-path <path>` first".to_string()
    })?;
    let decisions: Vec<_> = latest
        .report
        .control_plane
        .memory
        .decisions
        .iter()
        .filter(|decision| command.role.is_none_or(|role| decision.author_role == role))
        .collect();

    println!("decision_log_entries={}", decisions.len());
    for decision in decisions {
        println!(
            "decision={}|{}|{}",
            format_agent_role(decision.author_role),
            decision.summary,
            decision.rationale
        );
    }

    Ok(())
}
```

- [ ] **Step 4: Run GREEN tests**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli decision_log -- --nocapture
cargo test -p mdid-cli --test moat_cli
```

Expected: PASS.

- [ ] **Step 5: Update docs and spec**

Update `README.md` moat CLI documentation with:

```markdown
- `cargo run -p mdid-cli -- moat decision-log --history-path ./moat-history.json --role reviewer` filters the latest persisted decision log to one agent role (`planner`, `coder`, or `reviewer`) without running or appending a round.
```

Update `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md` shipped foundation slice to mention optional role-filtered `moat decision-log --history-path PATH --role reviewer` inspection.

- [ ] **Step 6: Run final verification**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli
cargo test -p mdid-runtime --test moat_runtime
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs README.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-26-med-de-id-moat-decision-log-role-filter.md
git commit -m "feat: filter moat decision logs by role"
```
