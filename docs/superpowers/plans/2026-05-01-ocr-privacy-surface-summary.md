# OCR-to-Privacy-Filter Surface Summary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add bounded Browser/Web and Desktop helpers that turn existing OCR-to-Privacy-Filter CLI/runtime JSON reports into PHI-safe summary/download-save artifacts.

**Architecture:** Reuse the existing Browser/Desktop local-only report summary pattern: parse an existing JSON report, sanitize it through strict allowlists, and emit only aggregate-safe OCR-to-text-PII fields. This is a user-facing surface summary for existing evidence only; it must not execute OCR, run Privacy Filter, perform visual redaction, rewrite PDFs, or add workflow orchestration semantics.

**Tech Stack:** Rust workspace; `mdid-browser` App mode/report download helpers; `mdid-desktop` report save helpers; existing `serde_json` tests; README completion truth-sync.

---

## File Structure

- Modify `crates/mdid-browser/src/app.rs`: add an `ocr-to-privacy-filter-summary` local-only browser mode, user-facing copy, and `build_ocr_to_privacy_filter_summary_download()` sanitizer for existing OCR-to-Privacy-Filter reports.
- Modify `crates/mdid-desktop/src/lib.rs`: add a matching public desktop helper that builds/saves an OCR-to-Privacy-Filter summary JSON from an existing report.
- Modify `README.md`: truth-sync evidence and completion snapshot after landed tests.

### Task 1: Browser/Web OCR-to-Privacy-Filter summary mode

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`
- Test: existing unit tests in `crates/mdid-browser/src/app.rs`

- [x] **Step 1: Write the failing browser tests**

Add tests near the existing Privacy Filter/OCR handoff summary tests:

```rust
#[test]
fn browser_ocr_to_privacy_filter_summary_download_uses_safe_nested_report_fields() {
    let response = serde_json::json!({
        "artifact": "ocr_to_privacy_filter_single",
        "ocr_candidate": "PP-OCRv5_mobile_rec",
        "ocr_engine": "PP-OCRv5-mobile-bounded-spike",
        "ocr_scope": "printed_text_line_extraction_only",
        "privacy_scope": "text_only_pii_detection",
        "privacy_filter_engine": "fallback_synthetic_patterns",
        "privacy_filter_contract": "text_only_normalized_input",
        "ready_for_text_pii_eval": true,
        "network_api_called": false,
        "privacy_filter_detected_span_count": 4,
        "privacy_filter_category_counts": {"NAME": 1, "MRN": 1, "EMAIL": 1, "PHONE": 1},
        "normalized_text": "Patient Jane Example MRN-12345 jane@example.com 555-123-4567",
        "masked_text": "Patient [NAME] [MRN] [EMAIL] [PHONE]",
        "spans": [{"preview": "Jane Example"}],
        "report_path": "/tmp/Jane-Example-MRN-12345/report.json",
        "non_goals": ["not_visual_redaction", "not_image_pixel_redaction", "not_final_pdf_rewrite_export"]
    });

    let download = build_ocr_to_privacy_filter_summary_download("Jane Example", &response).unwrap();
    let body = String::from_utf8(download.bytes).unwrap();
    let summary: serde_json::Value = serde_json::from_str(&body).unwrap();

    assert_eq!(download.filename, "Jane_Example-ocr-to-privacy-filter-summary.json");
    assert_eq!(summary["mode"], "ocr_to_privacy_filter_summary");
    assert_eq!(summary["ocr_candidate"], "PP-OCRv5_mobile_rec");
    assert_eq!(summary["ocr_scope"], "printed_text_line_extraction_only");
    assert_eq!(summary["privacy_scope"], "text_only_pii_detection");
    assert_eq!(summary["privacy_filter_engine"], "fallback_synthetic_patterns");
    assert_eq!(summary["privacy_filter_contract"], "text_only_normalized_input");
    assert_eq!(summary["ready_for_text_pii_eval"], true);
    assert_eq!(summary["network_api_called"], false);
    assert_eq!(summary["privacy_filter_detected_span_count"], 4);
    assert_eq!(summary["privacy_filter_category_counts"]["NAME"], 1);
    assert_eq!(summary["privacy_filter_category_counts"]["MRN"], 1);
    for forbidden in ["Patient Jane Example", "MRN-12345", "jane@example.com", "555-123-4567", "/tmp/Jane-Example-MRN-12345", "masked_text", "spans", "preview"] {
        assert!(!body.contains(forbidden), "summary leaked {forbidden}: {body}");
    }
}

