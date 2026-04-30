# Browser Decode Values Export Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an explicit high-risk browser vault decode values JSON download for successful decode responses while keeping PHI-safe response reports separate.

**Architecture:** `mdid-browser` already hides decoded values in the normal vault decode pane and PHI-safe response report. This slice adds a separate decoded-values-only download path gated to `InputMode::VaultDecode` with a parseable `decoded_values` object in the runtime output, uses sanitized source-derived filenames, and renders a separate explicit warning/action in the browser UI.

**Tech Stack:** Rust workspace, Leptos browser crate, serde_json, cargo tests, strict TDD and SDD review.

---

## File Structure

- Modify: `crates/mdid-browser/src/app.rs` — add decoded-values download helper methods, browser UI action, and focused unit tests.
- Modify: `README.md` — truth-sync completion snapshot and verification evidence after the implementation lands.

### Task 1: Browser decoded-values download helper

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`

- [ ] **Step 1: Write failing tests**

Add tests inside `mod tests`:

```rust
#[test]
fn browser_decode_values_download_exports_only_decoded_values() {
    let state = BrowserFlowState {
        input_mode: InputMode::VaultDecode,
        imported_file_name: Some("Clinic Vault 2026.vault".to_string()),
        result_output: serde_json::json!({
            "decoded_values": {"patient-1": {"name": "Alice Example"}},
            "vault_path": "/phi/vault",
            "passphrase": "secret",
            "audit_event": {"kind": "decode"}
        })
        .to_string(),
        ..BrowserFlowState::default()
    };

    assert!(state.can_export_decoded_values());
    let payload = state
        .prepared_decoded_values_download_payload()
        .expect("decoded values payload");
    let json: serde_json::Value = serde_json::from_slice(&payload.bytes).expect("json");
    let text = String::from_utf8(payload.bytes).expect("utf8");

    assert_eq!(payload.file_name, "clinic-vault-2026-decoded-values.json");
    assert_eq!(payload.mime_type, "application/json;charset=utf-8");
    assert!(payload.is_text);
    assert_eq!(json["mode"], "vault_decode_values");
    assert_eq!(json["decoded_values"]["patient-1"]["name"], "Alice Example");
    assert!(json.get("audit_event").is_none());
    assert!(!text.contains("/phi/vault"));
    assert!(!text.contains("secret"));
}

#[test]
fn browser_decode_values_download_is_unavailable_without_decoded_values() {
    let mut state = BrowserFlowState {
        input_mode: InputMode::VaultDecode,
        result_output: serde_json::json!({"decoded_count": 0}).to_string(),
        ..BrowserFlowState::default()
    };

    assert!(!state.can_export_decoded_values());
    assert_eq!(
        state.prepared_decoded_values_download_payload().unwrap_err(),
        "Decoded values download is only available after a successful vault decode response with decoded values."
    );

    state.input_mode = InputMode::VaultAuditEvents;
    state.result_output = serde_json::json!({
        "decoded_values": {"patient-1": {"name": "Alice Example"}}
    })
    .to_string();

    assert!(!state.can_export_decoded_values());
}
```

- [ ] **Step 2: Run tests to verify RED**

Run: `cargo test -p mdid-browser decode_values_download -- --nocapture`

Expected: FAIL because `can_export_decoded_values` and `prepared_decoded_values_download_payload` do not exist.

- [ ] **Step 3: Implement minimal helper code**

In `impl BrowserFlowState`, add:

```rust
fn suggested_decoded_values_file_name(&self) -> String {
    if let Some(imported_file_name) = &self.imported_file_name {
        let stem = sanitized_import_stem(imported_file_name);
        return format!("{stem}-decoded-values.json");
    }

    "mdid-browser-decoded-values.json".to_string()
}

fn decoded_values_payload(&self) -> Result<serde_json::Value, String> {
    if self.input_mode != InputMode::VaultDecode {
        return Err("Decoded values download is only available after a successful vault decode response with decoded values.".to_string());
    }

    let output: serde_json::Value = serde_json::from_str(&self.result_output).map_err(|_| {
        "Decoded values download is only available after a successful vault decode response with decoded values.".to_string()
    })?;
    let decoded_values = output
        .get("decoded_values")
        .filter(|value| value.is_object())
        .cloned()
        .ok_or_else(|| "Decoded values download is only available after a successful vault decode response with decoded values.".to_string())?;

    Ok(serde_json::json!({
        "mode": "vault_decode_values",
        "decoded_values": decoded_values,
    }))
}

