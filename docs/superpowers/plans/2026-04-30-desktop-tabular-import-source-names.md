# Desktop Tabular Import Source Names Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Preserve the imported CSV/XLSX desktop source filename in workflow request state so existing output-save suggestion logic can produce source-aware default save paths for tabular desktop imports.

**Architecture:** Keep the change in the desktop helper layer only. `DesktopFileImportPayload::from_bytes` already preserves source names for PDF/DICOM/media JSON; extend the same bounded behavior to CSV and XLSX without adding new runtime routes, upload workflows, controller/agent behavior, or PHI-revealing UI output.

**Tech Stack:** Rust workspace, `mdid-desktop` crate, Cargo tests.

---

## File Structure

- Modify `crates/mdid-desktop/src/lib.rs`: change the CSV and XLSX branches in `DesktopFileImportPayload::from_bytes` to set `source_name: Some(source_name)` and add focused regression tests in the existing desktop test module.
- Modify `README.md`: truth-sync completion snapshot and verification evidence after the landed slice.

### Task 1: Preserve desktop CSV/XLSX import source names

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs:137-149`
- Test: `crates/mdid-desktop/src/lib.rs` existing tests module

- [ ] **Step 1: Write failing tests**

Add tests in the existing `#[cfg(test)]` module of `crates/mdid-desktop/src/lib.rs`:

```rust
#[test]
fn desktop_csv_file_import_preserves_source_name_for_save_suggestions() {
    let payload = DesktopFileImportPayload::from_bytes("clinic-export.csv", b"name\nAda\n")
        .expect("csv import should be accepted");

    assert_eq!(payload.mode, DesktopWorkflowMode::CsvText);
    assert_eq!(payload.source_name.as_deref(), Some("clinic-export.csv"));
}

#[test]
fn desktop_xlsx_file_import_preserves_source_name_for_save_suggestions() {
    let payload = DesktopFileImportPayload::from_bytes("clinic-export.xlsx", b"not-real-xlsx")
        .expect("xlsx helper import should accept bytes before runtime validation");

    assert_eq!(payload.mode, DesktopWorkflowMode::XlsxBase64);
    assert_eq!(payload.source_name.as_deref(), Some("clinic-export.xlsx"));
}
```

- [ ] **Step 2: Run tests to verify RED**

Run: `cargo test -p mdid-desktop source_name_for_save_suggestions -- --nocapture`

Expected: FAIL because CSV/XLSX import currently sets `source_name: None`.

- [ ] **Step 3: Write minimal implementation**

Change only the CSV and XLSX branches:

```rust
"csv" => Ok(Self {
    mode: DesktopWorkflowMode::CsvText,
    payload: std::str::from_utf8(bytes)
        .map_err(|_| DesktopFileImportError::InvalidCsvUtf8)?
        .to_string(),
    source_name: Some(source_name),
}),
"xlsx" => Ok(Self {
    mode: DesktopWorkflowMode::XlsxBase64,
    payload: encode_base64(bytes),
    source_name: Some(source_name),
}),
```

- [ ] **Step 4: Run targeted and broader verification**

Run:

```bash
cargo test -p mdid-desktop source_name_for_save_suggestions -- --nocapture
cargo test -p mdid-desktop --lib
cargo fmt --check
git diff --check
```

Expected: all commands PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-desktop/src/lib.rs
git commit -m "fix(desktop): preserve tabular import source names"
```

### Task 2: Truth-sync README completion snapshot

**Files:**
- Modify: `README.md:64-79`

- [ ] **Step 1: Update README snapshot**

Update the completion snapshot to mention the new desktop tabular import source-name preservation and its verification evidence. Keep CLI at 95%, Browser/web at 76%, Desktop app at 70%, Overall at 93%. Explain that overall remains 93% because this removes a bounded desktop file-helper naming gap but does not remove larger blockers.

- [ ] **Step 2: Run documentation verification**

Run:

```bash
git diff --check
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add README.md docs/superpowers/plans/2026-04-30-desktop-tabular-import-source-names.md
git commit -m "docs: truth-sync desktop tabular import source names"
```

## Self-Review

- Spec coverage: Task 1 preserves source names for CSV/XLSX imports; Task 2 updates README completion and evidence.
- Placeholder scan: no TBD/TODO placeholders remain.
- Type consistency: uses existing `DesktopFileImportPayload`, `DesktopWorkflowMode`, and `source_name: Option<String>` fields exactly as defined in the desktop crate.
