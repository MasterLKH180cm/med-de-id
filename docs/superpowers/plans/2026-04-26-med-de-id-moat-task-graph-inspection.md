# Moat Task Graph Inspection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a read-only `mdid-cli moat task-graph --history-path PATH [--role planner|coder|reviewer] [--state pending|ready|in_progress|completed|blocked]` command that inspects the latest persisted moat task graph without mutating history or launching agents.

**Architecture:** Reuse `LocalMoatHistoryStore::open_existing` and the latest persisted `MoatRoundReport.control_plane.task_graph`. The CLI mirrors existing read-only inspection commands and prints deterministic escaped pipe-delimited rows for each matching task node. This advances the autonomous multi-agent control plane by making task graph state inspectable while explicitly avoiding background daemons, live crawling, agent dispatch, cron jobs, or repository mutation.

**Tech Stack:** Rust workspace, `mdid-cli`, `mdid-domain` task graph models, `mdid-runtime::moat_history::LocalMoatHistoryStore`, cargo integration tests.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `MoatTaskGraphCommand { history_path, role, state }`.
  - Parse `moat task-graph --history-path PATH [--role planner|coder|reviewer] [--state pending|ready|in_progress|completed|blocked]`.
  - Add `run_moat_task_graph` using `LocalMoatHistoryStore::open_existing` and latest entry only.
  - Print deterministic line-oriented escaped rows.
  - Keep command read-only.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add CLI integration tests for required history path, unknown role, unknown state, successful latest graph output, role filtering, state filtering, and missing history non-creation.
  - Update the shared usage string.
- Modify: `README.md`
  - Document the read-only task graph inspection command and state explicitly that it does not launch agents or mutate history.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Add the persisted task graph inspection surface to shipped/read-only control-plane capabilities.

### Task 1: CLI Task Graph Inspection Command

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`

- [ ] **Step 1: Write failing CLI tests**

Add tests to `crates/mdid-cli/tests/moat_cli.rs` near the existing moat inspection tests:

```rust
#[test]
fn cli_requires_history_path_for_moat_task_graph() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "task-graph"])
        .output()
        .expect("failed to run mdid-cli moat task-graph without history path");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("missing required flag: --history-path\n{}\n", USAGE)
    );
}

#[test]
fn cli_rejects_unknown_moat_task_graph_role_without_creating_history() {
    let history_path = unique_history_path("task-graph-unknown-role");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--role",
            "operator",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with unknown role");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("unknown moat task-graph role: operator\n{}\n", USAGE)
    );
    assert!(!history_path.exists());
}

#[test]
fn cli_rejects_unknown_moat_task_graph_state_without_creating_history() {
    let history_path = unique_history_path("task-graph-unknown-state");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--state",
            "waiting",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with unknown state");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("unknown moat task-graph state: waiting\n{}\n", USAGE)
    );
    assert!(!history_path.exists());
}
```

Update `USAGE` to include:

```text
moat task-graph --history-path PATH [--role planner|coder|reviewer] [--state pending|ready|in_progress|completed|blocked]
```

- [ ] **Step 2: Run tests to verify RED**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli task_graph -- --nocapture
```

Expected: FAIL because `moat task-graph` is still an unknown command or usage does not include it.

- [ ] **Step 3: Add parser support and minimal runner**

In `crates/mdid-cli/src/main.rs`, import `MoatTaskNodeState` if not already imported. Add:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
struct MoatTaskGraphCommand {
    history_path: String,
    role: Option<AgentRole>,
    state: Option<MoatTaskNodeState>,
}
```

Add enum variant:

```rust
MoatTaskGraph(MoatTaskGraphCommand),
```

Add parse arm after `assignments`:

```rust
[moat, task_graph, rest @ ..] if moat == "moat" && task_graph == "task-graph" => Ok(
    CliCommand::MoatTaskGraph(parse_moat_task_graph_command(rest)?),
),
```

Add main match arm:

```rust
Ok(CliCommand::MoatTaskGraph(command)) => {
    if let Err(error) = run_moat_task_graph(&command) {
        exit_with_error(error);
    }
}
```

Add parser:

```rust
fn parse_moat_task_graph_command(args: &[String]) -> Result<MoatTaskGraphCommand, String> {
    let mut history_path = None;
    let mut role = None;
    let mut state = None;
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
                role = Some(parse_moat_task_graph_role_filter(value)?);
            }
            "--state" => {
                let value = required_flag_value(args, index, "--state", false)?;
                if state.is_some() {
                    return Err(duplicate_flag_error("--state"));
                }
                state = Some(parse_moat_task_graph_state_filter(value)?);
            }
            flag => return Err(format!("unknown flag: {flag}")),
        }

        index += 2;
    }

    Ok(MoatTaskGraphCommand {
        history_path: history_path
            .ok_or_else(|| "missing required flag: --history-path".to_string())?,
        role,
        state,
    })
}

