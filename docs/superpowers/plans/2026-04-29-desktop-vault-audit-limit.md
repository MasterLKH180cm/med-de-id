# Desktop Vault Audit Limit Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add desktop vault-audit limit parity so the workstation surface can submit bounded `/vault/audit/events` requests with the same optional positive limit already supported by runtime, CLI, and browser.

**Architecture:** Keep the change inside the desktop request-state builder and UI only. The domain/runtime contract already accepts `limit`; desktop will parse an optional text field into a positive integer, omit blank values, and fail closed on invalid values before localhost submission.

**Tech Stack:** Rust workspace, `mdid-desktop`, `egui`, `serde_json`, Cargo tests/clippy.

---

## File Structure

- Modify: `crates/mdid-desktop/src/lib.rs`
  - Add `audit_limit: Option<String>` to `DesktopVaultRequestState`.
  - Include it in safe debug output.
  - Default it to `None`.
  - Parse optional limit for `DesktopVaultMode::AuditEvents` and include JSON `"limit": <usize>` only when a positive value is provided.
  - Add typed validation errors for invalid or zero audit limits.
  - Add TDD tests for included, omitted, and invalid limits.
- Modify: `crates/mdid-desktop/src/main.rs`
  - Add an “Audit limit (optional)” single-line input in the audit-events UI block.
- Modify: `README.md`
  - Truth-sync the completion snapshot and desktop/browser/overall wording after landed verification.

### Task 1: Desktop audit limit request builder and UI

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs:511-626`
- Modify: `crates/mdid-desktop/src/lib.rs:2121-2252`
- Modify: `crates/mdid-desktop/src/main.rs:328-334`
- Test: `crates/mdid-desktop/src/lib.rs` inline test module

- [x] **Step 1: Write failing tests for optional audit limit behavior**

Add these tests near `desktop_vault_audit_request_builds_read_only_filter_contract`:

```rust
#[test]
fn desktop_vault_audit_request_includes_optional_positive_limit() {
    let state = DesktopVaultRequestState {
        mode: DesktopVaultMode::AuditEvents,
        vault_path: "C:/vaults/local.mdid".to_string(),
        vault_passphrase: "correct horse battery staple".to_string(),
        record_ids_json: "[]".to_string(),
        output_target: "review-workbench".to_string(),
        justification: "desktop audit review".to_string(),
        requested_by: "desktop".to_string(),
        audit_kind: Some("Decode".to_string()),
        audit_actor: Some("Desktop".to_string()),
        audit_limit: Some("25".to_string()),
    };

    let request = state
        .try_build_request()
        .expect("audit request with positive limit should build");

    assert_eq!(request.route, "/vault/audit/events");
    assert_eq!(request.body["kind"], "decode");
    assert_eq!(request.body["actor"], "desktop");
    assert_eq!(request.body["limit"], 25);
}

#[test]
fn desktop_vault_audit_request_omits_blank_limit() {
    let state = DesktopVaultRequestState {
        mode: DesktopVaultMode::AuditEvents,
        vault_path: "C:/vaults/local.mdid".to_string(),
        vault_passphrase: "correct horse battery staple".to_string(),
        record_ids_json: "[]".to_string(),
        output_target: "review-workbench".to_string(),
        justification: "desktop audit review".to_string(),
        requested_by: "desktop".to_string(),
        audit_kind: None,
        audit_actor: None,
        audit_limit: Some("   ".to_string()),
    };

    let request = state
        .try_build_request()
        .expect("audit request with blank limit should build");

    assert_eq!(request.route, "/vault/audit/events");
    assert!(request.body.get("limit").is_none());
}

#[test]
fn desktop_vault_audit_request_rejects_invalid_limit() {
    let state = DesktopVaultRequestState {
        mode: DesktopVaultMode::AuditEvents,
        vault_path: "C:/vaults/local.mdid".to_string(),
        vault_passphrase: "correct horse battery staple".to_string(),
        record_ids_json: "[]".to_string(),
        output_target: "review-workbench".to_string(),
        justification: "desktop audit review".to_string(),
        requested_by: "desktop".to_string(),
        audit_kind: None,
        audit_actor: None,
        audit_limit: Some("not-a-number".to_string()),
    };

    assert_eq!(
        state.try_build_request(),
        Err(DesktopVaultValidationError::InvalidAuditLimit("not-a-number".to_string()))
    );
}

