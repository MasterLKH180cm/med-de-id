# Moat Task Graph Kind Filter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a read-only `--kind` filter to `mdid-cli moat task-graph` so operators can drill into latest persisted task graph nodes by exact persisted task-node kind.

**Architecture:** Extend only the CLI parsing and filtering surface for the existing read-only task graph inspection command. Preserve persisted field fidelity and output format; the command must continue opening existing history, inspecting only the latest round, and never mutating history or launching agents.

**Tech Stack:** Rust workspace, `mdid-cli`, existing runtime history store, Cargo integration tests.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Extend usage string to include `--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation`.
  - Add `kind: Option<MoatTaskNodeKind>` to `MoatTaskGraphCommand`.
  - Parse `--kind` with duplicate and missing-value handling matching other task-graph filters.
  - Add bounded `parse_moat_task_graph_kind_filter` using the persisted snake_case wire values.
  - Apply the `kind` filter conjunctively in `run_moat_task_graph`.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add TDD regression tests for `--kind` positive match, zero-match, conjunction with `--role`, unknown kind, missing value, flag-like missing value, duplicate flag, and read-only/no-append behavior.
  - Update the local `USAGE` constant to match CLI usage.
- Modify: `README.md`
  - Document the new `--kind` filter and its bounded values.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Document the new read-only task graph kind drilldown.
- Create: `docs/superpowers/plans/2026-04-26-med-de-id-moat-task-graph-kind-filter.md`
  - This implementation plan.

## Task 1: Add `--kind` filtering to `mdid-cli moat task-graph`

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
- Create: `docs/superpowers/plans/2026-04-26-med-de-id-moat-task-graph-kind-filter.md`

- [x] **Step 1: Write failing tests for kind filtering**

Add tests to `crates/mdid-cli/tests/moat_cli.rs` near the existing task graph filter tests:

```rust
#[test]
fn task_graph_filters_latest_graph_by_kind() {
    let history_path = unique_history_path("task-graph-kind");
    seed_successful_moat_history(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path.to_str().unwrap(),
            "--kind",
            "lock_in_analysis",
        ])
        .output()
        .expect("run moat task-graph kind filter");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("moat task graph\n"));
    assert!(stdout.contains("node=planner|lockin_analysis|Lock-in Analysis|lock_in_analysis|completed|competitor_analysis|<none>\n"));
    assert!(!stdout.contains("node=planner|strategy_generation|Strategy Generation|strategy_generation|ready|lockin_analysis|<none>\n"));

    let _ = std::fs::remove_file(history_path);
}

#[test]
fn task_graph_kind_filter_returns_header_only_when_no_nodes_match() {
    let history_path = unique_history_path("task-graph-kind-zero");
    seed_successful_moat_history(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path.to_str().unwrap(),
            "--kind",
            "evaluation",
        ])
        .output()
        .expect("run moat task-graph kind zero-match filter");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "moat task graph\n");

    let _ = std::fs::remove_file(history_path);
}

#[test]
fn task_graph_kind_filter_conjoins_with_role_filter() {
    let history_path = unique_history_path("task-graph-kind-role");
    seed_successful_moat_history(&history_path);

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path.to_str().unwrap(),
            "--role",
            "reviewer",
            "--kind",
            "strategy_generation",
        ])
        .output()
        .expect("run moat task-graph kind plus role filter");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "moat task graph\n");

    let _ = std::fs::remove_file(history_path);
}

#[test]
fn task_graph_rejects_unknown_kind_filter() {
    let history_path = unique_history_path("task-graph-kind-unknown");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path.to_str().unwrap(),
            "--kind",
            "operator",
        ])
        .output()
        .expect("run moat task-graph unknown kind");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown moat task-graph kind: operator"));
    assert!(stderr.contains(USAGE));
    assert!(!history_path.exists());
}

#[test]
fn task_graph_rejects_missing_kind_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "task-graph", "--history-path", "missing.json", "--kind"])
        .output()
        .expect("run moat task-graph missing kind value");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("missing value for --kind"));
    assert!(stderr.contains(USAGE));
}

#[test]
fn task_graph_rejects_flag_like_kind_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            "missing.json",
            "--kind",
            "--role",
            "planner",
        ])
        .output()
        .expect("run moat task-graph flag-like kind value");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("missing value for --kind"));
    assert!(stderr.contains(USAGE));
}

#[test]
fn task_graph_rejects_duplicate_kind_filter() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            "missing.json",
            "--kind",
            "market_scan",
            "--kind",
            "review",
        ])
        .output()
        .expect("run moat task-graph duplicate kind");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("duplicate flag: --kind"));
    assert!(stderr.contains(USAGE));
}

#[test]
fn task_graph_kind_filter_does_not_append_history() {
    let history_path = unique_history_path("task-graph-kind-readonly");
    seed_successful_moat_history(&history_path);

    let before = std::fs::read_to_string(&history_path).expect("read seeded history before kind filter");
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path.to_str().unwrap(),
            "--kind",
            "strategy_generation",
        ])
        .output()
        .expect("run moat task-graph kind read-only check");
    let after = std::fs::read_to_string(&history_path).expect("read seeded history after kind filter");

    assert!(output.status.success());
    assert_eq!(after, before);

    let _ = std::fs::remove_file(history_path);
}
```

