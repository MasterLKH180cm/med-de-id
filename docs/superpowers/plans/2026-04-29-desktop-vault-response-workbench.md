# Desktop Vault Response Workbench Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add bounded desktop rendering for runtime-shaped vault decode, vault audit, and portable artifact responses without exposing PHI or claiming deeper vault workflow execution.

**Architecture:** Extend the existing `mdid-desktop` response-state helpers so desktop can render already-received localhost runtime responses for vault/portable helper modes. Keep this as UI/workbench rendering only: no vault browsing, no local vault mutation, no decode execution UX beyond displaying safe runtime envelopes, and no controller/agent/orchestration behavior.

**Tech Stack:** Rust workspace, `mdid-desktop`, `serde_json`, existing desktop egui app and tests.

---

## File Structure

- Modify: `crates/mdid-desktop/src/lib.rs`
  - Add `DesktopVaultResponseMode` for `VaultDecode`, `VaultAudit`, `VaultExport`, `InspectArtifact`, and `ImportArtifact` response rendering.
  - Add a PHI-safe `DesktopVaultResponseState` or equivalent helper that consumes `serde_json::Value` and exposes banner/error/summary/artifact notice fields.
  - Ensure raw decoded values, original values, passphrases, artifact JSON, and raw audit details are not copied into display fields.
- Modify: `crates/mdid-desktop/src/main.rs`
  - Wire response helper text into the existing desktop UI only if it can be done without a large refactor; otherwise keep this slice as library-backed response workbench foundation.
  - Update stale copy that says decode/audit are not implemented if the bounded response rendering foundation is now implemented.
- Test: `crates/mdid-desktop/src/lib.rs`
  - Add unit tests beside existing desktop response/request tests.
- Modify: `README.md`
  - Truth-sync desktop and overall completion after landed tests.

### Task 1: Desktop vault runtime response rendering helper

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`
- Test: `crates/mdid-desktop/src/lib.rs`

- [ ] **Step 1: Write failing tests**

Add tests to the existing `#[cfg(test)]` module in `crates/mdid-desktop/src/lib.rs`:

```rust
#[test]
fn vault_response_state_renders_decode_summary_without_decoded_values() {
    let mut state = DesktopVaultResponseState::default();
    let response = serde_json::json!({
        "decoded_value_count": 2,
        "report_path": "/tmp/patient-report.json",
        "audit_event": {"kind": "decode", "detail": "patient Alice decoded for oncology"},
        "decoded_values": [{"original_value": "Alice Smith", "token": "PHI-TOKEN-1"}]
    });

    state.apply_success(DesktopVaultResponseMode::VaultDecode, &response);

    assert!(state.banner.contains("bounded vault decode response"));
    assert!(state.summary.contains("decoded values: 2"));
    assert!(state.artifact_notice.contains("/tmp/patient-report.json"));
    let rendered = format!("{} {} {}", state.banner, state.summary, state.artifact_notice);
    assert!(!rendered.contains("Alice Smith"));
    assert!(!rendered.contains("patient Alice"));
    assert!(!rendered.contains("PHI-TOKEN-1"));
}

#[test]
fn vault_response_state_renders_audit_counts_without_raw_details() {
    let mut state = DesktopVaultResponseState::default();
    let response = serde_json::json!({
        "event_count": 200,
        "returned_event_count": 100,
        "events": [
            {"kind": "decode", "detail": "patient Bob release"},
            {"kind": "encode", "detail": "encoded patient Carol"}
        ]
    });

    state.apply_success(DesktopVaultResponseMode::VaultAudit, &response);

    assert!(state.banner.contains("bounded vault audit response"));
    assert!(state.summary.contains("events returned: 100 / 200"));
    let rendered = format!("{} {} {}", state.banner, state.summary, state.artifact_notice);
    assert!(!rendered.contains("patient Bob"));
    assert!(!rendered.contains("patient Carol"));
}

#[test]
fn vault_response_state_renders_portable_artifact_without_raw_artifact_json() {
    let mut state = DesktopVaultResponseState::default();
    let response = serde_json::json!({
        "artifact_path": "/tmp/portable-artifact.json",
        "record_count": 3,
        "artifact_json": {"records": [{"original_value": "MRN-123"}]},
        "imported_record_count": 3
    });

    state.apply_success(DesktopVaultResponseMode::VaultExport, &response);
    assert!(state.banner.contains("bounded portable artifact response"));
    assert!(state.summary.contains("records: 3"));
    assert!(state.artifact_notice.contains("/tmp/portable-artifact.json"));

    state.apply_success(DesktopVaultResponseMode::ImportArtifact, &response);
    assert!(state.summary.contains("imported records: 3"));

    let rendered = format!("{} {} {}", state.banner, state.summary, state.artifact_notice);
    assert!(!rendered.contains("MRN-123"));
}

#[test]
fn vault_response_state_records_error_without_stale_phi() {
    let mut state = DesktopVaultResponseState::default();
    let response = serde_json::json!({"decoded_value_count": 1, "report_path": "/tmp/safe.json", "decoded_values": [{"original_value": "Alice Smith"}]});
    state.apply_success(DesktopVaultResponseMode::VaultDecode, &response);

    state.apply_error(DesktopVaultResponseMode::VaultDecode, "runtime failed for patient Alice Smith");

    assert!(state.banner.contains("bounded vault decode response"));
    assert!(state.error.as_deref().unwrap_or_default().contains("runtime failed"));
    assert!(!state.error.as_deref().unwrap_or_default().contains("patient Alice Smith"));
    assert!(state.summary.is_empty());
    assert!(state.artifact_notice.is_empty());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-desktop vault_response_state -- --nocapture`

