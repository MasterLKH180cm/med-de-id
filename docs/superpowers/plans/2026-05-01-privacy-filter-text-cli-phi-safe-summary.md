# Privacy Filter Text CLI PHI-Safe Summary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Harden `mdid-cli privacy-filter-text` so its stdout summary never echoes caller-controlled report paths and remains PHI-safe while preserving the checked-in text-only Privacy Filter report contract.

**Architecture:** Keep the existing Rust CLI wrapper and Python runner contract. Add TDD smoke coverage around the real checked-in runner path and then change only the stdout summary from a raw `report_path` echo to a redacted write indicator, leaving the written report JSON verbatim and validator-compatible.

**Tech Stack:** Rust `mdid-cli`, `assert_cmd`, `serde_json`, Python fixture runner/validator.

---

## File Structure

- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
  - Add a smoke test that creates a PHI-bearing temp output directory and proves `privacy-filter-text` stdout/stderr do not contain the raw path or fixture PHI while the report is still written and validator-compatible.
- Modify: `crates/mdid-cli/src/main.rs`
  - Change `run_privacy_filter_text_inner` stdout summary to emit `"report_path": "<redacted>"` and `"report_written": true` instead of echoing the raw requested report path.
- Modify: `README.md`
  - Truth-sync the current repository status and evidence for this CLI/runtime hardening. Completion remains CLI 95%, Browser/Web 99%, Desktop app 99%, Overall 97% because this adds and completes one CLI/runtime PHI-safety requirement in the same round and does not add Browser/Desktop capability.

### Task 1: Privacy Filter text CLI stdout path redaction

**Files:**
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/cli_smoke.rs`

- [ ] **Step 1: Write the failing test**

Add this test near the existing `privacy_filter_text_*` smoke tests in `crates/mdid-cli/tests/cli_smoke.rs`:

```rust
#[test]
fn privacy_filter_text_redacts_report_path_in_stdout_summary() {
    let dir = tempdir().unwrap();
    let phi_named_dir = dir.path().join("Jane-Example-MRN-12345");
    fs::create_dir(&phi_named_dir).unwrap();
    let report_path = phi_named_dir.join("privacy-filter-report.json");
    let input_path = repo_path("scripts/privacy_filter/fixtures/sample_text_input.txt");
    let runner_path = repo_path("scripts/privacy_filter/run_privacy_filter.py");

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("privacy-filter-text")
        .arg("--input-path")
        .arg(&input_path)
        .arg("--runner-path")
        .arg(&runner_path)
        .arg("--report-path")
        .arg(&report_path)
        .arg("--python-command")
        .arg(default_python_command())
        .arg("--mock")
        .assert()
        .success()
        .get_output()
        .clone();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stdout.contains("privacy-filter-text"));
    assert!(stdout.contains("<redacted>"));
    assert!(stdout.contains("\"report_written\":true"));
    assert!(!stdout.contains(report_path.to_str().unwrap()));
    assert!(!stdout.contains("Jane-Example-MRN-12345"));
    assert!(!stderr.contains(report_path.to_str().unwrap()));
    assert!(!stderr.contains("Jane-Example-MRN-12345"));
    assert!(stderr.is_empty());
    assert!(report_path.exists());

    let report_text = fs::read_to_string(&report_path).unwrap();
    assert!(report_text.contains("fallback_synthetic_patterns"));
    assert!(!report_text.contains("Jane-Example-MRN-12345"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test -p mdid-cli privacy_filter_text_redacts_report_path_in_stdout_summary -- --nocapture
```

Expected: FAIL because stdout currently contains the raw requested `report_path` and does not include `"report_written":true`.

- [ ] **Step 3: Write minimal implementation**

In `crates/mdid-cli/src/main.rs`, change the summary in `run_privacy_filter_text_inner` from:

```rust
    let summary = json!({
        "command": "privacy-filter-text",
        "report_path": args.report_path,
        "engine": value["metadata"]["engine"],
        "network_api_called": value["metadata"]["network_api_called"],
        "detected_span_count": value["summary"]["detected_span_count"],
    });
```

to:

```rust
    let summary = json!({
        "command": "privacy-filter-text",
        "report_path": "<redacted>",
        "report_written": true,
        "engine": value["metadata"]["engine"],
        "network_api_called": value["metadata"]["network_api_called"],
        "detected_span_count": value["summary"]["detected_span_count"],
    });
```

- [ ] **Step 4: Run targeted tests to verify pass**

Run:

```bash
cargo test -p mdid-cli privacy_filter_text_redacts_report_path_in_stdout_summary -- --nocapture
cargo test -p mdid-cli privacy_filter_text_runs_repo_fixture_runner_and_validator -- --nocapture
cargo test -p mdid-cli privacy_filter_text --test cli_smoke -- --nocapture
```

Expected: all selected tests PASS.

- [ ] **Step 5: Update README truth-sync**

Add a short evidence sentence to `README.md` under the current Privacy Filter CLI evidence paragraph:

```markdown
A follow-up CLI stdout hardening slice now redacts the caller-controlled `privacy-filter-text` report path in stdout summaries and emits only `report_written: true` plus bounded engine/count metadata while preserving the validator-compatible report file. This adds and completes one CLI/runtime PHI-safety requirement in the same round; conservative floor arithmetic remains CLI 95%, Browser/Web 99%, Desktop app 99%, Overall 97%, with no Browser/Desktop capability increase.
```

- [ ] **Step 6: Run final verification**

Run:

```bash
cargo fmt --check
cargo test -p mdid-cli privacy_filter_text --test cli_smoke -- --nocapture
git diff --check
git status --short
```

Expected: format check PASS, targeted smoke tests PASS, diff check PASS, and only intended files modified.

- [ ] **Step 7: Commit**

Run:

```bash
git add crates/mdid-cli/tests/cli_smoke.rs crates/mdid-cli/src/main.rs README.md docs/superpowers/plans/2026-05-01-privacy-filter-text-cli-phi-safe-summary.md
git commit -m "fix(cli): redact privacy filter text summary paths"
```
