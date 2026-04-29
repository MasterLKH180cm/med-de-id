# Desktop Vault Safe Response Export Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded desktop helper that exports the already-rendered vault/portable response pane as PHI-safe JSON without raw decoded values, artifact contents, passphrases, or paths.

**Architecture:** Keep the feature entirely in `mdid-desktop` state helpers: `DesktopVaultResponseState` already contains only banner, summary, generic artifact notice, and redacted error text. Add a serialization helper that emits those safe fields plus an explicit mode label so desktop callers can save/share local verification evidence without exposing sensitive runtime payloads.

**Tech Stack:** Rust workspace, `mdid-desktop`, serde_json, cargo tests.

---

## File Structure

- Modify: `crates/mdid-desktop/src/lib.rs` — add `DesktopVaultResponseState::safe_export_json(mode)` plus tests proving success/error exports are PHI-safe.
- Modify: `README.md` — truth-sync desktop/browser/CLI/overall completion snapshot and missing items after the landed helper.

### Task 1: Add PHI-safe desktop vault response export helper

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`
- Test: `crates/mdid-desktop/src/lib.rs` unit tests

- [ ] **Step 1: Write the failing success-export test**

Add this test inside the existing `#[cfg(test)] mod tests` block in `crates/mdid-desktop/src/lib.rs`:

```rust
    #[test]
    fn vault_response_safe_export_omits_decoded_values_paths_and_raw_audit_detail() {
        let response = serde_json::json!({
            "decoded_value_count": 2,
            "report_path": "/sensitive/patient/alice-decode.json",
            "decoded_values": [
                {"record_id": "patient-1", "field": "name", "value": "Alice Example"}
            ],
            "audit_event": {"kind": "decode", "detail": "released Alice Example to oncology"}
        });
        let mut state = DesktopVaultResponseState::default();
        state.apply_success(DesktopVaultResponseMode::VaultDecode, &response);

        let exported = state.safe_export_json(DesktopVaultResponseMode::VaultDecode);
        let exported_text = serde_json::to_string(&exported).expect("safe export serializes");

        assert_eq!(exported["mode"], "vault_decode");
        assert_eq!(exported["banner"], "bounded vault decode response rendered locally");
        assert_eq!(exported["summary"], "decoded values: 2");
        assert_eq!(exported["artifact_notice"], "artifact path returned; full path hidden");
        assert_eq!(exported["error"], serde_json::Value::Null);
        assert!(!exported_text.contains("Alice Example"));
        assert!(!exported_text.contains("/sensitive/patient"));
        assert!(!exported_text.contains("released Alice"));
        assert!(!exported_text.contains("decoded_values"));
        assert!(!exported_text.contains("audit_event"));
    }
```

- [ ] **Step 2: Run the targeted test to verify RED**

Run: `cargo test -p mdid-desktop vault_response_safe_export_omits_decoded_values_paths_and_raw_audit_detail -- --nocapture`

Expected: FAIL because `safe_export_json` does not exist.

- [ ] **Step 3: Implement the minimal helper**

In `impl DesktopVaultResponseState`, add:

```rust
    pub fn safe_export_json(&self, mode: DesktopVaultResponseMode) -> serde_json::Value {
        serde_json::json!({
            "mode": mode.safe_export_label(),
            "banner": self.banner,
            "summary": self.summary,
            "artifact_notice": self.artifact_notice,
            "error": self.error,
        })
    }
```

Add this impl near `DesktopVaultResponseMode`:

```rust
impl DesktopVaultResponseMode {
    fn safe_export_label(self) -> &'static str {
        match self {
            Self::VaultDecode => "vault_decode",
            Self::VaultAudit => "vault_audit",
            Self::VaultExport => "vault_export",
            Self::InspectArtifact => "portable_artifact_inspect",
            Self::ImportArtifact => "portable_artifact_import",
        }
    }
}
```

- [ ] **Step 4: Run the targeted test to verify GREEN**

Run: `cargo test -p mdid-desktop vault_response_safe_export_omits_decoded_values_paths_and_raw_audit_detail -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Write the failing error-export test**

Add this test inside the same tests module:

```rust
    #[test]
    fn vault_response_safe_export_keeps_runtime_errors_redacted() {
        let mut state = DesktopVaultResponseState::default();
        state.apply_error(
            DesktopVaultResponseMode::InspectArtifact,
            "failed to open /secret/artifact.json with passphrase hunter2",
        );

        let exported = state.safe_export_json(DesktopVaultResponseMode::InspectArtifact);
        let exported_text = serde_json::to_string(&exported).expect("safe export serializes");

        assert_eq!(exported["mode"], "portable_artifact_inspect");
        assert_eq!(exported["banner"], "bounded portable artifact response rendered locally");
        assert_eq!(exported["summary"], "");
        assert_eq!(exported["artifact_notice"], "");
        assert_eq!(exported["error"], "runtime failed; details redacted");
        assert!(!exported_text.contains("/secret/artifact.json"));
        assert!(!exported_text.contains("hunter2"));
    }
```

- [ ] **Step 6: Run the targeted test to verify RED/GREEN status**

Run: `cargo test -p mdid-desktop vault_response_safe_export_keeps_runtime_errors_redacted -- --nocapture`

Expected after Step 3 implementation: PASS because the existing error redaction is reused. If it fails, fix only the helper/error export shape until this test passes.

- [ ] **Step 7: Run focused desktop tests**

Run: `cargo test -p mdid-desktop vault_response -- --nocapture`

Expected: PASS.

- [ ] **Step 8: Run workspace verification**

Run: `cargo test -p mdid-desktop && cargo clippy --workspace --all-targets -- -D warnings`

Expected: PASS.

- [ ] **Step 9: Commit**

```bash
git add crates/mdid-desktop/src/lib.rs
git commit -m "feat(desktop): export phi-safe vault response summaries"
```

### Task 2: Truth-sync README completion snapshot

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update README completion text**

Update the completion snapshot to mention the desktop PHI-safe vault/portable response export helper. Keep the completion percentages honest: CLI 84%, Browser/web 58%, Desktop app 50%, Overall 81%.

- [ ] **Step 2: Run docs-adjacent verification**

Run: `cargo test -p mdid-desktop vault_response -- --nocapture`

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add README.md docs/superpowers/plans/2026-04-29-desktop-vault-safe-response-export.md
git commit -m "docs: truth-sync desktop vault response export status"
```
