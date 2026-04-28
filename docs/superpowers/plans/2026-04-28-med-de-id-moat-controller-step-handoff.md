# Moat Controller Step Handoff Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded local `mdid-cli moat controller-step` command that selects one ready moat task, optionally claims it, and emits a work-packet handoff in a single external-controller-friendly step.

**Architecture:** Compose the existing dispatch-next selection/claim semantics with the existing work-packet handoff semantics, without launching agents or adding daemon/scheduler behavior. The command is a one-shot local CLI surface over an explicitly supplied history file; dry-run is read-only, non-dry-run only mutates the selected task claim/lease state in local history.

**Tech Stack:** Rust workspace, `mdid-cli`, `mdid-runtime::moat_history`, existing CLI integration tests in `crates/mdid-cli/tests/moat_cli.rs`, serde JSON output.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `MoatControllerStepCommand`, parser, dispatch match arm, usage text, and `run_moat_controller_step`.
  - Reuse/adapter-convert existing `MoatDispatchNextCommand` filtering and claim logic.
  - Reuse/extract work-packet rendering helpers so controller-step can include the selected task context.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add RED/GREEN CLI integration tests for JSON/text controller-step, dry-run no-mutation, filtering, missing history, and invalid lease/format.
  - Update test-local `USAGE` mirror if present.
- Modify: `README.md`
  - Document `moat controller-step` as a bounded local external-controller handoff, not an agent launcher.
- Modify: `AGENTS.md`
  - Add rules for the controller-step command and forbidden interpretations.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Truth-sync implementation status with the new shipped bounded handoff slice and clarify that current repo does not own autonomous process execution.
- Create/keep: `docs/superpowers/plans/2026-04-28-med-de-id-moat-controller-step-handoff.md`
  - This plan.

### Task 1: CLI controller-step handoff command

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`

- [x] **Step 1: Write failing JSON controller-step test**

Add this test near existing `dispatch-next` / `work-packet` tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn moat_controller_step_json_claims_ready_task_and_embeds_work_packet() {
    let history_path = unique_history_path("controller-step-json");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--review-loops",
            "0",
            "--history-path",
            history_path.to_str().unwrap(),
        ])
        .output()
        .expect("failed to seed moat history for controller-step json");
    assert!(seed.status.success(), "seed failed: {}", String::from_utf8_lossy(&seed.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "controller-step",
            "--history-path",
            history_path.to_str().unwrap(),
            "--agent-id",
            "reviewer-1",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run moat controller-step json");

    assert!(output.status.success(), "controller-step failed: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8(output.stdout).expect("controller-step stdout was not utf8");
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("controller-step stdout was not json");

    assert_eq!(json["type"], "moat_controller_step");
    assert_eq!(json["history_path"], history_path.to_string_lossy().as_ref());
    assert_eq!(json["dry_run"], false);
    assert_eq!(json["claimed"], true);
    assert_eq!(json["agent_id"], "reviewer-1");
    assert_eq!(json["assigned_agent_id"], "reviewer-1");
    assert_eq!(json["node_id"], "review");
    assert_eq!(json["role"], "reviewer");
    assert_eq!(json["kind"], "review");
    assert_eq!(json["previous_state"], "ready");
    assert_eq!(json["new_state"], "in_progress");
    assert_eq!(json["lease_seconds"], 900);
    assert!(json["complete_command"].as_str().unwrap().contains("moat complete-task"));

    let packet = &json["work_packet"];
    assert_eq!(packet["type"], "moat_work_packet");
    assert_eq!(packet["node_id"], "review");
    assert_eq!(packet["role"], "reviewer");
    assert!(packet["acceptance_criteria"].as_array().unwrap().iter().any(|value| value.as_str().unwrap().contains("Use SDD and TDD")));
    assert!(packet["complete_command"].as_str().unwrap().contains("moat complete-task"));

    let graph = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path.to_str().unwrap(),
            "--node-id",
            "review",
        ])
        .output()
        .expect("failed to inspect controller-step task graph");
    assert!(graph.status.success(), "task-graph failed: {}", String::from_utf8_lossy(&graph.stderr));
    let graph_stdout = String::from_utf8(graph.stdout).expect("task graph stdout was not utf8");
    assert!(graph_stdout.contains("node=reviewer|review|Review Implementation|review|in_progress|implementation|<none>"));
    assert!(graph_stdout.contains("assigned_agent_id=review|reviewer-1"));
}
```

