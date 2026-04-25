# Moat Control Plane Snapshot Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a truthful bounded moat-loop control-plane snapshot that exposes canonical task-graph progress and latest decision memory through runtime and CLI.

**Architecture:** Keep the new slice local-first and deterministic. `mdid-application` will own the pure helper that projects canonical task-graph state from executed stages, `mdid-runtime` will attach a control-plane snapshot to bounded round reports, and `mdid-cli` will expose a new `moat control-plane` command while keeping the sample round runner deterministic.

**Tech Stack:** Rust workspace, Cargo, mdid-application, mdid-runtime, mdid-cli, mdid-domain, markdown docs.

---

## Scope note

This slice intentionally stays bounded. It adds:
- canonical moat task-graph progress projection
- an explicit `evaluation` node in the default graph
- runtime control-plane snapshots with decision-memory summaries
- a deterministic `mdid-cli moat control-plane` command
- docs truth-sync for the new bounded inspection surface

This slice does **not** add:
- persistent storage backends
- live market crawling
- background scheduling
- GitHub PR automation
- unrestricted autonomous looping

## File structure

**Modify:**
- `crates/mdid-application/src/lib.rs`
- `crates/mdid-application/tests/moat_control_plane.rs`
- `crates/mdid-runtime/src/moat.rs`
- `crates/mdid-runtime/tests/moat_runtime.rs`
- `crates/mdid-cli/src/main.rs`
- `crates/mdid-cli/tests/moat_cli.rs`
- `README.md`
- `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`

---

### Task 1: Add canonical task-graph projection helpers in `mdid-application`

**Files:**
- Modify: `crates/mdid-application/src/lib.rs`
- Test: `crates/mdid-application/tests/moat_control_plane.rs`

- [ ] **Step 1: Write the failing application tests**

Append these tests to `crates/mdid-application/tests/moat_control_plane.rs`:

```rust
#[test]
fn default_task_graph_includes_review_and_evaluation_chain() {
    let graph = build_default_moat_task_graph(Uuid::nil());

    assert_eq!(graph.nodes.len(), 8);

    let review = graph
        .nodes
        .iter()
        .find(|node| node.node_id == "review")
        .expect("review node should exist");
    assert_eq!(review.depends_on, vec!["implementation".to_string()]);

    let evaluation = graph
        .nodes
        .iter()
        .find(|node| node.node_id == "evaluation")
        .expect("evaluation node should exist");
    assert_eq!(evaluation.role, AgentRole::Reviewer);
    assert_eq!(evaluation.kind, MoatTaskNodeKind::Evaluation);
    assert_eq!(evaluation.depends_on, vec!["review".to_string()]);
}

#[test]
fn project_task_graph_progress_marks_completed_and_ready_nodes() {
    let graph = build_default_moat_task_graph(Uuid::nil());
    let executed_tasks = vec![
        "market_scan".to_string(),
        "competitor_analysis".to_string(),
        "lockin_analysis".to_string(),
        "strategy_generation".to_string(),
        "spec_planning".to_string(),
        "implementation".to_string(),
    ];

    let projected = project_task_graph_progress(graph, &executed_tasks);

    let actual_states = projected
        .nodes
        .iter()
        .map(|node| (node.node_id.as_str(), node.state))
        .collect::<Vec<_>>();

    assert_eq!(
        actual_states,
        vec![
            ("market_scan", MoatTaskNodeState::Completed),
            ("competitor_analysis", MoatTaskNodeState::Completed),
            ("lockin_analysis", MoatTaskNodeState::Completed),
            ("strategy_generation", MoatTaskNodeState::Completed),
            ("spec_planning", MoatTaskNodeState::Completed),
            ("implementation", MoatTaskNodeState::Completed),
            ("review", MoatTaskNodeState::Ready),
            ("evaluation", MoatTaskNodeState::Pending),
        ]
    );
    assert_eq!(projected.ready_node_ids(), vec!["review".to_string()]);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-application --test moat_control_plane
```

