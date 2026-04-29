# Desktop Workflow Output Save Implementation Plan

> **For implementation workers:** REQUIRED SUB-SKILL: Use subagent-driven-development or execute this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded desktop helper that saves already-received rewritten CSV/XLSX/DICOM runtime outputs to local files without exposing PHI-bearing runtime envelopes or paths in UI/error text.

**Architecture:** Keep the desktop layer thin: response state parses existing runtime-shaped success envelopes, extracts only the rewritten artifact bytes for modes that truly return rewritten artifacts, and a file-writing helper persists those bytes. PDF review remains review-only and returns no saveable rewritten artifact.

**Tech Stack:** Rust workspace, `mdid-desktop`, serde_json, base64 decode via the crate's existing dependencies, Rust unit tests.

---

## File Structure

- Modify: `crates/mdid-desktop/src/lib.rs`
  - Add a small `DesktopWorkflowOutputDownload` value carrying `file_name` and `bytes` with redacted `Debug`.
  - Add `DesktopWorkflowResponseState::workflow_output_download(...)` that fail-closed extracts CSV text, XLSX base64, or DICOM base64 from the latest runtime success for the matching workflow mode.
  - Add `write_workflow_output_file(...)` that writes extracted bytes and returns PHI-safe generic errors.
- Modify: `crates/mdid-desktop/Cargo.toml`
  - Use the workspace `base64` dependency for strict standard base64 decoding of downloaded XLSX/DICOM outputs.
- Modify: `crates/mdid-desktop/src/main.rs`
  - Add app-level helper around `workflow_output_download(...)` + `write_workflow_output_file(...)` for tests and future UI wiring.
  - Keep UI copy narrow: bounded helper save only; no full file picker/upload-download workflow claims.
- Modify: `README.md`
  - Truth-sync completion snapshot based on the landed helper and verification. Desktop may increase modestly only because this is helper-layer save support, not full workflow UX.
- Test: existing inline tests in `crates/mdid-desktop/src/lib.rs` and `crates/mdid-desktop/src/main.rs`.

### Task 1: Desktop rewritten workflow output extraction helper

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`

- [x] **Step 1: Write failing tests**

Add tests equivalent to:

```rust
#[test]
fn workflow_output_download_extracts_csv_bytes_without_raw_envelope() {
    let mut state = DesktopWorkflowResponseState::default();
    state.apply_success(
        DesktopWorkflowMode::CsvText,
        r#"{"csv":"name\nTOKEN-1\n","summary":{"total_rows":1},"review_queue":[]}"#,
    );

    let download = state
        .workflow_output_download(DesktopWorkflowMode::CsvText)
        .expect("csv download");

    assert_eq!(download.file_name, "desktop-deidentified.csv");
    assert_eq!(download.bytes, b"name\nTOKEN-1\n");
    let debug = format!("{download:?}");
    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("TOKEN-1"));
}

#[test]
fn workflow_output_download_extracts_xlsx_and_dicom_base64_bytes() {
    let mut xlsx_state = DesktopWorkflowResponseState::default();
    xlsx_state.apply_success(
        DesktopWorkflowMode::XlsxBase64,
        r#"{"rewritten_workbook_base64":"V09SS0JPT0s=","summary":{},"review_queue":[]}"#,
    );
    let xlsx = xlsx_state
        .workflow_output_download(DesktopWorkflowMode::XlsxBase64)
        .expect("xlsx download");
    assert_eq!(xlsx.file_name, "desktop-deidentified.xlsx");
    assert_eq!(xlsx.bytes, b"WORKBOOK");

    let mut dicom_state = DesktopWorkflowResponseState::default();
    dicom_state.apply_success(
        DesktopWorkflowMode::DicomBase64,
        r#"{"rewritten_dicom_bytes_base64":"RElDT00=","summary":{},"review_queue":[]}"#,
    );
    let dicom = dicom_state
        .workflow_output_download(DesktopWorkflowMode::DicomBase64)
        .expect("dicom download");
    assert_eq!(dicom.file_name, "desktop-deidentified.dcm");
    assert_eq!(dicom.bytes, b"DICOM");
}

