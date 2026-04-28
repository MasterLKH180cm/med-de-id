# Desktop File Import Export Helpers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add bounded desktop helper logic for local CSV/XLSX/PDF file import payload preparation and safe output export naming without adding a generalized workflow platform.

**Architecture:** Keep the slice in `mdid-desktop` library code so behavior is testable without a GUI harness. The helpers map file names and byte/text payloads onto the existing three desktop modes, enforce bounded size/type rules, and derive honest export suggestions from already-rendered runtime response state.

**Tech Stack:** Rust workspace, `mdid-desktop`, unit tests with `cargo test -p mdid-desktop`.

---

## File Structure

- Modify: `crates/mdid-desktop/src/lib.rs`
  - Add `DesktopFileImportPayload`, `DesktopFileImportError`, import size/type helpers, `DesktopWorkflowRequestState::apply_imported_file`, and response export helper methods.
  - Update disclosure/status copy to remove stale “file picker upload/download UX” missing claim once bounded helpers exist.
- Modify: `README.md`
  - Truth-sync desktop/browser/overall completion rows and remaining missing items after landed tests.

### Task 1: Desktop import payload helpers

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`
- Test: inline `#[cfg(test)]` module in `crates/mdid-desktop/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Add tests to the existing test module:

```rust
#[test]
fn desktop_file_import_maps_csv_xlsx_and_pdf_payloads_to_existing_modes() {
    let csv = DesktopFileImportPayload::from_file_bytes("patients.csv", b"patient_name\nAlice").unwrap();
    assert_eq!(csv.file_name, "patients.csv");
    assert_eq!(csv.mode, DesktopWorkflowMode::CsvText);
    assert_eq!(csv.payload, "patient_name\nAlice");
    assert_eq!(csv.source_name, None);

    let xlsx = DesktopFileImportPayload::from_file_bytes("patients.xlsx", &[0x50, 0x4b, 0x03, 0x04]).unwrap();
    assert_eq!(xlsx.mode, DesktopWorkflowMode::XlsxBase64);
    assert_eq!(xlsx.payload, "UEsDBA==");

    let pdf = DesktopFileImportPayload::from_file_bytes("chart.PDF", b"%PDF-1.7").unwrap();
    assert_eq!(pdf.mode, DesktopWorkflowMode::PdfBase64Review);
    assert_eq!(pdf.payload, "JVBERi0xLjc=");
    assert_eq!(pdf.source_name.as_deref(), Some("chart.PDF"));
}

#[test]
fn desktop_file_import_rejects_unknown_large_and_non_utf8_csv_files() {
    assert_eq!(
        DesktopFileImportPayload::from_file_bytes("notes.txt", b"hello"),
        Err(DesktopFileImportError::UnsupportedFileType)
    );
    assert_eq!(
        DesktopFileImportPayload::from_file_bytes("bad.csv", &[0xff, 0xfe]),
        Err(DesktopFileImportError::InvalidCsvUtf8)
    );
    let oversized = vec![b'a'; DESKTOP_FILE_IMPORT_MAX_BYTES + 1];
    assert_eq!(
        DesktopFileImportPayload::from_file_bytes("large.csv", &oversized),
        Err(DesktopFileImportError::FileTooLarge)
    );
}

#[test]
fn desktop_request_state_applies_imported_payload_without_changing_policy_json() {
    let mut state = DesktopWorkflowRequestState::default();
    state.field_policy_json = r#"[{"header":"patient_name","phi_type":"Name","action":"review"}]"#.to_string();
    let imported = DesktopFileImportPayload::from_file_bytes("chart.pdf", b"%PDF-1.7").unwrap();

    state.apply_imported_file(imported);

    assert_eq!(state.mode, DesktopWorkflowMode::PdfBase64Review);
    assert_eq!(state.payload, "JVBERi0xLjc=");
    assert_eq!(state.source_name, "chart.pdf");
    assert!(state.field_policy_json.contains("patient_name"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p mdid-desktop desktop_file_import -- --nocapture`

Expected: FAIL with unresolved `DesktopFileImportPayload` / `DesktopFileImportError` / `DESKTOP_FILE_IMPORT_MAX_BYTES`.

- [ ] **Step 3: Write minimal implementation**

Add near the request-state types:

```rust
pub const DESKTOP_FILE_IMPORT_MAX_BYTES: usize = 10 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopFileImportPayload {
    pub file_name: String,
    pub mode: DesktopWorkflowMode,
    pub payload: String,
    pub source_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopFileImportError {
    UnsupportedFileType,
    FileTooLarge,
    InvalidCsvUtf8,
}

impl DesktopFileImportPayload {
    pub fn from_file_bytes(file_name: &str, bytes: &[u8]) -> Result<Self, DesktopFileImportError> {
        if bytes.len() > DESKTOP_FILE_IMPORT_MAX_BYTES {
            return Err(DesktopFileImportError::FileTooLarge);
        }
        let lower_name = file_name.to_ascii_lowercase();
        if lower_name.ends_with(".csv") {
            let payload = std::str::from_utf8(bytes)
                .map_err(|_| DesktopFileImportError::InvalidCsvUtf8)?
                .to_string();
            return Ok(Self { file_name: file_name.to_string(), mode: DesktopWorkflowMode::CsvText, payload, source_name: None });
        }
        if lower_name.ends_with(".xlsx") {
            return Ok(Self { file_name: file_name.to_string(), mode: DesktopWorkflowMode::XlsxBase64, payload: base64_encode(bytes), source_name: None });
        }
        if lower_name.ends_with(".pdf") {
            return Ok(Self { file_name: file_name.to_string(), mode: DesktopWorkflowMode::PdfBase64Review, payload: base64_encode(bytes), source_name: Some(file_name.to_string()) });
        }
        Err(DesktopFileImportError::UnsupportedFileType)
    }
}

impl DesktopWorkflowRequestState {
    pub fn apply_imported_file(&mut self, imported: DesktopFileImportPayload) {
        self.mode = imported.mode;
        self.payload = imported.payload;
        if let Some(source_name) = imported.source_name {
            self.source_name = source_name;
        }
    }
}

fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::new();
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        output.push(TABLE[(b0 >> 2) as usize] as char);
        output.push(TABLE[(((b0 & 0b0000_0011) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            output.push(TABLE[(((b1 & 0b0000_1111) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            output.push('=');
        }
        if chunk.len() > 2 {
            output.push(TABLE[(b2 & 0b0011_1111) as usize] as char);
        } else {
            output.push('=');
        }
    }
    output
}
```

