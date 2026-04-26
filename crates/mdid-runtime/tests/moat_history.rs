use chrono::{DateTime, Utc};
use mdid_application::{build_default_moat_task_graph, summarize_round_memory};
use mdid_domain::{
    AgentRole, ContinueDecision, DecisionLogEntry, MoatRoundSummary, MoatTaskNodeState,
};
use mdid_runtime::{
    moat::{MoatControlPlaneReport, MoatRoundReport},
    moat_history::{LocalMoatHistoryStore, MoatHistorySummary},
};
use std::fs;
use tempfile::tempdir;
use uuid::Uuid;

#[test]
fn legacy_history_without_agent_assignments_opens_with_empty_assignments() {
    let dir = tempdir().expect("temp dir should exist");
    let history_path = dir.path().join("moat-history.json");
    let round_id = Uuid::new_v4();
    let recorded_at = recorded_at("2026-04-25T20:00:00Z");
    let report = sample_report(
        round_id,
        ContinueDecision::Continue,
        None,
        "legacy review approved bounded moat round",
        90,
        98,
        true,
        &[
            "market_scan",
            "competitor_analysis",
            "lockin_analysis",
            "strategy_generation",
            "spec_planning",
            "implementation",
            "review",
            "evaluation",
        ],
    );
    let mut legacy_entry = serde_json::json!({
        "recorded_at": recorded_at,
        "report": report,
    });
    legacy_entry["report"]["control_plane"]
        .as_object_mut()
        .expect("control plane should serialize as an object")
        .remove("agent_assignments");
    fs::write(
        &history_path,
        serde_json::to_vec_pretty(&vec![legacy_entry]).expect("legacy history should serialize"),
    )
    .expect("legacy history should be written");

    let store = LocalMoatHistoryStore::open_existing(&history_path)
        .expect("legacy history without agent assignments should open");
    let entries = store.entries();

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].recorded_at, recorded_at);
    assert_eq!(entries[0].report.summary.round_id, round_id);
    assert!(entries[0].report.control_plane.agent_assignments.is_empty());
}

#[test]
fn append_and_reload_keeps_rounds_sorted_by_recorded_at() {
    let dir = tempdir().expect("temp dir should exist");
    let history_path = dir.path().join("moat-history.json");
    let earlier_round_id = Uuid::new_v4();
    let later_round_id = Uuid::new_v4();
    let earlier_recorded_at = recorded_at("2026-04-25T20:00:00Z");
    let later_recorded_at = recorded_at("2026-04-25T21:00:00Z");

    let mut store = LocalMoatHistoryStore::open(&history_path).expect("history store should open");
    store
        .append(
            later_recorded_at,
            sample_report(
                later_round_id,
                ContinueDecision::Stop,
                Some("review budget exhausted"),
                "implementation stopped before review",
                60,
                60,
                true,
                &[
                    "market_scan",
                    "competitor_analysis",
                    "lockin_analysis",
                    "strategy_generation",
                    "spec_planning",
                    "implementation",
                ],
            ),
        )
        .expect("later report should persist");
    store
        .append(
            earlier_recorded_at,
            sample_report(
                earlier_round_id,
                ContinueDecision::Continue,
                None,
                "review approved bounded moat round",
                90,
                98,
                true,
                &[
                    "market_scan",
                    "competitor_analysis",
                    "lockin_analysis",
                    "strategy_generation",
                    "spec_planning",
                    "implementation",
                    "review",
                    "evaluation",
                ],
            ),
        )
        .expect("earlier report should persist");
    drop(store);

    let reloaded =
        LocalMoatHistoryStore::open(&history_path).expect("reloaded history store should open");
    let entries = reloaded.entries();

    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].recorded_at, earlier_recorded_at);
    assert_eq!(entries[0].report.summary.round_id, earlier_round_id);
    assert_eq!(entries[1].recorded_at, later_recorded_at);
    assert_eq!(entries[1].report.summary.round_id, later_round_id);
}

