# Desktop Vault Audit Pagination Controls Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Add bounded desktop vault audit offset/page-window support so the workstation can request and truthfully display paginated audit-event windows from the existing localhost runtime route.

**Architecture:** Extend the existing `DesktopVaultRequestState` audit-events request builder with an optional non-negative `audit_offset` field, preserving the current limit/kind/actor validation and omitting zero/blank offsets. Extend the vault response rendering helper with a PHI-safe pagination status derived from the runtime response fields (`returned_event_count`, `event_count`, `offset`, `limit`) and surface that status in the desktop vault response summary without exposing raw event details.

**Tech Stack:** Rust workspace; `mdid-desktop` crate; serde_json; existing cargo test/fmt tooling.

---

## File Structure

- Modify: `crates/mdid-desktop/src/lib.rs`
  - Add `audit_offset: Option<String>` to `DesktopVaultRequestState`.
  - Add `InvalidAuditOffset(String)` and any required validation variant.
  - Reuse/extract bounded unsigned parsing for optional audit offset.
  - Include `offset` in `/vault/audit/events` request JSON only when the trimmed parsed value is greater than zero.
  - Add a PHI-safe audit pagination status helper that uses only counts/window fields and can be rendered from `DesktopVaultResponseState::render_success`.
  - Add unit tests for offset request building, invalid negative offset rejection, omitted zero/blank offset, and status rendering.
- Modify: `crates/mdid-desktop/src/main.rs`
  - Add an “Offset” text input beside the existing audit limit input for the desktop audit-events form.
- Modify: `README.md`
  - Truth-sync desktop/browser/CLI/overall completion after landed verification. Completion numbers should only increase if the landed feature materially changes the status; this bounded desktop vault audit pagination control likely keeps overall at 93% and may increase desktop only if evidence supports it.

---

### Task 1: Desktop audit request offset support

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`
- Test: `crates/mdid-desktop/src/lib.rs`

- [x] **Step 1: Write the failing test**

Add these tests to the existing `#[cfg(test)]` module in `crates/mdid-desktop/src/lib.rs` near the other vault request tests:

```rust
#[test]
fn vault_audit_request_includes_positive_offset() {
    let state = DesktopVaultRequestState {
        mode: DesktopVaultMode::AuditEvents,
        vault_path: "C:/vaults/site.mdid".to_string(),
        vault_passphrase: "correct horse battery staple".to_string(),
        audit_limit: Some("25".to_string()),
        audit_offset: Some("50".to_string()),
        ..DesktopVaultRequestState::default()
    };

    let request = state.try_build_request().expect("audit request should build");

    assert_eq!(request.route, "/vault/audit/events");
    assert_eq!(request.body["limit"], serde_json::json!(25));
    assert_eq!(request.body["offset"], serde_json::json!(50));
}

#[test]
fn vault_audit_request_omits_blank_and_zero_offset() {
    let blank_state = DesktopVaultRequestState {
        mode: DesktopVaultMode::AuditEvents,
        vault_path: "C:/vaults/site.mdid".to_string(),
        vault_passphrase: "correct horse battery staple".to_string(),
        audit_offset: Some("   ".to_string()),
        ..DesktopVaultRequestState::default()
    };
    let blank_request = blank_state.try_build_request().expect("blank offset is omitted");
    assert!(blank_request.body.get("offset").is_none());

    let zero_state = DesktopVaultRequestState {
        mode: DesktopVaultMode::AuditEvents,
        vault_path: "C:/vaults/site.mdid".to_string(),
        vault_passphrase: "correct horse battery staple".to_string(),
        audit_offset: Some("0".to_string()),
        ..DesktopVaultRequestState::default()
    };
    let zero_request = zero_state.try_build_request().expect("zero offset is omitted");
    assert!(zero_request.body.get("offset").is_none());
}

#[test]
fn vault_audit_request_rejects_negative_offset_without_echoing_input() {
    let state = DesktopVaultRequestState {
        mode: DesktopVaultMode::AuditEvents,
        vault_path: "C:/vaults/site.mdid".to_string(),
        vault_passphrase: "correct horse battery staple".to_string(),
        audit_offset: Some("-10".to_string()),
        ..DesktopVaultRequestState::default()
    };

    let error = state.try_build_request().expect_err("negative offset must fail");

    assert!(matches!(error, DesktopVaultValidationError::InvalidAuditOffset(_)));
    assert!(!format!("{error:?}").contains("-10"));
}
```

