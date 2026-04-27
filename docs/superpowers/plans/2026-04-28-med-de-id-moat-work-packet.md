# Moat Work Packet Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded `mdid-cli moat work-packet` command that emits deterministic role-specific work packets for external Planner/Coder/Reviewer agents from persisted moat-loop task history.

**Architecture:** The CLI remains a local-first coordination surface: it reads an existing history file, selects one persisted task node, gathers completed dependency artifacts, and renders a text or JSON packet. The first releaseable slice is read-only and does not launch agents, crawl data, create PRs, mutate task state, or create cron jobs; claiming stays in existing `dispatch-next`/`claim-task` commands.

**Tech Stack:** Rust workspace, `mdid-cli`, `mdid-domain`, `mdid-runtime::moat_history::LocalMoatHistoryStore`, `serde_json`, Cargo integration tests.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `MoatWorkPacketCommand`, `CliCommand::MoatWorkPacket`, parser, runner, text renderer, JSON renderer, and usage string updates.
  - Reuse existing formatting helpers: `format_agent_role`, `format_moat_task_kind`, `format_task_node_state`, `shell_single_quote`, and JSON patterns from ready/dispatch/complete surfaces.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add integration tests for text output, JSON output, read-only behavior, missing selected node errors, and parser errors.
  - Update the mirrored `USAGE` constant to include the new command.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Document the shipped work-packet command as read-only external-controller handoff.
- Modify: `README.md`
  - Add operator usage examples for creating history, inspecting ready tasks, exporting a work packet, and completing a task with an artifact.
- Modify: `AGENTS.md`
  - Add concise boundary rules for `moat work-packet`: read-only, no daemon, no agent launching, no PR automation.

### Task 1: Add read-only work-packet CLI text and JSON envelope

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/moat_cli.rs`

- [x] **Step 1: Write the failing JSON work-packet test**

Add this test near the existing dispatch/ready-task CLI tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn moat_work_packet_json_exports_task_context_and_dependency_artifacts_read_only() {
    let history_path = unique_history_path("work-packet-json");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--review-loops", "0", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed work-packet history");
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));

    let claim = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "claim-task", "--history-path", history_path_arg, "--node-id", "implementation", "--agent-id", "coder-a"])
        .output()
        .expect("failed to claim implementation task");
    assert!(claim.status.success(), "{}", String::from_utf8_lossy(&claim.stderr));

    let complete = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "complete-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "implementation",
            "--artifact-ref",
            "plan://implementation-output",
            "--artifact-summary",
            "Implemented deterministic moat workflow audit slice",
        ])
        .output()
        .expect("failed to complete implementation task");
    assert!(complete.status.success(), "{}", String::from_utf8_lossy(&complete.stderr));

    let before = fs::read_to_string(&history_path).expect("history should be readable before packet");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "work-packet",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to export work packet as json");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let packet: Value = serde_json::from_slice(&output.stdout).expect("packet should be valid json");
    assert_eq!(packet["type"], "moat_work_packet");
    assert_eq!(packet["history_path"], history_path_arg);
    assert_eq!(packet["node_id"], "review");
    assert_eq!(packet["role"], "reviewer");
    assert_eq!(packet["kind"], "review");
    assert_eq!(packet["state"], "ready");
    assert_eq!(packet["dependencies"][0], "implementation");
    assert_eq!(packet["dependency_artifacts"][0]["node_id"], "implementation");
    assert_eq!(packet["dependency_artifacts"][0]["artifact_ref"], "plan://implementation-output");
    assert_eq!(packet["acceptance_criteria"][0], "Use SDD and TDD for any implementation work before completing this task.");
    assert!(packet["complete_command"].as_str().unwrap().contains("moat complete-task"));

    let after = fs::read_to_string(&history_path).expect("history should be readable after packet");
    assert_eq!(after, before, "work-packet must be read-only");
}
```

- [x] **Step 2: Run the JSON test to verify RED**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_work_packet_json_exports_task_context_and_dependency_artifacts_read_only -- --nocapture
```

Expected: FAIL because `moat work-packet` is not recognized yet.

- [x] **Step 3: Write the failing text work-packet test**

Add this test near the JSON test:

```rust
#[test]
fn moat_work_packet_text_exports_controller_handoff_without_mutating_history() {
    let history_path = unique_history_path("work-packet-text");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--review-loops", "0", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed work-packet text history");
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));

    let before = fs::read_to_string(&history_path).expect("history should be readable before packet");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "work-packet", "--history-path", history_path_arg, "--node-id", "review"])
        .output()
        .expect("failed to export work packet as text");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("moat work packet\n"));
    assert!(stdout.contains("node_id=review\n"));
    assert!(stdout.contains("role=reviewer\n"));
    assert!(stdout.contains("kind=review\n"));
    assert!(stdout.contains("state=ready\n"));
    assert!(stdout.contains("dependency=implementation\n"));
    assert!(stdout.contains("acceptance=Use SDD and TDD for any implementation work before completing this task.\n"));
    assert!(stdout.contains("complete_command=mdid-cli moat complete-task"));

    let after = fs::read_to_string(&history_path).expect("history should be readable after packet");
    assert_eq!(after, before, "work-packet must be read-only");
}
```

- [x] **Step 4: Run the text test to verify RED**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_work_packet_text_exports_controller_handoff_without_mutating_history -- --nocapture
```

Expected: FAIL because `moat work-packet` is not recognized yet.

- [x] **Step 5: Implement minimal command parsing and rendering**