fn can_export_decoded_values(&self) -> bool {
    self.decoded_values_payload().is_ok()
}

fn prepared_decoded_values_download_payload(&self) -> Result<BrowserDownloadPayload, String> {
    let bytes = serde_json::to_vec_pretty(&self.decoded_values_payload()?)
        .map_err(|_| "Browser output download could not encode decoded values JSON.".to_string())?;

    Ok(BrowserDownloadPayload {
        file_name: self.suggested_decoded_values_file_name(),
        mime_type: "application/json;charset=utf-8",
        bytes,
        is_text: true,
    })
}
```

- [ ] **Step 4: Run tests to verify GREEN**

Run: `cargo test -p mdid-browser decode_values_download -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Run broader browser tests and commit**

Run: `cargo test -p mdid-browser --lib`
Expected: PASS.

Commit:

```bash
git add crates/mdid-browser/src/app.rs
git commit -m "feat(browser): export decoded vault values"
```

### Task 2: Browser UI action for decoded-values download

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`

- [ ] **Step 1: Write failing UI-state tests**

Add tests inside `mod tests`:

```rust
#[test]
fn decoded_values_download_filename_falls_back_without_source_file() {
    let state = BrowserFlowState {
        input_mode: InputMode::VaultDecode,
        result_output: serde_json::json!({
            "decoded_values": {"patient-1": {"name": "Alice Example"}}
        })
        .to_string(),
        ..BrowserFlowState::default()
    };

    let payload = state
        .prepared_decoded_values_download_payload()
        .expect("decoded values payload");

    assert_eq!(payload.file_name, "mdid-browser-decoded-values.json");
}
```

- [ ] **Step 2: Run test to verify RED or coverage gap**

Run: `cargo test -p mdid-browser decoded_values_download_filename -- --nocapture`
Expected: FAIL before Task 1 implementation; PASS after helper implementation if Task 1 already supplied the behavior.

- [ ] **Step 3: Add UI action**

In `browser_flow_app`, add a second callback next to `on_export` that calls `prepared_decoded_values_download_payload()`, and render a separate button only for vault decode values:

```rust
let can_export_decoded_values = move || state.get().can_export_decoded_values();
let on_export_decoded_values = move |_| {
    let payload = state.with(|state| {
        if state.can_export_decoded_values() {
            Some(state.prepared_decoded_values_download_payload())
        } else {
            None
        }
    });
    if let Some(Ok(payload)) = payload {
        trigger_browser_download(payload);
    }
};
```

Render near the existing download button:

```rust
<Show when=move || state.get().input_mode == InputMode::VaultDecode>
    <p class="warning-copy">
        "Decoded values may contain PHI. Use this explicit download only for authorized re-identification and store the file locally according to policy."
    </p>
    <button disabled=move || !can_export_decoded_values() on:click=on_export_decoded_values type="button">
        "Download decoded values JSON"
    </button>
</Show>
```

- [ ] **Step 4: Run targeted tests**

Run: `cargo test -p mdid-browser decode_values_download -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Run broader verification and commit**

Run: `cargo test -p mdid-browser --lib`
Expected: PASS.
Run: `cargo fmt --check`
Expected: PASS.
Run: `git diff --check`
Expected: PASS.

Commit:

```bash
git add crates/mdid-browser/src/app.rs
git commit -m "feat(browser): add decoded values download action"
```

### Task 3: README truth-sync

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update completion snapshot**

Update the Current repository status snapshot to mention browser decoded-values JSON download as an explicit high-risk vault decode action. Set Browser/Web completion to 79%, Desktop app remains 72%, CLI remains 95%, Overall remains 93% unless later verification justifies a change.

- [ ] **Step 2: Run verification**

Run: `cargo test -p mdid-browser decode_values_download -- --nocapture`
Expected: PASS.
Run: `cargo test -p mdid-browser --lib`
Expected: PASS.
Run: `cargo fmt --check`
Expected: PASS.
Run: `git diff --check`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add README.md docs/superpowers/plans/2026-04-30-browser-decode-values-export.md
git commit -m "docs: truth-sync browser decode values export"
```

## Self-Review

- Spec coverage: This plan covers helper behavior, UI action, PHI-safe separation from existing response reports, source-derived filenames, tests, verification, and README completion maintenance.
- Placeholder scan: No TBD/TODO/implement-later placeholders are present.
- Type consistency: The plan consistently uses `BrowserFlowState`, `BrowserDownloadPayload`, `InputMode::VaultDecode`, `can_export_decoded_values`, and `prepared_decoded_values_download_payload`.
