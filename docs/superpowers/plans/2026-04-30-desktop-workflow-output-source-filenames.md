# Desktop Workflow Output Source Filenames Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make desktop rewritten CSV/XLSX/DICOM output save suggestions use safe source-derived filenames when an imported source name is available.

**Architecture:** Keep the change in the existing desktop helper layer and app refresh path. Reuse the existing `safe_source_file_stem` sanitizer so source-derived names never include paths and fall back to static PHI-safe defaults when the source name is blank or unsafe.

**Tech Stack:** Rust workspace, `mdid-desktop`, Cargo tests, existing `DesktopWorkflowResponseState` helpers.

---

## File Structure

- Modify `crates/mdid-desktop/src/lib.rs`: add a source-aware workflow output download helper that wraps the existing CSV/XLSX/DICOM download behavior and only changes the suggested filename.
- Modify `crates/mdid-desktop/src/main.rs`: use the source-aware helper in `DesktopApp::refresh_workflow_output_save_path` and add regression coverage in the existing `#[cfg(test)]` module.
- Modify `README.md`: truth-sync desktop/browser/CLI/overall completion snapshot and verification evidence after this slice lands.
- Modify this plan: mark completed task checkboxes as work lands.

### Task 1: Source-aware desktop workflow output suggestions

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`
- Modify: `crates/mdid-desktop/src/main.rs`
- Test: existing unit tests in `crates/mdid-desktop/src/main.rs` and `crates/mdid-desktop/src/lib.rs`

- [x] **Step 1: Write the failing app-level regression test**

Add this test to the existing `#[cfg(test)] mod tests` in `crates/mdid-desktop/src/main.rs` near the workflow output save path tests:

```rust
#[test]
fn workflow_output_save_path_uses_import_source_stem_after_dicom_success() {
    let mut app = DesktopApp::default();
    app.request_state.mode = DesktopWorkflowMode::DicomBase64;
    app.request_state.source_name = "C:\\scanner exports\\Patient One Scan.dcm".to_string();
    app.response_state.apply_success_json(
        DesktopWorkflowMode::DicomBase64,
        serde_json::json!({
            "rewritten_dicom_bytes_base64": base64::engine::general_purpose::STANDARD.encode(b"dicom"),
            "summary": {}
        }),
    );

    app.refresh_workflow_output_save_path(DesktopWorkflowMode::DicomBase64);

    assert_eq!(app.workflow_output_save_path, "Patient-One-Scan-deidentified.dcm");
    assert_eq!(
        app.generated_workflow_output_save_path.as_deref(),
        Some("Patient-One-Scan-deidentified.dcm")
    );
}
```

- [x] **Step 2: Run the targeted test and verify RED**

Run:

```bash
cargo test -p mdid-desktop workflow_output_save_path_uses_import_source_stem_after_dicom_success -- --nocapture
```

Expected: FAIL because the current app refresh path still suggests `desktop-deidentified.dcm` instead of `Patient-One-Scan-deidentified.dcm`.

- [x] **Step 3: Add the source-aware helper and use it from the app refresh path**

In `crates/mdid-desktop/src/lib.rs`, add this method inside `impl DesktopWorkflowResponseState` after `workflow_output_download`:

```rust
    pub fn workflow_output_download_for_source(
        &self,
        mode: DesktopWorkflowMode,
        source_name: Option<&str>,
    ) -> Option<DesktopWorkflowOutputDownload> {
        let mut download = self.workflow_output_download(mode)?;
        let stem = source_name.and_then(safe_source_file_stem)?;
        let extension = match mode {
            DesktopWorkflowMode::CsvText => "csv",
            DesktopWorkflowMode::XlsxBase64 => "xlsx",
            DesktopWorkflowMode::DicomBase64 => "dcm",
            DesktopWorkflowMode::PdfBase64Review | DesktopWorkflowMode::MediaMetadataJson => return None,
        };
        download.file_name = format!("{stem}-deidentified.{extension}");
        Some(download)
    }
```

In `crates/mdid-desktop/src/main.rs`, change `refresh_workflow_output_save_path` so the `next_path` calculation calls the new helper first:

```rust
        let next_path = self
            .response_state
            .workflow_output_download_for_source(mode, Some(self.request_state.source_name.trim()))
            .or_else(|| self.response_state.workflow_output_download(mode))
            .map(|download| download.file_name.to_string())
            .unwrap_or_else(|| DEFAULT_WORKFLOW_OUTPUT_SAVE_PATH.to_string());
```

- [x] **Step 4: Run targeted test and broader desktop tests**

Run:

```bash
cargo test -p mdid-desktop workflow_output_save_path_uses_import_source_stem_after_dicom_success -- --nocapture
cargo test -p mdid-desktop workflow_output_save_path -- --nocapture
cargo test -p mdid-desktop --lib
```

Expected: all commands PASS.

- [x] **Step 5: Format and commit implementation**

Run:

```bash
cargo fmt --check
git diff --check
git add crates/mdid-desktop/src/lib.rs crates/mdid-desktop/src/main.rs docs/superpowers/plans/2026-04-30-desktop-workflow-output-source-filenames.md
git commit -m "fix(desktop): use source names for workflow output saves"
```

Expected: format/diff checks PASS and commit is created.

### Task 2: README truth-sync for desktop workflow output source filenames

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/plans/2026-04-30-desktop-workflow-output-source-filenames.md`

- [x] **Step 1: Update README completion snapshot**

Edit `README.md` Current repository status so it says this round truth-synced after SDD-reviewed desktop workflow output source-name save suggestions. Keep CLI at 95%, Browser/web at 76%, Desktop app at 70%, and Overall at 93% unless a broader blocker is actually closed. Mention that the desktop app now uses sanitized imported source stems for rewritten CSV/XLSX/DICOM save suggestions when available and still falls back to static PHI-safe defaults.

- [x] **Step 2: Run docs/format verification**

Run:

```bash
cargo test -p mdid-desktop workflow_output_save_path -- --nocapture
cargo fmt --check
git diff --check
```

Expected: all commands PASS.

- [x] **Step 3: Commit README truth-sync**

Run:

```bash
git add README.md docs/superpowers/plans/2026-04-30-desktop-workflow-output-source-filenames.md
git commit -m "docs: truth-sync desktop workflow output filenames"
```

Expected: commit is created.

## Self-Review

1. Spec coverage: Task 1 adds source-aware desktop rewritten-output save suggestions and tests the DICOM path; existing CSV/XLSX behavior continues through the same helper. Task 2 updates README completion evidence.
2. Placeholder scan: no TBD/TODO/implement later placeholders remain.
3. Type consistency: method names are consistently `workflow_output_download_for_source`, mode names match existing `DesktopWorkflowMode` variants, and filenames use the existing `safe_source_file_stem` sanitizer.
