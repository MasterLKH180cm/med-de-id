# OCR Privacy Filter Single Summary Path Safety Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Harden `mdid-cli ocr-to-privacy-filter --summary-output` so equivalent report/summary paths are rejected before stale-output cleanup, preserving existing reports and preventing summary/report overwrite confusion.

**Architecture:** Reuse the existing CLI path-equivalence guard used by adjacent Privacy Filter summary-output commands. Keep this as CLI/runtime artifact-safety hardening for the bounded PP-OCRv5 mobile OCR-to-text-only-Privacy-Filter chain; do not add Browser/Web/Desktop execution, OCR model quality claims, visual redaction, PDF rewrite/export, or workflow orchestration behavior.

**Tech Stack:** Rust `mdid-cli`, Cargo smoke tests, README truth-sync.

---

## File Structure

- Modify `crates/mdid-cli/tests/cli_smoke.rs`
  - Add a RED smoke test for lexical alias paths (`report.json` versus `./report.json`) that must fail with a PHI/path-safe fixed error and preserve stale primary bytes.
- Modify `crates/mdid-cli/src/main.rs`
  - Ensure `ocr-to-privacy-filter` rejects same/equivalent `--report-path` and `--summary-output` before deleting stale outputs, using the existing path-equivalence helper rather than string equality.
- Modify `README.md`
  - Truth-sync the new CLI/runtime artifact-safety evidence and completion fraction accounting.

### Task 1: Reject equivalent single-chain report/summary paths before cleanup

**Files:**
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
- Modify: `crates/mdid-cli/src/main.rs`

- [x] **Step 1: Write the failing lexical-alias smoke test**

Add this test near `ocr_to_privacy_filter_single_rejects_identical_report_and_summary_paths_before_cleanup` in `crates/mdid-cli/tests/cli_smoke.rs`:

```rust
#[test]
fn ocr_to_privacy_filter_single_rejects_alias_report_and_summary_paths_before_cleanup() {
    let dir = tempdir().expect("tempdir");
    let output_path = dir.path().join("same-output.json");
    let alias_path = dir.path().join(".").join("same-output.json");
    std::fs::write(&output_path, "stale output").expect("write stale output");

    Command::cargo_bin("mdid-cli")
        .expect("binary")
        .args([
            "ocr-to-privacy-filter",
            "--image-path",
            &repo_path("scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png"),
            "--ocr-runner-path",
            &repo_path("scripts/ocr_eval/run_small_ocr.py"),
            "--privacy-runner-path",
            &repo_path("scripts/privacy_filter/run_privacy_filter.py"),
            "--report-path",
            output_path.to_str().expect("output path"),
            "--summary-output",
            alias_path.to_str().expect("alias output path"),
            "--python-command",
            default_python_command(),
            "--mock",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "ocr_to_privacy_filter summary path must differ from report path",
        ))
        .stderr(predicate::str::contains(output_path.to_str().expect("output path")).not())
        .stderr(predicate::str::contains(alias_path.to_str().expect("alias output path")).not())
        .stderr(predicate::str::contains("Patient Jane Example").not())
        .stderr(predicate::str::contains("MRN-12345").not());

    assert_eq!(
        std::fs::read_to_string(&output_path).expect("stale output remains"),
        "stale output"
    );
}
```

- [x] **Step 2: Run the test to verify RED**

Run: `cargo test -p mdid-cli ocr_to_privacy_filter_single_rejects_alias_report_and_summary_paths_before_cleanup -- --nocapture`

Expected: FAIL because the alias path is not rejected before execution/cleanup.

- [x] **Step 3: Implement the minimal path-equivalence guard**

In `crates/mdid-cli/src/main.rs`, locate the parsed args for `run_ocr_to_privacy_filter_single`. Before any stale report/summary cleanup, add or correct this guard:

```rust
if let Some(summary_output_path) = args.summary_output.as_ref() {
    if paths_are_same_existing_or_lexical(&args.report_path, summary_output_path) {
        return Err("ocr_to_privacy_filter summary path must differ from report path".to_string());
    }
}
```

Use the existing `paths_are_same_existing_or_lexical` helper already used by adjacent commands. Do not introduce ad hoc string comparison. Do not delete either output path before this check.

- [x] **Step 4: Run targeted tests to verify GREEN**

Run:

```bash
cargo test -p mdid-cli ocr_to_privacy_filter_single_rejects_alias_report_and_summary_paths_before_cleanup -- --nocapture
cargo test -p mdid-cli ocr_to_privacy_filter_single_rejects_identical_report_and_summary_paths_before_cleanup -- --nocapture
```

Expected: PASS.

- [x] **Step 5: Run broader single-chain verification**

Run:

```bash
cargo test -p mdid-cli ocr_to_privacy_filter_single -- --nocapture
cargo fmt --check
git diff --check
```

Expected: all PASS, no formatting or whitespace failures.

- [x] **Step 6: Commit code/test hardening**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs
git commit -m "fix(cli): guard OCR privacy summary path aliasing"
```

### Task 2: Truth-sync README for single-chain summary path safety

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/plans/2026-05-01-ocr-privacy-filter-single-summary-path-safety.md`

- [x] **Step 1: Update README completion evidence**

Add a paragraph near the other OCR-to-Privacy-Filter CLI evidence:

```markdown
Verification evidence for the `mdid-cli ocr-to-privacy-filter --summary-output` report-path alias hardening landed on this branch: same and equivalent summary/report paths are rejected before stale output cleanup with the fixed PHI-safe error `ocr_to_privacy_filter summary path must differ from report path`, preserving any stale primary report bytes rather than deleting or overwriting them. Repository-visible verification: `cargo test -p mdid-cli ocr_to_privacy_filter_single_rejects_alias_report_and_summary_paths_before_cleanup -- --nocapture`, `cargo test -p mdid-cli ocr_to_privacy_filter_single_rejects_identical_report_and_summary_paths_before_cleanup -- --nocapture`, `cargo test -p mdid-cli ocr_to_privacy_filter_single -- --nocapture`, `cargo fmt --check`, and `git diff --check`. This is CLI/runtime artifact-safety hardening for the bounded PP-OCRv5 mobile OCR-to-text-only-Privacy-Filter chain only; it is not Browser/Web execution, not Desktop execution, not visual redaction, not image pixel redaction, not final PDF rewrite/export, and not model-quality evidence. Fraction accounting adds and completes one CLI/runtime artifact-safety requirement after the existing Privacy Filter text/corpus path-safety hardening: CLI `106/111 -> 107/112 = 95%` floor, Browser/Web remains 99%, Desktop app remains 99%, and Overall remains 97%.
```

Also update the current snapshot sentence and Overall row if they still name an older hardening item as “this round”. Preserve completion percentages unless controller-visible facts support a conservative change.

- [x] **Step 2: Run README verification**

Run:

```bash
cargo test -p mdid-cli ocr_to_privacy_filter_single -- --nocapture
git diff --check
```

Expected: PASS.

- [x] **Step 3: Commit README truth-sync**

```bash
git add README.md docs/superpowers/plans/2026-05-01-ocr-privacy-filter-single-summary-path-safety.md
git commit -m "docs: truth-sync OCR privacy summary path safety"
```

## Self-Review

- Spec coverage: The plan covers same/equivalent path rejection, stale report preservation, PHI/path-safe stderr/stdout, broad single-chain tests, README truth-sync, and completion fraction accounting.
- Placeholder scan: No TBD/TODO/implement-later placeholders remain; every code/test/doc step includes concrete content and commands.
- Type consistency: The test names, command name, helper name, and README evidence all consistently use `ocr-to-privacy-filter`, `--summary-output`, and `paths_are_same_existing_or_lexical`.
