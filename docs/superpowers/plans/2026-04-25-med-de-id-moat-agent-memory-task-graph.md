# med-de-id Moat Agent Memory Task Graph Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the first Planner/Coder/Reviewer + task-graph + memory-store foundation slice for the moat loop without pretending live crawling, persistence backends, or autonomous scheduling already exist.

**Architecture:** Keep this slice inside the existing workspace. `mdid-domain` will own durable vocabulary for agent roles, task-graph nodes, and round decision memory. `mdid-application` will add deterministic helpers that build the default moat task graph and summarize round memory from already-existing moat round outputs. This slice stays pure-data + pure-logic so later runtime persistence and scheduler work can build on it safely.

**Tech Stack:** Rust workspace, Cargo, mdid-domain, mdid-application, Serde, Chrono, UUID, existing test harness.

---

## Scope note

This slice is intentionally narrow. It adds:
- Planner / Coder / Reviewer role vocabulary
- task-graph node/state types for moat-loop work
- round decision-log and memory-snapshot structures
- deterministic application helpers to build the default bounded task graph and memory snapshot

This slice does **not** add:
- persistent database storage
- live web crawling
- background schedulers / cron control
- GitFlow PR automation
- runtime execution of multiple autonomous agents

## File structure

**Create:**
- `crates/mdid-domain/tests/moat_agent_memory.rs`
- `crates/mdid-application/tests/moat_control_plane.rs`

**Modify:**
- `crates/mdid-domain/src/lib.rs`
- `crates/mdid-application/src/lib.rs`

---

### Task 1: Add agent-role, task-graph, and decision-memory domain models

**Files:**
- Modify: `crates/mdid-domain/src/lib.rs`
- Create: `crates/mdid-domain/tests/moat_agent_memory.rs`

- [ ] **Step 1: Write the failing domain tests**

Create `crates/mdid-domain/tests/moat_agent_memory.rs`:

```rust
use mdid_domain::{
    AgentRole, DecisionLogEntry, MoatMemorySnapshot, MoatTaskGraph, MoatTaskNode,
    MoatTaskNodeKind, MoatTaskNodeState,
};
use uuid::Uuid;

#[test]
fn agent_role_wire_values_are_stable() {
    assert_eq!(serde_json::to_string(&AgentRole::Planner).unwrap(), "\"planner\"");
    assert_eq!(serde_json::to_string(&AgentRole::Coder).unwrap(), "\"coder\"");
    assert_eq!(serde_json::to_string(&AgentRole::Reviewer).unwrap(), "\"reviewer\"");
}

#[test]
fn task_graph_reports_ready_nodes_when_dependencies_are_satisfied() {
    let graph = MoatTaskGraph {
        round_id: Uuid::nil(),
        nodes: vec![
            MoatTaskNode {
                node_id: "market-scan".into(),
                title: "Market Scan".into(),
                role: AgentRole::Planner,
                kind: MoatTaskNodeKind::MarketScan,
                state: MoatTaskNodeState::Completed,
                depends_on: vec![],
                spec_ref: None,
            },
            MoatTaskNode {
                node_id: "strategy-gen".into(),
                title: "Strategy Generation".into(),
                role: AgentRole::Planner,
                kind: MoatTaskNodeKind::StrategyGeneration,
                state: MoatTaskNodeState::Pending,
                depends_on: vec!["market-scan".into()],
                spec_ref: Some("docs/spec.md".into()),
            },
        ],
    };

    assert_eq!(graph.ready_node_ids(), vec!["strategy-gen".to_string()]);
}

#[test]
fn memory_snapshot_exposes_latest_decision_summary() {
    let snapshot = MoatMemorySnapshot {
        round_id: Uuid::nil(),
        latest_score: 98,
        improvement_delta: 8,
        decisions: vec![DecisionLogEntry {
            entry_id: Uuid::nil(),
            round_id: Uuid::nil(),
            author_role: AgentRole::Reviewer,
            summary: "approved workflow moat slice".into(),
            rationale: "tests passed and score improved".into(),
            recorded_at: chrono::Utc::now(),
        }],
    };

    assert_eq!(snapshot.latest_decision_summary().as_deref(), Some("approved workflow moat slice"));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-domain --test moat_agent_memory
```

Expected: FAIL because the new agent/task-graph/memory types do not exist yet.

- [ ] **Step 3: Write the minimal domain implementation**