Also update `status_message` copy so it says bounded file import/export helpers are present while still excluding OCR, visual redaction, PDF rewrite/export, vault/decode/audit workflow, and full review workflow.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p mdid-desktop desktop_file_import -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Run crate tests**

Run: `cargo test -p mdid-desktop`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/mdid-desktop/src/lib.rs
git commit -m "feat(desktop): add bounded file import helpers"
```

### Task 2: Desktop response export helpers and README truth-sync

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`
- Modify: `README.md`
- Test: inline `#[cfg(test)]` module in `crates/mdid-desktop/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Add tests:

```rust
#[test]
fn response_state_suggests_exports_only_when_output_bytes_exist() {
    let mut csv = DesktopWorkflowResponseState::default();
    csv.apply_success_json(
        DesktopWorkflowMode::CsvText,
        json!({"csv":"patient_name\n<NAME-1>","summary":{},"review_queue":[]}),
    );
    assert_eq!(csv.suggested_export_file_name(DesktopWorkflowMode::CsvText), Some("desktop-deidentified.csv"));
    assert_eq!(csv.exportable_output(), Some("patient_name\n<NAME-1>"));

    let mut pdf = DesktopWorkflowResponseState::default();
    pdf.apply_success_json(
        DesktopWorkflowMode::PdfBase64Review,
        json!({"rewritten_pdf_bytes_base64":null,"summary":{},"review_queue":[]}),
    );
    assert_eq!(pdf.suggested_export_file_name(DesktopWorkflowMode::PdfBase64Review), None);
    assert_eq!(pdf.exportable_output(), None);
}

#[test]
fn response_state_suggests_xlsx_export_for_rewritten_workbook_base64() {
    let mut xlsx = DesktopWorkflowResponseState::default();
    xlsx.apply_success_json(
        DesktopWorkflowMode::XlsxBase64,
        json!({"rewritten_workbook_base64":"UEsDBAo=","summary":{},"review_queue":[]}),
    );
    assert_eq!(xlsx.suggested_export_file_name(DesktopWorkflowMode::XlsxBase64), Some("desktop-deidentified.xlsx.base64.txt"));
    assert_eq!(xlsx.exportable_output(), Some("UEsDBAo="));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p mdid-desktop response_state_suggests -- --nocapture`

Expected: FAIL with missing methods.

- [ ] **Step 3: Write minimal implementation**

Add methods to `impl DesktopWorkflowResponseState`:

```rust
pub fn exportable_output(&self) -> Option<&str> {
    let output = self.output.trim();
    if output.is_empty() || output == "No rewritten PDF bytes returned by the bounded review route." {
        None
    } else {
        Some(self.output.as_str())
    }
}

pub fn suggested_export_file_name(&self, mode: DesktopWorkflowMode) -> Option<&'static str> {
    self.exportable_output()?;
    match mode {
        DesktopWorkflowMode::CsvText => Some("desktop-deidentified.csv"),
        DesktopWorkflowMode::XlsxBase64 => Some("desktop-deidentified.xlsx.base64.txt"),
        DesktopWorkflowMode::PdfBase64Review => None,
    }
}
```

Update README completion rows to reflect the bounded desktop import/export helper slice: desktop app +3 points if tests pass, overall +1 point if controller-visible verification passes. Keep missing items honest and do not claim OCR/PDF rewrite/full workflows.

- [ ] **Step 4: Run tests**

Run: `cargo test -p mdid-desktop response_state_suggests -- --nocapture && cargo test -p mdid-desktop`

Expected: PASS.

- [ ] **Step 5: README verification**

Run: `grep -n "Desktop app\|Overall\|Missing items" README.md`

Expected: Desktop app row mentions bounded CSV/XLSX/PDF file import/export helpers; Overall row remains below 90% unless broader landed functionality justifies more.

- [ ] **Step 6: Commit**

```bash
git add crates/mdid-desktop/src/lib.rs README.md docs/superpowers/plans/2026-04-28-desktop-file-import-export-helpers.md
git commit -m "feat(desktop): add bounded output export helpers"
```

## Self-Review

- Spec coverage: Covers desktop file import payload preparation and output export helper suggestions only; does not implement OCR, visual redaction, PDF rewrite/export, vault/decode/audit workflow, or agent/controller features.
- Placeholder scan: No TBD/TODO/fill-in placeholders remain.
- Type consistency: `DesktopWorkflowMode`, `DesktopWorkflowRequestState`, and `DesktopWorkflowResponseState` names match existing code.
