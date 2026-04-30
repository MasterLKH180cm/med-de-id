# Desktop Decode Values Export Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded desktop workstation export for already-rendered vault decode values so explicit decode results can be saved locally without exposing PHI in the general safe response report.

**Architecture:** Keep the existing PHI-safe response report unchanged and add a separate high-risk decode-values export path that is available only after a successful vault decode response. The helper writes the raw `decoded_values` object from the runtime response plus minimal workstation provenance fields to an explicit local JSON path; it must reject non-decode modes and missing decode values.

**Tech Stack:** Rust workspace, `mdid-desktop`, serde_json, egui desktop shell tests, Cargo tests.

---

## File Structure

- Modify `crates/mdid-desktop/src/lib.rs`: add decode-values export error variants, helper methods on `DesktopVaultResponseState`, writer function, and tests.
- Modify `crates/mdid-desktop/src/main.rs`: expose a separate decode-values save path/button in the vault workbench UI only for successful decode responses.
- Modify `README.md`: truth-sync desktop completion and verification evidence after the implementation lands.

### Task 1: Helper-layer decode values JSON export

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`
- Test: `crates/mdid-desktop/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Add tests near existing vault response report tests:

```rust
#[test]
fn desktop_decode_values_export_contains_decoded_values_for_decode_response() {
    let mut state = DesktopVaultResponseState::default();
    let response = serde_json::json!({
        "decoded_value_count": 2,
        "decoded_values": {
            "record-1": {"name": "Jane Doe"},
            "record-2": {"mrn": "12345"}
        },
        "audit_event_id": "audit-1"
    });

    state.apply_success(DesktopVaultResponseMode::VaultDecode, &response);

    let json = state.decode_values_export_json().expect("decode values export");

    assert_eq!(json["mode"], "vault_decode_values");
    assert_eq!(json["decoded_value_count"], 2);
    assert_eq!(json["decoded_values"]["record-1"]["name"], "Jane Doe");
    assert_eq!(json["decoded_values"]["record-2"]["mrn"], "12345");
    assert_eq!(json["disclosure"], "high-risk decoded values; store only in an approved local workstation location");
}

#[test]
fn desktop_decode_values_export_rejects_non_decode_response() {
    let mut state = DesktopVaultResponseState::default();
    state.apply_success(
        DesktopVaultResponseMode::VaultAudit,
        &serde_json::json!({"event_count": 0, "events": []}),
    );

    let error = state.decode_values_export_json().expect_err("not decode");

    assert_eq!(
        error.to_string(),
        "decoded values export is only available for successful vault decode responses"
    );
}

#[test]
fn desktop_decode_values_export_rejects_missing_decoded_values() {
    let mut state = DesktopVaultResponseState::default();
    state.apply_success(
        DesktopVaultResponseMode::VaultDecode,
        &serde_json::json!({"decoded_value_count": 0}),
    );

    let error = state.decode_values_export_json().expect_err("missing values");

    assert_eq!(error.to_string(), "decoded values are unavailable");
}
```

- [ ] **Step 2: Run tests to verify RED**

Run: `cargo test -p mdid-desktop decode_values_export -- --nocapture`

Expected: FAIL because `decode_values_export_json` does not exist.

- [ ] **Step 3: Implement minimal helper**

Add a `DesktopDecodedValuesExportError` enum with `NotVaultDecode`, `MissingDecodedValues`, `Io`, and `InvalidJson` variants; implement `Display` with exactly the tested strings. Add `DesktopVaultResponseState::decode_values_export_json()` that requires `last_success_mode == Some(DesktopVaultResponseMode::VaultDecode)`, reads `last_success_response.decoded_values` as an object, and returns:

```json
{
  "mode": "vault_decode_values",
  "decoded_value_count": <decoded_value_count from response or decoded_values object length>,
  "disclosure": "high-risk decoded values; store only in an approved local workstation location",
  "decoded_values": <raw decoded_values object>
}
```

Add `write_desktop_decode_values_json(state, path)` which pretty-serializes that JSON and writes it to `path`.

- [ ] **Step 4: Run helper tests to verify GREEN**

Run: `cargo test -p mdid-desktop decode_values_export -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Run broader desktop library tests**

Run: `cargo test -p mdid-desktop --lib`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/mdid-desktop/src/lib.rs docs/superpowers/plans/2026-04-30-desktop-decode-values-export.md
git commit -m "feat(desktop): export decoded vault values"
```

### Task 2: Desktop UI save action for decode values

**Files:**
- Modify: `crates/mdid-desktop/src/main.rs`
- Test: `crates/mdid-desktop/src/main.rs`

- [ ] **Step 1: Write the failing UI test**

Add a test near existing app save-vault-response-report tests that seeds a successful decode response and asserts the decode-values save path is present/enabled while the safe response report remains separate.

- [ ] **Step 2: Run test to verify RED**

Run: `cargo test -p mdid-desktop app_save_decode_values -- --nocapture`

Expected: FAIL because no decode-values save action exists.

- [ ] **Step 3: Implement minimal UI wiring**

Add a `decode_values_save_path` field to the app state, derive a sanitized default path from the vault source stem, render a separate button labelled `Save decoded values JSON` only when `DesktopVaultResponseState::decode_values_export_json()` succeeds, and call `write_desktop_decode_values_json` on click. Status messages must not echo paths or decoded values.

- [ ] **Step 4: Run UI tests to verify GREEN**

Run: `cargo test -p mdid-desktop app_save_decode_values -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Run desktop bin tests**

Run: `cargo test -p mdid-desktop --bin mdid-desktop`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/mdid-desktop/src/main.rs crates/mdid-desktop/src/lib.rs
git commit -m "feat(desktop): expose decode values save action"
```

### Task 3: README truth-sync and verification

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update completion snapshot**

Update the README completion snapshot to say desktop app increases from 71% to 72% and overall remains 93% because this removes a meaningful desktop decode workflow gap but does not land OCR/visual redaction, PDF/media rewrite/export, packaging/hardening, or deeper policy/detection.

- [ ] **Step 2: Run verification**

Run:

```bash
cargo test -p mdid-desktop decode_values_export -- --nocapture
cargo test -p mdid-desktop app_save_decode_values -- --nocapture
cargo test -p mdid-desktop
cargo clippy -p mdid-desktop --all-targets -- -D warnings
cargo fmt --check
git diff --check
```

Expected: all PASS.

- [ ] **Step 3: Commit**

```bash
git add README.md docs/superpowers/plans/2026-04-30-desktop-decode-values-export.md
git commit -m "docs: sync desktop decode values export status"
```

## Self-Review

- Spec coverage: The plan covers helper export, desktop UI exposure, tests, verification, and README completion truth-sync.
- Placeholder scan: No TBD/TODO/implement-later placeholders remain.
- Type consistency: `decode_values_export_json`, `write_desktop_decode_values_json`, and `DesktopDecodedValuesExportError` are consistently named across tasks.
