# Browser Vault Safe Response Download Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add PHI-safe structured browser JSON downloads for already-rendered vault/portable response panes without exposing runtime body, passphrases, vault paths, decoded values, tokens, artifact payloads, or raw debug text.

**Architecture:** Reuse the existing browser output download pipeline, but route vault audit/decode/portable inspect/import modes through a new safe report JSON builder instead of the generic `review_report_download_json` that includes `output`. Keep vault export unchanged because it intentionally downloads the encrypted portable artifact JSON.

**Tech Stack:** Rust, `mdid-browser`, Yew-compatible helper logic, `serde_json`, Cargo tests/clippy.

---

## File Structure

- Modify: `crates/mdid-browser/src/app.rs`
  - Add a browser safe vault response report builder on `AppState`.
  - Route vault/portable response modes to that builder in `prepared_download_payload`.
  - Add focused unit tests near existing browser download tests.
- Modify: `README.md`
  - Truth-sync browser/web, desktop, overall completion snapshot and verification evidence after the feature lands.

---

### Task 1: Safe browser vault/portable response report downloads

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`
- Test: `crates/mdid-browser/src/app.rs`

- [ ] **Step 1: Write the failing test**

Add this test near the existing browser download tests:

```rust
#[test]
fn browser_vault_response_download_is_structured_and_phi_safe() {
    let state = AppState {
        input_mode: InputMode::VaultDecode,
        summary: "Decoded 2 requested records; values hidden in browser response.".to_string(),
        review_queue: "Review queue: no browser-visible decoded values.".to_string(),
        result_output: serde_json::json!({
            "decoded_values": {"patient-1": {"name": "Alice Example"}},
            "vault_path": "/phi/vault",
            "passphrase": "secret",
            "token": "MDID-123",
            "audit_event": {"kind": "decode", "record_ids": ["patient-1"]}
        })
        .to_string(),
        ..AppState::default()
    };

    let payload = state.prepared_download_payload().expect("download payload");
    let report: serde_json::Value = serde_json::from_slice(&payload.bytes).expect("json report");

    assert_eq!(payload.file_name, "mdid-browser-vault-decode-response.json");
    assert_eq!(payload.mime_type, "application/json;charset=utf-8");
    assert_eq!(report["mode"], "vault_decode");
    assert_eq!(report["summary"], state.summary);
    assert_eq!(report["review_queue"], state.review_queue);
    assert!(report.get("output").is_none());
    let serialized = serde_json::to_string(&report).expect("serialized report");
    assert!(!serialized.contains("Alice Example"));
    assert!(!serialized.contains("/phi/vault"));
    assert!(!serialized.contains("secret"));
    assert!(!serialized.contains("MDID-123"));
    assert!(!serialized.contains("decoded_values"));
    assert!(!serialized.contains("audit_event"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p mdid-browser browser_vault_response_download_is_structured_and_phi_safe -- --nocapture`

Expected: FAIL because the current generic report includes `output` and leaks raw response content.

- [ ] **Step 3: Write minimal implementation**

In `impl AppState`, add:

```rust
fn safe_vault_response_download_json(&self) -> Result<Vec<u8>, String> {
    serde_json::to_vec_pretty(&serde_json::json!({
        "mode": self.input_mode.safe_vault_report_mode_label(),
        "summary": self.summary,
        "review_queue": self.review_queue,
    }))
    .map_err(|_| "Browser output download could not encode safe vault response JSON.".to_string())
}
```

Add this helper to `impl InputMode`:

```rust
fn safe_vault_report_mode_label(self) -> &'static str {
    match self {
        Self::VaultAuditEvents => "vault_audit_events",
        Self::VaultDecode => "vault_decode",
        Self::PortableArtifactInspect => "portable_artifact_inspect",
        Self::PortableArtifactImport => "portable_artifact_import",
        _ => self.label(),
    }
}
```

Change `prepared_download_payload` so `InputMode::VaultAuditEvents | InputMode::VaultDecode | InputMode::PortableArtifactInspect | InputMode::PortableArtifactImport` uses `safe_vault_response_download_json()` and `InputMode::PdfBase64` alone continues to use `review_report_download_json()`.

- [ ] **Step 4: Run targeted and package verification**

Run: `cargo test -p mdid-browser browser_vault_response_download_is_structured_and_phi_safe -- --nocapture`

Expected: PASS.

Run: `cargo test -p mdid-browser --lib`

Expected: PASS.

Run: `cargo clippy -p mdid-browser --all-targets -- -D warnings`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-browser/src/app.rs
git commit -m "fix(browser): sanitize vault response downloads"
```

---

### Task 2: README completion truth-sync

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update the completion snapshot**

Change the snapshot to state that browser vault/portable response downloads now use a PHI-safe structured report that omits raw runtime output, decoded values, audit details, vault paths, passphrases, tokens, and artifact payloads. Increase Browser/web only if justified by landed tests; otherwise state the number is unchanged.

- [ ] **Step 2: Add verification evidence**

Add the exact commands and PASS results from Task 1 to the verification evidence paragraph.

- [ ] **Step 3: Run docs verification**

Run: `git diff --check`

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add README.md docs/superpowers/plans/2026-04-30-browser-vault-safe-response-download.md
git commit -m "docs: truth-sync browser vault response downloads"
```

---

## Self-Review

- Spec coverage: Task 1 implements the safe browser report builder and routing; Task 2 updates README completion/verification evidence.
- Placeholder scan: no TBD/TODO/implement-later placeholders remain.
- Type consistency: `safe_vault_response_download_json`, `safe_vault_report_mode_label`, and `prepared_download_payload` names match across steps.