Expected: FAIL because the default graph still has 7 nodes and `project_task_graph_progress` does not exist yet.

- [ ] **Step 3: Write the minimal implementation**

Update `crates/mdid-application/src/lib.rs` so it exports the new helper and appends the evaluation node:

```rust
pub fn build_default_moat_task_graph(round_id: Uuid) -> MoatTaskGraph {
    MoatTaskGraph {
        round_id,
        nodes: vec![
            MoatTaskNode {
                node_id: "market_scan".into(),
                title: "Market Scan".into(),
                role: AgentRole::Planner,
                kind: MoatTaskNodeKind::MarketScan,
                state: MoatTaskNodeState::Pending,
                depends_on: vec![],
                spec_ref: None,
            },
            MoatTaskNode {
                node_id: "competitor_analysis".into(),
                title: "Competitor Analysis".into(),
                role: AgentRole::Planner,
                kind: MoatTaskNodeKind::CompetitorAnalysis,
                state: MoatTaskNodeState::Pending,
                depends_on: vec![],
                spec_ref: None,
            },
            MoatTaskNode {
                node_id: "lockin_analysis".into(),
                title: "Lock-In Analysis".into(),
                role: AgentRole::Planner,
                kind: MoatTaskNodeKind::LockInAnalysis,
                state: MoatTaskNodeState::Pending,
                depends_on: vec![],
                spec_ref: None,
            },
            MoatTaskNode {
                node_id: "strategy_generation".into(),
                title: "Strategy Generation".into(),
                role: AgentRole::Planner,
                kind: MoatTaskNodeKind::StrategyGeneration,
                state: MoatTaskNodeState::Pending,
                depends_on: vec![
                    "market_scan".into(),
                    "competitor_analysis".into(),
                    "lockin_analysis".into(),
                ],
                spec_ref: None,
            },
            MoatTaskNode {
                node_id: "spec_planning".into(),
                title: "Spec Planning".into(),
                role: AgentRole::Planner,
                kind: MoatTaskNodeKind::SpecPlanning,
                state: MoatTaskNodeState::Pending,
                depends_on: vec!["strategy_generation".into()],
                spec_ref: Some(
                    "docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md".into(),
                ),
            },
            MoatTaskNode {
                node_id: "implementation".into(),
                title: "Implementation".into(),
                role: AgentRole::Coder,
                kind: MoatTaskNodeKind::Implementation,
                state: MoatTaskNodeState::Pending,
                depends_on: vec!["spec_planning".into()],
                spec_ref: None,
            },
            MoatTaskNode {
                node_id: "review".into(),
                title: "Review".into(),
                role: AgentRole::Reviewer,
                kind: MoatTaskNodeKind::Review,
                state: MoatTaskNodeState::Pending,
                depends_on: vec!["implementation".into()],
                spec_ref: None,
            },
            MoatTaskNode {
                node_id: "evaluation".into(),
                title: "Evaluation".into(),
                role: AgentRole::Reviewer,
                kind: MoatTaskNodeKind::Evaluation,
                state: MoatTaskNodeState::Pending,
                depends_on: vec!["review".into()],
                spec_ref: None,
            },
        ],
    }
}

pub fn project_task_graph_progress(
    mut graph: MoatTaskGraph,
    executed_tasks: &[String],
) -> MoatTaskGraph {
    for node in &mut graph.nodes {
        if executed_tasks.iter().any(|executed| executed == &node.node_id) {
            node.state = MoatTaskNodeState::Completed;
        }
    }

    let completed_ids = graph
        .nodes
        .iter()
        .filter(|node| node.state == MoatTaskNodeState::Completed)
        .map(|node| node.node_id.clone())
        .collect::<std::collections::BTreeSet<_>>();

    for node in &mut graph.nodes {
        if node.state == MoatTaskNodeState::Pending
            && node.depends_on.iter().all(|dependency| completed_ids.contains(dependency))
        {
            node.state = MoatTaskNodeState::Ready;
        }
    }

    graph
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-application --test moat_control_plane
cargo test -p mdid-application
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-application/src/lib.rs crates/mdid-application/tests/moat_control_plane.rs
git commit -m "feat: add moat control plane graph projection"
```

