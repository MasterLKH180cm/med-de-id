use mdid_application::{build_default_moat_task_graph, summarize_round_memory};
use mdid_domain::{
    AgentRole, ContinueDecision, DecisionLogEntry, MoatRoundSummary, MoatTaskNodeKind,
};
use uuid::Uuid;

#[test]
fn default_task_graph_assigns_expected_roles_and_dependencies() {
    let graph = build_default_moat_task_graph(Uuid::nil());

    assert_eq!(graph.nodes.len(), 7);

    let spec_planning = graph
        .nodes
        .iter()
        .find(|node| node.node_id == "spec_planning")
        .expect("spec_planning node should exist");
    assert_eq!(spec_planning.role, AgentRole::Planner);
    assert_eq!(spec_planning.kind, MoatTaskNodeKind::SpecPlanning);
    assert_eq!(
        spec_planning.depends_on,
        vec!["strategy_generation".to_string()]
    );
    assert_eq!(
        spec_planning.spec_ref.as_deref(),
        Some("docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md")
    );

    let actual_shape = graph
        .nodes
        .iter()
        .map(|node| {
            (
                node.node_id.as_str(),
                node.role,
                node.kind,
                node.depends_on
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        actual_shape,
        vec![
            (
                "market_scan",
                AgentRole::Planner,
                MoatTaskNodeKind::MarketScan,
                vec![],
            ),
            (
                "competitor_analysis",
                AgentRole::Planner,
                MoatTaskNodeKind::CompetitorAnalysis,
                vec![],
            ),
            (
                "lockin_analysis",
                AgentRole::Planner,
                MoatTaskNodeKind::LockInAnalysis,
                vec![],
            ),
            (
                "strategy_generation",
                AgentRole::Planner,
                MoatTaskNodeKind::StrategyGeneration,
                vec!["market_scan", "competitor_analysis", "lockin_analysis"],
            ),
            (
                "spec_planning",
                AgentRole::Planner,
                MoatTaskNodeKind::SpecPlanning,
                vec!["strategy_generation"],
            ),
            (
                "implementation",
                AgentRole::Coder,
                MoatTaskNodeKind::Implementation,
                vec!["spec_planning"],
            ),
            (
                "review",
                AgentRole::Reviewer,
                MoatTaskNodeKind::Review,
                vec!["implementation"],
            ),
        ]
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
