# Moat Artifacts JSON Envelope Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `--format json` to `mdid-cli moat artifacts` so external autonomous controllers can consume completed artifact handoffs without scraping text.

**Architecture:** Extend the existing read-only artifacts command with the already-used `MoatOutputFormat` enum and deterministic JSON rendering. Keep text output unchanged, keep all filtering semantics unchanged, and update operator documentation/specs to truthfully describe the new machine-readable surface.

**Tech Stack:** Rust workspace, `mdid-cli`, `serde_json`, Cargo integration tests.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `format: MoatOutputFormat` to `MoatArtifactsCommand`.
  - Parse `--format text|json` for `moat artifacts`, rejecting missing/duplicate/unknown values.
  - Render the filtered artifacts as a deterministic JSON envelope when requested.
  - Update usage text for `moat artifacts`.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add integration tests for JSON artifact output and duplicate format rejection.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Document `moat artifacts ... [--format text|json]` and JSON envelope fields.
- Modify: `README.md`
  - Document the JSON artifacts surface in the moat-loop CLI guide.
- Modify: `AGENTS.md`
  - Truth-sync developer/operator rules with the new JSON artifact export.
- Create: `docs/superpowers/plans/2026-04-28-med-de-id-moat-artifacts-json-envelope.md`
  - This implementation plan.

### Task 1: Add JSON output to `moat artifacts`

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
- Modify: `README.md`
- Modify: `AGENTS.md`
- Test: `crates/mdid-cli/tests/moat_cli.rs`

- [ ] **Step 1: Write the failing JSON output integration test**

Add this test near existing `moat_artifacts_*` tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn moat_artifacts_json_prints_parseable_filtered_envelope() {
    let history_path = unique_history_path("artifacts-json");
    let history_path_arg = history_path.to_str().unwrap();

    Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("run moat round");

    let dispatch = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path_arg,
            "--node-id",
            "moat-implementation-tests",
            "--agent-id",
            "coder-json",
        ])
        .output()
        .expect("dispatch implementation task");
    assert!(dispatch.status.success(), "{}", String::from_utf8_lossy(&dispatch.stderr));

    let complete = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "complete-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "moat-implementation-tests",
            "--artifact-ref",
            "commit:abc123",
            "--artifact-summary",
            "Implemented deterministic JSON artifacts export",
        ])
        .output()
        .expect("complete task with artifact");
    assert!(complete.status.success(), "{}", String::from_utf8_lossy(&complete.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "artifacts",
            "--history-path",
            history_path_arg,
            "--node-id",
            "moat-implementation-tests",
            "--format",
            "json",
        ])
        .output()
        .expect("export artifacts json");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("parse artifacts json");
    assert_eq!(json["type"], "moat_artifacts");
    assert_eq!(json["round_id"], "round-001");
    assert_eq!(json["history_path"], history_path_arg);
    assert_eq!(json["artifact_entries"], 1);
    assert_eq!(json["artifacts"][0]["node_id"], "moat-implementation-tests");
    assert_eq!(json["artifacts"][0]["artifact_ref"], "commit:abc123");
    assert_eq!(
        json["artifacts"][0]["artifact_summary"],
        "Implemented deterministic JSON artifacts export"
    );
    assert_eq!(json["artifacts"][0]["node_title"], "Add deterministic test evidence capture");
    assert_eq!(json["artifacts"][0]["node_role"], "coder");
    assert_eq!(json["artifacts"][0]["node_kind"], "implementation");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli moat_artifacts_json_prints_parseable_filtered_envelope --test moat_cli -- --nocapture`

Expected: FAIL with an error such as `unknown option for moat artifacts: --format`.

- [ ] **Step 3: Write duplicate format parser test**

Add this test near parser validation tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn moat_artifacts_rejects_duplicate_format() {
    let history_path = unique_history_path("artifacts-duplicate-format");
    let history_path_arg = history_path.to_str().unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "artifacts",
            "--history-path",
            history_path_arg,
            "--format",
            "json",
            "--format",
            "text",
        ])
        .output()
        .expect("run artifacts with duplicate format");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("duplicate option: --format"), "{stderr}");
}
```

- [ ] **Step 4: Run duplicate test to verify it fails**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli moat_artifacts_rejects_duplicate_format --test moat_cli -- --nocapture`

Expected: FAIL because duplicate `--format` is not parsed as a known artifacts option yet.

- [ ] **Step 5: Implement minimal parser and JSON renderer**

In `crates/mdid-cli/src/main.rs`:

```rust
struct MoatArtifactsCommand {
    history_path: String,
    round_id: Option<String>,
    role: Option<AgentRole>,
    state: Option<MoatTaskNodeState>,
    kind: Option<MoatTaskNodeKind>,
    node_id: Option<String>,
    contains: Option<String>,
    artifact_ref: Option<String>,
    artifact_summary: Option<String>,
    limit: Option<usize>,
    format: MoatOutputFormat,
}
```

Parse `--format` in `parse_moat_artifacts_command` using the same `parse_moat_output_format` helper used by other commands, with duplicate detection and default `MoatOutputFormat::Text`.

Refactor `run_moat_artifacts` so it builds the filtered artifact rows once, then:
- for `Text`, print exactly the existing text output.
- for `Json`, print pretty JSON:

```json
{
  "type": "moat_artifacts",
  "round_id": "round-001",
  "history_path": "/path/to/history.json",
  "artifact_entries": 1,
  "artifacts": [
    {
      "node_id": "moat-implementation-tests",
      "node_title": "Add deterministic test evidence capture",
      "node_role": "coder",
      "node_kind": "implementation",
      "node_state": "completed",
      "artifact_ref": "commit:abc123",
      "artifact_summary": "Implemented deterministic JSON artifacts export"
    }
  ]
}
```

When no selected round exists, JSON must still be parseable with `round_id: null`, `artifact_entries: 0`, and `artifacts: []`.

- [ ] **Step 6: Verify targeted tests pass**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli moat_artifacts_json_prints_parseable_filtered_envelope --test moat_cli -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli moat_artifacts_rejects_duplicate_format --test moat_cli -- --nocapture
```

Expected: both PASS.

- [ ] **Step 7: Update README, AGENTS, and spec docs**

Update docs to state:

```markdown
`mdid-cli moat artifacts --history-path PATH [--round-id ROUND_ID] [--node-id NODE_ID] [--artifact-ref TEXT] [--artifact-summary TEXT] [--format text|json]` remains read-only. Text is the default. JSON emits a deterministic `moat_artifacts` envelope with `round_id`, `history_path`, `artifact_entries`, and one row per completed artifact handoff, including node metadata and artifact reference/summary.
```

- [ ] **Step 8: Run broader moat CLI verification**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli moat_artifacts --test moat_cli -- --nocapture`

Expected: all `moat_artifacts...` tests PASS.

- [ ] **Step 9: Commit**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs README.md AGENTS.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-28-med-de-id-moat-artifacts-json-envelope.md
git commit -m "feat(cli): export moat artifacts as json"
```

Expected: commit succeeds on `feature/moat-loop-autonomy`.

---

## Self-Review

- Spec coverage: The plan adds machine-readable artifacts for external controllers without launching agents or mutating history.
- Placeholder scan: No TBD/TODO/implement-later placeholders remain.
- Type consistency: The plan consistently uses `MoatOutputFormat`, `moat_artifacts`, `artifact_entries`, and `artifacts`.
