# Moat Continuation Gate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded history-backed continuation gate so the moat loop can truthfully decide whether a new round may start from the latest persisted round result.

**Architecture:** Keep this slice local-first and deterministic. `mdid-runtime` will extend the existing JSON-backed moat history store with a continuation-gate summary derived from the latest persisted round, including whether evaluation completed and whether the latest round cleared the configured improvement threshold. `mdid-cli` will expose a read-only `moat continue` command that prints the gate decision, while README/spec docs truth-sync the new bounded operator surface without pretending scheduler automation already exists.

**Tech Stack:** Rust workspace, Cargo, `mdid-runtime`, `mdid-cli`, `serde`, markdown docs.

---

## Scope note

This slice adds:
- a runtime continuation-gate report derived from the latest persisted moat history entry
- explicit evaluation-complete vs. pre-evaluation stop detection
- a read-only `mdid-cli moat continue --history-path PATH [--improvement-threshold N]` inspection surface
- README/spec truth-sync for the new bounded continuation contract

This slice does **not** add:
- automatic scheduling or background loop execution
- live market crawling
- GitHub PR automation
- unrestricted autonomous round chaining
- cloud persistence or multi-user orchestration

## File structure

**Modify:**
- `crates/mdid-runtime/src/moat_history.rs`
- `crates/mdid-runtime/tests/moat_history.rs`
- `crates/mdid-cli/src/main.rs`
- `crates/mdid-cli/tests/moat_cli.rs`
- `README.md`
- `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`

---

### Task 1: Add history-backed continuation-gate evaluation in `mdid-runtime`

**Files:**
- Modify: `crates/mdid-runtime/src/moat_history.rs`
- Modify: `crates/mdid-runtime/tests/moat_history.rs`

- [ ] **Step 1: Write the failing runtime tests**

Append these tests and helper adjustments to `crates/mdid-runtime/tests/moat_history.rs`:

```rust
#[test]
fn continuation_gate_allows_next_round_when_latest_round_completed_evaluation_and_cleared_threshold() {
    let dir = tempdir().expect("temp dir should exist");
    let history_path = dir.path().join("moat-history.json");
    let round_id = Uuid::new_v4();
    let mut store = LocalMoatHistoryStore::open(&history_path).expect("history store should open");

    store
        .append(
            recorded_at("2026-04-25T22:00:00Z"),
            sample_report(
                round_id,
                ContinueDecision::Continue,
                None,
                "review approved bounded moat round",
                90,
                98,
                true,
                &["market_scan", "competitor_analysis", "lockin_analysis", "strategy_generation", "spec_planning", "implementation", "review", "evaluation"],
            ),
        )
        .expect("continue report should persist");

    assert_eq!(
        store.continuation_gate(3),
        mdid_runtime::moat_history::MoatContinuationGate {
            latest_round_id: Some(round_id.to_string()),
            latest_continue_decision: Some(ContinueDecision::Continue),
            latest_tests_passed: Some(true),
            latest_improvement_delta: Some(8),
            latest_stop_reason: None,
            evaluation_completed: true,
            can_continue: true,
            reason: "latest round cleared continuation gate".to_string(),
            required_improvement_threshold: 3,
        }
    );
}

#[test]
fn continuation_gate_blocks_when_latest_round_never_reached_evaluation() {
    let dir = tempdir().expect("temp dir should exist");
    let history_path = dir.path().join("moat-history.json");
    let round_id = Uuid::new_v4();
    let mut store = LocalMoatHistoryStore::open(&history_path).expect("history store should open");

    store
        .append(
            recorded_at("2026-04-25T22:00:00Z"),
            sample_report(
                round_id,
                ContinueDecision::Stop,
                Some("review budget exhausted"),
                "implementation stopped before review",
                90,
                90,
                true,
                &["market_scan", "competitor_analysis", "lockin_analysis", "strategy_generation", "spec_planning", "implementation"],
            ),
        )
        .expect("stopped report should persist");

    assert_eq!(
        store.continuation_gate(3),
        mdid_runtime::moat_history::MoatContinuationGate {
            latest_round_id: Some(round_id.to_string()),
            latest_continue_decision: Some(ContinueDecision::Stop),
            latest_tests_passed: Some(true),
            latest_improvement_delta: Some(0),
            latest_stop_reason: Some("review budget exhausted".to_string()),
            evaluation_completed: false,
            can_continue: false,
            reason: "latest round did not complete evaluation".to_string(),
            required_improvement_threshold: 3,
        }
    );
}

#[test]
fn continuation_gate_blocks_when_latest_round_failed_tests_after_evaluation() {
    let dir = tempdir().expect("temp dir should exist");
    let history_path = dir.path().join("moat-history.json");
    let round_id = Uuid::new_v4();
    let mut store = LocalMoatHistoryStore::open(&history_path).expect("history store should open");

    store
        .append(
            recorded_at("2026-04-25T22:00:00Z"),
            sample_report(
                round_id,
                ContinueDecision::Stop,
                Some("tests failed"),
                "review stopped bounded moat round",
                90,
                90,
                false,
                &["market_scan", "competitor_analysis", "lockin_analysis", "strategy_generation", "spec_planning", "implementation", "review", "evaluation"],
            ),
        )
        .expect("failed test report should persist");

    let gate = store.continuation_gate(3);
    assert!(!gate.can_continue);
    assert!(gate.evaluation_completed);
    assert_eq!(gate.reason, "latest round tests failed");
    assert_eq!(gate.latest_tests_passed, Some(false));
}
```