### Task 2: Attach a bounded control-plane snapshot to moat runtime reports

**Files:**
- Modify: `crates/mdid-runtime/src/moat.rs`
- Test: `crates/mdid-runtime/tests/moat_runtime.rs`

**Task truth-sync note:**
This batch also enforces `max_review_loops` honestly. When review budget is zero, the runtime must stop after `implementation`, report `review budget exhausted`, and expose `review` as the next ready control-plane node instead of pretending `review` and `evaluation` already ran.

- [ ] **Step 1: Write the failing runtime tests**

Update `crates/mdid-runtime/tests/moat_runtime.rs` with these assertions:

```rust
#[test]
fn bounded_round_returns_control_plane_snapshot_for_successful_rounds() {
    let report = run_bounded_round(MoatRoundInput {
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
    });

    assert_eq!(
        report.executed_tasks,
        vec![
            "market_scan".to_string(),
            "competitor_analysis".to_string(),
            "lockin_analysis".to_string(),
            "strategy_generation".to_string(),
            "spec_planning".to_string(),
            "implementation".to_string(),
            "review".to_string(),
            "evaluation".to_string(),
        ]
    );
    assert!(report.control_plane.task_graph.ready_node_ids().is_empty());
    assert_eq!(
        report
            .control_plane
            .memory
            .latest_decision_summary()
            .as_deref(),
        Some("review approved bounded moat round")
    );
    assert_eq!(report.control_plane.memory.improvement_delta, 8);
}

#[test]
fn bounded_round_exposes_ready_strategy_generation_when_budget_stops_early() {
    let report = run_bounded_round(MoatRoundInput {
        market: MarketMoatSnapshot::default(),
        competitor: CompetitorProfile::default(),
        lock_in: LockInReport::default(),
        strategies: vec![MoatStrategy {
            strategy_id: "data-room".into(),
            title: "Data moat".into(),
            target_moat_type: MoatType::DataMoat,
            implementation_cost: 1,
            expected_moat_gain: 4,
            ..MoatStrategy::default()
        }],
        budget: ResourceBudget {
            max_round_minutes: 10,
            max_parallel_tasks: 1,
            max_strategy_candidates: 0,
            max_spec_generations: 0,
            max_implementation_tasks: 0,
            max_review_loops: 0,
        },
        improvement_threshold: 2,
        tests_passed: true,
    });

    assert_eq!(
        report.control_plane.task_graph.ready_node_ids(),
        vec!["strategy_generation".to_string()]
    );
    assert_eq!(
        report
            .control_plane
            .memory
            .latest_decision_summary()
            .as_deref(),
        Some("review stopped bounded moat round")
    );
}

#[test]
fn bounded_round_stops_before_review_when_review_budget_is_zero() {
    let report = run_bounded_round(MoatRoundInput {
        market: MarketMoatSnapshot {
            moat_score: 45,
            ..MarketMoatSnapshot::default()
        },
        competitor: CompetitorProfile {
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
            max_review_loops: 0,
        },
        improvement_threshold: 3,
        tests_passed: true,
    });

    assert_eq!(
        report.executed_tasks,
        vec![
            "market_scan".to_string(),
            "competitor_analysis".to_string(),
            "lockin_analysis".to_string(),
            "strategy_generation".to_string(),
            "spec_planning".to_string(),
            "implementation".to_string(),
        ]
    );
    assert_eq!(report.stop_reason.as_deref(), Some("review budget exhausted"));
    assert_eq!(
        report.control_plane.task_graph.ready_node_ids(),
        vec!["review".to_string()]
    );
    assert_eq!(
        report
            .control_plane
            .memory
            .latest_decision_summary()
            .as_deref(),
        Some("review stopped bounded moat round")
    );
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-runtime --test moat_runtime
```

