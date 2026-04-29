# Desktop Review Report Downloads Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add PHI-safe desktop JSON review-report download support for bounded PDF review and metadata-only conservative media review responses.

**Architecture:** Extend the existing `DesktopWorkflowResponseState` helper layer in `crates/mdid-desktop/src/lib.rs` so review-only responses can produce structured, safe JSON downloads without exposing raw runtime bodies or rewritten binary output. Keep this as a desktop helper/workstation surface only; do not add controller, agent, orchestration, planner, or moat semantics.

**Tech Stack:** Rust workspace, mdid-desktop crate, serde_json, cargo test/clippy.

---

## File Structure

- Modify: `crates/mdid-desktop/src/lib.rs`
  - Add a small `DesktopWorkflowReviewReportDownload` struct.
  - Add `DesktopWorkflowResponseState::review_report_download(mode)` to export structured JSON bytes for `PdfBase64Review` and `MediaMetadataJson` successes only.
  - Add tests near existing desktop workflow response tests.
- Modify: `README.md`
  - Truth-sync completion snapshot after the landed feature and controller-visible verification.

### Task 1: Desktop PDF Review Report JSON Download Helper

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`
- Test: `crates/mdid-desktop/src/lib.rs` unit tests

- [ ] **Step 1: Write the failing test**

Add this test inside `#[cfg(test)] mod tests` in `crates/mdid-desktop/src/lib.rs`:

```rust
#[test]
fn pdf_review_report_download_exports_structured_json_without_pdf_bytes() {
    let mut state = DesktopWorkflowResponseState::default();
    state.apply_success_json(
        DesktopWorkflowMode::PdfBase64Review,
        json!({
            "summary": {"pages": 1, "ocr_required": false},
            "review_queue": [{"page": 1, "reason": "text-layer review"}],
            "rewritten_pdf_bytes_base64": null,
            "debug_raw_text": "Alice Patient"
        }),
    );

    let download = state
        .review_report_download(DesktopWorkflowMode::PdfBase64Review)
        .expect("pdf review success should create a structured review report download");

    assert_eq!(download.file_name, "desktop-pdf-review-report.json");
    let text = std::str::from_utf8(&download.bytes).expect("report is utf8 json");
    let report: serde_json::Value = serde_json::from_str(text).expect("report parses");
    assert_eq!(report["mode"], "pdf_review");
    assert_eq!(report["summary"], json!({"pages": 1, "ocr_required": false}));
    assert_eq!(
        report["review_queue"],
        json!([{"page": 1, "reason": "text-layer review"}])
    );
    assert!(report.get("rewritten_pdf_bytes_base64").is_none());
    assert!(report.get("debug_raw_text").is_none());
    assert!(!text.contains("Alice Patient"));
}
```

- [ ] **Step 2: Run the failing test**

Run: `cargo test -p mdid-desktop pdf_review_report_download_exports_structured_json_without_pdf_bytes -- --nocapture`

Expected: FAIL because `review_report_download` and/or `DesktopWorkflowReviewReportDownload` does not exist.

- [ ] **Step 3: Implement the minimal helper**

In `crates/mdid-desktop/src/lib.rs`, near `DesktopWorkflowOutputDownload`, add:

```rust
#[derive(Clone, PartialEq, Eq)]
pub struct DesktopWorkflowReviewReportDownload {
    pub file_name: &'static str,
    pub bytes: Vec<u8>,
}

impl std::fmt::Debug for DesktopWorkflowReviewReportDownload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DesktopWorkflowReviewReportDownload")
            .field("file_name", &self.file_name)
            .field("bytes", &"<redacted>")
            .finish()
    }
}
```

Add this method inside `impl DesktopWorkflowResponseState`:

```rust
pub fn review_report_download(
    &self,
    mode: DesktopWorkflowMode,
) -> Option<DesktopWorkflowReviewReportDownload> {
    if self.error.is_some() || self.last_success_mode != Some(mode) {
        return None;
    }

    let response = self.last_success_response.as_ref()?;
    let (file_name, mode_label) = match mode {
        DesktopWorkflowMode::PdfBase64Review => ("desktop-pdf-review-report.json", "pdf_review"),
        DesktopWorkflowMode::MediaMetadataJson => {
            ("desktop-media-review-report.json", "media_metadata_review")
        }
        DesktopWorkflowMode::CsvText | DesktopWorkflowMode::XlsxBase64 | DesktopWorkflowMode::DicomBase64 => {
            return None;
        }
    };

    let report = serde_json::json!({
        "mode": mode_label,
        "summary": response.get("summary").cloned().unwrap_or(serde_json::Value::Null),
        "review_queue": response.get("review_queue").cloned().unwrap_or(serde_json::Value::Null),
    });
    let bytes = serde_json::to_vec_pretty(&report).ok()?;

    Some(DesktopWorkflowReviewReportDownload { file_name, bytes })
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p mdid-desktop pdf_review_report_download_exports_structured_json_without_pdf_bytes -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-desktop/src/lib.rs docs/superpowers/plans/2026-04-29-desktop-review-report-downloads.md
git commit -m "feat(desktop): export PDF review reports"
```

