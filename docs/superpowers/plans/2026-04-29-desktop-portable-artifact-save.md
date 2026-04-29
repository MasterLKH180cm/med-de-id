# Desktop Portable Artifact Save Implementation Plan

> **For implementation workers:** REQUIRED SUB-SKILL: Use subagent-driven-development (recommended) or executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded PHI-safe desktop helper that extracts downloadable encrypted portable artifact JSON from a successful vault-export runtime response and writes it to a local artifact file without exposing decoded values, vault paths, passphrases, or audit details.

**Architecture:** Keep the desktop app as a thin sensitive-workstation surface: runtime vault export still happens through the existing localhost route and response state. Add a focused pure helper in `mdid-desktop` for validating/exporting the encrypted portable artifact object plus a small egui save-path action; do not add vault browsing, decoded-value display, generalized transfer workflow, or controller/orchestration behavior.

**Tech Stack:** Rust workspace, `mdid-desktop`, serde_json, existing egui desktop shell, cargo test/clippy.

---

## File Structure

- Modify: `crates/mdid-desktop/src/lib.rs`
  - Add a `DesktopPortableArtifactSaveError` enum.
  - Add `DesktopVaultResponseState::portable_artifact_download_json(...)` that returns a pretty JSON string only for `DesktopVaultResponseMode::VaultExport` responses containing an `artifact` object.
  - Add `write_portable_artifact_json(...)` helper that writes that pretty JSON string to a caller-specified local path.
  - Add tests proving success, fail-closed behavior for malformed/non-export responses, and no PHI/path/passphrase/audit leakage in the generated artifact JSON.
- Modify: `crates/mdid-desktop/src/main.rs`
  - Add a bounded “Save portable artifact JSON” path field/button in the vault/portable response workbench, enabled only when the latest vault-export response contains a valid artifact object.
  - Keep status/error copy generic and PHI-safe.
- Modify: `README.md`
  - Truth-sync Desktop app and Overall completion based on landed helper + tests.

### Task 1: Desktop portable artifact save helper

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`
- Test: existing unit tests in `crates/mdid-desktop/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Add these tests inside the existing `#[cfg(test)] mod tests` in `crates/mdid-desktop/src/lib.rs`:

```rust
#[test]
fn vault_export_download_json_contains_only_artifact_object() {
    let response = serde_json::json!({
        "artifact": {"version": 1, "ciphertext": "encrypted-payload", "nonce": "safe-nonce"},
        "record_count": 1,
        "vault_path": "/sensitive/Alice-vault.json",
        "vault_passphrase": "hunter2",
        "audit_event": {"detail": "exported Alice Example MRN 123"},
        "original_value": "Alice Example"
    });
    let mut state = DesktopVaultResponseState::default();
    state.apply_success(DesktopVaultResponseMode::VaultExport, &response);

    let artifact_json = state
        .portable_artifact_download_json(DesktopVaultResponseMode::VaultExport)
        .expect("valid artifact JSON should be available");

    assert!(artifact_json.contains("encrypted-payload"));
    assert!(artifact_json.contains("safe-nonce"));
    assert!(!artifact_json.contains("Alice Example"));
    assert!(!artifact_json.contains("/sensitive"));
    assert!(!artifact_json.contains("hunter2"));
    assert!(!artifact_json.contains("audit_event"));
    assert!(!artifact_json.contains("original_value"));
}

#[test]
fn vault_export_download_json_fails_closed_for_malformed_or_non_export_responses() {
    let mut state = DesktopVaultResponseState::default();
    state.apply_success(DesktopVaultResponseMode::InspectArtifact, &serde_json::json!({"record_count": 3}));
    assert_eq!(
        state.portable_artifact_download_json(DesktopVaultResponseMode::InspectArtifact),
        Err(DesktopPortableArtifactSaveError::NotVaultExport)
    );

    let mut export_state = DesktopVaultResponseState::default();
    export_state.apply_success(DesktopVaultResponseMode::VaultExport, &serde_json::json!({"artifact": "not an object"}));
    assert_eq!(
        export_state.portable_artifact_download_json(DesktopVaultResponseMode::VaultExport),
        Err(DesktopPortableArtifactSaveError::MissingArtifact)
    );
}

#[test]
fn write_portable_artifact_json_writes_pretty_artifact_without_sensitive_runtime_envelope() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("export.mdid-portable.json");
    let response = serde_json::json!({
        "artifact": {"version": 1, "ciphertext": "encrypted-payload"},
        "audit_event": {"detail": "patient Alice handoff"},
        "vault_path": "/secret/patient.vault"
    });
    let mut state = DesktopVaultResponseState::default();
    state.apply_success(DesktopVaultResponseMode::VaultExport, &response);

    let written = write_portable_artifact_json(&state, &path).expect("artifact write succeeds");
    let persisted = std::fs::read_to_string(&path).expect("artifact file exists");

    assert_eq!(written, path);
    assert_eq!(persisted, "{\n  \"ciphertext\": \"encrypted-payload\",\n  \"version\": 1\n}");
    assert!(!persisted.contains("Alice"));
    assert!(!persisted.contains("/secret"));
    assert!(!persisted.contains("audit_event"));
}
```