#[test]
fn summary_reports_latest_best_and_improvement_fields() {
    let dir = tempdir().expect("temp dir should exist");
    let history_path = dir.path().join("moat-history.json");
    let continue_round_id = Uuid::new_v4();
    let stop_round_id = Uuid::new_v4();

    let mut store = LocalMoatHistoryStore::open(&history_path).expect("history store should open");
    store
        .append(
            recorded_at("2026-04-25T20:00:00Z"),
            sample_report(
                continue_round_id,
                ContinueDecision::Continue,
                None,
                "review approved bounded moat round",
                90,
                98,
                true,
                &[
                    "market_scan",
                    "competitor_analysis",
                    "lockin_analysis",
                    "strategy_generation",
                    "spec_planning",
                    "implementation",
                    "review",
                    "evaluation",
                ],
            ),
        )
        .expect("continue report should persist");
    store
        .append(
            recorded_at("2026-04-25T21:00:00Z"),
            sample_report(
                stop_round_id,
                ContinueDecision::Stop,
                Some("review budget exhausted"),
                "implementation stopped before review",
                60,
                60,
                true,
                &[
                    "market_scan",
                    "competitor_analysis",
                    "lockin_analysis",
                    "strategy_generation",
                    "spec_planning",
                    "implementation",
                ],
            ),
        )
        .expect("stop report should persist");

    assert_eq!(
        store.summary(),
        MoatHistorySummary {
            entry_count: 2,
            latest_round_id: Some(stop_round_id.to_string()),
            latest_continue_decision: Some(ContinueDecision::Stop),
            latest_stop_reason: Some("review budget exhausted".to_string()),
            latest_decision_summary: Some("implementation stopped before review".to_string()),
            latest_implemented_specs: vec!["moat-spec/workflow-audit".to_string()],
            latest_moat_score_after: Some(60),
            best_moat_score_after: Some(98),
            improvement_deltas: vec![8, 0],
        }
    );
}

#[test]
fn summary_exposes_latest_implemented_specs() {
    let dir = tempdir().expect("temp dir should exist");
    let history_path = dir.path().join("moat-history.json");
    let round_id = Uuid::new_v4();
    let mut report = sample_report(
        round_id,
        ContinueDecision::Continue,
        None,
        "review approved bounded moat round",
        90,
        98,
        true,
        &[
            "market_scan",
            "competitor_analysis",
            "lockin_analysis",
            "strategy_generation",
            "spec_planning",
            "implementation",
            "review",
            "evaluation",
        ],
    );
    report.summary.implemented_specs = vec!["moat-spec/workflow-audit".to_string()];

    let mut store = LocalMoatHistoryStore::open(&history_path).expect("history store should open");
    store
        .append(recorded_at("2026-04-25T22:00:00Z"), report)
        .expect("report should persist");

    assert_eq!(
        store.summary().latest_implemented_specs,
        vec!["moat-spec/workflow-audit".to_string()]
    );
}

#[test]
fn append_does_not_mutate_in_memory_entries_when_persistence_fails() {
    let dir = tempdir().expect("temp dir should exist");
    let history_path = dir.path().join("moat-history.json");
    let round_id = Uuid::new_v4();
    let recorded_at = recorded_at("2026-04-25T20:00:00Z");

    let mut store = LocalMoatHistoryStore::open(&history_path).expect("history store should open");
    std::fs::remove_file(&history_path).expect("history file should be removable");
    std::fs::create_dir(&history_path).expect("history path should become a directory");

    let error = store
        .append(
            recorded_at,
            sample_report(
                round_id,
                ContinueDecision::Continue,
                None,
                "review approved bounded moat round",
                90,
                98,
                true,
                &[
                    "market_scan",
                    "competitor_analysis",
                    "lockin_analysis",
                    "strategy_generation",
                    "spec_planning",
                    "implementation",
                    "review",
                    "evaluation",
                ],
            ),
        )
        .expect_err("append should fail when the history path is a directory");

    assert!(
        error
            .to_string()
            .contains("failed to access moat history file"),
        "unexpected error: {error}"
    );
    assert!(
        store.entries().is_empty(),
        "failed append must not update memory"
    );
    assert_eq!(store.summary(), MoatHistorySummary::default());
}

