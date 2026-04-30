# Core Completion Safety Disclosures Implementation Plan

> **For implementers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete five medium safety/product slices with test-first changes: XLSX multi-sheet disclosure, conservative media byte/base64 payload rejection, DICOM burned-in/pixel-redaction disclosure, PDF review-only rewrite status guard, and vault audit pagination.

**Architecture:** Preserve current local-first Rust workspace boundaries. Keep shared behavior in `mdid-domain`, adapter behavior in `mdid-adapters`, service orchestration in `mdid-application`, localhost HTTP contracts in `mdid-runtime`, automation in `mdid-cli`, browser workflow state in `mdid-browser`, and desktop workflow state in `mdid-desktop`. Do not add OCR, visual redaction, pixel redaction, PDF rewrite/export, vault browsing, auth/session, or media byte processing.

**Tech Stack:** Rust workspace, Axum runtime routes, serde JSON contracts, calamine/rust_xlsxwriter XLSX tests, DICOM adapter tests, browser/desktop state helper tests, Cargo test.

---

## Current repo patterns to preserve

- TDD is mandatory per repository development rules: write a failing test before production behavior changes, then make the smallest implementation pass.
- `crates/mdid-adapters/src/tabular.rs` currently selects the first non-empty worksheet while falling back to the first sheet if all sheets are blank. Keep that behavior.
- `crates/mdid-browser/src/app.rs` already has disclosure copy in `InputMode::scope_note()` and uses `BrowserFlowState` helper tests in the same file.
- `crates/mdid-desktop/src/lib.rs` has workflow scope copy and request builder tests in the same file.
- `crates/mdid-runtime/src/http.rs` owns route request/response DTOs and PHI-safe error envelopes.
- `crates/mdid-runtime/tests/runtime_http.rs` already covers `/tabular/deidentify/xlsx`, `/media/conservative/deidentify`, `/dicom/deidentify`, `/pdf/deidentify`, and `/vault/audit/events` as integration-style HTTP tests.
- `crates/mdid-cli/src/main.rs` has `build_vault_audit_report` tests and CLI command handling for vault audit output.

---

## File Structure

- Modify: `crates/mdid-domain/src/lib.rs`
  - Add reusable metadata fields/types only if a response disclosure needs a shared domain model.
- Modify: `crates/mdid-domain/tests/tabular_workflow_models.rs`
  - Add or harden XLSX disclosure model tests if the disclosure is represented in shared workflow data.
- Modify: `crates/mdid-domain/tests/conservative_media_workflow_models.rs`
  - Add media payload-rejection model tests if request validation is lifted into domain.
- Modify: `crates/mdid-domain/tests/dicom_workflow_models.rs`
  - Add DICOM unsupported pixel-redaction disclosure model tests if response summary is extended in domain.
- Modify: `crates/mdid-domain/tests/pdf_workflow_models.rs`
  - Add PDF review-only status model tests if response summary/page status is extended in domain.
- Modify: `crates/mdid-adapters/src/tabular.rs`
  - Return selected worksheet disclosure metadata without changing extraction behavior.
- Modify: `crates/mdid-adapters/tests/xlsx_tabular_adapter.rs`
  - Add tests proving first non-empty sheet behavior and disclosure metadata.
- Modify: `crates/mdid-adapters/src/conservative_media.rs`
  - Reject byte/base64 payload indicators before metadata candidate generation if implemented below runtime.
- Modify: `crates/mdid-adapters/tests/conservative_media_adapter.rs`
  - Add rejection tests for byte/base64 fields with PHI-safe errors.
- Modify: `crates/mdid-adapters/src/dicom.rs`
  - Surface a disclosure that burned-in annotations/pixel redaction are not performed.
- Modify: `crates/mdid-adapters/tests/dicom_adapter.rs`
  - Add DICOM disclosure tests.
- Modify: `crates/mdid-adapters/src/pdf.rs`
  - Preserve review-only extraction and ensure no rewrite status is ambiguous.
- Modify: `crates/mdid-adapters/tests/pdf_adapter.rs`
  - Add review-only/no-rewrite status tests.
- Modify: `crates/mdid-application/src/lib.rs`
  - Thread new disclosure/status fields through application output structs.
- Modify: `crates/mdid-application/tests/tabular_deidentification.rs`
  - Add XLSX disclosure service test.
- Modify: `crates/mdid-application/tests/conservative_media_deidentification.rs`
  - Add conservative media payload-rejection service test if validation is not runtime-only.
- Modify: `crates/mdid-application/tests/dicom_deidentification.rs`
  - Add burned-in/pixel-redaction disclosure service test.
