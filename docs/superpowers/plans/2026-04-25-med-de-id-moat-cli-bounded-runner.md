# Moat CLI Bounded Runner Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the canned moat CLI sample into a bounded operator-facing runner by allowing deterministic budget/test overrides and by surfacing honest stop reasons in round output.

**Architecture:** Keep the slice inside `mdid-cli` and reuse the existing deterministic sample artifacts plus `mdid-runtime::moat::run_bounded_round`. The CLI will parse a narrow set of override flags, apply them to the existing sample `MoatRoundInput`, and render either the round summary or control-plane snapshot without introducing persistence, live crawling, or scheduler behavior.

**Tech Stack:** Rust workspace, Cargo, `mdid-cli`, `mdid-runtime`, `mdid-domain`, markdown docs.

---

## Scope note

This slice is intentionally narrow. It adds:
- deterministic CLI overrides for bounded moat rounds
- honest `stop_reason` output on `mdid-cli moat round`
- README/spec truth-sync so the shipped CLI contract matches reality

This slice does **not** add:
- persisted round inputs or memory stores
- live market crawling
- GitHub PR/release automation
- background scheduling
- browser or desktop moat surfaces

## File structure

**Modify:**
- `crates/mdid-cli/src/main.rs`
- `crates/mdid-cli/tests/moat_cli.rs`
- `README.md`
- `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
- `docs/superpowers/plans/2026-04-25-med-de-id-moat-cli-bounded-runner.md`

---

### Task 1: Add bounded moat CLI override parsing and honest stop reporting

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/moat_cli.rs`

- [ ] **Step 1: Write the failing CLI tests**

Ensure `crates/mdid-cli/tests/moat_cli.rs` covers the bounded runner contract, including:

```rust
use std::process::Command;

#[test]
fn cli_runs_moat_round_and_prints_deterministic_report() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round"])
        .output()
        .expect("failed to run mdid-cli moat round");

    assert!(
        output.status.success(),
        "expected success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "moat round complete\n",
            "continue_decision=Continue\n",
            "executed_tasks=market_scan,competitor_analysis,lockin_analysis,strategy_generation,spec_planning,implementation,review,evaluation\n",
            "moat_score_before=90\n",
            "moat_score_after=98\n",
            "stop_reason=<none>\n",
        )
    );
}

#[test]
fn cli_runs_moat_round_with_review_budget_override_and_reports_stop_reason() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--review-loops", "0"])
        .output()
        .expect("failed to run mdid-cli moat round with review override");

    assert!(
        output.status.success(),
        "expected success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "moat round complete\n",
            "continue_decision=Stop\n",
            "executed_tasks=market_scan,competitor_analysis,lockin_analysis,strategy_generation,spec_planning,implementation\n",
            "moat_score_before=90\n",
            "moat_score_after=90\n",
            "stop_reason=review budget exhausted\n",
        )
    );
}

#[test]
fn cli_runs_moat_control_plane_with_strategy_budget_override() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "control-plane", "--strategy-candidates", "0"])
        .output()
        .expect("failed to run mdid-cli moat control-plane with strategy override");

    assert!(
        output.status.success(),
        "expected success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "moat control plane snapshot\n",
            "ready_nodes=strategy_generation\n",
            "latest_decision_summary=planning stopped before implementation\n",
            "improvement_delta=0\n",
            "task_states=market_scan:completed,competitor_analysis:completed,lockin_analysis:completed,strategy_generation:ready,spec_planning:pending,implementation:pending,review:pending,evaluation:pending\n",
        )
    );
}

#[test]
fn cli_rejects_non_numeric_override_values() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--review-loops", "bogus"])
        .output()
        .expect("failed to run mdid-cli moat round with invalid override");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid value for --review-loops: bogus"));
    assert!(stderr.contains("usage: mdid-cli [status | moat round [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false] | moat control-plane [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false]]"));
}

#[test]
fn cli_reports_helpful_error_for_unknown_commands() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .arg("bogus")
        .output()
        .expect("failed to run mdid-cli bogus");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown command: bogus"));
    assert!(stderr.contains("usage: mdid-cli [status | moat round [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false] | moat control-plane [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false]]"));
}
```

