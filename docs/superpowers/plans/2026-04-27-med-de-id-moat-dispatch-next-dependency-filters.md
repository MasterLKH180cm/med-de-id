# Moat Dispatch Next Dependency Filters Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add dependency-aware routing filters to `mdid-cli moat dispatch-next` so external multi-agent controllers can dispatch the next ready task by upstream dependency shape.

**Architecture:** Extend the existing bounded dispatch envelope only; do not add background agents, daemons, crawlers, PR automation, or cron jobs. The CLI parser will accept `--depends-on NODE_ID` and `--no-dependencies`, and the existing ready-node selection predicate will apply those filters before claim/dry-run output.

**Tech Stack:** Rust workspace, `mdid-cli` integration tests, local JSON moat history store, Cargo targeted test execution with `CARGO_INCREMENTAL=0`.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `depends_on: Option<String>` and `no_dependencies: bool` to `MoatDispatchNextCommand`.
  - Parse `--depends-on NODE_ID` and `--no-dependencies` in `parse_moat_dispatch_next_command`.
  - Filter selected ready nodes by exact dependency membership and/or empty dependency list in `select_dispatch_next_node`.
  - Update usage text for the new flags.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add CLI tests proving `dispatch-next --depends-on` dry-runs the first ready node whose persisted dependencies contain the requested upstream node.
  - Add CLI tests proving `dispatch-next --no-dependencies` dry-runs only ready nodes with an empty dependency list.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Truth-sync the shipped `dispatch-next` surface with the new dependency filters.

### Task 1: Dispatch-next dependency filters

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`

- [x] **Step 1: Write the failing `--depends-on` integration test**

Append this test near the existing `dispatch-next` filter tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn cli_dispatch_next_filters_ready_task_by_dependency() {
    let history_path = unique_history_path("dispatch-next-depends-on-filter");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let round_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history");
    assert!(
        round_output.status.success(),
        "expected seed round success, stderr was: {}",
        String::from_utf8_lossy(&round_output.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path_arg,
            "--depends-on",
            "market-scan",
            "--dry-run",
        ])
        .output()
        .expect("failed to run mdid-cli moat dispatch-next with dependency filter");

    assert!(
        output.status.success(),
        "expected success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("node_id=competitor-analysis\n"),
        "expected competitor-analysis dispatch, stdout was: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("dependencies=market-scan\n"),
        "expected persisted dependency output, stdout was: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    cleanup_history_path(&history_path);
}
```

- [x] **Step 2: Run the focused test and verify RED**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_dispatch_next_filters_ready_task_by_dependency -- --nocapture
```

Expected: FAIL with `unknown flag: --depends-on`.

- [x] **Step 3: Implement minimal `--depends-on` support**

In `crates/mdid-cli/src/main.rs`:

```rust
struct MoatDispatchNextCommand {
    history_path: String,
    round_id: Option<String>,
    role: Option<AgentRole>,
    kind: Option<MoatTaskNodeKind>,
    node_id: Option<String>,
    depends_on: Option<String>,
    no_dependencies: bool,
    title_contains: Option<String>,
    spec_ref: Option<String>,
    dry_run: bool,
}
```

Add parser locals:

```rust
let mut depends_on = None;
let mut no_dependencies = false;
```

Add match arms:

```rust
"--depends-on" => {
    let value = required_flag_value(args, index, "--depends-on", true)?;
    if depends_on.is_some() {
        return Err(duplicate_flag_error("--depends-on"));
    }
    depends_on = Some(value.to_string());
    index += 2;
}
"--no-dependencies" => {
    if no_dependencies {
        return Err(duplicate_flag_error("--no-dependencies"));
    }
    no_dependencies = true;
    index += 1;
}
```

Include both fields in the returned command, and add these predicate clauses to `select_dispatch_next_node`:

```rust
&& command
    .depends_on
    .as_deref()
    .map(|dependency| node.depends_on.iter().any(|node_dependency| node_dependency == dependency))
    .unwrap_or(true)
&& (!command.no_dependencies || node.depends_on.is_empty())
```

Update the usage string so `moat dispatch-next` includes `[--depends-on NODE_ID] [--no-dependencies]` between `[--node-id NODE_ID]` and `[--title-contains TEXT]`.

- [x] **Step 4: Run the focused test and verify GREEN**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_dispatch_next_filters_ready_task_by_dependency -- --nocapture
```

Expected: PASS.

- [x] **Step 5: Write the failing `--no-dependencies` integration test**

Append this second test near the same dispatch-next tests:

```rust
#[test]
fn cli_dispatch_next_filters_ready_task_to_nodes_without_dependencies() {
    let history_path = unique_history_path("dispatch-next-no-dependencies-filter");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let round_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history");
    assert!(
        round_output.status.success(),
        "expected seed round success, stderr was: {}",
        String::from_utf8_lossy(&round_output.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path_arg,
            "--no-dependencies",
            "--dry-run",
        ])
        .output()
        .expect("failed to run mdid-cli moat dispatch-next with no-dependencies filter");

    assert!(
        output.status.success(),
        "expected success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("node_id=market-scan\n"),
        "expected market-scan dispatch, stdout was: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("dependencies=<none>\n"),
        "expected empty dependency output, stdout was: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    cleanup_history_path(&history_path);
}
```

- [x] **Step 6: Run the second focused test and verify GREEN after minimal implementation**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_dispatch_next_filters_ready_task_to_nodes_without_dependencies -- --nocapture
```

Expected: PASS because Step 3 already implemented the required parser and predicate.

- [x] **Step 7: Truth-sync the moat-loop spec**

Replace the `dispatch-next` shipped-status bullet in `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md` so it says:

```markdown
- `mdid-cli moat dispatch-next --history-path PATH [--round-id ROUND_ID] [--role planner|coder|reviewer] [--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation] [--node-id NODE_ID] [--depends-on NODE_ID] [--no-dependencies] [--title-contains TEXT] [--spec-ref SPEC_REF] [--dry-run]` is a bounded one-task dispatch envelope for external Planner/Coder/Reviewer controllers. It opens only an existing history file, selects exactly one persisted ready node in task-graph order after optional round/role/kind/node-id/dependency/title/spec filters, with `--node-id NODE_ID` exact-matching the persisted ready `node.node_id`, `--depends-on NODE_ID` keeping only ready nodes whose persisted dependency list contains the requested upstream node ID exactly, `--no-dependencies` keeping only ready nodes with an empty persisted dependency list, `--title-contains TEXT` performing a case-sensitive substring match over the persisted ready node title before acceptance, and `--spec-ref SPEC_REF` exact-matching the raw persisted `node.spec_ref.as_deref() == Some(SPEC_REF)` before acceptance. Filters combine conjunctively. It emits deterministic task metadata plus the matching `complete-task` handoff command and never launches agents, daemons, PRs, crawlers, or cron jobs. `--dry-run` is read-only and reports `claimed=false`; without `--dry-run`, the command reloads current history and persists only the selected ready node's transition to `in_progress`, reporting `claimed=true`, `previous_state=ready`, and `new_state=in_progress`. If no ready node matches, it exits nonzero with `no ready moat task matched dispatch filters` and does not create or mutate history.
```

- [x] **Step 8: Run targeted dispatch-next test group**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli dispatch_next -- --nocapture
```

Expected: PASS for all dispatch-next tests.

- [x] **Step 9: Run broader CLI smoke validation**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli -- --nocapture
```

Expected: PASS.

- [x] **Step 10: Commit**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-27-med-de-id-moat-dispatch-next-dependency-filters.md
git commit -m "feat: filter moat dispatch by dependencies"
```