#[test]
fn workflow_output_download_fails_closed_for_pdf_errors_malformed_and_mode_mismatch() {
    let mut state = DesktopWorkflowResponseState::default();
    state.apply_success(
        DesktopWorkflowMode::PdfBase64Review,
        r#"{"rewritten_pdf_bytes_base64":null,"summary":{},"review_queue":[]}"#,
    );
    assert!(state.workflow_output_download(DesktopWorkflowMode::PdfBase64Review).is_none());

    let mut csv_state = DesktopWorkflowResponseState::default();
    csv_state.apply_success(
        DesktopWorkflowMode::CsvText,
        r#"{"csv":"name\nTOKEN-1\n","summary":{},"review_queue":[]}"#,
    );
    assert!(csv_state.workflow_output_download(DesktopWorkflowMode::XlsxBase64).is_none());

    let mut malformed_state = DesktopWorkflowResponseState::default();
    malformed_state.apply_success(
        DesktopWorkflowMode::XlsxBase64,
        r#"{"rewritten_workbook_base64":"not base64%%%","summary":{},"review_queue":[]}"#,
    );
    assert!(malformed_state.workflow_output_download(DesktopWorkflowMode::XlsxBase64).is_none());
}
```

- [x] **Step 2: Run tests to verify RED**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-desktop workflow_output_download -- --nocapture`

Expected: FAIL because `workflow_output_download` and/or `DesktopWorkflowOutputDownload` does not exist.

- [x] **Step 3: Implement minimal helper**

Implement:

```rust
#[derive(Clone, PartialEq, Eq)]
pub struct DesktopWorkflowOutputDownload {
    pub file_name: String,
    pub bytes: Vec<u8>,
}

impl std::fmt::Debug for DesktopWorkflowOutputDownload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DesktopWorkflowOutputDownload")
            .field("file_name", &self.file_name)
            .field("bytes", &"<redacted>")
            .finish()
    }
}
```

Add stored response provenance if not already present; `workflow_output_download(mode)` must return `None` unless the stored successful response provenance matches `mode`. Parse the stored success JSON with `serde_json::Value` and extract:

- CSV: `csv` string → `desktop-deidentified.csv` UTF-8 bytes
- XLSX: `rewritten_workbook_base64` string → `desktop-deidentified.xlsx` decoded bytes
- DICOM: `rewritten_dicom_bytes_base64` string → `desktop-deidentified.dcm` decoded bytes
- PDF/media metadata/anything else: `None`

- [x] **Step 4: Verify GREEN**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-desktop workflow_output_download -- --nocapture`

Expected: PASS.

Quality-fix evidence (2026-04-29): added a regression case for non-canonical padded DICOM base64 `/x==`, verified RED against the permissive local decoder, then switched download decoding to `base64::engine::general_purpose::STANDARD.decode(...)` via the workspace `base64` crate. GREEN: `cargo test -p mdid-desktop workflow_output_download_fails_closed_for_pdf_errors_malformed_and_mode_mismatch`, `cargo test -p mdid-desktop workflow_output_download`, and `cargo test -p mdid-desktop` all passed.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-desktop/src/lib.rs docs/superpowers/plans/2026-04-29-desktop-workflow-output-save.md
git commit -m "feat(desktop): extract rewritten workflow outputs for save"
```

### Task 2: Desktop PHI-safe workflow output file writing

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`
- Modify: `crates/mdid-desktop/src/main.rs`

- [ ] **Step 1: Write failing tests**

Add tests equivalent to:

```rust
#[test]
fn write_workflow_output_file_writes_bytes_without_exposing_phi_path() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("Patient-Jane-Doe-output.csv");
    let download = DesktopWorkflowOutputDownload {
        file_name: "desktop-deidentified.csv".to_string(),
        bytes: b"tokenized\n".to_vec(),
    };

    write_workflow_output_file(&path, &download).expect("write output");

    assert_eq!(std::fs::read(&path).expect("read output"), b"tokenized\n");
}

