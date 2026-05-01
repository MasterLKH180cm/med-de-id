# Privacy Filter Corpus Summary Path Safety Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prevent `mdid-cli privacy-filter-corpus --summary-output` from overwriting or aliasing the aggregate corpus report path.

**Architecture:** Add the same pre-cleanup artifact path guard already used by `privacy-filter-text` and OCR summary commands to the Privacy Filter corpus wrapper. The guard runs before stale-output cleanup so a caller mistake cannot delete existing report evidence, and the error remains fixed/PHI-safe.

**Tech Stack:** Rust `mdid-cli`, existing Python Privacy Filter corpus runner, Cargo CLI smoke tests.

---

## File Structure

- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
  - Responsibility: CLI regression coverage for Privacy Filter corpus artifact safety; add RED tests for same/equivalent report-summary paths.
- Modify: `crates/mdid-cli/src/main.rs`
  - Responsibility: CLI argument validation and execution for `privacy-filter-corpus`; add same-path rejection before artifact cleanup.
- Modify: `scripts/privacy_filter/README.md`
  - Responsibility: Local runner/wrapper docs; document that corpus `--summary-output` must differ from `--report-path`.
- Modify: `README.md`
  - Responsibility: Truth-sync current completion/evidence and record fraction arithmetic for the new completed CLI hardening requirement.

### Task 1: Reject same Privacy Filter corpus report and summary paths before cleanup

**Files:**
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `scripts/privacy_filter/README.md`
- Modify: `README.md`

- [ ] **Step 1: Write failing tests**

Add these tests near the existing `privacy_filter_corpus` CLI smoke tests in `crates/mdid-cli/tests/cli_smoke.rs`:

```rust
#[test]
fn privacy_filter_corpus_rejects_same_report_and_summary_path_before_cleanup() {
    let bin = assert_cmd::cargo::cargo_bin("mdid-cli");
    let dir = tempfile::tempdir().expect("tempdir");
    let output_path = dir.path().join("Jane-Example-MRN-12345-corpus.json");
    fs::write(&output_path, "stale Jane Example MRN-12345").expect("write stale output");

    Command::new(&bin)
        .args([
            "privacy-filter-corpus",
            "--fixture-dir",
            "scripts/privacy_filter/fixtures/corpus",
            "--runner-path",
            "scripts/privacy_filter/run_synthetic_corpus.py",
            "--report-path",
        ])
        .arg(&output_path)
        .arg("--summary-output")
        .arg(&output_path)
        .args(["--python-command", default_python_command()])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "privacy filter corpus summary path must differ from report path",
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
fn privacy_filter_corpus_rejects_alias_report_and_summary_path_before_cleanup() {
    let bin = assert_cmd::cargo::cargo_bin("mdid-cli");
    let dir = tempfile::tempdir().expect("tempdir");
    let output_path = dir.path().join("privacy-filter-corpus.json");
    let alias_path = dir.path().join(".").join("privacy-filter-corpus.json");
    fs::write(&output_path, "stale Jane Example MRN-12345").expect("write stale output");

    Command::new(&bin)
        .args([
            "privacy-filter-corpus",
            "--fixture-dir",
            "scripts/privacy_filter/fixtures/corpus",
            "--runner-path",
            "scripts/privacy_filter/run_synthetic_corpus.py",
            "--report-path",
        ])
        .arg(&output_path)
        .arg("--summary-output")
        .arg(&alias_path)
        .args(["--python-command", default_python_command()])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "privacy filter corpus summary path must differ from report path",
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
cargo test -p mdid-cli privacy_filter_corpus_rejects_same_report_and_summary_path_before_cleanup -- --nocapture
cargo test -p mdid-cli privacy_filter_corpus_rejects_alias_report_and_summary_path_before_cleanup -- --nocapture
```

Expected: both tests FAIL because `run_privacy_filter_corpus` currently removes `report_path` and `summary_output` before checking whether they are the same/equivalent path.

- [ ] **Step 3: Write minimal implementation**

In `crates/mdid-cli/src/main.rs`, at the start of `fn run_privacy_filter_corpus(args: PrivacyFilterCorpusArgs) -> Result<(), String>`, before any `fs::remove_file(...)` call, add:

```rust
    if let Some(summary_output) = &args.summary_output {
        if paths_are_same_existing_or_lexical(&args.report_path, summary_output) {
            return Err("privacy filter corpus summary path must differ from report path".to_string());
        }
    }
```

- [ ] **Step 4: Run targeted tests to verify GREEN**

Run:

```bash
cargo test -p mdid-cli privacy_filter_corpus_rejects_same_report_and_summary_path_before_cleanup -- --nocapture
cargo test -p mdid-cli privacy_filter_corpus_rejects_alias_report_and_summary_path_before_cleanup -- --nocapture
```

Expected: both PASS.

- [ ] **Step 5: Run broader relevant tests**

Run:

```bash
cargo test -p mdid-cli privacy_filter_corpus -- --nocapture
cargo test -p mdid-cli privacy_filter_text_rejects_same_report_and_summary_path_before_cleanup -- --nocapture
cargo fmt --check
git diff --check
```

Expected: all PASS with no formatting or whitespace failures.

- [ ] **Step 6: Update docs and completion truth-sync**

In `scripts/privacy_filter/README.md`, in the corpus CLI wrapper section that documents `--summary-output <summary.json>`, add:

```markdown
The corpus summary output path must differ from the primary `--report-path`, including equivalent aliases, so the PHI-safe aggregate summary cannot overwrite the validated aggregate corpus report. Same-path rejection happens before stale output cleanup and uses a fixed PHI-safe error message.
```

In `README.md`, add a new verification evidence paragraph for the completed `privacy-filter-corpus --summary-output` report-path alias hardening. Completion arithmetic: old CLI `105/110`; add and complete one necessary CLI artifact-safety requirement, new CLI `106/111 = 95%` floor. Browser/Web remains `99%`; Desktop remains `99%`; Overall remains `97%`. State explicitly that Browser/Web +5 and Desktop +5 are FAIL/not claimed because this slice is CLI/runtime Privacy Filter corpus artifact-safety hardening only.

- [ ] **Step 7: Run final verification**

Run:

```bash
cargo test -p mdid-cli privacy_filter_corpus -- --nocapture
cargo fmt --check
git diff --check
```

Expected: PASS.

- [ ] **Step 8: Commit**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs scripts/privacy_filter/README.md README.md docs/superpowers/plans/2026-05-01-privacy-filter-corpus-summary-path-safety.md
git commit -m "fix(cli): guard privacy filter corpus summary path aliasing"
```

## Self-Review

- Spec coverage: Covers same/equivalent path rejection, fixed PHI-safe error, stale artifact retention before cleanup, docs, README truth-sync, and verification.
- Placeholder scan: No TBD/TODO/fill-in placeholders remain.
- Type consistency: Uses existing `PrivacyFilterCorpusArgs`, `run_privacy_filter_corpus`, and `paths_are_same_existing_or_lexical` names consistently.
