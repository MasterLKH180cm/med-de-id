# Browser Tabular Report Download Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an explicit browser-side structured JSON report download for successful CSV/XLSX tabular de-identification responses, separate from rewritten data downloads.

**Architecture:** Keep the browser tool local-first and bounded: runtime responses are already rendered into `summary`, `review_queue`, and `result_output`; this slice adds a second PHI-safe report payload for tabular modes without changing runtime APIs. The rewritten CSV/XLSX output download remains unchanged, and the new report helper is only available after successful tabular output exists.

**Tech Stack:** Rust, Yew browser crate (`mdid-browser`), serde_json, cargo test.

---

## File Structure

- Modify: `crates/mdid-browser/src/app.rs` — add tabular report filename/payload helpers, expose availability, and add UI download control for CSV/XLSX report JSON.
- Modify: `README.md` — truth-sync completion/evidence after verified landing.

### Task 1: Browser tabular structured report download helper and UI control

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`
- Test: existing inline tests in `crates/mdid-browser/src/app.rs`

- [ ] **Step 1: Write failing tests for report payload and filename behavior**

Add tests in the `#[cfg(test)]` module of `crates/mdid-browser/src/app.rs`:

```rust
#[test]
fn tabular_report_download_payload_uses_safe_source_name_for_csv() {
    let mut state = BrowserFlowState::default();
    state.apply_imported_file("patient roster.csv", "name\nAlice", InputMode::CsvText);
    state.summary = "total_rows: 1\nencoded_cells: 1".to_string();
    state.review_queue = "No review items returned.".to_string();
    state.result_output = "name\nTOKEN_1".to_string();

    assert!(state.can_export_tabular_report());
    let payload = state.prepared_tabular_report_download_payload().unwrap();

    assert_eq!(payload.file_name, "patient_roster-tabular-report.json");
    assert_eq!(payload.mime_type, "application/json;charset=utf-8");
    assert!(payload.is_text);
    let report: serde_json::Value = serde_json::from_slice(&payload.bytes).unwrap();
    assert_eq!(report["mode"], "tabular_report");
    assert_eq!(report["input_mode"], "csv-text");
    assert_eq!(report["summary"], "total_rows: 1\nencoded_cells: 1");
    assert_eq!(report["review_queue"], "No review items returned.");
    assert!(report.get("rewritten_output").is_none());
}

#[test]
fn tabular_report_download_payload_supports_xlsx_without_rewritten_bytes() {
    let mut state = BrowserFlowState::default();
    state.apply_imported_file("workbook.xlsx", "UEsDBAo=", InputMode::XlsxBase64);
    state.summary = "total_rows: 2\nencoded_cells: 2".to_string();
    state.review_queue = "- row 2 needs review".to_string();
    state.result_output = "UEsDBAo=".to_string();

    let payload = state.prepared_tabular_report_download_payload().unwrap();
    let report: serde_json::Value = serde_json::from_slice(&payload.bytes).unwrap();

    assert_eq!(payload.file_name, "workbook-tabular-report.json");
    assert_eq!(report["mode"], "tabular_report");
    assert_eq!(report["input_mode"], "xlsx-base64");
    assert_eq!(report["review_queue"], "- row 2 needs review");
    assert!(report.get("rewritten_output").is_none());
}

#[test]
fn tabular_report_download_rejects_non_tabular_or_empty_output() {
    let mut state = BrowserFlowState::default();
    state.input_mode = InputMode::PdfBase64;
    state.result_output = "PDF rewrite/export unavailable".to_string();
    assert!(!state.can_export_tabular_report());
    assert!(state.prepared_tabular_report_download_payload().is_err());

    state.input_mode = InputMode::CsvText;
    state.result_output.clear();
    assert!(!state.can_export_tabular_report());
}
```

- [ ] **Step 2: Run tests to verify RED**

Run: `cargo test -p mdid-browser tabular_report_download -- --nocapture`
Expected: FAIL because `can_export_tabular_report` and `prepared_tabular_report_download_payload` do not exist.

- [ ] **Step 3: Add minimal browser state helpers**

In `impl BrowserFlowState`, add:

```rust
fn is_tabular_mode(&self) -> bool {
    matches!(self.input_mode, InputMode::CsvText | InputMode::XlsxBase64)
}

fn suggested_tabular_report_file_name(&self) -> String {
    self.imported_file_name
        .as_deref()
        .map(sanitized_import_stem)
        .filter(|stem| stem != "mdid-browser-output")
        .map(|stem| format!("{stem}-tabular-report.json"))
        .unwrap_or_else(|| "mdid-browser-tabular-report.json".to_string())
}

fn tabular_report_download_json(&self) -> Result<Vec<u8>, String> {
    if !self.is_tabular_mode() || self.result_output.trim().is_empty() {
        return Err("Browser tabular report download is only available after a successful CSV or XLSX runtime response.".to_string());
    }

    serde_json::to_vec_pretty(&serde_json::json!({
        "mode": "tabular_report",
        "input_mode": self.input_mode.value(),
        "summary": self.summary,
        "review_queue": self.review_queue,
    }))
    .map_err(|_| "Browser tabular report download could not encode JSON.".to_string())
}

fn can_export_tabular_report(&self) -> bool {
    self.tabular_report_download_json().is_ok()
}

fn prepared_tabular_report_download_payload(&self) -> Result<BrowserDownloadPayload, String> {
    Ok(BrowserDownloadPayload {
        file_name: self.suggested_tabular_report_file_name(),
        mime_type: "application/json;charset=utf-8",
        bytes: self.tabular_report_download_json()?,
        is_text: true,
    })
}
```

- [ ] **Step 4: Add UI button and wasm download callback**

Add a second download control near the existing output download button that is rendered only when `state.can_export_tabular_report()` is true and calls a new `Msg::DownloadTabularReport`. The callback must use `state.prepared_tabular_report_download_payload()` and the existing browser download helper so the report downloads as JSON without replacing rewritten CSV/XLSX output download behavior.

- [ ] **Step 5: Run targeted tests to verify GREEN**

Run: `cargo test -p mdid-browser tabular_report_download -- --nocapture`
Expected: PASS.

- [ ] **Step 6: Run broader browser tests and formatting**

Run:

```bash
cargo test -p mdid-browser --lib
cargo fmt --check
git diff --check
```

Expected: all pass.

- [ ] **Step 7: Commit implementation**

```bash
git add crates/mdid-browser/src/app.rs docs/superpowers/plans/2026-04-30-browser-tabular-report-download.md
git commit -m "feat(browser): add tabular report downloads"
```

### Task 2: Truth-sync README completion evidence

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update completion snapshot**

Update `README.md` current repository status to state that browser/web now includes explicit structured JSON tabular report downloads for successful CSV/XLSX browser runtime responses, separate from rewritten CSV/XLSX output downloads. Browser/Web completion increases from 79% to 80%; CLI remains 95%; Desktop app remains 72%; Overall remains 93%.

- [ ] **Step 2: Run docs verification**

Run:

```bash
git diff --check
```

Expected: PASS.

- [ ] **Step 3: Commit docs**

```bash
git add README.md
git commit -m "docs: truth-sync browser tabular report downloads"
```

## Self-Review

Spec coverage: Task 1 adds the separate structured tabular report helper and UI path without altering rewritten output downloads; Task 2 updates README completion/evidence. Placeholder scan: no TBD/TODO placeholders. Type consistency: helper names and `InputMode::value()` match existing browser code conventions.
