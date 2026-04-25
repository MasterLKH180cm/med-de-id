# Moat Round History Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a local-first persisted moat round history so the CLI can append bounded round results to disk and inspect prior rounds without introducing live crawling or scheduler behavior.

**Architecture:** Keep persistence narrow and reusable by adding a small file-backed history store in `mdid-runtime` that serializes `MoatRoundReport` snapshots plus a recorded-at timestamp to JSON. Then extend `mdid-cli` with an optional `--history-path` write path for `moat round` and a new `moat history` read surface that prints a deterministic text summary while honestly failing on missing history files, while truth-syncing README/spec/docs to the new bounded contract.

**Tech Stack:** Rust workspace, Cargo, `serde`, `serde_json`, `chrono`, `mdid-runtime`, `mdid-cli`, markdown docs.

---

## Scope note

This slice adds:
- a local JSON-backed moat round history store
- optional round persistence from `mdid-cli moat round --history-path <path>`
- a bounded `mdid-cli moat history --history-path <path>` summary surface
- README/spec/plan truth-sync for the persisted-history contract

This slice does **not** add:
- background scheduling
- live market crawling
- autonomous PR/release automation
- browser or desktop moat history UI
- cloud persistence or multi-user coordination

## File structure

**Create:**
- `crates/mdid-runtime/src/moat_history.rs`
- `crates/mdid-runtime/tests/moat_history.rs`

**Modify:**
- `crates/mdid-runtime/Cargo.toml`
- `crates/mdid-runtime/src/lib.rs`
- `crates/mdid-runtime/src/moat.rs`
- `crates/mdid-cli/src/main.rs`
- `crates/mdid-cli/tests/moat_cli.rs`
- `README.md`
- `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
- `docs/superpowers/plans/2026-04-25-med-de-id-moat-round-history.md`

---

### Task 1: Add reusable runtime moat history persistence

**Files:**
- Create: `crates/mdid-runtime/src/moat_history.rs`
- Create: `crates/mdid-runtime/tests/moat_history.rs`
- Modify: `crates/mdid-runtime/Cargo.toml`
- Modify: `crates/mdid-runtime/src/lib.rs`
- Modify: `crates/mdid-runtime/src/moat.rs`

- [ ] **Step 1: Write the failing runtime history tests**

Create `crates/mdid-runtime/tests/moat_history.rs` with:

```rust
use chrono::{DateTime, Utc};
use mdid_domain::{
    CompetitorProfile, ContinueDecision, LockInReport, MarketMoatSnapshot, MoatStrategy,
    MoatType, ResourceBudget,
};
use mdid_runtime::moat::{run_bounded_round, MoatRoundInput};
use mdid_runtime::moat_history::{LocalMoatHistoryStore, MoatHistorySummary};
use tempfile::tempdir;

fn recorded_at(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .expect("timestamp should parse")
        .with_timezone(&Utc)
}

fn sample_round(tests_passed: bool, review_loops: u8) -> mdid_runtime::moat::MoatRoundReport {
    run_bounded_round(MoatRoundInput {
        market: MarketMoatSnapshot {
            market_id: "healthcare-deid".into(),
            moat_score: 45,
            ..MarketMoatSnapshot::default()
        },
        competitor: CompetitorProfile {
            competitor_id: "comp-1".into(),
            threat_score: 30,
            ..CompetitorProfile::default()
        },
        lock_in: LockInReport {
            lockin_score: 60,
            workflow_dependency_strength: 72,
            ..LockInReport::default()
        },
        strategies: vec![MoatStrategy {
            strategy_id: "workflow-audit".into(),
            title: "Workflow audit moat".into(),
            target_moat_type: MoatType::WorkflowLockIn,
            implementation_cost: 2,
            expected_moat_gain: 8,
            ..MoatStrategy::default()
        }],
        budget: ResourceBudget {
            max_round_minutes: 30,
            max_parallel_tasks: 3,
            max_strategy_candidates: 2,
            max_spec_generations: 1,
            max_implementation_tasks: 1,
            max_review_loops: review_loops,
        },
        improvement_threshold: 3,
        tests_passed,
    })
}

