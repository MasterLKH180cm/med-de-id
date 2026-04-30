# Desktop Vault Safe Response Download Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded desktop helper that writes PHI-safe vault decode/audit/import response reports to JSON files without raw decoded values, audit details, passphrases, paths, or portable artifact payloads.

**Architecture:** Reuse the existing `DesktopVaultResponseState::safe_export_json()` allowlisted summary object and add a small file-writing helper beside `write_portable_artifact_json()`. This is a desktop helper-layer feature only; it does not add new runtime routes, background workflows, controller semantics, or broaden vault browsing.

**Tech Stack:** Rust workspace, `mdid-desktop`, `serde_json`, Rust unit tests, Cargo.

---

## File Structure

- Modify: `crates/mdid-desktop/src/lib.rs`
  - Add failing tests in the existing `#[cfg(test)]` module near vault response export tests.
  - Add `write_safe_vault_response_json()` beside `write_portable_artifact_json()` to persist the existing PHI-safe response envelope.
- Modify: `README.md`
  - Truth-sync the completion snapshot after verification, without inflating unsupported features.

### Task 1: Desktop vault PHI-safe response JSON writer

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`
- Test: `crates/mdid-desktop/src/lib.rs`

- [x] **Step 1: Write the failing test**

Add this test inside the existing `#[cfg(test)] mod tests` in `crates/mdid-desktop/src/lib.rs`, near the existing vault safe export tests:

```rust
    #[test]
    fn safe_vault_response_json_writer_persists_allowlisted_audit_summary_only() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let target = temp_dir.path().join("vault-audit-report.json");
        let mut state = DesktopVaultResponseState::default();
        let response = serde_json::json!({
            "event_count": 4,
            "returned_event_count": 2,
            "events": [
                {"kind": "decode", "detail": "patient Alice decoded for oncology"},
                {"kind": "export", "detail": "exported C:/vaults/alice.mdid"}
            ],
            "vault_path": "C:/vaults/alice.mdid",
            "vault_passphrase": "correct horse battery staple"
        });
        state.apply_success(DesktopVaultResponseMode::VaultAudit, &response);

        let written_path = write_safe_vault_response_json(
            &state,
            DesktopVaultResponseMode::VaultAudit,
            &target,
        )
        .expect("safe vault response report should be written");

        assert_eq!(written_path, target);
        let persisted = std::fs::read_to_string(&written_path).expect("read report");
        assert!(persisted.contains("\"mode\": \"vault_audit\""));
        assert!(persisted.contains("events returned: 2 / 4"));
        assert!(!persisted.contains("patient Alice"));
        assert!(!persisted.contains("C:/vaults/alice.mdid"));
        assert!(!persisted.contains("correct horse battery staple"));
        assert!(!persisted.contains("\"events\""));
    }
```

- [x] **Step 2: Run test to verify it fails**

Run: `cargo test -p mdid-desktop safe_vault_response_json_writer_persists_allowlisted_audit_summary_only -- --nocapture`

Expected: FAIL with an unresolved function error for `write_safe_vault_response_json`.

Actual: the initial narrow audit writer test could not produce the planned unresolved-function RED because an earlier helper already existed. Follow-up default-state and mismatched-mode regression tests produced real RED before the validating helper contract was completed.

- [x] **Step 3: Write minimal implementation**

Add this public helper immediately after `write_portable_artifact_json()` in `crates/mdid-desktop/src/lib.rs`:

Actual: implementation landed as a validating helper that writes only the allowlisted safe export JSON and rejects non-renderable/mismatched state; `main.rs` caller changes were also needed so the workspace compiled against the updated helper contract.

```rust
pub fn write_safe_vault_response_json(
    state: &DesktopVaultResponseState,
    mode: DesktopVaultResponseMode,
    path: impl AsRef<std::path::Path>,
) -> Result<std::path::PathBuf, DesktopPortableArtifactSaveError> {
    let report_json = serde_json::to_string_pretty(&state.safe_export_json(mode))
        .map_err(|error| DesktopPortableArtifactSaveError::InvalidJson(error.to_string()))?;
    let path = path.as_ref();
    std::fs::write(path, report_json)
        .map_err(|error| DesktopPortableArtifactSaveError::Io(error.to_string()))?;
    Ok(path.to_path_buf())
}
```

- [x] **Step 4: Run test to verify it passes**

Run: `cargo test -p mdid-desktop safe_vault_response_json_writer_persists_allowlisted_audit_summary_only -- --nocapture`

Expected: PASS.

- [x] **Step 5: Run focused package checks**

Run: `cargo test -p mdid-desktop --lib`

Expected: PASS.

Run: `cargo clippy -p mdid-desktop --all-targets -- -D warnings`

Expected: PASS.

Run: `git diff --check`

Expected: no output and exit code 0.

- [x] **Step 6: Commit**

```bash
git add crates/mdid-desktop/src/lib.rs
git commit -m "feat(desktop): add safe vault response report writer"
```

### Task 2: README completion truth-sync

**Files:**
- Modify: `README.md`

- [x] **Step 1: Update completion snapshot text**

Update the README completion snapshot to mention the landed desktop PHI-safe vault response report JSON writer. Keep percentages honest: CLI unchanged at 95%, Browser/web unchanged at 73%, Desktop app increases only if the verified helper materially improves the bounded desktop vault workflow, and Overall may remain 93% if no broader user-facing runtime workflow changed.

- [x] **Step 2: Verify docs diff**

Run: `git diff -- README.md`

Expected: Diff only updates completion snapshot wording and verification evidence; no unsupported OCR/PDF rewrite/full vault UX claims.

Run: `git diff --check`

Expected: no output and exit code 0.

- [x] **Step 3: Commit**

```bash
git add README.md docs/superpowers/plans/2026-04-30-desktop-vault-safe-response-download.md
git commit -m "docs: truth-sync desktop vault report completion"
```