#[test]
fn browser_ocr_to_privacy_filter_summary_omits_unsafe_fields() {
    let response = serde_json::json!({
        "artifact": "ocr_to_privacy_filter_single",
        "ocr_candidate": "Jane Example",
        "ocr_engine": "PP-OCRv5-mobile-bounded-spike",
        "ocr_scope": "printed_text_line_extraction_only",
        "privacy_scope": "text_only_pii_detection",
        "privacy_filter_engine": "fallback_synthetic_patterns",
        "privacy_filter_contract": "text_only_normalized_input",
        "ready_for_text_pii_eval": true,
        "network_api_called": false,
        "privacy_filter_detected_span_count": 1,
        "privacy_filter_category_counts": {"PATIENT_JANE_EXAMPLE": 1, "NAME": 1},
        "visual_redaction": true
    });

    let download = build_ocr_to_privacy_filter_summary_download("unsafe", &response).unwrap();
    let body = String::from_utf8(download.bytes).unwrap();
    let summary: serde_json::Value = serde_json::from_str(&body).unwrap();

    assert!(summary.get("ocr_candidate").is_none());
    assert_eq!(summary["privacy_filter_category_counts"].get("NAME").unwrap(), 1);
    assert!(summary["privacy_filter_category_counts"].get("PATIENT_JANE_EXAMPLE").is_none());
    assert!(!body.contains("Jane Example"));
    assert!(!body.contains("visual_redaction"));
}
```

- [x] **Step 2: Run browser RED**

Run: `cargo test -p mdid-browser ocr_to_privacy_filter_summary -- --nocapture`

Expected: FAIL because `build_ocr_to_privacy_filter_summary_download` does not exist.

- [x] **Step 3: Implement browser summary mode and sanitizer**

Add an `OcrToPrivacyFilterSummary` variant to the browser mode enum with slug `ocr-to-privacy-filter-summary`, label `OCR to Privacy Filter summary`, placeholder copy for existing JSON reports, and local-only status text. Add `build_ocr_to_privacy_filter_summary_download()` and sanitizer helpers that output this allowlist only:

```json
{
  "mode": "ocr_to_privacy_filter_summary",
  "ocr_candidate": "PP-OCRv5_mobile_rec",
  "ocr_engine": "PP-OCRv5-mobile-bounded-spike",
  "ocr_scope": "printed_text_line_extraction_only",
  "privacy_scope": "text_only_pii_detection",
  "privacy_filter_engine": "fallback_synthetic_patterns",
  "privacy_filter_contract": "text_only_normalized_input",
  "ready_for_text_pii_eval": true,
  "network_api_called": false,
  "privacy_filter_detected_span_count": 4,
  "privacy_filter_category_counts": {"NAME": 1, "MRN": 1, "EMAIL": 1, "PHONE": 1, "ID": 1}
}
```

Only include string fields if they match the known safe values above. Only include category labels in `NAME`, `MRN`, `EMAIL`, `PHONE`, `ID` with nonnegative integer counts. Never copy raw text, masked text, spans, previews, paths, visual redaction, image pixel redaction, PDF export/rewrite, or arbitrary metadata.

- [x] **Step 4: Run browser GREEN**

Run: `cargo test -p mdid-browser ocr_to_privacy_filter_summary -- --nocapture`

Expected: PASS.

- [x] **Step 5: Commit browser slice**

Run:

```bash
git add crates/mdid-browser/src/app.rs
git commit -m "feat(browser): add OCR privacy summary downloads"
```

### Task 2: Desktop OCR-to-Privacy-Filter summary save helper

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`
- Test: existing unit tests in `crates/mdid-desktop/src/lib.rs`

- [x] **Step 1: Write failing desktop tests**

Add tests near the existing Desktop Privacy Filter/OCR handoff summary save helper tests with the same safe/unsafe payloads from Task 1. The success test must call the new public helper, parse the saved JSON, assert safe aggregate fields, and assert the saved body omits raw sentinels `Patient Jane Example`, `MRN-12345`, `jane@example.com`, `555-123-4567`, raw paths, `masked_text`, `spans`, and `preview`.

- [x] **Step 2: Run desktop RED**

Run: `cargo test -p mdid-desktop ocr_to_privacy_filter_summary -- --nocapture`

Expected: FAIL because the Desktop helper does not exist.

- [x] **Step 3: Implement desktop helper**

Add a public helper matching existing desktop report-save conventions, for example `save_ocr_to_privacy_filter_summary_report(...)`, that writes the same allowlisted summary contract used by Browser/Web and rejects/omits unsafe fields identically.

- [x] **Step 4: Run desktop GREEN**

Run: `cargo test -p mdid-desktop ocr_to_privacy_filter_summary -- --nocapture`

Expected: PASS.

- [x] **Step 5: Commit desktop slice**

Run:

```bash
git add crates/mdid-desktop/src/lib.rs
git commit -m "feat(desktop): add OCR privacy summary saves"
```

### Task 3: README completion truth-sync and final verification

**Files:**
- Modify: `README.md`

- [x] **Step 1: Update README evidence and completion snapshot**

Add a verification evidence paragraph saying Browser/Web now has a local-only OCR-to-Privacy-Filter summary mode for existing CLI/runtime JSON reports and Desktop now has a matching save helper. Completion remains capped at the 99% target for Browser/Web and Desktop; Overall may stay 97% unless the repository-visible rubric arithmetic supports a conservative increase.

- [x] **Step 2: Run verification**

Run:

```bash
cargo test -p mdid-browser ocr_to_privacy_filter_summary -- --nocapture
cargo test -p mdid-desktop ocr_to_privacy_filter_summary -- --nocapture
cargo fmt --check
git diff --check
```

Expected: all pass.

- [x] **Step 3: Commit docs truth-sync**

Run:

```bash
git add README.md
git commit -m "docs: truth-sync OCR privacy surface summaries"
```
