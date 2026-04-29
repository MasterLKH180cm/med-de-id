# Browser Binary Output Download Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the browser surface download real rewritten XLSX and DICOM bytes from base64 runtime responses instead of only exporting base64 text files.

**Architecture:** Keep the feature bounded to `mdid-browser` output-export helpers. Add a small pure helper that classifies browser export payloads as UTF-8 text or base64-decoded bytes with an explicit MIME type, then route the wasm download action through text or binary blob creation. Do not add OCR, PDF rewrite, vault browsing, auth/session, agent/controller workflow, or generalized orchestration behavior.

**Tech Stack:** Rust, Leptos, wasm-bindgen/web-sys, base64, Cargo tests/clippy.

---

## File Structure

- Modify: `crates/mdid-browser/src/app.rs`
  - Add an `BrowserDownloadPayload` enum for text vs binary downloads.
  - Add `BrowserFlowState::prepared_download_payload()` to decode XLSX/DICOM base64 output and keep CSV/PDF/media/vault/portable responses as text.
  - Add wasm-only binary blob download helper while preserving existing text helper.
  - Update export button handler to use the prepared payload.
  - Add focused tests for XLSX/DICOM binary output, CSV text output, and invalid base64 error handling.
- Modify: `README.md`
  - Truth-sync completion snapshot and browser/web current-state bullets after landed feature and verification.

### Task 1: Browser binary output download helper

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`

- [ ] **Step 1: Write failing tests**

Add these tests inside the existing `#[cfg(test)] mod tests` in `crates/mdid-browser/src/app.rs`:

```rust
#[test]
fn xlsx_output_download_decodes_base64_to_binary_payload() {
    let mut state = BrowserFlowState::default();
    state.input_mode = InputMode::XlsxBase64;
    state.result_output = base64::engine::general_purpose::STANDARD.encode(b"workbook-bytes");

    let payload = state.prepared_download_payload().expect("xlsx payload");

    assert_eq!(payload.file_name, "mdid-browser-output.xlsx");
    assert_eq!(payload.mime_type, "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet");
    assert_eq!(payload.bytes, b"workbook-bytes");
    assert!(!payload.is_text);
}

#[test]
fn dicom_output_download_decodes_base64_to_binary_payload() {
    let mut state = BrowserFlowState::default();
    state.input_mode = InputMode::DicomBase64;
    state.imported_file_name = Some("CT Head.dcm".to_string());
    state.result_output = base64::engine::general_purpose::STANDARD.encode(b"dicom-bytes");

    let payload = state.prepared_download_payload().expect("dicom payload");

    assert_eq!(payload.file_name, "ct-head-deidentified.dcm");
    assert_eq!(payload.mime_type, "application/dicom");
    assert_eq!(payload.bytes, b"dicom-bytes");
    assert!(!payload.is_text);
}

#[test]
fn csv_output_download_keeps_text_payload() {
    let mut state = BrowserFlowState::default();
    state.input_mode = InputMode::CsvText;
    state.result_output = "patient_id\nMDID-1\n".to_string();

    let payload = state.prepared_download_payload().expect("csv payload");

    assert_eq!(payload.file_name, "mdid-browser-output.csv");
    assert_eq!(payload.mime_type, "text/plain;charset=utf-8");
    assert_eq!(payload.bytes, b"patient_id\nMDID-1\n");
    assert!(payload.is_text);
}

#[test]
fn binary_output_download_rejects_invalid_base64() {
    let mut state = BrowserFlowState::default();
    state.input_mode = InputMode::XlsxBase64;
    state.result_output = "not valid base64".to_string();

    let error = state.prepared_download_payload().expect_err("invalid base64 should fail");

    assert_eq!(
        error,
        "Browser output download could not decode rewritten XLSX base64 bytes."
    );
}
```

- [ ] **Step 2: Run tests to verify RED**

Run: `cargo test -p mdid-browser xlsx_output_download_decodes_base64_to_binary_payload dicom_output_download_decodes_base64_to_binary_payload csv_output_download_keeps_text_payload binary_output_download_rejects_invalid_base64 -- --exact`

Expected: FAIL because `prepared_download_payload` and `BrowserDownloadPayload` do not exist.

- [ ] **Step 3: Implement minimal helper and export route**

In `crates/mdid-browser/src/app.rs`, add:

```rust
use base64::Engine;

#[derive(Debug, Clone, PartialEq, Eq)]
struct BrowserDownloadPayload {
    file_name: String,
    mime_type: &'static str,
    bytes: Vec<u8>,
    is_text: bool,
}
```

