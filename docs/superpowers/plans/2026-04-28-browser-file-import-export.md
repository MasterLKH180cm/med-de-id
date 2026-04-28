# Browser File Import/Export Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded browser file import/export foundation so users can load CSV text and base64-transported XLSX/PDF payloads into the existing browser runtime flow and save returned output with truthful limitations.

**Architecture:** Keep `mdid-browser` thin: add pure Rust state helpers for file import metadata, safe default output filenames, and export availability, then wire minimal Leptos controls around the existing payload/result state. Do not add new runtime endpoints, auth, vault browsing, desktop flows, OCR, visual redaction, or orchestration semantics.

**Tech Stack:** Rust, Leptos, wasm `web-sys`/`gloo-file`-style browser APIs where available, serde JSON, existing `mdid-runtime` contracts.

---

## File Structure

- Modify: `crates/mdid-browser/Cargo.toml` — add any narrowly required browser-file dependency only if the existing dependency set cannot read file input in wasm.
- Modify: `crates/mdid-browser/src/app.rs` — add browser import/export state helpers, UI copy, file import controls, output filename logic, and unit tests.
- Modify: `README.md` — truth-sync Browser/web and Overall completion plus missing browser upload/download limitations.

## Task 1: Pure browser import/export state helpers

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`

- [ ] **Step 1: Write failing tests for import/export helper behavior**

Add these tests inside `#[cfg(test)] mod tests` in `crates/mdid-browser/src/app.rs`:

```rust
#[test]
fn file_import_metadata_updates_payload_source_and_clears_generated_state() {
    let mut state = BrowserFlowState::default();
    state.result_output = "old output".to_string();
    state.summary = "old summary".to_string();
    state.review_queue = "old review".to_string();
    state.error_banner = Some("old error".to_string());

    state.apply_imported_file("report.pdf", "UERG", InputMode::PdfBase64);

    assert_eq!(state.input_mode, InputMode::PdfBase64);
    assert_eq!(state.payload, "UERG");
    assert_eq!(state.source_name, "report.pdf");
    assert_eq!(state.result_output, "");
    assert_eq!(state.summary, IDLE_SUMMARY);
    assert_eq!(state.review_queue, IDLE_REVIEW_QUEUE);
    assert!(state.error_banner.is_none());
    assert_eq!(state.imported_file_name.as_deref(), Some("report.pdf"));
}

#[test]
fn imported_file_name_selects_mode_from_safe_extension() {
    assert_eq!(InputMode::from_file_name("patients.csv"), Some(InputMode::CsvText));
    assert_eq!(InputMode::from_file_name("workbook.XLSX"), Some(InputMode::XlsxBase64));
    assert_eq!(InputMode::from_file_name("scan.PDF"), Some(InputMode::PdfBase64));
    assert_eq!(InputMode::from_file_name("archive.zip"), None);
}

#[test]
fn export_filename_is_safe_and_mode_specific() {
    let mut state = BrowserFlowState::default();
    state.imported_file_name = Some("Jane Patient.csv".to_string());
    assert_eq!(state.suggested_export_file_name(), "mdid-browser-output.csv");

    state.input_mode = InputMode::XlsxBase64;
    state.imported_file_name = Some("clinic workbook.xlsx".to_string());
    assert_eq!(state.suggested_export_file_name(), "mdid-browser-output.xlsx.base64.txt");

    state.input_mode = InputMode::PdfBase64;
    state.imported_file_name = Some("scan.pdf".to_string());
    assert_eq!(state.suggested_export_file_name(), "mdid-browser-review-report.txt");
}

#[test]
fn export_is_available_only_after_runtime_output_exists() {
    let mut state = BrowserFlowState::default();
    assert!(!state.can_export_output());

    state.result_output = "rewritten".to_string();
    assert!(state.can_export_output());

    state.result_output = "   ".to_string();
    assert!(!state.can_export_output());
}
```

- [ ] **Step 2: Run tests and verify RED**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-browser file_import_metadata_updates_payload_source_and_clears_generated_state imported_file_name_selects_mode_from_safe_extension export_filename_is_safe_and_mode_specific export_is_available_only_after_runtime_output_exists -- --nocapture
```

Expected: FAIL because `apply_imported_file`, `InputMode::from_file_name`, `imported_file_name`, `suggested_export_file_name`, and `can_export_output` do not exist.

- [ ] **Step 3: Implement minimal pure helpers**

In `crates/mdid-browser/src/app.rs`:

1. Add `imported_file_name: Option<String>` to `BrowserFlowState` and default it to `None`.
2. Add `InputMode::from_file_name(file_name: &str) -> Option<Self>` using lower-case extensions: `.csv`, `.xlsx`, `.pdf`.
3. Add `BrowserFlowState::apply_imported_file(&mut self, file_name: &str, payload: &str, mode: InputMode)` that updates mode, payload, PDF source name, imported filename, and calls `invalidate_generated_state()`.
4. Add `BrowserFlowState::suggested_export_file_name(&self) -> &'static str` returning exactly:
   - CSV: `mdid-browser-output.csv`
   - XLSX: `mdid-browser-output.xlsx.base64.txt`
   - PDF: `mdid-browser-review-report.txt`
