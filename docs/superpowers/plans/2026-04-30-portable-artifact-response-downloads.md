# Portable Artifact Response Downloads Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add bounded, PHI-safe portable artifact response report/download helpers to both browser/web and desktop surfaces so browser portable inspect/import responses and desktop portable export/inspect/import responses can be saved separately from high-risk artifact JSON; browser vault export remains the explicit encrypted artifact download path.

**Architecture:** Keep this as a local-first surface UX slice only: browser builds sanitized JSON report payloads from existing runtime-shaped portable inspect/import responses, while desktop builds sanitized JSON report payloads from portable export/inspect/import responses; both suggest source-derived safe filenames. Do not add agent/controller/workflow orchestration semantics, vault browsing, decoded value display, auth/session, or background coordination.

**Tech Stack:** Rust workspace, Leptos browser crate helpers, desktop Rust helper library, serde_json, cargo test.

---

## File Structure

- Modify `crates/mdid-browser/src/app.rs`: add portable response report payload and filename helper tests/functions; wire successful portable modes to expose a structured report download action.
- Modify `crates/mdid-desktop/src/lib.rs`: add desktop portable response report save helper tests/functions; expose PHI-safe save suggestion/status for portable runtime responses.
- Modify `README.md`: truth-sync completion snapshot after verified landed functionality.

### Task 1: Browser Portable Response Report Download

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`
- Test: existing unit tests in `crates/mdid-browser/src/app.rs`

- [ ] **Step 1: Write the failing browser tests**

Add tests that call a new helper named `build_portable_response_report_download` and assert it returns a JSON download payload for portable inspect/import/export responses with sanitized source-derived names, redacted artifact fields, mode labels, and no raw decoded values.

```rust
#[test]
fn portable_response_report_download_uses_safe_source_name_and_redacts_artifact() {
    let payload = build_portable_response_report_download(
        InputMode::PortableArtifactImport,
        Some("Patient Alice bundle.mdid-portable.json"),
        r#"{"artifact":{"records":[{"id":"phi-1"}]},"imported_record_count":1,"audit_event_count":2}"#,
    )
    .expect("portable import response should produce report download");

    assert_eq!(
        payload.file_name,
        "Patient_Alice_bundle-portable-artifact-import-report.json"
    );
    assert_eq!(payload.mime_type, "application/json");
    assert!(payload.is_text);
    let report: serde_json::Value = serde_json::from_slice(&payload.bytes).unwrap();
    assert_eq!(report["mode"], "portable_artifact_import");
    assert_eq!(report["imported_record_count"], 1);
    assert_eq!(report["audit_event_count"], 2);
    assert_eq!(report["artifact"], "redacted");
    assert!(!String::from_utf8(payload.bytes).unwrap().contains("phi-1"));
}

#[test]
fn portable_response_report_download_rejects_non_portable_modes() {
    let error = build_portable_response_report_download(
        InputMode::CsvText,
        Some("rows.csv"),
        r#"{"summary":"ok"}"#,
    )
    .unwrap_err();

    assert_eq!(error, "Portable response report download is only available for portable artifact modes.");
}
```

- [ ] **Step 2: Run browser RED**

Run: `cargo test -p mdid-browser portable_response_report_download -- --nocapture`
Expected: FAIL because `build_portable_response_report_download` is not defined.

- [ ] **Step 3: Implement minimal browser helper and wire action**

Implement `build_portable_response_report_download(mode, imported_file_name, response_json)` returning `BrowserDownloadPayload` only for `PortableArtifactInspect` and `PortableArtifactImport`; do not route `VaultExport` through this helper because browser vault export is the intentional encrypted portable artifact download. Parse response JSON object, replace any `artifact`, `decoded_values`, `records`, or `vault_passphrase` fields with string `"redacted"`, add `mode`, and use sanitized source stem plus `-portable-artifact-<mode>-report.json`. Wire the successful browser portable response area to expose this structured report download separately from any artifact/decoded-value export.

- [ ] **Step 4: Run browser GREEN and broader tests**

Run: `cargo test -p mdid-browser portable_response_report_download -- --nocapture`
Expected: PASS.
Run: `cargo test -p mdid-browser --lib`
Expected: PASS.

- [ ] **Step 5: Commit browser task**

```bash
git add crates/mdid-browser/src/app.rs
git commit -m "feat(browser): add portable response report downloads"
```

### Task 2: Desktop Portable Response Report Save

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`
- Test: existing unit tests in `crates/mdid-desktop/src/lib.rs`

