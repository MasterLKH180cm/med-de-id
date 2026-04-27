# med-de-id Moat Artifact Inspection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a read-only CLI surface that lets autonomous controllers inspect completed task artifact handoffs from persisted moat-loop history.

**Architecture:** Extend the existing `mdid-cli moat` control-plane inspection family with `moat artifacts`, backed by the persisted task graph in `LocalMoatHistoryStore`. The command must be read-only, select the latest round by default or an exact `--round-id`, filter by node/artifact text, and print stable escaped rows for downstream agent routing.

**Tech Stack:** Rust workspace, `mdid-cli`, `mdid-runtime::moat_history`, existing task-graph domain types, Cargo tests with `CARGO_INCREMENTAL=0`.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `MoatArtifactsCommand`, parser branch for `mdid-cli moat artifacts`, and `run_moat_artifacts` read-only renderer.
  - Reuse existing `required_flag_value`, `parse_optional_round_id_flag`, `parse_positive_limit`, `escape_assignment_output_field`, `LocalMoatHistoryStore::open_existing`, and selected-round patterns from `task-graph`, `assignments`, and `ready-tasks`.
- Test: `crates/mdid-cli/src/main.rs`
  - Add unit tests beside existing moat CLI tests because this crate currently keeps CLI behavior tests in `main.rs`.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Update implementation status to include `moat artifacts`.

### Task 1: Add read-only `moat artifacts` CLI

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`

- [ ] **Step 1: Write failing parser tests**

Add these tests in `crates/mdid-cli/src/main.rs` inside the existing `#[cfg(test)] mod tests`:

```rust
#[test]
fn parse_moat_artifacts_command_requires_history_path() {
    let args = strings(&["moat", "artifacts"]);

    assert_eq!(
        parse_command(&args),
        Err("missing required flag: --history-path".to_string())
    );
}

#[test]
fn parse_moat_artifacts_command_accepts_round_node_contains_and_limit_filters() {
    let args = strings(&[
        "moat",
        "artifacts",
        "--history-path",
        "history.json",
        "--round-id",
        "round-7",
        "--node-id",
        "implementation-task",
        "--contains",
        "handoff",
        "--limit",
        "2",
    ]);

    assert_eq!(
        parse_command(&args),
        Ok(CliCommand::MoatArtifacts(MoatArtifactsCommand {
            history_path: "history.json".to_string(),
            round_id: Some("round-7".to_string()),
            node_id: Some("implementation-task".to_string()),
            contains: Some("handoff".to_string()),
            limit: Some(2),
        }))
    );
}
```

