# med-de-id Moat Dispatch Next Title Filter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a read-only `--title-contains TEXT` routing filter to `mdid-cli moat dispatch-next` so external autonomous controllers can dispatch a ready task by persisted task title substring without mutating history during dry runs.

**Architecture:** Extend the existing bounded dispatch-next CLI command in the same narrow surface as `--node-id` and `--spec-ref`. The parser stores the optional title substring on `MoatDispatchNextCommand`, and `select_dispatch_next_node` applies it conjunctively to ready nodes before dry-run or claim mutation.

**Tech Stack:** Rust workspace, `mdid-cli`, existing JSON-backed moat history fixtures, Cargo integration tests.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `title_contains: Option<String>` to `MoatDispatchNextCommand`.
  - Parse `--title-contains TEXT` in `parse_moat_dispatch_next_command` with duplicate/missing-value handling via `required_flag_value`.
  - Apply a case-sensitive substring predicate over `node.title` in `select_dispatch_next_node`.
  - Update `USAGE` text if the command usage includes dispatch-next flags.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add an integration test proving `dispatch-next --title-contains TEXT --dry-run` selects the expected ready task and does not claim it.
  - Add an integration test proving an unmatched title substring fails with `no ready moat task matched dispatch filters`.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Truth-sync the shipped dispatch-next bullet to include `--title-contains TEXT` and its read-only, conjunctive, case-sensitive semantics.

### Task 1: Dispatch-next title routing filter

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`

- [ ] **Step 1: Write the failing dispatch-next title match test**

Append this test near the existing dispatch-next filter tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn cli_dispatch_next_filters_ready_task_by_title_substring() {
    let history_path = unique_history_path("dispatch-next-title-contains");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed dispatch-next title history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path_arg,
            "--title-contains",
            "workflow audit",
            "--dry-run",
        ])
        .output()
        .expect("failed to dry-run dispatch-next with title filter");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("dry_run=true"), "{stdout}");
    assert!(stdout.contains("claimed=false"), "{stdout}");
    assert!(stdout.contains("node_id=moat-implementation-workflow-audit"), "{stdout}");
    assert!(stdout.contains("title=Implement workflow audit trail"), "{stdout}");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_dispatch_next_filters_ready_task_by_title_substring -- --nocapture
```

Expected: FAIL with stderr containing `unknown flag: --title-contains`.

- [ ] **Step 3: Write the failing unmatched-title test**

Append this test near the same dispatch-next tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn cli_dispatch_next_title_filter_reports_no_matching_ready_task() {
    let history_path = unique_history_path("dispatch-next-title-no-match");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed dispatch-next title no-match history");
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "dispatch-next",
            "--history-path",
            history_path_arg,
            "--title-contains",
            "nonexistent routing title",
            "--dry-run",
        ])
        .output()
        .expect("failed to dry-run dispatch-next with unmatched title filter");

    assert!(!output.status.success(), "unexpected success: {}", String::from_utf8_lossy(&output.stdout));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no ready moat task matched dispatch filters"), "{stderr}");
}
```

- [ ] **Step 4: Run unmatched test to verify it fails for the current missing flag, not false-positive behavior**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_dispatch_next_title_filter_reports_no_matching_ready_task -- --nocapture
```

Expected: FAIL because stderr contains `unknown flag: --title-contains` instead of `no ready moat task matched dispatch filters`.

- [ ] **Step 5: Implement the minimal parser and selector change**

In `crates/mdid-cli/src/main.rs`, change `MoatDispatchNextCommand` to:

```rust
struct MoatDispatchNextCommand {
    history_path: String,
    round_id: Option<String>,
    role: Option<AgentRole>,
    kind: Option<MoatTaskNodeKind>,
    node_id: Option<String>,
    title_contains: Option<String>,
    spec_ref: Option<String>,
    dry_run: bool,
}
```

In `parse_moat_dispatch_next_command`, add `let mut title_contains = None;` beside `node_id` and `spec_ref`, add this match arm before `--spec-ref`:

```rust
"--title-contains" => {
    let value = required_flag_value(args, index, "--title-contains", true)?;
    if title_contains.is_some() {
        return Err(duplicate_flag_error("--title-contains"));
    }
    title_contains = Some(value.to_string());
    index += 2;
}
```

and include `title_contains,` in the returned `MoatDispatchNextCommand`.

In `select_dispatch_next_node`, add this conjunctive predicate before the `spec_ref` predicate:

```rust
&& command
    .title_contains
    .as_deref()
    .map(|title_contains| node.title.contains(title_contains))
    .unwrap_or(true)
```

If the usage string at the top of `crates/mdid-cli/tests/moat_cli.rs` or `crates/mdid-cli/src/main.rs` lists `dispatch-next` flags, add `[--title-contains TEXT]` between `[--node-id NODE_ID]` and `[--spec-ref SPEC_REF]`.

- [ ] **Step 6: Run the targeted tests to verify they pass**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_dispatch_next_filters_ready_task_by_title_substring -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_dispatch_next_title_filter_reports_no_matching_ready_task -- --nocapture
```

Expected: PASS for both tests.

- [ ] **Step 7: Update the moat-loop design spec**

In `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`, update the shipped `dispatch-next` bullet so the command signature reads:

```markdown
`mdid-cli moat dispatch-next --history-path PATH [--round-id ROUND_ID] [--role planner|coder|reviewer] [--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation] [--node-id NODE_ID] [--title-contains TEXT] [--spec-ref SPEC_REF] [--dry-run]`
```

and add this sentence to the same bullet:

```markdown
`--title-contains TEXT` performs a case-sensitive substring match over the persisted ready node title before acceptance.
```

- [ ] **Step 8: Run broader relevant verification**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_dispatch_next -- --nocapture
```

Expected: all dispatch-next-related tests pass.

Then run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --lib --bins
```

Expected: CLI crate library/binary tests pass.

- [ ] **Step 9: Commit**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-27-med-de-id-moat-dispatch-next-title-filter.md
git commit -m "feat: filter moat dispatch by title"
```

Expected: commit succeeds on `feature/moat-loop-autonomy`.