- [x] **Step 2: Run JSON test to verify RED**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_controller_step_json_claims_ready_task_and_embeds_work_packet -- --nocapture
```

Expected: FAIL because `controller-step` is not a recognized `moat` subcommand.

- [x] **Step 3: Write failing dry-run no-mutation test**

Add this test in the same test cluster:

```rust
#[test]
fn moat_controller_step_dry_run_json_exports_packet_without_mutating_history() {
    let history_path = unique_history_path("controller-step-dry-run");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--review-loops",
            "0",
            "--history-path",
            history_path.to_str().unwrap(),
        ])
        .output()
        .expect("failed to seed moat history for controller-step dry-run");
    assert!(seed.status.success(), "seed failed: {}", String::from_utf8_lossy(&seed.stderr));
    let before = std::fs::read_to_string(&history_path).expect("failed to read seeded history");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "controller-step",
            "--history-path",
            history_path.to_str().unwrap(),
            "--dry-run",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run dry-run moat controller-step");

    assert!(output.status.success(), "controller-step dry-run failed: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8(output.stdout).expect("controller-step dry-run stdout was not utf8");
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("controller-step dry-run stdout was not json");
    assert_eq!(json["type"], "moat_controller_step");
    assert_eq!(json["dry_run"], true);
    assert_eq!(json["claimed"], false);
    assert_eq!(json["assigned_agent_id"], serde_json::Value::Null);
    assert_eq!(json["node_id"], "review");
    assert_eq!(json["work_packet"]["node_id"], "review");

    let after = std::fs::read_to_string(&history_path).expect("failed to read dry-run history");
    assert_eq!(after, before, "dry-run controller-step mutated history");
}
```

- [x] **Step 4: Run dry-run test to verify RED**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_controller_step_dry_run_json_exports_packet_without_mutating_history -- --nocapture
```

Expected: FAIL because `controller-step` is not implemented.

- [x] **Step 5: Write failing text output and parser validation tests**

Add these tests:

```rust
#[test]
fn moat_controller_step_text_prints_bounded_handoff() {
    let history_path = unique_history_path("controller-step-text");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--review-loops",
            "0",
            "--history-path",
            history_path.to_str().unwrap(),
        ])
        .output()
        .expect("failed to seed moat history for controller-step text");
    assert!(seed.status.success(), "seed failed: {}", String::from_utf8_lossy(&seed.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "controller-step",
            "--history-path",
            history_path.to_str().unwrap(),
            "--agent-id",
            "reviewer-text",
        ])
        .output()
        .expect("failed to run moat controller-step text");

    assert!(output.status.success(), "controller-step text failed: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8(output.stdout).expect("controller-step text stdout was not utf8");
    assert!(stdout.contains("moat controller step"));
    assert!(stdout.contains("claimed=true"));
    assert!(stdout.contains("node_id=review"));
    assert!(stdout.contains("role=reviewer"));
    assert!(stdout.contains("kind=review"));
    assert!(stdout.contains("previous_state=ready"));
    assert!(stdout.contains("new_state=in_progress"));
    assert!(stdout.contains("complete_command=mdid-cli moat complete-task"));
    assert!(stdout.contains("acceptance=Use SDD and TDD"));
}

#[test]
fn moat_controller_step_filters_select_exact_ready_task() {
    let history_path = unique_history_path("controller-step-filters");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--review-loops",
            "0",
            "--history-path",
            history_path.to_str().unwrap(),
        ])
        .output()
        .expect("failed to seed moat history for controller-step filters");
    assert!(seed.status.success(), "seed failed: {}", String::from_utf8_lossy(&seed.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "controller-step",
            "--history-path",
            history_path.to_str().unwrap(),
            "--role",
            "reviewer",
            "--kind",
            "review",
            "--node-id",
            "review",
            "--dry-run",
        ])
        .output()
        .expect("failed to run filtered moat controller-step");
    assert!(output.status.success(), "filtered controller-step failed: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8(output.stdout).expect("filtered controller-step stdout was not utf8");
    assert!(stdout.contains("node_id=review"));
    assert!(stdout.contains("dry_run=true"));
    assert!(stdout.contains("claimed=false"));
}

#[test]
fn moat_controller_step_rejects_missing_history_without_creating_it() {
    let history_path = unique_history_path("controller-step-missing-history");
    let _ = std::fs::remove_file(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "controller-step",
            "--history-path",
            history_path.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run moat controller-step with missing history");

    assert!(!output.status.success(), "controller-step unexpectedly succeeded");
    let stderr = String::from_utf8(output.stderr).expect("stderr was not utf8");
    assert!(stderr.contains("failed to open moat history store"));
    assert!(!history_path.exists(), "controller-step created a missing history file");
}

#[test]
fn moat_controller_step_rejects_invalid_format_and_non_positive_lease() {
    let unknown_format = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "controller-step", "--history-path", "history.json", "--format", "yaml"])
        .output()
        .expect("failed to run moat controller-step with unknown format");
    assert!(!unknown_format.status.success(), "controller-step unexpectedly accepted yaml format");
    let stderr = String::from_utf8(unknown_format.stderr).expect("stderr was not utf8");
    assert!(stderr.contains("unknown moat controller-step format: yaml"));

    let bad_lease = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "controller-step", "--history-path", "history.json", "--lease-seconds", "0"])
        .output()
        .expect("failed to run moat controller-step with bad lease");
    assert!(!bad_lease.status.success(), "controller-step unexpectedly accepted zero lease");
    let stderr = String::from_utf8(bad_lease.stderr).expect("stderr was not utf8");
    assert!(stderr.contains("moat controller-step --lease-seconds must be positive"));
}
```

