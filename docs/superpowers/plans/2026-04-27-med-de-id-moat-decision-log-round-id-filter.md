# med-de-id Moat Decision Log Round ID Filter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a read-only `--round-id ROUND_ID` filter to `mdid-cli moat decision-log` so operators can inspect Planner/Coder/Reviewer decisions for a specific persisted moat round instead of only the latest round.

**Architecture:** Extend the existing CLI-only decision-log inspection surface. The parser stores an optional round id, and the renderer selects either the exact persisted round id or the latest round before applying existing role/text/summary/rationale/limit filters.

**Tech Stack:** Rust, Cargo, `mdid-cli`, `mdid-runtime::moat_history::LocalMoatHistoryStore`, integration tests in `crates/mdid-cli/tests/moat_cli.rs`.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `round_id: Option<String>` to `MoatDecisionLogCommand`.
  - Parse `--round-id ROUND_ID` in `parse_moat_decision_log_command` with duplicate and missing-value validation.
  - In `run_moat_decision_log`, select the exact persisted entry whose `entry.report.summary.round_id.to_string()` equals the requested id; when no entry matches, print `decision_log_entries=0` and no decisions.
  - Preserve read-only behavior: use `LocalMoatHistoryStore::open_existing`, do not append history, schedule work, launch agents, or create cron jobs.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add an integration test proving `--round-id` selects older persisted decision logs even after a later round exists.
  - Add an integration test proving an unmatched `--round-id` succeeds with zero decision rows.
- Modify: `README.md`
  - Update the CLI usage/help prose for `moat decision-log` to include `--round-id ROUND_ID` and exact-match semantics.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Update the shipped foundation bullet for `moat decision-log` to document `--round-id` read-only exact-match behavior.

### Task 1: Add `--round-id` to decision-log inspection

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`

- [ ] **Step 1: Write the failing older-round selection test**

Add this test near the existing decision-log tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn cli_filters_moat_decision_log_by_exact_round_id() {
    let history_path = unique_history_path("decision-log-round-id-filter");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let first_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to run first mdid-cli moat round with history path");
    assert!(
        first_output.status.success(),
        "expected first round success, stderr was: {}",
        String::from_utf8_lossy(&first_output.stderr)
    );

    let store_after_first = LocalMoatHistoryStore::open(&history_path).expect("history store should open");
    let first_round_id = store_after_first
        .summary()
        .latest_round_id
        .expect("first round id should be persisted");

    let second_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--review-loops",
            "0",
            "--history-path",
            history_path_arg,
        ])
        .output()
        .expect("failed to run second mdid-cli moat round with history path");
    assert!(
        second_output.status.success(),
        "expected second round success, stderr was: {}",
        String::from_utf8_lossy(&second_output.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "decision-log",
            "--history-path",
            history_path_arg,
            "--round-id",
            &first_round_id.to_string(),
        ])
        .output()
        .expect("failed to run mdid-cli moat decision-log with round id");

    assert!(
        output.status.success(),
        "expected success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "decision_log_entries=4\n",
            "decision=planner|market and competitor scan complete|identified local-first compliance workflow gap\n",
            "decision=planner|strategy selected|selected workflow-audit moat because lock-in and compliance asymmetry improved\n",
            "decision=coder|implementation task completed|implemented deterministic workflow audit spec handoff\n",
            "decision=reviewer|review passed|tests passed and moat score improved\n",
        )
    );

    cleanup_history_path(&history_path);
}
```

- [ ] **Step 2: Run the older-round test to verify RED**

Run:

```bash
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/tmp/med-de-id-moat-loop-target cargo test -p mdid-cli --test moat_cli cli_filters_moat_decision_log_by_exact_round_id -- --nocapture
```

Expected: FAIL with `unknown flag: --round-id`.

- [ ] **Step 3: Implement minimal parser and renderer support**

In `crates/mdid-cli/src/main.rs`, make these exact changes:

```rust
struct MoatDecisionLogCommand {
    history_path: String,
    round_id: Option<String>,
    role: Option<AgentRole>,
    contains: Option<String>,
    summary_contains: Option<String>,
    rationale_contains: Option<String>,
    limit: Option<usize>,
}
```

In `parse_moat_decision_log_command`, add `let mut round_id = None;`, parse this match arm after `--history-path`, and include `round_id` in the returned command:

```rust
"--round-id" => {
    let value = required_flag_value(args, index, "--round-id", false)?;
    if round_id.is_some() {
        return Err(duplicate_flag_error("--round-id"));
    }
    round_id = Some(value.clone());
}
```

Replace latest-only selection in `run_moat_decision_log` with:

```rust
let maybe_entry = match command.round_id.as_deref() {
    Some(round_id) => store
        .entries()
        .iter()
        .find(|entry| entry.report.summary.round_id.to_string() == round_id),
    None => Some(store.entries().last().ok_or_else(|| {
        "moat history is empty; run `mdid-cli moat round --history-path <path>` first".to_string()
    })?),
};
let Some(latest) = maybe_entry else {
    println!("decision_log_entries=0");
    return Ok(());
};
```

- [ ] **Step 4: Run the older-round test to verify GREEN**

Run:

```bash
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/tmp/med-de-id-moat-loop-target cargo test -p mdid-cli --test moat_cli cli_filters_moat_decision_log_by_exact_round_id -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Write the failing unmatched-round test**

Add this second test near the first one:

```rust
#[test]
fn cli_moat_decision_log_round_id_filter_reports_zero_for_missing_round() {
    let history_path = unique_history_path("decision-log-missing-round-id-filter");
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
            "decision-log",
            "--history-path",
            history_path_arg,
            "--round-id",
            "moat-round-missing",
        ])
        .output()
        .expect("failed to run mdid-cli moat decision-log with missing round id");

    assert!(
        output.status.success(),
        "expected success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "decision_log_entries=0\n"
    );

    cleanup_history_path(&history_path);
}
```

- [ ] **Step 6: Run the unmatched-round test to verify GREEN**

Run:

```bash
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/tmp/med-de-id-moat-loop-target cargo test -p mdid-cli --test moat_cli cli_moat_decision_log_round_id_filter_reports_zero_for_missing_round -- --nocapture
```

Expected: PASS.

- [ ] **Step 7: Update docs**

In `README.md`, update the `moat decision-log` usage text to include:

```text
[--round-id ROUND_ID]
```

Document that `--round-id` exact-matches a persisted `entry.report.summary.round_id`, selects that persisted round before role/text/summary/rationale/limit filters, succeeds with zero rows when no persisted round matches, and remains read-only.

In `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`, update the shipped decision-log bullet with the same semantics.

- [ ] **Step 8: Run targeted and broader validation**

Run:

```bash
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/tmp/med-de-id-moat-loop-target cargo test -p mdid-cli --test moat_cli decision_log -- --nocapture
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/tmp/med-de-id-moat-loop-target cargo test -p mdid-cli -- --nocapture
```

Expected: PASS.

- [ ] **Step 9: Clean build artifacts and commit**

Run:

```bash
rm -rf /tmp/med-de-id-moat-loop-target
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs README.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-27-med-de-id-moat-decision-log-round-id-filter.md
git commit -m "feat: filter moat decision log by round id"
```

Expected: commit succeeds on `feature/moat-loop-autonomy`.

## Self-Review

- Spec coverage: The plan covers parser state, exact persisted round selection, zero-row unmatched behavior, existing filter ordering, docs, tests, and read-only constraints.
- Placeholder scan: No TBD/TODO/fill-in placeholders are present.
- Type consistency: `round_id: Option<String>` matches the command structs and exact string comparison pattern already used by assignments and task-graph filters.