Expected: FAIL because `MoatRoundReport` does not yet expose `control_plane`, and the runtime still emits the older stage names.

- [ ] **Step 3: Write the minimal runtime implementation**

Update `crates/mdid-runtime/src/moat.rs` to attach a deterministic control-plane snapshot:

```rust
use chrono::Utc;
use mdid_application::{
    build_default_moat_task_graph, evaluate_moat_round, project_task_graph_progress,
    select_top_strategies, summarize_round_memory, MoatImprovementThreshold,
};
use mdid_domain::{
    AgentRole, CompetitorProfile, ContinueDecision, DecisionLogEntry, LockInReport,
    MarketMoatSnapshot, MoatMemorySnapshot, MoatTaskGraph, MoatStrategy, ResourceBudget,
};

const STRATEGY_GENERATION: &str = "strategy_generation";
const SPEC_PLANNING: &str = "spec_planning";
const IMPLEMENTATION: &str = "implementation";
const REVIEW: &str = "review";
const EVALUATION: &str = "evaluation";

#[derive(Debug, Clone)]
pub struct MoatControlPlaneReport {
    pub task_graph: MoatTaskGraph,
    pub memory_snapshot: MoatMemorySnapshot,
}

#[derive(Debug, Clone)]
pub struct MoatRoundReport {
    pub summary: mdid_domain::MoatRoundSummary,
    pub executed_tasks: Vec<String>,
    pub stop_reason: Option<String>,
    pub control_plane: MoatControlPlaneReport,
}

fn build_control_plane(
    summary: &mdid_domain::MoatRoundSummary,
    executed_tasks: &[String],
) -> MoatControlPlaneReport {
    let task_graph = project_task_graph_progress(build_default_moat_task_graph(summary.round_id), executed_tasks);
    let decisions = vec![DecisionLogEntry {
        entry_id: Uuid::new_v4(),
        round_id: summary.round_id,
        author_role: AgentRole::Reviewer,
        summary: if summary.continue_decision == ContinueDecision::Continue {
            "review approved bounded moat round".into()
        } else {
            "review stopped bounded moat round".into()
        },
        rationale: summary
            .stop_reason
            .clone()
            .unwrap_or_else(|| "tests passed and moat improved".into()),
        recorded_at: Utc::now(),
    }];

    MoatControlPlaneReport {
        task_graph,
        memory_snapshot: summarize_round_memory(summary, decisions),
    }
}
```

Then wire `run_bounded_round` and `stop_report` so they emit these canonical `executed_tasks` values:

```rust
vec![
    "market_scan".to_string(),
    "competitor_analysis".to_string(),
    "lockin_analysis".to_string(),
    "strategy_generation".to_string(),
    "spec_planning".to_string(),
    "implementation".to_string(),
    "review".to_string(),
    "evaluation".to_string(),
]
```

and assign `control_plane: build_control_plane(&summary, &executed_tasks)` in both the success path and the stop path.

- [ ] **Step 4: Run the tests to verify they pass**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-runtime --test moat_runtime
cargo test -p mdid-runtime
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-runtime/src/moat.rs crates/mdid-runtime/tests/moat_runtime.rs
git commit -m "feat: add moat runtime control plane snapshot"
```

### Task 3: Expose the control-plane snapshot through CLI and truth-sync docs

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`

- [ ] **Step 1: Write the failing CLI/docs tests**

Update `crates/mdid-cli/tests/moat_cli.rs` with this additional test and the new canonical `executed_tasks` expectation:

```rust
#[test]
fn cli_runs_moat_control_plane_and_prints_graph_snapshot() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "control-plane"])
        .output()
        .expect("failed to run mdid-cli moat control-plane");

    assert!(
        output.status.success(),
        "expected success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "moat control plane snapshot\n",
            "ready_nodes=<none>\n",
            "latest_decision_summary=review approved bounded moat round\n",
            "improvement_delta=8\n",
            "task_states=market_scan:completed,competitor_analysis:completed,lockin_analysis:completed,strategy_generation:completed,spec_planning:completed,implementation:completed,review:completed,evaluation:completed\n",
        )
    );
}
```