#[test]
fn history_store_appends_round_reports_and_loads_them_in_recorded_order() {
    let tempdir = tempdir().expect("tempdir should exist");
    let path = tempdir.path().join("moat-history.json");
    let mut store = LocalMoatHistoryStore::open(&path).expect("history store should open");

    let approved = sample_round(true, 1);
    let stopped = sample_round(true, 0);

    store
        .append(recorded_at("2026-04-25T16:00:00Z"), approved.clone())
        .expect("first report should persist");
    store
        .append(recorded_at("2026-04-25T17:00:00Z"), stopped.clone())
        .expect("second report should persist");

    let records = store.entries();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].recorded_at, recorded_at("2026-04-25T16:00:00Z"));
    assert_eq!(records[0].report.summary.continue_decision, ContinueDecision::Continue);
    assert_eq!(records[1].recorded_at, recorded_at("2026-04-25T17:00:00Z"));
    assert_eq!(records[1].report.stop_reason.as_deref(), Some("review budget exhausted"));
    assert_ne!(records[0].report.summary.round_id, records[1].report.summary.round_id);
}

#[test]
fn history_summary_reports_latest_best_score_and_improvement_series() {
    let tempdir = tempdir().expect("tempdir should exist");
    let path = tempdir.path().join("moat-history.json");
    let mut store = LocalMoatHistoryStore::open(&path).expect("history store should open");

    store
        .append(recorded_at("2026-04-25T16:00:00Z"), sample_round(true, 1))
        .expect("approved report should persist");
    store
        .append(recorded_at("2026-04-25T17:00:00Z"), sample_round(true, 0))
        .expect("stopped report should persist");

    let summary = store.summary();

    assert_eq!(summary.entry_count, 2);
    assert_eq!(summary.best_moat_score_after, Some(98));
    assert_eq!(summary.improvement_deltas, vec![8, 0]);
    assert_eq!(summary.latest_continue_decision, Some(ContinueDecision::Stop));
    assert_eq!(
        summary.latest_decision_summary.as_deref(),
        Some("implementation stopped before review")
    );
    assert_eq!(summary.latest_stop_reason.as_deref(), Some("review budget exhausted"));
}

#[test]
fn history_summary_is_empty_when_store_has_no_records() {
    let tempdir = tempdir().expect("tempdir should exist");
    let path = tempdir.path().join("moat-history.json");
    let store = LocalMoatHistoryStore::open(&path).expect("history store should open");

    assert_eq!(store.summary(), MoatHistorySummary::default());
}

#[test]
fn history_inspection_requires_an_existing_file() {
    let tempdir = tempdir().expect("tempdir should exist");
    let path = tempdir.path().join("missing-moat-history.json");

    let error = LocalMoatHistoryStore::open_existing(&path)
        .expect_err("inspection should fail when the history file is missing");

    assert!(error.to_string().contains("moat history file does not exist"));
    assert!(!path.exists());
}
```

Also extend `crates/mdid-runtime/tests/moat_runtime.rs` expectations so a bounded round now produces a non-nil `round_id` and a non-nil decision `entry_id`:

```rust
assert_ne!(report.summary.round_id, uuid::Uuid::nil());
assert_eq!(report.control_plane.task_graph.round_id, report.summary.round_id);
assert_eq!(report.control_plane.memory.round_id, report.summary.round_id);
assert_ne!(report.control_plane.memory.decisions[0].entry_id, uuid::Uuid::nil());
```

- [ ] **Step 2: Run the focused runtime tests to verify RED**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-runtime --test moat_history
cargo test -p mdid-runtime --test moat_runtime
```

Expected: FAIL because `mdid-runtime` does not yet expose `moat_history`, does not serialize `MoatRoundReport`, and still stamps bounded rounds with nil IDs.

- [ ] **Step 3: Write the minimal runtime implementation**

Update `crates/mdid-runtime/Cargo.toml` so runtime code can serialize JSON and timestamp records:

```toml
[dependencies]
axum.workspace = true
chrono.workspace = true
mdid-application = { path = "../mdid-application" }
mdid-domain = { path = "../mdid-domain" }
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
uuid.workspace = true

[dev-dependencies]
tempfile.workspace = true
serde_json.workspace = true
tokio.workspace = true
tower.workspace = true
```

