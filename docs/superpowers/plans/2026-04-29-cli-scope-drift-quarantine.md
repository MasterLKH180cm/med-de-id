# CLI Scope Drift Quarantine Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the active `mdid-cli moat controller-*` implementation from the product CLI surface and replace it with explicit de-identification-only status/usage behavior plus truthful documentation.

**Architecture:** The CLI should stay an automation surface for med-de-id workflows, not an agent/controller orchestration platform. This slice quarantines scope-drift command behavior by deleting the moat controller parser/runner and preserving only a small de-identification status/usage shell with regression tests proving controller vocabulary is rejected.

**Tech Stack:** Rust 2021, Cargo workspace, `assert_cmd`, README truth-sync.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs` — remove moat/controller command types, parser branches, JSON history logic, and controller runners; keep `status`, no-arg ready banner, and unknown-command usage.
- Modify: `crates/mdid-cli/tests/cli_smoke.rs` — strengthen smoke tests that product usage contains de-identification vocabulary and rejects `moat`, `controller-step`, `agent_id`, `claim`, and `complete_command` style inputs.
- Delete: `crates/mdid-cli/tests/moat_cli.rs` — remove tests that assert agent/controller orchestration behavior.
- Modify: `README.md` — truth-sync CLI completion and overall completion after scope-drift quarantine, and note that active moat/controller CLI behavior was removed rather than counted.

### Task 1: Quarantine CLI controller scope drift

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
- Delete: `crates/mdid-cli/tests/moat_cli.rs`

- [ ] **Step 1: Write the failing CLI regression tests**

Replace `crates/mdid-cli/tests/cli_smoke.rs` with:

```rust
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn cli_prints_ready_banner_with_no_args() {
    let mut cmd = Command::cargo_bin("mdid-cli").unwrap();

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("med-de-id CLI ready"));
}

#[test]
fn cli_prints_status_banner() {
    let mut cmd = Command::cargo_bin("mdid-cli").unwrap();

    cmd.arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("med-de-id CLI ready"));
}

#[test]
fn cli_usage_stays_deidentification_scoped() {
    let mut cmd = Command::cargo_bin("mdid-cli").unwrap();

    cmd.arg("--help")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage: mdid-cli [status]"))
        .stderr(predicate::str::contains("local de-identification automation"))
        .stderr(predicate::str::contains("moat").not())
        .stderr(predicate::str::contains("controller").not())
        .stderr(predicate::str::contains("agent").not());
}

#[test]
fn cli_rejects_scope_drift_controller_commands() {
    for args in [
        vec!["moat"],
        vec!["moat", "controller-plan", "--history-path", "history.json"],
        vec!["moat", "controller-step", "--history-path", "history.json", "--agent-id", "agent-1"],
        vec!["controller-step"],
        vec!["claim"],
        vec!["complete_command"],
    ] {
        let mut cmd = Command::cargo_bin("mdid-cli").unwrap();

        cmd.args(args)
            .assert()
            .failure()
            .stderr(predicate::str::contains("unknown command"))
            .stderr(predicate::str::contains("Usage: mdid-cli [status]"))
            .stderr(predicate::str::contains("moat").not())
            .stderr(predicate::str::contains("controller").not())
            .stderr(predicate::str::contains("agent").not());
    }
}
```

Remove `crates/mdid-cli/tests/moat_cli.rs` from the test suite.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p mdid-cli --test cli_smoke -- --nocapture`

Expected: FAIL because current help/usage still exposes `moat controller-plan`, `moat controller-step`, and agent/controller vocabulary.

- [ ] **Step 3: Write minimal implementation**

Replace `crates/mdid-cli/src/main.rs` with:

```rust
use std::process;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match parse_command(&args) {
        Ok(CliCommand::Status) => println!("med-de-id CLI ready"),
        Err(error) => exit_with_usage(&error),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CliCommand {
    Status,
}

fn parse_command(args: &[String]) -> Result<CliCommand, String> {
    match args {
        [] => Ok(CliCommand::Status),
        [status] if status == "status" => Ok(CliCommand::Status),
        _ => Err(format!("unknown command: {}", args.join(" "))),
    }
}

fn exit_with_usage(error: &str) -> ! {
    eprintln!("{error}");
    eprintln!();
    eprintln!("{}", usage());
    process::exit(2);
}

fn usage() -> &'static str {
    "Usage: mdid-cli [status]\n\nmdid-cli is the local de-identification automation surface.\nCurrent landed command:\n  status    Print a readiness banner for the local CLI surface.\n\nNon-goals: workflow orchestration, planner/coder/reviewer coordination, and controller loops are not part of the med-de-id product CLI."
}
```

