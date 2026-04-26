# Med De ID Moat Task Graph Spec Ref Filter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a read-only `--spec-ref SPEC_REF` filter to `mdid-cli moat task-graph` so operators can drill into latest persisted task graph nodes tied to a specific implementation handoff reference.

**Architecture:** Extend the existing CLI-only task graph inspection surface. Parsing adds one optional exact-match string field to `MoatTaskGraphCommand`; execution filters latest persisted `node.spec_ref.as_deref()` before printing the existing escaped node rows. No history mutation, scheduling, agent launch, cron creation, or runtime/app-layer behavior changes.

**Tech Stack:** Rust, Cargo workspace, `mdid-cli`, existing `LocalMoatHistoryStore`, existing `std::process::Command` integration tests.

---

## File Structure

- Modify `crates/mdid-cli/src/main.rs`
  - Add `spec_ref: Option<String>` to `MoatTaskGraphCommand`.
  - Parse `--spec-ref SPEC_REF` with strict flag-like value rejection and duplicate detection.
  - Apply exact raw persisted spec-ref matching in `run_moat_task_graph`.
  - Update CLI usage string.
- Modify `crates/mdid-cli/tests/moat_cli.rs`
  - Update test `USAGE` constant.
  - Add integration tests for positive exact match, zero match, role conjunction, missing value, flag-like missing value, duplicate flag, and read-only/no-append behavior.
- Modify `README.md`
  - Document the new `--spec-ref` flag and read-only semantics.
- Modify `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Document task graph spec-ref filtering as latest-round, read-only, exact-match inspection.
- Modify this plan file
  - Check completed steps after implementation and review.

### Task 1: Add `--spec-ref` to task graph inspection

**Files:**
- Modify: `crates/mdid-cli/src/main.rs:50-57`
- Modify: `crates/mdid-cli/src/main.rs:352-419`
- Modify: `crates/mdid-cli/src/main.rs:931-983`
- Modify: `crates/mdid-cli/src/main.rs:1388-1389`
- Test: `crates/mdid-cli/tests/moat_cli.rs`
- Docs: `README.md`
- Docs: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`

- [x] **Step 1: Write failing CLI tests for spec-ref filtering**

Add tests in `crates/mdid-cli/tests/moat_cli.rs` near existing `task_graph` tests. Use existing helpers such as `binary_path()`, `unique_history_path()`, and existing history seeding style.

```rust
#[test]
fn task_graph_filters_latest_graph_by_spec_ref() {
    let history_path = unique_history_path("task-graph-spec-ref");
    let seed_output = Command::new(binary_path())
        .args(["moat", "round", "--history-path"])
        .arg(&history_path)
        .output()
        .expect("run moat round");
    assert!(seed_output.status.success());

    let output = Command::new(binary_path())
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path.to_str().unwrap(),
            "--spec-ref",
            "docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md",
        ])
        .output()
        .expect("run task graph");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("moat task graph\n"));
    assert!(stdout.contains("|docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md\n"));
    assert!(!stdout.contains("|<none>\n"));

    let _ = std::fs::remove_file(history_path);
}

#[test]
fn task_graph_spec_ref_filter_returns_header_only_when_no_node_matches() {
    let history_path = unique_history_path("task-graph-spec-ref-empty");
    let seed_output = Command::new(binary_path())
        .args(["moat", "round", "--history-path"])
        .arg(&history_path)
        .output()
        .expect("run moat round");
    assert!(seed_output.status.success());

    let output = Command::new(binary_path())
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path.to_str().unwrap(),
            "--spec-ref",
            "docs/superpowers/specs/does-not-exist.md",
        ])
        .output()
        .expect("run task graph");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "moat task graph\n");

    let _ = std::fs::remove_file(history_path);
}

#[test]
fn task_graph_spec_ref_filter_conjoins_with_role_filter() {
    let history_path = unique_history_path("task-graph-spec-ref-role");
    let seed_output = Command::new(binary_path())
        .args(["moat", "round", "--history-path"])
        .arg(&history_path)
        .output()
        .expect("run moat round");
    assert!(seed_output.status.success());

    let output = Command::new(binary_path())
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path.to_str().unwrap(),
            "--role",
            "planner",
            "--spec-ref",
            "docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md",
        ])
        .output()
        .expect("run task graph");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("node=planner|"));
    assert!(!stdout.contains("node=coder|"));
    assert!(!stdout.contains("node=reviewer|"));

    let _ = std::fs::remove_file(history_path);
}

#[test]
fn task_graph_rejects_missing_spec_ref_value() {
    let output = Command::new(binary_path())
        .args(["moat", "task-graph", "--history-path", "/tmp/missing.json", "--spec-ref"])
        .output()
        .expect("run task graph");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("missing value for --spec-ref"));
    assert!(stderr.contains(USAGE));
}

#[test]
fn task_graph_rejects_flag_like_spec_ref_value() {
    let output = Command::new(binary_path())
        .args([
            "moat",
            "task-graph",
            "--history-path",
            "/tmp/missing.json",
            "--spec-ref",
            "--role",
            "planner",
        ])
        .output()
        .expect("run task graph");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("missing value for --spec-ref"));
    assert!(stderr.contains(USAGE));
}

#[test]
fn task_graph_rejects_duplicate_spec_ref_filter() {
    let first_history_path = unique_history_path("task-graph-spec-ref-duplicate-a");
    let output = Command::new(binary_path())
        .args([
            "moat",
            "task-graph",
            "--history-path",
            first_history_path.to_str().unwrap(),
            "--spec-ref",
            "docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md",
            "--spec-ref",
            "docs/superpowers/specs/other.md",
        ])
        .output()
        .expect("run task graph");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("duplicate flag: --spec-ref"));
    assert!(stderr.contains(USAGE));
    assert!(!first_history_path.exists());
}

#[test]
fn task_graph_spec_ref_filter_does_not_append_history() {
    let history_path = unique_history_path("task-graph-spec-ref-readonly");
    let seed_output = Command::new(binary_path())
        .args(["moat", "round", "--history-path"])
        .arg(&history_path)
        .output()
        .expect("run moat round");
    assert!(seed_output.status.success());

    let inspect_output = Command::new(binary_path())
        .args([
            "moat",
            "task-graph",
            "--history-path",
            history_path.to_str().unwrap(),
            "--spec-ref",
            "docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md",
        ])
        .output()
        .expect("run task graph");
    assert!(inspect_output.status.success());

    let history_output = Command::new(binary_path())
        .args(["moat", "history", "--history-path"])
        .arg(&history_path)
        .output()
        .expect("run moat history");
    assert!(history_output.status.success());
    let stdout = String::from_utf8_lossy(&history_output.stdout);
    assert!(stdout.contains("entries=1"));

    let _ = std::fs::remove_file(history_path);
}
```

