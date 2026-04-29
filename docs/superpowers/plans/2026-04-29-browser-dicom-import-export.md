# Browser DICOM Import/Export Helper Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded browser DICOM file import/export helper path that submits base64 DICOM payloads to the existing local runtime route and renders PHI-safe response summaries without adding workflow/controller semantics.

**Architecture:** Extend the existing `mdid-browser` single-page local-first flow with one additional `InputMode::DicomBase64`, reusing the existing bounded file import, validation, submission, response rendering, and text export helpers. The browser remains a local helper around existing localhost runtime endpoints and must disclose that this is not OCR, visual redaction, vault browsing, auth/session, or a generalized workflow surface.

**Tech Stack:** Rust workspace, Leptos browser crate, serde JSON response parsing, cargo test.

---

## File Structure

- Modify: `crates/mdid-browser/src/app.rs`
  - Add DICOM mode selection, `.dcm`/`.dicom` extension detection, base64 import read mode, `/dicom/deidentify` endpoint, DICOM request body, DICOM response parsing, export filename, and unit tests.
- Modify: `README.md`
  - Truth-sync completion table and current-scope bullets after the landed browser DICOM helper.

### Task 1: Browser DICOM mode, request, response, and tests

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`

- [ ] **Step 1: Write failing tests for DICOM mode and import/export helpers**

Add these tests inside the existing `#[cfg(test)] mod tests` in `crates/mdid-browser/src/app.rs`:

```rust
#[test]
fn imported_dicom_file_selects_dicom_mode_and_base64_read() {
    assert_eq!(InputMode::from_file_name("scan.dcm"), Some(InputMode::DicomBase64));
    assert_eq!(InputMode::from_file_name("SCAN.DICOM"), Some(InputMode::DicomBase64));
    assert_eq!(InputMode::DicomBase64.browser_file_read_mode(), BrowserFileReadMode::DataUrlBase64);
}

#[test]
fn dicom_mode_builds_bounded_runtime_request() {
    let mut state = BrowserFlowState {
        input_mode: InputMode::DicomBase64,
        payload: "ZGljb20=".to_string(),
        source_name: "local-scan.dcm".to_string(),
        ..BrowserFlowState::default()
    };

    let request = state.validate_submission().expect("valid DICOM request");

    assert_eq!(request.endpoint, "/dicom/deidentify");
    assert!(request.body.contains("\"dicom_bytes_base64\":\"ZGljb20=\""));
    assert!(request.body.contains("\"source_name\":\"local-scan.dcm\""));
    assert!(request.body.contains("\"private_tag_policy\":\"remove\""));
}

#[test]
fn dicom_response_renders_summary_review_queue_and_rewritten_bytes() {
    let response = r#"{
        "summary": {"fields_processed": 3, "encoded_fields": 1, "review_required": 1, "removed_private_tags": 2},
        "review_queue": [
            {"field": "PatientName", "reason": "manual review", "sample": "REDACTED"}
        ],
        "rewritten_dicom_bytes_base64": "cmV3cml0dGVu"
    }"#;

    let rendered = render_runtime_response(InputMode::DicomBase64, response).expect("DICOM response renders");

    assert!(rendered.summary.contains("fields processed: 3"));
    assert!(rendered.summary.contains("removed private tags: 2"));
    assert!(rendered.review_queue.contains("PatientName"));
    assert!(rendered.rewritten_output.contains("cmV3cml0dGVu"));
}

#[test]
fn dicom_mode_discloses_bounded_browser_limits() {
    assert!(InputMode::DicomBase64.disclosure_copy().unwrap().contains("DICOM mode uses the existing local runtime tag-level de-identification route"));
    assert!(InputMode::DicomBase64.disclosure_copy().unwrap().contains("does not add pixel redaction"));
    assert_eq!(BrowserFlowState { input_mode: InputMode::DicomBase64, ..BrowserFlowState::default() }.suggested_export_file_name(), "mdid-browser-output.dcm.base64.txt");
}
```

- [ ] **Step 2: Run tests to verify RED**

Run: `cargo test -p mdid-browser imported_dicom_file_selects_dicom_mode_and_base64_read dicom_mode_builds_bounded_runtime_request dicom_response_renders_summary_review_queue_and_rewritten_bytes dicom_mode_discloses_bounded_browser_limits -- --exact`

Expected: FAIL because `InputMode::DicomBase64` and DICOM rendering do not exist yet.

- [ ] **Step 3: Implement minimal DICOM browser mode**

In `crates/mdid-browser/src/app.rs`:

- Add `DicomBase64` to `InputMode`.
- Recognize `.dcm` and `.dicom` in `from_file_name`.
- Add select value `dicom-base64`, label `DICOM base64`, payload hint `Paste base64-encoded DICOM content here`, disclosure text, endpoint `/dicom/deidentify`, and base64 file read mode.
- Treat DICOM like PDF for requiring a non-empty source name by replacing `is_pdf()` with a method that covers runtime file modes.
- Build DICOM request JSON with `dicom_bytes_base64`, `source_name`, and `private_tag_policy: "remove"`.
- Add a `DicomRuntimeResponse`/summary/review item parser matching the runtime route shape and render summary, review queue, and `rewritten_dicom_bytes_base64`.
- Add DICOM to the mode selector UI and import copy.

- [ ] **Step 4: Run targeted tests to verify GREEN**

Run: `cargo test -p mdid-browser imported_dicom_file_selects_dicom_mode_and_base64_read dicom_mode_builds_bounded_runtime_request dicom_response_renders_summary_review_queue_and_rewritten_bytes dicom_mode_discloses_bounded_browser_limits -- --exact`

Expected: PASS.

- [ ] **Step 5: Run broader browser tests**

Run: `cargo test -p mdid-browser`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/mdid-browser/src/app.rs
git commit -m "feat(browser): add bounded dicom import export helper"
```

### Task 2: README truth-sync for browser DICOM helper

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Write the README update**

Update the completion table and current-scope bullets to state:

- Browser/web increases from 34% to 38% because browser now covers CSV/XLSX/PDF/DICOM bounded import/export helpers.
- Overall increases from 63% to 64% because one important browser workflow gap is landed and tested.
- Remaining gaps still include deeper browser UX, desktop vault browsing/decode/audit execution, full OCR/visual redaction/PDF rewrite, broader policy/detection crates, verification/audit polish, and production packaging.
- Keep explicit wording that browser DICOM uses the existing runtime tag-level route and does not add pixel redaction, OCR, auth/session, vault browsing, or generalized workflow behavior.

- [ ] **Step 2: Verify README contains the new truthful numbers and caveats**

Run: `grep -n "Browser/web\|Overall\|DICOM" README.md`

Expected: lines show Browser/web 38%, Overall 64%, and bounded browser DICOM helper caveats.

- [ ] **Step 3: Run verification tests for the landed slice**

Run: `cargo test -p mdid-browser`

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add README.md
git commit -m "docs: truth-sync browser dicom helper completion"
```
