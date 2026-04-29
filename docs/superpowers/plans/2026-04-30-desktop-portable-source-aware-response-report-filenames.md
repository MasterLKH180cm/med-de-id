# Desktop Portable Source-Aware Response Report Filenames Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add bounded desktop source-aware safe filenames for PHI-safe portable artifact inspect/import response report JSON saves.

**Compliance follow-up scope:** The helper is intentionally bounded to `InspectArtifact` and `ImportArtifact` responses only. Decode, audit, and export responses must not receive portable source-aware response report filenames from this helper; broader `main.rs` save-path rewiring is out of scope for this narrow helper-layer compliance fix.

**Architecture:** Extend the existing `DesktopVaultResponseState` report-download helper layer without changing runtime routes or PHI-safe report contents. The desktop app should derive report filenames from an imported portable artifact filename when available, sanitize the stem, and fall back to the existing generic desktop vault/portable response report name when no source filename is available.

**Tech Stack:** Rust workspace, `mdid-desktop` crate, cargo tests/clippy, existing desktop helper-layer patterns.

---

## File Structure

- Modify: `crates/mdid-desktop/src/lib.rs`
  - Add a focused source-aware safe response report filename helper on `DesktopVaultResponseState`.
  - Reuse the existing `safe_source_file_stem` sanitization helper.
  - Add tests proving portable inspect/import report filenames use sanitized source stems and that PHI-safe JSON contents do not include source paths or artifact payloads.
- Modify: `crates/mdid-desktop/src/main.rs`
  - Use the helper when saving desktop vault/portable response reports so imported portable artifact source names can drive a safe default save filename.
- Modify: `README.md`
  - Truth-sync completion snapshot and verification evidence after the landed desktop portable filename helper.

### Task 1: Desktop portable source-aware response report filenames

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`
- Modify: `crates/mdid-desktop/src/main.rs`
- Test: `crates/mdid-desktop/src/lib.rs` module tests
- Modify: `README.md`

- [x] **Step 1: Write the failing test**

Add this test to the existing `#[cfg(test)]` module in `crates/mdid-desktop/src/lib.rs` near other desktop vault/portable response report tests:

```rust
#[test]
fn desktop_portable_response_report_for_source_uses_safe_imported_artifact_filename() {
    let mut inspect_state = DesktopVaultResponseState::default();
    inspect_state.apply_success(
        DesktopVaultResponseMode::InspectArtifact,
        &serde_json::json!({
            "record_count": 2,
            "records": [{"record_id": "patient-123", "token": "tok-secret"}],
            "artifact_path": "C:\\vaults\\sensitive\\Clinic Batch.mdid-portable.json"
        }),
    );

    let inspect_report = inspect_state
        .safe_response_report_download_for_source(Some("C:\\vaults\\sensitive\\Clinic Batch.mdid-portable.json"))
        .expect("portable inspect response should create a safe report download");
    assert_eq!(
        inspect_report.file_name,
        "Clinic-Batch.mdid-portable-portable-response-report.json"
    );
    let inspect_text = std::str::from_utf8(&inspect_report.bytes).expect("report is utf8 json");
    assert!(inspect_text.contains("bounded portable artifact response rendered locally"));
    assert!(inspect_text.contains("artifact path returned; full path hidden"));
    assert!(!inspect_text.contains("Clinic Batch"));
    assert!(!inspect_text.contains("patient-123"));
    assert!(!inspect_text.contains("tok-secret"));

    let mut import_state = DesktopVaultResponseState::default();
    import_state.apply_success(
        DesktopVaultResponseMode::ImportArtifact,
        &serde_json::json!({
            "imported_record_count": 1,
            "duplicate_record_count": 1,
            "artifact_path": "/tmp/Partner Export.mdid-portable.json"
        }),
    );

    let import_report = import_state
        .safe_response_report_download_for_source(Some("/tmp/Partner Export.mdid-portable.json"))
        .expect("portable import response should create a safe report download");
    assert_eq!(
        import_report.file_name,
        "Partner-Export.mdid-portable-portable-response-report.json"
    );
}
```

- [x] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test -p mdid-desktop desktop_portable_response_report_for_source_uses_safe_imported_artifact_filename -- --nocapture
```

Expected: FAIL with a missing method error for `safe_response_report_download_for_source`.

- [x] **Step 3: Write minimal implementation**

In `crates/mdid-desktop/src/lib.rs`, add this helper struct near `DesktopWorkflowReviewReportDownload` or reuse that struct if keeping one PHI-safe JSON download shape is clearer:

```rust
#[derive(Clone, PartialEq, Eq)]
pub struct DesktopSafeResponseReportDownload {
    pub file_name: String,
    pub bytes: Vec<u8>,
}

impl std::fmt::Debug for DesktopSafeResponseReportDownload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DesktopSafeResponseReportDownload")
            .field("file_name", &self.file_name)
            .field("bytes", &"<redacted>")
            .finish()
    }
}
```

Then add this method to `impl DesktopVaultResponseState`:

```rust
pub fn safe_response_report_download_for_source(
    &self,
    source_name: Option<&str>,
) -> Result<DesktopSafeResponseReportDownload, DesktopPortableArtifactSaveError> {
    let report_json = serde_json::to_string_pretty(&self.safe_response_report_json()?)
        .map_err(|error| DesktopPortableArtifactSaveError::InvalidJson(error.to_string()))?;
    let stem = source_name
        .and_then(safe_source_file_stem)
        .unwrap_or_else(|| "desktop-vault".to_string());

    Ok(DesktopSafeResponseReportDownload {
        file_name: format!("{stem}-portable-response-report.json"),
        bytes: report_json.into_bytes(),
    })
}
```

In `crates/mdid-desktop/src/main.rs`, update `save_safe_vault_response_report` so it computes the suggested filename from the current portable import/inspect source when available and uses that helper for its report bytes/path behavior without exposing source paths in the JSON. If the UI still lets the user override `vault_response_report_save_path`, only replace the default path when the current field is blank or still equals `desktop-vault-response-report.json`.

- [x] **Step 4: Run targeted tests to verify pass**

Run:

```bash
cargo test -p mdid-desktop desktop_portable_response_report_for_source_uses_safe_imported_artifact_filename -- --nocapture
```

Expected: PASS.

- [x] **Step 5: Run broader desktop verification**

Run:

```bash
cargo test -p mdid-desktop --lib
cargo clippy -p mdid-desktop --all-targets -- -D warnings
git diff --check
```

Expected: all PASS with no warnings and no whitespace errors.

- [x] **Step 6: README truth-sync**

Update `README.md` Current repository status to mention that desktop portable inspect/import response report saves now use sanitized imported portable artifact stems when available, keep overall completion at 93% unless a broader landed feature justifies a truthful increase, and add the test/clippy commands from Step 5 to verification evidence.

- [x] **Step 7: Commit**

Run:

```bash
git add crates/mdid-desktop/src/lib.rs README.md docs/superpowers/plans/2026-04-30-desktop-portable-source-aware-response-report-filenames.md
git commit -m "fix(desktop): bound portable response report filenames"
```

Expected: commit succeeds on a feature branch or develop integration branch per GitFlow.

## Self-Review

- Spec coverage: The plan covers source-aware safe filenames for desktop portable inspect/import response report JSON saves, tests PHI-safe contents, updates UI save behavior, verifies with cargo test/clippy/diff-check, and truth-syncs README.
- Placeholder scan: No TBD/TODO/implement-later placeholders remain.
- Type consistency: The new `DesktopSafeResponseReportDownload` and `safe_response_report_download_for_source` names are used consistently in the test and implementation step.