#[test]
fn open_creates_missing_history_file_for_round_persistence() {
    let dir = tempdir().expect("temp dir should exist");
    let history_path = dir.path().join("moat-history.json");

    assert!(!history_path.exists());

    let store = LocalMoatHistoryStore::open(&history_path).expect("history store should open");

    assert!(history_path.exists());
    assert!(store.entries().is_empty());
    assert_eq!(store.summary(), MoatHistorySummary::default());
}

#[test]
fn open_existing_fails_for_missing_history_file_without_creating_it() {
    let dir = tempdir().expect("temp dir should exist");
    let history_path = dir.path().join("missing-moat-history.json");

    let error = LocalMoatHistoryStore::open_existing(&history_path)
        .expect_err("opening a missing history file for inspection should fail");

    assert!(
        error
            .to_string()
            .contains("moat history file does not exist"),
        "unexpected error: {error}"
    );
    assert!(!history_path.exists());
}

#[test]
fn empty_store_summary_is_default() {
    let dir = tempdir().expect("temp dir should exist");
    let history_path = dir.path().join("moat-history.json");

    let store = LocalMoatHistoryStore::open(&history_path).expect("history store should open");

    assert_eq!(store.summary(), MoatHistorySummary::default());
}

#[test]
fn continuation_gate_returns_blocked_outcome_for_empty_history() {
    let dir = tempdir().expect("temp dir should exist");
    let history_path = dir.path().join("moat-history.json");
    let store = LocalMoatHistoryStore::open(&history_path).expect("history store should open");

    assert_eq!(
        store.continuation_gate(3),
        mdid_runtime::moat_history::MoatContinuationGate {
            latest_round_id: None,
            latest_continue_decision: None,
            latest_tests_passed: None,
            latest_improvement_delta: None,
            latest_stop_reason: None,
            evaluation_completed: false,
            can_continue: false,
            reason: "no persisted moat rounds to evaluate".to_string(),
            required_improvement_threshold: 3,
        }
    );
}

#[test]
fn continuation_gate_allows_next_round_when_latest_round_completed_evaluation_and_cleared_threshold(
) {
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
                &[
                    "market_scan",
                    "competitor_analysis",
                    "lockin_analysis",
                    "strategy_generation",
                    "spec_planning",
                    "implementation",
                    "review",
                    "evaluation",
                ],
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
                &[
                    "market_scan",
                    "competitor_analysis",
                    "lockin_analysis",
                    "strategy_generation",
                    "spec_planning",
                    "implementation",
                ],
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
                &[
                    "market_scan",
                    "competitor_analysis",
                    "lockin_analysis",
                    "strategy_generation",
                    "spec_planning",
                    "implementation",
                    "review",
                    "evaluation",
                ],
            ),
        )
        .expect("failed test report should persist");

    let gate = store.continuation_gate(3);
    assert!(!gate.can_continue);
    assert!(gate.evaluation_completed);
    assert_eq!(gate.reason, "latest round tests failed");
    assert_eq!(gate.latest_tests_passed, Some(false));
}

#[test]
fn continuation_gate_blocks_when_latest_round_improvement_is_below_threshold() {
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
                92,
                true,
                &[
                    "market_scan",
                    "competitor_analysis",
                    "lockin_analysis",
                    "strategy_generation",
                    "spec_planning",
                    "implementation",
                    "review",
                    "evaluation",
                ],
            ),
        )
        .expect("report should persist");

    assert_eq!(
        store.continuation_gate(3),
        mdid_runtime::moat_history::MoatContinuationGate {
            latest_round_id: Some(round_id.to_string()),
            latest_continue_decision: Some(ContinueDecision::Continue),
            latest_tests_passed: Some(true),
            latest_improvement_delta: Some(2),
            latest_stop_reason: None,
            evaluation_completed: true,
            can_continue: false,
            reason: "latest round improvement below threshold".to_string(),
            required_improvement_threshold: 3,
        }
    );
}

