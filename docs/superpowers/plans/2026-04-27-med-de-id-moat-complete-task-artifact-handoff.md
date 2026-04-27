# Moat Complete Task Artifact Handoff Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Allow external Planner/Coder/Reviewer workers to complete a claimed moat task while persisting a bounded artifact reference and summary as the durable handoff to downstream tasks.

**Architecture:** Extend the existing local JSON-backed task lifecycle instead of launching agents or generating files. The domain task node gains defaulted artifact metadata, the runtime complete-task mutation appends optional validated artifact metadata while preserving locking/reload semantics, and the CLI exposes paired `--artifact-ref` / `--artifact-summary` flags with deterministic output.

**Tech Stack:** Rust workspace, serde, chrono, `mdid-domain`, `mdid-runtime::moat_history::LocalMoatHistoryStore`, `mdid-cli`, Cargo integration tests.

---

## File Structure

- Modify: `crates/mdid-domain/src/lib.rs`
  - Add `MoatTaskArtifact` and `artifacts: Vec<MoatTaskArtifact>` to `MoatTaskNode` with `#[serde(default)]` for backward compatibility.
- Modify: `crates/mdid-runtime/src/moat_history.rs`
  - Add `CompleteTaskArtifact` input model and `complete_in_progress_task_with_artifact(...)` mutation that validates paired non-blank artifact refs/summaries and persists them atomically with the completed state.
- Modify: `crates/mdid-cli/src/main.rs`
  - Parse `--artifact-ref TEXT` and `--artifact-summary TEXT` for `moat complete-task`; require both or neither; print `artifact_recorded`, `artifact_ref`, and `artifact_summary` before downstream ready-task rows.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add integration coverage for artifact completion and paired-flag validation.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Truth-sync shipped status and lifecycle scope.
- Modify: `README.md`
  - Add a concise artifact handoff example to the moat-loop CLI surface.

---

### Task 1: Domain/runtime artifact persistence

**Files:**
- Modify: `crates/mdid-domain/src/lib.rs`
- Modify: `crates/mdid-runtime/src/moat_history.rs`
- Test: runtime/domain unit tests in existing files or `crates/mdid-runtime/tests/moat_history.rs`

- [ ] **Step 1: Write the failing persistence test**

Add a test proving completing an in-progress task with an artifact stores the artifact on that task node:

```rust
#[test]
fn completing_in_progress_task_with_artifact_persists_worker_handoff() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let history_path = temp_dir.path().join("history.json");
    let mut store = LocalMoatHistoryStore::open(&history_path).expect("open history");
    let mut report = sample_moat_round_report();
    let round_id = report.summary.round_id.to_string();
    let node = report
        .control_plane
        .task_graph
        .nodes
        .iter_mut()
        .find(|node| node.node_id == "implementation")
        .expect("implementation node");
    node.state = MoatTaskNodeState::InProgress;
    store.append(Utc::now(), report).expect("append report");

    let selected_round_id = store
        .complete_in_progress_task_with_artifact(
            None,
            "implementation",
            Some(CompleteTaskArtifact {
                artifact_ref: "docs/superpowers/plans/generated/workflow-audit.md".to_string(),
                artifact_summary: "Generated workflow audit implementation plan".to_string(),
                recorded_at: Utc::now(),
            }),
        )
        .expect("complete task with artifact");

    assert_eq!(selected_round_id, round_id);
    let reloaded = LocalMoatHistoryStore::open_existing(&history_path).expect("reload history");
    let completed_node = reloaded.entries()[0]
        .report
        .control_plane
        .task_graph
        .nodes
        .iter()
        .find(|node| node.node_id == "implementation")
        .expect("completed node");
    assert_eq!(completed_node.state, MoatTaskNodeState::Completed);
    assert_eq!(completed_node.artifacts.len(), 1);
    assert_eq!(completed_node.artifacts[0].artifact_ref, "docs/superpowers/plans/generated/workflow-audit.md");
    assert_eq!(completed_node.artifacts[0].summary, "Generated workflow audit implementation plan");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-runtime completing_in_progress_task_with_artifact_persists_worker_handoff -- --nocapture
```

