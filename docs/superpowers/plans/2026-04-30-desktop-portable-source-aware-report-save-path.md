# Desktop Portable Source-Aware Report Save Path Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire the desktop UI save path field to the already-landed PHI-safe portable inspect/import source-aware report filename helper.

**Architecture:** Keep the behavior in the desktop app shell only: remember the imported portable artifact source name after a bounded drop/import handoff, suggest the sanitized report filename only for portable inspect/import safe-response reports, and preserve explicit user edits. Do not change runtime routes, report JSON contents, vault/decode/audit/export semantics, or any agent/controller workflow surface.

**Tech Stack:** Rust workspace, `mdid-desktop` crate, egui app shell, cargo tests/clippy.

---

## File Structure

- Modify: `crates/mdid-desktop/src/main.rs`
  - Add app-shell helpers for detecting whether the report save path is still the default/generic value.
  - Store the last imported portable artifact source name in `DesktopApp`.
  - Update the default report save path after portable artifact inspect/import responses, without overwriting non-default user-entered paths.
  - Add unit tests for the default-path update helper.
- Modify: `README.md`
  - Truth-sync completion snapshot and verification evidence.

### Task 1: Desktop portable source-aware report save path wiring

**Files:**
- Modify: `crates/mdid-desktop/src/main.rs`
- Modify: `README.md`
- Test: `crates/mdid-desktop/src/main.rs` unit tests

- [ ] **Step 1: Write the failing tests**

Add focused tests to `crates/mdid-desktop/src/main.rs` that assert:

```rust
#[test]
fn portable_response_report_path_uses_sanitized_imported_source_when_default() {
    let next = next_vault_response_report_save_path(
        "desktop-vault-response-report.json",
        Some("C:\\vaults\\Clinic Batch.mdid-portable.json"),
        &portable_inspect_report_state(),
    );
    assert_eq!(next, "Clinic-Batch.mdid-portable-portable-response-report.json");
}

#[test]
fn portable_response_report_path_preserves_user_overridden_path() {
    let next = next_vault_response_report_save_path(
        "C:\\exports\\custom-report.json",
        Some("C:\\vaults\\Clinic Batch.mdid-portable.json"),
        &portable_inspect_report_state(),
    );
    assert_eq!(next, "C:\\exports\\custom-report.json");
}

#[test]
fn portable_response_report_path_keeps_generic_path_for_non_portable_report_modes() {
    let mut state = DesktopVaultResponseState::default();
    state.apply_success(
        DesktopVaultResponseMode::VaultDecode,
        &serde_json::json!({"decoded_value_count": 1}),
    );
    let next = next_vault_response_report_save_path(
        "desktop-vault-response-report.json",
        Some("C:\\vaults\\Clinic Batch.mdid-portable.json"),
        &state,
    );
    assert_eq!(next, "desktop-vault-response-report.json");
}
```

Include a small `portable_inspect_report_state()` test helper that applies a bounded `InspectArtifact` success JSON.

- [ ] **Step 2: Run tests to verify RED**

Run:

```bash
cargo test -p mdid-desktop portable_response_report_path_ -- --nocapture
```

Expected: FAIL because `next_vault_response_report_save_path` does not exist.

- [ ] **Step 3: Implement the minimal app-shell wiring**

In `main.rs`:

```rust
const DEFAULT_VAULT_RESPONSE_REPORT_SAVE_PATH: &str = "desktop-vault-response-report.json";

fn is_default_vault_response_report_save_path(path: &str) -> bool {
    let trimmed = path.trim();
    trimmed.is_empty() || trimmed == DEFAULT_VAULT_RESPONSE_REPORT_SAVE_PATH
}

fn next_vault_response_report_save_path(
    current_path: &str,
    portable_source_name: Option<&str>,
    state: &DesktopVaultResponseState,
) -> String {
    if !is_default_vault_response_report_save_path(current_path) {
        return current_path.to_string();
    }

    state
        .safe_response_report_download_for_source(portable_source_name)
        .map(|download| download.file_name)
        .unwrap_or_else(|_| DEFAULT_VAULT_RESPONSE_REPORT_SAVE_PATH.to_string())
}
```

Add `portable_response_report_source_name: Option<String>` to `DesktopApp`. Set it from `DesktopFileImportTarget::PortableArtifactInspect(payload)` before moving fields into request state, clear it for workflow imports, and after `vault_response_state.apply_success(...)` call:

```rust
self.vault_response_report_save_path = next_vault_response_report_save_path(
    &self.vault_response_report_save_path,
    self.portable_response_report_source_name.as_deref(),
    &self.vault_response_state,
);
```

Use `DEFAULT_VAULT_RESPONSE_REPORT_SAVE_PATH.to_string()` in `Default`.

- [ ] **Step 4: Run targeted tests to verify GREEN**

Run:

```bash
cargo test -p mdid-desktop portable_response_report_path_ -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Run broader verification**

Run:

```bash
cargo test -p mdid-desktop --lib
cargo test -p mdid-desktop --bin mdid-desktop portable_response_report_path_ -- --nocapture
cargo clippy -p mdid-desktop --all-targets -- -D warnings
git diff --check
```

Expected: all PASS with no warnings and no whitespace errors.

- [ ] **Step 6: README truth-sync**

Update `README.md` Current repository status to mention the desktop app-shell save path now adopts sanitized imported portable artifact stems for bounded portable inspect/import response report saves while preserving user override paths. Keep Overall at 93% unless a broader feature landed.

- [ ] **Step 7: Commit**

Run:

```bash
git add crates/mdid-desktop/src/main.rs README.md docs/superpowers/plans/2026-04-30-desktop-portable-source-aware-report-save-path.md
git commit -m "fix(desktop): suggest portable response report save paths"
```

Expected: commit succeeds on `develop` or a GitFlow integration branch.

## Self-Review

- Spec coverage: This plan covers app-shell wiring, user override preservation, non-portable fallback, verification, README truth-sync, and commit.
- Placeholder scan: No TBD/TODO/implement-later placeholders remain.
- Type consistency: Helper names, struct fields, and response-state method names match the existing desktop code and previous landed helper.
