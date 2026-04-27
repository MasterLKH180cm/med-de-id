# Moat Artifact-Required Routing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded `--requires-artifacts` routing filter so autonomous controllers can dispatch downstream tasks only when every declared dependency completed with at least one artifact handoff.

**Architecture:** Keep the routing decision local to the CLI/controller surface by extending the existing ready-task and dispatch-next filters. The filter remains read-only for `ready-tasks` and `dispatch-next --dry-run`; normal `dispatch-next` still mutates only by claiming the selected task.

**Tech Stack:** Rust workspace, `mdid-cli`, integration tests in `crates/mdid-cli/tests/moat_cli.rs`, persisted JSON moat history via `mdid-runtime`.

---

### Task 1: Add `--requires-artifacts` ready-task routing filter

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`

- [ ] **Step 1: Write the failing ready-tasks test**

Add this integration test to `crates/mdid-cli/tests/moat_cli.rs` near the existing ready-tasks dependency filter tests:

```rust
#[test]
fn cli_ready_tasks_requires_completed_dependency_artifacts() {
    let (_temp_dir, history_path) = write_history_with_artifact_routing_tasks();

    let output = run_cli([
        "moat",
        "ready-tasks",
        "--history-path",
        history_path.to_str().unwrap(),
        "--requires-artifacts",
    ]);

    assert_success(&output);
    let stdout = stdout_text(&output);
    assert!(stdout.contains("ready_with_artifact"));
    assert!(!stdout.contains("ready_without_artifact"));
}
```

Also add a helper fixture in the same test file that writes one completed dependency with artifacts, one completed dependency without artifacts, and two dependent ready tasks:

```rust
fn write_history_with_artifact_routing_tasks() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let history_path = temp_dir.path().join("moat-history.json");
    let recorded_at = timestamp("2026-04-27T16:00:00Z");
    let round_id = uuid("11111111-1111-1111-1111-111111111111");
    let report = MoatRoundReport {
        summary: MoatRoundSummary {
            round_id,
            selected_strategies: vec!["artifact routing".into()],
            implemented_specs: vec![],
            tests_passed: true,
            moat_score_before: 40,
            moat_score_after: 45,
            continue_decision: ContinueDecision::Continue,
            stop_reason: None,
            pivot_reason: None,
        },
        market_snapshot: MarketMoatSnapshot::default(),
        competitors: vec![],
        lock_in_report: LockInReport::default(),
        generated_strategies: vec![],
        task_graph: MoatTaskGraph {
            round_id,
            nodes: vec![
                MoatTaskNode {
                    node_id: "dependency_with_artifact".into(),
                    title: "Dependency With Artifact".into(),
                    role: AgentRole::Planner,
                    kind: MoatTaskNodeKind::MarketScan,
                    state: MoatTaskNodeState::Completed,
                    depends_on: vec![],
                    spec_ref: Some("docs/specs/market.md".into()),
                    artifacts: vec![MoatTaskArtifact {
                        artifact_ref: "artifacts/market.md".into(),
                        summary: "market evidence".into(),
                        recorded_at,
                    }],
                },
                MoatTaskNode {
                    node_id: "dependency_without_artifact".into(),
                    title: "Dependency Without Artifact".into(),
                    role: AgentRole::Planner,
                    kind: MoatTaskNodeKind::CompetitorAnalysis,
                    state: MoatTaskNodeState::Completed,
                    depends_on: vec![],
                    spec_ref: Some("docs/specs/competitors.md".into()),
                    artifacts: vec![],
                },
                MoatTaskNode {
                    node_id: "ready_with_artifact".into(),
                    title: "Ready With Artifact".into(),
                    role: AgentRole::Coder,
                    kind: MoatTaskNodeKind::StrategyGeneration,
                    state: MoatTaskNodeState::Ready,
                    depends_on: vec!["dependency_with_artifact".into()],
                    spec_ref: Some("docs/specs/strategy.md".into()),
                    artifacts: vec![],
                },
                MoatTaskNode {
                    node_id: "ready_without_artifact".into(),
                    title: "Ready Without Artifact".into(),
                    role: AgentRole::Coder,
                    kind: MoatTaskNodeKind::StrategyGeneration,
                    state: MoatTaskNodeState::Ready,
                    depends_on: vec!["dependency_without_artifact".into()],
                    spec_ref: Some("docs/specs/strategy.md".into()),
                    artifacts: vec![],
                },
            ],
        },
        decision_memory: MoatMemorySnapshot {
            round_id,
            latest_score: 45,
            improvement_delta: 5,
            decisions: vec![],
        },
        resource_budget: ResourceBudget::default(),
    };
    LocalMoatHistoryStore::open(&history_path)
        .unwrap()
        .append(recorded_at, report)
        .unwrap();

    (temp_dir, history_path)
}
```

- [ ] **Step 2: Run the focused test and verify RED**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_ready_tasks_requires_completed_dependency_artifacts -- --nocapture`

