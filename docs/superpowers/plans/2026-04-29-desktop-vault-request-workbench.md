# Desktop Vault Request Workbench Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded desktop-side vault/decode/audit request-preparation workbench foundation for existing localhost runtime vault routes.

**Architecture:** Keep `mdid-desktop` as a local workstation request-preparation surface: add pure Rust request-state helpers for vault decode and audit browsing, reuse existing runtime route contracts, and render only honest local-runtime envelopes. This does not add vault browsing, credential storage, import/export transfer UX, controller/orchestration semantics, or production packaging.

**Tech Stack:** Rust, serde_json, existing `mdid-runtime` HTTP JSON contracts, existing `mdid-desktop` pure helper test pattern.

---

## File Structure

- Modify: `crates/mdid-desktop/src/lib.rs` — add vault runtime request modes, request-state helpers, validation, UI copy, tests, and minimal rendering support if needed by the existing app shell.
- Modify: `README.md` — truth-sync Desktop app and Overall completion plus remaining vault/decode/audit limitations.
- Modify: `docs/superpowers/plans/2026-04-29-desktop-vault-request-workbench.md` — mark task steps complete after implementation.

## Task 1: Desktop vault decode/audit request helpers

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/plans/2026-04-29-desktop-vault-request-workbench.md`

- [ ] **Step 1: Write failing tests for bounded vault request preparation**

Add these tests inside `#[cfg(test)] mod tests` in `crates/mdid-desktop/src/lib.rs`:

```rust
#[test]
fn desktop_vault_decode_request_builds_existing_runtime_contract() {
    let state = DesktopVaultRequestState {
        mode: DesktopVaultMode::Decode,
        vault_path: "C:/vaults/local.mdid".to_string(),
        vault_passphrase: "correct horse battery staple".to_string(),
        record_ids_json: r#"["550e8400-e29b-41d4-a716-446655440000"]"#.to_string(),
        output_target: "review-workbench".to_string(),
        audit_kind: None,
        audit_actor: None,
    };

    let request = state.try_build_request().expect("decode request should build");

    assert_eq!(request.route, "/vault/decode");
    assert_eq!(request.body["vault_path"], "C:/vaults/local.mdid");
    assert_eq!(request.body["vault_passphrase"], "correct horse battery staple");
    assert_eq!(request.body["record_ids"][0], "550e8400-e29b-41d4-a716-446655440000");
    assert_eq!(request.body["output_target"], "review-workbench");
}

#[test]
fn desktop_vault_audit_request_builds_read_only_filter_contract() {
    let state = DesktopVaultRequestState {
        mode: DesktopVaultMode::AuditEvents,
        vault_path: "C:/vaults/local.mdid".to_string(),
        vault_passphrase: "correct horse battery staple".to_string(),
        record_ids_json: "[]".to_string(),
        output_target: "review-workbench".to_string(),
        audit_kind: Some("Decode".to_string()),
        audit_actor: Some("Desktop".to_string()),
    };

    let request = state.try_build_request().expect("audit request should build");

    assert_eq!(request.route, "/vault/audit/events");
    assert_eq!(request.body["vault_path"], "C:/vaults/local.mdid");
    assert_eq!(request.body["vault_passphrase"], "correct horse battery staple");
    assert_eq!(request.body["kind"], "Decode");
    assert_eq!(request.body["actor"], "Desktop");
    assert!(request.body.get("record_ids").is_none());
}

#[test]
fn desktop_vault_request_validation_rejects_blank_sensitive_inputs() {
    let mut state = DesktopVaultRequestState::default();
    assert_eq!(state.try_build_request(), Err(DesktopVaultValidationError::BlankVaultPath));

    state.vault_path = "C:/vaults/local.mdid".to_string();
    assert_eq!(state.try_build_request(), Err(DesktopVaultValidationError::BlankVaultPassphrase));

    state.vault_passphrase = "correct horse battery staple".to_string();
    state.record_ids_json = "not json".to_string();
    assert!(matches!(
        state.try_build_request(),
        Err(DesktopVaultValidationError::InvalidRecordIdsJson(_))
    ));
}

#[test]
fn desktop_vault_workbench_copy_is_bounded_and_non_orchestrating() {
    assert!(DESKTOP_VAULT_WORKBENCH_COPY.contains("existing localhost runtime vault routes"));
    assert!(DESKTOP_VAULT_WORKBENCH_COPY.contains("does not store passphrases"));
    assert!(DESKTOP_VAULT_WORKBENCH_COPY.contains("does not add controller, agent, or orchestration behavior"));
}
```

