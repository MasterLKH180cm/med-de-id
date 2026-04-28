     1|# Desktop File Import Export Helpers Implementation Plan
     2|
     3|> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
     4|
     5|**Goal:** Add bounded desktop helper logic for local CSV/XLSX/PDF file import payload preparation and safe output export naming without adding a generalized workflow platform.
     6|
     7|**Architecture:** Keep the slice in `mdid-desktop` library code so behavior is testable without a GUI harness. The helpers map file names and byte/text payloads onto the existing three desktop modes, enforce bounded size/type rules, and derive honest export suggestions from already-rendered runtime response state.
     8|
     9|**Tech Stack:** Rust workspace, `mdid-desktop`, unit tests with `cargo test -p mdid-desktop`.
    10|
    11|---
    12|
    13|## File Structure
    14|
    15|- Modify: `crates/mdid-desktop/src/lib.rs`
    16|  - Add `DesktopFileImportPayload`, `DesktopFileImportError`, import size/type helpers, `DesktopWorkflowRequestState::apply_imported_file`, and response export helper methods.
    17|  - Update disclosure/status copy to remove stale “file picker upload/download UX” missing claim once bounded helpers exist.
    18|- Modify: `README.md`
    19|  - Truth-sync desktop/browser/overall completion rows and remaining missing items after landed tests.
    20|
    21|### Task 1: Desktop import payload helpers
    22|
    23|**Files:**
    24|- Modify: `crates/mdid-desktop/src/lib.rs`
    25|- Test: inline `#[cfg(test)]` module in `crates/mdid-desktop/src/lib.rs`
    26|
    27|- [x] **Step 1: Write the failing tests**
    28|
    29|Add tests to the existing test module:
    30|
    31|```rust
    32|#[test]
    33|fn desktop_file_import_maps_csv_xlsx_and_pdf_payloads_to_existing_modes() {
    34|    let csv = DesktopFileImportPayload::from_file_bytes("patients.csv", b"patient_name\nAlice").unwrap();
    35|    assert_eq!(csv.file_name, "patients.csv");
    36|    assert_eq!(csv.mode, DesktopWorkflowMode::CsvText);
    37|    assert_eq!(csv.payload, "patient_name\nAlice");
    38|    assert_eq!(csv.source_name, None);
    39|
    40|    let xlsx = DesktopFileImportPayload::from_file_bytes("patients.xlsx", &[0x50, 0x4b, 0x03, 0x04]).unwrap();
    41|    assert_eq!(xlsx.mode, DesktopWorkflowMode::XlsxBase64);
    42|    assert_eq!(xlsx.payload, "UEsDBA==");
    43|
    44|    let pdf = DesktopFileImportPayload::from_file_bytes("chart.PDF", b"%PDF-1.7").unwrap();
    45|    assert_eq!(pdf.mode, DesktopWorkflowMode::PdfBase64Review);
    46|    assert_eq!(pdf.payload, "JVBERi0xLjc=");
    47|    assert_eq!(pdf.source_name.as_deref(), Some("chart.PDF"));
    48|}
    49|
    50|#[test]
    51|fn desktop_file_import_rejects_unknown_large_and_non_utf8_csv_files() {
    52|    assert_eq!(
    53|        DesktopFileImportPayload::from_file_bytes("notes.txt", b"hello"),
    54|        Err(DesktopFileImportError::UnsupportedFileType)
    55|    );
    56|    assert_eq!(
    57|        DesktopFileImportPayload::from_file_bytes("bad.csv", &[0xff, 0xfe]),
    58|        Err(DesktopFileImportError::InvalidCsvUtf8)
    59|    );
    60|    let oversized = vec![b'a'; DESKTOP_FILE_IMPORT_MAX_BYTES + 1];
    61|    assert_eq!(
    62|        DesktopFileImportPayload::from_file_bytes("large.csv", &oversized),
    63|        Err(DesktopFileImportError::FileTooLarge)
    64|    );
    65|}
    66|
    67|#[test]
    68|fn desktop_request_state_applies_imported_payload_without_changing_policy_json() {
    69|    let mut state = DesktopWorkflowRequestState::default();
    70|    state.field_policy_json = r#"[{"header":"patient_name","phi_type":"Name","action":"review"}]"#.to_string();
    71|    let imported = DesktopFileImportPayload::from_file_bytes("chart.pdf", b"%PDF-1.7").unwrap();
    72|
    73|    state.apply_imported_file(imported);
    74|
    75|    assert_eq!(state.mode, DesktopWorkflowMode::PdfBase64Review);
    76|    assert_eq!(state.payload, "JVBERi0xLjc=");
    77|    assert_eq!(state.source_name, "chart.pdf");
    78|    assert!(state.field_policy_json.contains("patient_name"));
    79|}
    80|```
    81|
    82|- [x] **Step 2: Run test to verify it fails**
    83|
    84|Run: `cargo test -p mdid-desktop desktop_file_import -- --nocapture`
    85|
    86|Expected: FAIL with unresolved `DesktopFileImportPayload` / `DesktopFileImportError` / `DESKTOP_FILE_IMPORT_MAX_BYTES`.
    87|
    88|- [x] **Step 3: Write minimal implementation**
    89|
    90|Add near the request-state types:
    91|
    92|```rust
    93|pub const DESKTOP_FILE_IMPORT_MAX_BYTES: usize = 10 * 1024 * 1024;
    94|
    95|#[derive(Debug, Clone, PartialEq, Eq)]
    96|pub struct DesktopFileImportPayload {
    97|    pub file_name: String,
    98|    pub mode: DesktopWorkflowMode,
    99|    pub payload: String,
   100|    pub source_name: Option<String>,
   101|}
   102|
   103|#[derive(Debug, Clone, PartialEq, Eq)]
   104|pub enum DesktopFileImportError {
   105|    UnsupportedFileType,
   106|    FileTooLarge,
   107|    InvalidCsvUtf8,
   108|}
   109|
   110|impl DesktopFileImportPayload {
   111|    pub fn from_file_bytes(file_name: &str, bytes: &[u8]) -> Result<Self, DesktopFileImportError> {
   112|        if bytes.len() > DESKTOP_FILE_IMPORT_MAX_BYTES {
   113|            return Err(DesktopFileImportError::FileTooLarge);
   114|        }
   115|        let lower_name = file_name.to_ascii_lowercase();
   116|        if lower_name.ends_with(".csv") {
   117|            let payload = std::str::from_utf8(bytes)
   118|                .map_err(|_| DesktopFileImportError::InvalidCsvUtf8)?
   119|                .to_string();
   120|            return Ok(Self { file_name: file_name.to_string(), mode: DesktopWorkflowMode::CsvText, payload, source_name: None });
   121|        }
   122|        if lower_name.ends_with(".xlsx") {
   123|            return Ok(Self { file_name: file_name.to_string(), mode: DesktopWorkflowMode::XlsxBase64, payload: base64_encode(bytes), source_name: None });
   124|        }
   125|        if lower_name.ends_with(".pdf") {
   126|            return Ok(Self { file_name: file_name.to_string(), mode: DesktopWorkflowMode::PdfBase64Review, payload: base64_encode(bytes), source_name: Some(file_name.to_string()) });
   127|        }
   128|        Err(DesktopFileImportError::UnsupportedFileType)
   129|    }
   130|}
   131|
   132|impl DesktopWorkflowRequestState {
   133|    pub fn apply_imported_file(&mut self, imported: DesktopFileImportPayload) {
   134|        self.mode = imported.mode;
   135|        self.payload = imported.payload;
   136|        if let Some(source_name) = imported.source_name {
   137|            self.source_name = source_name;
   138|        }
   139|    }
   140|}
   141|
   142|fn base64_encode(bytes: &[u8]) -> String {
   143|    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
   144|    let mut output = String::new();
   145|    for chunk in bytes.chunks(3) {
   146|        let b0 = chunk[0];
   147|        let b1 = *chunk.get(1).unwrap_or(&0);
   148|        let b2 = *chunk.get(2).unwrap_or(&0);
   149|        output.push(TABLE[(b0 >> 2) as usize] as char);
   150|        output.push(TABLE[(((b0 & 0b0000_0011) << 4) | (b1 >> 4)) as usize] as char);
   151|        if chunk.len() > 1 {
   152|            output.push(TABLE[(((b1 & 0b0000_1111) << 2) | (b2 >> 6)) as usize] as char);
   153|        } else {
   154|            output.push('=');
   155|        }
   156|        if chunk.len() > 2 {
   157|            output.push(TABLE[(b2 & 0b0011_1111) as usize] as char);
   158|        } else {
   159|            output.push('=');
   160|        }
   161|    }
   162|    output
   163|}
   164|```
   165|
   166|Also update `status_message` copy so it says bounded file import/export helpers are present while still excluding OCR, visual redaction, PDF rewrite/export, vault/decode/audit workflow, and full review workflow.
   167|
   168|- [x] **Step 4: Run test to verify it passes**
   169|
   170|Run: `cargo test -p mdid-desktop desktop_file_import -- --nocapture`
   171|
   172|Expected: PASS.
   173|
   174|- [x] **Step 5: Run crate tests**
   175|
   176|Run: `cargo test -p mdid-desktop`
   177|
   178|Expected: PASS.
   179|
   180|- [x] **Step 6: Commit**
   181|
   182|```bash
   183|git add crates/mdid-desktop/src/lib.rs
   184|git commit -m "feat(desktop): add bounded file import helpers"
   185|```
   186|
   187|### Task 2: Desktop response export helpers and README truth-sync
   188|
   189|**Files:**
   190|- Modify: `crates/mdid-desktop/src/lib.rs`
   191|- Modify: `README.md`
   192|- Test: inline `#[cfg(test)]` module in `crates/mdid-desktop/src/lib.rs`
   193|
   194|- [x] **Step 1: Write the failing tests**
   195|
   196|Add tests:
   197|
   198|```rust
   199|#[test]
   200|fn response_state_suggests_exports_only_when_output_bytes_exist() {
   201|    let mut csv = DesktopWorkflowResponseState::default();
   202|    csv.apply_success_json(
   203|        DesktopWorkflowMode::CsvText,
   204|        json!({"csv":"patient_name\n<NAME-1>","summary":{},"review_queue":[]}),
   205|    );
   206|    assert_eq!(csv.suggested_export_file_name(DesktopWorkflowMode::CsvText), Some("desktop-deidentified.csv"));
   207|    assert_eq!(csv.exportable_output(), Some("patient_name\n<NAME-1>"));
   208|
   209|    let mut pdf = DesktopWorkflowResponseState::default();
   210|    pdf.apply_success_json(
   211|        DesktopWorkflowMode::PdfBase64Review,
   212|        json!({"rewritten_pdf_bytes_base64":null,"summary":{},"review_queue":[]}),
   213|    );
   214|    assert_eq!(pdf.suggested_export_file_name(DesktopWorkflowMode::PdfBase64Review), None);
   215|    assert_eq!(pdf.exportable_output(), None);
   216|}
   217|
   218|#[test]
   219|fn response_state_suggests_xlsx_export_for_rewritten_workbook_base64() {
   220|    let mut xlsx = DesktopWorkflowResponseState::default();
   221|    xlsx.apply_success_json(
   222|        DesktopWorkflowMode::XlsxBase64,
   223|        json!({"rewritten_workbook_base64":"UEsDBAo=","summary":{},"review_queue":[]}),
   224|    );
   225|    assert_eq!(xlsx.suggested_export_file_name(DesktopWorkflowMode::XlsxBase64), Some("desktop-deidentified.xlsx.base64.txt"));
   226|    assert_eq!(xlsx.exportable_output(), Some("UEsDBAo="));
   227|}
   228|```
   229|
   230|- [x] **Step 2: Run test to verify it fails**
   231|
   232|Run: `cargo test -p mdid-desktop response_state_suggests -- --nocapture`
   233|
   234|Expected: FAIL with missing methods.
   235|
   236|- [x] **Step 3: Write minimal implementation**
   237|
   238|Add methods to `impl DesktopWorkflowResponseState`:
   239|
   240|```rust
   241|pub fn exportable_output(&self) -> Option<&str> {
   242|    let output = self.output.trim();
   243|    if output.is_empty() || output == "No rewritten PDF bytes returned by the bounded review route." {
   244|        None
   245|    } else {
   246|        Some(self.output.as_str())
   247|    }
   248|}
   249|
   250|pub fn suggested_export_file_name(&self, mode: DesktopWorkflowMode) -> Option<&'static str> {
   251|    self.exportable_output()?;
   252|    match mode {
   253|        DesktopWorkflowMode::CsvText => Some("desktop-deidentified.csv"),
   254|        DesktopWorkflowMode::XlsxBase64 => Some("desktop-deidentified.xlsx.base64.txt"),
   255|        DesktopWorkflowMode::PdfBase64Review => None,
   256|    }
   257|}
   258|```
   259|
   260|Update README completion rows to reflect the bounded desktop import/export helper slice: desktop app +3 points if tests pass, overall +1 point if controller-visible verification passes. Keep missing items honest and do not claim OCR/PDF rewrite/full workflows.
   261|
   262|- [x] **Step 4: Run tests**
   263|
   264|Run: `cargo test -p mdid-desktop response_state_suggests -- --nocapture && cargo test -p mdid-desktop`
   265|
   266|Expected: PASS.
   267|
   268|- [x] **Step 5: README verification**
   269|
   270|Run: `grep -n "Desktop app\|Overall\|Missing items" README.md`
   271|
   272|Expected: Desktop app row mentions bounded CSV/XLSX/PDF file import/export helpers; Overall row remains below 90% unless broader landed functionality justifies more.
   273|
   274|- [x] **Step 6: Commit**
   275|
   276|```bash
   277|git add crates/mdid-desktop/src/lib.rs README.md docs/superpowers/plans/2026-04-28-desktop-file-import-export-helpers.md
   278|git commit -m "feat(desktop): add bounded output export helpers"
   279|```
   280|
   281|## Self-Review
   282|
   283|- Spec coverage: Covers desktop file import payload preparation and output export helper suggestions only; does not implement OCR, visual redaction, PDF rewrite/export, vault/decode/audit workflow, or agent/controller features.
   284|- Placeholder scan: No TBD/TODO/fill-in placeholders remain.
   285|- Type consistency: `DesktopWorkflowMode`, `DesktopWorkflowRequestState`, and `DesktopWorkflowResponseState` names match existing code.
   286|