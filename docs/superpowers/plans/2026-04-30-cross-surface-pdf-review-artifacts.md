# Cross-Surface PDF Review Artifacts Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add bounded, PHI-safe PDF review artifact download/save support to both Browser/Web and Desktop surfaces.

**Architecture:** Keep PDF mode review-only and local-first. Reuse existing runtime `/pdf/deidentify` responses and add surface helpers that generate sanitized review report artifacts from successful PDF responses without adding OCR, visual redaction, PDF rewrite/export, vault browsing, controller, or agent workflow semantics.

**Tech Stack:** Rust workspace, Leptos browser crate, desktop helper crate, serde_json, cargo tests.

---

## File Structure

- Modify: `crates/mdid-browser/src/app.rs` — add Browser PDF review report payload helper and UI state wiring for PDF runtime responses.
- Modify: `crates/mdid-desktop/src/lib.rs` — add Desktop PDF review report save helper/state methods for workstation PDF responses.
- Modify: `README.md` — truth-sync completion snapshot and missing-items list after verified landed functionality.

### Task 1: Browser PDF Review Report Download Helper

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`
- Test: `crates/mdid-browser/src/app.rs`

- [ ] **Step 1: Write failing browser tests**

Add tests in the existing browser test module near other download/report tests:

```rust
#[test]
fn pdf_review_report_download_redacts_text_and_uses_source_stem() {
    let response = serde_json::json!({
        "summary": {
            "total_pages": 2,
            "pages_with_text": 1,
            "ocr_required_pages": 1,
            "sensitive_text": "Alice Smith MRN 123"
        },
        "review_queue": [
            {
                "page": 1,
                "kind": "text_layer_candidate",
                "status": "needs_review",
                "text": "Alice Smith MRN 123",
                "bbox": [1, 2, 3, 4]
            }
        ],
        "pdf_bytes_base64": "SHOULD_NOT_LEAK"
    });

    let payload = build_pdf_review_report_download(
        &response.to_string(),
        Some("Clinic Intake Form.pdf"),
    )
    .expect("pdf report download");

    assert_eq!(payload.file_name, "Clinic_Intake_Form-pdf-review-report.json");
    assert_eq!(payload.mime_type, "application/json");
    assert!(payload.is_text);

    let report: serde_json::Value = serde_json::from_slice(&payload.bytes).unwrap();
    assert_eq!(report["mode"], "pdf_review_report");
    assert_eq!(report["summary"]["total_pages"], 2);
    assert_eq!(report["summary"]["pages_with_text"], 1);
    assert_eq!(report["summary"]["ocr_required_pages"], 1);
    assert_eq!(report["review_queue"][0]["page"], 1);
    assert_eq!(report["review_queue"][0]["kind"], "text_layer_candidate");
    assert_eq!(report["review_queue"][0]["status"], "needs_review");
    let serialized = serde_json::to_string(&report).unwrap();
    assert!(!serialized.contains("Alice"));
    assert!(!serialized.contains("MRN"));
    assert!(!serialized.contains("SHOULD_NOT_LEAK"));
    assert!(!serialized.contains("pdf_bytes_base64"));
    assert!(!serialized.contains("text"));
}

#[test]
fn pdf_review_report_download_rejects_non_object_runtime_response() {
    let error = build_pdf_review_report_download("[]", Some("scan.pdf")).unwrap_err();
    assert!(error.contains("PDF review report requires a JSON object response"));
}
```

- [ ] **Step 2: Run browser RED test**

Run: `cargo test -p mdid-browser pdf_review_report_download -- --nocapture`

Expected: FAIL because `build_pdf_review_report_download` is not defined.

- [ ] **Step 3: Implement minimal browser helper**

Add a helper that parses the PDF response JSON object, emits only `mode`, sanitized numeric/string/boolean/null summary fields from an allowlist (`total_pages`, `pages_with_text`, `ocr_required_pages`, `candidate_count`, `requires_ocr`, `status`), and sanitized review queue items with only `page`, `kind`, and `status`. Use `sanitize_import_source_stem`/existing source-stem sanitization helpers if present; otherwise add a focused helper that replaces unsafe filename characters with `_`, trims dots/spaces, caps at the existing stem length, and falls back to `mdid-browser-pdf`.

- [ ] **Step 4: Run browser GREEN test**

Run: `cargo test -p mdid-browser pdf_review_report_download -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Commit browser task**

