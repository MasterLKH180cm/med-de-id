# Browser Vault Decode Mode Implementation Plan

**Goal:** Add a bounded browser vault decode request/response mode that submits explicit record ids to the existing local `/vault/decode` runtime route and renders only a PHI-risk disclosure plus decoded-value count, not decoded PHI.

**Architecture:** Extend the existing single-file `mdid-browser` local form state with a new `VaultDecode` mode next to `VaultAuditEvents`. Keep the browser surface thin: build runtime-compatible JSON, validate explicit record scope/output target/justification, submit to localhost, and render a PHI-safe summary of the response while warning that decoded values are intentionally hidden in the browser pane.

**Tech Stack:** Rust, Leptos, serde_json, existing `mdid-browser` test module, existing `mdid-runtime` `/vault/decode` contract.

**Completion evidence (2026-04-29):**
- Request-mode implementation landed in commit `99d11d1` with `InputMode::VaultDecode`, explicit-record request payload validation, and `/vault/decode` endpoint wiring.
- PHI-safe response rendering landed in commit `8314d5e`, rendering decoded counts and disclosure only while hiding decoded values, tokens, raw audit detail, output target, justification, vault path, passphrase, and raw runtime response bodies.
- Verification for the completed implementation included `cargo test -p mdid-browser vault_decode -- --nocapture`, `cargo test -p mdid-browser --lib`, `cargo clippy -p mdid-browser --all-targets -- -D warnings`, and `git diff --check`.

---

## File Structure

- Modify: `crates/mdid-browser/src/app.rs`
  - Add `InputMode::VaultDecode` metadata, endpoint, validation, request payload builder, state fields, UI controls, and PHI-safe response rendering.
  - Add focused unit tests for request payload shape, validation, mode metadata, and response rendering.
- Modify: `README.md`
  - Truth-sync Browser/web and Overall completion once the feature lands.

## Guardrails

- This is not vault browsing, not browser-side report download, not vault export/import, not auth/session, and not a workflow builder.
- The browser response pane must not render decoded `original_value`, tokens, raw audit `detail`, output target, justification, vault path, passphrase, or raw runtime response bodies.
- Do not add product-management vocabulary to product code, README, or this plan.

### Task 1: Browser vault decode request builder and mode wiring

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`
- Test: `crates/mdid-browser/src/app.rs`

- [x] **Step 1: Write failing tests**

Add these tests to the existing `#[cfg(test)] mod tests` in `crates/mdid-browser/src/app.rs`:

```rust
#[test]
fn vault_decode_mode_uses_existing_runtime_endpoint() {
    assert_eq!(InputMode::from_select_value("vault-decode"), InputMode::VaultDecode);
    assert_eq!(InputMode::VaultDecode.select_value(), "vault-decode");
    assert_eq!(InputMode::VaultDecode.label(), "Vault decode");
    assert_eq!(InputMode::VaultDecode.endpoint(), "/vault/decode");
    assert!(!InputMode::VaultDecode.requires_field_policy());
    assert!(!InputMode::VaultDecode.requires_source_name());
    assert!(InputMode::VaultDecode.disclosure_copy().unwrap().contains("explicit record ids"));
}

#[test]
fn vault_decode_payload_maps_form_to_runtime_contract() {
    let payload = build_vault_decode_request_payload(
        " /tmp/patient-vault.json ",
        " passphrase ",
        "[\"550e8400-e29b-41d4-a716-446655440000\"]",
        " local clinical review workstation ",
        " treatment continuity request ",
    )
    .expect("valid vault decode payload");

    assert_eq!(payload["vault_path"], "/tmp/patient-vault.json");
    assert_eq!(payload["vault_passphrase"], "passphrase");
    assert_eq!(payload["record_ids"], serde_json::json!(["550e8400-e29b-41d4-a716-446655440000"]));
    assert_eq!(payload["output_target"], "local clinical review workstation");
    assert_eq!(payload["justification"], "treatment continuity request");
    assert_eq!(payload["requested_by"], "browser");
}

#[test]
fn vault_decode_payload_rejects_missing_scope_and_blank_fields() {
    assert!(build_vault_decode_request_payload("", "pass", "[]", "target", "reason").unwrap_err().contains("Vault path"));
    assert!(build_vault_decode_request_payload("vault.json", "", "[]", "target", "reason").unwrap_err().contains("Vault passphrase"));
    assert!(build_vault_decode_request_payload("vault.json", "pass", "[]", "target", "reason").unwrap_err().contains("record id"));
    assert!(build_vault_decode_request_payload("vault.json", "pass", "not json", "target", "reason").unwrap_err().contains("record ids JSON"));
    assert!(build_vault_decode_request_payload("vault.json", "pass", "[\"not-a-uuid\"]", "target", "reason").unwrap_err().contains("UUID"));
    assert!(build_vault_decode_request_payload("vault.json", "pass", "[\"550e8400-e29b-41d4-a716-446655440000\"]", "", "reason").unwrap_err().contains("Output target"));
    assert!(build_vault_decode_request_payload("vault.json", "pass", "[\"550e8400-e29b-41d4-a716-446655440000\"]", "target", "").unwrap_err().contains("Justification"));
}
```

