# Cross-Surface Vault Response Metadata Reports Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add PHI-safe vault response metadata to Browser/Web and Desktop safe response reports so decode/audit/import/export workflows expose useful counts and IDs without exporting decoded values, vault paths, passphrases, or raw request bodies.

**Architecture:** Browser/Web and Desktop already have separate high-risk exports for decoded values and audit events; this slice improves the safe response report channel only. Each surface derives an allowlisted `metadata` object from the successful runtime response and keeps sensitive fields out of the safe report JSON.

**Tech Stack:** Rust workspace, Leptos browser crate, egui desktop crate, serde_json, cargo tests, strict TDD and SDD review.

---

## File Structure

- Modify: `crates/mdid-browser/src/app.rs` — add `safe_vault_response_metadata`, include `metadata` in safe vault response report JSON, and add focused browser tests.
- Modify: `crates/mdid-desktop/src/lib.rs` — add `safe_metadata`, include `metadata` in desktop safe response report JSON, and add focused desktop library tests.
- Modify: `README.md` — truth-sync completion snapshot after landed verification, including CLI/browser/desktop/overall percentages and remaining gaps.

### Task 1: Browser safe vault response report metadata

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`

- [ ] **Step 1: Write the failing tests**

Add these tests inside the existing `#[cfg(test)] mod tests` in `crates/mdid-browser/src/app.rs`:

```rust
#[test]
fn browser_safe_vault_response_report_includes_allowlisted_decode_metadata() {
    let state = BrowserFlowState {
        input_mode: InputMode::VaultDecode,
        summary: "Decoded 2 values.".to_string(),
        review_queue: "Decoded values hidden from safe report.".to_string(),
        decoded_values_output: Some(
            serde_json::json!({
                "decoded_count": 2,
                "decoded_value_count": 2,
                "audit_event_id": "audit-123",
                "decoded_values": {"record-1": {"name": "Jane Doe"}},
                "vault_path": "/phi/vault",
                "passphrase": "secret"
            })
            .to_string(),
        ),
        ..BrowserFlowState::default()
    };

    let report: serde_json::Value =
        serde_json::from_slice(&state.safe_vault_response_download_json().expect("report json"))
            .expect("parse safe report");
    let text = serde_json::to_string(&report).expect("report text");

    assert_eq!(report["mode"], "vault_decode_safe_response");
    assert_eq!(report["metadata"]["decoded_count"], 2);
    assert_eq!(report["metadata"]["decoded_value_count"], 2);
    assert_eq!(report["metadata"]["audit_event_id"], "audit-123");
    assert!(report.get("decoded_values").is_none());
    assert!(report["metadata"].get("decoded_values").is_none());
    assert!(!text.contains("Jane Doe"));
    assert!(!text.contains("/phi/vault"));
    assert!(!text.contains("secret"));
}

#[test]
fn browser_safe_vault_response_report_includes_allowlisted_audit_metadata() {
    let state = BrowserFlowState {
        input_mode: InputMode::VaultAuditEvents,
        summary: "Returned 2 audit events.".to_string(),
        review_queue: "Audit event details are exported separately.".to_string(),
        result_output: serde_json::json!({
            "returned_event_count": 2,
            "total_event_count": 5,
            "offset": 1,
            "limit": 2,
            "events": [{"event_id": "event-1", "path": "/phi/vault"}],
            "vault_path": "/phi/vault",
            "passphrase": "secret"
        })
        .to_string(),
        ..BrowserFlowState::default()
    };

    let report: serde_json::Value =
        serde_json::from_slice(&state.safe_vault_response_download_json().expect("report json"))
            .expect("parse safe report");
    let text = serde_json::to_string(&report).expect("report text");

    assert_eq!(report["mode"], "vault_audit_events_safe_response");
    assert_eq!(report["metadata"]["returned_event_count"], 2);
    assert_eq!(report["metadata"]["total_event_count"], 5);
    assert_eq!(report["metadata"]["offset"], 1);
    assert_eq!(report["metadata"]["limit"], 2);
    assert!(report["metadata"].get("events").is_none());
    assert!(!text.contains("/phi/vault"));
    assert!(!text.contains("secret"));
}
```

- [ ] **Step 2: Run tests to verify RED**

Run: `cargo test -p mdid-browser safe_vault_response_report_includes_allowlisted -- --nocapture`

Expected: FAIL because the safe report does not yet include a `metadata` object.

- [ ] **Step 3: Implement minimal browser metadata helper**

Add this helper inside `impl BrowserFlowState` and update `safe_vault_response_download_json` to include `"metadata": self.safe_vault_response_metadata()`:

```rust
fn safe_vault_response_metadata(&self) -> serde_json::Value {
    let response_text = match self.input_mode {
        InputMode::VaultDecode => self.decoded_values_output.as_deref(),
        InputMode::VaultAuditEvents => Some(self.result_output.as_str()),
        InputMode::VaultExport | InputMode::PortableInspect | InputMode::PortableImport => {
            Some(self.result_output.as_str())
        }
        _ => None,
    };

    let Some(response_text) = response_text else {
        return serde_json::json!({});
    };
    let Ok(response) = serde_json::from_str::<serde_json::Value>(response_text) else {
        return serde_json::json!({});
    };

    let keys = [
        "artifact_record_count",
        "decoded_count",
        "decoded_value_count",
        "audit_event_id",
        "returned_event_count",
        "total_event_count",
        "offset",
        "limit",
        "imported_record_count",
        "skipped_record_count",
    ];
    let mut metadata = serde_json::Map::new();
    for key in keys {
        if let Some(value) = response.get(key) {
            if value.is_number() || value.is_string() || value.is_boolean() || value.is_null() {
                metadata.insert(key.to_string(), value.clone());
            }
        }
    }
    serde_json::Value::Object(metadata)
}
```