Keep the landed default control-plane, unknown-override-flag, and missing-override-value coverage as well; the shipped CLI contract includes both successful stop-path inspection and helpful usage errors.

- [ ] **Step 2: Run the focused CLI test target to verify RED**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli
```

Expected: FAIL before implementation because the CLI does not yet print `stop_reason`, does not accept override flags, and does not yet cover the stricter override-validation/usage contract.

- [ ] **Step 3: Write the minimal CLI implementation**

Replace `crates/mdid-cli/src/main.rs` with:

```rust
use mdid_domain::{
    CompetitorProfile, ContinueDecision, LockInReport, MarketMoatSnapshot, MoatStrategy,
    MoatTaskNodeState, MoatType, ResourceBudget,
};
use mdid_runtime::moat::{run_bounded_round, MoatRoundInput, MoatRoundReport};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct MoatRoundOverrides {
    strategy_candidates: Option<u8>,
    spec_generations: Option<u8>,
    implementation_tasks: Option<u8>,
    review_loops: Option<u8>,
    tests_passed: Option<bool>,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match parse_command(&args) {
        Ok(CliCommand::Status) => println!("med-de-id CLI ready"),
        Ok(CliCommand::MoatRound(overrides)) => run_moat_round(&overrides),
        Ok(CliCommand::MoatControlPlane(overrides)) => run_moat_control_plane(&overrides),
        Err(error) => exit_with_usage(error),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CliCommand {
    Status,
    MoatRound(MoatRoundOverrides),
    MoatControlPlane(MoatRoundOverrides),
}

fn parse_command(args: &[String]) -> Result<CliCommand, String> {
    match args {
        [] => Ok(CliCommand::Status),
        [status] if status == "status" => Ok(CliCommand::Status),
        [moat, round, rest @ ..] if moat == "moat" && round == "round" => {
            Ok(CliCommand::MoatRound(parse_moat_round_overrides(rest)?))
        }
        [moat, control_plane, rest @ ..] if moat == "moat" && control_plane == "control-plane" => {
            Ok(CliCommand::MoatControlPlane(parse_moat_round_overrides(rest)?))
        }
        _ => Err(format!("unknown command: {}", format_command(args))),
    }
}

fn parse_moat_round_overrides(args: &[String]) -> Result<MoatRoundOverrides, String> {
    let mut overrides = MoatRoundOverrides::default();
    let mut index = 0;

    while index < args.len() {
        let flag = &args[index];
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {flag}"))?;

        match flag.as_str() {
            "--strategy-candidates" => {
                overrides.strategy_candidates = Some(parse_u8_flag(flag, value)?);
            }
            "--spec-generations" => {
                overrides.spec_generations = Some(parse_u8_flag(flag, value)?);
            }
            "--implementation-tasks" => {
                overrides.implementation_tasks = Some(parse_u8_flag(flag, value)?);
            }
            "--review-loops" => {
                overrides.review_loops = Some(parse_u8_flag(flag, value)?);
            }
            "--tests-passed" => {
                overrides.tests_passed = Some(parse_bool_flag(flag, value)?);
            }
            _ => return Err(format!("unknown flag: {flag}")),
        }

        index += 2;
    }

    Ok(overrides)
}

fn parse_u8_flag(flag: &str, value: &str) -> Result<u8, String> {
    value
        .parse::<u8>()
        .map_err(|_| format!("invalid value for {flag}: {value}"))
}

fn parse_bool_flag(flag: &str, value: &str) -> Result<bool, String> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("invalid value for {flag}: {value}")),
    }
}

fn run_moat_round(overrides: &MoatRoundOverrides) {
    let report = sample_round_report(overrides);

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
}

fn run_moat_control_plane(overrides: &MoatRoundOverrides) {
    let report = sample_round_report(overrides);
    let control_plane = report.control_plane;
    let ready_nodes = format_ready_nodes(&control_plane.task_graph.ready_node_ids());
    let latest_decision_summary = control_plane
        .memory
        .latest_decision_summary()
        .unwrap_or_else(|| "<none>".to_string());
    let task_states = format_task_states(&control_plane.task_graph.nodes);

    println!("moat control plane snapshot");
    println!("ready_nodes={ready_nodes}");
    println!("latest_decision_summary={latest_decision_summary}");
    println!(
        "improvement_delta={}",
        control_plane.memory.improvement_delta
    );
    println!("task_states={task_states}");
}