- [x] **Step 2: Run test to verify it fails**

Run: `cargo test -p mdid-desktop vault_audit_request_ -- --nocapture`

Expected: FAIL to compile because `audit_offset` and `InvalidAuditOffset` do not exist yet.

- [x] **Step 3: Write minimal implementation**

Update `DesktopVaultRequestState` and audit-events request building in `crates/mdid-desktop/src/lib.rs`:

```rust
pub audit_offset: Option<String>,
```

Include the field in `Debug` and `Default`:

```rust
.field("audit_offset", &self.audit_offset)
```

```rust
audit_offset: None,
```

Add the validation variant:

```rust
InvalidAuditOffset(String),
```

In the `DesktopVaultMode::AuditEvents` branch, parse and include positive offsets only:

```rust
let offset = parse_optional_non_negative_usize(
    self.audit_offset.as_deref(),
    DesktopVaultValidationError::InvalidAuditOffset,
)?;
```

After the existing optional limit insertion:

```rust
if let Some(offset) = offset.filter(|offset| *offset > 0) {
    body["offset"] = serde_json::json!(offset);
}
```

Add this helper next to the existing parse helper:

```rust
fn parse_optional_non_negative_usize<E>(
    value: Option<&str>,
    invalid: fn(String) -> E,
) -> Result<Option<usize>, E> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if value.starts_with('-') {
        return Err(invalid("negative values are not allowed".to_string()));
    }
    value
        .parse::<usize>()
        .map(Some)
        .map_err(|_| invalid("expected non-negative integer".to_string()))
}
```

- [x] **Step 4: Run test to verify it passes**

Run: `cargo test -p mdid-desktop vault_audit_request_ -- --nocapture`

Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add crates/mdid-desktop/src/lib.rs
git commit -m "feat(desktop): add vault audit offset requests"
```

---

### Task 2: Desktop audit pagination status rendering and UI input

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`
- Modify: `crates/mdid-desktop/src/main.rs`
- Test: `crates/mdid-desktop/src/lib.rs`

- [x] **Step 1: Write the failing test**

Add this test near the desktop vault response tests in `crates/mdid-desktop/src/lib.rs`:

```rust
#[test]
fn vault_audit_response_summary_includes_pagination_status() {
    let mut state = DesktopVaultResponseState::default();
    let response = serde_json::json!({
        "events": [
            {"event_id": "evt-1", "kind": "decode", "actor": "desktop"},
            {"event_id": "evt-2", "kind": "export", "actor": "desktop"}
        ],
        "event_count": 7,
        "returned_event_count": 2,
        "offset": 5,
        "limit": 2
    });

    state.render_success(DesktopVaultResponseMode::VaultAudit, &response);

    assert!(state.summary.contains("Audit events page: showing 6-7 of 7"));
    assert!(state.summary.contains("limit 2"));
    assert!(!state.summary.contains("evt-1"));
    assert!(!state.summary.contains("decode"));
}
```

- [x] **Step 2: Run test to verify it fails**

Run: `cargo test -p mdid-desktop vault_audit_response_summary_includes_pagination_status -- --nocapture`

Expected: FAIL because the summary does not yet include the pagination status.

- [x] **Step 3: Write minimal implementation**

In `DesktopVaultResponseState::render_success`, when rendering `DesktopVaultResponseMode::VaultAudit`, append a status string from a helper such as:

```rust
fn vault_audit_pagination_status(response: &serde_json::Value) -> Option<String> {
    let total = response.get("event_count").and_then(serde_json::Value::as_u64)?;
    let returned = response
        .get("returned_event_count")
        .and_then(serde_json::Value::as_u64)
        .or_else(|| response.get("events").and_then(serde_json::Value::as_array).map(|events| events.len() as u64))
        .unwrap_or(0);
    let offset = response
        .get("offset")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let limit = response.get("limit").and_then(serde_json::Value::as_u64);

    if returned == 0 {
        let mut status = format!("Audit events page: showing 0 of {total} from offset {offset}");
        if let Some(limit) = limit {
            status.push_str(&format!(" (limit {limit})"));
        }
        return Some(status);
    }

    let start = offset.saturating_add(1).min(total);
    let end = offset.saturating_add(returned).min(total);
    let mut status = format!("Audit events page: showing {start}-{end} of {total}");
    if let Some(limit) = limit {
        status.push_str(&format!(" (limit {limit})"));
    }
    Some(status)
}
```

Append this helper result to the PHI-safe summary string for vault audit responses only. Do not include raw `events`, event IDs, actor values, paths, passphrases, decoded values, tokens, or audit detail.

In `crates/mdid-desktop/src/main.rs`, add an Offset input in the audit-events form:

```rust
let offset = self.vault_request_state.audit_offset.get_or_insert_with(String::new);
ui.horizontal(|ui| {
    ui.label("Offset");
    ui.text_edit_singleline(offset);
});
```

Place it near the existing Limit input.

- [x] **Step 4: Run tests to verify they pass**

Run:

```bash
cargo test -p mdid-desktop vault_audit_request_ -- --nocapture
cargo test -p mdid-desktop vault_audit_response_summary_includes_pagination_status -- --nocapture
cargo test -p mdid-desktop --lib
cargo fmt --check
git diff --check
```

Expected: all pass.

- [x] **Step 5: Commit**

```bash
git add crates/mdid-desktop/src/lib.rs crates/mdid-desktop/src/main.rs
git commit -m "feat(desktop): show vault audit pagination status"
```

---

### Task 3: README truth-sync and final verification

**Files:**
- Modify: `README.md`

- [x] **Step 1: Re-run verification**

Run:

```bash
cargo test -p mdid-desktop vault_audit -- --nocapture
cargo test -p mdid-desktop --lib
cargo fmt --check
git diff --check
```

Expected: all pass.

- [x] **Step 2: Update README completion snapshot**

Update `README.md` current repository status to mention desktop vault audit pagination controls and status rendering. Keep completion percentages truthful; use CLI 95%, Browser/Web 78%, Desktop app 71% only if the landed desktop audit pagination controls materially improve the desktop workstation vault/audit workflow; keep Overall 93% unless a major blocker is removed.

Add verification evidence including the exact commands from Step 1 and the landed commit hashes.

- [x] **Step 3: Commit**

```bash
git add README.md docs/superpowers/plans/2026-04-30-desktop-vault-audit-pagination-controls.md
git commit -m "docs: truth-sync desktop audit pagination controls"
```

- [x] **Step 4: Merge to develop**

```bash
git checkout develop
git merge --no-ff feat/desktop-vault-audit-pagination-controls -m "merge: desktop audit pagination controls"
```

- [x] **Step 5: Final controller verification**

Run:

```bash
git branch --show-current
git status --short
git log --oneline -8
cargo test -p mdid-desktop vault_audit -- --nocapture
cargo fmt --check
git diff --check
```

Expected: on `develop`, clean or only intentional uncommitted none, commands pass.

---

Execution note: This plan was executed on `feat/desktop-vault-audit-pagination-controls` and verified through `27257e8`; implementation commits were `d584024` and `26e1a9e`, followed by docs truth-sync commits `f6fabdd` and `27257e8`.
