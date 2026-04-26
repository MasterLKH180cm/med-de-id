# Moat Task Graph Title Filter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a read-only `--title-contains TEXT` filter to `mdid-cli moat task-graph` so operators can drill into latest persisted task graph nodes by title text.

**Architecture:** Extend the existing CLI parser and `MoatTaskGraphCommand` filter pipeline only. The command remains latest-round scoped and read-only over `LocalMoatHistoryStore::open_existing`, applying role, state, node-id, and title substring filters conjunctively before printing persisted graph rows unchanged.

**Tech Stack:** Rust workspace, `mdid-cli`, `mdid-runtime::LocalMoatHistoryStore`, existing CLI integration tests in `crates/mdid-cli/tests/moat_cli.rs`.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `title_contains: Option<String>` to `MoatTaskGraphCommand`.
  - Parse `--title-contains TEXT` with strict flag-value handling.
  - Reject duplicate `--title-contains` with `duplicate flag: --title-contains`.
  - Reject trailing or flag-like missing values with `missing value for --title-contains`.
  - Apply a case-sensitive substring filter to persisted `node.title` together with existing filters.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add failing CLI tests for positive title matching, empty title matching, role+title conjunction, missing value, flag-like value, duplicate flag, and read-only/no-append behavior.
- Modify: `README.md`
  - Document the new `--title-contains TEXT` option on `moat task-graph`.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Document the read-only exact/latest-round task graph title filter.
- Modify: this plan file
  - Check off steps after implementation and keep snippets aligned with shipped behavior.

### Task 1: Task Graph Title Filter

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
- Modify: `docs/superpowers/plans/2026-04-26-med-de-id-moat-task-graph-title-filter.md`

- [x] **Step 1: Write failing CLI tests**

Add these tests near the existing task graph filter tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn task_graph_filters_latest_graph_by_title_contains() {
    let history_path = unique_history_path("task-graph-title-contains");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history for title filter");
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--title-contains",
            "Strategy",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with title filter");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.lines().filter(|line| line.starts_with("node=")).count(), 1);
    assert!(stdout.contains("node=planner|strategy_generation|Strategy Generation|strategy_generation|"));
    assert!(!stdout.contains("node=planner|market_scan|"));

    cleanup_history_path(&history_path);
}

#[test]
fn task_graph_title_filter_returns_header_only_when_no_title_matches() {
    let history_path = unique_history_path("task-graph-title-empty");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history for empty title filter");
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--title-contains",
            "not in any persisted title",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with unmatched title filter");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "moat task graph\n");

    cleanup_history_path(&history_path);
}

#[test]
fn task_graph_title_filter_is_conjunctive_with_role_filter() {
    let history_path = unique_history_path("task-graph-title-role");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history for role and title filter");
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--role",
            "reviewer",
            "--title-contains",
            "Strategy",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with conjunctive filters");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "moat task graph\n");

    cleanup_history_path(&history_path);
}

#[test]
fn task_graph_rejects_missing_title_contains_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            "/tmp/mdid-unused-history.json",
            "--title-contains",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with missing title value");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("missing value for --title-contains\n{}\n", USAGE)
    );
}

#[test]
fn task_graph_rejects_flag_like_title_contains_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            "/tmp/mdid-unused-history.json",
            "--title-contains",
            "--role",
            "planner",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with flag-like title value");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("missing value for --title-contains\n{}\n", USAGE)
    );
}

#[test]
fn task_graph_rejects_duplicate_title_contains_filter() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            "/tmp/mdid-unused-history.json",
            "--title-contains",
            "Strategy",
            "--title-contains",
            "Review",
        ])
        .output()
        .expect("failed to run mdid-cli moat task-graph with duplicate title filter");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("duplicate flag: --title-contains\n{}\n", USAGE)
    );
}