#[test]
fn desktop_vault_audit_request_rejects_zero_limit() {
    let state = DesktopVaultRequestState {
        mode: DesktopVaultMode::AuditEvents,
        vault_path: "C:/vaults/local.mdid".to_string(),
        vault_passphrase: "correct horse battery staple".to_string(),
        record_ids_json: "[]".to_string(),
        output_target: "review-workbench".to_string(),
        justification: "desktop audit review".to_string(),
        requested_by: "desktop".to_string(),
        audit_kind: None,
        audit_actor: None,
        audit_limit: Some("0".to_string()),
    };

    assert_eq!(state.try_build_request(), Err(DesktopVaultValidationError::ZeroAuditLimit));
}
```

- [x] **Step 2: Run test to verify RED**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-desktop desktop_vault_audit_request_includes_optional_positive_limit -- --nocapture
```

Expected: FAIL to compile because `DesktopVaultRequestState` has no `audit_limit` field and `DesktopVaultValidationError` has no `InvalidAuditLimit` / `ZeroAuditLimit` variants.

- [x] **Step 3: Implement minimal request-state support**

Update `DesktopVaultRequestState` with:

```rust
pub audit_limit: Option<String>,
```

Update debug/default/all tests constructing the struct by adding `audit_limit: None` unless the test sets it. Add validation variants:

```rust
InvalidAuditLimit(String),
ZeroAuditLimit,
```

Add helper:

```rust
fn parse_optional_positive_usize(value: Option<&str>) -> Result<Option<usize>, DesktopVaultValidationError> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    let parsed = value
        .parse::<usize>()
        .map_err(|_| DesktopVaultValidationError::InvalidAuditLimit(value.to_string()))?;
    if parsed == 0 {
        return Err(DesktopVaultValidationError::ZeroAuditLimit);
    }
    Ok(Some(parsed))
}
```

Build audit JSON by starting with an object and only inserting `limit` when present:

```rust
DesktopVaultMode::AuditEvents => {
    let mut body = serde_json::json!({
        "vault_path": vault_path,
        "vault_passphrase": self.vault_passphrase.clone(),
        "kind": lowercase_optional_filter(self.audit_kind.as_deref()),
        "actor": lowercase_optional_filter(self.audit_actor.as_deref()),
    });
    if let Some(limit) = parse_optional_positive_usize(self.audit_limit.as_deref())? {
        body["limit"] = serde_json::json!(limit);
    }
    body
}
```

- [x] **Step 4: Add desktop UI input**

In the `DesktopVaultMode::AuditEvents` UI block, add:

```rust
let limit = self.vault_request_state.audit_limit.get_or_insert_with(String::new);
ui.label("Audit limit (optional)");
ui.text_edit_singleline(limit);
```

- [x] **Step 5: Verify GREEN and broader desktop checks**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-desktop desktop_vault_audit_request_includes_optional_positive_limit -- --nocapture
cargo test -p mdid-desktop desktop_vault_audit_request_omits_blank_limit -- --nocapture
cargo test -p mdid-desktop desktop_vault_audit_request_rejects_invalid_limit -- --nocapture
cargo test -p mdid-desktop desktop_vault_audit_request_rejects_zero_limit -- --nocapture
cargo test -p mdid-desktop
cargo clippy -p mdid-desktop --all-targets -- -D warnings
```

Expected: all PASS.

- [x] **Step 6: Commit**

```bash
git add crates/mdid-desktop/src/lib.rs crates/mdid-desktop/src/main.rs
git commit -m "feat(desktop): add vault audit limit input"
```

### Task 2: README truth-sync and completion accounting

**Files:**
- Modify: `README.md:64-73`
- Test: no code test; verification commands from Task 1 are the evidence.

- [x] **Step 1: Update completion snapshot text**

Update the README completion snapshot date/note to mention desktop vault audit optional limit parity. Desktop may increase only if justified by landed behavior and passing tests; do not inflate CLI/browser/overall without evidence.

- [x] **Step 2: Verify docs plus code checks**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-desktop
cargo clippy -p mdid-desktop --all-targets -- -D warnings
git diff -- README.md
```

Expected: tests/clippy PASS; README diff only truth-syncs completion and missing-item wording.

- [x] **Step 3: Commit**

```bash
git add README.md docs/superpowers/plans/2026-04-29-desktop-vault-audit-limit.md
git commit -m "docs: truth-sync desktop audit limit progress"
```

## Self-Review

- Spec coverage: This plan covers only the desktop audit limit parity gap. It does not add broader vault browsing, decoded PHI display, orchestration, controller semantics, packaging, auth/session, or generalized workflow behavior.
- Placeholder scan: No TBD/TODO/implement-later placeholders remain.
- Type consistency: The plan consistently names `audit_limit`, `InvalidAuditLimit(String)`, and `ZeroAuditLimit`.