fn sample_round_report(overrides: &MoatRoundOverrides) -> MoatRoundReport {
    run_bounded_round(sample_round_input(overrides))
}

fn format_continue_decision(continue_decision: ContinueDecision) -> &'static str {
    match continue_decision {
        ContinueDecision::Continue => "Continue",
        ContinueDecision::Stop => "Stop",
        ContinueDecision::Pivot => "Pivot",
    }
}

fn format_ready_nodes(ready_nodes: &[String]) -> String {
    if ready_nodes.is_empty() {
        "<none>".to_string()
    } else {
        ready_nodes.join(",")
    }
}

fn format_task_states(nodes: &[mdid_domain::MoatTaskNode]) -> String {
    nodes
        .iter()
        .map(|node| format!("{}:{}", node.node_id, format_task_node_state(node.state)))
        .collect::<Vec<_>>()
        .join(",")
}

fn format_task_node_state(state: MoatTaskNodeState) -> &'static str {
    match state {
        MoatTaskNodeState::Pending => "pending",
        MoatTaskNodeState::Ready => "ready",
        MoatTaskNodeState::InProgress => "in_progress",
        MoatTaskNodeState::Completed => "completed",
        MoatTaskNodeState::Blocked => "blocked",
    }
}

fn sample_round_input(overrides: &MoatRoundOverrides) -> MoatRoundInput {
    let mut input = MoatRoundInput {
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
            max_review_loops: 1,
        },
        improvement_threshold: 3,
        tests_passed: true,
    };

    if let Some(value) = overrides.strategy_candidates {
        input.budget.max_strategy_candidates = value;
    }
    if let Some(value) = overrides.spec_generations {
        input.budget.max_spec_generations = value;
    }
    if let Some(value) = overrides.implementation_tasks {
        input.budget.max_implementation_tasks = value;
    }
    if let Some(value) = overrides.review_loops {
        input.budget.max_review_loops = value;
    }
    if let Some(value) = overrides.tests_passed {
        input.tests_passed = value;
    }

    input
}

fn format_command(args: &[String]) -> String {
    if args.is_empty() {
        "<none>".to_string()
    } else {
        args.join(" ")
    }
}

fn usage() -> &'static str {
    "usage: mdid-cli [status | moat round [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false] | moat control-plane [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false]]"
}

fn exit_with_usage(message: String) -> ! {
    eprintln!("{message}");
    eprintln!("{}", usage());
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use mdid_domain::ContinueDecision;

    #[test]
    fn continue_decision_formatter_uses_stable_contract_strings() {
        assert_eq!(
            format_continue_decision(ContinueDecision::Continue),
            "Continue"
        );
        assert_eq!(format_continue_decision(ContinueDecision::Stop), "Stop");
        assert_eq!(format_continue_decision(ContinueDecision::Pivot), "Pivot");
    }

    #[test]
    fn task_node_state_formatter_uses_stable_contract_strings() {
        assert_eq!(
            format_task_node_state(MoatTaskNodeState::Pending),
            "pending"
        );
        assert_eq!(format_task_node_state(MoatTaskNodeState::Ready), "ready");
        assert_eq!(
            format_task_node_state(MoatTaskNodeState::InProgress),
            "in_progress"
        );
        assert_eq!(
            format_task_node_state(MoatTaskNodeState::Completed),
            "completed"
        );
        assert_eq!(
            format_task_node_state(MoatTaskNodeState::Blocked),
            "blocked"
        );
    }

    #[test]
    fn parse_command_maps_round_and_control_plane_overrides() {
        assert_eq!(
            parse_command(&[
                "moat".into(),
                "round".into(),
                "--review-loops".into(),
                "0".into(),
            ])
            .unwrap(),
            CliCommand::MoatRound(MoatRoundOverrides {
                review_loops: Some(0),
                ..MoatRoundOverrides::default()
            })
        );
        assert_eq!(
            parse_command(&[
                "moat".into(),
                "control-plane".into(),
                "--strategy-candidates".into(),
                "0".into(),
            ])
            .unwrap(),
            CliCommand::MoatControlPlane(MoatRoundOverrides {
                strategy_candidates: Some(0),
                ..MoatRoundOverrides::default()
            })
        );
    }
}
```

- [ ] **Step 4: Run focused and broader CLI verification to verify GREEN**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli
cargo test -p mdid-cli
```