Update `crates/mdid-runtime/src/lib.rs` to export the new module:

```rust
pub mod http;
pub mod moat;
pub mod moat_history;
```

Create `crates/mdid-runtime/src/moat_history.rs` with:

```rust
use crate::moat::MoatRoundReport;
use chrono::{DateTime, Utc};
use mdid_domain::ContinueDecision;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoatHistoryEntry {
    pub recorded_at: DateTime<Utc>,
    pub report: MoatRoundReport,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoatHistorySummary {
    pub entry_count: usize,
    pub latest_round_id: Option<String>,
    pub latest_continue_decision: Option<ContinueDecision>,
    pub latest_stop_reason: Option<String>,
    pub latest_decision_summary: Option<String>,
    pub latest_moat_score_after: Option<i16>,
    pub best_moat_score_after: Option<i16>,
    pub improvement_deltas: Vec<i16>,
}

#[derive(Debug)]
pub struct LocalMoatHistoryStore {
    path: PathBuf,
    entries: Vec<MoatHistoryEntry>,
}

impl LocalMoatHistoryStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, LocalMoatHistoryStoreError> {
        Self::open_with_mode(path, MissingHistoryBehavior::CreateEmptyFile)
    }

    pub fn open_existing(path: impl AsRef<Path>) -> Result<Self, LocalMoatHistoryStoreError> {
        Self::open_with_mode(path, MissingHistoryBehavior::Fail)
    }

    fn open_with_mode(
        path: impl AsRef<Path>,
        missing_history_behavior: MissingHistoryBehavior,
    ) -> Result<Self, LocalMoatHistoryStoreError> {
        let path = path.as_ref().to_path_buf();
        match missing_history_behavior {
            MissingHistoryBehavior::CreateEmptyFile => {
                if let Some(parent) = path.parent() {
                    if !parent.as_os_str().is_empty() {
                        fs::create_dir_all(parent)?;
                    }
                }
                if !path.exists() {
                    atomic_write(&path, b"[]")?;
                }
            }
            MissingHistoryBehavior::Fail => {
                if !path.exists() {
                    return Err(LocalMoatHistoryStoreError::MissingFile(path));
                }
            }
        }

        Ok(Self {
            entries: load_entries(&path)?,
            path,
        })
    }

    pub fn entries(&self) -> &[MoatHistoryEntry] {
        &self.entries
    }

    pub fn append(
        &mut self,
        recorded_at: DateTime<Utc>,
        report: MoatRoundReport,
    ) -> Result<(), LocalMoatHistoryStoreError> {
        let mut next_entries = self.entries.clone();
        next_entries.push(MoatHistoryEntry {
            recorded_at,
            report,
        });
        sort_entries(&mut next_entries);
        self.persist(&next_entries)?;
        self.entries = next_entries;
        Ok(())
    }

    pub fn summary(&self) -> MoatHistorySummary {
        let Some(latest) = self.entries.last() else {
            return MoatHistorySummary::default();
        };

        MoatHistorySummary {
            entry_count: self.entries.len(),
            latest_round_id: Some(latest.report.summary.round_id.to_string()),
            latest_continue_decision: Some(latest.report.summary.continue_decision),
            latest_stop_reason: latest.report.summary.stop_reason.clone(),
            latest_decision_summary: latest.report.control_plane.memory.latest_decision_summary(),
            latest_moat_score_after: Some(latest.report.summary.moat_score_after),
            best_moat_score_after: self
                .entries
                .iter()
                .map(|entry| entry.report.summary.moat_score_after)
                .max(),
            improvement_deltas: self
                .entries
                .iter()
                .map(|entry| entry.report.summary.improvement())
                .collect(),
        }
    }
}
```

Update `crates/mdid-runtime/src/moat.rs` so runtime reports can be serialized and each bounded round gets honest unique IDs:

