# Moat Decision Log Inspection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a read-only CLI surface that inspects the latest persisted moat round decision log without running or appending a new round.

**Architecture:** Reuse the persisted history store and latest control-plane snapshot already produced by `LocalMoatHistoryStore`. The CLI adds `mdid-cli moat decision-log --history-path PATH` and renders a stable, deterministic line per latest decision entry so operators can audit Planner/Coder/Reviewer handoffs without launching agents.

**Tech Stack:** Rust workspace, `mdid-cli`, `mdid-runtime`, JSON-backed local history, Cargo integration tests.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `MoatDecisionLog(String)` CLI command, parser arm, runner, and deterministic decision-log rendering helper.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add TDD coverage for required `--history-path`, latest decision-log output, and missing history failure.
- Modify: `README.md`
  - Document that `moat decision-log` is read-only and does not append or run rounds.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Truth-sync shipped status to include read-only decision-log inspection.

---

### Task 1: CLI decision-log inspection

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`

- [ ] **Step 1: Write failing CLI tests**

Append these tests to `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn moat_decision_log_requires_history_path() {
    let assert = Command::cargo_bin("mdid-cli")
        .expect("binary exists")
        .args(["moat", "decision-log"])
        .assert();

    assert
        .failure()
        .stderr(predicate::str::contains("missing required flag: --history-path"));
}

#[test]
fn moat_decision_log_prints_latest_persisted_decision() {
    let temp = tempfile::tempdir().expect("tempdir");
    let history_path = temp.path().join("moat-history.json");

    Command::cargo_bin("mdid-cli")
        .expect("binary exists")
        .args([
            "moat",
            "round",
            "--history-path",
            history_path.to_str().expect("utf8 path"),
        ])
        .assert()
        .success();

    let assert = Command::cargo_bin("mdid-cli")
        .expect("binary exists")
        .args([
            "moat",
            "decision-log",
            "--history-path",
            history_path.to_str().expect("utf8 path"),
        ])
        .assert();

    assert
        .success()
        .stdout(predicate::str::contains("decision_log_entries=1"))
        .stdout(predicate::str::contains(
            "decision=reviewer|review approved bounded moat round|review approved bounded moat round after evaluation cleared the improvement threshold",
        ));
}

#[test]
fn moat_decision_log_fails_for_missing_history_file() {
    let temp = tempfile::tempdir().expect("tempdir");
    let history_path = temp.path().join("missing-history.json");

    let assert = Command::cargo_bin("mdid-cli")
        .expect("binary exists")
        .args([
            "moat",
            "decision-log",
            "--history-path",
            history_path.to_str().expect("utf8 path"),
        ])
        .assert();

    assert
        .failure()
        .stderr(predicate::str::contains("failed to open moat history"));
}
```

- [ ] **Step 2: Run RED tests**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli decision_log -- --nocapture
```

Expected: FAIL because `moat decision-log` is not recognized.

- [ ] **Step 3: Implement minimal CLI command**

In `crates/mdid-cli/src/main.rs`:

1. Extend `CliCommand`:

```rust
MoatDecisionLog(String),
```

2. Add a `main` match arm:

```rust
Ok(CliCommand::MoatDecisionLog(history_path)) => {
    if let Err(error) = run_moat_decision_log(&history_path) {
        exit_with_error(error);
    }
}
```

3. Add a parser arm after `moat history`:

```rust
[moat, decision_log, rest @ ..] if moat == "moat" && decision_log == "decision-log" => {
    Ok(CliCommand::MoatDecisionLog(parse_required_history_path(rest)?))
}
```

4. Add the runner and renderer near other moat runners:

```rust
fn run_moat_decision_log(history_path: &str) -> Result<(), String> {
    let store = LocalMoatHistoryStore::open_existing(history_path)
        .map_err(|error| format!("failed to open moat history store: {error}"))?;
    let latest = store.entries().last().ok_or_else(|| {
        "moat history is empty; run `mdid-cli moat round --history-path <path>` first".to_string()
    })?;
    let decisions = &latest.report.control_plane.memory.decisions;

    println!("decision_log_entries={}", decisions.len());
    for decision in decisions {
        println!(
            "decision={}|{}|{}",
            format_agent_role(decision.author_role),
            decision.summary,
            decision.rationale
        );
    }

    Ok(())
}
```

- [ ] **Step 4: Run GREEN tests**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli decision_log -- --nocapture
cargo test -p mdid-cli --test moat_cli
```

Expected: PASS.

- [ ] **Step 5: Update docs and spec**

Update `README.md` moat CLI documentation with:

```markdown
- `cargo run -p mdid-cli -- moat decision-log --history-path ./moat-history.json` inspects the latest persisted Planner/Coder/Reviewer decision log without running or appending a round.
```

Update the shipped foundation slice in `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md` to mention read-only `moat decision-log --history-path PATH` inspection.

- [ ] **Step 6: Run final verification**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli
cargo test -p mdid-runtime --test moat_runtime
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs README.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-26-med-de-id-moat-decision-log-inspection.md
git commit -m "feat: inspect moat decision logs"
```