Expected: FAIL because `MoatTaskNode.artifacts`, `CompleteTaskArtifact`, and `complete_in_progress_task_with_artifact` do not exist.

- [ ] **Step 3: Implement minimal domain/runtime support**

In `crates/mdid-domain/src/lib.rs`, add:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoatTaskArtifact {
    pub artifact_ref: String,
    pub summary: String,
    pub recorded_at: DateTime<Utc>,
}
```

and update `MoatTaskNode`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoatTaskNode {
    pub node_id: String,
    pub title: String,
    pub role: AgentRole,
    pub kind: MoatTaskNodeKind,
    pub state: MoatTaskNodeState,
    pub depends_on: Vec<String>,
    pub spec_ref: Option<String>,
    #[serde(default)]
    pub artifacts: Vec<MoatTaskArtifact>,
}
```

In `crates/mdid-runtime/src/moat_history.rs`, import `MoatTaskArtifact`, add:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompleteTaskArtifact {
    pub artifact_ref: String,
    pub artifact_summary: String,
    pub recorded_at: DateTime<Utc>,
}
```

and implement:

```rust
pub fn complete_in_progress_task_with_artifact(
    &mut self,
    round_id: Option<&str>,
    node_id: &str,
    artifact: Option<CompleteTaskArtifact>,
) -> Result<String, CompleteInProgressTaskError> {
    self.transition_task_state_with_artifact(
        round_id,
        node_id,
        MoatTaskNodeState::InProgress,
        MoatTaskNodeState::Completed,
        artifact,
    )
}
```

The helper must mirror existing locking/reload/round selection behavior, reject blank `artifact_ref` or `artifact_summary`, append `MoatTaskArtifact { artifact_ref, summary: artifact_summary, recorded_at }` before persisting, and return the selected round ID.

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-runtime completing_in_progress_task_with_artifact_persists_worker_handoff -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Run targeted runtime lifecycle tests**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-runtime moat_history -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/mdid-domain/src/lib.rs crates/mdid-runtime/src/moat_history.rs crates/mdid-runtime/tests/moat_history.rs
git commit -m "feat: persist moat task artifact handoffs"
```

---

### Task 2: CLI artifact handoff flags

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
- Modify: `README.md`

- [ ] **Step 1: Write failing CLI tests**

Add tests that claim `implementation`, complete it with `--artifact-ref` and `--artifact-summary`, assert deterministic output includes artifact metadata, reload the history JSON, and assert the node contains the persisted artifact. Add paired-flag validation tests for `--artifact-ref` without `--artifact-summary` and the reverse.

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli complete_task_artifact -- --nocapture
```

Expected: FAIL because `moat complete-task` does not recognize artifact flags.

- [ ] **Step 3: Implement minimal CLI parsing and output**

Extend `MoatCompleteTaskCommand` with:

```rust
artifact_ref: Option<String>,
artifact_summary: Option<String>,
```

Parse `--artifact-ref` and `--artifact-summary` only for complete-task. Reject duplicates, missing values, and unpaired flags with precise errors. In `run_moat_complete_task`, construct `CompleteTaskArtifact` when both are present, pass it to `complete_in_progress_task_with_artifact`, and print:

```text
artifact_recorded=true|false
artifact_ref=<none or escaped ref>
artifact_summary=<none or escaped summary>
```

before `next_ready_task_entries`.

- [ ] **Step 4: Update docs**

Update the spec shipped-foundation `complete-task` bullet to mention optional artifact refs and that the command does not create artifact files. Add a README example showing completing a claimed task with artifact metadata.

- [ ] **Step 5: Run targeted CLI tests**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli complete_task -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Run package-level CLI tests**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md README.md docs/superpowers/plans/2026-04-27-med-de-id-moat-complete-task-artifact-handoff.md
git commit -m "feat: add moat complete-task artifact handoff"
```
