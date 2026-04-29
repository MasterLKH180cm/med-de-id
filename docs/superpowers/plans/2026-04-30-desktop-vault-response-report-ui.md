# Desktop Vault Response Report UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expose the existing PHI-safe desktop vault/portable response JSON report writer in the desktop workstation UI so already-rendered audit/decode/import/inspect summaries can be saved without leaking sensitive runtime details.

**Architecture:** Keep the behavior in the desktop binary shell because the shared library already owns the safe report writer and allowlist. The UI will add a dedicated save path/status and call `mdid_desktop::write_safe_vault_response_json` with the currently rendered `DesktopVaultResponseMode`; tests exercise the app action directly, not egui rendering.

**Tech Stack:** Rust workspace, `mdid-desktop`, egui/eframe desktop shell, `serde_json`, cargo tests/clippy.

---

## File Structure

- Modify: `crates/mdid-desktop/src/main.rs`
  - Add `vault_response_report_save_path` and `vault_response_report_save_status` to `DesktopApp`.
  - Initialize default path to `desktop-vault-response-report.json`.
  - Add `save_vault_response_report(&self, path)` helper using `mdid_desktop::write_safe_vault_response_json` and the current `DesktopVaultResponseMode` inferred from `vault_response_state.mode`.
  - Add `save_vault_response_report_response(&mut self)` action that sets PHI-safe status strings.
  - Render a save control in the vault/portable response workbench whenever a safe report is available.
  - Add unit tests for success and no-response status safety.
- Modify: `README.md`
  - Truth-sync desktop/browser/overall completion language and verification evidence after tests/reviews land.

### Task 1: Desktop app action for safe vault/portable response report save

**Files:**
- Modify: `crates/mdid-desktop/src/main.rs:1-775`
- Test: `crates/mdid-desktop/src/main.rs` inline `#[cfg(test)]` module

- [ ] **Step 1: Write the failing success test**

Add this test in the existing `#[cfg(test)] mod tests` after `app_save_portable_artifact_writes_artifact_json_without_sensitive_runtime_envelope`:

```rust
    #[test]
    fn app_save_vault_response_report_writes_safe_audit_summary_only() {
        let dir = std::env::temp_dir().join(format!(
            "mdid-desktop-vault-response-report-ui-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir(&dir).expect("tempdir");
        let path = dir.join("patient-jane-doe-mrn-12345-vault-report.json");
        let mut app = DesktopApp::default();
        app.vault_response_state.apply_success(
            mdid_desktop::DesktopVaultResponseMode::VaultAudit,
            &serde_json::json!({
                "events": [
                    {
                        "event_id": "evt-1",
                        "kind": "decode",
                        "actor": "clinician-a",
                        "record_id": "record-7",
                        "scope": ["patient_name"],
                        "occurred_at": "2026-04-30T01:00:00Z",
                        "detail": "decoded Alice Example with token <NAME-1>"
                    }
                ],
                "vault_path": "/secret/Alice.vault",
                "passphrase": "do-not-save"
            }),
        );
        app.vault_response_report_save_path = path.to_string_lossy().to_string();

        app.save_vault_response_report_response();

        let saved = std::fs::read_to_string(&path).expect("safe vault report saved");
        assert!(saved.contains("\"mode\": \"vault_audit\""));
        assert!(saved.contains("\"event_count\": 1"));
        assert!(saved.contains("\"kind\": \"decode\""));
        assert!(!saved.contains("Alice Example"));
        assert!(!saved.contains("<NAME-1>"));
        assert!(!saved.contains("/secret"));
        assert!(!saved.contains("do-not-save"));
        assert_eq!(
            app.vault_response_report_save_status,
            "Safe vault/portable response report saved."
        );
        assert!(!app
            .vault_response_report_save_status
            .contains(path.to_string_lossy().as_ref()));
        assert!(!app.vault_response_report_save_status.contains("jane-doe"));
        assert!(!app.vault_response_report_save_status.contains("12345"));
        std::fs::remove_dir_all(dir).expect("remove tempdir");
    }
```

- [ ] **Step 2: Run the targeted test and verify RED**

Run:

```bash
cargo test -p mdid-desktop app_save_vault_response_report_writes_safe_audit_summary_only -- --nocapture
```

Expected: FAIL to compile because `DesktopApp` has no `vault_response_report_save_path`, no `vault_response_report_save_status`, and no `save_vault_response_report_response` method.

- [ ] **Step 3: Implement minimal app state and save action**

In `crates/mdid-desktop/src/main.rs`:

1. Add `write_safe_vault_response_json` to the top `use mdid_desktop::{ ... }` list.
2. Add fields to `DesktopApp`:

```rust
    vault_response_report_save_path: String,
    vault_response_report_save_status: String,
```