Append the new types to `crates/mdid-domain/src/lib.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    Planner,
    Coder,
    Reviewer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoatTaskNodeKind {
    MarketScan,
    CompetitorAnalysis,
    LockInAnalysis,
    StrategyGeneration,
    SpecPlanning,
    Implementation,
    Review,
    Evaluation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoatTaskNodeState {
    Pending,
    Ready,
    InProgress,
    Completed,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoatTaskNode {
    pub node_id: String,
    pub title: String,
    pub role: AgentRole,
    pub kind: MoatTaskNodeKind,
    pub state: MoatTaskNodeState,
    pub depends_on: Vec<String>,
    pub spec_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoatTaskGraph {
    pub round_id: Uuid,
    pub nodes: Vec<MoatTaskNode>,
}

impl MoatTaskGraph {
    pub fn ready_node_ids(&self) -> Vec<String> {
        self.nodes
            .iter()
            .filter(|node| node.state == MoatTaskNodeState::Pending)
            .filter(|node| {
                node.depends_on.iter().all(|dependency| {
                    self.nodes.iter().any(|candidate| {
                        candidate.node_id == *dependency && candidate.state == MoatTaskNodeState::Completed
                    })
                })
            })
            .map(|node| node.node_id.clone())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionLogEntry {
    pub entry_id: Uuid,
    pub round_id: Uuid,
    pub author_role: AgentRole,
    pub summary: String,
    pub rationale: String,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoatMemorySnapshot {
    pub round_id: Uuid,
    pub latest_score: i16,
    pub improvement_delta: i16,
    pub decisions: Vec<DecisionLogEntry>,
}

impl MoatMemorySnapshot {
    pub fn latest_decision_summary(&self) -> Option<String> {
        self.decisions.last().map(|entry| entry.summary.clone())
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-domain --test moat_agent_memory
cargo test -p mdid-domain
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-domain/src/lib.rs crates/mdid-domain/tests/moat_agent_memory.rs
git commit -m "feat: add moat agent memory domain models"
```

### Task 2: Add deterministic application helpers for task-graph construction and round memory snapshots

**Files:**
- Modify: `crates/mdid-application/src/lib.rs`
- Create: `crates/mdid-application/tests/moat_control_plane.rs`

- [ ] **Step 1: Write the failing application tests**

Create `crates/mdid-application/tests/moat_control_plane.rs`:

```rust
use mdid_application::{build_default_moat_task_graph, summarize_round_memory};
use mdid_domain::{AgentRole, ContinueDecision, DecisionLogEntry, MoatRoundSummary, MoatTaskNodeKind};
use uuid::Uuid;

#[test]
fn default_task_graph_assigns_expected_roles_and_dependencies() {
    let graph = build_default_moat_task_graph(Uuid::nil());

    assert_eq!(graph.nodes.len(), 7);
    assert_eq!(graph.nodes[0].role, AgentRole::Planner);
    assert_eq!(graph.nodes[0].kind, MoatTaskNodeKind::MarketScan);
    assert_eq!(graph.nodes[4].kind, MoatTaskNodeKind::SpecPlanning);
    assert_eq!(graph.nodes[5].role, AgentRole::Coder);
    assert_eq!(graph.nodes[6].role, AgentRole::Reviewer);
    assert_eq!(graph.nodes[6].depends_on, vec!["implementation".to_string()]);
}

#[test]
fn summarize_round_memory_captures_score_delta_and_latest_decision() {
    let summary = MoatRoundSummary {
        round_id: Uuid::nil(),
        moat_score_before: 90,
        moat_score_after: 98,
        continue_decision: ContinueDecision::Continue,
        ..MoatRoundSummary::default()
    };
    let decisions = vec![DecisionLogEntry {
        entry_id: Uuid::new_v4(),
        round_id: Uuid::nil(),
        author_role: AgentRole::Reviewer,
        summary: "approve strategy batch".into(),
        rationale: "improvement threshold cleared".into(),
        recorded_at: chrono::Utc::now(),
    }];

    let memory = summarize_round_memory(&summary, decisions.clone());

    assert_eq!(memory.latest_score, 98);
    assert_eq!(memory.improvement_delta, 8);
    assert_eq!(memory.latest_decision_summary().as_deref(), Some("approve strategy batch"));
    assert_eq!(memory.decisions, decisions);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-application --test moat_control_plane
```

Expected: FAIL because the helper functions do not exist yet.

- [ ] **Step 3: Write the minimal application implementation**

Add to `crates/mdid-application/src/lib.rs`:

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
                depends_on: vec!["market_scan".into(), "competitor_analysis".into(), "lockin_analysis".into()],
                spec_ref: None,
            },
            MoatTaskNode {
                node_id: "spec_planning".into(),
                title: "Spec Planning".into(),
                role: AgentRole::Planner,
                kind: MoatTaskNodeKind::SpecPlanning,
                state: MoatTaskNodeState::Pending,
                depends_on: vec!["strategy_generation".into()],
                spec_ref: Some("docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md".into()),
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
        ],
    }
}

pub fn summarize_round_memory(
    summary: &MoatRoundSummary,
    decisions: Vec<DecisionLogEntry>,
) -> MoatMemorySnapshot {
    MoatMemorySnapshot {
        round_id: summary.round_id,
        latest_score: summary.moat_score_after,
        improvement_delta: summary.improvement(),
        decisions,
    }
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
git commit -m "feat: add moat task graph memory helpers"
```