Expected: FAIL because `DesktopVaultResponseState` and `DesktopVaultResponseMode` do not exist yet.

- [ ] **Step 3: Write minimal implementation**

Add the public enum/helper in `crates/mdid-desktop/src/lib.rs` near the existing response-state code:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopVaultResponseMode {
    VaultDecode,
    VaultAudit,
    VaultExport,
    InspectArtifact,
    ImportArtifact,
}

impl DesktopVaultResponseMode {
    fn banner(self) -> &'static str {
        match self {
            Self::VaultDecode => "Bounded vault decode response rendering: displays only safe counts and report artifact path from the existing localhost runtime response.",
            Self::VaultAudit => "Bounded vault audit response rendering: displays only safe event counts from the existing localhost runtime response.",
            Self::VaultExport | Self::InspectArtifact | Self::ImportArtifact => "Bounded portable artifact response rendering: displays only safe counts and artifact notices from existing localhost runtime responses.",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopVaultResponseState {
    pub banner: String,
    pub error: Option<String>,
    pub summary: String,
    pub artifact_notice: String,
}

impl Default for DesktopVaultResponseState {
    fn default() -> Self {
        Self {
            banner: "Bounded vault/portable response rendering is idle; submit to a localhost runtime and display only PHI-safe envelope fields.".to_string(),
            error: None,
            summary: String::new(),
            artifact_notice: String::new(),
        }
    }
}

impl DesktopVaultResponseState {
    pub fn apply_success(&mut self, mode: DesktopVaultResponseMode, response: &serde_json::Value) {
        self.banner = mode.banner().to_string();
        self.error = None;
        self.summary = match mode {
            DesktopVaultResponseMode::VaultDecode => format!(
                "decoded values: {}",
                response.get("decoded_value_count").and_then(serde_json::Value::as_u64).unwrap_or(0)
            ),
            DesktopVaultResponseMode::VaultAudit => format!(
                "events returned: {} / {}",
                response.get("returned_event_count").and_then(serde_json::Value::as_u64).unwrap_or(0),
                response.get("event_count").and_then(serde_json::Value::as_u64).unwrap_or(0)
            ),
            DesktopVaultResponseMode::VaultExport | DesktopVaultResponseMode::InspectArtifact => format!(
                "records: {}",
                response.get("record_count").and_then(serde_json::Value::as_u64).unwrap_or(0)
            ),
            DesktopVaultResponseMode::ImportArtifact => format!(
                "imported records: {}",
                response.get("imported_record_count").and_then(serde_json::Value::as_u64).unwrap_or(0)
            ),
        };
        self.artifact_notice = response
            .get("report_path")
            .or_else(|| response.get("artifact_path"))
            .and_then(serde_json::Value::as_str)
            .map(|path| format!("local artifact: {path}"))
            .unwrap_or_else(|| "no local artifact path returned".to_string());
    }

    pub fn apply_error(&mut self, mode: DesktopVaultResponseMode, error: &str) {
        self.banner = mode.banner().to_string();
        self.error = Some(redact_desktop_runtime_error(error));
        self.summary.clear();
        self.artifact_notice.clear();
    }
}
```

If `redact_desktop_runtime_error` does not exist, add this conservative helper near other desktop response helpers:

```rust
fn redact_desktop_runtime_error(error: &str) -> String {
    if error.trim().is_empty() {
        "runtime failed".to_string()
    } else {
        "runtime failed; details redacted".to_string()
    }
}
```

- [ ] **Step 4: Run targeted test to verify it passes**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-desktop vault_response_state -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Run broader desktop checks**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-desktop && cargo clippy -p mdid-desktop --all-targets -- -D warnings && git diff --check`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/mdid-desktop/src/lib.rs docs/superpowers/plans/2026-04-29-desktop-vault-response-workbench.md
git commit -m "feat(desktop): render bounded vault responses"
```

### Task 2: Desktop UI copy and README truth-sync

**Files:**
- Modify: `crates/mdid-desktop/src/main.rs`
- Modify: `README.md`
- Test: `crates/mdid-desktop/src/lib.rs`

- [x] **Step 1: Write/update docs and copy tests if needed**

No new production behavior is required beyond honest copy. If adding tests, assert that desktop copy no longer says all decode/audit response rendering is missing while still saying deeper vault browsing/decode workflow execution/audit investigation are missing.

- [x] **Step 2: Update UI limitation copy**

Change the footer copy in `crates/mdid-desktop/src/main.rs` to:

```rust
ui.label(
    "Not implemented in this desktop slice: file picker upload/download UX beyond bounded helper import/export, vault browsing, full decode workflow execution UX, audit investigation, OCR, visual redaction, PDF rewrite/export, and full review workflows.",
);
```

- [x] **Step 3: Update README completion snapshot**

Update `README.md` completion table based on landed tests:

```markdown
| Desktop app | 38% | Bounded sensitive-workstation foundation prepares CSV, XLSX, PDF review, DICOM, bounded vault decode/audit, and portable artifact export/inspect/import request envelopes for existing localhost runtime routes, can apply bounded CSV/XLSX/PDF/DICOM file import/export helpers, submit prepared non-vault and portable helper envelopes to a localhost runtime, render response panes with honest disclosures, and now has PHI-safe vault/portable response rendering helpers for decode/audit/portable runtime envelopes; deeper desktop vault browsing, full decode workflow execution UX, audit investigation/execution workflow polish, generalized portable transfer workflow UX, desktop PDF flow beyond request preparation and bounded review/export helper naming, desktop DICOM flow beyond bounded request/response/import/export helpers, auth/session, and full governance/review queues remain missing. |
| Overall | 69% | Core workspace, vault MVP, tabular path, bounded DICOM/PDF/runtime slices, conservative media/FCS domain models, adapter/application review foundation, bounded runtime metadata review/PDF review/DICOM/vault decode/audit/portable export/import entries, browser tabular/PDF review/DICOM helper surface with bounded CSV/XLSX/PDF/DICOM import/export helpers, desktop request-preparation/localhost-submit/response workbench foundation with bounded CSV/XLSX/PDF/DICOM file import/export helpers and PHI-safe vault/portable response rendering helpers, and CLI automation for CSV/XLSX/DICOM/PDF/conservative-media/vault audit/decode/portable import/export are present; deeper detection/policy, richer browser/desktop workflows, OCR/visual redaction, full desktop vault/decode/audit execution UX, and governance polish remain. |
```

Keep CLI at 84% and Browser/web at 38% unless landed code changed those surfaces.

- [x] **Step 4: Verify README and scope-drift wording**

Run:

```bash
grep -nE 'CLI \||Browser/web|Desktop app|Overall|Missing items' README.md
grep -nE 'agent|controller|orchestration|planner|coder|reviewer|moat' README.md || true
git diff --check
```

Expected: completion rows show CLI 84%, Browser/web 38%, Desktop app 38%, Overall 69%; any scope-drift terms appear only as explicit negative limitations, not as roadmap/product claims.

- [x] **Step 5: Run tests**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-desktop && git diff --check`

Expected: PASS.

- [x] **Step 6: Commit**

```bash
git add crates/mdid-desktop/src/main.rs README.md docs/superpowers/plans/2026-04-29-desktop-vault-response-workbench.md
git commit -m "docs: truth-sync desktop vault response status"
```

## Self-Review

- Spec coverage: Adds bounded desktop vault/portable response rendering foundation and README completion truth-sync; does not claim full vault browsing, decode execution UX, audit investigation, OCR, PDF rewrite/export, or orchestration.
- Placeholder scan: No TBD/TODO/fill-in placeholders remain.
- Type consistency: `DesktopVaultResponseMode` and `DesktopVaultResponseState` names are used consistently in tests and implementation.