- [x] **Step 2: Run tests to verify RED**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli spec_ref -- --nocapture
```

Expected: FAIL because `moat task-graph` does not yet parse `--spec-ref`; error output contains `unknown flag: --spec-ref` for the new positive tests.

- [x] **Step 3: Implement minimal parser and filter**

Update `MoatTaskGraphCommand` in `crates/mdid-cli/src/main.rs`:

```rust
struct MoatTaskGraphCommand {
    history_path: String,
    role: Option<AgentRole>,
    state: Option<MoatTaskNodeState>,
    kind: Option<MoatTaskNodeKind>,
    node_id: Option<String>,
    title_contains: Option<String>,
    spec_ref: Option<String>,
}
```

In `parse_moat_task_graph_command`, add `let mut spec_ref = None;`, add this match arm before `flag =>`:

```rust
"--spec-ref" => {
    let value = required_flag_value(args, index, "--spec-ref", true)?;
    if spec_ref.is_some() {
        return Err(duplicate_flag_error("--spec-ref"));
    }
    spec_ref = Some(value.to_string());
}
```

And include `spec_ref,` in the returned `MoatTaskGraphCommand`.

In `run_moat_task_graph`, add this filter after the title filter:

```rust
.filter(|node| {
    command
        .spec_ref
        .as_deref()
        .map(|expected_spec_ref| node.spec_ref.as_deref() == Some(expected_spec_ref))
        .unwrap_or(true)
})
```

Update both usage strings to include:

```text
moat task-graph --history-path PATH [--role planner|coder|reviewer] [--state pending|ready|in_progress|completed|blocked] [--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation] [--node-id NODE_ID] [--title-contains TEXT] [--spec-ref SPEC_REF]
```

- [x] **Step 4: Run tests to verify GREEN**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli spec_ref -- --nocapture
cargo test -p mdid-cli --test moat_cli task_graph -- --nocapture
```

Expected: PASS.

- [x] **Step 5: Update README and moat-loop design spec**

In `README.md`, update the `moat task-graph` command documentation so it lists `[--spec-ref SPEC_REF]` and states:

```markdown
`--spec-ref SPEC_REF` exact-matches the persisted optional task graph node `spec_ref` field. It is read-only, latest-round scoped, and does not append history, schedule work, launch agents, open PRs, or create cron jobs.
```

In `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`, add the same persisted-contract note near task graph inspection documentation.

- [x] **Step 6: Run formatting and broader verification**

Run:

```bash
source "$HOME/.cargo/env"
cargo fmt --check
cargo test -p mdid-cli --test moat_cli spec_ref -- --nocapture
cargo test -p mdid-cli --test moat_cli task_graph -- --nocapture
cargo test -p mdid-cli
```

Expected: PASS.

- [x] **Step 7: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs README.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-26-med-de-id-moat-task-graph-spec-ref-filter.md
git commit -m "feat: filter moat task graph by spec ref"
```