- [x] **Step 6: Run text/parser tests to verify RED**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_controller_step -- --nocapture
```

Expected: FAIL because `controller-step` is not implemented.

- [x] **Step 7: Implement minimal CLI parser and command wiring**

In `crates/mdid-cli/src/main.rs`, add `MoatControllerStepCommand` with the same fields as `MoatDispatchNextCommand`. Add a `CliCommand::MoatControllerStep(MoatControllerStepCommand)` variant. In `parse_command`, route `moat controller-step` to `parse_moat_controller_step_command`. Implement parser handling for:

```rust
--history-path PATH
--round-id ROUND_ID
--role planner|coder|reviewer
--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation
--node-id NODE_ID
--depends-on NODE_ID
--no-dependencies
--requires-artifacts
--title-contains TEXT
--spec-ref SPEC_REF
--agent-id AGENT_ID
--lease-seconds N
--dry-run
--format text|json
```

Required parser errors:

```text
missing --history-path for moat controller-step
missing value for moat controller-step --history-path
missing value for moat controller-step --round-id
missing value for moat controller-step --role
unknown moat task role: <value>
missing value for moat controller-step --kind
unknown moat task kind: <value>
missing value for moat controller-step --node-id
missing value for moat controller-step --depends-on
moat controller-step cannot combine --depends-on and --no-dependencies
missing value for moat controller-step --title-contains
missing value for moat controller-step --spec-ref
missing value for moat controller-step --agent-id
missing value for moat controller-step --lease-seconds
moat controller-step --lease-seconds must be positive
missing value for moat controller-step --format
unknown moat controller-step format: <value>
unknown option for moat controller-step: <flag>
```

Update both CLI usage constants (production and test mirror) to include:

```text
moat controller-step --history-path PATH [--round-id ROUND_ID] [--role planner|coder|reviewer] [--kind KIND] [--node-id NODE_ID] [--depends-on NODE_ID] [--no-dependencies] [--requires-artifacts] [--title-contains TEXT] [--spec-ref SPEC_REF] [--agent-id AGENT_ID] [--lease-seconds N] [--dry-run] [--format text|json]
```

- [x] **Step 8: Implement minimal runner by composing dispatch selection with work-packet output**

In `crates/mdid-cli/src/main.rs` implement `run_moat_controller_step`:

1. Open history with `LocalMoatHistoryStore::open_existing(&command.history_path)`.
2. Select latest or exact round using existing dispatch-next selection behavior.
3. Apply all filters and select exactly one ready node.
4. If `dry_run=false`, reload latest history before mutation and claim the selected task with lease/agent semantics equivalent to `dispatch-next`.
5. Build the work-packet data for the selected node from the selected persisted entry.
6. Print text or JSON.

Required JSON shape:

```json
{
  "type": "moat_controller_step",
  "history_path": "...",
  "dry_run": false,
  "claimed": true,
  "agent_id": "reviewer-1",
  "assigned_agent_id": "reviewer-1",
  "round_id": "...",
  "node_id": "review",
  "role": "reviewer",
  "kind": "review",
  "title": "Review Implementation",
  "dependencies": ["implementation"],
  "spec_ref": null,
  "complete_command": "mdid-cli moat complete-task ...",
  "previous_state": "ready",
  "new_state": "in_progress",
  "lease_seconds": 900,
  "work_packet": {
    "type": "moat_work_packet",
    "history_path": "...",
    "round_id": "...",
    "node_id": "review",
    "role": "reviewer",
    "kind": "review",
    "title": "Review Implementation",
    "dependencies": ["implementation"],
    "dependency_artifacts": [],
    "acceptance_criteria": ["Use SDD and TDD before completing this task.", "Record artifact handoff with moat complete-task when work is complete."],
    "complete_command": "mdid-cli moat complete-task ..."
  },
  "constraints": {
    "local_only": true,
    "bounded_one_task": true,
    "no_agent_launch": true,
    "no_daemon": true,
    "no_background_work": true,
    "no_crawling": true,
    "no_pr_creation": true,
    "no_cron_creation": true,
    "no_artifact_writes": true
  }
}
```

Text output must include at least:

```text
moat controller step
history_path=...
dry_run=true|false
claimed=true|false
agent_id=<none>|...
assigned_agent_id=<none>|...
round_id=...
node_id=...
role=...
kind=...
title=...
dependencies=<none>|...
spec_ref=<none>|...
complete_command=mdid-cli moat complete-task ...
acceptance=Use SDD and TDD before completing this task.
acceptance=Record artifact handoff with moat complete-task when work is complete.
```

When claimed, text must also include:

```text
lease_seconds=900
previous_state=ready
new_state=in_progress
```

- [x] **Step 9: Run controller-step tests to verify GREEN**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_controller_step -- --nocapture
```

