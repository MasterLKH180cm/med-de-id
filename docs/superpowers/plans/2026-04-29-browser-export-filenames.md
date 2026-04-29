# Browser Export Filenames Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Improve the browser/web workflow by deriving safe, mode-specific export filenames from imported local files instead of always using generic download names.

**Architecture:** Keep the change inside `mdid-browser` state/helpers. Add a pure filename-sanitizing helper and make `BrowserFlowState::suggested_export_file_name()` return an owned `String` that uses imported file context when available while retaining existing static fallbacks for manual requests.

**Tech Stack:** Rust, Leptos browser crate, cargo tests for `mdid-browser`.

---

## File Structure

- Modify: `crates/mdid-browser/src/app.rs`
  - Add `sanitize_export_stem(input: &str) -> String` helper near browser import helpers.
  - Change `BrowserFlowState::suggested_export_file_name(&self) -> String` to return import-aware filenames.
  - Add tests in the existing `#[cfg(test)] mod tests` module.
- Modify: `README.md`
  - Truth-sync completion snapshot and missing browser workflow item after the landed browser filename UX improvement.

### Task 1: Browser import-aware export filenames

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`
- Test: `crates/mdid-browser/src/app.rs` existing unit-test module

- [ ] **Step 1: Write failing tests**

Add these tests inside the existing `#[cfg(test)] mod tests` in `crates/mdid-browser/src/app.rs`:

```rust
#[test]
fn imported_csv_suggests_sanitized_deidentified_export_name() {
    let mut state = BrowserFlowState::default();
    state.apply_imported_file("Clinic Patient List.csv", "name\nAda", InputMode::CsvText);

    assert_eq!(
        state.suggested_export_file_name(),
        "clinic-patient-list-deidentified.csv"
    );
}

#[test]
fn imported_pdf_suggests_sanitized_review_report_name_without_phi() {
    let mut state = BrowserFlowState::default();
    state.apply_imported_file("../Patient #42 Intake.PDF", "JVBERi0=", InputMode::PdfBase64);

    assert_eq!(
        state.suggested_export_file_name(),
        "patient-42-intake-review-report.txt"
    );
}

#[test]
fn manual_vault_export_keeps_static_portable_artifact_name() {
    let state = BrowserFlowState::default();

    assert_eq!(
        state.suggested_export_file_name(),
        "mdid-browser-output.csv"
    );
}
```

- [ ] **Step 2: Run tests to verify RED**

Run:

```bash
RUSTFLAGS='-C link-arg=-fuse-ld=bfd' cargo test -j1 -p mdid-browser imported_csv_suggests_sanitized_deidentified_export_name imported_pdf_suggests_sanitized_review_report_name_without_phi manual_vault_export_keeps_static_portable_artifact_name -- --nocapture
```

Expected: FAIL because `suggested_export_file_name()` currently returns static filenames and cannot produce import-aware names.

- [ ] **Step 3: Implement minimal filename helper and state method**

Change `BrowserFlowState::suggested_export_file_name()` in `crates/mdid-browser/src/app.rs` to return `String`. Add a helper that uses only ASCII alphanumeric characters, converts other runs to a single `-`, lowercases, trims separators, removes the final extension from the imported file name, and falls back to `mdid-browser-output` when no safe stem remains.

The behavior must map:

```rust
InputMode::CsvText => "{stem}-deidentified.csv"
InputMode::XlsxBase64 => "{stem}-deidentified.xlsx.base64.txt"
InputMode::PdfBase64 => "{stem}-review-report.txt"
InputMode::DicomBase64 => "{stem}-deidentified.dcm.base64.txt"
InputMode::MediaMetadataJson => "{stem}-media-review-report.txt"
InputMode::VaultAuditEvents => "mdid-browser-vault-audit-events.json"
InputMode::VaultDecode => "mdid-browser-vault-decode-response.json"
InputMode::VaultExport => "mdid-browser-portable-artifact.json"
InputMode::PortableArtifactInspect => "mdid-browser-portable-artifact-inspect.txt"
InputMode::PortableArtifactImport => "mdid-browser-portable-artifact-import.txt"
```

Manual/no-import flows must retain the current static names.

- [ ] **Step 4: Run targeted browser tests to verify GREEN**

Run:

```bash
RUSTFLAGS='-C link-arg=-fuse-ld=bfd' cargo test -j1 -p mdid-browser imported_csv_suggests_sanitized_deidentified_export_name imported_pdf_suggests_sanitized_review_report_name_without_phi manual_vault_export_keeps_static_portable_artifact_name -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Run relevant crate tests**

Run:

```bash
RUSTFLAGS='-C link-arg=-fuse-ld=bfd' cargo test -j1 -p mdid-browser -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/mdid-browser/src/app.rs
git commit -m "feat(browser): derive export filenames from imports"
```

### Task 2: README truth-sync for browser filename UX

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update README completion snapshot**

Update the truth-sync line to mention the browser import-aware export filename improvement. Increase Browser/web completion from 61% to 63% and Overall from 86% to 87%. Keep CLI at 93% and Desktop app at 58%. In the missing-items section, replace “file picker/upload-download depth” with “remaining file picker/upload-download depth beyond bounded import-aware export naming”.

- [ ] **Step 2: Run verification commands**

Run:

```bash
RUSTFLAGS='-C link-arg=-fuse-ld=bfd' cargo test -j1 -p mdid-browser -- --nocapture
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add README.md docs/superpowers/plans/2026-04-29-browser-export-filenames.md
git commit -m "docs: truth-sync browser export filename completion"
```