Expected: FAIL because `--requires-artifacts` is not recognized or is ignored.

- [ ] **Step 3: Implement the minimal ready-tasks filter**

In `crates/mdid-cli/src/main.rs`, add a `requires_artifacts: bool` field to the ready-task filter command struct, parse `--requires-artifacts`, reject duplicate flags, and include it in the ready-task candidate predicate. A candidate passes when `requires_artifacts` is false, or every node ID in `candidate.depends_on` resolves to a completed node whose `artifacts` vector is non-empty. Root tasks with no dependencies pass.

- [ ] **Step 4: Run the focused test and verify GREEN**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_ready_tasks_requires_completed_dependency_artifacts -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Update spec text**

Update `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md` so the `ready-tasks` bullet includes `[--requires-artifacts]` and states that it keeps only candidates whose completed dependencies each include at least one artifact handoff.

- [ ] **Step 6: Commit**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-27-med-de-id-moat-artifact-required-routing.md
git commit -m "feat: filter ready moat tasks by dependency artifacts"
```

### Task 2: Add `--requires-artifacts` dispatch-next routing filter

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`

- [ ] **Step 1: Write the failing dispatch-next test**

Add this integration test to `crates/mdid-cli/tests/moat_cli.rs` near the dispatch-next dependency filter tests:

```rust
#[test]
fn cli_dispatch_next_requires_completed_dependency_artifacts() {
    let (_temp_dir, history_path) = write_history_with_artifact_routing_tasks();

    let output = run_cli([
        "moat",
        "dispatch-next",
        "--history-path",
        history_path.to_str().unwrap(),
        "--requires-artifacts",
        "--dry-run",
    ]);

    assert_success(&output);
    let stdout = stdout_text(&output);
    assert!(stdout.contains("ready_with_artifact"));
    assert!(!stdout.contains("ready_without_artifact"));
}
```

- [ ] **Step 2: Run the focused test and verify RED**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_dispatch_next_requires_completed_dependency_artifacts -- --nocapture`

Expected: FAIL because `dispatch-next` does not parse or apply `--requires-artifacts` yet.

- [ ] **Step 3: Implement the minimal dispatch-next filter**

In `crates/mdid-cli/src/main.rs`, add `requires_artifacts: bool` to the dispatch-next command struct, parse `--requires-artifacts` with duplicate detection, and reuse the same dependency-artifact predicate used by ready-tasks.

- [ ] **Step 4: Run the focused test and verify GREEN**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_dispatch_next_requires_completed_dependency_artifacts -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Run broader directly-related verification**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli dependency_artifacts -- --nocapture`

Expected: PASS for both ready-tasks and dispatch-next artifact-required routing tests.

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli ready_tasks -- --nocapture`

Expected: PASS for existing ready-task routing tests.

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli dispatch_next -- --nocapture`

Expected: PASS for existing dispatch-next routing tests.

- [ ] **Step 6: Update spec text**

Update `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md` so the `dispatch-next` bullet includes `[--requires-artifacts]` and states that it uses the same completed-dependency artifact handoff filter before selecting the single task.

- [ ] **Step 7: Commit**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md
git commit -m "feat: require artifacts for moat dispatch routing"
```