#[test]
fn claim_ready_task_marks_latest_ready_node_in_progress_and_persists() {
    let dir = tempdir().expect("temp dir should exist");
    let history_path = dir.path().join("moat-history.json");
    let earlier_round_id = Uuid::new_v4();
    let latest_round_id = Uuid::new_v4();
    let mut earlier_report = sample_report(
        earlier_round_id,
        ContinueDecision::Continue,
        None,
        "earlier round remains untouched",
        80,
        88,
        true,
        &[],
    );
    let mut latest_report = sample_report(
        latest_round_id,
        ContinueDecision::Continue,
        None,
        "latest round has a ready implementation task",
        90,
        98,
        true,
        &[
            "market_scan",
            "competitor_analysis",
            "lockin_analysis",
            "strategy_generation",
            "spec_planning",
        ],
    );
    set_node_state(
        &mut earlier_report,
        "implementation",
        MoatTaskNodeState::Ready,
    );
    set_node_state(
        &mut latest_report,
        "implementation",
        MoatTaskNodeState::Ready,
    );

    let mut store = LocalMoatHistoryStore::open(&history_path).expect("history store should open");
    store
        .append(recorded_at("2026-04-25T20:00:00Z"), earlier_report)
        .expect("earlier report should persist");
    store
        .append(recorded_at("2026-04-25T21:00:00Z"), latest_report)
        .expect("latest report should persist");

    store
        .claim_ready_task(None, "implementation")
        .expect("ready latest task should be claimed");
    drop(store);

    let reloaded = LocalMoatHistoryStore::open_existing(&history_path)
        .expect("reloaded history store should open");
    let entries = reloaded.entries();
    assert_eq!(entries.len(), 2);
    assert_eq!(
        node_state(&entries[0].report, "implementation"),
        MoatTaskNodeState::Ready
    );
    assert_eq!(
        node_state(&entries[1].report, "implementation"),
        MoatTaskNodeState::InProgress
    );
    assert_eq!(entries[1].report.summary.round_id, latest_round_id);
    assert_eq!(entries[1].report.summary.moat_score_after, 98);
}

#[test]
fn claim_ready_task_rejects_stale_second_handle_without_overwriting_claim() {
    let dir = tempdir().expect("temp dir should exist");
    let history_path = dir.path().join("moat-history.json");
    let round_id = Uuid::new_v4();
    let mut report = sample_report(
        round_id,
        ContinueDecision::Continue,
        None,
        "ready node can only be claimed once",
        90,
        98,
        true,
        &[],
    );
    set_node_state(&mut report, "implementation", MoatTaskNodeState::Ready);

    let mut seed_store =
        LocalMoatHistoryStore::open(&history_path).expect("history store should open");
    seed_store
        .append(recorded_at("2026-04-25T21:00:00Z"), report)
        .expect("ready report should persist");
    drop(seed_store);

    let mut first_handle = LocalMoatHistoryStore::open_existing(&history_path)
        .expect("first handle should open ready history");
    let mut stale_second_handle = LocalMoatHistoryStore::open_existing(&history_path)
        .expect("second handle should open same ready history");

    first_handle
        .claim_ready_task(None, "implementation")
        .expect("first handle should claim ready task");

    let error = stale_second_handle
        .claim_ready_task(None, "implementation")
        .expect_err("stale second handle must not claim already in-progress task");
    assert!(
        error.to_string().contains("moat task node is not ready"),
        "unexpected error: {error}"
    );

    let reloaded = LocalMoatHistoryStore::open_existing(&history_path).expect("reload should open");
    assert_eq!(
        node_state(&reloaded.entries()[0].report, "implementation"),
        MoatTaskNodeState::InProgress,
        "stale failed claim must not overwrite persisted in-progress state"
    );
}

