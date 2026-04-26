# med-de-id Moat History Round ID Filter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a read-only `mdid-cli moat history --round-id ROUND_ID` filter so operators can inspect one persisted moat-loop round by exact ID.

**Architecture:** Extend the existing CLI-only inspection pipeline for `moat history` without changing runtime scheduling or history mutation behavior. The parser stores an optional exact round ID string, the history renderer applies it conjunctively with existing filters before `--limit`, and the spec documents that it is read-only and exact-match.

**Tech Stack:** Rust workspace, `mdid-cli`, `mdid-runtime::moat_history::LocalMoatHistoryStore`, Cargo integration tests.

---

## File Structure

- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Adds integration tests proving `--round-id` filters persisted history by exact latest/older round IDs, combines with existing filters, and returns zero rows without mutation when there is no match.
- Modify: `crates/mdid-cli/src/main.rs`
  - Adds `round_id: Option<String>` to `MoatHistoryCommand`, parses `--round-id`, includes it in usage text, and applies exact filtering to history rows before `--limit`.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Truth-syncs the shipped `moat history` surface to include `--round-id ROUND_ID` and exact-match/read-only semantics.
- Modify: `docs/superpowers/plans/2026-04-27-med-de-id-moat-history-round-id-filter.md`
  - Mark this plan complete after implementation and verification.

### Task 1: CLI history exact round-id filter

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
- Modify: `docs/superpowers/plans/2026-04-27-med-de-id-moat-history-round-id-filter.md`

- [x] **Step 1: Write the failing tests**

Add these tests near the existing `moat history` filter tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn cli_filters_recent_moat_history_rounds_by_exact_round_id() {
    let history_path = unique_history_path("history-round-id-filter");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let first_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to run first mdid-cli moat round with history path");
    assert!(first_output.status.success(), "expected first round success, stderr was: {}", String::from_utf8_lossy(&first_output.stderr));

    let second_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--review-loops", "0", "--history-path", history_path_arg])
        .output()
        .expect("failed to run second mdid-cli moat round with history path");
    assert!(second_output.status.success(), "expected second round success, stderr was: {}", String::from_utf8_lossy(&second_output.stderr));

    let store = LocalMoatHistoryStore::open(&history_path).expect("history store should open");
    let summary = store.summary();
    let latest_round_id = summary.latest_round_id.clone().expect("summary should expose latest round id");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "history", "--history-path", history_path_arg, "--round-id", &latest_round_id])
        .output()
        .expect("failed to run mdid-cli moat history with round-id filter");

    assert!(output.status.success(), "expected success, stderr was: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "moat history summary\n",
            "entries=2\n",
            "latest_round_id={latest_round_id}\n",
            "latest_continue_decision=Stop\n",
            "latest_stop_reason=review budget exhausted\n",
            "latest_decision_summary=implementation stopped before review\n",
            "latest_implemented_specs=moat-spec/workflow-audit\n",
            "latest_moat_score_after=90\n",
            "best_moat_score_after=98\n",
            "improvement_deltas=8,0\n",
            "history_rounds=1\n",
            "round={latest_round_id}|Stop|90|review budget exhausted\n",
        )
        .replace("{latest_round_id}", &latest_round_id)
    );

    cleanup_history_path(&history_path);
}