Expected: PASS.

- [x] **Step 10: Run related dispatch/work-packet regression tests**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_dispatch_next -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_work_packet -- --nocapture
```

Expected: PASS.

- [x] **Step 11: Commit Task 1**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs docs/superpowers/plans/2026-04-28-med-de-id-moat-controller-step-handoff.md
git commit -m "feat(cli): add moat controller-step handoff"
```

### Task 2: Documentation and spec truth-sync

**Files:**
- Modify: `README.md`
- Modify: `AGENTS.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`

- [ ] **Step 1: Update README controller-step section**

Add near the existing dispatch/work-packet docs in `README.md`:

```markdown
### Moat controller step handoff

External controllers that need one bounded unit of local work can combine task routing and work-packet export with:

```bash
cargo run -p mdid-cli -- moat controller-step --history-path .mdid/moat-history.json --agent-id reviewer-1
cargo run -p mdid-cli -- moat controller-step --history-path .mdid/moat-history.json --agent-id reviewer-1 --format json
cargo run -p mdid-cli -- moat controller-step --history-path .mdid/moat-history.json --role reviewer --kind review --node-id review --dry-run --format json
```

`moat controller-step` is a one-shot local external-controller handoff. It opens an existing local history file, selects at most one ready task with the same routing filters as `dispatch-next`, and emits the selected task plus a work-packet context in text or JSON. Non-dry-run mode claims the task and records local lease metadata in the history file; `--dry-run` is read-only and does not mutate history. This command does not launch agents, supervise processes, run as a daemon, schedule background work, crawl data, open PRs, create cron jobs, write code, or generate artifact files.
```