### Task 2: Desktop Media Review Report JSON Download Helper and Verification

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`
- Modify: `README.md`
- Test: `crates/mdid-desktop/src/lib.rs` unit tests

- [ ] **Step 1: Write the failing test**

Add this test inside `#[cfg(test)] mod tests` in `crates/mdid-desktop/src/lib.rs`:

```rust
#[test]
fn media_review_report_download_exports_structured_json_without_raw_metadata() {
    let mut state = DesktopWorkflowResponseState::default();
    state.apply_success_json(
        DesktopWorkflowMode::MediaMetadataJson,
        json!({
            "summary": {"metadata_fields": 2, "ocr_or_visual_review_required": true},
            "review_queue": [{"field": "PatientName", "reason": "metadata candidate"}],
            "metadata": [{"key": "PatientName", "value": "Jane Patient"}],
            "rewritten_media_bytes_base64": null
        }),
    );

    let download = state
        .review_report_download(DesktopWorkflowMode::MediaMetadataJson)
        .expect("media review success should create a structured review report download");

    assert_eq!(download.file_name, "desktop-media-review-report.json");
    let text = std::str::from_utf8(&download.bytes).expect("report is utf8 json");
    let report: serde_json::Value = serde_json::from_str(text).expect("report parses");
    assert_eq!(report["mode"], "media_metadata_review");
    assert_eq!(
        report["summary"],
        json!({"metadata_fields": 2, "ocr_or_visual_review_required": true})
    );
    assert_eq!(
        report["review_queue"],
        json!([{"field": "PatientName", "reason": "metadata candidate"}])
    );
    assert!(report.get("metadata").is_none());
    assert!(report.get("rewritten_media_bytes_base64").is_none());
    assert!(!text.contains("Jane Patient"));
}
```

- [ ] **Step 2: Run the failing test**

Run: `cargo test -p mdid-desktop media_review_report_download_exports_structured_json_without_raw_metadata -- --nocapture`

Expected: FAIL until media mode is allowed by `review_report_download`.

- [ ] **Step 3: Implement or adjust the helper**

Ensure `DesktopWorkflowResponseState::review_report_download` returns `desktop-media-review-report.json` with mode `media_metadata_review` for `DesktopWorkflowMode::MediaMetadataJson`, and returns `None` for CSV/XLSX/DICOM and error/stale-mode states.

- [ ] **Step 4: Add guard test for non-review outputs**

Add this test:

```rust
#[test]
fn review_report_download_is_unavailable_for_rewritten_binary_outputs_and_errors() {
    let mut state = DesktopWorkflowResponseState::default();
    state.apply_success_json(
        DesktopWorkflowMode::XlsxBase64,
        json!({
            "summary": {"row_count": 1},
            "review_queue": [],
            "rewritten_workbook_base64": "AAE="
        }),
    );
    assert!(state
        .review_report_download(DesktopWorkflowMode::XlsxBase64)
        .is_none());

    state.apply_error("runtime failed with /secret/path and PHI");
    assert!(state
        .review_report_download(DesktopWorkflowMode::MediaMetadataJson)
        .is_none());
}
```

- [ ] **Step 5: Run targeted tests**

Run: `cargo test -p mdid-desktop review_report_download -- --nocapture`

Expected: PASS for all review-report download tests.

- [ ] **Step 6: Run broader verification**

Run:

```bash
cargo test -p mdid-desktop --lib
cargo clippy -p mdid-desktop --all-targets -- -D warnings
git diff --check
```

Expected: all PASS.

- [ ] **Step 7: Update README completion snapshot**

Update `README.md` completion snapshot to mention desktop structured review report JSON downloads. Keep CLI at 95%, raise desktop app only if justified by landed feature and verification, keep browser/web truthful, and keep missing blockers explicit.

- [ ] **Step 8: Commit**

```bash
git add crates/mdid-desktop/src/lib.rs README.md
git commit -m "feat(desktop): export review report downloads"
```
