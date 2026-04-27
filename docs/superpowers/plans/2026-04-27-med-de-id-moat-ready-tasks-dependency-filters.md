# Moat Ready Tasks Dependency Filters Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add read-only `mdid-cli moat ready-tasks --depends-on NODE_ID` and `--no-dependencies` filters so autonomous controllers can select claimable ready tasks by dependency shape.

**Architecture:** Extend the existing `MoatReadyTasksCommand` parser and renderer only; reuse the persisted latest-round task graph and the same dependency semantics already shipped for `task-graph` and `assignments`. Filters remain conjunctive with the existing ready-state predicate and never mutate history.

**Tech Stack:** Rust workspace, `mdid-cli`, Cargo integration tests in `crates/mdid-cli/tests/moat_cli.rs`.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `depends_on: Option<String>` and `no_dependencies: bool` to `MoatReadyTasksCommand`.
  - Parse `--depends-on NODE_ID` and `--no-dependencies` for `moat ready-tasks`.
  - Apply both filters inside `run_moat_ready_tasks` before output and limit handling.
  - Update the usage string and spec summary text.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add integration tests covering matching `--depends-on`, matching `--no-dependencies`, and missing value error.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Update the shipped `ready-tasks` command bullet to include the new filters.

### Task 1: Add ready-task dependency filters

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`

- [ ] **Step 1: Write the failing tests**

Add these tests to `crates/mdid-cli/tests/moat_cli.rs` near the existing ready-task filter tests:

```rust
#[test]
fn moat_ready_tasks_filters_by_dependency_node_id() {
    let history_path = unique_history_path("ready-tasks-depends-on");
    write_history_fixture(&history_path, sample_history_json());
    let history_path_arg = history_path.to_string_lossy().to_string();

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "ready-tasks",
            "--history-path",
            history_path_arg.as_str(),
            "--depends-on",
            "implementation",
        ])
        .output()
        .expect("failed to run mdid-cli moat ready-tasks with dependency filter");

    assert!(output.status.success(), "ready-tasks failed: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("moat ready tasks\n"));
    assert!(stdout.contains("node=reviewer|review|Review|review|2026-04-25-med-de-id-moat-loop-design.md\n"));
    assert!(!stdout.contains("node=planner|market_scan|"));
}

#[test]
fn moat_ready_tasks_filters_to_nodes_without_dependencies() {
    let history_path = unique_history_path("ready-tasks-no-dependencies");
    write_history_fixture(&history_path, sample_history_json());
    let history_path_arg = history_path.to_string_lossy().to_string();

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "ready-tasks",
            "--history-path",
            history_path_arg.as_str(),
            "--no-dependencies",
        ])
        .output()
        .expect("failed to run mdid-cli moat ready-tasks with no-dependencies filter");

    assert!(output.status.success(), "ready-tasks failed: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("moat ready tasks\n"));
    assert!(stdout.contains("node=planner|market_scan|Market Scan|market_scan|<none>\n"));
    assert!(!stdout.contains("node=reviewer|review|"));
}

#[test]
fn moat_ready_tasks_rejects_missing_dependency_filter_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "ready-tasks", "--depends-on"])
        .output()
        .expect("failed to run mdid-cli moat ready-tasks with missing dependency value");

    assert!(!output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stderr), "error: --depends-on requires a node id\n");
}
```

- [ ] **Step 2: Run tests to verify RED**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_ready_tasks_filters_by_dependency_node_id moat_ready_tasks_filters_to_nodes_without_dependencies moat_ready_tasks_rejects_missing_dependency_filter_value -- --nocapture
```

Expected: fail because `--depends-on` and `--no-dependencies` are not recognized for `ready-tasks` yet.

- [ ] **Step 3: Implement parser and filter support**

In `crates/mdid-cli/src/main.rs`, update `MoatReadyTasksCommand`:

```rust
struct MoatReadyTasksCommand {
    history_path: String,
    round_id: Option<String>,
    role: Option<AgentRole>,
    kind: Option<MoatTaskNodeKind>,
    node_id: Option<String>,
    depends_on: Option<String>,
    no_dependencies: bool,
    title_contains: Option<String>,
    spec_ref: Option<String>,
    limit: Option<usize>,
}
```

Update `parse_moat_ready_tasks_command` to initialize `depends_on` and `no_dependencies`, parse `--depends-on`, parse `--no-dependencies`, reject duplicate `--depends-on`, and include fields in the returned struct:

```rust
let mut depends_on = None;
let mut no_dependencies = false;
```

```rust
"--depends-on" => {
    if depends_on.is_some() {
        return Err("--depends-on provided more than once".to_string());
    }
    let value = args
        .get(index + 1)
        .ok_or_else(|| "--depends-on requires a node id".to_string())?;
    depends_on = Some(value.clone());
    index += 2;
}
"--no-dependencies" => {
    no_dependencies = true;
    index += 1;
}
```

```rust
Ok(MoatReadyTasksCommand {
    history_path,
    round_id,
    role,
    kind,
    node_id,
    depends_on,
    no_dependencies,
    title_contains,
    spec_ref,
    limit,
})
```

In `run_moat_ready_tasks`, add filters next to the existing role/kind/node/title/spec filters:

```rust
if let Some(depends_on) = command.depends_on.as_deref() {
    if !node.depends_on.iter().any(|dependency| dependency == depends_on) {
        return false;
    }
}
if command.no_dependencies && !node.depends_on.is_empty() {
    return false;
}
```

Update the usage string to include:

```text
[--depends-on NODE_ID] [--no-dependencies]
```

inside the `moat ready-tasks` command syntax.

- [ ] **Step 4: Run tests to verify GREEN**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_ready_tasks_filters_by_dependency_node_id moat_ready_tasks_filters_to_nodes_without_dependencies moat_ready_tasks_rejects_missing_dependency_filter_value -- --nocapture
```

Expected: all three tests pass.

- [ ] **Step 5: Update spec docs**

In `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`, update the `ready-tasks` shipped foundation bullet so the command signature includes:

```text
[--depends-on NODE_ID] [--no-dependencies]
```

and add this exact sentence:

```text
`--depends-on` selects only ready nodes whose persisted dependency list contains the given node id; `--no-dependencies` selects only ready nodes with an empty dependency list, and both filters are read-only and conjunctive with the existing ready-task filters.
```

- [ ] **Step 6: Run relevant broader verification**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_ready_tasks -- --nocapture
```

Expected: all ready-task related tests pass.

- [ ] **Step 7: Commit**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-27-med-de-id-moat-ready-tasks-dependency-filters.md
git commit -m "feat: filter moat ready tasks by dependencies"
```

## Self-Review

- Spec coverage: covers dependency-based ready-task routing for external autonomous controllers while preserving read-only behavior.
- Placeholder scan: no TBD/TODO/fill-in placeholders remain.
- Type consistency: `depends_on` and `no_dependencies` match existing task graph and assignments vocabulary and use persisted `node.depends_on` values.