- [ ] **Step 2: Run parser tests to verify RED**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli parse_moat_artifacts_command -- --nocapture
```

Expected: FAIL because `MoatArtifactsCommand` and `CliCommand::MoatArtifacts` do not exist.

- [ ] **Step 3: Implement command shape and parser only**

In `crates/mdid-cli/src/main.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
struct MoatArtifactsCommand {
    history_path: String,
    round_id: Option<String>,
    node_id: Option<String>,
    contains: Option<String>,
    limit: Option<usize>,
}
```

Add `MoatArtifacts(MoatArtifactsCommand),` to `CliCommand`.

Add this match arm in `parse_command` after `ready-tasks`:

```rust
[moat, artifacts, rest @ ..] if moat == "moat" && artifacts == "artifacts" => Ok(
    CliCommand::MoatArtifacts(parse_moat_artifacts_command(rest)?),
),
```

Add parser:

```rust
fn parse_moat_artifacts_command(args: &[String]) -> Result<MoatArtifactsCommand, String> {
    let mut history_path = None;
    let mut round_id = None;
    let mut node_id = None;
    let mut contains = None;
    let mut limit = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--history-path" => {
                let value = required_flag_value(args, index, "--history-path", false)?;
                if history_path.is_some() {
                    return Err(duplicate_flag_error("--history-path"));
                }
                history_path = Some(value.to_string());
                index += 2;
            }
            "--round-id" => {
                let value = required_flag_value(args, index, "--round-id", false)?;
                if round_id.is_some() {
                    return Err(duplicate_flag_error("--round-id"));
                }
                round_id = Some(value.to_string());
                index += 2;
            }
            "--node-id" => {
                let value = required_flag_value(args, index, "--node-id", false)?;
                if node_id.is_some() {
                    return Err(duplicate_flag_error("--node-id"));
                }
                node_id = Some(value.to_string());
                index += 2;
            }
            "--contains" => {
                let value = required_flag_value(args, index, "--contains", false)?;
                if contains.is_some() {
                    return Err(duplicate_flag_error("--contains"));
                }
                contains = Some(value.to_string());
                index += 2;
            }
            "--limit" => {
                let value = required_flag_value(args, index, "--limit", false)?;
                if limit.is_some() {
                    return Err(duplicate_flag_error("--limit"));
                }
                limit = Some(parse_positive_limit(value, "--limit")?);
                index += 2;
            }
            flag => return Err(format!("unknown flag: {flag}")),
        }
    }

    Ok(MoatArtifactsCommand {
        history_path: history_path.ok_or_else(|| "missing required flag: --history-path".to_string())?,
        round_id,
        node_id,
        contains,
        limit,
    })
}
```

In `main()`, add a dispatch arm:

```rust
Ok(CliCommand::MoatArtifacts(command)) => {
    if let Err(error) = run_moat_artifacts(&command) {
        exit_with_error(error);
    }
}
```

Add a temporary implementation stub:

```rust
fn run_moat_artifacts(_command: &MoatArtifactsCommand) -> Result<(), String> {
    Ok(())
}
```

- [ ] **Step 4: Run parser tests to verify GREEN**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli parse_moat_artifacts_command -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Write failing behavior test for read-only artifact output**

Add a CLI integration-style unit test following existing history-backed command test helpers:

```rust
#[test]
fn run_moat_artifacts_prints_completed_task_artifact_handoffs() {
    let temp_dir = tempdir().unwrap();
    let history_path = temp_dir.path().join("moat-history.json");
    seed_moat_history_with_completed_artifact(&history_path, "round-artifacts", "implementation-task", "docs/spec.md", "Spec handoff ready");

    let command = MoatArtifactsCommand {
        history_path: history_path.to_string_lossy().to_string(),
        round_id: Some("round-artifacts".to_string()),
        node_id: Some("implementation-task".to_string()),
        contains: Some("handoff".to_string()),
        limit: Some(1),
    };

    let output = capture_stdout(|| run_moat_artifacts(&command).unwrap());

    assert!(output.contains("moat artifacts"));
    assert!(output.contains("artifact_entries=1"));
    assert!(output.contains("artifact=round-artifacts|implementation-task|docs/spec.md|Spec handoff ready"));
}
```

If the existing test helper names differ, reuse the repository's actual helpers; do not create duplicate temp/history/capture helpers unless none exist.

- [ ] **Step 6: Run behavior test to verify RED**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli run_moat_artifacts_prints_completed_task_artifact_handoffs -- --nocapture
```

Expected: FAIL because `run_moat_artifacts` currently prints nothing or because the seeded helper is not yet adapted.

- [ ] **Step 7: Implement read-only artifact inspection**

Replace the `run_moat_artifacts` stub with code that:

1. opens `LocalMoatHistoryStore::open_existing(&command.history_path)`
2. reads `summary().entries`
3. selects latest entry when `round_id` is `None`, or exact matching `entry.report.summary.round_id` when present
4. prints `moat artifacts`
5. prints `round_id=<selected round id>` when a selected round exists
6. collects task-graph nodes whose `artifact`/`completion_artifact` field is present
7. filters by exact `node_id` if supplied
8. filters by case-sensitive `contains` over raw `node_id`, artifact ref, or artifact summary
9. applies positive `limit` after filters
10. prints `artifact_entries=N`
11. prints each row as `artifact=<round_id>|<node_id>|<artifact_ref>|<artifact_summary>` with existing escaping helper applied to all fields after the prefix
12. if no round matches, prints `artifact_entries=0` and returns `Ok(())`
13. never mutates history or appends a round

- [ ] **Step 8: Run behavior test to verify GREEN**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli run_moat_artifacts_prints_completed_task_artifact_handoffs -- --nocapture
```

Expected: PASS.

- [ ] **Step 9: Run related CLI tests**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli moat_artifacts -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli moat_complete_task -- --nocapture
```

Expected: PASS.

- [ ] **Step 10: Update spec implementation status**

In `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`, add `moat artifacts` to the shipped foundation slice after `complete-task`:

```markdown
- `mdid-cli moat artifacts --history-path PATH [--round-id ROUND_ID] [--node-id NODE_ID] [--contains TEXT] [--limit N]` is a read-only inspection surface for completed task artifact handoffs persisted in the selected round's task graph. It opens only an existing history file, selects the latest persisted round unless `--round-id` exact-matches a specific persisted round, filters exact node IDs and case-sensitive raw node/ref/summary text, applies `--limit` after filtering, escapes pipe-delimited output fields, and never mutates history, appends rounds, schedules work, launches agents, opens PRs, creates cron jobs, crawls data, or writes artifact files.
```

- [ ] **Step 11: Final verification**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli moat_artifacts -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli moat_complete_task -- --nocapture
```

Expected: PASS.

- [ ] **Step 12: Commit**

Run:

```bash
git add crates/mdid-cli/src/main.rs docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-27-med-de-id-moat-artifact-inspection.md
git commit -m "feat: add moat artifact inspection"
```
