# Persisted Moat Control Plane Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a read-only `mdid-cli moat control-plane --history-path PATH` surface that inspects the latest persisted moat round without running or appending a new round.

**Architecture:** Keep the existing deterministic sample `moat control-plane` behavior for no-history invocations, and add a persisted-history branch in `mdid-cli` that opens `LocalMoatHistoryStore::open_existing`, reads the latest entry, and prints the same control-plane snapshot fields with persisted context. This is a bounded local operator inspection surface only; it does not create cron jobs, start a daemon, crawl live data, append history, or launch coding agents.

**Tech Stack:** Rust workspace, `mdid-cli`, `mdid-runtime::moat_history::LocalMoatHistoryStore`, CLI integration tests with `std::process::Command`.

---

## File Structure

- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add RED tests for persisted `moat control-plane --history-path` inspection and missing-history failure.
  - Update the `USAGE` string to document optional `--history-path`.
- Modify: `crates/mdid-cli/src/main.rs`
  - Replace `CliCommand::MoatControlPlane(MoatRoundOverrides)` with a `MoatControlPlaneCommand` containing `overrides` and optional `history_path`.
  - Add parsing for `--history-path` on `moat control-plane`.
  - Split printing through a helper so sample and persisted snapshots stay identical where intended.
  - Add read-only persisted-history inspection using `LocalMoatHistoryStore::open_existing`.
- Modify: `README.md`
  - Document the persisted control-plane inspection command and its non-daemon/read-only boundary.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Truth-sync shipped-foundation text to include read-only persisted control-plane inspection while preserving full Autonomous Multi-Agent System future-work boundaries.

---

### Task 1: Add persisted control-plane CLI tests

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`

- [ ] **Step 1: Write the failing tests**

Add these tests near the existing control-plane/history tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn cli_runs_moat_control_plane_from_latest_persisted_history_round() {
    let history_path = unique_history_path("control-plane-history");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history");
    assert!(
        seed.status.success(),
        "stderr was: {}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let store = LocalMoatHistoryStore::open_existing(&history_path).expect("history should exist");
    let latest_round_id = store
        .summary()
        .latest_round_id
        .expect("seeded history should expose latest round id");
    drop(store);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "control-plane", "--history-path", history_path_arg])
        .output()
        .expect("failed to run persisted moat control-plane inspection");

    assert!(
        output.status.success(),
        "stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!(
            concat!(
                "moat control plane snapshot\n",
                "source=history\n",
                "latest_round_id={}\n",
                "history_path={}\n",
                "ready_nodes=<none>\n",
                "latest_decision_summary=review approved bounded moat round\n",
                "improvement_delta=8\n",
                "task_states=market_scan:completed,competitor_analysis:completed,lockin_analysis:completed,strategy_generation:completed,spec_planning:completed,implementation:completed,review:completed,evaluation:completed\n",
            ),
            latest_round_id,
            history_path.display()
        )
    );

    cleanup_history_path(&history_path);
}

#[test]
fn cli_control_plane_history_requires_existing_history_file() {
    let history_path = unique_history_path("control-plane-missing-history");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "control-plane", "--history-path", history_path_arg])
        .output()
        .expect("failed to run persisted moat control-plane inspection against missing history");

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed to open moat history store"));
    assert!(stderr.contains("moat history file does not exist"));
    assert!(!history_path.exists());

    cleanup_history_path(&history_path);
}
```

Update the `USAGE` constant at the top of the test file to include `--history-path PATH` for `moat control-plane`:

```rust
const USAGE: &str = "usage: mdid-cli [status | moat round [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false] [--history-path PATH] | moat control-plane [--history-path PATH] [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false] | moat history --history-path PATH | moat continue --history-path PATH [--improvement-threshold N] | moat schedule-next --history-path PATH [--improvement-threshold N] | moat export-specs --history-path PATH --output-dir DIR | moat export-plans --history-path PATH --output-dir DIR]";
```

- [ ] **Step 2: Run RED tests**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli control_plane -- --nocapture
```

Expected: FAIL. The new `--history-path` test should fail because `moat control-plane` currently rejects `--history-path` as an unknown flag.

- [ ] **Step 3: Commit is not allowed yet**

Do not commit after RED. Continue to Task 2.

---

### Task 2: Implement read-only persisted control-plane inspection

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`

- [ ] **Step 1: Add command struct and enum variant**

Add this struct after `MoatRoundCommand`:

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct MoatControlPlaneCommand {
    overrides: MoatRoundOverrides,
    history_path: Option<String>,
}
```

Change the enum variant from:

```rust
MoatControlPlane(MoatRoundOverrides),
```

to:

```rust
MoatControlPlane(MoatControlPlaneCommand),
```

- [ ] **Step 2: Parse `moat control-plane --history-path`**

Replace the `moat control-plane` arm in `parse_command` with:

```rust
[moat, control_plane, rest @ ..] if moat == "moat" && control_plane == "control-plane" => {
    Ok(CliCommand::MoatControlPlane(parse_moat_control_plane_command(rest)?))
}
```

Add this parser after `parse_moat_round_command`:

```rust
fn parse_moat_control_plane_command(args: &[String]) -> Result<MoatControlPlaneCommand, String> {
    let (overrides, history_path) = parse_moat_round_overrides(args, true)?;
    Ok(MoatControlPlaneCommand {
        overrides,
        history_path,
    })
}
```

- [ ] **Step 3: Route persisted inspections without mutating history**

Change the `main` match arm from:

```rust
Ok(CliCommand::MoatControlPlane(overrides)) => run_moat_control_plane(&overrides),
```

to:

```rust
Ok(CliCommand::MoatControlPlane(command)) => {
    if let Err(error) = run_moat_control_plane(&command) {
        exit_with_error(error);
    }
}
```

Replace `run_moat_control_plane` with:

```rust
fn run_moat_control_plane(command: &MoatControlPlaneCommand) -> Result<(), String> {
    if let Some(history_path) = &command.history_path {
        return run_persisted_moat_control_plane(history_path);
    }

    let report = sample_round_report(&command.overrides);
    print_control_plane_snapshot("sample", None, None, &report);
    Ok(())
}

