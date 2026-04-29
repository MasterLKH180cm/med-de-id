# Browser Portable Source-Aware Report Filenames Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make bounded browser portable artifact inspect/import JSON response downloads use sanitized source-aware filenames when the user has imported a portable `.json` artifact.

**Architecture:** Keep the behavior entirely in the browser surface filename helper; do not alter runtime payloads, vault semantics, portable artifact contents, or PHI-safe report bodies. Reuse the existing `imported_file_name` and `sanitized_import_stem` path that already powers CSV/XLSX/PDF/DICOM/media browser download names.

**Tech Stack:** Rust workspace, `mdid-browser`, unit tests in `crates/mdid-browser/src/app.rs`, Cargo test/clippy.

---

## File Structure

- Modify: `crates/mdid-browser/src/app.rs`
  - `BrowserFlowState::suggested_export_file_name` owns browser download filename selection.
  - Existing browser tests in the same file cover prepared download payload naming and PHI-safe JSON report bodies.
- Modify: `README.md`
  - Truth-sync current repository status after the landed browser filename helper is verified.

### Task 1: Browser portable artifact response downloads use source-aware safe filenames

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`
- Test: `crates/mdid-browser/src/app.rs`

- [ ] **Step 1: Write the failing test**

Add this test inside the existing `#[cfg(test)] mod tests` in `crates/mdid-browser/src/app.rs`, near the existing portable download tests:

```rust
    #[test]
    fn browser_portable_response_downloads_use_safe_source_filenames() {
        let mut inspect_state = BrowserFlowState {
            input_mode: InputMode::PortableArtifactInspect,
            summary: "2 portable record(s) available for import.".to_string(),
            review_queue: "Portable artifact preview: values hidden in browser report.".to_string(),
            result_output: "Portable artifact contains 2 record(s). Artifact contents are hidden."
                .to_string(),
            ..BrowserFlowState::default()
        };
        inspect_state.imported_file_name = Some("../Clinic Export 2026.JSON".to_string());

        let inspect_payload = inspect_state
            .prepared_download_payload()
            .expect("inspect payload");

        assert_eq!(
            inspect_payload.file_name,
            "clinic-export-2026-portable-artifact-inspect.json"
        );
        assert_eq!(inspect_payload.mime_type, "application/json;charset=utf-8");
        assert!(inspect_payload.is_text);

        let mut import_state = BrowserFlowState {
            input_mode: InputMode::PortableArtifactImport,
            summary: "Imported 2 portable record(s); skipped 0 duplicate(s).".to_string(),
            review_queue: "Portable import response: artifact contents hidden.".to_string(),
            result_output: "Portable import completed; raw artifact payload hidden.".to_string(),
            ..BrowserFlowState::default()
        };
        import_state.imported_file_name = Some("Patient Bundle!!.json".to_string());

        let import_payload = import_state
            .prepared_download_payload()
            .expect("import payload");

        assert_eq!(
            import_payload.file_name,
            "patient-bundle-portable-artifact-import.json"
        );
        assert_eq!(import_payload.mime_type, "application/json;charset=utf-8");
        assert!(import_payload.is_text);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test -p mdid-browser browser_portable_response_downloads_use_safe_source_filenames -- --nocapture
```

Expected: FAIL because `PortableArtifactInspect` still returns `mdid-browser-portable-artifact-inspect.json` and `PortableArtifactImport` still returns `mdid-browser-portable-artifact-import.json` even when `imported_file_name` is set.

- [ ] **Step 3: Write minimal implementation**

In `BrowserFlowState::suggested_export_file_name`, extend the existing imported-file branch with only these two source-aware portable response filenames:

```rust
                InputMode::PortableArtifactInspect => {
                    return format!("{stem}-portable-artifact-inspect.json");
                }
                InputMode::PortableArtifactImport => {
                    return format!("{stem}-portable-artifact-import.json");
                }
```

Keep `VaultAuditEvents`, `VaultDecode`, and `VaultExport` on their existing default filenames because they are vault/path driven or artifact-producing rather than imported-artifact response reports.

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
cargo test -p mdid-browser browser_portable_response_downloads_use_safe_source_filenames -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Run relevant broader verification**

Run:

```bash
cargo test -p mdid-browser --lib
cargo clippy -p mdid-browser --all-targets -- -D warnings
git diff --check
```

Expected: all PASS with no warnings and no whitespace errors.

- [ ] **Step 6: Commit**

```bash
git add crates/mdid-browser/src/app.rs
git commit -m "feat(browser): add source-aware portable report filenames"
```

### Task 2: README truth-sync for browser portable source-aware filenames

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update completion snapshot text**

Update the `Current repository status` snapshot date/context, Browser/web row, Overall row, verification evidence, and missing-items text to truthfully mention that browser portable artifact inspect/import response report JSON downloads now use sanitized source-aware filenames when backed by an imported artifact name. Do not raise completion percentages unless the landed feature materially removes a larger blocker; this slice is polish, so keep Overall at 93% unless fresh evidence supports a different number.

- [ ] **Step 2: Verify README mentions the new landed slice and keeps scope boundaries**

Run:

```bash
grep -n "source-aware" README.md
grep -n "Overall | 93%" README.md
git diff -- README.md
```

Expected: README mentions the browser portable source-aware filename helper, keeps Overall at 93%, and does not claim OCR, visual redaction, full PDF/media rewrite/export, generalized transfer workflow UX, auth/session handling, or agent/controller platform behavior.

- [ ] **Step 3: Commit README truth-sync**

```bash
git add README.md
git commit -m "docs: truth-sync browser portable report filenames"
```