- [ ] **Step 4: Run browser targeted tests to verify GREEN**

Run: `cargo test -p mdid-browser safe_vault_response_report_includes_allowlisted -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Run broader browser tests**

Run: `cargo test -p mdid-browser --lib`

Expected: PASS.

- [ ] **Step 6: Commit browser task**

Run:

```bash
git add crates/mdid-browser/src/app.rs docs/superpowers/plans/2026-04-30-cross-surface-vault-response-metadata-reports.md
git commit -m "feat(browser): add safe vault response metadata"
```

### Task 2: Desktop safe vault response report metadata

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Add these tests inside the existing desktop library test module in `crates/mdid-desktop/src/lib.rs`:

```rust
#[test]
fn desktop_safe_response_report_includes_allowlisted_decode_metadata() {
    let mut state = DesktopVaultResponseState::default();
    state.apply_success(
        DesktopVaultResponseMode::VaultDecode,
        &serde_json::json!({
            "decoded_count": 2,
            "decoded_value_count": 2,
            "audit_event_id": "audit-123",
            "decoded_values": {"record-1": {"name": "Jane Doe"}},
            "vault_path": "/phi/vault",
            "passphrase": "secret"
        }),
    );

    let report = state.safe_response_report_json().expect("safe report");
    let text = serde_json::to_string(&report).expect("report text");

    assert_eq!(report["mode"], "vault_decode_safe_response");
    assert_eq!(report["metadata"]["decoded_count"], 2);
    assert_eq!(report["metadata"]["decoded_value_count"], 2);
    assert_eq!(report["metadata"]["audit_event_id"], "audit-123");
    assert!(report.get("decoded_values").is_none());
    assert!(report["metadata"].get("decoded_values").is_none());
    assert!(!text.contains("Jane Doe"));
    assert!(!text.contains("/phi/vault"));
    assert!(!text.contains("secret"));
}

#[test]
fn desktop_safe_response_report_includes_allowlisted_audit_metadata() {
    let mut state = DesktopVaultResponseState::default();
    state.apply_success(
        DesktopVaultResponseMode::VaultAuditEvents,
        &serde_json::json!({
            "returned_event_count": 2,
            "total_event_count": 5,
            "offset": 1,
            "limit": 2,
            "events": [{"event_id": "event-1", "path": "/phi/vault"}],
            "vault_path": "/phi/vault",
            "passphrase": "secret"
        }),
    );

    let report = state.safe_response_report_json().expect("safe report");
    let text = serde_json::to_string(&report).expect("report text");

    assert_eq!(report["mode"], "vault_audit_events_safe_response");
    assert_eq!(report["metadata"]["returned_event_count"], 2);
    assert_eq!(report["metadata"]["total_event_count"], 5);
    assert_eq!(report["metadata"]["offset"], 1);
    assert_eq!(report["metadata"]["limit"], 2);
    assert!(report["metadata"].get("events").is_none());
    assert!(!text.contains("/phi/vault"));
    assert!(!text.contains("secret"));
}
```

- [ ] **Step 2: Run tests to verify RED**

Run: `cargo test -p mdid-desktop safe_response_report_includes_allowlisted -- --nocapture`

Expected: FAIL because the safe report does not yet include a `metadata` object.

- [ ] **Step 3: Implement minimal desktop metadata helper**

Add this method in `impl DesktopVaultResponseState` and update `safe_export_json` to include `"metadata": self.safe_metadata()`:

```rust
fn safe_metadata(&self) -> serde_json::Value {
    let Some(response) = self.last_success_response.as_ref() else {
        return serde_json::json!({});
    };

    let keys = [
        "artifact_record_count",
        "decoded_count",
        "decoded_value_count",
        "audit_event_id",
        "returned_event_count",
        "total_event_count",
        "offset",
        "limit",
        "imported_record_count",
        "skipped_record_count",
    ];
    let mut metadata = serde_json::Map::new();
    for key in keys {
        if let Some(value) = response.get(key) {
            if value.is_number() || value.is_string() || value.is_boolean() || value.is_null() {
                metadata.insert(key.to_string(), value.clone());
            }
        }
    }
    serde_json::Value::Object(metadata)
}
```

- [ ] **Step 4: Run desktop targeted tests to verify GREEN**

Run: `cargo test -p mdid-desktop safe_response_report_includes_allowlisted -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Run broader desktop tests**

Run: `cargo test -p mdid-desktop --lib`

Expected: PASS.

- [ ] **Step 6: Commit desktop task**

Run:

```bash
git add crates/mdid-desktop/src/lib.rs
git commit -m "feat(desktop): add safe vault response metadata"
```

### Task 3: README truth-sync and verification

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Run final verification**

Run:

```bash
cargo test -p mdid-browser safe_vault_response_report_includes_allowlisted -- --nocapture
cargo test -p mdid-browser --lib
cargo test -p mdid-desktop safe_response_report_includes_allowlisted -- --nocapture
cargo test -p mdid-desktop --lib
cargo fmt --check
git diff --check
```

Expected: all PASS with no whitespace errors.

- [ ] **Step 2: Update README completion snapshot**

Update the README status table and evidence paragraph to state that cross-surface safe vault response reports now include allowlisted metadata for decode/audit/portable response flows while excluding decoded values, audit event arrays, vault paths, passphrases, and raw requests. Increase Browser/Web by 5 percentage points and Desktop app by 5 percentage points only if the controller-visible implementation and tests above pass.

- [ ] **Step 3: Commit README truth-sync**

Run:

```bash
git add README.md
git commit -m "docs: truth-sync safe response metadata reports"
```