- [x] **Step 2: Run test to verify it fails**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-browser vault_decode -- --nocapture`

Expected: FAIL because `InputMode::VaultDecode` and `build_vault_decode_request_payload` do not exist.

- [x] **Step 3: Write minimal implementation**

In `crates/mdid-browser/src/app.rs`:

```rust
// Add VaultDecode to InputMode and all match arms.
// Add state fields:
vault_decode_record_ids_json: String,
vault_decode_output_target: String,
vault_decode_justification: String,

#[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
fn build_vault_decode_request_payload(
    vault_path: &str,
    vault_passphrase: &str,
    record_ids_json: &str,
    output_target: &str,
    justification: &str,
) -> Result<serde_json::Value, String> {
    let vault_path = vault_path.trim();
    if vault_path.is_empty() {
        return Err("Vault path is required before submitting.".to_string());
    }
    let vault_passphrase = vault_passphrase.trim();
    if vault_passphrase.is_empty() {
        return Err("Vault passphrase is required before submitting.".to_string());
    }
    let record_ids: Vec<String> = serde_json::from_str(record_ids_json.trim())
        .map_err(|_| "Vault decode record ids JSON must be an array of UUID strings.".to_string())?;
    if record_ids.is_empty() {
        return Err("Vault decode requires at least one record id.".to_string());
    }
    for record_id in &record_ids {
        uuid::Uuid::parse_str(record_id)
            .map_err(|_| "Vault decode record ids must be valid UUID strings.".to_string())?;
    }
    let output_target = output_target.trim();
    if output_target.is_empty() {
        return Err("Output target is required before submitting vault decode.".to_string());
    }
    let justification = justification.trim();
    if justification.is_empty() {
        return Err("Justification is required before submitting vault decode.".to_string());
    }
    Ok(serde_json::json!({
        "vault_path": vault_path,
        "vault_passphrase": vault_passphrase,
        "record_ids": record_ids,
        "output_target": output_target,
        "justification": justification,
        "requested_by": "browser"
    }))
}
```

Wire submit request creation to call the builder when `input_mode == InputMode::VaultDecode`. Add UI fields for record ids JSON, output target, and justification only in vault decode mode.

- [x] **Step 4: Run test to verify it passes**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-browser vault_decode -- --nocapture`

Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add crates/mdid-browser/src/app.rs docs/superpowers/plans/2026-04-29-browser-vault-decode-mode.md
git commit -m "feat(browser): add bounded vault decode request mode"
```

### Task 2: PHI-safe vault decode response rendering and README truth-sync

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`
- Modify: `README.md`
- Test: `crates/mdid-browser/src/app.rs`

- [x] **Step 1: Write failing tests**

Add these tests to the browser test module:

```rust
#[test]
fn parse_vault_decode_runtime_success_hides_decoded_values_and_audit_detail() {
    let response = serde_json::json!({
        "values": [
            {"record_id":"550e8400-e29b-41d4-a716-446655440000","token":"MDID-123","original_value":"Jane Patient"}
        ],
        "audit_event": {"id":"660e8400-e29b-41d4-a716-446655440000","kind":"decode","actor":"browser","detail":"decoded Jane Patient for oncology board","timestamp":"2026-04-29T00:00:00Z"}
    });

    let parsed = parse_runtime_success_response(InputMode::VaultDecode, &response).expect("valid response");

    assert_eq!(parsed.summary, "Vault decode completed for 1 decoded value(s). Decoded PHI is intentionally hidden in the browser pane.");
    assert_eq!(parsed.review_queue, "Audit event recorded: decode.");
    assert!(parsed.output.contains("Decoded values hidden"));
    assert!(!parsed.output.contains("Jane Patient"));
    assert!(!parsed.output.contains("MDID-123"));
    assert!(!parsed.output.contains("oncology"));
}
```

- [x] **Step 2: Run test to verify it fails**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-browser vault_decode -- --nocapture`

Expected: FAIL because vault decode parsing/rendering is not implemented.

- [x] **Step 3: Write minimal implementation**

Update `parse_runtime_success_response` and any helper structs in `crates/mdid-browser/src/app.rs` so `InputMode::VaultDecode` reads the `values` array length and `audit_event.kind` only. It must not serialize or display raw `values`, `token`, `original_value`, `audit_event.detail`, output target, justification, vault path, or passphrase.

- [x] **Step 4: Run verification**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-browser vault_decode -- --nocapture
cargo test -p mdid-browser --lib
cargo clippy -p mdid-browser --all-targets -- -D warnings
git diff --check
```

Expected: all PASS.

- [x] **Step 5: README truth-sync and commit**

Update `README.md` completion snapshot based on landed functionality. Expected bounded bump if tests pass: Browser/web 49%, Overall 74%. CLI remains 84%; Desktop app remains 41%. Mention browser vault decode as explicit-record, localhost `/vault/decode`, PHI-hidden in browser pane, not vault browsing/export.

```bash
git add crates/mdid-browser/src/app.rs README.md docs/superpowers/plans/2026-04-29-browser-vault-decode-mode.md
git commit -m "docs: truth-sync browser vault decode completion"
```
