# Moat History Text Filter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `mdid-cli moat history --contains TEXT` filter so operators can inspect only history rounds whose serialized round content contains a specific text fragment.

**Architecture:** Extend the existing moat-history CLI query path in `crates/mdid-cli/src/main.rs` without changing the persisted history format. The filter should be applied after loading history entries and before rendering the existing summary, alongside existing decision and limit filters.

**Tech Stack:** Rust, Cargo, existing `mdid-cli` integration tests in `crates/mdid-cli/tests/moat_cli.rs`, local JSONL moat history store.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `contains: Option<String>` to `MoatHistoryCommand`.
  - Parse `--contains TEXT` in `parse_moat_history_command`.
  - Include `--contains TEXT` in the usage string.
  - Apply the filter in `run_moat_history` by matching against rendered/serialized round content that includes round id, selected strategy/spec/task text, and decision rationale.
  - Return a clear error for `--contains` without a value.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add integration coverage for a matching `--contains` filter.
  - Add integration coverage for no-match output.
  - Add integration coverage for missing `--contains` value.

### Task 1: Moat History `--contains` Filter

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`

- [x] **Step 1: Write the failing matching-filter test**

Add this test near the existing moat history filter tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn moat_history_filters_rounds_by_text_fragment() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let history_path = temp_dir.path().join("moat-history.jsonl");
    let history_path_arg = history_path.to_string_lossy().to_string();

    let first = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg.as_str()])
        .output()
        .expect("failed to seed first moat history round");
    assert!(first.status.success(), "stderr: {}", String::from_utf8_lossy(&first.stderr));

    let second = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--history-path",
            history_path_arg.as_str(),
            "--tests-passed",
            "false",
        ])
        .output()
        .expect("failed to seed second moat history round");
    assert!(second.status.success(), "stderr: {}", String::from_utf8_lossy(&second.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "history",
            "--history-path",
            history_path_arg.as_str(),
            "--contains",
            "tests passed",
        ])
        .output()
        .expect("failed to run mdid-cli moat history with contains filter");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("entries=1"), "stdout: {stdout}");
    assert!(stdout.contains("decision=Continue"), "stdout: {stdout}");
    assert!(!stdout.contains("decision=Stop"), "stdout: {stdout}");
}
```

- [x] **Step 2: Run the matching-filter test to verify it fails**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_history_filters_rounds_by_text_fragment -- --exact --nocapture`

Expected: FAIL because `moat history` does not recognize `--contains` yet.

- [x] **Step 3: Write the failing no-match test**

Add this test near the matching-filter test:

```rust
#[test]
fn moat_history_contains_filter_can_return_empty_summary() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let history_path = temp_dir.path().join("moat-history.jsonl");
    let history_path_arg = history_path.to_string_lossy().to_string();

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg.as_str()])
        .output()
        .expect("failed to seed moat history");
    assert!(seed.status.success(), "stderr: {}", String::from_utf8_lossy(&seed.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "history",
            "--history-path",
            history_path_arg.as_str(),
            "--contains",
            "text that is absent from every moat round",
        ])
        .output()
        .expect("failed to run mdid-cli moat history with absent contains filter");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "moat history summary\nentries=0\nlatest_round_id=none\nlatest_decision=none\n"
    );
}
```

- [x] **Step 4: Run the no-match test to verify it fails**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_history_contains_filter_can_return_empty_summary -- --exact --nocapture`

Expected: FAIL because `--contains` is not implemented.

- [x] **Step 5: Write the failing missing-value parse test**

Add this assertion to the existing invalid-argument/usage test area, or as a new test:

```rust
#[test]
fn moat_history_contains_requires_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "history", "--history-path", "history.jsonl", "--contains"])
        .output()
        .expect("failed to run mdid-cli moat history with missing contains value");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--contains requires a value"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
```

- [x] **Step 6: Run the missing-value test to verify it fails**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_history_contains_requires_value -- --exact --nocapture`

Expected: FAIL because `--contains` is not parsed yet.

- [x] **Step 7: Implement the minimal filter**

In `crates/mdid-cli/src/main.rs`:

```rust
struct MoatHistoryCommand {
    history_path: String,
    decision: Option<ContinueDecision>,
    contains: Option<String>,
    limit: Option<usize>,
}
```

Update the usage string segment for `moat history` to:

```text
moat history --history-path PATH [--decision Continue|Stop] [--contains TEXT] [--limit N]
```

Update `parse_moat_history_command` to accept `--contains TEXT` and return `Err("--contains requires a value".to_string())` when missing.

Update `run_moat_history` filtering so history entries are retained only when all provided filters match. The `--contains` match must be case-sensitive and should search the round id, selected strategy name/type/rationale, generated spec title/body/acceptance criteria, implementation task title/description/acceptance criteria, review summary, and continue-decision/rationale text.

- [x] **Step 8: Run targeted tests to verify pass**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_history_filters_rounds_by_text_fragment -- --exact --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_history_contains_filter_can_return_empty_summary -- --exact --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_history_contains_requires_value -- --exact --nocapture
```

Expected: PASS for all three tests.

- [x] **Step 9: Run relevant broader CLI tests**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_history -- --nocapture`

Expected: PASS for the moat history related integration tests.

- [x] **Step 10: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs docs/superpowers/plans/2026-04-26-med-de-id-moat-history-text-filter.md
git commit -m "feat: filter moat history by text"
```

---

## Self-Review

1. Spec coverage: This plan extends the autonomous moat-loop operator inspection surface with a conservative text filter for persisted round history. It covers parser, behavior, empty results, missing-value errors, tests, and docs plan tracking.
2. Placeholder scan: No TBD/TODO/implement-later placeholders are present.
3. Type consistency: The new field is consistently named `contains: Option<String>` and CLI flag is consistently `--contains TEXT`.
