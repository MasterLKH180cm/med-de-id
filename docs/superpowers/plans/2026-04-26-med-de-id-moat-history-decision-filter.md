# Med De-Id Moat History Decision Filter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a deterministic `mdid-cli moat history --decision Continue|Stop` filter so operators can inspect only rounds with the requested continuation decision.

**Architecture:** Extend the existing CLI-only moat history inspection path by parsing one optional decision filter into `MoatHistoryCommand`, applying it only to the limited `history_rounds` detail listing, and leaving the all-history summary unchanged. Reuse the existing `ContinueDecision` enum and `format_continue_decision` formatter to avoid duplicate string conventions.

**Tech Stack:** Rust, Cargo workspace, mdid-cli integration tests, mdid-runtime moat history store.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `decision: Option<ContinueDecision>` to `MoatHistoryCommand`.
  - Parse `--decision Continue|Stop` in `parse_moat_history_command`.
  - Filter displayed recent history rounds before `--limit` is applied.
  - Update the CLI usage string.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Update `USAGE` constant.
  - Add a failing integration test covering `moat history --decision Stop --limit 5` after one Continue and one Stop round.

### Task 1: Moat history decision filter

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`

- [x] **Step 1: Write the failing test**

Add this test near the existing moat history tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn cli_filters_recent_moat_history_rounds_by_continue_decision() {
    let history_path = unique_history_path("history-decision-filter");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let continue_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to run continuing mdid-cli moat round");
    assert!(
        continue_output.status.success(),
        "expected continuing round success, stderr was: {}",
        String::from_utf8_lossy(&continue_output.stderr)
    );

    let stop_output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--review-loops",
            "0",
            "--history-path",
            history_path_arg,
        ])
        .output()
        .expect("failed to run stopping mdid-cli moat round");
    assert!(
        stop_output.status.success(),
        "expected stopping round success, stderr was: {}",
        String::from_utf8_lossy(&stop_output.stderr)
    );

    let store = LocalMoatHistoryStore::open(&history_path).expect("history store should open");
    let latest_round_id = store
        .summary()
        .latest_round_id
        .clone()
        .expect("summary should expose latest round id");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "history",
            "--history-path",
            history_path_arg,
            "--decision",
            "Stop",
            "--limit",
            "5",
        ])
        .output()
        .expect("failed to run mdid-cli moat history with decision filter");

    assert!(
        output.status.success(),
        "expected history filter success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
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
```

Update the `USAGE` constant in the same test file so the moat history segment reads:

```rust
moat history --history-path PATH [--decision Continue|Stop] [--limit N]
```

- [x] **Step 2: Run test to verify it fails**

Run:

```bash
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/tmp/med-de-id-moat-loop-target cargo test -p mdid-cli --test moat_cli cli_filters_recent_moat_history_rounds_by_continue_decision -- --nocapture
```

Expected: FAIL because `--decision` is reported as an unknown moat history flag.

- [x] **Step 3: Write minimal implementation**

In `crates/mdid-cli/src/main.rs`:

1. Update the usage string moat history portion to:

```rust
moat history --history-path PATH [--decision Continue|Stop] [--limit N]
```

2. Change `MoatHistoryCommand` to:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
struct MoatHistoryCommand {
    history_path: String,
    decision: Option<ContinueDecision>,
    limit: Option<usize>,
}
```

3. In `parse_moat_history_command`, add `let mut decision = None;`, parse this match arm, and include it in the returned struct:

```rust
"--decision" => {
    let value = required_flag_value(args, index, "--decision", false)?;
    if decision.is_some() {
        return Err(duplicate_flag_error("--decision"));
    }
    decision = Some(parse_continue_decision_filter(value)?);
    index += 2;
}
```

Return:

```rust
Ok(MoatHistoryCommand {
    history_path: history_path
        .ok_or_else(|| "missing required flag: --history-path".to_string())?,
    decision,
    limit,
})
```

4. Add this helper near the role/state/kind parser helpers:

```rust
fn parse_continue_decision_filter(value: &str) -> Result<ContinueDecision, String> {
    match value {
        "Continue" => Ok(ContinueDecision::Continue),
        "Stop" => Ok(ContinueDecision::Stop),
        other => Err(format!("unknown moat history decision: {other}")),
    }
}
```

5. In `run_moat_history`, replace the `recent_entries` construction with:

```rust
let mut filtered_entries = entries
    .iter()
    .filter(|entry| {
        command
            .decision
            .map(|decision| entry.report.summary.continue_decision == decision)
            .unwrap_or(true)
    })
    .collect::<Vec<_>>();
if let Some(limit) = command.limit {
    let excess = filtered_entries.len().saturating_sub(limit);
    if excess > 0 {
        filtered_entries.drain(0..excess);
    }
}
println!("history_rounds={}", filtered_entries.len());
for entry in filtered_entries {
    println!(
        "round={}|{}|{}|{}",
        entry.report.summary.round_id,
        format_continue_decision(entry.report.summary.continue_decision),
        entry.report.summary.moat_score_after,
        entry
            .report
            .stop_reason
            .as_deref()
            .map(escape_assignment_output_field)
            .unwrap_or_else(|| "<none>".to_string())
    );
}
```

Keep the current behavior where detailed `history_rounds` are printed only when `--limit` is supplied.

- [x] **Step 4: Run test to verify it passes**

Run:

```bash
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/tmp/med-de-id-moat-loop-target cargo test -p mdid-cli --test moat_cli cli_filters_recent_moat_history_rounds_by_continue_decision -- --nocapture
```

Expected: PASS.

- [x] **Step 5: Run relevant regression tests**

Run:

```bash
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/tmp/med-de-id-moat-loop-target cargo test -p mdid-cli --test moat_cli moat_history -- --nocapture
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/tmp/med-de-id-moat-loop-target cargo test -p mdid-cli --test moat_cli invalid -- --nocapture
```

Expected: PASS for all selected tests.

- [ ] **Step 6: Commit**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs docs/superpowers/plans/2026-04-26-med-de-id-moat-history-decision-filter.md
git commit -m "feat: filter moat history by decision"
```
