use mdid_application::{
    build_default_moat_task_graph, project_task_graph_progress, summarize_round_memory,
};
use mdid_domain::{
    AgentRole, ContinueDecision, DecisionLogEntry, MoatRoundSummary, MoatTaskNodeKind,
    MoatTaskNodeState,
};
use uuid::Uuid;

#[test]
fn default_task_graph_includes_review_and_evaluation_chain() {
    let graph = build_default_moat_task_graph(Uuid::nil());

    assert_eq!(graph.nodes.len(), 8);

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

    let review = graph
        .nodes
        .iter()
        .find(|node| node.node_id == "review")
        .expect("review node should exist");
    assert_eq!(review.role, AgentRole::Reviewer);
    assert_eq!(review.kind, MoatTaskNodeKind::Review);
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

    let states = projected
        .nodes
        .iter()
        .map(|node| (node.node_id.as_str(), node.state))
        .collect::<std::collections::BTreeMap<_, _>>();

    assert_eq!(
        states.get("market_scan"),
        Some(&MoatTaskNodeState::Completed)
    );
    assert_eq!(
        states.get("competitor_analysis"),
        Some(&MoatTaskNodeState::Completed)
    );
    assert_eq!(
        states.get("lockin_analysis"),
        Some(&MoatTaskNodeState::Completed)
    );
    assert_eq!(
        states.get("strategy_generation"),
        Some(&MoatTaskNodeState::Completed)
    );
    assert_eq!(
        states.get("spec_planning"),
        Some(&MoatTaskNodeState::Completed)
    );
    assert_eq!(
        states.get("implementation"),
        Some(&MoatTaskNodeState::Completed)
    );
    assert_eq!(states.get("review"), Some(&MoatTaskNodeState::Ready));
    assert_eq!(states.get("evaluation"), Some(&MoatTaskNodeState::Pending));
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
