# Desktop Review Report Source Filenames Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add bounded source-aware desktop review report JSON filenames for existing PDF review and media metadata review report downloads.

**Architecture:** Keep the feature in the existing `mdid-desktop` helper layer. Reuse the existing safe source stem sanitizer so filenames never expose paths and remain bounded to already-rendered PHI-safe review reports.

**Tech Stack:** Rust, `mdid-desktop`, serde_json, cargo tests/clippy.

---

## File Structure

- Modify: `crates/mdid-desktop/src/lib.rs`
  - Add `review_report_download_for_source(&self, mode, source_name)` on `DesktopWorkflowState`.
  - Reuse existing `review_report_download(mode)` payload generation and `safe_source_file_stem` sanitizer.
  - Add focused unit tests near existing desktop review report tests.
- Modify: `README.md`
  - Truth-sync desktop/browser/CLI/overall completion snapshot and verification evidence after the landed helper.

### Task 1: Desktop source-aware review report filenames

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`
- Test: `crates/mdid-desktop/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Add this test in the existing `#[cfg(test)]` module near the desktop report download tests:

```rust
#[test]
fn desktop_review_report_download_for_source_uses_safe_pdf_and_media_filenames() {
    let mut pdf_state = DesktopWorkflowState::default();
    pdf_state.apply_success_json(
        DesktopWorkflowMode::PdfBase64Review,
        serde_json::json!({
            "summary": { "total_pages": 1, "pages_requiring_review": 1 },
            "review_queue": [{ "page_index": 0, "reason": "ocr_required" }],
            "rewritten_pdf_bytes_base64": null,
        }),
    );

    let pdf_report = pdf_state
        .review_report_download_for_source(
            DesktopWorkflowMode::PdfBase64Review,
            Some("C:\\clinic\\March intake.pdf"),
        )
        .expect("pdf review report should be exportable");
    assert_eq!(pdf_report.file_name, "March-intake-pdf-review-report.json");

    let mut media_state = DesktopWorkflowState::default();
    media_state.apply_success_json(
        DesktopWorkflowMode::MediaMetadataJson,
        serde_json::json!({
            "summary": { "metadata_fields_reviewed": 2, "metadata_fields_requiring_review": 1 },
            "review_queue": [{ "field_index": 0, "reason": "metadata_identifier" }],
            "rewritten_media_bytes_base64": null,
        }),
    );

    let media_report = media_state
        .review_report_download_for_source(
            DesktopWorkflowMode::MediaMetadataJson,
            Some("/uploads/Camera Roll.metadata.json"),
        )
        .expect("media review report should be exportable");
    assert_eq!(
        media_report.file_name,
        "Camera-Roll.metadata-media-review-report.json"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p mdid-desktop desktop_review_report_download_for_source_uses_safe_pdf_and_media_filenames -- --nocapture`
Expected: FAIL with no method named `review_report_download_for_source`.

- [ ] **Step 3: Write minimal implementation**

Add this method next to `review_report_download`:

```rust
pub fn review_report_download_for_source(
    &self,
    mode: DesktopWorkflowMode,
    source_name: Option<&str>,
) -> Option<DesktopWorkflowReviewReportDownload> {
    let mut download = self.review_report_download(mode)?;
    let stem = source_name.and_then(safe_source_file_stem).unwrap_or_else(|| "desktop".to_string());
    download.file_name = match mode {
        DesktopWorkflowMode::PdfBase64Review => format!("{stem}-pdf-review-report.json"),
        DesktopWorkflowMode::MediaMetadataJson => format!("{stem}-media-review-report.json"),
        DesktopWorkflowMode::CsvText | DesktopWorkflowMode::XlsxBase64 | DesktopWorkflowMode::DicomBase64 => return None,
    };
    Some(download)
}
```

This method must not change report bytes and must not include source paths, runtime body, metadata values, decoded values, vault paths, passphrases, tokens, or artifact payloads.

- [ ] **Step 4: Run targeted test to verify it passes**

Run: `cargo test -p mdid-desktop desktop_review_report_download_for_source_uses_safe_pdf_and_media_filenames -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Run relevant broader verification**

Run: `cargo test -p mdid-desktop --lib`
Expected: PASS.

Run: `cargo clippy -p mdid-desktop --all-targets -- -D warnings`
Expected: PASS.

Run: `git diff --check`
Expected: no output and exit 0.

- [ ] **Step 6: Commit**

```bash
git add crates/mdid-desktop/src/lib.rs docs/superpowers/plans/2026-04-30-desktop-review-report-source-filenames.md
git commit -m "feat(desktop): add source-aware review report filenames"
```

### Task 2: README truth-sync for source-aware review report filenames

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update README snapshot**

Revise the Current repository status section so the desktop row states that desktop review report JSON save/download helpers now include source-aware safe filenames for bounded PDF and media review reports. Keep CLI/browser claims unchanged unless landed features justify otherwise. Keep Overall at 93% unless a broader blocker is honestly closed.

- [ ] **Step 2: Verify docs wording**

Run: `grep -n "Completion snapshot\|Desktop app\|Overall\|Verification evidence" README.md`
Expected: updated desktop wording is visible, Overall remains 93%, and verification evidence cites this SDD slice.

- [ ] **Step 3: Commit**

```bash
git add README.md docs/superpowers/plans/2026-04-30-desktop-review-report-source-filenames.md
git commit -m "docs: truth-sync desktop source-aware review reports"
```

## Self-Review

- Spec coverage: Task 1 adds the helper and tests source-aware safe filenames for PDF and media review reports. Task 2 truth-syncs README completion and evidence.
- Placeholder scan: No TBD/TODO/fill-in placeholders remain.
- Type consistency: The plan uses existing `DesktopWorkflowState`, `DesktopWorkflowMode`, `DesktopWorkflowReviewReportDownload`, `safe_source_file_stem`, and cargo commands already used by this crate.
