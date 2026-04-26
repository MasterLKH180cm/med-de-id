# Moat Agent Assignment Projection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a deterministic, read-only Planner/Coder/Reviewer assignment projection for ready moat task-graph nodes.

**Architecture:** The application crate owns the pure projection from `MoatTaskGraph` plus `ResourceBudget` into ready agent assignments. Runtime embeds that projection in `MoatControlPlaneReport` without executing tasks or spawning agents. CLI renders a stable `agent_assignments=` line on the existing `moat control-plane` surface.

**Tech Stack:** Rust workspace, `mdid-domain`, `mdid-application`, `mdid-runtime`, `mdid-cli`, Cargo integration tests.

---

## File Structure

- Modify: `crates/mdid-application/src/lib.rs`
  - Add `MoatAgentAssignment` DTO and `project_ready_moat_agent_assignments(graph, budget)` pure helper.
- Modify: `crates/mdid-application/tests/moat_control_plane.rs`
  - Add TDD coverage for ready-node grouping, dependency progression, and `max_parallel_tasks` limits.
- Modify: `crates/mdid-runtime/src/moat.rs`
  - Add `agent_assignments` to `MoatControlPlaneReport` and populate it from the final task graph and active round budget.
- Modify: `crates/mdid-runtime/tests/moat_runtime.rs`
  - Verify success has no pending assignments and bounded stop paths expose the correct next agent assignment.
- Modify: `crates/mdid-cli/src/main.rs`
  - Render `agent_assignments=<none>` or comma-separated `role:node_id` values from the control-plane report.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Verify sample control-plane output exposes deterministic ready assignments for planner/reviewer stop paths.
- Modify: `README.md`
  - Document that control-plane output now includes inspection-only `agent_assignments` and still does not launch agents.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Truth-sync current status: first-class read-only Planner/Coder/Reviewer assignment projection is shipped; autonomous execution remains future work.

---

### Task 1: Application assignment projection

**Files:**
- Modify: `crates/mdid-application/tests/moat_control_plane.rs`
- Modify: `crates/mdid-application/src/lib.rs`

- [ ] **Step 1: Write failing tests**

Append these tests to `crates/mdid-application/tests/moat_control_plane.rs`:

```rust
use mdid_application::{
    build_default_moat_task_graph, project_ready_moat_agent_assignments,
    project_task_graph_progress,
};
use mdid_domain::{AgentRole, ResourceBudget};

#[test]
fn agent_assignment_projection_groups_initial_ready_nodes_by_role() {
    let graph = build_default_moat_task_graph();
    let budget = ResourceBudget::default();

    let assignments = project_ready_moat_agent_assignments(&graph, &budget);

    let observed: Vec<_> = assignments
        .iter()
        .map(|assignment| (assignment.role.clone(), assignment.node_id.as_str()))
        .collect();

    assert_eq!(
        observed,
        vec![
            (AgentRole::Planner, "market_scan"),
            (AgentRole::Planner, "competitor_analysis"),
            (AgentRole::Planner, "lockin_analysis"),
        ]
    );
}

#[test]
fn agent_assignment_projection_respects_dependencies() {
    let graph = build_default_moat_task_graph();
    let progressed = project_task_graph_progress(
        &graph,
        &[
            "market_scan",
            "competitor_analysis",
            "lockin_analysis",
            "strategy_generation",
            "spec_planning",
        ],
    );
    let budget = ResourceBudget::default();

    let assignments = project_ready_moat_agent_assignments(&progressed, &budget);

    let observed: Vec<_> = assignments
        .iter()
        .map(|assignment| (assignment.role.clone(), assignment.node_id.as_str()))
        .collect();

    assert_eq!(observed, vec![(AgentRole::Coder, "implementation")]);
}

#[test]
fn agent_assignment_projection_respects_max_parallel_tasks() {
    let graph = build_default_moat_task_graph();
    let budget = ResourceBudget {
        max_parallel_tasks: 2,
        ..ResourceBudget::default()
    };

    let assignments = project_ready_moat_agent_assignments(&graph, &budget);

    let observed: Vec<_> = assignments
        .iter()
        .map(|assignment| assignment.node_id.as_str())
        .collect();

    assert_eq!(observed, vec!["market_scan", "competitor_analysis"]);
}

#[test]
fn agent_assignment_projection_returns_no_assignments_when_parallel_budget_is_zero() {
    let graph = build_default_moat_task_graph();
    let budget = ResourceBudget {
        max_parallel_tasks: 0,
        ..ResourceBudget::default()
    };

    let assignments = project_ready_moat_agent_assignments(&graph, &budget);

    assert!(assignments.is_empty());
}
```

