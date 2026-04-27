# Moat History Tests Passed Filter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a typed `--tests-passed true|false` filter to `mdid-cli moat history` so operators can inspect successful or failed moat rounds without broad text search.

**Architecture:** Extend the existing history command parser with an optional boolean filter that reuses the existing `parse_bool_flag` semantics from round overrides. The read-only history renderer filters persisted entries before summary/row output, includes the new filter in filtered-summary mode, and never mutates the history store.

**Tech Stack:** Rust, Cargo workspace, `mdid-cli` integration tests, local JSON moat history store.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `tests_passed: Option<bool>` to `MoatHistoryCommand`.
  - Update `USAGE` to advertise `moat history --history-path PATH ... [--tests-passed true|false] ...`.
  - Parse `--tests-passed true|false` in `parse_moat_history_command`, reject duplicate flags, and reuse `parse_bool_flag` for exact accepted values.
  - Filter entries by `entry.report.summary.tests_passed == tests_passed` in `run_moat_history`.
  - Include `tests_passed` in the filtered-summary and row-output triggers.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Update the test-local `USAGE` string.
  - Add focused integration tests for false filtering, empty result summaries, and duplicate flag rejection.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Truth-sync the implemented CLI surface to include `--tests-passed true|false` for `moat history`.

### Task 1: Add `moat history --tests-passed` filter

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`

- [x] **Step 1: Write failing filter and duplicate tests**

Add these tests near the existing moat history filter tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn cli_filters_recent_moat_history_rounds_by_tests_passed() {
    let history_path = unique_history_path("history-tests-passed-filter");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed_success = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed successful history round");
    assert!(seed_success.status.success(), "{}", String::from_utf8_lossy(&seed_success.stderr));
    let seed_failed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--history-path",
            history_path_arg,
            "--tests-passed",
            "false",
        ])
        .output()
        .expect("failed to seed failed history round");
    assert!(seed_failed.status.success(), "{}", String::from_utf8_lossy(&seed_failed.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "history",
            "--history-path",
            history_path_arg,
            "--tests-passed",
            "false",
            "--limit",
            "5",
        ])
        .output()
        .expect("failed to inspect history by tests-passed filter");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("moat history summary\n"));
    assert!(stdout.contains("entries=1\n"));
    assert!(stdout.contains("latest_continue_decision=Stop\n"));
    assert!(stdout.contains("latest_stop_reason=tests failed\n"));
    assert!(stdout.contains("latest_moat_score_after=90\n"));
    assert!(stdout.contains("history_rounds=1\n"));
    assert!(stdout.contains("|Stop|90|tests failed\n"));
    assert!(!stdout.contains("|Continue|98|<none>\n"));

    let verify = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "history", "--history-path", history_path_arg, "--limit", "5"])
        .output()
        .expect("failed to verify history was not mutated");
    assert!(verify.status.success(), "{}", String::from_utf8_lossy(&verify.stderr));
    assert!(String::from_utf8_lossy(&verify.stdout).contains("history_rounds=2\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn cli_reports_empty_moat_history_summary_when_tests_passed_filter_matches_no_rounds() {
    let history_path = unique_history_path("history-tests-passed-empty");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed successful history round");
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "history",
            "--history-path",
            history_path_arg,
            "--tests-passed",
            "false",
            "--limit",
            "5",
        ])
        .output()
        .expect("failed to inspect history by unmatched tests-passed filter");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "moat history summary\nentries=0\nlatest_round_id=none\nlatest_decision=none\nhistory_rounds=0\n"
    );

    cleanup_history_path(&history_path);
}

#[test]
fn cli_history_rejects_duplicate_tests_passed_filter() {
    let history_path = unique_history_path("history-tests-passed-duplicate");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "history",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--tests-passed",
            "true",
            "--tests-passed",
            "false",
        ])
        .output()
        .expect("failed to run mdid-cli moat history with duplicate tests-passed filter");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("duplicate flag: --tests-passed"));
    assert!(!history_path.exists());
}
```

- [x] **Step 2: Run focused tests to verify RED**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_filters_recent_moat_history_rounds_by_tests_passed -- --exact --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_reports_empty_moat_history_summary_when_tests_passed_filter_matches_no_rounds -- --exact --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_history_rejects_duplicate_tests_passed_filter -- --exact --nocapture
```

Expected: the first two tests fail because `--tests-passed` is currently reported as an unknown moat history flag; the duplicate test also fails until duplicate parsing is implemented.

- [x] **Step 3: Implement minimal CLI support**

In `crates/mdid-cli/src/main.rs`:

```rust
struct MoatHistoryCommand {
    history_path: String,
    round_id: Option<String>,
    decision: Option<ContinueDecision>,
    contains: Option<String>,
    stop_reason_contains: Option<String>,
    min_score: Option<u32>,
    tests_passed: Option<bool>,
    limit: Option<usize>,
}
```

Inside `parse_moat_history_command`, initialize `let mut tests_passed = None;`, add this parser arm before the unknown flag arm, and return it in `MoatHistoryCommand`:

```rust
"--tests-passed" => {
    let value = required_flag_value(args, index, "--tests-passed", false)?;
    if tests_passed.is_some() {
        return Err(duplicate_flag_error("--tests-passed"));
    }
    tests_passed = Some(parse_bool_flag("--tests-passed", value)?);
    index += 2;
}
```

Inside `run_moat_history`, add this filter after the min-score filter:

```rust
.filter(|entry| {
    command
        .tests_passed
        .map(|tests_passed| entry.report.summary.tests_passed == tests_passed)
        .unwrap_or(true)
})
```

Include `|| command.tests_passed.is_some()` in the filtered-summary trigger and `command.tests_passed.is_some()` in the row-output trigger:

```rust
if command.contains.is_some()
    || command.stop_reason_contains.is_some()
    || command.min_score.is_some()
    || command.tests_passed.is_some()
{
    ...
}

if command.limit.is_some() || command.round_id.is_some() || command.tests_passed.is_some() {
    ...
}
```

Update both `USAGE` strings to include:

```text
moat history --history-path PATH [--round-id ROUND_ID] [--decision Continue|Stop|Pivot] [--contains TEXT] [--stop-reason-contains TEXT] [--min-score N] [--tests-passed true|false] [--limit N]
```

- [x] **Step 4: Run focused tests to verify GREEN**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_filters_recent_moat_history_rounds_by_tests_passed -- --exact --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_reports_empty_moat_history_summary_when_tests_passed_filter_matches_no_rounds -- --exact --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_history_rejects_duplicate_tests_passed_filter -- --exact --nocapture
```

Expected: all three tests pass.

- [x] **Step 5: Update spec implementation status**

In `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`, update the shipped CLI bullet for `mdid-cli moat history` so it includes `[--tests-passed true|false]` and state that this is read-only, applies before `--limit`, and combines conjunctively with the other history filters.

- [x] **Step 6: Run broader relevant verification**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_history -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli -- --nocapture
```

Expected: relevant CLI integration tests pass.

- [x] **Step 7: Commit slice**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-27-med-de-id-moat-history-tests-passed-filter.md
git commit -m "feat: filter moat history by tests status"
```
