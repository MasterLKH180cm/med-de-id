# med-de-id Moat Artifact Node Routing Filters Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend `mdid-cli moat artifacts` so autonomous controllers can route completed artifact handoffs by the producing task node's role, kind, and state.

**Architecture:** Keep this as a bounded read-only CLI inspection slice in `mdid-cli`. Parsing stores optional node metadata filters on `MoatArtifactsCommand`, and `run_moat_artifacts` applies them against the selected persisted round's task graph nodes before artifact projection; no history mutation, scheduling, agent launch, crawling, PR creation, or cron creation is introduced.

**Tech Stack:** Rust 2021 workspace, `mdid-cli` integration tests, existing `mdid-domain` enums (`AgentRole`, `MoatTaskNodeKind`, `MoatTaskNodeState`), existing local JSON moat history store.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `role`, `kind`, and `state` fields to `MoatArtifactsCommand`.
  - Parse `--role planner|coder|reviewer`, `--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation`, and `--state pending|ready|in_progress|completed|blocked` for `moat artifacts` using existing parsers.
  - Apply these filters to the producing task node before flattening artifacts.
  - Update the usage string for `moat artifacts`.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add integration tests proving artifact inspection can filter by role/kind/state and that filters are conjunctive.

### Task 1: Artifact role/kind/state CLI filters

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/moat_cli.rs`

- [x] **Step 1: Write the failing tests**

Append these tests near the existing `moat artifacts` tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn cli_filters_moat_artifacts_by_node_role_kind_and_state() {
    let history_path = unique_history_path("artifacts-node-routing-filters");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let round_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to run mdid-cli moat round with history path");
    assert!(round_output.status.success(), "expected round success, stderr was: {}", String::from_utf8_lossy(&round_output.stderr));

    let claim_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "claim-task", "--history-path", history_path_arg, "--node-id", "implementation"])
        .output()
        .expect("failed to claim implementation task");
    assert!(claim_output.status.success(), "expected claim success, stderr was: {}", String::from_utf8_lossy(&claim_output.stderr));

    let complete_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "complete-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "implementation",
            "--artifact-ref",
            "docs/superpowers/plans/implementation.md",
            "--artifact-summary",
            "Implementation handoff ready for reviewer",
        ])
        .output()
        .expect("failed to complete implementation task with artifact");
    assert!(complete_output.status.success(), "expected complete success, stderr was: {}", String::from_utf8_lossy(&complete_output.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "artifacts",
            "--history-path",
            history_path_arg,
            "--role",
            "coder",
            "--kind",
            "implementation",
            "--state",
            "completed",
        ])
        .output()
        .expect("failed to inspect artifacts with node routing filters");

    assert!(output.status.success(), "expected artifacts success, stderr was: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("moat artifacts\n"));
    assert!(stdout.contains("artifact_entries=1\n"));
    assert!(stdout.contains("|implementation|docs/superpowers/plans/implementation.md|Implementation handoff ready for reviewer\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn cli_filters_moat_artifacts_conjunctively_by_node_metadata() {
    let history_path = unique_history_path("artifacts-node-filter-conjunction");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let round_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to run mdid-cli moat round with history path");
    assert!(round_output.status.success(), "expected round success, stderr was: {}", String::from_utf8_lossy(&round_output.stderr));

    let claim_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "claim-task", "--history-path", history_path_arg, "--node-id", "implementation"])
        .output()
        .expect("failed to claim implementation task");
    assert!(claim_output.status.success(), "expected claim success, stderr was: {}", String::from_utf8_lossy(&claim_output.stderr));

    let complete_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "complete-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "implementation",
            "--artifact-ref",
            "docs/superpowers/plans/implementation.md",
            "--artifact-summary",
            "Implementation handoff ready for reviewer",
        ])
        .output()
        .expect("failed to complete implementation task with artifact");
    assert!(complete_output.status.success(), "expected complete success, stderr was: {}", String::from_utf8_lossy(&complete_output.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "artifacts",
            "--history-path",
            history_path_arg,
            "--role",
            "reviewer",
            "--kind",
            "implementation",
            "--state",
            "completed",
        ])
        .output()
        .expect("failed to inspect artifacts with mismatched node routing filters");

    assert!(output.status.success(), "expected artifacts success, stderr was: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|line| line.starts_with("artifact_entries="))
            .collect::<Vec<_>>(),
        vec!["artifact_entries=0"]
    );

    cleanup_history_path(&history_path);
}
```

- [x] **Step 2: Run tests to verify RED**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_filters_moat_artifacts_by_node_role_kind_and_state -- --exact
```

Expected: FAIL with `unknown flag: --role` from `mdid-cli moat artifacts`.

- [x] **Step 3: Add minimal implementation**

In `crates/mdid-cli/src/main.rs`, change `MoatArtifactsCommand` to:

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
}
```

In `parse_moat_artifacts_command`, add local variables:

```rust
let mut role = None;
let mut state = None;
let mut kind = None;
```

Add match arms before `--node-id`:

```rust
"--role" => {
    let value = required_flag_value(args, index, "--role", false)?;
    if role.is_some() {
        return Err(duplicate_flag_error("--role"));
    }
    role = Some(parse_agent_role(value)?);
}
"--state" => {
    let value = required_flag_value(args, index, "--state", false)?;
    if state.is_some() {
        return Err(duplicate_flag_error("--state"));
    }
    state = Some(parse_task_node_state(value)?);
}
"--kind" => {
    let value = required_flag_value(args, index, "--kind", false)?;
    if kind.is_some() {
        return Err(duplicate_flag_error("--kind"));
    }
    kind = Some(parse_moat_task_kind(value)?);
}
```

Include the new fields in the returned `MoatArtifactsCommand`.

In `run_moat_artifacts`, change the flattening to keep the node and filter on node metadata:

```rust
let mut artifacts = entry
    .report
    .control_plane
    .task_graph
    .nodes
    .iter()
    .filter(|node| command.role.map(|role| node.role == role).unwrap_or(true))
    .filter(|node| command.state.map(|state| node.state == state).unwrap_or(true))
    .filter(|node| command.kind.map(|kind| node.kind == kind).unwrap_or(true))
    .flat_map(|node| node.artifacts.iter().map(move |artifact| (node.node_id.as_str(), artifact)))
```

Leave the existing `--node-id`, `--contains`, `--artifact-ref`, `--artifact-summary`, and `--limit` behavior unchanged after this point.

Update the `moat artifacts` section in the `USAGE` constant to include:

```text
[--role planner|coder|reviewer] [--state pending|ready|in_progress|completed|blocked] [--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation]
```

- [x] **Step 4: Run tests to verify GREEN**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_filters_moat_artifacts_by_node_role_kind_and_state -- --exact
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_filters_moat_artifacts_conjunctively_by_node_metadata -- --exact
```

Expected: both PASS.

- [x] **Step 5: Run relevant regression tests**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_artifacts -- --nocapture
```

Expected: all tests whose names include `moat_artifacts` PASS.

- [x] **Step 6: Commit**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs docs/superpowers/plans/2026-04-27-med-de-id-moat-artifact-node-routing-filters.md
git commit -m "feat: filter moat artifacts by node metadata"
```