- Modify: `crates/mdid-application/tests/pdf_deidentification.rs`
  - Add PDF review-only/no-rewrite service test.
- Modify: `crates/mdid-runtime/src/http.rs`
  - Add/serialize disclosures, reject media byte/base64 request fields with PHI-safe errors, and paginate `/vault/audit/events`.
- Modify: `crates/mdid-runtime/tests/runtime_http.rs`
  - Add HTTP contract tests for all five slices.
- Modify: `crates/mdid-cli/src/main.rs`
  - Add `vault audit` offset/cursor flags and output fields if CLI exposes audit pagination.
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
  - Add end-to-end CLI audit pagination smoke test if CLI flags are added.
- Modify: `crates/mdid-browser/src/app.rs`
  - Sync copy, request validation, response parsing, PDF status guard, DICOM disclosure, XLSX disclosure, and vault audit form/request state.
- Modify: `crates/mdid-desktop/src/lib.rs`
  - Sync workflow scope copy and request/response helper behavior for desktop.
- Modify: `README.md`
  - Truth-sync completion snapshot after all slices land.

---

## Task 1: XLSX multi-sheet disclosure while preserving first non-empty sheet behavior

**Files:**
- Test: `crates/mdid-adapters/tests/xlsx_tabular_adapter.rs`
- Test: `crates/mdid-runtime/tests/runtime_http.rs`
- Test: `crates/mdid-browser/src/app.rs`
- Test: `crates/mdid-desktop/src/lib.rs`
- Modify: `crates/mdid-adapters/src/tabular.rs`
- Modify: `crates/mdid-application/src/lib.rs`
- Modify: `crates/mdid-runtime/src/http.rs`
- Modify: `crates/mdid-browser/src/app.rs`
- Modify: `crates/mdid-desktop/src/lib.rs`

- [ ] **Step 1: Write failing adapter tests**

Add tests near existing XLSX adapter tests:

- `xlsx_extract_discloses_selected_first_non_empty_sheet`
  - Build a workbook with sheet 1 blank and named `Cover`, sheet 2 named `Patients` with headers `name,dob` and one PHI row.
  - Use `XlsxTabularAdapter::new(vec![FieldPolicy::encode("name", "patient_name")])`.
  - Assert extracted rows come from `Patients`, not `Cover`.
  - Assert new metadata/disclosure fields report:
    - selected sheet name: `Patients`
    - selected sheet index: `1`
    - total sheet count: `2`
    - disclosure text exactly: `XLSX processing used the first non-empty worksheet; other worksheets were not processed.`
- `xlsx_extract_preserves_first_sheet_when_all_sheets_blank`
  - Build two blank sheets.
  - Assert selected sheet name/index remain first sheet (`0`) and rows are empty.

Run: `cargo test -p mdid-adapters --test xlsx_tabular_adapter xlsx_extract_discloses_selected_first_non_empty_sheet -- --nocapture`

Expected RED: compile failure because `ExtractedTabularData` has no sheet disclosure fields, or assertion failure if fields exist but are incomplete.

- [ ] **Step 2: Add minimal adapter metadata**

In `crates/mdid-adapters/src/tabular.rs`:

- Add a small public struct, e.g. `XlsxWorksheetDisclosure { pub selected_sheet_name: String, pub selected_sheet_index: usize, pub total_sheet_count: usize, pub disclosure: String }`.
- Add `pub xlsx_worksheet_disclosure: Option<XlsxWorksheetDisclosure>` to `ExtractedTabularData`.
- Keep CSV extraction setting this field to `None`.
- In `XlsxTabularAdapter::extract`, track `selected_sheet_name`, `selected_sheet_index`, and `sheet_names.len()` while preserving the existing first non-empty/fallback logic.
- Do not process additional sheets.

Run:

```bash
cargo test -p mdid-adapters --test xlsx_tabular_adapter xlsx_extract_discloses_selected_first_non_empty_sheet -- --nocapture
cargo test -p mdid-adapters --test xlsx_tabular_adapter xlsx_extract_preserves_first_sheet_when_all_sheets_blank -- --nocapture
```

Expected GREEN.

- [ ] **Step 3: Thread disclosure through application/runtime response**

Write failing HTTP test in `crates/mdid-runtime/tests/runtime_http.rs` named `tabular_xlsx_deidentify_endpoint_discloses_selected_sheet_scope`:

- POST to `/tabular/deidentify/xlsx` with a two-sheet workbook whose first sheet is blank and second sheet has PHI.
- Assert HTTP 200.
- Assert `summary.total_fields` and rewritten workbook behavior remain unchanged from existing tests.
- Assert response JSON contains `worksheet_disclosure` with the exact fields above.
- Assert response JSON does not include cell values from unprocessed sheets.

