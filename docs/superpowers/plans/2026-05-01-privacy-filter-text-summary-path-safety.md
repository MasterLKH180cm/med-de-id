# Privacy Filter Text Summary Path Safety Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prevent `mdid-cli privacy-filter-text --summary-output` from overwriting or aliasing the primary report path.

**Architecture:** Add a pre-cleanup CLI argument guard that rejects same/equivalent report and summary paths before stale output cleanup or runner execution. Reuse the existing path-equivalence helper already used by OCR summary commands so Privacy Filter text-only PII detection evidence has the same artifact-safety boundary.

**Tech Stack:** Rust `mdid-cli`, existing Python Privacy Filter runner fixture, Cargo smoke tests.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Responsibility: CLI argument validation and execution for `privacy-filter-text`; add same-path rejection before artifact cleanup.
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
  - Responsibility: CLI smoke/regression coverage; add RED tests proving same/alias report-summary paths fail PHI-safely and do not delete stale evidence.
- Modify: `scripts/privacy_filter/README.md`
  - Responsibility: Local command documentation; document that `--summary-output` must differ from `--report-path`.
- Modify: `README.md`
  - Responsibility: Truth-sync completion/evidence; record the new CLI/runtime hardening requirement and fraction arithmetic while keeping Browser/Desktop capped honestly.

### Task 1: Reject same Privacy Filter report and summary paths

**Files:**
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `scripts/privacy_filter/README.md`
- Modify: `README.md`

- [ ] **Step 1: Write failing tests**

Add the following tests near the existing `privacy_filter_text` tests in `crates/mdid-cli/tests/cli_smoke.rs`:

```rust
#[test]
fn privacy_filter_text_rejects_same_report_and_summary_path_before_cleanup() {
    let bin = assert_cmd::cargo::cargo_bin("mdid-cli");
    let dir = tempfile::tempdir().expect("tempdir");
    let output_path = dir.path().join("Jane-Example-MRN-12345-output.json");
    fs::write(&output_path, "stale Jane Example MRN-12345").expect("write stale output");

    Command::new(&bin)
        .args([
            "privacy-filter-text",
            "--input-path",
            "scripts/privacy_filter/fixtures/sample_text_input.txt",
            "--runner-path",
            "scripts/privacy_filter/run_privacy_filter.py",
            "--report-path",
        ])
        .arg(&output_path)
        .arg("--summary-output")
        .arg(&output_path)
        .args(["--python-command", default_python_command(), "--mock"])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "privacy filter summary path must differ from report path",
        ))
        .stderr(predicate::str::contains("Jane Example").not())
        .stderr(predicate::str::contains("MRN-12345").not())
        .stderr(predicate::str::contains(output_path.to_string_lossy().as_ref()).not());

    assert_eq!(
        fs::read_to_string(&output_path).expect("stale output retained"),
        "stale Jane Example MRN-12345"
    );
}

#[test]
fn privacy_filter_text_rejects_alias_report_and_summary_path_before_cleanup() {
    let bin = assert_cmd::cargo::cargo_bin("mdid-cli");
    let dir = tempfile::tempdir().expect("tempdir");
    let output_path = dir.path().join("privacy-filter-report.json");
    let alias_path = dir.path().join(".").join("privacy-filter-report.json");
    fs::write(&output_path, "stale Jane Example MRN-12345").expect("write stale output");

    Command::new(&bin)
        .args([
            "privacy-filter-text",
            "--input-path",
            "scripts/privacy_filter/fixtures/sample_text_input.txt",
            "--runner-path",
            "scripts/privacy_filter/run_privacy_filter.py",
            "--report-path",
        ])
        .arg(&output_path)
        .arg("--summary-output")
        .arg(&alias_path)
        .args(["--python-command", default_python_command(), "--mock"])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "privacy filter summary path must differ from report path",
        ))
        .stderr(predicate::str::contains("Jane Example").not())
        .stderr(predicate::str::contains("MRN-12345").not())
        .stderr(predicate::str::contains(output_path.to_string_lossy().as_ref()).not())
        .stderr(predicate::str::contains(alias_path.to_string_lossy().as_ref()).not());

    assert_eq!(
        fs::read_to_string(&output_path).expect("stale output retained"),
        "stale Jane Example MRN-12345"
    );
}
```

- [ ] **Step 2: Run tests to verify RED**

Run:

```bash
cargo test -p mdid-cli privacy_filter_text_rejects_same_report_and_summary_path_before_cleanup -- --nocapture
cargo test -p mdid-cli privacy_filter_text_rejects_alias_report_and_summary_path_before_cleanup -- --nocapture
```

Expected: both tests FAIL because the command currently allows same/equivalent paths and can overwrite the stale output.

- [ ] **Step 3: Write minimal implementation**

In `crates/mdid-cli/src/main.rs`, inside `fn run_privacy_filter_text(args: PrivacyFilterTextArgs) -> Result<(), String>`, add this guard before any `fs::remove_file(...)` cleanup:

```rust
    if let Some(summary_output) = &args.summary_output {
        if paths_are_same_existing_or_lexical(&args.report_path, summary_output) {
            return Err("privacy filter summary path must differ from report path".to_string());
        }
    }
```

- [ ] **Step 4: Run targeted tests to verify GREEN**

Run:

```bash
cargo test -p mdid-cli privacy_filter_text_rejects_same_report_and_summary_path_before_cleanup -- --nocapture
cargo test -p mdid-cli privacy_filter_text_rejects_alias_report_and_summary_path_before_cleanup -- --nocapture
```

Expected: both PASS.

- [ ] **Step 5: Run broader relevant tests**

Run:

```bash
cargo test -p mdid-cli privacy_filter_text -- --nocapture
cargo test -p mdid-cli ocr_small_json_rejects_same_report_and_summary_path -- --nocapture
cargo fmt --check
git diff --check
```

Expected: all PASS with no whitespace errors.

- [ ] **Step 6: Update local docs and README truth-sync**

In `scripts/privacy_filter/README.md`, under the `--summary-output <summary.json>` paragraph, add:

```markdown
The summary output path must differ from the primary `--report-path`, including equivalent aliases, so the aggregate summary cannot overwrite the validated full text-only Privacy Filter report. Same-path rejection happens before stale output cleanup and uses a fixed PHI-safe error message.
```

In `README.md`, update the completion snapshot/evidence to add this new CLI/runtime hardening requirement. Fraction arithmetic: old CLI `104/109`; add and complete one requirement, new CLI `105/110 = 95%` floor. Browser/Web remains `99%`; Desktop remains `99%`; Overall remains `97%`. State that Browser/Web +5 and Desktop +5 are not claimed because this is CLI/runtime artifact-safety hardening only.

- [ ] **Step 7: Run final verification**

Run:

```bash
cargo test -p mdid-cli privacy_filter_text -- --nocapture
cargo fmt --check
git diff --check
```

Expected: PASS.

- [ ] **Step 8: Commit**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs scripts/privacy_filter/README.md README.md docs/superpowers/plans/2026-05-01-privacy-filter-text-summary-path-safety.md
git commit -m "fix(cli): guard privacy filter summary path aliasing"
```

## Self-Review

- Spec coverage: The plan covers same/equivalent path rejection, PHI-safe fixed error, stale artifact retention on argument rejection, docs, README truth-sync, and verification.
- Placeholder scan: No TBD/TODO/fill-in placeholders remain.
- Type consistency: Uses existing `PrivacyFilterTextArgs`, `run_privacy_filter_text`, and `paths_are_same_existing_or_lexical` names consistently.
