# OCR Privacy Filter Single CLI Chain Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded `mdid-cli ocr-to-privacy-filter` single-fixture CLI chain that proves PP-OCRv5 mobile printed-text extraction output can feed the existing text-only Privacy Filter runner without raw PHI in stdout or summary artifacts.

**Architecture:** Reuse the existing `ocr-handoff` and `privacy-filter-text` contracts without introducing UI execution, visual redaction, PDF rewrite, or agent/controller semantics. The CLI will call the local OCR runner in `--mock --json` mode for a single image, validate the OCR handoff JSON, pipe `normalized_text` to the local Privacy Filter runner via `--stdin`, validate the Privacy Filter JSON, then write a bounded aggregate wrapper report and optional PHI-safe summary.

**Tech Stack:** Rust `mdid-cli`, Python repository runners under `scripts/ocr_eval` and `scripts/privacy_filter`, `serde_json`, Cargo integration tests.

---

## File Structure

- Modify `crates/mdid-cli/src/main.rs`
  - Add `ocr-to-privacy-filter` command parsing.
  - Add `OcrToPrivacyFilterArgs` with image, OCR runner, privacy runner, report, optional summary, and python command fields.
  - Add execution helper that delegates to local Python runners only, validates outputs, writes bounded wrapper report, and prints PHI-safe stdout.
- Modify `crates/mdid-cli/tests/cli_smoke.rs`
  - Add TDD smoke tests for the successful single-image chain and PHI-safe summary output.
  - Add failure test for stale report/summary cleanup when OCR input is missing.
- Modify `README.md`
  - Truth-sync the landed single-image OCR-to-Privacy-Filter CLI evidence, completion arithmetic, and non-goals.