Expected: PASS.

- [ ] **Step 5: Commit the CLI slice**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs
git commit -m "feat: add bounded moat cli overrides"
```

### Task 2: Truth-sync docs for the shipped bounded runner contract

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
- Modify: `docs/superpowers/plans/2026-04-25-med-de-id-moat-cli-bounded-runner.md`

- [ ] **Step 1: Write the failing docs expectations as a repository consistency check**

Run this one-off check first:

```bash
python - <<'PY'
from pathlib import Path
readme = Path('README.md').read_text()
spec = Path('docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md').read_text()
plan = Path('docs/superpowers/plans/2026-04-25-med-de-id-moat-cli-bounded-runner.md').read_text()
required = {
    'README.md': [
        'stop_reason=<none>|...',
        'cargo run -p mdid-cli -- moat round --review-loops 0',
        'cargo run -p mdid-cli -- moat control-plane --strategy-candidates 0',
    ],
    'spec': [
        'bounded operator-facing `mdid-cli moat round` runner',
        'override flags',
        'stop_reason',
        'not persisted, not live, and not autonomous over external data',
    ],
    'plan': [
        'Option<u8>',
        'parse_u8_flag',
        '.parse::<u8>()',
    ],
}
missing = []
for item in required['README.md']:
    if item not in readme:
        missing.append(f'README missing {item!r}')
for item in required['spec']:
    if item not in spec:
        missing.append(f'spec missing {item!r}')
for item in required['plan']:
    if item not in plan:
        missing.append(f'plan missing {item!r}')
stale_tokens = ['Option<u' + '16>', 'parse_u' + '16_flag', '.parse::<u' + '16>()']
if any(token in plan for token in stale_tokens):
    missing.append('plan still contains stale u16 override contract')
if not missing:
    raise SystemExit('expected docs check to fail before docs are updated')
print('\n'.join(missing))
PY
```

Expected: FAIL-ish check output listing the missing README/spec/plan strings and any stale `u16` override references.

- [ ] **Step 2: Update README to document the bounded runner**

Update the moat-loop section in `README.md` to:

```md
## Moat Loop Foundation

`med-de-id` now includes a local-first moat-loop foundation for deterministic bounded strategy rounds. The shipped slice models market snapshots, competitor profiles, lock-in analysis artifacts, moat strategies, deterministic moat scoring, and a bounded control-plane snapshot for canonical task-state inspection through the CLI.

Run the default bounded round with:

```bash
cargo run -p mdid-cli -- moat round
```

The round command prints a deterministic report containing:

- `continue_decision=Continue|Stop|Pivot`
- `executed_tasks=market_scan,competitor_analysis,lockin_analysis,strategy_generation,spec_planning,implementation,review,evaluation`
- `moat_score_before`
- `moat_score_after`
- `stop_reason=<none>|...`

Run bounded stop-path scenarios by overriding sample budgets, for example:

```bash
cargo run -p mdid-cli -- moat round --review-loops 0
cargo run -p mdid-cli -- moat control-plane --strategy-candidates 0
```