#[test]
fn write_workflow_output_file_error_is_phi_safe() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("Patient-Jane-Doe-directory");
    std::fs::create_dir(&path).expect("directory");
    let download = DesktopWorkflowOutputDownload {
        file_name: "desktop-deidentified.csv".to_string(),
        bytes: b"tokenized\n".to_vec(),
    };

    let error = write_workflow_output_file(&path, &download).expect_err("write should fail");

    assert_eq!(error, "workflow output save failed: unable to write output file");
    assert!(!error.contains("Patient-Jane-Doe"));
}
```

In `main.rs`, add an app helper test equivalent to:

```rust
#[test]
fn app_save_workflow_output_writes_latest_csv_output() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("Patient-Jane-Doe-output.csv");
    let mut app = MedDeIdDesktopApp::new();
    app.response_state.apply_success(
        mdid_desktop::DesktopWorkflowMode::CsvText,
        r#"{"csv":"name\nTOKEN-1\n","summary":{},"review_queue":[]}"#,
    );

    app.save_workflow_output(&path).expect("save output");

    assert_eq!(std::fs::read(&path).expect("read output"), b"name\nTOKEN-1\n");
}
```

- [ ] **Step 2: Run tests to verify RED**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-desktop workflow_output_file -- --nocapture`

Expected: FAIL because `write_workflow_output_file` and/or app save helper does not exist.

- [ ] **Step 3: Implement minimal file-writing helper**

Implement `pub fn write_workflow_output_file(path: impl AsRef<std::path::Path>, download: &DesktopWorkflowOutputDownload) -> Result<(), String>` with `std::fs::write(path, &download.bytes)` and map any error to exactly `workflow output save failed: unable to write output file`.

In `main.rs`, add a narrow helper on `MedDeIdDesktopApp`:

```rust
fn save_workflow_output(&self, path: impl AsRef<std::path::Path>) -> Result<(), String> {
    let download = self
        .response_state
        .workflow_output_download(self.request_state.mode)
        .ok_or_else(|| "workflow output save failed: no rewritten output is available".to_string())?;
    mdid_desktop::write_workflow_output_file(path, &download)
}
```

Do not add raw path display or broad file-picker workflow claims.

- [ ] **Step 4: Verify GREEN and broader desktop tests**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-desktop workflow_output -- --nocapture
cargo test -p mdid-desktop
cargo clippy -p mdid-desktop --all-targets -- -D warnings
git diff --check
```

Expected: all PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-desktop/src/lib.rs crates/mdid-desktop/src/main.rs docs/superpowers/plans/2026-04-29-desktop-workflow-output-save.md
git commit -m "feat(desktop): save rewritten workflow outputs safely"
```

### Task 3: README truth-sync and integration verification

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/plans/2026-04-29-desktop-workflow-output-save.md`

- [ ] **Step 1: Update README completion snapshot**

Update the completion snapshot to credit only landed helper-layer desktop output save support. Suggested truthful numbers after successful verification:

- CLI: `95%` unchanged
- Browser/web: `63%` unchanged
- Desktop app: `62%`
- Overall: `89%`

Keep missing items explicit: full desktop file picker/save UI depth, richer browser/desktop workflows, OCR/visual redaction, deeper vault UX, and full portable transfer workflow UX still block >=95%.

- [ ] **Step 2: Mark this plan complete**

Mark completed checkboxes or add a completion evidence section with the actual commits and commands run.

- [ ] **Step 3: Verify docs and full affected surface**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-desktop
cargo clippy -p mdid-desktop --all-targets -- -D warnings
git diff --check
grep -n "Completion snapshot\|CLI | 95%\|Browser/web | 63%\|Desktop app | 62%\|Overall | 89%" README.md
```

Expected: all PASS and grep shows the updated snapshot.

- [ ] **Step 4: Commit**

```bash
git add README.md docs/superpowers/plans/2026-04-29-desktop-workflow-output-save.md
git commit -m "docs: truth-sync desktop output save completion"
```

## Self-Review

- Spec coverage: The plan implements a bounded desktop save helper for actual rewritten CSV/XLSX/DICOM outputs, keeps PDF review-only, and updates README completion truthfully.
- Placeholder scan: No TBD/TODO/implement-later placeholders remain.
- Type consistency: The plan consistently uses `DesktopWorkflowOutputDownload`, `workflow_output_download`, and `write_workflow_output_file`.