```bash
git add crates/mdid-browser/src/app.rs
git commit -m "feat(browser): add pdf review report downloads"
```

### Task 2: Desktop PDF Review Report Save Helper

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`
- Test: `crates/mdid-desktop/src/lib.rs`

- [ ] **Step 1: Write failing desktop tests**

Add tests in the existing desktop test module near other report-save tests:

```rust
#[test]
fn desktop_pdf_review_report_save_redacts_text_and_uses_source_stem() {
    let response = serde_json::json!({
        "summary": {
            "total_pages": 3,
            "pages_with_text": 2,
            "ocr_required_pages": 1,
            "sensitive_text": "Jane Roe DOB 1970"
        },
        "review_queue": [
            {
                "page": 2,
                "kind": "ocr_required",
                "status": "needs_review",
                "text": "Jane Roe DOB 1970"
            }
        ],
        "pdf_bytes_base64": "SHOULD_NOT_LEAK"
    });

    let save = build_desktop_pdf_review_report_save(
        &response.to_string(),
        Some("workstation scan.pdf"),
    )
    .expect("desktop pdf report save");

    assert_eq!(save.file_name, "workstation_scan-pdf-review-report.json");
    assert_eq!(save.status, "PDF review report ready to save; text content and PDF bytes are redacted from this report.");
    let report: serde_json::Value = serde_json::from_str(&save.contents).unwrap();
    assert_eq!(report["mode"], "pdf_review_report");
    assert_eq!(report["summary"]["total_pages"], 3);
    assert_eq!(report["review_queue"][0]["page"], 2);
    assert_eq!(report["review_queue"][0]["kind"], "ocr_required");
    let serialized = serde_json::to_string(&report).unwrap();
    assert!(!serialized.contains("Jane"));
    assert!(!serialized.contains("DOB"));
    assert!(!serialized.contains("SHOULD_NOT_LEAK"));
    assert!(!serialized.contains("pdf_bytes_base64"));
    assert!(!serialized.contains("text"));
}

#[test]
fn desktop_pdf_review_report_save_rejects_non_object_runtime_response() {
    let error = build_desktop_pdf_review_report_save("null", Some("scan.pdf")).unwrap_err();
    assert!(error.contains("PDF review report requires a JSON object response"));
}
```

- [ ] **Step 2: Run desktop RED test**

Run: `cargo test -p mdid-desktop pdf_review_report_save -- --nocapture`

Expected: FAIL because `build_desktop_pdf_review_report_save` is not defined.

- [ ] **Step 3: Implement minimal desktop helper**

Add `DesktopPdfReviewReportSave { file_name: String, contents: String, status: String }` and `build_desktop_pdf_review_report_save(response_json: &str, source_name: Option<&str>) -> Result<DesktopPdfReviewReportSave, String>`. Reuse or mirror the Browser sanitizer exactly: no raw text fields, no PDF bytes/base64, no nested sensitive payloads, only allowlisted primitive summary fields and queue item `page`/`kind`/`status`.

- [ ] **Step 4: Run desktop GREEN test**

Run: `cargo test -p mdid-desktop pdf_review_report_save -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Commit desktop task**

```bash
git add crates/mdid-desktop/src/lib.rs
git commit -m "feat(desktop): add pdf review report saves"
```

### Task 3: Truth-Sync README Completion Snapshot

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Verify cross-surface tests**

Run:

```bash
cargo test -p mdid-browser pdf_review_report_download -- --nocapture
cargo test -p mdid-desktop pdf_review_report_save -- --nocapture
cargo fmt --check
git diff --check
```

Expected: all PASS.

- [ ] **Step 2: Update README completion snapshot**

Update the completion snapshot to truthfully record this landed cross-surface PDF review artifact slice. Increase Browser/Web by +5 and Desktop app by +5 only if both tests pass and the commits are controller-visible. Keep CLI unchanged. Document that PDF remains review-only and still lacks OCR, visual redaction, handwriting handling, and PDF rewrite/export.

- [ ] **Step 3: Commit docs task**

```bash
git add README.md docs/superpowers/plans/2026-04-30-cross-surface-pdf-review-artifacts.md
git commit -m "docs: truth-sync pdf review artifact progress"
```
