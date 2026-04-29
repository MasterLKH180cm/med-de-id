# Browser Vault Source-Aware Report Filenames Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make browser vault audit/decode safe response report downloads use PHI-safe source-aware filenames when the browser state already has an imported vault/source filename.

**Architecture:** Reuse the existing `BrowserFlowState::suggested_export_file_name` filename-selection boundary and the existing `sanitized_import_stem` helper. Keep the change bounded to browser filename suggestion tests and logic; do not change runtime vault behavior, output envelope contents, portable transfer semantics, or any agent/controller workflow surface.

**Tech Stack:** Rust workspace, `mdid-browser` crate, existing Rust unit tests via `cargo test -p mdid-browser --lib`.

---

## File Structure

- Modify: `crates/mdid-browser/src/app.rs`
  - Add tests in the existing `tests` module near browser filename tests.
  - Extend `BrowserFlowState::suggested_export_file_name` so `VaultAuditEvents` and `VaultDecode` produce source-aware JSON report filenames when `imported_file_name` is present.
- Modify: `README.md`
  - Truth-sync the completion snapshot and browser/web row with the landed source-aware browser vault response filename evidence.

### Task 1: Browser vault audit/decode source-aware filenames

**Files:**
- Modify: `crates/mdid-browser/src/app.rs:711-744`
- Test: `crates/mdid-browser/src/app.rs` existing `#[cfg(test)] mod tests`

- [ ] **Step 1: Write the failing test**

Add this test near the existing `browser_portable_response_downloads_use_safe_source_filenames` test:

```rust
    #[test]
    fn browser_vault_response_downloads_use_safe_source_filenames() {
        let mut audit_state = BrowserFlowState {
            input_mode: InputMode::VaultAuditEvents,
            imported_file_name: Some("Clinic Vault Backup 2026.vault".to_string()),
            ..BrowserFlowState::default()
        };
        audit_state.summary = "events returned: 2 / 2".to_string();
        audit_state.review_queue = "audit event summaries available".to_string();
        audit_state.result_output = "safe summary".to_string();

        let audit_payload = audit_state
            .prepared_download_payload()
            .expect("vault audit payload should be prepared");
        assert_eq!(
            audit_payload.file_name,
            "clinic-vault-backup-2026-vault-audit-events.json"
        );
        assert_eq!(audit_payload.mime_type, "application/json;charset=utf-8");
        let audit_json = String::from_utf8(audit_payload.bytes).expect("audit json utf8");
        assert!(audit_json.contains("\"mode\": \"vault_audit\""));
        assert!(audit_json.contains("events returned: 2 / 2"));
        assert!(!audit_json.contains("safe summary"));

        let decode_state = BrowserFlowState {
            input_mode: InputMode::VaultDecode,
            imported_file_name: Some("Clinic Vault Backup 2026.vault".to_string()),
            summary: "decoded count: 1".to_string(),
            review_queue: "decoded PHI is not included in the safe report".to_string(),
            result_output: "Jane Doe".to_string(),
            ..BrowserFlowState::default()
        };

        let decode_payload = decode_state
            .prepared_download_payload()
            .expect("vault decode payload should be prepared");
        assert_eq!(
            decode_payload.file_name,
            "clinic-vault-backup-2026-vault-decode-response.json"
        );
        assert_eq!(decode_payload.mime_type, "application/json;charset=utf-8");
        let decode_json = String::from_utf8(decode_payload.bytes).expect("decode json utf8");
        assert!(decode_json.contains("\"mode\": \"vault_decode\""));
        assert!(decode_json.contains("decoded count: 1"));
        assert!(!decode_json.contains("Jane Doe"));
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test -p mdid-browser --lib browser_vault_response_downloads_use_safe_source_filenames -- --nocapture
```

Expected: FAIL because `suggested_export_file_name()` currently falls back to `mdid-browser-vault-audit-events.json` and `mdid-browser-vault-decode-response.json` for vault modes even when `imported_file_name` is present.

- [ ] **Step 3: Write minimal implementation**

In `BrowserFlowState::suggested_export_file_name`, replace the current vault no-op arm:

```rust
                InputMode::VaultAuditEvents | InputMode::VaultDecode | InputMode::VaultExport => {}
```

with:

```rust
                InputMode::VaultAuditEvents => {
                    return format!("{stem}-vault-audit-events.json");
                }
                InputMode::VaultDecode => {
                    return format!("{stem}-vault-decode-response.json");
                }
                InputMode::VaultExport => {}
```

- [ ] **Step 4: Run targeted test to verify it passes**

Run:

```bash
cargo test -p mdid-browser --lib browser_vault_response_downloads_use_safe_source_filenames -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Run browser crate tests**

Run:

```bash
cargo test -p mdid-browser --lib
```

Expected: PASS with all browser unit tests passing.

- [ ] **Step 6: Commit**

Run:

```bash
git add crates/mdid-browser/src/app.rs docs/superpowers/plans/2026-04-30-browser-vault-source-aware-report-filenames.md
git commit -m "feat(browser): add source-aware vault report filenames"
```

### Task 2: README completion truth-sync

**Files:**
- Modify: `README.md:64-77`

- [ ] **Step 1: Update README completion evidence**

Update the completion snapshot to state that this round landed browser source-aware vault audit/decode response report filenames. Keep overall completion at `93%` unless additional landed functionality justifies changing it. Update Browser/web to `75%` because browser download UX depth improved in a narrow but real way.

Use wording with these concrete facts:

```markdown
- Browser/web includes source-aware safe filenames for vault audit/decode safe response report JSON downloads when an imported vault/source filename is present.
- Verification evidence: `cargo test -p mdid-browser --lib browser_vault_response_downloads_use_safe_source_filenames -- --nocapture` and `cargo test -p mdid-browser --lib` passed on the feature branch before merge.
```

- [ ] **Step 2: Run README-related verification**

Run:

```bash
cargo test -p mdid-browser --lib browser_vault_response_downloads_use_safe_source_filenames -- --nocapture
```

Expected: PASS.

- [ ] **Step 3: Commit**

Run:

```bash
git add README.md
git commit -m "docs: truth-sync browser vault report filename completion"
```
