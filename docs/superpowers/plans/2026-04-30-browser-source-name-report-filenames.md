# Browser Source Name Report Filenames Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make browser PDF review report downloads use the provided source name when no imported browser file name exists, without exposing PHI in report JSON or changing binary DICOM output behavior.

**Architecture:** Extend the existing browser safe filename helper path in `BrowserFlowState::suggested_export_file_name` with a narrow fallback for report-only PDF review mode. Imported file names remain the highest-priority source, and default filenames remain unchanged when neither imported file name nor source name is usable.

**Tech Stack:** Rust, mdid-browser crate, cargo tests.

---

## File Structure

- Modify: `crates/mdid-browser/src/app.rs`
  - `sanitized_import_stem` remains the single filename stem sanitizer.
  - `BrowserFlowState::suggested_export_file_name` gains a PDF-only `source_name` fallback when `imported_file_name` is absent.
  - Unit tests live in the existing test module in the same file.
- Modify: `README.md`
  - Truth-sync completion snapshot after landed implementation and verification.
- Create: `docs/superpowers/plans/2026-04-30-browser-source-name-report-filenames.md`
  - This plan.

### Task 1: Browser PDF source-name fallback for review report filenames

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`
- Test: `crates/mdid-browser/src/app.rs`

- [ ] **Step 1: Write the failing test**

Add this test to the existing `#[cfg(test)] mod tests` in `crates/mdid-browser/src/app.rs`:

```rust
#[test]
fn pdf_review_download_uses_safe_source_name_when_no_imported_file_exists() {
    let mut state = BrowserFlowState {
        input_mode: InputMode::PdfBase64,
        source_name: "C:/records/Patient Jane MRI Scan.pdf".to_string(),
        result_output: "review only".to_string(),
        summary: "PDF review summary".to_string(),
        review_queue: "review queue".to_string(),
        ..BrowserFlowState::default()
    };
    state.imported_file_name = None;

    let payload = state.prepared_download_payload().expect("download payload");

    assert_eq!(
        payload.file_name,
        "patient-jane-mri-scan-review-report.json"
    );
    assert_eq!(payload.mime_type, "application/json;charset=utf-8");
    assert!(payload.is_text);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p mdid-browser --lib pdf_review_download_uses_safe_source_name_when_no_imported_file_exists -- --nocapture`

Expected: FAIL because the current fallback filename is `mdid-browser-review-report.json`.

- [ ] **Step 3: Write minimal implementation**

In `BrowserFlowState::suggested_export_file_name`, immediately after the `if let Some(imported_file_name)` block and before the default `match self.input_mode`, add:

```rust
        if self.input_mode == InputMode::PdfBase64 && !self.source_name.trim().is_empty() {
            let stem = sanitized_import_stem(&self.source_name);
            if stem != "mdid-browser-output" {
                return format!("{stem}-review-report.json");
            }
        }
```

- [ ] **Step 4: Run targeted test to verify it passes**

Run: `cargo test -p mdid-browser --lib pdf_review_download_uses_safe_source_name_when_no_imported_file_exists -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Run broader browser tests**

Run: `cargo test -p mdid-browser --lib`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/mdid-browser/src/app.rs docs/superpowers/plans/2026-04-30-browser-source-name-report-filenames.md
git commit -m "feat(browser): use PDF source name for report downloads"
```

### Task 2: README completion truth-sync

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update completion snapshot text**

Update the current repository status snapshot to mention the browser PDF source-name report filename fallback and the verification commands from Task 1. Keep CLI unchanged at 95%, Browser/web at 75%, Desktop app at 69%, Overall at 93% unless landed functionality justifies a stricter change.

- [ ] **Step 2: Run verification evidence commands**

Run:

```bash
cargo test -p mdid-browser --lib pdf_review_download_uses_safe_source_name_when_no_imported_file_exists -- --nocapture
cargo test -p mdid-browser --lib
```

Expected: both PASS.

- [ ] **Step 3: Commit README truth-sync**

```bash
git add README.md docs/superpowers/plans/2026-04-30-browser-source-name-report-filenames.md
git commit -m "docs: truth-sync browser source-name report filenames"
```