#[test]
fn task_graph_title_filter_does_not_append_history() {
    let history_path = unique_history_path("task-graph-title-read-only");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history for title read-only check");
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));

    let inspect = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path_arg,
            "--title-contains",
            "Strategy",
        ])
        .output()
        .expect("failed to inspect moat task graph by title");
    assert!(inspect.status.success(), "{}", String::from_utf8_lossy(&inspect.stderr));

    let history = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "history", "--history-path", history_path_arg])
        .output()
        .expect("failed to inspect moat history after title task graph filter");
    assert!(history.status.success(), "{}", String::from_utf8_lossy(&history.stderr));
    assert!(String::from_utf8_lossy(&history.stdout).contains("entries=1\n"));

    cleanup_history_path(&history_path);
}
```

- [x] **Step 2: Run tests to verify RED**

Run the narrowest title-filter RED check, then the broader task-graph filter because the title-filter test names intentionally include both `title_contains` and `task_graph_title` substrings:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli title_contains -- --nocapture
cargo test -p mdid-cli --test moat_cli task_graph -- --nocapture
```

Expected: FAIL because `--title-contains` is not accepted or not implemented.

- [x] **Step 3: Implement minimal CLI parsing and filtering**

In `crates/mdid-cli/src/main.rs`, make these concrete changes:

```rust
struct MoatTaskGraphCommand {
    history_path: String,
    role: Option<AgentRole>,
    state: Option<MoatTaskNodeState>,
    node_id: Option<String>,
    title_contains: Option<String>,
}
```

Initialize `let mut title_contains = None;` in `parse_moat_task_graph_command`, add this match arm:

```rust
"--title-contains" => {
    let value = required_flag_value(args, index, "--title-contains", true)?;
    if title_contains.is_some() {
        return Err(duplicate_flag_error("--title-contains"));
    }
    title_contains = Some(value.to_string());
}
```

Include `title_contains` in the returned `MoatTaskGraphCommand` and apply this filter in `run_moat_task_graph` after the node-id filter:

```rust
if let Some(expected_title) = command.title_contains.as_deref() {
    if !node.title.contains(expected_title) {
        return false;
    }
}
```

Update the usage string to include `--title-contains TEXT` for the `moat task-graph` command.

- [x] **Step 4: Run targeted tests to verify GREEN**

Run the narrowest title-filter GREEN check, then the broader task-graph filter because the title-filter test names intentionally include both `title_contains` and `task_graph_title` substrings:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli title_contains -- --nocapture
cargo test -p mdid-cli --test moat_cli task_graph -- --nocapture
```

Expected: PASS for all title filter and task-graph tests.

- [x] **Step 5: Update docs and plan checkboxes**

Update `README.md` and `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md` to state:

```markdown
`mdid-cli moat task-graph --history-path PATH [--role planner|coder|reviewer] [--state pending|ready|in_progress|completed|blocked] [--node-id NODE_ID] [--title-contains TEXT]` inspects the latest persisted task graph without mutating history. Filters are conjunctive. `--node-id` uses exact persisted node-id matching, and `--title-contains` uses case-sensitive substring matching against persisted node titles.
```

Then change this plan's task checkboxes from `[ ]` to `[x]` only after the corresponding steps really completed.

- [x] **Step 6: Run broader verification**

Run:

```bash
source "$HOME/.cargo/env"
cargo fmt --check
cargo test -p mdid-cli --test moat_cli task_graph -- --nocapture
cargo test -p mdid-cli --test moat_cli moat -- --nocapture
cargo test -p mdid-cli
```

Expected: PASS.

- [x] **Step 7: Commit**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs README.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-26-med-de-id-moat-task-graph-title-filter.md
git commit -m "feat: filter moat task graph by title"
```

## Self-Review

- Spec coverage: The plan adds the requested safe, incremental Autonomous Multi-Agent System control-plane capability by improving read-only task graph inspection without launching agents, creating cron jobs, or mutating history.
- Placeholder scan: No TBD/TODO/fill-in placeholders remain.
- Type consistency: The plan uses existing `MoatTaskGraphCommand`, `AgentRole`, `MoatTaskNodeState`, `required_flag_value`, and `LocalMoatHistoryStore` names consistently with the current CLI architecture.