Run: `cargo test -p mdid-runtime --test runtime_http tabular_xlsx_deidentify_endpoint_discloses_selected_sheet_scope -- --nocapture`

Expected RED.

Implement minimal threading:

- Add `worksheet_disclosure` to `TabularDeidentificationOutput` or the XLSX-specific output in `crates/mdid-application/src/lib.rs`.
- Add serializable DTO to `crates/mdid-runtime/src/http.rs`, include it in `TabularXlsxDeidentifyResponse`.
- Map adapter metadata to response DTO.

Run:

```bash
cargo test -p mdid-runtime --test runtime_http tabular_xlsx_deidentify_endpoint_discloses_selected_sheet_scope -- --nocapture
cargo test -p mdid-application --test tabular_deidentification
cargo test -p mdid-runtime --test runtime_http tabular_xlsx_deidentify_endpoint_returns_rewritten_workbook_and_summary -- --nocapture
```

- [ ] **Step 4: Sync browser/desktop copy and parsing**

If browser/desktop already disclose `XLSX mode only processes the first non-empty worksheet`, add hardening tests only; otherwise update scope copy.

Browser tests in `crates/mdid-browser/src/app.rs`:

- Add `xlsx_scope_note_discloses_first_non_empty_sheet_only` asserting `InputMode::XlsxBase64.scope_note().unwrap()` contains `first non-empty worksheet` and `Sheet selection is not supported`.
- Add response parsing test asserting a runtime response containing `worksheet_disclosure` appends or preserves a PHI-safe line in `summary` without exposing workbook cell values.

Desktop tests in `crates/mdid-desktop/src/lib.rs`:

- Add `xlsx_workflow_scope_note_discloses_first_non_empty_sheet_only` for the matching desktop mode/copy.

Run:

```bash
cargo test -p mdid-browser --lib xlsx_scope_note_discloses_first_non_empty_sheet_only -- --nocapture
cargo test -p mdid-desktop --lib xlsx_workflow_scope_note_discloses_first_non_empty_sheet_only -- --nocapture
```

- [ ] **Step 5: Broader verification for Task 1**

Run:

```bash
cargo test -p mdid-adapters --test xlsx_tabular_adapter
cargo test -p mdid-application --test tabular_deidentification
cargo test -p mdid-runtime --test runtime_http tabular_xlsx -- --nocapture
cargo test -p mdid-browser --lib xlsx -- --nocapture
cargo test -p mdid-desktop --lib xlsx -- --nocapture
```

---

## Task 2: Conservative media byte/base64 payload rejection with PHI-safe error

**Files:**
- Test: `crates/mdid-runtime/tests/runtime_http.rs`
- Test: `crates/mdid-adapters/tests/conservative_media_adapter.rs`
- Test: `crates/mdid-application/tests/conservative_media_deidentification.rs`
- Test: `crates/mdid-browser/src/app.rs`
- Test: `crates/mdid-desktop/src/lib.rs`
- Modify: `crates/mdid-runtime/src/http.rs`
- Modify: `crates/mdid-adapters/src/conservative_media.rs`
- Modify: `crates/mdid-application/src/lib.rs`
- Modify: `crates/mdid-browser/src/app.rs`
- Modify: `crates/mdid-desktop/src/lib.rs`

- [ ] **Step 1: Write failing runtime rejection tests**

In `crates/mdid-runtime/tests/runtime_http.rs`, add:

- `conservative_media_deidentify_endpoint_rejects_media_bytes_base64_field`
- `conservative_media_deidentify_endpoint_rejects_raw_media_bytes_field`
- `conservative_media_deidentify_endpoint_rejects_metadata_value_that_declares_base64_payload`

For the first two tests, POST to `/media/conservative/deidentify` with otherwise valid JSON plus one unexpected field:

```json
{
  "artifact_label": "face-photo.jpg",
  "format": "image",
  "metadata": [{"key":"filename","value":"face-photo.jpg"}],
  "media_bytes_base64": "SmFuZSBQYXRpZW50IGZhY2U="
}
```

and:

```json
{
  "artifact_label": "face-photo.jpg",
  "format": "image",
  "metadata": [{"key":"filename","value":"face-photo.jpg"}],
  "media_bytes": [1,2,3,4]
}
```

Assert:

- HTTP status is `400 BAD_REQUEST`.
- JSON error code is `invalid_conservative_media_request`.
- Error message is exactly `Media byte payloads are not accepted by this metadata-only route.`
- Response has no `summary`, no `review_queue`, and no `rewritten_media_bytes_base64`.
- Response body does not contain `Jane`, `Patient`, `SmFu`, or the byte array.