fn run_persisted_moat_control_plane(history_path: &str) -> Result<(), String> {
    let store = LocalMoatHistoryStore::open_existing(history_path)
        .map_err(|error| format!("failed to open moat history store: {error}"))?;
    let latest = store.entries().last().ok_or_else(|| {
        "moat history is empty; run `mdid-cli moat round --history-path <path>` first".to_string()
    })?;

    print_control_plane_snapshot(
        "history",
        Some(history_path),
        Some(latest.report.summary.round_id.as_str()),
        &latest.report,
    );
    Ok(())
}

fn print_control_plane_snapshot(
    source: &str,
    history_path: Option<&str>,
    latest_round_id: Option<&str>,
    report: &MoatRoundReport,
) {
    let control_plane = &report.control_plane;
    let ready_nodes = format_ready_nodes(&control_plane.task_graph.ready_node_ids());
    let latest_decision_summary = control_plane
        .memory
        .latest_decision_summary()
        .unwrap_or_else(|| "<none>".to_string());
    let task_states = format_task_states(&control_plane.task_graph.nodes);

    println!("moat control plane snapshot");
    println!("source={source}");
    if let Some(latest_round_id) = latest_round_id {
        println!("latest_round_id={latest_round_id}");
    }
    if let Some(history_path) = history_path {
        println!("history_path={history_path}");
    }
    println!("ready_nodes={ready_nodes}");
    println!("latest_decision_summary={latest_decision_summary}");
    println!(
        "improvement_delta={}",
        control_plane.memory.improvement_delta
    );
    println!("task_states={task_states}");
}
```

- [ ] **Step 4: Update production usage text**

Update the `usage()` return string in `crates/mdid-cli/src/main.rs` to match the test constant:

```rust
"usage: mdid-cli [status | moat round [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false] [--history-path PATH] | moat control-plane [--history-path PATH] [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false] | moat history --history-path PATH | moat continue --history-path PATH [--improvement-threshold N] | moat schedule-next --history-path PATH [--improvement-threshold N] | moat export-specs --history-path PATH --output-dir DIR | moat export-plans --history-path PATH --output-dir DIR]"
```

- [ ] **Step 5: Run GREEN tests**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli control_plane -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Run broader CLI tests**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli
cargo test -p mdid-cli
```

Expected: PASS.

---

### Task 3: Truth-sync docs and run final verification

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`

- [ ] **Step 1: Update README moat-loop CLI docs**

In `README.md`, add this paragraph near the existing moat control-plane/history docs:

```markdown
Inspect the latest persisted moat control-plane snapshot with:

```bash
cargo run -p mdid-cli -- moat control-plane --history-path .mdid/moat-history.json
```

This read-only local operator surface reports the latest persisted task states, ready-node visibility, decision-memory summary, and improvement delta. It does not schedule work, append rounds, start a daemon, crawl the web, or automate code changes.
```

- [ ] **Step 2: Update moat-loop design spec shipped-foundation boundary**

In `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`, update the shipped-foundation section to include:

```markdown
- a bounded operator-facing `mdid-cli moat control-plane` runner over deterministic sample inputs, plus read-only `--history-path PATH` inspection of the latest persisted control-plane snapshot, including task states, ready-node visibility, and bounded decision-memory summary
```

Keep these future-work boundaries present:

```markdown
- Planner / Coder / Reviewer role orchestration
- full persistent memory store and decision-log workflow beyond bounded local round-history snapshots
- non-linear task graph persistence and background scheduler/daemon control
- GitFlow PR / release automation
- live market / competitor / lock-in data collection
- continuous improvement loop stopping on resource or improvement thresholds
```

- [x] **Quality review follow-up: reject ambiguous persisted/sample mixed flags**

Quality review found that `mdid-cli moat control-plane --history-path history.json --review-loops 0` parsed successfully while silently ignoring override flags on the persisted-history branch. The implementation now treats `--history-path` as mutually exclusive with control-plane override flags and returns: `cannot combine --history-path with control-plane override flags`. CLI coverage also includes the empty-existing-history error path.

- [x] **Step 3: Run final verification**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli
cargo test -p mdid-cli
cargo test -p mdid-runtime
```

Expected: PASS.

- [ ] **Step 4: Commit**

Run:

```bash
git add crates/mdid-cli/tests/moat_cli.rs crates/mdid-cli/src/main.rs README.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-26-med-de-id-moat-persisted-control-plane.md
git commit -m "feat: inspect persisted moat control plane"
```

---

## Self-Review

- **Spec coverage:** The plan advances the Autonomous Multi-Agent System by adding a bounded persisted control-plane inspection surface. It does not add daemon/cron/background work, live crawling, PR automation, or unrestricted autonomous agents.
- **Placeholder scan:** No TBD/TODO placeholders remain. Every code-changing step includes exact code snippets or exact text to add.
- **Type consistency:** The new `MoatControlPlaneCommand` is used consistently in `CliCommand`, parser, `main`, and runner. The persisted branch uses existing `LocalMoatHistoryStore::open_existing`, `entries().last()`, and `MoatRoundReport` fields.

Autonomous controller choice for this cron run: **Subagent-Driven (recommended)** — execute task-by-task with fresh subagents, spec review first, code-quality review second.