```rust
use chrono::Utc;
use serde::{Deserialize, Serialize};
```

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoatControlPlaneReport {
    pub task_graph: MoatTaskGraph,
    pub memory: MoatMemorySnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoatRoundReport {
    pub summary: MoatRoundSummary,
    pub executed_tasks: Vec<String>,
    pub stop_reason: Option<String>,
    pub control_plane: MoatControlPlaneReport,
}
```

```rust
pub fn run_bounded_round(input: MoatRoundInput) -> MoatRoundReport {
    let round_id = Uuid::new_v4();
    // keep the remaining bounded execution flow unchanged
}
```

```rust
DecisionLogEntry {
    entry_id: Uuid::new_v4(),
    round_id: summary.round_id,
    author_role,
    summary: decision_summary.to_string(),
    rationale: rationale.to_string(),
    recorded_at: Utc::now(),
}
```

- [ ] **Step 4: Run the focused runtime tests to verify GREEN**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-runtime --test moat_history
cargo test -p mdid-runtime --test moat_runtime
```

Expected: PASS — history persistence works, runtime reports serialize, bounded rounds now carry non-nil IDs, and the control-plane memory stays internally consistent.

- [ ] **Step 5: Run broader crate verification**

Run:

```bash
source "$HOME/.cargo/env"
CARGO_BUILD_JOBS=1 cargo test -p mdid-runtime
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/mdid-runtime/Cargo.toml \
  crates/mdid-runtime/src/lib.rs \
  crates/mdid-runtime/src/moat.rs \
  crates/mdid-runtime/src/moat_history.rs \
  crates/mdid-runtime/tests/moat_history.rs \
  crates/mdid-runtime/tests/moat_runtime.rs
git commit -m "feat: persist moat round history"
```

---

### Task 2: Expose persisted history through the CLI and truth-sync docs

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
- Modify: `docs/superpowers/plans/2026-04-25-med-de-id-moat-round-history.md`

- [ ] **Step 1: Write the failing CLI and doc-facing tests**

Extend `crates/mdid-cli/tests/moat_cli.rs` with coverage for the shipped CLI contract. Include duplicate `--history-path` coverage so both `moat round` and `moat history` reject repeated flags with `duplicate flag: --history-path` instead of silently taking the last value or reporting a mismatched parser error:

```rust
use mdid_runtime::moat_history::LocalMoatHistoryStore;
use std::{
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

const USAGE: &str = "usage: mdid-cli [status | moat round [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false] [--history-path PATH] | moat control-plane [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false] | moat history --history-path PATH]";

#[test]
fn cli_runs_moat_round_with_history_path_and_persists_report() {
    let history_path = unique_history_path("persisted-round");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
        ])
        .output()
        .expect("failed to run mdid-cli moat round with history path");

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("history_saved_to="));

    let store = LocalMoatHistoryStore::open(&history_path).expect("history store should open");
    assert_eq!(store.summary().entry_count, 1);
}

#[test]
fn cli_reports_history_summary_for_two_persisted_rounds() {
    let history_path = unique_history_path("history-summary");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to run first persisted round");
    Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--review-loops",
            "0",
            "--history-path",
            history_path_arg,
        ])
        .output()
        .expect("failed to run second persisted round");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "history", "--history-path", history_path_arg])
        .output()
        .expect("failed to run mdid-cli moat history");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("moat history summary"));
    assert!(stdout.contains("entries=2"));
    assert!(stdout.contains("latest_continue_decision=Stop"));
    assert!(stdout.contains("latest_stop_reason=review budget exhausted"));
    assert!(stdout.contains("best_moat_score_after=98"));
    assert!(stdout.contains("improvement_deltas=8,0"));
}

#[test]
fn cli_requires_history_path_for_history_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "history"])
        .output()
        .expect("failed to run mdid-cli moat history without path");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("missing required flag: --history-path"));
    assert!(stderr.contains(USAGE));
}

#[test]
fn cli_history_reports_missing_history_path_value_before_unknown_flags() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "history",
            "--history-path",
            "--review-loops",
            "0",
            "--bogus",
        ])
        .output()
        .expect("failed to run mdid-cli moat history with malformed history path");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("missing value for --history-path"));
    assert!(stderr.contains(USAGE));
}

#[test]
fn cli_history_rejects_missing_history_file_without_creating_it() {
    let history_path = unique_history_path("missing-history-summary");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "history",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
        ])
        .output()
        .expect("failed to run mdid-cli moat history with missing history path");

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed to open moat history store"));
    assert!(stderr.contains("moat history file does not exist"));
    assert!(!history_path.exists());
}
```

Also keep the existing success/error coverage intact. The landed CLI contract must still reject unknown commands and malformed flags with the updated usage string.

- [ ] **Step 2: Run the focused CLI tests to verify RED**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli
```

Expected: FAIL because the CLI does not yet recognize `moat history`, does not parse `--history-path`, and does not append persisted reports.

- [ ] **Step 3: Write the minimal CLI and docs implementation**

Update `crates/mdid-cli/src/main.rs` so the command model includes optional history persistence for round execution and a dedicated history subcommand:

```rust
use mdid_runtime::{
    moat::{run_bounded_round, MoatRoundInput, MoatRoundReport},
    moat_history::{LocalMoatHistoryStore, MoatHistorySummary},
};
```

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct MoatRoundCommand {
    overrides: MoatRoundOverrides,
    history_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CliCommand {
    Status,
    MoatRound(MoatRoundCommand),
    MoatControlPlane(MoatRoundOverrides),
    MoatHistory(String),
}
```

```rust
fn parse_command(args: &[String]) -> Result<CliCommand, String> {
    match args {
        [] => Ok(CliCommand::Status),
        [status] if status == "status" => Ok(CliCommand::Status),
        [moat, round, rest @ ..] if moat == "moat" && round == "round" => {
            Ok(CliCommand::MoatRound(parse_moat_round_command(rest)?))
        }
        [moat, control_plane, rest @ ..] if moat == "moat" && control_plane == "control-plane" => {
            let (overrides, _) = parse_moat_round_overrides(rest, false)?;
            Ok(CliCommand::MoatControlPlane(overrides))
        }
        [moat, history, rest @ ..] if moat == "moat" && history == "history" => {
            Ok(CliCommand::MoatHistory(parse_required_history_path(rest)?))
        }
        _ => Err(format!("unknown command: {}", format_command(args))),
    }
}
```

```rust
fn parse_moat_round_command(args: &[String]) -> Result<MoatRoundCommand, String> {
    let (overrides, history_path) = parse_moat_round_overrides(args, true)?;
    Ok(MoatRoundCommand {
        overrides,
        history_path,
    })
}

fn parse_required_history_path(args: &[String]) -> Result<String, String> {
    let Some(flag) = args.first() else {
        return Err("missing required flag: --history-path".to_string());
    };

    if flag != "--history-path" {
        return Err(format!("unknown flag: {flag}"));
    }

    let history_path = required_history_path_value(args, 0)?.clone();

    if let Some(extra) = args.get(2) {
        Err(format!("unknown flag: {extra}"))
    } else {
        Ok(history_path)
    }
}
```

Extend round parsing so `--history-path` is accepted only for `moat round`, leaving `moat control-plane` unchanged:

```rust
fn parse_moat_round_overrides(
    args: &[String],
    allow_history_path: bool,
) -> Result<(MoatRoundOverrides, Option<String>), String> {
    // existing override parsing omitted
    match flag.as_str() {
        "--history-path" if allow_history_path => {
            let value = required_flag_value(args, index, flag, allow_history_path)?;
            history_path = Some(value.clone());
        }
        _ => return Err(format!("unknown flag: {flag}")),
    }
}
```

Persist only when the caller explicitly provides a path:

```rust
fn run_moat_round(command: &MoatRoundCommand) -> Result<(), String> {
    let report = sample_round_report(&command.overrides);

    if let Some(history_path) = &command.history_path {
        append_report_to_history(history_path, &report)?;
    }

    println!("moat round complete");
    println!(
        "continue_decision={}",
        format_continue_decision(report.summary.continue_decision)
    );
    println!("executed_tasks={}", report.executed_tasks.join(","));
    println!("moat_score_before={}", report.summary.moat_score_before);
    println!("moat_score_after={}", report.summary.moat_score_after);
    println!(
        "stop_reason={}",
        report.stop_reason.as_deref().unwrap_or("<none>")
    );

    if let Some(history_path) = &command.history_path {
        println!("history_saved_to={history_path}");
    }

    Ok(())
}

fn append_report_to_history(history_path: &str, report: &MoatRoundReport) -> Result<(), String> {
    let mut store = LocalMoatHistoryStore::open(history_path)
        .map_err(|error| format!("failed to open moat history store: {error}"))?;
    store
        .append(std::time::SystemTime::now().into(), report.clone())
        .map_err(|error| format!("failed to append moat history entry: {error}"))
}
```

Add a new printer for persisted history:

```rust
fn run_moat_history(history_path: &str) -> Result<(), String> {
    let store = LocalMoatHistoryStore::open_existing(history_path)
        .map_err(|error| format!("failed to open moat history store: {error}"))?;
    print_history_summary(&store.summary());
    Ok(())
}

fn print_history_summary(summary: &MoatHistorySummary) {
    println!("moat history summary");
    println!("entries={}", summary.entry_count);
    println!(
        "latest_round_id={}",
        summary.latest_round_id.as_deref().unwrap_or("<none>")
    );
    println!(
        "latest_continue_decision={}",
        summary
            .latest_continue_decision
            .map(format_continue_decision)
            .unwrap_or("<none>")
    );
    println!(
        "latest_stop_reason={}",
        summary.latest_stop_reason.as_deref().unwrap_or("<none>")
    );
    println!(
        "latest_decision_summary={}",
        summary.latest_decision_summary.as_deref().unwrap_or("<none>")
    );
    println!(
        "latest_moat_score_after={}",
        format_optional_i16(summary.latest_moat_score_after)
    );
    println!(
        "best_moat_score_after={}",
        format_optional_i16(summary.best_moat_score_after)
    );
    println!(
        "improvement_deltas={}",
        format_improvement_deltas(&summary.improvement_deltas)
    );
}
```

Update the usage string everywhere to:

```text
usage: mdid-cli [status | moat round [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false] [--history-path PATH] | moat control-plane [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false] | moat history --history-path PATH]
```

Truth-sync `README.md` so the moat-loop section documents bounded local history persistence and inspection, while still stating that the shipped slice is not live market crawling, scheduler control, PR automation, or a full autonomous multi-agent runtime.

Truth-sync `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md` so persisted local history moves into shipped status and the spec explicitly references the landed runtime contract (`LocalMoatHistoryStore`, `open(path)`, `open_existing(path)`, `MoatHistoryEntry`, `append(recorded_at, report)`, `summary()`).

- [ ] **Step 4: Run the focused CLI tests to verify GREEN**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli
```

Expected: PASS — round persistence works when requested, `moat history` prints the bounded summary, and usage/error handling stays truthful.

- [ ] **Step 5: Run broader workspace verification for the touched crates**

Run:

```bash
source "$HOME/.cargo/env"
CARGO_BUILD_JOBS=1 cargo test -p mdid-cli
CARGO_BUILD_JOBS=1 cargo test -p mdid-runtime
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/mdid-cli/src/main.rs \
  crates/mdid-cli/tests/moat_cli.rs \
  README.md \
  docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md \
  docs/superpowers/plans/2026-04-25-med-de-id-moat-round-history.md
git commit -m "feat: add moat round history cli"
```

---

## Self-review

### Spec coverage
- Persistent memory store and decision log: covered by Task 1 runtime history persistence.
- CLI inspect round history: covered by Task 2 `moat history --history-path PATH`.
- Decision logging and historical comparison: covered by persisted records plus `best_moat_score_after` and `improvement_deltas` summary output.
- Bounded/local-first scope honesty: covered by Task 2 README/spec truth-sync.

### Placeholder scan
- No `TBD`, `TODO`, or “similar to previous task” placeholders remain.
- Every code-changing step includes concrete code blocks.
- Every verification step includes exact commands and expected outcomes.

### Type consistency
- `LocalMoatHistoryStore`, `MoatHistoryEntry`, `append(recorded_at, report)`, and `summary()` are defined in Task 1 before Task 2 consumes them.
- CLI uses a dedicated `MoatRoundCommand` plus `String` history paths while still routing persistence through the runtime store.
- Runtime and CLI both treat `MoatRoundReport` as the persisted round payload.