fn parse_moat_task_graph_role_filter(value: &str) -> Result<AgentRole, String> {
    match value {
        "planner" => Ok(AgentRole::Planner),
        "coder" => Ok(AgentRole::Coder),
        "reviewer" => Ok(AgentRole::Reviewer),
        other => Err(format!("unknown moat task-graph role: {other}")),
    }
}

fn parse_moat_task_graph_state_filter(value: &str) -> Result<MoatTaskNodeState, String> {
    match value {
        "pending" => Ok(MoatTaskNodeState::Pending),
        "ready" => Ok(MoatTaskNodeState::Ready),
        "in_progress" => Ok(MoatTaskNodeState::InProgress),
        "completed" => Ok(MoatTaskNodeState::Completed),
        "blocked" => Ok(MoatTaskNodeState::Blocked),
        other => Err(format!("unknown moat task-graph state: {other}")),
    }
}

fn run_moat_task_graph(command: &MoatTaskGraphCommand) -> Result<(), String> {
    let _ = command;
    Err("moat task graph inspection is not implemented".to_string())
}
```

Update `usage()` to include the new command.

- [ ] **Step 4: Run parser tests to verify GREEN**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli task_graph -- --nocapture
```

Expected: PASS for parser error tests.

- [ ] **Step 5: Write failing behavior tests for persisted latest graph output and filters**

Add these tests to `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn cli_prints_latest_persisted_moat_task_graph_rows() {
    let history_path = unique_history_path("task-graph-output");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history");
    assert!(seed.status.success(), "stderr was: {}", String::from_utf8_lossy(&seed.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "task-graph", "--history-path", history_path_arg])
        .output()
        .expect("failed to run mdid-cli moat task-graph");

    assert!(output.status.success(), "stderr was: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("moat task graph\n"));
    assert!(stdout.contains("node=planner|market-scan|Market scan|market_scan|completed|<none>|<none>\n"));
    assert!(stdout.contains("node=coder|implementation|Implement selected moat spec|implementation|completed|spec-planning|moat-spec/workflow-audit\n"));
    assert!(stdout.contains("node=reviewer|review|Review implementation and moat impact|review|completed|implementation|moat-spec/workflow-audit\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn cli_filters_moat_task_graph_by_role_and_state() {
    let history_path = unique_history_path("task-graph-filters");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--review-loops",
            "0",
            "--history-path",
            history_path_arg,
        ])
        .output()
        .expect("failed to seed stopped moat history");
    assert!(seed.status.success(), "stderr was: {}", String::from_utf8_lossy(&seed.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--role",
            "reviewer",
            "--state",
            "ready",
        ])
        .output()
        .expect("failed to run filtered mdid-cli moat task-graph");

    assert!(output.status.success(), "stderr was: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "moat task graph\n",
            "node=reviewer|review|Review implementation and moat impact|review|ready|implementation|moat-spec/workflow-audit\n",
        )
    );

    cleanup_history_path(&history_path);
}

#[test]
fn cli_moat_task_graph_missing_history_does_not_create_file() {
    let history_path = unique_history_path("task-graph-missing-history");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with missing history");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("moat history file does not exist"));
    assert!(!history_path.exists());
}
```