These override flags make the CLI a bounded operator-facing runner over deterministic sample inputs, but it is still intentionally narrow. It does not yet perform live market crawling, persistent memory storage, PR automation, scheduler control, or unrestricted autonomous iteration over external data.
```

- [ ] **Step 3: Update the spec implementation-status section**

Change the current implementation-status wording in `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md` so the shipped-slice bullets say:

```md
- a bounded operator-facing `mdid-cli moat round` runner over deterministic sample inputs, including override flags for strategy/spec/implementation/review budgets plus `tests_passed`, and honest `stop_reason` reporting
- a bounded operator-facing `mdid-cli moat control-plane` runner over the same deterministic sample inputs, including override flags for stop-path inspection through canonical task states, ready-node visibility, and the latest bounded decision-memory summary
```

And change the paragraph below them to:

```md
This shipped slice is intentionally narrower than the full autonomous moat-loop vision. It provides a deterministic single-round foundation for evaluating and inspecting moat work locally through both the round report and control-plane snapshot, and now exposes bounded operator-facing override flags over deterministic sample data. It is still not persisted, not live, and not autonomous over external data: there is no scheduler control, no live market crawling, no persistent memory store, and no fully autonomous multi-agent runtime over user-supplied or external inputs.
```

- [ ] **Step 4: Truth-sync this plan to the landed `u8` override contract**

Patch this plan so every implementation snippet and verification check matches the shipped CLI contract:

- `MoatRoundOverrides` budget overrides use `Option<u8>`
- numeric override parsing uses `parse_u8_flag` and `.parse::<u8>()`
- Task 2 checks cover `README.md`, the moat-loop spec, and this plan file itself
- no prose claims persistence, live crawling, scheduler control, or autonomous execution over external data

- [ ] **Step 5: Run docs verification after the edits**

Run:

```bash
python - <<'PY'
from pathlib import Path
readme = Path('README.md').read_text()
spec = Path('docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md').read_text()
plan = Path('docs/superpowers/plans/2026-04-25-med-de-id-moat-cli-bounded-runner.md').read_text()
required = {
    'README.md': [
        'stop_reason=<none>|...',
        'cargo run -p mdid-cli -- moat round --review-loops 0',
        'cargo run -p mdid-cli -- moat control-plane --strategy-candidates 0',
    ],
    'spec': [
        'bounded operator-facing `mdid-cli moat round` runner',
        'override flags',
        'stop_reason',
        'not persisted, not live, and not autonomous over external data',
    ],
    'plan': [
        'Option<u8>',
        'parse_u8_flag',
        '.parse::<u8>()',
    ],
}
missing = []
for item in required['README.md']:
    if item not in readme:
        missing.append(f'README missing {item!r}')
for item in required['spec']:
    if item not in spec:
        missing.append(f'spec missing {item!r}')
for item in required['plan']:
    if item not in plan:
        missing.append(f'plan missing {item!r}')
stale_tokens = ['Option<u' + '16>', 'parse_u' + '16_flag', '.parse::<u' + '16>()']
if any(token in plan for token in stale_tokens):
    missing.append('plan still contains stale u16 override contract')
if missing:
    raise SystemExit('\n'.join(missing))
print('docs truth-sync check passed')
PY
```

Expected: PASS with `docs truth-sync check passed`.

- [ ] **Step 6: Commit the docs truth-sync**

```bash
git add README.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-25-med-de-id-moat-cli-bounded-runner.md
git commit -m "docs: truth-sync moat bounded runner"
```

### Task 3: Re-run the honest moat verification slice

**Files:**
- Modify: none

- [ ] **Step 1: Run the directly related test suite**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-runtime --test moat_runtime
cargo test -p mdid-cli
```

Expected: PASS.

- [ ] **Step 2: Run the bounded CLI commands manually**

Run:

```bash
source "$HOME/.cargo/env"
cargo run -q -p mdid-cli -- moat round
cargo run -q -p mdid-cli -- moat round --review-loops 0
cargo run -q -p mdid-cli -- moat control-plane --strategy-candidates 0
```

Expected:
- default round prints `stop_reason=<none>`
- review-budget-zero round prints `continue_decision=Stop` and `stop_reason=review budget exhausted`
- strategy-budget-zero control-plane prints `ready_nodes=strategy_generation`

- [ ] **Step 3: Record honest branch state**

Run:

```bash
git branch --show-current
git status --short
git log --oneline --decorate -5
```

Expected:
- branch is `feature/moat-loop-autonomy`
- worktree is clean after the task commits
- recent history includes the new CLI/docs commits for this slice
