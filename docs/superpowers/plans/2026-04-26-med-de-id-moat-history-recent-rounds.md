# Moat History Recent Rounds Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded `mdid-cli moat history --limit N` view that prints recent persisted moat rounds for autonomous-loop handoff without dumping full task graphs.

**Architecture:** Keep this as a CLI-only projection over the existing `LocalMoatHistoryStore`; no new persistence format or runtime behavior is needed. Parse `moat history` into a small command struct with `history_path` and optional `limit`, then print the existing summary plus one stable line per recent round.

**Tech Stack:** Rust workspace, `mdid-cli`, `mdid-runtime::moat_history::LocalMoatHistoryStore`, Cargo integration tests.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Replace `CliCommand::MoatHistory(String)` with `MoatHistory(MoatHistoryCommand)`.
  - Add `MoatHistoryCommand { history_path: String, limit: Option<usize> }`.
  - Add `parse_moat_history_command()` accepting `--history-path PATH` and optional `--limit N` in any order.
  - Update `run_moat_history()` to print summary and recent `round=` lines only when `--limit` is present.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Update the `USAGE` constant to include `[--limit N]` for `moat history`.
  - Add focused CLI tests for recent-round limiting and invalid limits.

### Task 1: Add `moat history --limit N` recent-round output

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/moat_cli.rs`

- [x] **Step 1: Write the failing test for bounded recent history**

Add this test after `cli_reports_history_summary_for_two_persisted_rounds` in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn cli_reports_limited_recent_moat_history_rounds() {
    let history_path = unique_history_path("history-limit");
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

    let store = LocalMoatHistoryStore::open(&history_path).expect("history store should open");
    let summary = store.summary();
    let latest_round_id = summary
        .latest_round_id
        .clone()
        .expect("summary should expose latest round id");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "history",
            "--history-path",
            history_path_arg,
            "--limit",
            "1",
        ])
        .output()
        .expect("failed to run mdid-cli moat history with limit");

    assert!(
        output.status.success(),
        "expected success, stderr was: {}",
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

- [x] **Step 2: Run the failing test**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_reports_limited_recent_moat_history_rounds -- --nocapture
```

Expected: FAIL because `moat history` rejects or ignores `--limit`.

- [x] **Step 3: Write the failing test for invalid limits**

Add this test near other CLI parser/error tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn cli_rejects_invalid_moat_history_limit() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "history",
            "--history-path",
            "ignored-history.jsonl",
            "--limit",
            "0",
        ])
        .output()
        .expect("failed to run mdid-cli moat history with invalid limit");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("limit must be greater than zero\n{USAGE}\n")
    );
}
```

- [x] **Step 4: Run the invalid-limit failing test**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_rejects_invalid_moat_history_limit -- --nocapture
```

Expected: FAIL because `--limit` is not yet parsed for `moat history`.

- [x] **Step 5: Implement minimal parsing and output**

In `crates/mdid-cli/src/main.rs`, make these concrete changes:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
struct MoatHistoryCommand {
    history_path: String,
    limit: Option<usize>,
}
```

Change the enum variant to:

```rust
MoatHistory(MoatHistoryCommand),
```

Change the parse match arm to:

```rust
[moat, history, rest @ ..] if moat == "moat" && history == "history" => {
    Ok(CliCommand::MoatHistory(parse_moat_history_command(rest)?))
}
```

Add a parser that accepts `--history-path PATH` and `--limit N`, rejects duplicates, rejects unknown flags, and uses the existing `parse_limit_value()` helper:

```rust
fn parse_moat_history_command(args: &[String]) -> Result<MoatHistoryCommand, String> {
    let mut history_path = None;
    let mut limit = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--history-path" => {
                let value = required_history_path_value(args, index)?.clone();
                if history_path.is_some() {
                    return Err(duplicate_flag_error("--history-path"));
                }
                history_path = Some(value);
                index += 2;
            }
            "--limit" => {
                let value = required_flag_value(args, index, "--limit")?;
                if limit.is_some() {
                    return Err(duplicate_flag_error("--limit"));
                }
                limit = Some(parse_limit_value(value)?);
                index += 2;
            }
            flag => return Err(format!("unknown moat history flag: {flag}")),
        }
    }

    Ok(MoatHistoryCommand {
        history_path: history_path.ok_or_else(|| "missing required flag: --history-path".to_string())?,
        limit,
    })
}
```

Change the main dispatch to call `run_moat_history(&command)`.

Change `run_moat_history` to:

```rust
fn run_moat_history(command: &MoatHistoryCommand) -> Result<(), String> {
    let store = LocalMoatHistoryStore::open_existing(&command.history_path)
        .map_err(|error| format!("failed to open moat history store: {error}"))?;
    let entries = store.entries();
    print_history_summary(&store.summary());

    if let Some(limit) = command.limit {
        let recent_entries = entries
            .iter()
            .rev()
            .take(limit)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>();
        println!("history_rounds={}", recent_entries.len());
        for entry in recent_entries {
            println!(
                "round={}|{}|{}|{}",
                entry.report.round_id,
                format_continue_decision(entry.report.continue_decision),
                entry.report.evaluation.moat_score_after,
                entry.report
                    .stop_reason
                    .as_deref()
                    .map(escape_assignment_output_field)
                    .unwrap_or_else(|| "<none>".to_string())
            );
        }
    }

    Ok(())
}
```

Update `USAGE` in `crates/mdid-cli/tests/moat_cli.rs` and `usage()` in `crates/mdid-cli/src/main.rs` so the `moat history` clause is exactly:

```text
moat history --history-path PATH [--limit N]
```

- [x] **Step 6: Run targeted tests**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli cli_reports_limited_recent_moat_history_rounds cli_rejects_invalid_moat_history_limit -- --nocapture
```

Expected: PASS.

- [x] **Step 7: Run broader relevant CLI tests**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli -- --nocapture
```

Expected: PASS.

- [x] **Step 8: Commit**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs docs/superpowers/plans/2026-04-26-med-de-id-moat-history-recent-rounds.md
git commit -m "feat: show limited moat history rounds"
```

Expected: commit succeeds on `feature/moat-loop-autonomy`.

---

## Self-Review

- Spec coverage: The plan adds a bounded recent-round handoff view for the autonomous moat loop, updates usage, validates invalid limits, and preserves existing summary output.
- Placeholder scan: No placeholder instructions remain.
- Type consistency: `MoatHistoryCommand`, `parse_moat_history_command`, and `run_moat_history(&MoatHistoryCommand)` are consistently named.
