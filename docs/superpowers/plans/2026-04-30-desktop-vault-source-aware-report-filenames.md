# Desktop Vault Source-Aware Report Filenames Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the desktop PHI-safe vault response report download helper so all already-rendered vault/portable response reports can use a sanitized source-aware filename when the caller provides one.

**Architecture:** Keep behavior inside `crates/mdid-desktop/src/lib.rs` next to the existing `DesktopVaultResponseState` safe report helpers. Reuse the existing PHI-safe report envelope and `safe_source_file_stem` sanitizer; only change filename derivation, not report contents or runtime behavior.

**Tech Stack:** Rust workspace, mdid-desktop crate tests, `serde_json`, existing desktop safe writer helpers.

---

## File Structure

- Modify: `crates/mdid-desktop/src/lib.rs`
  - Add tests in the existing `#[cfg(test)]` module for source-aware desktop vault decode/audit/export report filenames.
  - Relax `DesktopVaultResponseState::safe_response_report_download_for_source` so it works for any rendered safe report mode, while preserving PHI-safe JSON bytes.
- Modify: `README.md`
  - Truth-sync desktop/browser/CLI/overall completion after the landed behavior and verification evidence.

### Task 1: Desktop safe vault report source-aware filenames for all modes

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`
- Test: `crates/mdid-desktop/src/lib.rs` existing unit test module

- [ ] **Step 1: Write the failing tests**

Add these tests in the existing `#[cfg(test)]` module in `crates/mdid-desktop/src/lib.rs` near the existing vault response report download tests:

```rust
#[test]
fn safe_response_report_download_uses_source_stem_for_vault_decode() {
    let mut state = DesktopVaultResponseState::default();
    state.apply_success(
        DesktopVaultResponseMode::VaultDecode,
        &serde_json::json!({ "decoded_value_count": 2 }),
    );

    let download = state
        .safe_response_report_download_for_source(Some("C:/Vault Exports/Patient Alpha.mdid-vault.json"))
        .expect("decode report download should be available");

    assert_eq!(
        download.file_name,
        "Patient_Alpha.mdid-vault-response-report.json"
    );
    let report: serde_json::Value = serde_json::from_slice(&download.bytes).unwrap();
    assert_eq!(report["mode"], "vault-decode");
    assert_eq!(report["summary"], "decoded values: 2");
}

#[test]
fn safe_response_report_download_uses_source_stem_for_vault_audit() {
    let mut state = DesktopVaultResponseState::default();
    state.apply_success(
        DesktopVaultResponseMode::VaultAudit,
        &serde_json::json!({ "returned_event_count": 3, "event_count": 8 }),
    );

    let download = state
        .safe_response_report_download_for_source(Some("audit export.json"))
        .expect("audit report download should be available");

    assert_eq!(download.file_name, "audit_export-response-report.json");
    let report: serde_json::Value = serde_json::from_slice(&download.bytes).unwrap();
    assert_eq!(report["mode"], "vault-audit");
    assert_eq!(report["summary"], "events returned: 3 / 8");
}

#[test]
fn safe_response_report_download_uses_source_stem_for_vault_export() {
    let mut state = DesktopVaultResponseState::default();
    state.apply_success(
        DesktopVaultResponseMode::VaultExport,
        &serde_json::json!({ "record_count": 4, "artifact_path": "/sensitive/path/export.json" }),
    );

    let download = state
        .safe_response_report_download_for_source(Some("portable subset.mdid-portable.json"))
        .expect("export report download should be available");

    assert_eq!(
        download.file_name,
        "portable_subset.mdid-portable-response-report.json"
    );
    let report: serde_json::Value = serde_json::from_slice(&download.bytes).unwrap();
    assert_eq!(report["mode"], "vault-export");
    assert_eq!(report["summary"], "records: 4");
    assert_eq!(report["artifact_notice"], "artifact path returned; full path hidden");
    assert!(report.get("artifact_path").is_none());
}
```

- [ ] **Step 2: Run tests to verify RED**

Run:

```bash
cargo test -p mdid-desktop --lib safe_response_report_download_uses_source_stem_for_vault -- --nocapture
```

Expected: the new tests fail because `safe_response_report_download_for_source` currently rejects vault decode/audit/export modes with `NotVaultExport`.

- [ ] **Step 3: Implement minimal production change**

Replace the start of `DesktopVaultResponseState::safe_response_report_download_for_source` with this implementation:

```rust
    pub fn safe_response_report_download_for_source(
        &self,
        source_name: Option<&str>,
    ) -> Result<DesktopVaultResponseReportDownload, DesktopPortableArtifactSaveError> {
        let mode = self
            .safe_response_report_mode()
            .ok_or(DesktopPortableArtifactSaveError::MissingArtifact)?;
        let json = serde_json::to_string_pretty(&self.safe_export_json(mode))
            .map_err(|error| DesktopPortableArtifactSaveError::InvalidJson(error.to_string()))?;
        let stem = source_name
            .and_then(safe_source_file_stem)
            .unwrap_or_else(|| "desktop".to_string());

        Ok(DesktopVaultResponseReportDownload {
            file_name: format!("{stem}-response-report.json"),
            bytes: json.into_bytes(),
        })
    }
```

This keeps report bytes PHI-safe by continuing to serialize `safe_export_json(mode)` and only changes the safe filename suffix to the general `-response-report.json` for all desktop vault/portable response modes.

- [ ] **Step 4: Run targeted tests to verify GREEN**

Run:

```bash
cargo test -p mdid-desktop --lib safe_response_report_download_uses_source_stem_for_vault -- --nocapture
```

Expected: PASS, with the three new tests passing.

- [ ] **Step 5: Run relevant broader desktop tests**

Run:

```bash
cargo test -p mdid-desktop --lib safe_response_report -- --nocapture
cargo test -p mdid-desktop --bin mdid-desktop vault_response_report -- --nocapture
```

Expected: PASS. Existing tests that asserted the old inspect/import-only `-portable-response-report.json` suffix should be updated only if they are testing the helper’s intended generalized safe-report filename behavior; report bytes must remain PHI-safe.

- [ ] **Step 6: Commit**

Run:

```bash
git add crates/mdid-desktop/src/lib.rs
git commit -m "feat(desktop): generalize safe vault report filenames"
```

### Task 2: README truth-sync for desktop report filename completion

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update completion snapshot**

Update `README.md` current repository status so it truthfully states that desktop already-rendered vault/portable response report JSON downloads now use sanitized source-aware filenames for all vault/portable response modes when a source filename is supplied. Keep CLI at 95%, browser/web at 75%, desktop at 69%, and overall at 93% unless additional verified landed functionality justifies a change.

- [ ] **Step 2: Run verification evidence commands**

Run:

```bash
cargo test -p mdid-desktop --lib safe_response_report -- --nocapture
cargo test -p mdid-desktop --bin mdid-desktop vault_response_report -- --nocapture
```

Expected: PASS.

- [ ] **Step 3: Commit**

Run:

```bash
git add README.md docs/superpowers/plans/2026-04-30-desktop-vault-source-aware-report-filenames.md
git commit -m "docs: truth-sync desktop vault report filename completion"
```