Delete `crates/mdid-cli/tests/moat_cli.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p mdid-cli --test cli_smoke -- --nocapture`

Expected: PASS with 4 tests.

- [ ] **Step 5: Run broader CLI/workspace tests**

Run: `cargo test -p mdid-cli`

Expected: PASS with only `cli_smoke` as the CLI integration test suite.

Run: `cargo test --workspace --all-targets`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs crates/mdid-cli/tests/moat_cli.rs
git commit -m "fix(cli): quarantine controller scope drift"
```

### Task 2: README truth-sync after CLI quarantine

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Write the failing documentation check**

Run this command before editing README:

```bash
python3 - <<'PY'
from pathlib import Path
text = Path('README.md').read_text()
assert 'active moat/controller CLI behavior has been removed' in text
assert '| CLI | 45% |' in text
assert '| Overall | 50% |' in text
PY
```

Expected: FAIL because README still describes the old 42% CLI / 49% overall state and does not say the active moat/controller CLI behavior was removed.

- [ ] **Step 2: Update README completion table and CLI notes**

Edit `README.md` so the completion table rows read:

```markdown
| CLI | 45% | Early automation surface with local de-identification readiness, vault/decode, audit, and import/export foundations in library/runtime layers; active moat/controller CLI behavior has been removed from the product surface rather than counted as completion. |
| Browser/web | 34% | Bounded localhost tabular de-identification page plus bounded PDF review mode backed by local runtime routes, with bounded CSV/XLSX/PDF file import/export helper controls; not a broader browser governance workspace. |
| Desktop app | 35% | Bounded sensitive-workstation foundation prepares CSV, XLSX, PDF review, DICOM, bounded vault decode/audit, and portable artifact export/inspect/import request envelopes for existing localhost runtime routes, can apply bounded CSV/XLSX/PDF/DICOM file import/export helpers, submit prepared non-vault and portable helper envelopes to a localhost runtime, and render response panes with honest disclosures; deeper desktop vault browsing, decode workflow execution UX, audit investigation workflow, and portable transfer UX remain missing. |
| Overall | 50% | Core workspace, vault MVP, tabular path, bounded DICOM/PDF/runtime slices, conservative media/FCS domain models, adapter/application review foundation, bounded runtime metadata review/PDF review/DICOM/vault decode/audit/portable export/import entries, browser tabular/PDF review surface with bounded CSV/XLSX/PDF import/export helpers, desktop request-preparation/localhost-submit/response workbench foundation with bounded CSV/XLSX/PDF/DICOM file import/export helpers and bounded portable helper support, plus CLI scope-drift quarantine are landed and tested. |
```

Update the CLI bullet near the status notes to read:

```markdown
- `mdid-cli` remains an early de-identification automation surface. Active moat/controller CLI behavior has been removed from the product surface because agent workflow / controller loop / planner-coder-reviewer coordination semantics are scope drift for med-de-id; future CLI work should expose only de-identification workflows such as local tabular runs, vault/decode, audit, import/export, and verification.
```

- [ ] **Step 3: Run documentation check**

Run:

```bash
python3 - <<'PY'
from pathlib import Path
text = Path('README.md').read_text()
assert 'active moat/controller CLI behavior has been removed' in text
assert '| CLI | 45% |' in text
assert '| Overall | 50% |' in text
assert 'planner-coder-reviewer coordination semantics are scope drift' in text
PY
```

Expected: PASS.

- [ ] **Step 4: Run relevant tests after docs edit**

Run: `cargo test -p mdid-cli`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add README.md
git commit -m "docs: truth-sync CLI scope drift quarantine"
```

## Self-Review

- Spec coverage: The plan removes active controller/moat CLI behavior, preserves status readiness, adds regressions for forbidden vocabulary, deletes old orchestration tests, and truth-syncs README completion numbers.
- Placeholder scan: No TBD/TODO/implement-later placeholders are present.
- Type consistency: The only production type used by the plan is `CliCommand::Status`, consistently defined and parsed in Task 1.