#[test]
fn cli_reports_zero_recent_moat_history_rounds_for_unknown_round_id() {
    let history_path = unique_history_path("history-round-id-no-match");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let round_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to run mdid-cli moat round with history path");
    assert!(round_output.status.success(), "expected round success, stderr was: {}", String::from_utf8_lossy(&round_output.stderr));

    let store = LocalMoatHistoryStore::open(&history_path).expect("history store should open");
    let summary = store.summary();
    let latest_round_id = summary.latest_round_id.clone().expect("summary should expose latest round id");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "history", "--history-path", history_path_arg, "--round-id", "missing-round"])
        .output()
        .expect("failed to run mdid-cli moat history with missing round-id filter");

    assert!(output.status.success(), "expected success, stderr was: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "moat history summary\n",
            "entries=1\n",
            "latest_round_id={latest_round_id}\n",
            "latest_continue_decision=Continue\n",
            "latest_stop_reason=<none>\n",
            "latest_decision_summary=all budgets cleared\n",
            "latest_implemented_specs=moat-spec/workflow-audit\n",
            "latest_moat_score_after=98\n",
            "best_moat_score_after=98\n",
            "improvement_deltas=8\n",
            "history_rounds=0\n",
        )
        .replace("{latest_round_id}", &latest_round_id)
    );

    let after_summary = LocalMoatHistoryStore::open(&history_path).expect("history store should reopen").summary();
    assert_eq!(after_summary.entry_count, 1, "read-only filter must not append or mutate history");

    cleanup_history_path(&history_path);
}
```

- [x] **Step 2: Run tests to verify they fail**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_filters_recent_moat_history_rounds_by_exact_round_id -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_reports_zero_recent_moat_history_rounds_for_unknown_round_id -- --nocapture
```

Expected: both fail because `--round-id` is currently an unknown flag in `moat history` usage parsing.

- [x] **Step 3: Implement parser and filter**

In `crates/mdid-cli/src/main.rs`:

1. Change the usage text `moat history` segment to include `[--round-id ROUND_ID]`.
2. Add `round_id: Option<String>` to `MoatHistoryCommand`.
3. Initialize `let mut round_id = None;` in `parse_moat_history_command`.
4. Add parser arm:

```rust
"--round-id" => {
    let value = required_flag_value(args, index, "--round-id", false)?;
    if round_id.is_some() {
        return Err(duplicate_flag_error("--round-id"));
    }
    round_id = Some(value.clone());
    index += 2;
}
```

5. Include `round_id` in the returned `MoatHistoryCommand`.
6. In the history row filtering function, require exact persisted ID equality:

```rust
if let Some(round_id) = command.round_id.as_deref() {
    rows.retain(|entry| entry.report.round_id == round_id);
}
```

The filter must run before `--limit`, combine conjunctively with `--decision`, `--contains`, `--stop-reason-contains`, and `--min-score`, and must not call `append`, `run_bounded_round`, scheduler code, agent dispatch, or file creation beyond opening the existing history path.

- [x] **Step 4: Run targeted tests to verify they pass**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_filters_recent_moat_history_rounds_by_exact_round_id -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_reports_zero_recent_moat_history_rounds_for_unknown_round_id -- --nocapture
```

Expected: both pass.

- [x] **Step 5: Run relevant broader CLI test surface**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_history -- --nocapture
```

Expected: all `moat_history` integration tests pass. If Cargo's name filter does not match enough tests, run `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli -- --nocapture`.

- [x] **Step 6: Update spec and plan**

Update line 46 of `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md` so the shipped `moat history` bullet includes `--round-id ROUND_ID` and says it exact-matches persisted `entry.report.round_id`, combines conjunctively with other filters, applies before `--limit`, and remains read-only.

Append this completion note to this plan:

```markdown

---

## Completion Notes

- Implemented `mdid-cli moat history --round-id ROUND_ID` as an exact read-only persisted round filter.
- Verified targeted round-id tests and the relevant broader `mdid-cli` moat CLI integration surface with `CARGO_INCREMENTAL=0`.
```

---

## Completion Notes

- Implemented `mdid-cli moat history --round-id ROUND_ID` as an exact read-only persisted round filter.
- Verified targeted round-id tests and the full `mdid-cli` moat CLI integration surface with `CARGO_INCREMENTAL=0`.
- During implementation, the failing broader run exposed stale expected `USAGE` text in integration tests; updated the shared `USAGE` constant and re-ran the full surface successfully.

- [x] **Step 7: Commit**

Run:

```bash
git add crates/mdid-cli/tests/moat_cli.rs crates/mdid-cli/src/main.rs docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-27-med-de-id-moat-history-round-id-filter.md
git commit -m "feat: filter moat history by round id"
```

Expected: commit succeeds on `feature/moat-loop-autonomy`.