- [ ] **Step 2: Run tests to verify RED**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-desktop portable_artifact -- --nocapture`

Expected: FAIL with missing `DesktopVaultResponseState::portable_artifact_download_json`, missing `DesktopPortableArtifactSaveError`, and missing `write_portable_artifact_json`.

- [ ] **Step 3: Implement minimal helper**

Add public helper types/functions in `crates/mdid-desktop/src/lib.rs` near `DesktopVaultResponseState`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopPortableArtifactSaveError {
    NotVaultExport,
    MissingArtifact,
    Serialize(String),
    Io(String),
}

impl std::fmt::Display for DesktopPortableArtifactSaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotVaultExport => write!(f, "portable artifact save is only available for vault export responses"),
            Self::MissingArtifact => write!(f, "vault export response did not include a portable artifact object"),
            Self::Serialize(_) => write!(f, "portable artifact JSON could not be prepared"),
            Self::Io(_) => write!(f, "portable artifact JSON could not be written"),
        }
    }
}

impl std::error::Error for DesktopPortableArtifactSaveError {}
```

Add a `last_success_response: Option<serde_json::Value>` field to `DesktopVaultResponseState`; set it to `Some(response.clone())` in `apply_success(...)` and to `None` in `apply_error(...)`.

Add these methods/helpers:

```rust
impl DesktopVaultResponseState {
    pub fn portable_artifact_download_json(
        &self,
        mode: DesktopVaultResponseMode,
    ) -> Result<String, DesktopPortableArtifactSaveError> {
        if mode != DesktopVaultResponseMode::VaultExport {
            return Err(DesktopPortableArtifactSaveError::NotVaultExport);
        }
        let artifact = self
            .last_success_response
            .as_ref()
            .and_then(|response| response.get("artifact"))
            .and_then(|artifact| artifact.as_object())
            .ok_or(DesktopPortableArtifactSaveError::MissingArtifact)?;
        serde_json::to_string_pretty(&serde_json::Value::Object(artifact.clone()))
            .map_err(|error| DesktopPortableArtifactSaveError::Serialize(error.to_string()))
    }
}

pub fn write_portable_artifact_json(
    state: &DesktopVaultResponseState,
    path: impl AsRef<std::path::Path>,
) -> Result<std::path::PathBuf, DesktopPortableArtifactSaveError> {
    let artifact_json = state.portable_artifact_download_json(DesktopVaultResponseMode::VaultExport)?;
    let path = path.as_ref();
    std::fs::write(path, artifact_json)
        .map_err(|error| DesktopPortableArtifactSaveError::Io(error.to_string()))?;
    Ok(path.to_path_buf())
}
```

- [ ] **Step 4: Run tests to verify GREEN**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-desktop portable_artifact -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Run broader verification**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-desktop`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/mdid-desktop/src/lib.rs
git commit -m "feat(desktop): add portable artifact save helper"
```

### Task 2: Wire desktop save action and truth-sync README

**Files:**
- Modify: `crates/mdid-desktop/src/main.rs`
- Modify: `README.md`
- Test: existing unit tests in `crates/mdid-desktop/src/main.rs` and README grep verification

- [x] **Step 1: Write the failing UI-state test**

Add this test inside the existing `#[cfg(test)] mod tests` in `crates/mdid-desktop/src/main.rs`:

```rust
#[test]
fn app_save_portable_artifact_writes_artifact_json_without_sensitive_runtime_envelope() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("desktop-export.mdid-portable.json");
    let mut app = MdidDesktopApp::default();
    app.vault_response_mode = DesktopVaultResponseMode::VaultExport;
    app.vault_response_state.apply_success(
        DesktopVaultResponseMode::VaultExport,
        &serde_json::json!({
            "artifact": {"version": 1, "ciphertext": "encrypted-payload"},
            "audit_event": {"detail": "exported Alice Example"},
            "vault_path": "/secret/Alice.vault"
        }),
    );
    app.portable_artifact_save_path = path.to_string_lossy().to_string();

    app.save_portable_artifact_response();

    let saved = std::fs::read_to_string(&path).expect("artifact saved");
    assert!(saved.contains("encrypted-payload"));
    assert!(!saved.contains("Alice Example"));
    assert!(!saved.contains("/secret"));
    assert_eq!(
        app.portable_artifact_save_status,
        "Portable artifact JSON saved; encrypted contents only."
    );
}
```

- [x] **Step 2: Run test to verify RED**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-desktop app_save_portable_artifact --bin mdid-desktop -- --nocapture`

Expected: FAIL with missing app fields/method.

- [x] **Step 3: Implement minimal UI action**

In `crates/mdid-desktop/src/main.rs`:
- Import `write_portable_artifact_json`.
- Add fields to `MdidDesktopApp`: `portable_artifact_save_path: String`, `portable_artifact_save_status: String`.
- Initialize them in `Default` with `desktop-portable-artifact.mdid-portable.json` and empty status.
- Add method:

```rust
fn save_portable_artifact_response(&mut self) {
    match write_portable_artifact_json(
        &self.vault_response_state,
        self.portable_artifact_save_path.trim(),
    ) {
        Ok(_) => {
            self.portable_artifact_save_status =
                "Portable artifact JSON saved; encrypted contents only.".to_string();
        }
        Err(error) => {
            self.portable_artifact_save_status = error.to_string();
        }
    }
}
```

- In the vault/portable response workbench UI, render the save path field and button only when `self.vault_response_mode == DesktopVaultResponseMode::VaultExport`. Button calls `self.save_portable_artifact_response()`. Display `portable_artifact_save_status` when non-empty.
- Do not render raw artifact JSON in the UI.

- [x] **Step 4: Run tests to verify GREEN**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-desktop app_save_portable_artifact --bin mdid-desktop -- --nocapture`

Expected: PASS.

- [x] **Step 5: Update README completion snapshot**

In `README.md`, update the completion snapshot date/narrative:
- Desktop app: raise from `52%` to `57%` because desktop now has bounded encrypted portable artifact JSON save from successful vault export responses plus explicit portable artifact JSON import detection and localhost submission/rendering.
- Overall: raise from `83%` to `85%`.
- Keep CLI at `84%` and Browser/web at `61%` unless separately changed.
- Missing items must still list gaps to >=95%: full portable transfer workflow UX, vault browsing, decoded-value display, audit investigation, OCR/visual redaction, PDF/media rewrite/export, packaging/hardening, richer review/governance workflows.

- [x] **Step 6: Run verification**

Run:

```bash
source "$HOME/.cargo/env" && cargo test -p mdid-desktop
source "$HOME/.cargo/env" && cargo clippy -p mdid-desktop --all-targets -- -D warnings
git diff --check
grep -n "Completion snapshot\|CLI | 84%\|Browser/web | 61%\|Desktop app | 57%\|Overall | 85%" README.md
```

Expected: all commands pass and grep shows the updated snapshot.

- [x] **Step 7: Commit**

```bash
git add crates/mdid-desktop/src/main.rs README.md
git commit -m "feat(desktop): wire portable artifact save action"
```

## Self-Review

- Spec coverage: Task 1 provides the safe artifact extraction/write helper; Task 2 wires the desktop action and README truth-sync. The slice deliberately does not add vault browsing, decoded-value display, generalized transfer workflow, OCR, or rewrite/export beyond saving the encrypted portable artifact object returned by vault export.
- Placeholder scan: no TBD/TODO/implement-later placeholders remain.
- Type consistency: `DesktopPortableArtifactSaveError`, `portable_artifact_download_json`, and `write_portable_artifact_json` names are consistent across tasks.