### Task 1: Add `mdid-cli ocr-to-privacy-filter` single-image CLI chain

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/cli_smoke.rs`

- [ ] **Step 1: Write the failing success smoke test**

Add this test near the existing `ocr-to-privacy-filter-corpus` tests in `crates/mdid-cli/tests/cli_smoke.rs`:

```rust
#[test]
fn ocr_to_privacy_filter_single_runs_fixture_chain_without_phi_leaks() {
    let dir = tempdir().expect("tempdir");
    let report_path = dir.path().join("ocr-to-privacy-filter.json");
    let summary_path = dir.path().join("ocr-to-privacy-filter-summary.json");

    let assert = Command::cargo_bin("mdid-cli")
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
            report_path.to_str().expect("report path"),
            "--summary-output",
            summary_path.to_str().expect("summary path"),
            "--python-command",
            "python",
            "--mock",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    assert!(stdout.contains("ocr-to-privacy-filter"));
    assert!(stdout.contains("\"report_path\":\"...\""));
    assert!(!stdout.contains(report_path.to_str().expect("report path")));
    assert!(!stdout.contains("Patient Jane Example"));
    assert!(!stdout.contains("MRN-12345"));
    assert!(!stdout.contains("jane@example.com"));

    let report: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&report_path).expect("report file")
    )
    .expect("report json");
    assert_eq!(report["artifact"], "ocr_to_privacy_filter_single");
    assert_eq!(report["ocr_scope"], "printed_text_line_extraction_only");
    assert_eq!(report["privacy_scope"], "text_only_pii_detection");
    assert_eq!(report["network_api_called"], false);
    assert_eq!(report["ready_for_text_pii_eval"], true);
    assert!(report["privacy_filter_detected_span_count"].as_u64().unwrap_or(0) >= 3);
    assert!(report["privacy_filter_category_counts"].get("NAME").is_some());
    let report_text = report.to_string();
    assert!(!report_text.contains("Patient Jane Example"));
    assert!(!report_text.contains("MRN-12345"));
    assert!(!report_text.contains("jane@example.com"));
    assert!(!report_text.contains("normalized_text"));
    assert!(!report_text.contains("masked_text"));
    assert!(!report_text.contains("spans"));

    let summary: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&summary_path).expect("summary file")
    )
    .expect("summary json");
    assert_eq!(summary["artifact"], "ocr_to_privacy_filter_single_summary");
    assert_eq!(summary["ocr_scope"], "printed_text_line_extraction_only");
    assert_eq!(summary["privacy_scope"], "text_only_pii_detection");
    assert_eq!(summary["network_api_called"], false);
    let summary_text = summary.to_string();
    assert!(!summary_text.contains("Patient Jane Example"));
    assert!(!summary_text.contains("MRN-12345"));
    assert!(!summary_text.contains("jane@example.com"));
    assert!(!summary_text.contains("normalized_text"));
    assert!(!summary_text.contains("masked_text"));
    assert!(!summary_text.contains("spans"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p mdid-cli ocr_to_privacy_filter_single_runs_fixture_chain_without_phi_leaks -- --nocapture`

Expected: FAIL because `ocr-to-privacy-filter` is not yet a known command.

- [ ] **Step 3: Implement minimal CLI command**

In `crates/mdid-cli/src/main.rs`, add a new command variant, parser, help text entry, runner, and helpers by following the existing `ocr-to-privacy-filter-corpus` wrapper style. The command must:

```text
mdid-cli ocr-to-privacy-filter \
  --image-path <image> \
  --ocr-runner-path <run_small_ocr.py> \
  --privacy-runner-path <run_privacy_filter.py> \
  --report-path <report.json> \
  [--summary-output <summary.json>] \
  [--python-command <python>] \
  [--mock]
```

Implementation requirements:
- Remove stale report and summary files before execution.
- Execute OCR runner as: `<python> <ocr-runner-path> --json [--mock] <image-path>`.
- Parse OCR JSON and require `scope == "printed_text_line_extraction_only"`, `ready_for_text_pii_eval == true`, and string `normalized_text`.
- Execute Privacy Filter runner as: `<python> <privacy-runner-path> --stdin --mock`, piping OCR `normalized_text` to stdin.
- Parse Privacy Filter JSON and require `metadata.network_api_called == false` or top-level `network_api_called == false`, and summary category/count fields from the existing contract.
- Write wrapper report with only safe aggregate fields:

```json
{
  "artifact": "ocr_to_privacy_filter_single",
  "ocr_candidate": "PP-OCRv5_mobile_rec",
  "ocr_engine": "PP-OCRv5-mobile-bounded-spike",
  "ocr_scope": "printed_text_line_extraction_only",
  "privacy_scope": "text_only_pii_detection",
  "privacy_filter_engine": "fallback_synthetic_patterns",
  "privacy_filter_contract": "text_only_normalized_input",
  "ready_for_text_pii_eval": true,
  "network_api_called": false,
  "privacy_filter_detected_span_count": 0,
  "privacy_filter_category_counts": {},
  "non_goals": [
    "not_visual_redaction",
    "not_image_pixel_redaction",
    "not_final_pdf_rewrite_export",
    "not_browser_or_desktop_execution",
    "not_model_quality_evidence"
  ]
}
```

The count values must be populated from the Privacy Filter output summary. Do not include raw OCR text, normalized text, masked text, spans, previews, local paths, fixture filenames, image bytes, bbox values, or raw synthetic PHI.

- [ ] **Step 4: Run success test to verify it passes**

Run: `cargo test -p mdid-cli ocr_to_privacy_filter_single_runs_fixture_chain_without_phi_leaks -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Write stale cleanup failure test**

Add this test in `crates/mdid-cli/tests/cli_smoke.rs`:

```rust
#[test]
fn ocr_to_privacy_filter_single_removes_stale_outputs_on_missing_image() {
    let dir = tempdir().expect("tempdir");
    let report_path = dir.path().join("stale-report.json");
    let summary_path = dir.path().join("stale-summary.json");
    std::fs::write(&report_path, "stale report").expect("write stale report");
    std::fs::write(&summary_path, "stale summary").expect("write stale summary");

    Command::cargo_bin("mdid-cli")
        .expect("binary")
        .args([
            "ocr-to-privacy-filter",
            "--image-path",
            dir.path().join("missing.png").to_str().expect("missing image path"),
            "--ocr-runner-path",
            &repo_path("scripts/ocr_eval/run_small_ocr.py"),
            "--privacy-runner-path",
            &repo_path("scripts/privacy_filter/run_privacy_filter.py"),
            "--report-path",
            report_path.to_str().expect("report path"),
            "--summary-output",
            summary_path.to_str().expect("summary path"),
            "--python-command",
            "python",
            "--mock",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("ocr_to_privacy_filter single-image chain failed"))
        .stderr(predicate::str::contains("missing.png").not())
        .stderr(predicate::str::contains("Patient Jane Example").not());

    assert!(!report_path.exists(), "stale report should be removed on failure");
    assert!(!summary_path.exists(), "stale summary should be removed on failure");
}
```

- [ ] **Step 6: Run cleanup test to verify it passes**

Run: `cargo test -p mdid-cli ocr_to_privacy_filter_single_removes_stale_outputs_on_missing_image -- --nocapture`

Expected: PASS.

- [ ] **Step 7: Run broader CLI tests and formatting**

Run:

```bash
cargo test -p mdid-cli ocr_to_privacy_filter_single -- --nocapture
cargo test -p mdid-cli --test cli_smoke ocr_to_privacy_filter -- --nocapture
cargo fmt --check
git diff --check
```

Expected: all PASS, no diff whitespace errors.

- [ ] **Step 8: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs
git commit -m "feat(cli): add single OCR privacy filter chain"
```

### Task 2: Truth-sync README for single-image OCR-to-Privacy-Filter chain

**Files:**
- Modify: `README.md`

- [x] **Step 1: Update README completion evidence**

Add a paragraph after the existing OCR-to-Privacy-Filter corpus bridge evidence:

```markdown
Verification evidence for the single-image OCR-to-Privacy-Filter CLI chain landed on this branch: `mdid-cli ocr-to-privacy-filter --image-path scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png --ocr-runner-path scripts/ocr_eval/run_small_ocr.py --privacy-runner-path scripts/privacy_filter/run_privacy_filter.py --report-path <report.json> --summary-output <summary.json> --python-command python --mock` composes the bounded PP-OCRv5 mobile small OCR runner JSON output with the existing text-only Privacy Filter runner through stdin for one pre-cropped synthetic printed-text fixture. The wrapper validates the OCR handoff contract, pipes only normalized text into the text-only Privacy Filter runner, validates `network_api_called: false`, writes a bounded aggregate report with safe readiness and category counts, and optionally writes a PHI-safe summary artifact. Both stdout and the optional summary omit raw OCR text, normalized text, masked text, spans, previews, local paths, fixture filenames, image bytes, bbox values, and raw synthetic PHI. This is CLI/runtime bridge evidence only: it is not Browser/Web execution, not Desktop execution, not model-quality proof, not OCR page segmentation, not visual redaction, not image pixel redaction, not handwriting recognition, not final PDF rewrite/export, and not a complete OCR pipeline. Fraction accounting adds and completes one CLI/runtime single-image bridge requirement in the same round: CLI `101/106 -> 102/107 = 95%` floor, Browser/Web remains 99%, Desktop app remains 99%, and Overall remains 97%, with no Browser/Desktop +5 because this is CLI/runtime only.
```

Also update the current snapshot sentence and CLI status line to include `mdid-cli ocr-to-privacy-filter` single-image bridge evidence. Preserve the existing completion numbers unless controller-visible evidence supports a conservative arithmetic change.

- [x] **Step 2: Run README verification commands**

Run:

```bash
cargo test -p mdid-cli ocr_to_privacy_filter_single -- --nocapture
git diff --check
```

Expected: PASS.

- [x] **Step 3: Commit README truth-sync**

```bash
git add README.md docs/superpowers/plans/2026-05-01-ocr-privacy-filter-single-cli-chain.md
git commit -m "docs: truth-sync single OCR privacy filter chain"
```
