# OCR Small JSON Summary Output Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an optional PHI-safe aggregate summary artifact to `mdid-cli ocr-small-json` so the PP-OCRv5 mobile synthetic small-runner JSON evidence can be handed off without exposing raw OCR text.

**Architecture:** Extend the existing `ocr-small-json` CLI wrapper with `--summary-output <summary.json>`. The primary report remains the current validated OCR handoff JSON containing OCR text for downstream Privacy Filter evaluation; the optional summary is a second aggregate-only artifact derived only after primary report validation and containing allowlisted metadata/count-free readiness fields and non-goals.

**Tech Stack:** Rust `mdid-cli`, existing Python OCR small runner fixtures, `assert_cmd` smoke tests, README truth-sync.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs` — add parser/storage for `--summary-output`, stale summary cleanup, summary rendering, strict summary contract, usage text.
- Modify: `crates/mdid-cli/tests/cli_smoke.rs` — add TDD smoke coverage for PHI-safe summary success, failure cleanup, and same-path rejection.
- Modify: `scripts/ocr_eval/README.md` — document optional summary output as aggregate-only PP-OCRv5 mobile printed-text extraction readiness evidence.
- Modify: `README.md` — truth-sync completion snapshot/evidence without claiming Browser/Desktop or visual-redaction/PDF rewrite progress.

### Task 1: Add `ocr-small-json --summary-output` CLI wrapper support

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`

- [ ] **Step 1: Write failing summary success smoke test**

Append a test to `crates/mdid-cli/tests/cli_smoke.rs` near the existing `ocr-small-json` tests:

```rust
#[test]
fn ocr_small_json_writes_phi_safe_summary_output() {
    let dir = tempdir().unwrap();
    let report_path = dir.path().join("ocr-small-json-report.json");
    let summary_path = dir.path().join("ocr-small-json-summary.json");

    let assert = Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("ocr-small-json")
        .arg("--image-path")
        .arg("scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png")
        .arg("--ocr-runner-path")
        .arg("scripts/ocr_eval/run_small_ocr.py")
        .arg("--report-path")
        .arg(&report_path)
        .arg("--summary-output")
        .arg(&summary_path)
        .arg("--python-command")
        .arg(default_python_command())
        .arg("--mock")
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("ocr-small-json"));
    assert!(stdout.contains("\"summary_written\":true"));
    assert!(!stdout.contains(summary_path.to_string_lossy().as_ref()));
    for sentinel in ["Jane Example", "MRN-12345", "jane@example.com", "555-123-4567"] {
        assert!(!stdout.contains(sentinel));
    }

    let report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report_path).unwrap()).unwrap();
    assert_eq!(report["candidate"], "PP-OCRv5_mobile_rec");
    assert!(report["normalized_text"].as_str().unwrap().contains("Jane Example"));

    let summary_text = std::fs::read_to_string(&summary_path).unwrap();
    for sentinel in ["Jane Example", "MRN-12345", "jane@example.com", "555-123-4567"] {
        assert!(!summary_text.contains(sentinel));
    }
    let summary: serde_json::Value = serde_json::from_str(&summary_text).unwrap();
    assert_eq!(summary["artifact"], "ocr_small_json_summary");
    assert_eq!(summary["candidate"], "PP-OCRv5_mobile_rec");
    assert_eq!(summary["engine"], "PP-OCRv5-mobile-bounded-spike");
    assert_eq!(summary["scope"], "printed_text_line_extraction_only");
    assert_eq!(summary["privacy_filter_contract"], "text_only_normalized_input");
    assert_eq!(summary["ready_for_text_pii_eval"], true);
    assert!(summary["non_goals"].as_array().unwrap().contains(&serde_json::json!("visual_redaction")));
    assert!(summary.get("extracted_text").is_none());
    assert!(summary.get("normalized_text").is_none());
    assert!(summary.get("source").is_none());
}
```

- [ ] **Step 2: Run the test to verify RED**

Run: `cargo test -p mdid-cli --test cli_smoke ocr_small_json_writes_phi_safe_summary_output -- --nocapture`

Expected: FAIL because `ocr-small-json` currently rejects unknown flag `--summary-output`.

- [ ] **Step 3: Implement minimal parser and summary writer**

Change `OcrSmallJsonArgs` to include:

```rust
summary_output: Option<PathBuf>,
```

Update `parse_ocr_small_json_args` to accept `--summary-output <summary.json>`, reject missing values, and store it. In `run_ocr_small_json`, remove stale report and stale summary before prerequisite checks; on any error remove both. Reject identical report/summary paths before deletion with a PHI-safe `OCR small JSON summary path must differ from report path` error. After `validate_ocr_small_json_report(&value)?` and primary report write, write this summary if requested:

```json
{
  "artifact": "ocr_small_json_summary",
  "candidate": "PP-OCRv5_mobile_rec",
  "engine": "PP-OCRv5-mobile-bounded-spike",
  "engine_status": "deterministic_synthetic_fixture_fallback",
  "scope": "printed_text_line_extraction_only",
  "privacy_filter_contract": "text_only_normalized_input",
  "ready_for_text_pii_eval": true,
  "non_goals": [
    "visual_redaction",
    "image_pixel_redaction",
    "final_pdf_rewrite_export",
    "handwriting_recognition",
    "full_page_detection_or_segmentation",
    "complete_ocr_pipeline",
    "browser_ui",
    "desktop_ui"
  ]
}
```

The summary must omit `source`, `extracted_text`, `normalized_text`, local paths, bbox/image data, spans, previews, masked text, and raw fixture PHI. Keep stdout redacted and add `summary_written: true` only when a summary was written.

- [ ] **Step 4: Run targeted tests to verify GREEN**

Run: `cargo test -p mdid-cli --test cli_smoke ocr_small_json_writes_phi_safe_summary_output -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Write failure cleanup and same-path RED tests**

Add tests proving stale summary cleanup when prerequisites fail and same path rejection is PHI/path safe:

```rust
#[test]
fn ocr_small_json_removes_stale_summary_on_missing_runner() {
    let dir = tempdir().unwrap();
    let report_path = dir.path().join("ocr-small-json-report.json");
    let summary_path = dir.path().join("ocr-small-json-summary.json");
    std::fs::write(&summary_path, "Jane Example stale summary").unwrap();

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("ocr-small-json")
        .arg("--image-path")
        .arg("scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png")
        .arg("--ocr-runner-path")
        .arg(dir.path().join("missing-runner.py"))
        .arg("--report-path")
        .arg(&report_path)
        .arg("--summary-output")
        .arg(&summary_path)
        .arg("--python-command")
        .arg(default_python_command())
        .arg("--mock")
        .assert()
        .failure()
        .stderr(predicate::str::contains("OCR small JSON failed"));

    assert!(!report_path.exists());
    assert!(!summary_path.exists());
}

#[test]
fn ocr_small_json_rejects_same_report_and_summary_path() {
    let dir = tempdir().unwrap();
    let shared_path = dir.path().join("Jane-Example-MRN-12345.json");
    std::fs::write(&shared_path, "Jane Example stale artifact").unwrap();

    let assert = Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("ocr-small-json")
        .arg("--image-path")
        .arg("scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png")
        .arg("--ocr-runner-path")
        .arg("scripts/ocr_eval/run_small_ocr.py")
        .arg("--report-path")
        .arg(&shared_path)
        .arg("--summary-output")
        .arg(&shared_path)
        .arg("--python-command")
        .arg(default_python_command())
        .arg("--mock")
        .assert()
        .failure();

    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(stderr.contains("OCR small JSON summary path must differ from report path"));
    assert!(!stderr.contains(shared_path.to_string_lossy().as_ref()));
    assert!(!stderr.contains("Jane Example"));
    assert!(!stderr.contains("MRN-12345"));
    assert!(shared_path.exists());
}
```

- [ ] **Step 6: Implement cleanup/same-path handling and run target suite**

Run: `cargo test -p mdid-cli --test cli_smoke ocr_small_json -- --nocapture`

Expected: PASS for all `ocr_small_json` smoke tests.

- [ ] **Step 7: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs
git commit -m "feat(cli): add ocr small json summary output"
```

### Task 2: Truth-sync docs and completion evidence

**Files:**
- Modify: `scripts/ocr_eval/README.md`
- Modify: `README.md`

- [x] **Step 1: Update local OCR docs**

Add exact CLI usage showing `--summary-output` and state the summary is aggregate-only PP-OCRv5 mobile printed-text extraction readiness evidence for downstream text-only Privacy Filter evaluation. Explicitly list non-goals: not OCR model-quality proof, not visual redaction, not image pixel redaction, not final PDF rewrite/export, not Browser/Web OCR execution, not Desktop OCR execution, not a full OCR pipeline.

- [x] **Step 2: Update top-level README completion snapshot**

Truth-sync the completion snapshot to include this new optional summary artifact. Completion arithmetic: add and complete one CLI/runtime requirement in the same round, so CLI moves from `99/104` to `100/105 = 95%` conservative floor; Browser/Web remains 99%; Desktop remains 99%; Overall remains 97%. This is CLI/runtime only, so Browser/Web +5% is FAIL and Desktop +5% is FAIL for the round.

- [x] **Step 3: Run verification**

Run:

```bash
cargo test -p mdid-cli --test cli_smoke ocr_small_json -- --nocapture
cargo fmt --check
git diff --check
```

Expected: all PASS.

- [x] **Step 4: Commit docs**

```bash
git add README.md scripts/ocr_eval/README.md
git commit -m "docs: truth-sync ocr small summary output"
```
