# Desktop Vault Runtime Submission Implementation Plan

> **For implementation workers:** REQUIRED SUB-SKILL: Use subagent-driven-development or executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire the existing bounded desktop vault and portable request/response helpers into a real localhost runtime submission UI path.

**Architecture:** Keep `mdid-desktop` thin: the egui shell chooses between existing de-identification request state, existing vault request state, and existing portable request state, submits the prepared `DesktopWorkflowRequest` to the configured localhost runtime, and renders through the existing PHI-safe response helpers. No vault browsing, decoded-value display, audit investigation, portable transfer workflow management, OCR, PDF rewrite/export, or agent/controller/orchestration behavior is added.

**Tech Stack:** Rust, egui/eframe, serde_json, existing `mdid-desktop` library helpers, cargo tests/clippy.

---

## File Structure

- Modify: `crates/mdid-desktop/src/lib.rs`
  - Add a small `DesktopRuntimeSurface`/submission classification helper that maps workflow/vault/portable modes to response rendering modes and submit labels without exposing sensitive data.
  - Add tests for vault/portable submission classification and PHI-safe response handoff behavior.
- Modify: `crates/mdid-desktop/src/main.rs`
  - Add desktop app fields for `DesktopVaultRequestState`, `DesktopPortableRequestState`, `DesktopVaultResponseState`, and a submission mode enum covering workflow/vault/portable requests.
  - Add bounded UI sections for vault decode/audit and portable export/inspect/import request submission using existing state objects.
  - Keep response rendering PHI-safe through `DesktopWorkflowResponseState` and `DesktopVaultResponseState`.
- Modify: `README.md`
  - Truth-sync completion snapshot after landed desktop vault/portable runtime submission wiring.

### Task 1: Desktop vault/portable runtime submission wiring

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`
- Modify: `crates/mdid-desktop/src/main.rs`
- Test: `crates/mdid-desktop/src/lib.rs` test module

- [x] **Step 1: Write failing library tests for submission classification and PHI-safe vault response rendering**

Add tests to the existing `#[cfg(test)]` module in `crates/mdid-desktop/src/lib.rs`:

```rust
#[test]
fn vault_and_portable_submission_modes_map_to_phi_safe_response_modes() {
    assert_eq!(
        DesktopRuntimeSubmissionMode::Vault(DesktopVaultMode::Decode).vault_response_mode(),
        Some(DesktopVaultResponseMode::VaultDecode)
    );
    assert_eq!(
        DesktopRuntimeSubmissionMode::Vault(DesktopVaultMode::AuditEvents).vault_response_mode(),
        Some(DesktopVaultResponseMode::VaultAudit)
    );
    assert_eq!(
        DesktopRuntimeSubmissionMode::Portable(DesktopPortableMode::VaultExport).vault_response_mode(),
        Some(DesktopVaultResponseMode::VaultExport)
    );
    assert_eq!(
        DesktopRuntimeSubmissionMode::Portable(DesktopPortableMode::InspectArtifact).vault_response_mode(),
        Some(DesktopVaultResponseMode::InspectArtifact)
    );
    assert_eq!(
        DesktopRuntimeSubmissionMode::Portable(DesktopPortableMode::ImportArtifact).vault_response_mode(),
        Some(DesktopVaultResponseMode::ImportArtifact)
    );
    assert_eq!(
        DesktopRuntimeSubmissionMode::Workflow(DesktopWorkflowMode::CsvText).vault_response_mode(),
        None
    );
}

#[test]
fn vault_runtime_success_handoff_uses_safe_summary_not_raw_response_values() {
    let response = serde_json::json!({
        "decoded_value_count": 1,
        "values": [{"original_value": "Alice Patient", "token": "PATIENT_TOKEN"}],
        "audit_event": {"kind": "decode", "detail": "released to Dr Patient"}
    });
    let mut state = DesktopVaultResponseState::default();
    let mode = DesktopRuntimeSubmissionMode::Vault(DesktopVaultMode::Decode)
        .vault_response_mode()
        .expect("vault response mode");

    state.apply_success(mode, &response);

    assert!(state.summary.contains("decoded values: 1"));
    assert!(!state.summary.contains("Alice Patient"));
    assert!(!state.summary.contains("PATIENT_TOKEN"));
    assert!(!state.summary.contains("Dr Patient"));
}
```

- [x] **Step 2: Run tests to verify RED**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-desktop vault_and_portable_submission_modes_map_to_phi_safe_response_modes vault_runtime_success_handoff_uses_safe_summary_not_raw_response_values -- --nocapture`

Expected: FAIL because `DesktopRuntimeSubmissionMode` does not exist yet.

- [x] **Step 3: Add minimal submission classification helper**

Add near existing desktop runtime submission types in `crates/mdid-desktop/src/lib.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopRuntimeSubmissionMode {
    Workflow(DesktopWorkflowMode),
    Vault(DesktopVaultMode),
    Portable(DesktopPortableMode),
}

impl DesktopRuntimeSubmissionMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Workflow(mode) => mode.label(),
            Self::Vault(DesktopVaultMode::Decode) => "Vault decode",
            Self::Vault(DesktopVaultMode::AuditEvents) => "Vault audit events",
            Self::Portable(DesktopPortableMode::VaultExport) => "Portable vault export",
            Self::Portable(DesktopPortableMode::InspectArtifact) => "Portable artifact inspect",
            Self::Portable(DesktopPortableMode::ImportArtifact) => "Portable artifact import",
        }
    }

    pub fn vault_response_mode(self) -> Option<DesktopVaultResponseMode> {
        match self {
            Self::Workflow(_) => None,
            Self::Vault(DesktopVaultMode::Decode) => Some(DesktopVaultResponseMode::VaultDecode),
            Self::Vault(DesktopVaultMode::AuditEvents) => Some(DesktopVaultResponseMode::VaultAudit),
            Self::Portable(DesktopPortableMode::VaultExport) => Some(DesktopVaultResponseMode::VaultExport),
            Self::Portable(DesktopPortableMode::InspectArtifact) => Some(DesktopVaultResponseMode::InspectArtifact),
            Self::Portable(DesktopPortableMode::ImportArtifact) => Some(DesktopVaultResponseMode::ImportArtifact),
        }
    }
}
```

- [x] **Step 4: Run tests to verify GREEN**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-desktop vault_and_portable_submission_modes_map_to_phi_safe_response_modes vault_runtime_success_handoff_uses_safe_summary_not_raw_response_values -- --nocapture`

Expected: PASS.

- [x] **Step 5: Wire main UI and async submit state**

Modify `crates/mdid-desktop/src/main.rs` so `DesktopApp` imports and stores:

```rust
use mdid_desktop::{
    DesktopPortableMode, DesktopPortableRequestState, DesktopRuntimeSettings,
    DesktopRuntimeSubmissionMode, DesktopRuntimeSubmissionSnapshot, DesktopRuntimeSubmitError,
    DesktopVaultMode, DesktopVaultRequestState, DesktopVaultResponseState, DesktopWorkflowMode,
    DesktopWorkflowRequest, DesktopWorkflowRequestState, DesktopWorkflowResponseState,
};
```

Change `runtime_submission_mode` to `Option<DesktopRuntimeSubmissionMode>`, add fields `vault_request_state`, `portable_request_state`, and `vault_response_state`, and route successful responses as:

```rust
match mode {
    DesktopRuntimeSubmissionMode::Workflow(workflow_mode) => {
        self.response_state.apply_success_json(workflow_mode, envelope);
    }
    DesktopRuntimeSubmissionMode::Vault(_) | DesktopRuntimeSubmissionMode::Portable(_) => {
        let response_mode = mode.vault_response_mode().expect("vault response mode");
        self.vault_response_state.apply_success(response_mode, &envelope);
    }
}
```

Add a private helper on `DesktopApp`:

```rust
fn submit_runtime_request(
    &mut self,
    mode: DesktopRuntimeSubmissionMode,
    request: DesktopWorkflowRequest,
) {
    match self.runtime_settings.client() {
        Ok(client) => {
            let route = request.route;
            let (sender, receiver) = std::sync::mpsc::channel();
            self.runtime_submission_receiver = Some(receiver);
            self.runtime_submission_mode = Some(mode);
            let banner = format!("Submitting {route} to local runtime...");
            match mode.vault_response_mode() {
                Some(response_mode) => {
                    self.vault_response_state.banner = banner;
                    self.vault_response_state.error = None;
                    self.vault_response_state.summary.clear();
                    self.vault_response_state.artifact_notice.clear();
                    let _ = response_mode;
                }
                None => {
                    self.response_state.banner = banner;
                    self.response_state.error = None;
                }
            }
            std::thread::spawn(move || {
                let _ = sender.send(client.submit(&request));
            });
        }
        Err(error) => match mode.vault_response_mode() {
            Some(response_mode) => self.vault_response_state.apply_error(response_mode, format!("{error:?}")),
            None => self.response_state.apply_error(format!("{error:?}")),
        },
    }
}
```

Add UI controls for the existing vault and portable states: mode combo, vault path/passphrase, record ids JSON/output target/justification for decode, optional kind/actor for audit, portable export/inspect/import fields, submit buttons that call `try_build_request()` and then `submit_runtime_request(...)`. Keep labels bounded and do not display decoded values or raw audit details.

- [x] **Step 6: Run relevant desktop tests and clippy**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-desktop -- --nocapture`
Expected: PASS.

Run: `source "$HOME/.cargo/env" && cargo clippy -p mdid-desktop --all-targets -- -D warnings`
Expected: PASS.

- [x] **Step 7: Commit**

Run:

```bash
git add crates/mdid-desktop/src/lib.rs crates/mdid-desktop/src/main.rs
git commit -m "feat(desktop): submit vault and portable runtime requests"
```

### Task 2: README completion truth-sync

**Completion evidence:** Task 1 landed in commits `75e9bb8` and `5cde7e7`. Verification for the landed desktop work passed with `cargo test -p mdid-desktop -- --nocapture` and `cargo clippy -p mdid-desktop --all-targets -- -D warnings`. Task 2 truth-sync updates the README snapshot to those landed commits and verification results without changing code.

**Files:**
- Modify: `README.md`

- [x] **Step 1: Update completion snapshot**

Update the snapshot to cite this branch/commit and grounded verification. Set desktop completion to 49% and overall completion to 77% only if Task 1 landed and desktop tests/clippy pass. Keep CLI 84% and Browser/web 49% unchanged. Missing items must still include full desktop vault/decode/audit UX, file picker depth, browser vault export/import, OCR/visual redaction, governance/review polish, and deeper policy/detection.

- [x] **Step 2: Verify README wording**

Run: `grep -n "Completion snapshot\|CLI | 84%\|Browser/web | 49%\|Desktop app | 49%\|Overall | 77%\|Missing items" README.md`
Expected: matching lines.

Run: `grep -niE 'agent workflow|controller loop|planner|coder|reviewer|complete_command|claim' README.md || true`
Expected: no product-roadmap scope drift matches.

- [x] **Step 3: Commit**

Run:

```bash
git add README.md docs/superpowers/plans/2026-04-29-desktop-vault-runtime-submission.md
git commit -m "docs: truth-sync desktop vault submission completion"
```