For metadata declaration test, use metadata key `media_bytes_base64` or `payload_base64`; assert same PHI-safe envelope. This closes the obvious bypass where bytes arrive as metadata.

Run: `cargo test -p mdid-runtime --test runtime_http conservative_media_deidentify_endpoint_rejects_media_bytes_base64_field -- --nocapture`

Expected RED because current serde ignores unknown fields and accepts metadata-only payloads.

- [ ] **Step 2: Implement minimal request rejection**

In `crates/mdid-runtime/src/http.rs`:

- Change `/media/conservative/deidentify` request extraction to inspect raw JSON object keys before deserializing to `ConservativeMediaDeidentifyRequest`, or add `#[serde(deny_unknown_fields)]` plus a custom PHI-safe mapping.
- Reject at least these top-level keys case-insensitively after normalizing `_`/`-`: `media_bytes`, `media_bytes_base64`, `bytes`, `bytes_base64`, `payload`, `payload_base64`, `image_bytes`, `audio_bytes`, `video_bytes`.
- Reject metadata entries whose `key` normalizes to any of the same byte/base64 payload names.
- Return the existing `invalid_conservative_media_request` code with the exact PHI-safe message above.
- Do not log or echo field values.

If validation belongs better in `mdid-adapters/src/conservative_media.rs`, add a dedicated adapter error and map it to the same runtime envelope. Keep the runtime unknown top-level field protection regardless, because serde currently discards unknown fields.

Run the three targeted tests until GREEN.

- [ ] **Step 3: Add application/adapter hardening tests if validation moved below runtime**

If production validation is in adapter/application, add:

- `conservative_media_rejects_byte_payload_metadata_key` in `crates/mdid-adapters/tests/conservative_media_adapter.rs`.
- `conservative_media_service_rejects_byte_payload_without_phi_echo` in `crates/mdid-application/tests/conservative_media_deidentification.rs`.

Run:

```bash
cargo test -p mdid-adapters --test conservative_media_adapter conservative_media_rejects_byte_payload_metadata_key -- --nocapture
cargo test -p mdid-application --test conservative_media_deidentification conservative_media_service_rejects_byte_payload_without_phi_echo -- --nocapture
```

- [ ] **Step 4: Sync browser/desktop request builders and copy**

Browser (`crates/mdid-browser/src/app.rs`):

- Add test `media_metadata_json_rejects_byte_payload_keys_before_submit` constructing a media metadata JSON body containing `media_bytes_base64` and asserting `try_build_runtime_request()` returns `Media byte payloads are not accepted by this metadata-only route.` without including the base64 value.
- Keep `InputMode::MediaMetadataJson.scope_note()` saying metadata-only, no media bytes, no visual redaction/rewrite.

Desktop (`crates/mdid-desktop/src/lib.rs`):

- Add test `media_metadata_workflow_rejects_byte_payload_keys_before_submit` for the desktop request builder.

Run:

```bash
cargo test -p mdid-browser --lib media_metadata_json_rejects_byte_payload_keys_before_submit -- --nocapture
cargo test -p mdid-desktop --lib media_metadata_workflow_rejects_byte_payload_keys_before_submit -- --nocapture
```

- [ ] **Step 5: Broader verification for Task 2**

Run:

```bash
cargo test -p mdid-runtime --test runtime_http conservative_media -- --nocapture
cargo test -p mdid-adapters --test conservative_media_adapter
cargo test -p mdid-application --test conservative_media_deidentification
cargo test -p mdid-browser --lib media -- --nocapture
cargo test -p mdid-desktop --lib media -- --nocapture
```

---

## Task 3: DICOM burned-in/pixel-redaction not-performed disclosure

**Files:**
- Test: `crates/mdid-domain/tests/dicom_workflow_models.rs`
- Test: `crates/mdid-adapters/tests/dicom_adapter.rs`
- Test: `crates/mdid-application/tests/dicom_deidentification.rs`
- Test: `crates/mdid-runtime/tests/runtime_http.rs`
- Test: `crates/mdid-browser/src/app.rs`
- Test: `crates/mdid-desktop/src/lib.rs`
- Modify: `crates/mdid-domain/src/lib.rs`
- Modify: `crates/mdid-adapters/src/dicom.rs`
- Modify: `crates/mdid-application/src/lib.rs`
- Modify: `crates/mdid-runtime/src/http.rs`
- Modify: `crates/mdid-browser/src/app.rs`
- Modify: `crates/mdid-desktop/src/lib.rs`

- [ ] **Step 1: Write failing domain/runtime disclosure tests**