- [ ] **Step 6: Run behavior tests to verify RED**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli task_graph -- --nocapture
```

Expected: FAIL because the minimal runner returns `moat task graph inspection is not implemented`.

- [ ] **Step 7: Implement read-only persisted task graph inspection**

Replace `run_moat_task_graph` with:

```rust
fn run_moat_task_graph(command: &MoatTaskGraphCommand) -> Result<(), String> {
    let store = LocalMoatHistoryStore::open_existing(&command.history_path)
        .map_err(|error| error.to_string())?;
    let latest = store
        .entries()
        .last()
        .ok_or_else(|| "moat history is empty".to_string())?;

    println!("moat task graph");
    for node in latest.report.control_plane.task_graph.nodes.iter().filter(|node| {
        command.role.map_or(true, |role| node.role == role)
            && command.state.map_or(true, |state| node.state == state)
    }) {
        let dependencies = if node.depends_on.is_empty() {
            "<none>".to_string()
        } else {
            node.depends_on
                .iter()
                .map(|dependency| escape_pipe_field(dependency))
                .collect::<Vec<_>>()
                .join(",")
        };
        let spec_ref = node
            .spec_ref
            .as_deref()
            .map(escape_pipe_field)
            .unwrap_or_else(|| "<none>".to_string());
        println!(
            "node={}|{}|{}|{}|{}|{}|{}",
            agent_role_label(node.role),
            escape_pipe_field(&node.node_id),
            escape_pipe_field(&node.title),
            moat_task_kind_label(node.kind),
            moat_task_state_label(node.state),
            dependencies,
            spec_ref,
        );
    }

    Ok(())
}
```

Add helpers near existing label helpers:

```rust
fn moat_task_kind_label(kind: MoatTaskNodeKind) -> &'static str {
    match kind {
        MoatTaskNodeKind::MarketScan => "market_scan",
        MoatTaskNodeKind::CompetitorAnalysis => "competitor_analysis",
        MoatTaskNodeKind::LockInAnalysis => "lock_in_analysis",
        MoatTaskNodeKind::StrategyGeneration => "strategy_generation",
        MoatTaskNodeKind::SpecPlanning => "spec_planning",
        MoatTaskNodeKind::Implementation => "implementation",
        MoatTaskNodeKind::Review => "review",
        MoatTaskNodeKind::Evaluation => "evaluation",
    }
}

fn moat_task_state_label(state: MoatTaskNodeState) -> &'static str {
    match state {
        MoatTaskNodeState::Pending => "pending",
        MoatTaskNodeState::Ready => "ready",
        MoatTaskNodeState::InProgress => "in_progress",
        MoatTaskNodeState::Completed => "completed",
        MoatTaskNodeState::Blocked => "blocked",
    }
}
```

- [ ] **Step 8: Run task graph tests to verify GREEN**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli task_graph -- --nocapture
```

Expected: PASS.

- [ ] **Step 9: Update docs and spec**

In `README.md`, add under moat commands:

```markdown
- `mdid-cli moat task-graph --history-path PATH [--role planner|coder|reviewer] [--state pending|ready|in_progress|completed|blocked]` inspects the latest persisted task graph and prints `node=<role>|<node_id>|<title>|<kind>|<state>|<depends_on>|<spec_ref>` rows. It is read-only: it requires an existing history file, does not append rounds, does not launch agents, does not create cron jobs, and does not mutate the repository.
```

In `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`, update shipped foundation status to include the new read-only task graph inspection surface.

- [ ] **Step 10: Run relevant tests and commit**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli task_graph -- --nocapture
cargo test -p mdid-cli --test moat_cli moat -- --nocapture
cargo test -p mdid-runtime -p mdid-application -p mdid-domain
```

Expected: PASS.

Commit:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs README.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-26-med-de-id-moat-task-graph-inspection.md
git commit -m "feat: inspect persisted moat task graph"
```

## Self-Review

- Spec coverage: The plan implements a bounded read-only task graph inspection surface, role/state filters, missing-history non-creation, documentation, and spec sync. It does not implement background scheduling or live agent launch, matching the conservative next slice.
- Placeholder scan: No TBD/TODO/placeholders are present; code snippets and commands are concrete.
- Type consistency: `MoatTaskGraphCommand`, `MoatTaskNodeState`, `moat_task_kind_label`, and `moat_task_state_label` are consistently named across tests and implementation steps.
