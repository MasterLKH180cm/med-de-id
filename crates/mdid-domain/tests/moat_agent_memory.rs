use chrono::{DateTime, Utc};
use mdid_domain::{
    AgentRole, DecisionLogEntry, MoatMemorySnapshot, MoatTaskGraph, MoatTaskNode,
    MoatTaskNodeKind, MoatTaskNodeState,
};
use uuid::Uuid;

fn timestamp(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc)
}

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
fn task_graph_keeps_ready_nodes_when_dependencies_are_satisfied() {
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
                state: MoatTaskNodeState::Ready,
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
            recorded_at: timestamp("2026-04-25T16:00:00Z"),
        }],
    };

    assert_eq!(
        snapshot.latest_decision_summary().as_deref(),
        Some("approved workflow moat slice")
    );
}

#[test]
fn memory_snapshot_uses_latest_recorded_at_not_last_vector_position() {
    let snapshot = MoatMemorySnapshot {
        round_id: Uuid::nil(),
        latest_score: 98,
        improvement_delta: 8,
        decisions: vec![
            DecisionLogEntry {
                entry_id: Uuid::from_u128(1),
                round_id: Uuid::nil(),
                author_role: AgentRole::Planner,
                summary: "first draft".into(),
                rationale: "initial path".into(),
                recorded_at: timestamp("2026-04-25T15:00:00Z"),
            },
            DecisionLogEntry {
                entry_id: Uuid::from_u128(2),
                round_id: Uuid::nil(),
                author_role: AgentRole::Reviewer,
                summary: "final decision".into(),
                rationale: "highest confidence".into(),
                recorded_at: timestamp("2026-04-25T17:00:00Z"),
            },
            DecisionLogEntry {
                entry_id: Uuid::from_u128(3),
                round_id: Uuid::nil(),
                author_role: AgentRole::Coder,
                summary: "middle follow-up".into(),
                rationale: "logged later in vector".into(),
                recorded_at: timestamp("2026-04-25T16:00:00Z"),
            },
        ],
    };

    assert_eq!(
        snapshot.latest_decision_summary().as_deref(),
        Some("final decision")
    );
}

#[test]
fn memory_snapshot_returns_none_when_no_decisions_exist() {
    let snapshot = MoatMemorySnapshot {
        round_id: Uuid::nil(),
        latest_score: 98,
        improvement_delta: 8,
        decisions: vec![],
    };

    assert_eq!(snapshot.latest_decision_summary(), None);
}