- [ ] **Step 2: Run RED test**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-application agent_assignment_projection -- --nocapture
```

Expected: FAIL because `project_ready_moat_agent_assignments` does not exist.

- [ ] **Step 3: Implement minimal application projection**

In `crates/mdid-application/src/lib.rs`, extend the domain imports to include `AgentRole` and `MoatTaskNodeKind` if not already imported, then add:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoatAgentAssignment {
    pub role: AgentRole,
    pub node_id: String,
    pub title: String,
    pub kind: MoatTaskNodeKind,
    pub spec_ref: Option<String>,
}

pub fn project_ready_moat_agent_assignments(
    graph: &MoatTaskGraph,
    budget: &ResourceBudget,
) -> Vec<MoatAgentAssignment> {
    if budget.max_parallel_tasks == 0 {
        return Vec::new();
    }

    let ready_node_ids = graph.ready_node_ids();

    graph
        .nodes
        .iter()
        .filter(|node| ready_node_ids.iter().any(|ready_id| ready_id == &node.node_id))
        .take(budget.max_parallel_tasks)
        .map(|node| MoatAgentAssignment {
            role: node.role.clone(),
            node_id: node.node_id.clone(),
            title: node.title.clone(),
            kind: node.kind.clone(),
            spec_ref: node.spec_ref.clone(),
        })
        .collect()
}
```

- [ ] **Step 4: Run GREEN tests**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-application agent_assignment_projection -- --nocapture
cargo test -p mdid-application --test moat_control_plane
```

Expected: PASS.

- [ ] **Step 5: Commit application projection**

```bash
git add crates/mdid-application/src/lib.rs crates/mdid-application/tests/moat_control_plane.rs docs/superpowers/plans/2026-04-26-med-de-id-moat-agent-assignment-projection.md
git commit -m "feat: project ready moat agent assignments"
```

---

### Task 2: Runtime control-plane assignment report

**Files:**
- Modify: `crates/mdid-runtime/tests/moat_runtime.rs`
- Modify: `crates/mdid-runtime/src/moat.rs`

- [ ] **Step 1: Write failing runtime tests**

Append these tests to `crates/mdid-runtime/tests/moat_runtime.rs`:

```rust
use mdid_domain::AgentRole;
use mdid_runtime::{run_bounded_round, MoatRoundInput};

#[test]
fn successful_round_has_no_pending_agent_assignments() {
    let report = run_bounded_round(MoatRoundInput::default()).expect("round should run");

    assert!(report.control_plane.agent_assignments.is_empty());
}

#[test]
fn strategy_budget_stop_exposes_planner_assignment() {
    let input = MoatRoundInput {
        strategy_candidates: 0,
        ..MoatRoundInput::default()
    };

    let report = run_bounded_round(input).expect("round should stop cleanly");

    let observed: Vec<_> = report
        .control_plane
        .agent_assignments
        .iter()
        .map(|assignment| (assignment.role.clone(), assignment.node_id.as_str()))
        .collect();

    assert_eq!(observed, vec![(AgentRole::Planner, "strategy_generation")]);
}

#[test]
fn review_budget_stop_exposes_reviewer_assignment() {
    let input = MoatRoundInput {
        review_loops: 0,
        ..MoatRoundInput::default()
    };

    let report = run_bounded_round(input).expect("round should stop cleanly");

    let observed: Vec<_> = report
        .control_plane
        .agent_assignments
        .iter()
        .map(|assignment| (assignment.role.clone(), assignment.node_id.as_str()))
        .collect();

    assert_eq!(observed, vec![(AgentRole::Reviewer, "review")]);
}
```

- [ ] **Step 2: Run RED test**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-runtime agent_assignments -- --nocapture
```

Expected: FAIL because `MoatControlPlaneReport` has no `agent_assignments` field.

- [ ] **Step 3: Implement runtime report field**

In `crates/mdid-runtime/src/moat.rs`:

1. Import `MoatAgentAssignment` and `project_ready_moat_agent_assignments` from `mdid_application`.
2. Add `pub agent_assignments: Vec<MoatAgentAssignment>,` to `MoatControlPlaneReport`.
3. Change the internal report builder to accept the active `ResourceBudget` and populate the field:

```rust
let agent_assignments = project_ready_moat_agent_assignments(&task_graph, budget);

MoatControlPlaneReport {
    task_graph,
    memory,
    agent_assignments,
}
```

- [ ] **Step 4: Run GREEN tests**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-runtime agent_assignments -- --nocapture
cargo test -p mdid-runtime --test moat_runtime
```

Expected: PASS.

- [ ] **Step 5: Commit runtime integration**

```bash
git add crates/mdid-runtime/src/moat.rs crates/mdid-runtime/tests/moat_runtime.rs
git commit -m "feat: include moat agent assignments in control plane"
```

---

### Task 3: CLI control-plane assignment output and docs

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`

- [ ] **Step 1: Write failing CLI tests**

Add assertions to existing control-plane tests in `crates/mdid-cli/tests/moat_cli.rs`, or append these focused tests if equivalent tests do not exist:

```rust
#[test]
fn moat_control_plane_prints_planner_agent_assignment() {
    let output = assert_cmd::Command::cargo_bin("mdid")
        .expect("binary exists")
        .args(["moat", "control-plane", "--strategy-candidates", "0"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).expect("stdout should be utf8");

    assert!(stdout.contains("agent_assignments=planner:strategy_generation"));
}

#[test]
fn moat_control_plane_prints_reviewer_agent_assignment() {
    let output = assert_cmd::Command::cargo_bin("mdid")
        .expect("binary exists")
        .args(["moat", "control-plane", "--review-loops", "0"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).expect("stdout should be utf8");

    assert!(stdout.contains("agent_assignments=reviewer:review"));
}
```

- [ ] **Step 2: Run RED test**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli moat_control_plane_prints_ -- --nocapture
```

Expected: FAIL because stdout does not contain `agent_assignments=`.

- [ ] **Step 3: Implement CLI renderer**

In `crates/mdid-cli/src/main.rs`, add helper functions near existing moat formatting helpers:

```rust
fn format_agent_role(role: &mdid_domain::AgentRole) -> &'static str {
    match role {
        mdid_domain::AgentRole::Planner => "planner",
        mdid_domain::AgentRole::Coder => "coder",
        mdid_domain::AgentRole::Reviewer => "reviewer",
    }
}

fn format_agent_assignments(assignments: &[mdid_application::MoatAgentAssignment]) -> String {
    if assignments.is_empty() {
        return "<none>".to_string();
    }

    assignments
        .iter()
        .map(|assignment| format!("{}:{}", format_agent_role(&assignment.role), assignment.node_id))
        .collect::<Vec<_>>()
        .join(",")
}
```

Then update the control-plane printer to include:

```rust
println!(
    "agent_assignments={}",
    format_agent_assignments(&report.agent_assignments)
);
```

- [ ] **Step 4: Update README and spec**

In `README.md`, update the moat control-plane section to state:

```markdown
The `agent_assignments` line is an inspection-only Planner/Coder/Reviewer handoff projection for ready task-graph nodes. It does not launch agents, write code, schedule background work, or append history.
```

In `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`, update current status to say the shipped foundation includes read-only Planner/Coder/Reviewer ready-assignment projection, while full autonomous role execution remains future work.

- [ ] **Step 5: Run GREEN tests**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli moat_control_plane_prints_ -- --nocapture
cargo test -p mdid-cli --test moat_cli
```

Expected: PASS.

- [ ] **Step 6: Run broader verification**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-application
cargo test -p mdid-runtime
cargo test -p mdid-cli
```

Expected: PASS.

- [ ] **Step 7: Commit CLI/docs**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs README.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md
git commit -m "feat: surface moat agent assignments in control plane"
```

---

## Self-Review

- Spec coverage: advances Autonomous Multi-Agent System by adding a safe, deterministic Planner/Coder/Reviewer assignment handoff without launching agents or background daemons.
- Placeholder scan: no TBD/TODO/fill-in-later placeholders; every code step includes concrete snippets and commands.
- Type consistency: `MoatAgentAssignment`, `project_ready_moat_agent_assignments`, `agent_assignments`, and CLI `agent_assignments=` are consistently named across tasks.
