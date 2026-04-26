# med-de-id Moat Assignments Depends-On Filter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Add a read-only `mdid-cli moat assignments --depends-on NODE_ID` filter so operators can inspect role assignments whose task-graph nodes depend on a specific upstream node.

**Architecture:** Extend the existing latest-round assignment inspection pipeline without changing history mutation, scheduling, or agent launch behavior. The CLI parser stores an optional exact dependency node ID, the assignments renderer looks up each assignment's task-graph node by `node_id`, and the filter keeps assignments whose persisted node `dependencies` contain the requested upstream ID before applying `--limit`.

**Tech Stack:** Rust workspace, `mdid-cli`, `mdid-runtime::moat_history::LocalMoatHistoryStore`, Cargo integration tests.

---

## File Structure

- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Adds integration tests proving `--depends-on` filters assignments through persisted task graph dependencies, combines with existing filters, rejects missing/duplicate values, and does not mutate history.
- Modify: `crates/mdid-cli/src/main.rs`
  - Adds `depends_on: Option<String>` to `MoatAssignmentsCommand`, parses `--depends-on`, includes it in usage text, and applies exact dependency filtering before `--limit`.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Truth-syncs the shipped `moat assignments` surface to include `--depends-on NODE_ID` and exact/read-only semantics.
- Modify: `docs/superpowers/plans/2026-04-27-med-de-id-moat-assignments-depends-on-filter.md`
  - Mark this plan complete after implementation and verification.

### Task 1: CLI assignments exact dependency filter

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
- Modify: `docs/superpowers/plans/2026-04-27-med-de-id-moat-assignments-depends-on-filter.md`

- [x] **Step 1: Write failing tests**

Add these tests near the existing `moat assignments` filter tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn cli_filters_moat_assignments_by_task_dependency() {
    let history_path = unique_history_path("assignments-depends-on-filter");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let round_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to run mdid-cli moat round with history path");
    assert!(
        round_output.status.success(),
        "expected round success, stderr was: {}",
        String::from_utf8_lossy(&round_output.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--depends-on",
            "implementation",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with depends-on filter");

    assert!(
        output.status.success(),
        "expected assignments success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "moat assignments\n",
            "assignment_entries=1\n",
            "assignment=reviewer|review|Review|review|<none>\n",
        )
    );

    let after_summary = LocalMoatHistoryStore::open(&history_path)
        .expect("history store should reopen")
        .summary();
    assert_eq!(
        after_summary.entry_count, 1,
        "read-only assignments dependency filter must not append history"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn cli_combines_moat_assignments_depends_on_with_role_filter() {
    let history_path = unique_history_path("assignments-depends-on-role-filter");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let round_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to run mdid-cli moat round with history path");
    assert!(
        round_output.status.success(),
        "expected round success, stderr was: {}",
        String::from_utf8_lossy(&round_output.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            history_path_arg,
            "--depends-on",
            "implementation",
            "--role",
            "coder",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with combined depends-on and role filters");

    assert!(
        output.status.success(),
        "expected assignments success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!("moat assignments\n", "assignment_entries=0\n")
    );

    cleanup_history_path(&history_path);
}

#[test]
fn cli_rejects_moat_assignments_depends_on_without_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            "history.json",
            "--depends-on",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with missing depends-on value");

    assert!(!output.status.success(), "expected command failure");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--depends-on requires a value"),
        "stderr should explain missing depends-on value, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn cli_rejects_duplicate_moat_assignments_depends_on_filter() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "assignments",
            "--history-path",
            "history.json",
            "--depends-on",
            "implementation",
            "--depends-on",
            "review",
        ])
        .output()
        .expect("failed to run mdid-cli moat assignments with duplicate depends-on filter");

    assert!(!output.status.success(), "expected command failure");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("duplicate flag: --depends-on"),
        "stderr should explain duplicate depends-on filter, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
```

- [x] **Step 2: Run tests to verify they fail**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli assignments_depends_on -- --nocapture
```

Expected: tests fail because `mdid-cli moat assignments` currently rejects `--depends-on` as an unknown flag.

- [x] **Step 3: Implement parser and filter**

In `crates/mdid-cli/src/main.rs`:

1. Change the usage text `moat assignments` segment to include `[--depends-on NODE_ID]`.
2. Add `depends_on: Option<String>` to `MoatAssignmentsCommand` after `node_id`.
3. Initialize `let mut depends_on = None;` in `parse_moat_assignments_command`.
4. Add parser arm:

```rust
"--depends-on" => {
    let value = required_flag_value(args, index, "--depends-on", false)?;
    if depends_on.is_some() {
        return Err(duplicate_flag_error("--depends-on"));
    }
    depends_on = Some(value.clone());
}
```

5. Include `depends_on` in the returned `MoatAssignmentsCommand`.
6. In `run_moat_assignments`, add a filter after the `--node-id` filter and before title/spec/contains filters:

```rust
.filter(|assignment| {
    command
        .depends_on
        .as_deref()
        .map(|expected_dependency| {
            latest
                .report
                .control_plane
                .task_graph
                .nodes
                .iter()
                .find(|node| node.node_id == assignment.node_id)
                .map(|node| node.dependencies.iter().any(|dependency| dependency == expected_dependency))
                .unwrap_or(false)
        })
        .unwrap_or(true)
})
```

The filter must run before `--limit`, combine conjunctively with `--role`, `--state`, `--kind`, `--node-id`, `--title-contains`, `--spec-ref`, and `--contains`, and must not call `append`, `run_bounded_round`, scheduler code, agent dispatch, PR automation, or create cron jobs.

- [x] **Step 4: Run targeted tests to verify they pass**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli assignments_depends_on -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_filters_moat_assignments_by_task_dependency -- --nocapture
```

Expected: all parser/error `assignments_depends_on` tests pass, and the primary positive dependency-filter behavior test passes directly.

- [x] **Step 5: Run relevant broader CLI test surface**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_assignments -- --nocapture
```

Expected: all assignment-related integration tests pass. If Cargo's name filter does not match enough tests, run `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli -- --nocapture`.

- [x] **Step 6: Update spec and plan**

Update `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md` so the shipped `moat assignments` bullet includes `--depends-on NODE_ID` and says it exact-matches persisted task-graph dependency IDs for each assignment's node, combines conjunctively with other filters, applies before `--limit`, and remains read-only.

Append this completion note to this plan:

```markdown

## Completion Notes

- Implemented `mdid-cli moat assignments --depends-on NODE_ID` as a read-only latest-round filter over persisted task-graph dependencies.
- Verified the filter combines with role filtering, rejects missing and duplicate flag values, and does not append to history.
- Targeted verification: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli assignments_depends_on -- --nocapture`.
- Broader verification: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_assignments -- --nocapture`.
```

- [x] **Step 7: Commit**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-27-med-de-id-moat-assignments-depends-on-filter.md
git commit -m "feat: filter moat assignments by task dependency"
```

Expected: one commit on `feature/moat-loop-autonomy` containing the TDD-tested CLI filter, spec update, and plan completion note.

## Completion Notes

- Implemented `mdid-cli moat assignments --depends-on NODE_ID` as a read-only latest-round filter over persisted task-graph dependencies.
- Verified the filter combines with role filtering, rejects missing and duplicate flag values, and does not append to history.
- Targeted verification: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli assignments_depends_on -- --nocapture` and `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_filters_moat_assignments_by_task_dependency -- --nocapture`.
- Broader verification: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_assignments -- --nocapture`.