- [x] **Step 2: Update AGENTS controller-step rules**

Add to `AGENTS.md`:

```markdown
## Moat controller step

`mdid-cli moat controller-step --history-path PATH [--round-id ROUND_ID] [--role planner|coder|reviewer] [--kind KIND] [--node-id NODE_ID] [--depends-on NODE_ID] [--no-dependencies] [--requires-artifacts] [--title-contains TEXT] [--spec-ref SPEC_REF] [--agent-id AGENT_ID] [--lease-seconds N] [--dry-run] [--format text|json]` is a bounded local handoff command for external controllers. It selects at most one ready task, optionally claims it with local lease metadata, and emits a work-packet context. It must not be treated as an agent launcher, daemon, crawler, PR automation, cron job, background scheduler, code writer, or artifact generator.
```

- [x] **Step 3: Update moat-loop spec implementation status**

In `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`, add an implementation-status bullet after the `dispatch-next` / `work-packet` bullets:

```markdown
- `mdid-cli moat controller-step --history-path PATH [--round-id ROUND_ID] [--role planner|coder|reviewer] [--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation] [--node-id NODE_ID] [--depends-on NODE_ID] [--no-dependencies] [--requires-artifacts] [--title-contains TEXT] [--spec-ref SPEC_REF] [--agent-id AGENT_ID] [--lease-seconds N] [--dry-run] [--format text|json]` is a bounded one-shot local handoff for external controllers. It composes the existing ready-task dispatch filters with work-packet context export, selects at most one ready task, and in non-dry-run mode claims only that task with local lease metadata. Text output is default; `--format json` emits a deterministic `moat_controller_step` envelope with the selected task, claim metadata, embedded `moat_work_packet`, complete command, and local-only constraints. It does not launch agents, supervise processes, schedule background work, run as a daemon, crawl data, open PRs, create cron jobs, write code, or generate artifact files.
```

Also update the “shipped slice is intentionally narrower” summary paragraph to include bounded `controller-step` handoff while preserving the statement that full autonomous Planner/Coder/Reviewer process execution remains future work.

- [x] **Step 4: Run docs-related regression tests**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_controller_step -- --nocapture
```

Expected: PASS.

- [x] **Step 5: Commit Task 2**

```bash
git add README.md AGENTS.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md
git commit -m "docs: document moat controller-step handoff"
```

### Task 3: Final verification and develop merge

**Files:**
- Verify all modified files.

- [ ] **Step 1: Run targeted verification**

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_controller_step -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_dispatch_next -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_work_packet -- --nocapture
```

Expected: PASS for all targeted tests.

- [x] **Step 2: Run broader CLI moat test slice if disk budget permits**

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli -- --nocapture
```

Expected: PASS. If disk/time is constrained, record why the broader run was skipped and keep targeted verification results.

- [x] **Step 3: Inspect changed files and status**

```bash
git status --short
git diff --stat develop...HEAD
```

Expected: only controller-step implementation/docs/plan files are modified or committed on the feature branch.

- [x] **Step 4: Merge feature branch to develop using gitflow**

```bash
git checkout develop
git pull --ff-only origin develop
git merge --no-ff feature/moat-loop-autonomy -m "feat: add moat controller-step handoff"
```

Expected: merge succeeds. If `develop` has advanced or conflicts, stop the merge, truth-sync, resolve only controller-step-related conflicts, rerun targeted verification, then complete the merge.

- [x] **Step 5: Final status**

```bash
git status --short
git branch --show-current
```

Expected: on `develop` with a clean worktree, unless local unpushed commits remain by design.

## Self-Review

- Spec coverage: This plan implements a conservative next Autonomous Multi-Agent System slice: one-shot local controller handoff combining routing/claim and work-packet context. It deliberately does not implement daemon scheduling, live crawling, PR automation, code writing, or process execution.
- Placeholder scan: No TODO/TBD/fill-in placeholders are present. Tests, commands, error messages, files, and expected outputs are explicit.
- Type consistency: The command is consistently named `controller-step`; JSON type is consistently `moat_controller_step`; text header is consistently `moat controller step`; command struct name is consistently `MoatControllerStepCommand`.
