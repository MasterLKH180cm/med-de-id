use mdid_application::{build_default_moat_task_graph, summarize_round_memory};
use mdid_domain::{
    AgentRole, ContinueDecision, DecisionLogEntry, MoatRoundSummary, MoatTaskNodeKind,
};
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
    assert_eq!(
        graph.nodes[6].depends_on,
        vec!["implementation".to_string()]
    );
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
    assert_eq!(
        memory.latest_decision_summary().as_deref(),
        Some("approve strategy batch")
    );
    assert_eq!(memory.decisions, decisions);
}