- [ ] **Step 2: Run tests to verify RED**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli kind -- --nocapture
```

Expected: FAIL because `--kind` is still an unknown flag or missing from `MoatTaskGraphCommand`.

- [ ] **Step 3: Implement minimal CLI parsing and filtering**

In `crates/mdid-cli/src/main.rs`, make these changes:

```rust
const USAGE: &str = "usage: mdid-cli [status | moat round [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false] [--history-path PATH] | moat control-plane [--history-path PATH] [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false] | moat history --history-path PATH | moat decision-log --history-path PATH [--role planner|coder|reviewer] [--contains TEXT] | moat assignments --history-path PATH [--role planner|coder|reviewer] [--node-id NODE_ID] | moat task-graph --history-path PATH [--role planner|coder|reviewer] [--state pending|ready|in_progress|completed|blocked] [--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation] [--node-id NODE_ID] [--title-contains TEXT] | moat export-specs --history-path PATH --output-dir DIR | moat export-plans --history-path PATH --output-dir DIR | moat continue --history-path PATH [--improvement-threshold N] | moat schedule-next --history-path PATH [--improvement-threshold N]]";

struct MoatTaskGraphCommand {
    history_path: String,
    role: Option<AgentRole>,
    state: Option<MoatTaskNodeState>,
    kind: Option<MoatTaskNodeKind>,
    node_id: Option<String>,
    title_contains: Option<String>,
}

fn parse_moat_task_graph_kind_filter(value: &str) -> Result<MoatTaskNodeKind, String> {
    match value {
        "market_scan" => Ok(MoatTaskNodeKind::MarketScan),
        "competitor_analysis" => Ok(MoatTaskNodeKind::CompetitorAnalysis),
        "lock_in_analysis" => Ok(MoatTaskNodeKind::LockInAnalysis),
        "strategy_generation" => Ok(MoatTaskNodeKind::StrategyGeneration),
        "spec_planning" => Ok(MoatTaskNodeKind::SpecPlanning),
        "implementation" => Ok(MoatTaskNodeKind::Implementation),
        "review" => Ok(MoatTaskNodeKind::Review),
        "evaluation" => Ok(MoatTaskNodeKind::Evaluation),
        other => Err(format!("unknown moat task-graph kind: {other}")),
    }
}
```

Add `--kind` parsing in `parse_moat_task_graph_command` using `required_flag_value(args, index, "--kind", true)?`, duplicate rejection with `duplicate_flag_error("--kind")`, and `kind = Some(parse_moat_task_graph_kind_filter(value)?);`.

Update `run_moat_task_graph` node filtering with:

```rust
if let Some(kind) = command.kind {
    if node.kind != kind {
        continue;
    }
}
```

Update the test `USAGE` constant in `crates/mdid-cli/tests/moat_cli.rs` to the same command shape.

- [ ] **Step 4: Update docs/spec**

In `README.md`, update the task graph command sentence to:

```markdown
- `mdid-cli moat task-graph --history-path PATH [--role planner|coder|reviewer] [--state pending|ready|in_progress|completed|blocked] [--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation] [--node-id NODE_ID] [--title-contains TEXT]` inspects the latest persisted task graph read-only and prints `moat task graph` followed by deterministic `node=<role>|<node_id>|<title>|<kind>|<state>|<dependencies>|<spec_ref>` rows. Missing dependency/spec fields print `<none>`, dependency lists are comma-joined, and pipe-delimited string fields are escaped. Filters are conjunctive and compare against persisted fields exactly; `--kind` uses the persisted snake_case task-kind wire values.
```

In `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`, update the matching task graph inspection section with the same command contract and read-only guarantee.

- [ ] **Step 5: Run targeted tests to verify GREEN**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli kind -- --nocapture
```

Expected: PASS for all kind-filter tests.

- [ ] **Step 6: Run broader relevant verification**

Run:

```bash
source "$HOME/.cargo/env"
cargo fmt --check
cargo test -p mdid-cli --test moat_cli task_graph -- --nocapture
cargo test -p mdid-cli --test moat_cli moat -- --nocapture
cargo test -p mdid-cli
```

Expected: all commands PASS.

- [ ] **Step 7: Commit**

Run:

```bash
git add README.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-26-med-de-id-moat-task-graph-kind-filter.md crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs
git commit -m "feat: filter moat task graph by kind"
```