5. Add `BrowserFlowState::can_export_output(&self) -> bool` returning true only when `result_output.trim()` is non-empty.

- [ ] **Step 4: Run targeted tests and verify GREEN**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-browser file_import_metadata_updates_payload_source_and_clears_generated_state imported_file_name_selects_mode_from_safe_extension export_filename_is_safe_and_mode_specific export_is_available_only_after_runtime_output_exists -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Run browser crate tests**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-browser
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/mdid-browser/src/app.rs docs/superpowers/plans/2026-04-28-browser-file-import-export.md
git commit -m "feat(browser): add file import export state helpers"
```

## Task 2: Browser UI controls for bounded import/export

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`
- Modify: `README.md`

- [ ] **Step 1: Write failing tests for user-visible copy and download boundary**

Add these tests inside `#[cfg(test)] mod tests`:

```rust
#[test]
fn import_export_copy_discloses_bounded_browser_file_limits() {
    assert!(BROWSER_FILE_IMPORT_COPY.contains("CSV files load as text"));
    assert!(BROWSER_FILE_IMPORT_COPY.contains("XLSX and PDF files load as base64 payloads"));
    assert!(BROWSER_FILE_IMPORT_COPY.contains("does not add OCR, visual redaction, vault browsing, or auth/session"));
}

#[test]
fn unsupported_import_extension_error_is_honest() {
    let mut state = BrowserFlowState::default();
    state.reject_imported_file("notes.txt");

    assert_eq!(
        state.error_banner.as_deref(),
        Some("Unsupported browser import file type. Use .csv, .xlsx, or .pdf.")
    );
    assert_eq!(state.summary, IDLE_SUMMARY);
    assert_eq!(state.review_queue, IDLE_REVIEW_QUEUE);
}
```

- [ ] **Step 2: Run tests and verify RED**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-browser import_export_copy_discloses_bounded_browser_file_limits unsupported_import_extension_error_is_honest -- --nocapture
```

Expected: FAIL because `BROWSER_FILE_IMPORT_COPY` and `reject_imported_file` do not exist.

- [ ] **Step 3: Implement minimal browser UI/copy helpers**

In `crates/mdid-browser/src/app.rs`:

1. Add constant:

```rust
const BROWSER_FILE_IMPORT_COPY: &str = "Bounded browser file import: CSV files load as text; XLSX and PDF files load as base64 payloads for existing localhost runtime routes. This does not add OCR, visual redaction, vault browsing, or auth/session.";
```

2. Add `BrowserFlowState::reject_imported_file(&mut self, file_name: &str)` that calls `invalidate_generated_state()`, stores the file name in `imported_file_name`, and sets error banner exactly to `Unsupported browser import file type. Use .csv, .xlsx, or .pdf.`.
3. Add a visible paragraph in the input section rendering `BROWSER_FILE_IMPORT_COPY`.
4. Add a file input with truthful label `Import local CSV/XLSX/PDF payload` and wasm-only event handling that reads `.csv` as text and `.xlsx`/`.pdf` as base64 text before calling `apply_imported_file`. In non-wasm unit-test builds, keep the pure helpers testable and avoid pretending real file upload works.
5. Add export controls that render the suggested file name and only enable download when `can_export_output()` is true. Export should save the current `result_output` text only; PDF export remains a review report text file, not a rewritten PDF.

- [ ] **Step 4: Run targeted tests and browser tests**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-browser import_export_copy_discloses_bounded_browser_file_limits unsupported_import_extension_error_is_honest -- --nocapture
cargo test -p mdid-browser
```

Expected: PASS.

- [ ] **Step 5: Update README completion truthfully**

In `README.md`, update the completion table if warranted by landed tests:
- Browser/web: from `30%` to `34%`, mentioning bounded CSV/XLSX/PDF file import/export helpers on top of existing runtime routes.
- Overall: from `43%` to `44%`, mentioning browser file import/export foundation only.
- Missing items must still include richer upload/download UX depth, desktop file picker upload/download, vault/decode/audit workflows, OCR, visual redaction, and PDF rewrite/export.

- [ ] **Step 6: Verify docs and tests**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-browser
cargo test -p mdid-runtime --test runtime_http
cargo clippy -p mdid-browser --all-targets -- -D warnings
git diff --check
grep -nE 'Browser/web|Desktop app|Overall|Missing|OCR|visual redaction|auth/session|controller|orchestration|agent|moat' README.md
```

Expected: tests and clippy PASS; grep output shows only honest limitations or scope-drift warnings for controller/orchestration/agent/moat terms.

- [ ] **Step 7: Commit**

```bash
git add crates/mdid-browser/src/app.rs README.md docs/superpowers/plans/2026-04-28-browser-file-import-export.md
git commit -m "feat(browser): add bounded file import export controls"
```

## Self-Review

- Spec coverage: The plan covers a high-leverage browser upload/download foundation using existing runtime routes, with no new core runtime behavior and no scope-drift workflow semantics.
- Placeholder scan: No TBD/TODO placeholders are present; all tests and commands are explicit.
- Type consistency: Helper names are consistent across tasks: `apply_imported_file`, `reject_imported_file`, `suggested_export_file_name`, `can_export_output`, and `InputMode::from_file_name`.
