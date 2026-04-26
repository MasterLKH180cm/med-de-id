# Moat History Score Filter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a read-only `mdid-cli moat history --min-score N` filter for bounded recent-round inspection by persisted `moat_score_after`.

**Architecture:** Extend the existing moat history command parser with an optional positive/zero integer score threshold, filter persisted history entries before limit/detail rendering, and keep summary semantics unchanged unless the score filter is explicitly supplied. This is an operator-facing inspection-only slice; it must not append history, run rounds, schedule work, launch agents, or create cron jobs.

**Tech Stack:** Rust, Cargo, `mdid-cli`, `mdid-runtime::moat_history`, existing std::process integration tests.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `min_score: Option<u32>` to `MoatHistoryCommand`.
  - Parse `--min-score N` in `parse_moat_history_command` with duplicate/missing/invalid handling.
  - Apply `entry.report.summary.moat_score_after >= min_score` conjunctively with existing `--decision` and `--contains` filters.
  - Print filtered summary when `--min-score` is supplied, matching existing `--contains` filtered-summary behavior.
  - Add `--min-score N` to the usage string.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add tests for positive filtering, zero-match filtering, parser errors, and no-append/read-only behavior.
  - Update the duplicated `USAGE` constant.
- Modify: `README.md`
  - Document `moat history --min-score N` as read-only, conjunctive, and applied before `--limit`.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Document the moat history min-score filter.
- Modify: `docs/superpowers/plans/2026-04-26-med-de-id-moat-history-score-filter.md`
  - Mark checklist items complete after implementation and verification.

### Task 1: Add moat history `--min-score` filter

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
- Modify: `docs/superpowers/plans/2026-04-26-med-de-id-moat-history-score-filter.md`

- [x] **Step 1: Write failing tests**

Add tests in `crates/mdid-cli/tests/moat_cli.rs` near the existing moat history tests:

```rust
#[test]
fn cli_filters_recent_moat_history_rounds_by_min_score() {
    let history_path = unique_history_path("history-min-score");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--review-loops", "0", "--history-path", history_path_arg])
        .output()
        .expect("failed to run stop round");
    Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to run continue round");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "history", "--history-path", history_path_arg, "--min-score", "95", "--limit", "5"])
        .output()
        .expect("failed to run mdid-cli moat history with min score");

    assert!(output.status.success(), "stderr was: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("moat history summary\nentries=1\n"));
    assert!(stdout.contains("history_rounds=1\n"));
    assert!(stdout.contains("|Continue|98|<none>\n"));
    assert!(!stdout.contains("|Stop|90|review budget exhausted\n"));

    cleanup_history_path(&history_path);
}

#[test]
fn cli_reports_empty_moat_history_summary_when_min_score_matches_no_rounds() {
    let history_path = unique_history_path("history-min-score-empty");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to run round");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "history", "--history-path", history_path_arg, "--min-score", "101", "--limit", "5"])
        .output()
        .expect("failed to run mdid-cli moat history with impossible min score");

    assert!(output.status.success(), "stderr was: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "moat history summary\n",
            "entries=0\n",
            "latest_round_id=<none>\n",
            "latest_continue_decision=<none>\n",
            "latest_stop_reason=<none>\n",
            "latest_decision_summary=<none>\n",
            "latest_implemented_specs=<none>\n",
            "latest_moat_score_after=<none>\n",
            "best_moat_score_after=<none>\n",
            "improvement_deltas=<none>\n",
            "history_rounds=0\n",
        )
    );

    cleanup_history_path(&history_path);
}

#[test]
fn cli_history_rejects_duplicate_min_score() {
    let missing_path = unique_history_path("history-duplicate-min-score");
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "history",
            "--history-path",
            missing_path.to_str().expect("history path should be utf-8"),
            "--min-score",
            "90",
            "--min-score",
            "95",
        ])
        .output()
        .expect("failed to run mdid-cli moat history with duplicate min score");

    assert!(!output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stderr), format!("duplicate flag: --min-score\n{}\n", USAGE));
    assert!(!missing_path.exists());
}

#[test]
fn cli_history_rejects_invalid_min_score() {
    let missing_path = unique_history_path("history-invalid-min-score");
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "history",
            "--history-path",
            missing_path.to_str().expect("history path should be utf-8"),
            "--min-score",
            "not-a-number",
        ])
        .output()
        .expect("failed to run mdid-cli moat history with invalid min score");

    assert!(!output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stderr), format!("invalid value for --min-score: expected non-negative integer, got not-a-number\n{}\n", USAGE));
    assert!(!missing_path.exists());
}
```

Update the `USAGE` constant to include `moat history --history-path PATH [--decision Continue|Stop] [--contains TEXT] [--min-score N] [--limit N]`.

- [x] **Step 2: Run targeted RED tests**

Run:

```bash
source "$HOME/.cargo/env"
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/tmp/med-de-id-moat-loop-target cargo test -p mdid-cli --test moat_cli min_score -- --nocapture
```

Expected: FAIL because `--min-score` is not recognized or tests are not yet implemented.

- [x] **Step 3: Implement parser and filter**

In `crates/mdid-cli/src/main.rs`:

```rust
struct MoatHistoryCommand {
    history_path: String,
    decision: Option<ContinueDecision>,
    contains: Option<String>,
    min_score: Option<u32>,
    limit: Option<usize>,
}
```

In `parse_moat_history_command`, initialize `let mut min_score = None;`, parse:

```rust
"--min-score" => {
    let value = required_flag_value(args, index, "--min-score", true)?;
    if min_score.is_some() {
        return Err(duplicate_flag_error("--min-score"));
    }
    min_score = Some(parse_min_score_value(value)?);
    index += 2;
}
```

Add:

```rust
fn parse_min_score_value(value: &str) -> Result<u32, String> {
    value.parse::<u32>().map_err(|_| {
        format!("invalid value for --min-score: expected non-negative integer, got {value}")
    })
}
```

Include `min_score` in the returned command and in any test-only command literals.

In `run_moat_history`, add a filter:

```rust
.filter(|entry| {
    command
        .min_score
        .map(|min_score| entry.report.summary.moat_score_after >= min_score)
        .unwrap_or(true)
})
```

Change the filtered summary condition to:

```rust
if command.contains.is_some() || command.min_score.is_some() {
```

Update the usage string in `usage()`.

- [x] **Step 4: Update docs/spec**

In `README.md`, add `--min-score N` to the `moat history` paragraph and state that it filters by persisted `moat_score_after >= N`, is conjunctive with `--decision` and `--contains`, applies before `--limit`, and is read-only.

In `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`, add the same operator contract to the moat history inspection bullet.

- [x] **Step 5: Run GREEN verification**

Run:

```bash
source "$HOME/.cargo/env"
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/tmp/med-de-id-moat-loop-target cargo test -p mdid-cli --test moat_cli min_score -- --nocapture
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/tmp/med-de-id-moat-loop-target cargo test -p mdid-cli --test moat_cli history -- --nocapture
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/tmp/med-de-id-moat-loop-target cargo fmt --check
```

Expected: all pass.

- [x] **Step 6: Mark this plan complete and commit**

Update this checklist from `- [ ]` to `- [x]` after the tests pass.

Commit:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs README.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-26-med-de-id-moat-history-score-filter.md
git commit -m "feat: filter moat history by score"
```