Replace the existing `sample_report` helper signature and `executed_tasks` assignment with:

```rust
fn sample_report(
    round_id: Uuid,
    continue_decision: ContinueDecision,
    stop_reason: Option<&str>,
    decision_summary: &str,
    moat_score_before: i16,
    moat_score_after: i16,
    tests_passed: bool,
    executed_tasks: &[&str],
) -> MoatRoundReport {
    let summary = MoatRoundSummary {
        round_id,
        selected_strategies: vec!["workflow-audit".to_string()],
        implemented_specs: Vec::new(),
        tests_passed,
        moat_score_before,
        moat_score_after,
        continue_decision,
        stop_reason: stop_reason.map(str::to_string),
        pivot_reason: None,
    };
    let decision = DecisionLogEntry {
        entry_id: Uuid::new_v4(),
        round_id,
        author_role: if continue_decision == ContinueDecision::Continue {
            AgentRole::Reviewer
        } else {
            AgentRole::Coder
        },
        summary: decision_summary.to_string(),
        rationale: stop_reason.unwrap_or("approved").to_string(),
        recorded_at: recorded_at("2026-04-25T19:59:00Z"),
    };
    let control_plane = MoatControlPlaneReport {
        task_graph: build_default_moat_task_graph(round_id),
        memory: summarize_round_memory(&summary, vec![decision]),
    };

    MoatRoundReport {
        summary,
        executed_tasks: executed_tasks.iter().map(|task| (*task).to_string()).collect(),
        stop_reason: stop_reason.map(str::to_string),
        control_plane,
    }
}
```

Update the existing `sample_report(...)` call sites in the file to pass `tests_passed` and the explicit executed-task slices shown above.

- [ ] **Step 2: Run the focused runtime tests to verify RED**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-runtime --test moat_history
```

Expected: FAIL because `LocalMoatHistoryStore` does not yet expose `continuation_gate`, and the new gate contract types do not exist.

- [ ] **Step 3: Write the minimal runtime implementation**

Add this contract and helper logic to `crates/mdid-runtime/src/moat_history.rs` near `MoatHistorySummary` and `impl LocalMoatHistoryStore`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoatContinuationGate {
    pub latest_round_id: Option<String>,
    pub latest_continue_decision: Option<ContinueDecision>,
    pub latest_tests_passed: Option<bool>,
    pub latest_improvement_delta: Option<i16>,
    pub latest_stop_reason: Option<String>,
    pub evaluation_completed: bool,
    pub can_continue: bool,
    pub reason: String,
    pub required_improvement_threshold: i16,
}

impl LocalMoatHistoryStore {
    pub fn continuation_gate(&self, required_improvement_threshold: i16) -> MoatContinuationGate {
        let Some(latest) = self.entries.last() else {
            return MoatContinuationGate {
                latest_round_id: None,
                latest_continue_decision: None,
                latest_tests_passed: None,
                latest_improvement_delta: None,
                latest_stop_reason: None,
                evaluation_completed: false,
                can_continue: false,
                reason: "no persisted moat rounds to evaluate".to_string(),
                required_improvement_threshold,
            };
        };

        let improvement_delta = latest.report.summary.improvement();
        let evaluation_completed = latest
            .report
            .executed_tasks
            .iter()
            .any(|task| task == "evaluation");

        let (can_continue, reason) = if !evaluation_completed {
            (false, "latest round did not complete evaluation")
        } else if !latest.report.summary.tests_passed {
            (false, "latest round tests failed")
        } else if improvement_delta < required_improvement_threshold {
            (false, "latest round improvement below threshold")
        } else if latest.report.summary.continue_decision == ContinueDecision::Continue {
            (true, "latest round cleared continuation gate")
        } else {
            (false, "latest round requested stop")
        };

        MoatContinuationGate {
            latest_round_id: Some(latest.report.summary.round_id.to_string()),
            latest_continue_decision: Some(latest.report.summary.continue_decision),
            latest_tests_passed: Some(latest.report.summary.tests_passed),
            latest_improvement_delta: Some(improvement_delta),
            latest_stop_reason: latest.report.summary.stop_reason.clone(),
            evaluation_completed,
            can_continue,
            reason: reason.to_string(),
            required_improvement_threshold,
        }
    }
}
```