In `crates/mdid-domain/tests/dicom_workflow_models.rs`, add `dicom_summary_discloses_burned_in_and_pixel_redaction_not_performed` if `DicomDeidentificationSummary` is the shared response place. Assert fields:

- `burned_in_or_pixel_phi_review_required == true` when suspected burned-in annotation exists.
- `pixel_redaction_performed == false`.
- `burned_in_disclosure == "DICOM pixel data was not inspected or redacted; burned-in annotations require separate visual review."`

In `crates/mdid-runtime/tests/runtime_http.rs`, add `dicom_deidentify_endpoint_discloses_pixel_redaction_not_performed`:

- Submit a minimal DICOM payload that existing tests use.
- Assert 200 and existing `rewritten_dicom_bytes_base64` still exists.
- Assert JSON `summary.burned_in_suspicions` still exists.
- Assert either `summary.pixel_redaction_performed == false` and `summary.burned_in_disclosure` equals the exact string above, or a top-level `pixel_redaction_disclosure` object with the same data.

Run:

```bash
cargo test -p mdid-domain --test dicom_workflow_models dicom_summary_discloses_burned_in_and_pixel_redaction_not_performed -- --nocapture
cargo test -p mdid-runtime --test runtime_http dicom_deidentify_endpoint_discloses_pixel_redaction_not_performed -- --nocapture
```

Expected RED if response disclosure is absent or only present as UI copy.

- [ ] **Step 2: Implement minimal shared disclosure**

Preferred implementation:

- Extend `DicomDeidentificationSummary` in `crates/mdid-domain/src/lib.rs` with:
  - `pub pixel_redaction_performed: bool`
  - `pub burned_in_disclosure: String`
- Set defaults wherever summaries are built: `pixel_redaction_performed: false`, exact disclosure string above.
- If `burned_in_suspicions` is computed in `crates/mdid-adapters/src/dicom.rs`, leave the count unchanged; do not add pixel inspection.
- Thread unchanged through `crates/mdid-application/src/lib.rs` and `crates/mdid-runtime/src/http.rs` by relying on serde for the extended summary.

Run targeted domain/runtime tests until GREEN.

- [ ] **Step 3: Add adapter/application tests**

Add:

- `dicom_adapter_reports_pixel_redaction_not_performed` in `crates/mdid-adapters/tests/dicom_adapter.rs`.
- `dicom_service_preserves_pixel_redaction_disclosure` in `crates/mdid-application/tests/dicom_deidentification.rs`.

Assert no rewritten bytes are removed and private-tag behavior remains unchanged.

Run:

```bash
cargo test -p mdid-adapters --test dicom_adapter dicom_adapter_reports_pixel_redaction_not_performed -- --nocapture
cargo test -p mdid-application --test dicom_deidentification dicom_service_preserves_pixel_redaction_disclosure -- --nocapture
```

- [ ] **Step 4: Sync browser/desktop disclosure copy**

Browser:

- Update `InputMode::DicomBase64.scope_note()` only if it does not explicitly say pixel redaction is not performed.
- Add `dicom_scope_note_discloses_no_pixel_redaction` asserting copy includes `pixel redaction` and does not imply visual redaction.
- Add response-format test that `format_dicom_summary` includes `pixel_redaction_performed: false` and the exact disclosure text.

Desktop:

- Update `DesktopWorkflowMode::DicomBase64.scope_note()` similarly.
- Add `dicom_workflow_scope_note_discloses_no_pixel_redaction`.

Run:

```bash
cargo test -p mdid-browser --lib dicom_scope_note_discloses_no_pixel_redaction -- --nocapture
cargo test -p mdid-desktop --lib dicom_workflow_scope_note_discloses_no_pixel_redaction -- --nocapture
```

- [ ] **Step 5: Broader verification for Task 3**

Run:

```bash
cargo test -p mdid-domain --test dicom_workflow_models
cargo test -p mdid-adapters --test dicom_adapter
cargo test -p mdid-application --test dicom_deidentification
cargo test -p mdid-runtime --test runtime_http dicom -- --nocapture
cargo test -p mdid-browser --lib dicom -- --nocapture
cargo test -p mdid-desktop --lib dicom -- --nocapture
```

---

## Task 4: PDF review-only/no rewritten PDF status guard