- [ ] **Step 2: Run targeted tests and verify RED**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-desktop desktop_vault_decode_request_builds_existing_runtime_contract desktop_vault_audit_request_builds_read_only_filter_contract desktop_vault_request_validation_rejects_blank_sensitive_inputs desktop_vault_workbench_copy_is_bounded_and_non_orchestrating -- --nocapture
```

Expected: FAIL because `DesktopVaultRequestState`, `DesktopVaultMode`, `DesktopVaultValidationError`, and `DESKTOP_VAULT_WORKBENCH_COPY` do not exist.

- [ ] **Step 3: Implement minimal pure request helpers**

Add the following public types and helpers near the existing desktop runtime request state in `crates/mdid-desktop/src/lib.rs`:

```rust
pub const DESKTOP_VAULT_WORKBENCH_COPY: &str = "Bounded desktop vault workbench: prepares request envelopes for existing localhost runtime vault routes, including explicit decode and read-only audit browsing. It does not store passphrases, browse vault contents directly, transfer portable artifacts, or add controller, agent, or orchestration behavior.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopVaultMode {
    Decode,
    AuditEvents,
}

impl DesktopVaultMode {
    pub fn route(self) -> &'static str {
        match self {
            Self::Decode => "/vault/decode",
            Self::AuditEvents => "/vault/audit/events",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopVaultRequestState {
    pub mode: DesktopVaultMode,
    pub vault_path: String,
    pub vault_passphrase: String,
    pub record_ids_json: String,
    pub output_target: String,
    pub audit_kind: Option<String>,
    pub audit_actor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopVaultValidationError {
    BlankVaultPath,
    BlankVaultPassphrase,
    BlankOutputTarget,
    EmptyRecordIds,
    InvalidRecordIdsJson(String),
}
```

Implement `Default` with `mode: Decode`, blank `vault_path`, blank `vault_passphrase`, `record_ids_json: "[]"`, `output_target: "desktop-workbench"`, and no audit filters. Implement `try_build_request(&self) -> Result<DesktopWorkflowRequest, DesktopVaultValidationError>` so decode emits `vault_path`, `vault_passphrase`, `record_ids`, `output_target`; audit emits `vault_path`, `vault_passphrase`, `kind`, and `actor`, where filters are JSON null when absent. Parse record ids as `Vec<uuid::Uuid>` so invalid UUIDs fail through `InvalidRecordIdsJson`.

- [ ] **Step 4: Run targeted tests and verify GREEN**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-desktop desktop_vault_decode_request_builds_existing_runtime_contract desktop_vault_audit_request_builds_read_only_filter_contract desktop_vault_request_validation_rejects_blank_sensitive_inputs desktop_vault_workbench_copy_is_bounded_and_non_orchestrating -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Run desktop and runtime contract tests**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-desktop
cargo test -p mdid-runtime --test runtime_http vault_decode vault_audit
cargo clippy -p mdid-desktop --all-targets -- -D warnings
git diff --check
```

Expected: PASS for desktop tests, relevant runtime vault HTTP tests, clippy, and whitespace check.

- [ ] **Step 6: Update README completion truthfully**

Update `README.md` completion snapshot based only on landed behavior:
- Desktop app may increase from `30%` to `33%` only if the helpers and tests pass, describing bounded vault decode/audit request-preparation helpers without claiming real vault browsing.
- Overall may increase from `47%` to `48%` only if tests pass.
- Missing items must still include deeper desktop vault browsing, decode workflow execution UX, audit investigation workflow, portable transfer UX, OCR, visual redaction, PDF rewrite/export, and full review workflows.

- [ ] **Step 7: Mark this plan task complete and commit**

Update this plan's checkboxes for completed steps, then run:

```bash
git add crates/mdid-desktop/src/lib.rs README.md docs/superpowers/plans/2026-04-29-desktop-vault-request-workbench.md
git commit -m "feat(desktop): add bounded vault request workbench helpers"
```

## Self-Review

- Spec coverage: This plan covers only bounded desktop vault decode/audit request preparation against existing runtime routes and README truth-sync.
- Placeholder scan: No TBD/TODO placeholders are present; tests, commands, exact strings, and file paths are explicit.
- Type consistency: `DesktopVaultMode`, `DesktopVaultRequestState`, `DesktopVaultValidationError`, and `DESKTOP_VAULT_WORKBENCH_COPY` are consistently named.
