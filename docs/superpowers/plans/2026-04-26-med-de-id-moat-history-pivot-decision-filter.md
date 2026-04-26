# Moat History Pivot Decision Filter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the read-only `mdid-cli moat history --decision` filter so operators can inspect persisted `Pivot` rounds in addition to `Continue` and `Stop`.

**Architecture:** This is a narrow CLI parser and documentation slice over existing persisted history inspection. The domain enum and formatter already support `ContinueDecision::Pivot`; the CLI decision-filter parser, usage string, docs, and tests must expose the existing persisted value without mutating history or changing summary semantics.

**Tech Stack:** Rust workspace, `mdid-cli` integration tests, `mdid-domain::ContinueDecision`, `mdid-runtime::moat_history::LocalMoatHistoryStore`, Markdown docs.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Update the main usage string to advertise `--decision Continue|Stop|Pivot`.
  - Update `parse_continue_decision_filter` to accept `Pivot` and map it to `ContinueDecision::Pivot`.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Update the duplicated `USAGE` constant to advertise `--decision Continue|Stop|Pivot`.
  - Add a test proving `--decision Pivot` is accepted by the parser before history is opened and therefore reports the missing history file instead of `unknown moat history decision: Pivot`.
  - Keep existing unknown decision tests unchanged for unsupported values such as `Pause`.
- Modify: `README.md`
  - Update the moat history command synopsis and text to list `Pivot` as an accepted decision filter.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Update the moat history implementation status/operator contract to list `Pivot` as an accepted decision filter.

## Task 1: Accept Pivot in the moat history decision filter

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
- Test: `crates/mdid-cli/tests/moat_cli.rs`

- [x] **Step 1: Write the failing parser test**

Add this test near the existing moat history decision parser tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn cli_history_accepts_pivot_decision_filter_before_opening_history() {
    let history_path = unique_history_path("pivot-history-decision");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "history",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--decision",
            "Pivot",
        ])
        .output()
        .expect("failed to run mdid-cli moat history with pivot decision");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!(
            "failed to open moat history store: moat history file does not exist: {}\n",
            history_path.display()
        )
    );
    assert!(!history_path.exists());
}
```

- [x] **Step 2: Run the targeted RED test**

Run:

```bash
source "$HOME/.cargo/env"
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/tmp/med-de-id-moat-loop-target cargo test -p mdid-cli --test moat_cli cli_history_accepts_pivot_decision_filter_before_opening_history -- --nocapture
```

Expected: FAIL because stderr is currently `unknown moat history decision: Pivot`.

- [x] **Step 3: Implement the minimal parser and usage change**

In `crates/mdid-cli/src/main.rs`, change both the usage string and parser.

Replace the moat history usage fragment:

```rust
moat history --history-path PATH [--decision Continue|Stop]
```

with:

```rust
moat history --history-path PATH [--decision Continue|Stop|Pivot]
```

Update `parse_continue_decision_filter` to:

```rust
fn parse_continue_decision_filter(value: &str) -> Result<ContinueDecision, String> {
    match value {
        "Continue" => Ok(ContinueDecision::Continue),
        "Stop" => Ok(ContinueDecision::Stop),
        "Pivot" => Ok(ContinueDecision::Pivot),
        other => Err(format!("unknown moat history decision: {other}")),
    }
}
```

In `crates/mdid-cli/tests/moat_cli.rs`, update the duplicated `USAGE` constant fragment from:

```rust
moat history --history-path PATH [--decision Continue|Stop]
```

to:

```rust
moat history --history-path PATH [--decision Continue|Stop|Pivot]
```

- [x] **Step 4: Run targeted GREEN tests**

Run:

```bash
source "$HOME/.cargo/env"
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/tmp/med-de-id-moat-loop-target cargo test -p mdid-cli --test moat_cli cli_history_accepts_pivot_decision_filter_before_opening_history -- --nocapture
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/tmp/med-de-id-moat-loop-target cargo test -p mdid-cli --test moat_cli cli_history_rejects_unknown_decision_value -- --nocapture
```

Expected: both tests PASS. The Pivot missing-history runtime error prints only the open failure and does not append usage; parser errors such as unknown decisions still append usage through the outer command parsing path.

- [x] **Step 5: Update operator docs**

In `README.md`, update the `moat history` synopsis so it includes:

```text
mdid-cli moat history --history-path PATH [--decision Continue|Stop|Pivot] [--contains TEXT] [--stop-reason-contains TEXT] [--min-score N] [--limit N]
```

Also update nearby prose to say:

```text
`--decision Continue|Stop|Pivot` filters detailed history rows by persisted continuation decision.
```

In `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`, update the moat history implementation-status/operator-contract text so every `--decision Continue|Stop` mention becomes:

```text
--decision Continue|Stop|Pivot
```

- [x] **Step 6: Run focused and broader verification**

Run:

```bash
source "$HOME/.cargo/env"
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/tmp/med-de-id-moat-loop-target cargo fmt --check
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/tmp/med-de-id-moat-loop-target cargo test -p mdid-cli --test moat_cli cli_history_accepts_pivot_decision_filter_before_opening_history -- --nocapture
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/tmp/med-de-id-moat-loop-target cargo test -p mdid-cli --test moat_cli cli_history_rejects_unknown_decision_value -- --nocapture
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/tmp/med-de-id-moat-loop-target cargo test -p mdid-cli --test moat_cli cli_filters_recent_moat_history_rounds_by_continue_decision -- --nocapture
```

Expected: all commands PASS.

- [x] **Step 7: Commit**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs README.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-26-med-de-id-moat-history-pivot-decision-filter.md
git commit -m "feat: filter moat history by pivot decision"
```

## Self-Review

- Spec coverage: The plan exposes the existing persisted `Pivot` continuation decision through the read-only history filter, updates usage/docs/spec, preserves unknown-value rejection, and keeps missing-history behavior read-only without appending usage to runtime open failures.
- Placeholder scan: No placeholders, TBDs, or unspecified code steps remain.
- Type consistency: The plan consistently uses `ContinueDecision::Pivot`, `parse_continue_decision_filter`, `--decision Pivot`, and the `Continue|Stop|Pivot` usage contract.