- [ ] **Step 4: Run the tests to verify GREEN**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-runtime --test moat_history
cargo test -p mdid-runtime
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-runtime/src/moat_history.rs crates/mdid-runtime/tests/moat_history.rs
git commit -m "feat: add moat continuation gate"
```

### Task 2: Expose the continuation gate through `mdid-cli` and truth-sync docs

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`

- [ ] **Step 1: Write the failing CLI and docs-backed tests**

Append these tests to `crates/mdid-cli/tests/moat_cli.rs` and update the shared `USAGE` string:

```rust
const USAGE: &str = "usage: mdid-cli [status | moat round [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false] [--history-path PATH] | moat control-plane [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false] | moat history --history-path PATH | moat continue --history-path PATH [--improvement-threshold N]]";

#[test]
fn cli_reports_continuation_gate_for_latest_successful_round() {
    let history_path = unique_history_path("continue-success");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history");
    assert!(seed.status.success(), "stderr was: {}", String::from_utf8_lossy(&seed.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "continue", "--history-path", history_path_arg])
        .output()
        .expect("failed to run mdid-cli moat continue");

    assert!(output.status.success(), "stderr was: {}", String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8_lossy(&output.stdout).contains("moat continuation gate\n"));
    assert!(String::from_utf8_lossy(&output.stdout).contains("can_continue=true\n"));
    assert!(String::from_utf8_lossy(&output.stdout).contains("reason=latest round cleared continuation gate\n"));
    assert!(String::from_utf8_lossy(&output.stdout).contains("required_improvement_threshold=3\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn cli_reports_continuation_gate_for_pre_evaluation_stop_round() {
    let history_path = unique_history_path("continue-stop");
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
        .args(["moat", "continue", "--history-path", history_path_arg])
        .output()
        .expect("failed to run mdid-cli moat continue");

    assert!(output.status.success(), "stderr was: {}", String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8_lossy(&output.stdout).contains("can_continue=false\n"));
    assert!(String::from_utf8_lossy(&output.stdout).contains("evaluation_completed=false\n"));
    assert!(String::from_utf8_lossy(&output.stdout).contains("reason=latest round did not complete evaluation\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn cli_continue_rejects_invalid_improvement_threshold() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "continue", "--history-path", "history.json", "--improvement-threshold", "bogus"])
        .output()
        .expect("failed to run mdid-cli moat continue with invalid threshold");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid value for --improvement-threshold: bogus"));
    assert!(stderr.contains(USAGE));
}
```

- [ ] **Step 2: Run the focused CLI tests to verify RED**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli
```

Expected: FAIL because the CLI does not yet parse or print `moat continue`.

- [ ] **Step 3: Write the minimal CLI and docs implementation**

Update `crates/mdid-cli/src/main.rs` to add the command contract:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
enum CliCommand {
    Status,
    MoatRound(MoatRoundCommand),
    MoatControlPlane(MoatRoundOverrides),
    MoatHistory(String),
    MoatContinue {
        history_path: String,
        improvement_threshold: i16,
    },
}
```

Add command parsing for `moat continue`:

```rust
[moat, continue_command, rest @ ..] if moat == "moat" && continue_command == "continue" => {
    Ok(parse_moat_continue_command(rest)?)
}
```

Add the parser helpers:

```rust
fn parse_moat_continue_command(args: &[String]) -> Result<CliCommand, String> {
    let mut history_path = None;
    let mut improvement_threshold = 3;
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
            "--improvement-threshold" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --improvement-threshold".to_string())?;
                improvement_threshold = value
                    .parse::<i16>()
                    .map_err(|_| format!("invalid value for --improvement-threshold: {value}"))?;
            }
            flag => return Err(format!("unknown flag: {flag}")),
        }

        index += 2;
    }

    Ok(CliCommand::MoatContinue {
        history_path: history_path.ok_or_else(|| "missing required flag: --history-path".to_string())?,
        improvement_threshold,
    })
}
```

Dispatch it from `main()` and print the report with:

```rust
Ok(CliCommand::MoatContinue {
    history_path,
    improvement_threshold,
}) => {
    if let Err(error) = run_moat_continue(&history_path, improvement_threshold) {
        exit_with_error(error);
    }
}
```

Add the runner and printer:

```rust
fn run_moat_continue(history_path: &str, improvement_threshold: i16) -> Result<(), String> {
    let store = LocalMoatHistoryStore::open_existing(history_path)
        .map_err(|error| format!("failed to open moat history store: {error}"))?;
    let gate = store.continuation_gate(improvement_threshold);
    println!("moat continuation gate");
    println!("latest_round_id={}", gate.latest_round_id.as_deref().unwrap_or("<none>"));
    println!(
        "latest_continue_decision={}",
        gate.latest_continue_decision
            .map(format_continue_decision)
            .unwrap_or("<none>")
    );
    println!(
        "latest_tests_passed={}",
        gate.latest_tests_passed
            .map(|value| if value { "true" } else { "false" })
            .unwrap_or("<none>")
    );
    println!(
        "latest_improvement_delta={}",
        gate.latest_improvement_delta
            .map(|value| value.to_string())
            .unwrap_or_else(|| "<none>".to_string())
    );
    println!(
        "latest_stop_reason={}",
        gate.latest_stop_reason.as_deref().unwrap_or("<none>")
    );
    println!("evaluation_completed={}", if gate.evaluation_completed { "true" } else { "false" });
    println!("can_continue={}", if gate.can_continue { "true" } else { "false" });
    println!("reason={}", gate.reason);
    println!("required_improvement_threshold={}", gate.required_improvement_threshold);
    Ok(())
}
```

Update `usage()` to:

```rust
"usage: mdid-cli [status | moat round [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false] [--history-path PATH] | moat control-plane [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false] | moat history --history-path PATH | moat continue --history-path PATH [--improvement-threshold N]]"
```

Update `README.md` by inserting after the `moat history` section:

```md
Inspect whether the latest persisted round is eligible to start another bounded round with:

```bash
cargo run -p mdid-cli -- moat continue --history-path .mdid/moat-history.json
```

The continuation command prints a bounded gate summary containing:
- `latest_round_id`
- `latest_continue_decision`
- `latest_tests_passed`
- `latest_improvement_delta`
- `latest_stop_reason`
- `evaluation_completed=true|false`
- `can_continue=true|false`
- `reason`
- `required_improvement_threshold`

This is still an inspection surface only. It does not schedule or launch the next round automatically.
```

Update `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md` by adding one shipped-foundation bullet under the implementation-status section:

```md
- a bounded operator-facing `mdid-cli moat continue --history-path PATH [--improvement-threshold N]` gate that truthfully reports whether the latest persisted round completed evaluation and cleared the configured continuation threshold
```

And add one matching sentence to the “still not live and not fully autonomous” paragraph clarifying that the continuation gate is inspection-only and does not start new rounds.

- [ ] **Step 4: Run the tests and docs-backed verification to verify GREEN**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli
cargo test -p mdid-cli
cargo run -q -p mdid-cli -- moat round --history-path .mdid/moat-history.json
cargo run -q -p mdid-cli -- moat continue --history-path .mdid/moat-history.json
python - <<'PY'
from pathlib import Path
readme = Path('README.md').read_text()
spec = Path('docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md').read_text()
required = [
    'cargo run -p mdid-cli -- moat continue --history-path .mdid/moat-history.json',
    'can_continue=true|false',
    'inspection surface only',
    'mdid-cli moat continue --history-path PATH [--improvement-threshold N]',
]
missing = [item for item in required if item not in readme + spec]
if missing:
    raise SystemExit(f'missing: {missing}')
print('docs truth-synced')
PY
```

Expected: PASS with `docs truth-synced`.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs README.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md
git commit -m "feat: add moat continuation gate cli"
```