Update `BrowserFlowState::suggested_export_file_name()` so XLSX/DICOM binary outputs end in real binary extensions:

```rust
InputMode::XlsxBase64 => return format!("{stem}-deidentified.xlsx"),
InputMode::DicomBase64 => return format!("{stem}-deidentified.dcm"),
```

and defaults:

```rust
InputMode::XlsxBase64 => "mdid-browser-output.xlsx",
InputMode::DicomBase64 => "mdid-browser-output.dcm",
```

Add this method in `impl BrowserFlowState`:

```rust
fn prepared_download_payload(&self) -> Result<BrowserDownloadPayload, String> {
    let file_name = self.suggested_export_file_name();
    match self.input_mode {
        InputMode::XlsxBase64 => {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(self.result_output.trim())
                .map_err(|_| "Browser output download could not decode rewritten XLSX base64 bytes.".to_string())?;
            Ok(BrowserDownloadPayload {
                file_name,
                mime_type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                bytes,
                is_text: false,
            })
        }
        InputMode::DicomBase64 => {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(self.result_output.trim())
                .map_err(|_| "Browser output download could not decode rewritten DICOM base64 bytes.".to_string())?;
            Ok(BrowserDownloadPayload {
                file_name,
                mime_type: "application/dicom",
                bytes,
                is_text: false,
            })
        }
        _ => Ok(BrowserDownloadPayload {
            file_name,
            mime_type: "text/plain;charset=utf-8",
            bytes: self.result_output.as_bytes().to_vec(),
            is_text: true,
        }),
    }
}
```

Add a wasm binary download helper and non-wasm stub:

```rust
#[cfg(target_arch = "wasm32")]
fn trigger_browser_download(payload: &BrowserDownloadPayload) -> Result<(), String> {
    if payload.is_text {
        let text = std::str::from_utf8(&payload.bytes)
            .map_err(|_| "Browser text export payload was not valid UTF-8.".to_string())?;
        return trigger_browser_text_download(&payload.file_name, text);
    }
    trigger_browser_binary_download(&payload.file_name, &payload.bytes, payload.mime_type)
}

#[cfg(not(target_arch = "wasm32"))]
fn trigger_browser_download(_payload: &BrowserDownloadPayload) -> Result<(), String> {
    Err(FETCH_UNAVAILABLE_MESSAGE.to_string())
}
```

Implement `trigger_browser_binary_download` on wasm using `js_sys::Uint8Array`, `Blob`, `Url`, and an anchor exactly like the text helper, but with `Blob::new_with_u8_array_sequence_and_options` and `options.set_type(mime_type)`.

Update the export click handler to call `state.prepared_download_payload()` and pass the payload to `trigger_browser_download`.

- [ ] **Step 4: Run targeted tests to verify GREEN**

Run: `cargo test -p mdid-browser xlsx_output_download_decodes_base64_to_binary_payload dicom_output_download_decodes_base64_to_binary_payload csv_output_download_keeps_text_payload binary_output_download_rejects_invalid_base64 -- --exact`

Expected: PASS.

- [ ] **Step 5: Run broader browser verification**

Run: `cargo test -p mdid-browser`

Expected: PASS.

Run: `cargo clippy -p mdid-browser --all-targets -- -D warnings`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/mdid-browser/src/app.rs
git commit -m "feat(browser): download rewritten binary outputs"
```

### Task 2: README truth-sync for browser binary downloads

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update README completion snapshot**

Update the snapshot date/evidence to mention bounded browser binary downloads for XLSX/DICOM, increase Browser/web only if honestly justified by the landed tests, and keep missing blockers explicit. Suggested truthful values after Task 1: CLI 95%, Browser/web 68%, Desktop app 62%, Overall 90%.

- [ ] **Step 2: Run README grep verification**

Run: `grep -n "Completion snapshot\|Browser/web\|Desktop app\|Overall\|binary" README.md`

Expected: output includes browser binary download truth-sync and updated percentages.

- [ ] **Step 3: Commit**

```bash
git add README.md docs/superpowers/plans/2026-04-29-browser-binary-output-download.md
git commit -m "docs: truth-sync browser binary downloads completion"
```

## Self-Review

- Spec coverage: covers browser real download depth for the existing rewritten XLSX/DICOM runtime outputs without claiming PDF/OCR/media rewrite or vault browsing.
- Placeholder scan: no TBD/TODO/implement-later placeholders.
- Type consistency: `BrowserDownloadPayload`, `prepared_download_payload`, and export handler naming are consistent across tasks.
