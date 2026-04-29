# Browser Vault Audit Mode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded browser-side vault audit request mode that prepares and submits PHI-safe audit event browsing requests to the existing localhost runtime.

**Architecture:** Reuse the existing `mdid-browser` single-page local-first workflow and add one narrowly scoped `InputMode` for `/vault/audit/events`. The browser surface will collect a local vault path, passphrase, optional event kind/actor filters, and a limit, then render only the runtime response JSON with disclosures that this is read-only audit browsing and not vault decode/export, governance orchestration, or controller workflow behavior.

**Tech Stack:** Rust, Leptos, serde/serde_json, existing `mdid-runtime` HTTP contract, Cargo test harness.

---

## File Structure

- Modify: `crates/mdid-browser/src/app.rs`
  - Add `InputMode::VaultAuditEvents` metadata: select value, label, hint, disclosure, endpoint, source-name/field-policy requirements.
  - Add pure helpers for building vault audit request JSON from browser form state.
  - Add tests proving the endpoint, disclosure, and payload mapping are bounded and PHI-safe.
- Modify: `README.md`
  - Truth-sync browser/web and overall completion after landed implementation and verification.

### Task 1: Browser vault audit event mode

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`
- Test: existing unit tests in `crates/mdid-browser/src/app.rs`

- [ ] **Step 1: Write the failing tests**

Add tests in `crates/mdid-browser/src/app.rs` under the existing `#[cfg(test)] mod tests` block:

```rust
#[test]
fn vault_audit_events_mode_uses_existing_read_only_runtime_endpoint() {
    let mode = InputMode::from_select_value("vault-audit-events");

    assert_eq!(mode, InputMode::VaultAuditEvents);
    assert_eq!(mode.select_value(), "vault-audit-events");
    assert_eq!(mode.endpoint(), "/vault/audit/events");
    assert!(!mode.requires_field_policy());
    assert!(!mode.requires_source_name());
    assert_eq!(mode.browser_file_read_mode(), BrowserFileReadMode::Text);
    assert!(mode
        .disclosure_copy()
        .expect("vault audit mode has bounded disclosure")
        .contains("read-only"));
}

#[test]
fn vault_audit_payload_maps_text_form_to_bounded_runtime_contract() {
    let payload = build_vault_audit_request_payload(
        "/tmp/local-vault",
        "passphrase kept local",
        "decode",
        "browser",
        "25",
    )
    .expect("valid bounded audit payload");

    assert_eq!(payload["vault_path"], "/tmp/local-vault");
    assert_eq!(payload["vault_passphrase"], "passphrase kept local");
    assert_eq!(payload["kind"], "decode");
    assert_eq!(payload["actor"], "browser");
    assert_eq!(payload["limit"], 25);
}

#[test]
fn vault_audit_payload_omits_blank_optional_filters() {
    let payload = build_vault_audit_request_payload(
        "/tmp/local-vault",
        "passphrase kept local",
        " ",
        "",
        "",
    )
    .expect("blank optional filters are valid");

    assert_eq!(payload["vault_path"], "/tmp/local-vault");
    assert_eq!(payload["vault_passphrase"], "passphrase kept local");
    assert!(payload.get("kind").is_none());
    assert!(payload.get("actor").is_none());
    assert!(payload.get("limit").is_none());
}

#[test]
fn vault_audit_payload_rejects_invalid_limit() {
    let error = build_vault_audit_request_payload(
        "/tmp/local-vault",
        "passphrase kept local",
        "decode",
        "browser",
        "not-a-number",
    )
    .expect_err("invalid limit must be rejected before localhost submission");

    assert!(error.contains("limit"));
}
```

- [ ] **Step 2: Run tests to verify RED**

Run: `cargo test -p mdid-browser vault_audit -- --nocapture`

Expected: FAIL because `InputMode::VaultAuditEvents` and `build_vault_audit_request_payload` do not exist yet.

- [ ] **Step 3: Implement minimal browser mode and payload helper**

In `crates/mdid-browser/src/app.rs`:

1. Add `VaultAuditEvents` to `InputMode`.
2. Map select value `vault-audit-events`, label `Vault audit events`, payload hint `Vault audit request fields are rendered by the browser form`, endpoint `/vault/audit/events`, no field policy, no source name, text read mode.
3. Add a helper:

```rust
#[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
fn build_vault_audit_request_payload(
    vault_path: &str,
    vault_passphrase: &str,
    kind: &str,
    actor: &str,
    limit: &str,
) -> Result<serde_json::Value, String> {
    let trimmed_path = vault_path.trim();
    if trimmed_path.is_empty() {
        return Err("Vault audit requires a local vault path.".to_string());
    }

    let trimmed_passphrase = vault_passphrase.trim();
    if trimmed_passphrase.is_empty() {
        return Err("Vault audit requires a local vault passphrase.".to_string());
    }

    let mut payload = serde_json::json!({
        "vault_path": trimmed_path,
        "vault_passphrase": trimmed_passphrase,
    });

    if !kind.trim().is_empty() {
        payload["kind"] = serde_json::Value::String(kind.trim().to_string());
    }

    if !actor.trim().is_empty() {
        payload["actor"] = serde_json::Value::String(actor.trim().to_string());
    }

    if !limit.trim().is_empty() {
        let parsed_limit = limit
            .trim()
            .parse::<usize>()
            .map_err(|_| "Vault audit limit must be a positive integer.".to_string())?;
        payload["limit"] = serde_json::json!(parsed_limit);
    }

    Ok(payload)
}
```

4. Wire the existing browser form state minimally so selecting Vault audit events shows read-only disclosure and can submit the helper-built JSON body to `/vault/audit/events`. Do not add decode/export, agent/controller, claim, or workflow semantics.

- [ ] **Step 4: Run targeted tests to verify GREEN**

Run: `cargo test -p mdid-browser vault_audit -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Run broader browser tests**

Run: `cargo test -p mdid-browser -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/mdid-browser/src/app.rs
git commit -m "feat(browser): add bounded vault audit request mode"
```

### Task 2: README truth-sync for browser vault audit mode

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update completion snapshot text**

Change the README completion snapshot to truthfully describe the landed bounded browser vault audit mode. Raise browser/web and overall only if controller-visible tests pass. Keep CLI and desktop unchanged unless this task changes them.

Use this factual completion basis:
- CLI: unchanged at 84%.
- Browser/web: 44% after adding bounded read-only vault audit event browsing request/submission to the existing localhost runtime route.
- Desktop app: unchanged at 41%.
- Overall: 72% after adding one real browser workflow surface for existing vault audit runtime capability.

- [ ] **Step 2: Run README grep verification**

Run: `grep -n "Completion snapshot\|Browser/web\|Overall\|vault audit" README.md`

Expected: README mentions browser bounded vault audit event browsing and overall 72%.

- [ ] **Step 3: Commit**

```bash
git add README.md docs/superpowers/plans/2026-04-29-browser-vault-audit-mode.md
git commit -m "docs: truth-sync browser vault audit completion"
```