- [ ] **Step 1: Write the failing desktop tests**

Add tests that call a new helper named `build_desktop_portable_response_report_save` and assert it returns a PHI-safe save payload/status for portable runtime responses.

```rust
#[test]
fn desktop_portable_response_report_save_uses_safe_source_name_and_redacts_artifact() {
    let payload = build_desktop_portable_response_report_save(
        DesktopPortableMode::ImportArtifact,
        Some("Alice portable.mdid-portable.json"),
        r#"{"artifact":{"records":[{"id":"phi-1"}]},"imported_record_count":1,"audit_event_count":2}"#,
    )
    .expect("portable import response should produce report save payload");

    assert_eq!(
        payload.suggested_file_name,
        "Alice_portable-portable-artifact-import-report.json"
    );
    assert_eq!(payload.mime_type, "application/json");
    assert_eq!(payload.status, "Portable artifact import report ready to save; artifact and decoded values are redacted from this report.");
    let report: serde_json::Value = serde_json::from_str(&payload.contents).unwrap();
    assert_eq!(report["mode"], "portable_artifact_import");
    assert_eq!(report["imported_record_count"], 1);
    assert_eq!(report["audit_event_count"], 2);
    assert_eq!(report["artifact"], "redacted");
    assert!(!payload.contents.contains("phi-1"));
}

#[test]
fn desktop_portable_response_report_save_rejects_invalid_json() {
    let error = build_desktop_portable_response_report_save(
        DesktopPortableMode::InspectArtifact,
        Some("portable.mdid-portable.json"),
        "not-json",
    )
    .unwrap_err();

    assert_eq!(error, DesktopPortableReportSaveError::InvalidResponseJson);
}
```

- [ ] **Step 2: Run desktop RED**

Run: `cargo test -p mdid-desktop portable_response_report_save -- --nocapture`
Expected: FAIL because `build_desktop_portable_response_report_save` is not defined.

- [ ] **Step 3: Implement minimal desktop helper and status**

Create a public `DesktopPortableReportSavePayload { suggested_file_name: String, mime_type: &'static str, contents: String, status: String }` plus `DesktopPortableReportSaveError`. Implement `build_desktop_portable_response_report_save(mode, imported_file_name, response_json)` with the same redaction allowlist and filename convention as browser, using `desktop-portable-artifact-report.json` fallback. Do not expose raw artifact or decoded values in status/debug.

- [ ] **Step 4: Run desktop GREEN and broader tests**

Run: `cargo test -p mdid-desktop portable_response_report_save -- --nocapture`
Expected: PASS.
Run: `cargo test -p mdid-desktop --lib`
Expected: PASS.

- [ ] **Step 5: Commit desktop task**

```bash
git add crates/mdid-desktop/src/lib.rs
git commit -m "feat(desktop): add portable response report saves"
```

### Task 3: README Truth-Sync and Verification

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Run final verification**

Run:
```bash
cargo test -p mdid-browser portable_response_report_download -- --nocapture
cargo test -p mdid-browser --lib
cargo test -p mdid-desktop portable_response_report_save -- --nocapture
cargo test -p mdid-desktop --lib
cargo fmt --check
git diff --check
```
Expected: all PASS.

- [ ] **Step 2: Update README completion snapshot**

Update the completion snapshot to truthfully state: CLI remains 95%; Browser/Web increases from 80% to 85%; Desktop app increases from 72% to 77%; Overall increases from 93% to 94% if and only if the controller-visible verified commits land on this branch. Explain that the increase comes from separate PHI-safe portable response report save/download actions on both browser and desktop, not from generalized workflow/orchestration behavior.

- [ ] **Step 3: Commit README truth-sync**

```bash
git add README.md docs/superpowers/plans/2026-04-30-portable-artifact-response-downloads.md
git commit -m "docs: truth-sync portable response report downloads"
```