3. Initialize them in `Default`:

```rust
            vault_response_report_save_path: "desktop-vault-response-report.json".to_string(),
            vault_response_report_save_status: String::new(),
```

4. Add these methods in `impl DesktopApp` after `save_portable_artifact_response`:

```rust
    fn save_vault_response_report(&self, path: impl AsRef<Path>) -> Result<(), String> {
        write_safe_vault_response_json(
            &self.vault_response_state,
            self.vault_response_state.mode,
            path,
        )
        .map(|_| ())
    }

    fn save_vault_response_report_response(&mut self) {
        match self.save_vault_response_report(self.vault_response_report_save_path.trim()) {
            Ok(()) => {
                self.vault_response_report_save_status =
                    "Safe vault/portable response report saved.".to_string();
            }
            Err(error) => {
                self.vault_response_report_save_status = error;
            }
        }
    }
```

- [ ] **Step 4: Run the targeted test and verify GREEN**

Run:

```bash
cargo test -p mdid-desktop app_save_vault_response_report_writes_safe_audit_summary_only -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Add the no-response PHI-safe status test**

Add this test after the success test:

```rust
    #[test]
    fn app_save_vault_response_report_action_sets_phi_safe_no_response_status() {
        let path = "/tmp/patient-jane-doe-mrn-12345-vault-report.json";
        let mut app = DesktopApp {
            vault_response_report_save_path: path.to_string(),
            ..DesktopApp::default()
        };

        app.save_vault_response_report_response();

        assert_eq!(
            app.vault_response_report_save_status,
            "vault response report save failed: no safe response summary is available"
        );
        assert!(!app.vault_response_report_save_status.contains(path));
        assert!(!app.vault_response_report_save_status.contains("jane-doe"));
        assert!(!app.vault_response_report_save_status.contains("12345"));
    }
```

- [ ] **Step 6: Run the no-response test and verify RED/GREEN honestly**

Run:

```bash
cargo test -p mdid-desktop app_save_vault_response_report_action_sets_phi_safe_no_response_status -- --nocapture
```

Expected: It may pass immediately if the existing library writer already returns the correct safe no-response error. If it passes immediately, record this as test-hardening truthfulness evidence; do not force production churn.

- [ ] **Step 7: Render the UI save controls**

In the vault/portable response workbench after the portable artifact JSON block and before the runtime-shaped response workbench separator, add:

```rust
            if self
                .vault_response_state
                .safe_response_report_json(self.vault_response_state.mode)
                .is_ok()
            {
                ui.label("Save safe vault/portable response report JSON");
                ui.text_edit_singleline(&mut self.vault_response_report_save_path);
                if ui
                    .button("Save safe vault/portable response report JSON")
                    .clicked()
                {
                    self.save_vault_response_report_response();
                }
                if !self.vault_response_report_save_status.is_empty() {
                    ui.label(&self.vault_response_report_save_status);
                }
            }
```

- [ ] **Step 8: Run desktop tests and clippy**

Run:

```bash
cargo test -p mdid-desktop --lib
cargo test -p mdid-desktop --bin mdid-desktop
cargo clippy -p mdid-desktop --all-targets -- -D warnings
```

Expected: all PASS with no clippy warnings.

- [ ] **Step 9: Commit**

```bash
git add crates/mdid-desktop/src/main.rs
git commit -m "feat(desktop): expose safe vault response report save"
```

### Task 2: README truth-sync for desktop vault response report UI

**Files:**
- Modify: `README.md:62-75`

- [ ] **Step 1: Update completion snapshot text**

Edit the current repository status section to mention that the desktop app now exposes the PHI-safe vault/portable response report JSON save action in the desktop UI, not only the helper-layer writer.

- [ ] **Step 2: Run doc hygiene and relevant verification**

Run:

```bash
git diff --check
cargo test -p mdid-desktop app_save_vault_response_report_writes_safe_audit_summary_only -- --nocapture
cargo test -p mdid-desktop app_save_vault_response_report_action_sets_phi_safe_no_response_status -- --nocapture
cargo clippy -p mdid-desktop --all-targets -- -D warnings
```

Expected: all PASS.

- [ ] **Step 3: Commit docs**

```bash
git add README.md
git commit -m "docs: truth-sync desktop vault report UI"
```

## Self-Review

- Spec coverage: The plan covers app state, app action, UI rendering, no-response safety, success safety, verification, and README truth-sync. No PDF rewrite, OCR, agent/controller, or broader workflow behavior is included.
- Placeholder scan: No TBD/TODO/fill-in-later placeholders are present.
- Type consistency: The plan uses existing `DesktopVaultResponseState`, `DesktopVaultResponseMode`, `write_safe_vault_response_json`, and `safe_response_report_json` names consistently.