**Files:**
- Test: `crates/mdid-domain/tests/pdf_workflow_models.rs`
- Test: `crates/mdid-adapters/tests/pdf_adapter.rs`
- Test: `crates/mdid-application/tests/pdf_deidentification.rs`
- Test: `crates/mdid-runtime/tests/runtime_http.rs`
- Test: `crates/mdid-browser/src/app.rs`
- Test: `crates/mdid-desktop/src/lib.rs`
- Modify: `crates/mdid-domain/src/lib.rs`
- Modify: `crates/mdid-adapters/src/pdf.rs`
- Modify: `crates/mdid-application/src/lib.rs`
- Modify: `crates/mdid-runtime/src/http.rs`
- Modify: `crates/mdid-browser/src/app.rs`
- Modify: `crates/mdid-desktop/src/lib.rs`

- [ ] **Step 1: Write failing runtime contract test**

In `crates/mdid-runtime/tests/runtime_http.rs`, add `pdf_deidentify_endpoint_returns_review_only_status_and_no_rewritten_pdf`:

- POST to `/pdf/deidentify` with an existing fixture/minimal PDF pattern from current tests.
- Assert HTTP 200.
- Assert `rewritten_pdf_bytes_base64` is `null`.
- Assert a status field exists and is exact:
  - `rewrite_status: "not_performed_review_only"`
  - `rewrite_disclosure: "PDF mode is review-only; no rewritten PDF bytes are produced."`
- Assert response body does not contain a non-null rewritten byte string.

Run: `cargo test -p mdid-runtime --test runtime_http pdf_deidentify_endpoint_returns_review_only_status_and_no_rewritten_pdf -- --nocapture`

Expected RED if only `rewritten_pdf_bytes_base64: null` exists without explicit status/disclosure.

- [ ] **Step 2: Add shared PDF status model**

In `crates/mdid-domain/src/lib.rs`, add a serde-friendly enum or string-backed status used by the PDF output, e.g.:

```rust
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PdfRewriteStatus {
    NotPerformedReviewOnly,
}
```

Thread through:

- `crates/mdid-adapters/src/pdf.rs`: output status is always `NotPerformedReviewOnly`; no rewrite implementation.
- `crates/mdid-application/src/lib.rs`: `PdfDeidentificationOutput` includes `rewrite_status` and `rewrite_disclosure`.
- `crates/mdid-runtime/src/http.rs`: `PdfDeidentifyResponse` serializes both fields and keeps `rewritten_pdf_bytes_base64: None`.

Run targeted runtime test until GREEN.

- [ ] **Step 3: Add domain/adapter/application tests**

Add:

- `pdf_rewrite_status_is_review_only` in `crates/mdid-domain/tests/pdf_workflow_models.rs`.
- `pdf_adapter_never_returns_rewritten_pdf_bytes` in `crates/mdid-adapters/tests/pdf_adapter.rs`.
- `pdf_service_preserves_review_only_rewrite_status` in `crates/mdid-application/tests/pdf_deidentification.rs`.

Assert exact status/disclosure and `None` bytes.

Run:

```bash
cargo test -p mdid-domain --test pdf_workflow_models pdf_rewrite_status_is_review_only -- --nocapture
cargo test -p mdid-adapters --test pdf_adapter pdf_adapter_never_returns_rewritten_pdf_bytes -- --nocapture
cargo test -p mdid-application --test pdf_deidentification pdf_service_preserves_review_only_rewrite_status -- --nocapture
```

- [ ] **Step 4: Harden browser/desktop guards**

Browser (`crates/mdid-browser/src/app.rs`):

- Existing code ignores runtime `rewritten_pdf_bytes_base64`; preserve that.
- Add `pdf_runtime_response_with_rewritten_bytes_still_downloads_review_report_only`:
  - Parse a synthetic successful runtime response containing `rewritten_pdf_bytes_base64: "JVBERi0="`.
  - Assert `RuntimeResponseEnvelope.rewritten_output` is `PDF rewrite/export unavailable: runtime returned review-only PDF analysis.` or contains no PDF bytes.
  - Assert `prepared_download_payload()` for `InputMode::PdfBase64` produces JSON report with filename ending `-review-report.json`, MIME `application/json;charset=utf-8`, and not `application/pdf`.
- Add `pdf_review_report_includes_rewrite_status` asserting report JSON includes `rewrite_status: "not_performed_review_only"` when the runtime provides it.

Desktop (`crates/mdid-desktop/src/lib.rs`):

- Add equivalent helper test for PDF review mode ensuring no output save is treated as rewritten PDF bytes.

Run:

```bash
cargo test -p mdid-browser --lib pdf_runtime_response_with_rewritten_bytes_still_downloads_review_report_only -- --nocapture
cargo test -p mdid-browser --lib pdf_review_report_includes_rewrite_status -- --nocapture
cargo test -p mdid-desktop --lib pdf_review_mode_does_not_treat_runtime_rewrite_bytes_as_export -- --nocapture
```