In `crates/mdid-cli/src/main.rs`, add a command struct:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
struct MoatWorkPacketCommand {
    history_path: String,
    round_id: Option<String>,
    node_id: String,
    format: MoatOutputFormat,
}
```

Add `MoatWorkPacket(MoatWorkPacketCommand)` to `CliCommand`, parse `moat work-packet`, require `--history-path` and `--node-id`, accept optional `--round-id` and `--format text|json`, and run a new `run_moat_work_packet` function.

The runner must:

```rust
let store = LocalMoatHistoryStore::open_existing(&command.history_path)
    .map_err(|error| format!("failed to open moat history: {error}"))?;
let summary = store.summary();
let selected_entry = select_history_entry(summary.entries(), command.round_id.as_deref())?;
let node = selected_entry
    .report
    .control_plane
    .task_graph
    .nodes
    .iter()
    .find(|candidate| candidate.node_id == command.node_id)
    .ok_or_else(|| format!("moat work-packet node not found: {}", command.node_id))?;
let dependency_artifacts: Vec<_> = node
    .dependencies
    .iter()
    .flat_map(|dependency_id| {
        selected_entry
            .report
            .control_plane
            .task_graph
            .nodes
            .iter()
            .filter(move |candidate| candidate.node_id == *dependency_id)
            .flat_map(move |dependency| dependency.artifacts.iter().map(move |artifact| (dependency, artifact)))
    })
    .collect();
```

Text output must include:

```text
moat work packet
round_id=<round>
history_path=<path>
node_id=<node>
title=<title>
role=<planner|coder|reviewer>
kind=<kind>
state=<state>
spec_ref=<spec-or-<none>>
dependency=<dep-id>
dependency_artifact=<dep-node>|<artifact-ref>|<artifact-summary>
acceptance=Use SDD and TDD for any implementation work before completing this task.
acceptance=Complete the task by recording an artifact with mdid-cli moat complete-task.
complete_command=mdid-cli moat complete-task --history-path '<path>' --node-id '<node>' --artifact-ref '<artifact-ref>' --artifact-summary '<artifact-summary>'
```

JSON output must be pretty JSON with raw field values:

```json
{
  "type": "moat_work_packet",
  "round_id": "...",
  "history_path": "...",
  "node_id": "...",
  "title": "...",
  "role": "reviewer",
  "kind": "review",
  "state": "ready",
  "spec_ref": "... or null",
  "dependencies": ["implementation"],
  "dependency_artifacts": [{"node_id":"implementation","artifact_ref":"...","artifact_summary":"..."}],
  "acceptance_criteria": [
    "Use SDD and TDD for any implementation work before completing this task.",
    "Complete the task by recording an artifact with mdid-cli moat complete-task."
  ],
  "complete_command": "mdid-cli moat complete-task --history-path '...' --node-id '...' --artifact-ref '<artifact-ref>' --artifact-summary '<artifact-summary>'"
}
```

- [x] **Step 6: Run both work-packet tests to verify GREEN**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_work_packet -- --nocapture
```

Expected: PASS.

- [x] **Step 7: Commit task 1**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs
git commit -m "feat(cli): export moat work packets"
```

### Task 2: Add work-packet failure and parser coverage

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`

- [x] **Step 1: Write failure/parser tests**

Add tests asserting:

```rust
#[test]
fn moat_work_packet_fails_for_missing_node() {
    let history_path = unique_history_path("work-packet-missing-node");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--review-loops", "0", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed missing-node packet history");
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "work-packet", "--history-path", history_path_arg, "--node-id", "not-a-node"])
        .output()
        .expect("failed to run missing-node packet command");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("moat work-packet node not found: not-a-node"));
}

#[test]
fn moat_work_packet_rejects_unknown_format() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "work-packet", "--history-path", "history.json", "--node-id", "review", "--format", "yaml"])
        .output()
        .expect("failed to run invalid work-packet format command");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unknown moat work-packet format: yaml"));
}
```

- [x] **Step 2: Run tests to verify RED**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_work_packet_fails_for_missing_node moat_work_packet_rejects_unknown_format -- --nocapture
```

Expected: FAIL until parser/error behavior is implemented exactly.

- [x] **Step 3: Implement error exactness and parser guards**

Ensure `parse_moat_work_packet_command` returns:

```rust
Err("missing required moat work-packet --history-path".to_string())
Err("missing required moat work-packet --node-id".to_string())
Err("duplicate moat work-packet --history-path".to_string())
Err("duplicate moat work-packet --node-id".to_string())
Err("duplicate moat work-packet --format".to_string())
Err(format!("unknown moat work-packet format: {other}"))
```

- [x] **Step 4: Run work-packet tests**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_work_packet -- --nocapture
```

Expected: PASS.

- [x] **Step 5: Commit task 2**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs
git commit -m "test(cli): cover moat work packet errors"
```

### Task 3: Truth-sync docs and run package verification

**Files:**
- Modify: `README.md`
- Modify: `AGENTS.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
- Modify: `docs/superpowers/plans/2026-04-28-med-de-id-moat-work-packet.md`

- [x] **Step 1: Update docs**

Add concise docs that state:

```markdown
`mdid-cli moat work-packet --history-path PATH --node-id NODE_ID [--round-id ROUND_ID] [--format text|json]` exports a deterministic read-only work packet for an external Planner/Coder/Reviewer controller. It includes task metadata, dependency IDs, completed upstream artifact handoffs, acceptance criteria, and the recommended `complete-task` command. It never launches agents, mutates history, schedules work, crawls data, opens PRs, creates cron jobs, or writes artifact files.
```

- [x] **Step 2: Run focused verification**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_work_packet -- --nocapture
```

Expected: PASS.

- [x] **Step 3: Run package verification**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli
```

Expected: PASS.

- [x] **Step 4: Commit docs and verification evidence**

Run:

```bash
git add README.md AGENTS.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-28-med-de-id-moat-work-packet.md
git commit -m "docs: describe moat work packets"
```