Update the existing round assertion in the same file so `executed_tasks=` becomes:

```text
executed_tasks=market_scan,competitor_analysis,lockin_analysis,strategy_generation,spec_planning,implementation,review,evaluation
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli
```

Expected: FAIL because the CLI does not yet support `moat control-plane` and still prints the older stage-name contract.

- [ ] **Step 3: Write the minimal CLI and docs implementation**

Update `crates/mdid-cli/src/main.rs` to support the new command and stable formatter helpers:

```rust
match args.as_slice() {
    [] => println!("med-de-id CLI ready"),
    [status] if status == "status" => println!("med-de-id CLI ready"),
    [moat, round] if moat == "moat" && round == "round" => run_moat_round(),
    [moat, control_plane] if moat == "moat" && control_plane == "control-plane" => {
        run_moat_control_plane()
    }
    _ => exit_unknown_command(&args),
}

fn run_moat_control_plane() {
    let report = run_bounded_round(sample_round_input());
    let ready_nodes = report.control_plane.task_graph.ready_node_ids();
    let task_states = report
        .control_plane
        .task_graph
        .nodes
        .iter()
        .map(|node| format!("{}:{}", node.node_id, format_task_state(node.state)))
        .collect::<Vec<_>>()
        .join(",");

    println!("moat control plane snapshot");
    println!(
        "ready_nodes={}",
        if ready_nodes.is_empty() {
            "<none>".to_string()
        } else {
            ready_nodes.join(",")
        }
    );
    println!(
        "latest_decision_summary={}",
        report
            .control_plane
            .memory_snapshot
            .latest_decision_summary()
            .unwrap_or_else(|| "<none>".into())
    );
    println!(
        "improvement_delta={}",
        report.control_plane.memory_snapshot.improvement_delta
    );
    println!("task_states={task_states}");
}

fn format_task_state(state: mdid_domain::MoatTaskNodeState) -> &'static str {
    match state {
        mdid_domain::MoatTaskNodeState::Pending => "pending",
        mdid_domain::MoatTaskNodeState::Ready => "ready",
        mdid_domain::MoatTaskNodeState::InProgress => "in_progress",
        mdid_domain::MoatTaskNodeState::Completed => "completed",
        mdid_domain::MoatTaskNodeState::Blocked => "blocked",
    }
}
```

Truth-sync `README.md` by adding this command under `## Moat Loop Foundation`:

```md
Inspect the bounded control-plane snapshot with:

```bash
cargo run -p mdid-cli -- moat control-plane
```

The command prints:
- ready task-graph nodes
- the latest deterministic review decision summary
- the improvement delta from the bounded round
- canonical task states for `market_scan`, `competitor_analysis`, `lockin_analysis`, `strategy_generation`, `spec_planning`, `implementation`, `review`, and `evaluation`
```

Truth-sync `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md` by expanding the shipped-slice section with this bullet:

```md
- deterministic CLI inspection commands for both `mdid-cli moat round` and `mdid-cli moat control-plane`, using canonical planner/coder/reviewer/evaluation task IDs and a bounded decision-memory snapshot
```

- [ ] **Step 4: Run the tests and command verification**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli
cargo run -q -p mdid-cli -- moat control-plane
cargo test --workspace
```

Expected: PASS, and the control-plane command prints the exact deterministic snapshot string from Step 1.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs README.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md
git commit -m "feat: expose moat control plane snapshot"
```

## Self-review

- Spec coverage: the plan covers canonical task-graph projection, runtime control-plane reporting, CLI exposure, and docs truth-sync for the bounded inspection slice.
- Placeholder scan: no TBD/TODO placeholders remain; every code step includes concrete snippets and commands.
- Type consistency: `evaluation`, `project_task_graph_progress`, `MoatControlPlaneReport`, and `moat control-plane` are named consistently across all tasks.