- [ ] **Step 5: Broader verification for Task 4**

Run:

```bash
cargo test -p mdid-domain --test pdf_workflow_models
cargo test -p mdid-adapters --test pdf_adapter
cargo test -p mdid-application --test pdf_deidentification
cargo test -p mdid-runtime --test runtime_http pdf -- --nocapture
cargo test -p mdid-browser --lib pdf -- --nocapture
cargo test -p mdid-desktop --lib pdf -- --nocapture
```

---

## Task 5: Vault audit pagination offset/cursor

**Files:**
- Test: `crates/mdid-runtime/tests/runtime_http.rs`
- Test: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/cli_smoke.rs`
- Test: `crates/mdid-browser/src/app.rs`
- Modify: `crates/mdid-runtime/src/http.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-browser/src/app.rs`
- Modify: `crates/mdid-desktop/src/lib.rs` only if desktop exposes vault audit request helpers

- [ ] **Step 1: Define pagination contract**

Use offset pagination first because current `/vault/audit/events` already accepts JSON body filters and uses reverse chronological order:

Request additions to `VaultAuditEventsRequest`:

- `offset: Option<usize>`; default `0`, max is unbounded but saturating against event count.
- Keep `limit` default `100` and max `100` via existing `deserialize_optional_limit`/`min(100)` pattern.

Response additions to `VaultAuditEventsResponse`:

- `total_matching_events: usize`
- `limit: usize`
- `offset: usize`
- `next_offset: Option<usize>`
- `has_more: bool`

Filtering order must remain: unlock vault -> reverse chronological -> apply kind/actor filters -> count total matching -> skip offset -> take limit.

- [ ] **Step 2: Write failing runtime pagination tests**

In `crates/mdid-runtime/tests/runtime_http.rs`, add:

- `vault_audit_events_endpoint_paginates_with_offset`
  - Create/populate a vault with at least 5 audit events using existing helper patterns.
  - POST `/vault/audit/events` with `limit: 2, offset: 0` and assert 2 newest events, `total_matching_events: 5`, `next_offset: 2`, `has_more: true`.
  - POST with `limit: 2, offset: 2` and assert next 2 events, `next_offset: 4`, `has_more: true`.
  - POST with `limit: 2, offset: 4` and assert final 1 event, `next_offset: null`, `has_more: false`.
- `vault_audit_events_endpoint_applies_offset_after_filters`
  - Mix `AuditEventKind::Encode` and `AuditEventKind::Decode` events.
  - POST with `kind: "encode", limit: 1, offset: 1`.
  - Assert total matching counts only encode events and returned event is the second newest encode event.
- `vault_audit_events_endpoint_rejects_invalid_offset_type`
  - POST `offset: "not-a-number"`.
  - Assert `400` with `invalid_audit_events_request` and no event data.

Run: `cargo test -p mdid-runtime --test runtime_http vault_audit_events_endpoint_paginates_with_offset -- --nocapture`

Expected RED because current route ignores offset and response lacks pagination metadata.

- [ ] **Step 3: Implement runtime pagination**

In `crates/mdid-runtime/src/http.rs`:

- Add to `VaultAuditEventsRequest`:

```rust
#[serde(default, deserialize_with = "deserialize_optional_limit")]
offset: Option<usize>,
```

If `deserialize_optional_limit` is semantically named for limit, either rename it to `deserialize_optional_usize` and update both fields, or add a second helper `deserialize_optional_offset` with same parsing but no max.

- Update `VaultAuditEventsResponse` with pagination fields above.
- Replace `.take(limit)` pipeline with collecting filtered matching events first, then count, skip, take.
- Compute:

```rust
let offset = payload.offset.unwrap_or(0);
let has_more = offset.saturating_add(events.len()) < total_matching_events;
let next_offset = has_more.then_some(offset + events.len());
```

Use `saturating_add` for comparisons; do not panic for very large offsets.

Run all three runtime tests until GREEN.

- [ ] **Step 4: Add CLI audit pagination flags/output if CLI exposes vault audit**

In `crates/mdid-cli/src/main.rs`:

- Add `--offset <N>` to the existing vault audit command args alongside `--limit`.
- Update `build_vault_audit_report` signature from `(&[AuditEvent], Option<usize>)` to `(&[AuditEvent], Option<usize>, Option<usize>)` or use a small options struct.
- Preserve existing default/max limit behavior.
- Add `total_matching_events`, `limit`, `offset`, `next_offset`, and `has_more` to JSON output.
- Existing tests `vault_audit_report_applies_default_and_max_limit_bounds` and `vault_audit_report_returns_multiple_events_in_reverse_chronological_order` must be updated to pass `None` offset and assert metadata.
- Add `vault_audit_report_paginates_with_offset` asserting the second page returns expected events.

Run:

```bash
cargo test -p mdid-cli --bin mdid-cli vault_audit_report_paginates_with_offset -- --nocapture
cargo test -p mdid-cli --bin mdid-cli vault_audit_report -- --nocapture
```

If CLI smoke coverage has a vault audit command fixture, add a smoke test in `crates/mdid-cli/tests/cli_smoke.rs` using `--limit 1 --offset 1` and assert JSON `offset` and `has_more`.

- [ ] **Step 5: Add browser audit request pagination controls/state**

In `crates/mdid-browser/src/app.rs`:

- Extend `VaultAuditEvents` form state with an offset field if the browser mode currently exposes limit/kind/actor.
- Validate offset as non-negative integer text; blank means `0`/omit.
- Include `offset` in the request body only when non-zero, mirroring current optional `limit` handling.
- Parse/display pagination metadata in the review/report JSON.
- Add tests:
  - `vault_audit_request_includes_offset_when_provided`
  - `vault_audit_request_rejects_invalid_offset`
  - `vault_audit_events_report_includes_pagination_metadata`

Run:

```bash
cargo test -p mdid-browser --lib vault_audit_request_includes_offset_when_provided -- --nocapture
cargo test -p mdid-browser --lib vault_audit_request_rejects_invalid_offset -- --nocapture
cargo test -p mdid-browser --lib vault_audit_events_report_includes_pagination_metadata -- --nocapture
```

- [ ] **Step 6: Broader verification for Task 5**

Run:

```bash
cargo test -p mdid-runtime --test runtime_http vault_audit -- --nocapture
cargo test -p mdid-cli --bin mdid-cli vault_audit -- --nocapture
cargo test -p mdid-cli --test cli_smoke vault_audit -- --nocapture
cargo test -p mdid-browser --lib vault_audit -- --nocapture
```

---

## Final integration and truth-sync

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Run focused package suites**

Run:

```bash
cargo test -p mdid-domain --tests
cargo test -p mdid-adapters --tests
cargo test -p mdid-application --tests
cargo test -p mdid-runtime --test runtime_http
cargo test -p mdid-cli --bin mdid-cli
cargo test -p mdid-browser --lib
cargo test -p mdid-desktop --lib
```

- [ ] **Step 2: Run workspace verification**

Run:

```bash
cargo test --workspace
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
git diff --check
```

- [ ] **Step 3: Update README completion snapshot**

Update only the current repository status/completion snapshot. Mention:

- XLSX responses disclose first non-empty worksheet processing and selected-sheet scope while preserving first non-empty sheet behavior.
- Conservative media route rejects byte/base64 payload attempts with PHI-safe errors and remains metadata-only.
- DICOM responses disclose that burned-in/pixel PHI visual review is required and pixel redaction is not performed.
- PDF responses explicitly report review-only/no rewritten PDF status.
- Vault audit events support offset pagination with total/next-page metadata.

Include verification evidence from the commands above. Adjust percentages only if the README's existing completion rubric says these slices remove a larger blocker; otherwise state that percentages are unchanged because this is safety disclosure/completion depth.

- [ ] **Step 4: Final commit sequence**

Use small commits by slice:

```bash
git add crates/mdid-domain crates/mdid-adapters crates/mdid-application crates/mdid-runtime crates/mdid-browser crates/mdid-desktop
git commit -m "feat(core): disclose xlsx worksheet processing scope"

git add crates/mdid-runtime crates/mdid-adapters crates/mdid-application crates/mdid-browser crates/mdid-desktop
git commit -m "feat(media): reject byte payloads on metadata route"

git add crates/mdid-domain crates/mdid-adapters crates/mdid-application crates/mdid-runtime crates/mdid-browser crates/mdid-desktop
git commit -m "feat(dicom): disclose pixel redaction status"

git add crates/mdid-domain crates/mdid-adapters crates/mdid-application crates/mdid-runtime crates/mdid-browser crates/mdid-desktop
git commit -m "feat(pdf): report review-only rewrite status"

git add crates/mdid-runtime crates/mdid-cli crates/mdid-browser crates/mdid-desktop
git commit -m "feat(vault): paginate audit events"

git add README.md
git commit -m "docs: truth-sync core safety disclosures"
```

If an implementation finds a slice already present, do not rewrite production code. Add the named hardening tests, ensure copy/response consistency across runtime/browser/desktop surfaces, run the same verification commands, and commit with the closest message above using `test(...)` or `docs(...)` scope as appropriate.