#[test]
fn claim_ready_task_uses_exact_round_id_when_provided() {
    let dir = tempdir().expect("temp dir should exist");
    let history_path = dir.path().join("moat-history.json");
    let earlier_round_id = Uuid::new_v4();
    let latest_round_id = Uuid::new_v4();
    let mut earlier_report = sample_report(
        earlier_round_id,
        ContinueDecision::Continue,
        None,
        "claim exact earlier round",
        80,
        88,
        true,
        &[],
    );
    let mut latest_report = sample_report(
        latest_round_id,
        ContinueDecision::Continue,
        None,
        "latest remains untouched",
        90,
        98,
        true,
        &[],
    );
    set_node_state(&mut earlier_report, "market_scan", MoatTaskNodeState::Ready);
    set_node_state(&mut latest_report, "market_scan", MoatTaskNodeState::Ready);

    let mut store = LocalMoatHistoryStore::open(&history_path).expect("history store should open");
    store
        .append(recorded_at("2026-04-25T20:00:00Z"), earlier_report)
        .expect("earlier persist");
    store
        .append(recorded_at("2026-04-25T21:00:00Z"), latest_report)
        .expect("latest persist");

    store
        .claim_ready_task(Some(&earlier_round_id.to_string()), "market_scan")
        .expect("exact earlier ready task should be claimed");
    drop(store);

    let reloaded = LocalMoatHistoryStore::open_existing(&history_path).expect("reload should open");
    assert_eq!(
        node_state(&reloaded.entries()[0].report, "market_scan"),
        MoatTaskNodeState::InProgress
    );
    assert_eq!(
        node_state(&reloaded.entries()[1].report, "market_scan"),
        MoatTaskNodeState::Ready
    );
}

#[test]
fn claim_ready_task_rejects_unknown_nodes() {
    let dir = tempdir().expect("temp dir should exist");
    let history_path = dir.path().join("moat-history.json");
    let round_id = Uuid::new_v4();
    let mut store = LocalMoatHistoryStore::open(&history_path).expect("history store should open");
    store
        .append(
            recorded_at("2026-04-25T20:00:00Z"),
            sample_report(
                round_id,
                ContinueDecision::Continue,
                None,
                "ready round",
                90,
                98,
                true,
                &[],
            ),
        )
        .expect("report should persist");

    let error = store
        .claim_ready_task(None, "missing_node")
        .expect_err("unknown node should fail");

    assert!(
        error.to_string().contains("moat task node not found"),
        "unexpected error: {error}"
    );
}

#[test]
fn claim_ready_task_rejects_non_ready_nodes() {
    let dir = tempdir().expect("temp dir should exist");
    let history_path = dir.path().join("moat-history.json");
    let round_id = Uuid::new_v4();
    let mut report = sample_report(
        round_id,
        ContinueDecision::Continue,
        None,
        "completed node cannot be claimed",
        90,
        98,
        true,
        &["market_scan"],
    );
    set_node_state(&mut report, "market_scan", MoatTaskNodeState::Completed);
    let mut store = LocalMoatHistoryStore::open(&history_path).expect("history store should open");
    store
        .append(recorded_at("2026-04-25T20:00:00Z"), report)
        .expect("report should persist");

    let error = store
        .claim_ready_task(None, "market_scan")
        .expect_err("non-ready node should fail");

    assert!(
        error.to_string().contains("moat task node is not ready"),
        "unexpected error: {error}"
    );
}

#[test]
fn continuation_gate_uses_reloaded_persisted_history() {
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
                &[
                    "market_scan",
                    "competitor_analysis",
                    "lockin_analysis",
                    "strategy_generation",
                    "spec_planning",
                    "implementation",
                    "review",
                    "evaluation",
                ],
            ),
        )
        .expect("report should persist");
    drop(store);

    let reloaded = LocalMoatHistoryStore::open_existing(&history_path)
        .expect("reloaded history store should open");

    assert_eq!(
        reloaded.continuation_gate(3),
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
        implemented_specs: vec!["moat-spec/workflow-audit".to_string()],
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
        agent_assignments: Vec::new(),
    };

    MoatRoundReport {
        summary,
        executed_tasks: executed_tasks
            .iter()
            .map(|task| (*task).to_string())
            .collect(),
        stop_reason: stop_reason.map(str::to_string),
        control_plane,
    }
}

fn set_node_state(report: &mut MoatRoundReport, node_id: &str, state: MoatTaskNodeState) {
    let node = report
        .control_plane
        .task_graph
        .nodes
        .iter_mut()
        .find(|node| node.node_id == node_id)
        .expect("sample task node should exist");
    node.state = state;
}

fn node_state(report: &MoatRoundReport, node_id: &str) -> MoatTaskNodeState {
    report
        .control_plane
        .task_graph
        .nodes
        .iter()
        .find(|node| node.node_id == node_id)
        .expect("sample task node should exist")
        .state
}

fn recorded_at(value: &str) -> DateTime<Utc> {
    value.parse().expect("timestamp should parse")
}
