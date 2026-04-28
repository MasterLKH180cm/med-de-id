# Desktop Portable Artifact Workbench Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded desktop request-preparation workbench for existing local portable artifact inspect/import and vault export runtime routes.

**Architecture:** Extend `mdid-desktop` only with pure request-state helpers and validation for the already-landed localhost runtime routes. The slice must not add controllers, agents, planners, claims, orchestration, or new runtime behavior; it only prepares explicit JSON request envelopes and redacts secrets in debug output.

**Tech Stack:** Rust workspace, `mdid-desktop` crate, unit tests with `cargo test -p mdid-desktop`.

---

## File Structure

- Modify: `crates/mdid-desktop/src/lib.rs`
  - Add `DesktopPortableMode`, `DesktopPortableRequestState`, `DesktopPortableValidationError`, route/disclosure helpers, and safe debug formatting.
  - Add tests in the existing `#[cfg(test)]` module near current vault workbench tests.
- Modify: `README.md`
  - Truth-sync desktop/browser/overall completion and missing-items wording after landed behavior is verified.

### Task 1: Desktop portable artifact request helpers

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`
- Test: `crates/mdid-desktop/src/lib.rs`

- [ ] **Step 1: Write failing tests**

Add these tests to the existing test module in `crates/mdid-desktop/src/lib.rs`:

```rust
#[test]
fn portable_mode_routes_match_existing_runtime_routes() {
    assert_eq!(DesktopPortableMode::VaultExport.route(), "/vault/export");
    assert_eq!(DesktopPortableMode::InspectArtifact.route(), "/portable-artifacts/inspect");
    assert_eq!(DesktopPortableMode::ImportArtifact.route(), "/portable-artifacts/import");
    assert!(DesktopPortableMode::VaultExport.disclosure().contains("bounded"));
    assert!(!DesktopPortableMode::VaultExport.disclosure().contains("controller"));
}

#[test]
fn portable_export_request_builds_runtime_envelope() {
    let state = DesktopPortableRequestState {
        mode: DesktopPortableMode::VaultExport,
        vault_path: "/safe/local.vault".to_string(),
        vault_passphrase: "vault-secret".to_string(),
        record_ids_json: "[\"record-1\",\"record-2\"]".to_string(),
        export_passphrase: "portable-secret".to_string(),
        export_context: "handoff to privacy office".to_string(),
        artifact_json: String::new(),
        portable_passphrase: String::new(),
        destination_vault_path: String::new(),
        destination_vault_passphrase: String::new(),
        import_context: String::new(),
    };

    let request = state.try_build_request().unwrap();

    assert_eq!(request.endpoint, "/vault/export");
    assert!(request.body_json.contains("\"vault_path\":\"/safe/local.vault\""));
    assert!(request.body_json.contains("\"vault_passphrase\":\"vault-secret\""));
    assert!(request.body_json.contains("\"record_ids\":[\"record-1\",\"record-2\"]"));
    assert!(request.body_json.contains("\"export_passphrase\":\"portable-secret\""));
    assert!(request.body_json.contains("\"export_context\":\"handoff to privacy office\""));
}

#[test]
fn portable_inspect_request_builds_runtime_envelope() {
    let mut state = DesktopPortableRequestState::default();
    state.mode = DesktopPortableMode::InspectArtifact;
    state.artifact_json = "{\"version\":1}".to_string();
    state.portable_passphrase = "portable-secret".to_string();

    let request = state.try_build_request().unwrap();

    assert_eq!(request.endpoint, "/portable-artifacts/inspect");
    assert_eq!(request.body_json, "{\"artifact\":{\"version\":1},\"portable_passphrase\":\"portable-secret\"}");
}

#[test]
fn portable_import_request_builds_runtime_envelope() {
    let mut state = DesktopPortableRequestState::default();
    state.mode = DesktopPortableMode::ImportArtifact;
    state.destination_vault_path = "/safe/target.vault".to_string();
    state.destination_vault_passphrase = "target-secret".to_string();
    state.artifact_json = "{\"version\":1}".to_string();
    state.portable_passphrase = "portable-secret".to_string();
    state.import_context = "restore approved records".to_string();

    let request = state.try_build_request().unwrap();

    assert_eq!(request.endpoint, "/portable-artifacts/import");
    assert!(request.body_json.contains("\"vault_path\":\"/safe/target.vault\""));
    assert!(request.body_json.contains("\"vault_passphrase\":\"target-secret\""));
    assert!(request.body_json.contains("\"artifact\":{\"version\":1}"));
    assert!(request.body_json.contains("\"portable_passphrase\":\"portable-secret\""));
    assert!(request.body_json.contains("\"import_context\":\"restore approved records\""));
}

#[test]
fn portable_request_validation_rejects_blank_required_fields() {
    let state = DesktopPortableRequestState::default();
    assert_eq!(state.try_build_request(), Err(DesktopPortableValidationError::BlankVaultPath));

    let mut inspect = DesktopPortableRequestState::default();
    inspect.mode = DesktopPortableMode::InspectArtifact;
    inspect.artifact_json = "{\"version\":1}".to_string();
    assert_eq!(inspect.try_build_request(), Err(DesktopPortableValidationError::BlankPortablePassphrase));

    let mut import = DesktopPortableRequestState::default();
    import.mode = DesktopPortableMode::ImportArtifact;
    import.destination_vault_path = "/safe/target.vault".to_string();
    import.destination_vault_passphrase = "target-secret".to_string();
    import.portable_passphrase = "portable-secret".to_string();
    assert_eq!(import.try_build_request(), Err(DesktopPortableValidationError::BlankArtifactJson));
}

#[test]
fn portable_request_debug_redacts_passphrases_and_artifact() {
    let state = DesktopPortableRequestState {
        vault_passphrase: "vault-secret".to_string(),
        export_passphrase: "portable-export-secret".to_string(),
        portable_passphrase: "portable-secret".to_string(),
        artifact_json: "{\"patient\":\"Alice\"}".to_string(),
        ..DesktopPortableRequestState::default()
    };

    let debug = format!("{state:?}");

    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("vault-secret"));
    assert!(!debug.contains("portable-export-secret"));
    assert!(!debug.contains("portable-secret"));
    assert!(!debug.contains("Alice"));
}
```

- [ ] **Step 2: Run tests to verify RED**

Run: `cargo test -p mdid-desktop portable_ -- --nocapture`
Expected: FAIL to compile with missing `DesktopPortableMode`, `DesktopPortableRequestState`, and `DesktopPortableValidationError`.

- [ ] **Step 3: Write minimal implementation**

Add focused desktop-only types in `crates/mdid-desktop/src/lib.rs`. Reuse the existing `DesktopWorkflowRequest { endpoint, body_json }` type and existing JSON escaping helper if present; otherwise add a tiny string escape helper for request-body strings.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopPortableMode {
    VaultExport,
    InspectArtifact,
    ImportArtifact,
}

impl DesktopPortableMode {
    pub fn route(self) -> &'static str {
        match self {
            Self::VaultExport => "/vault/export",
            Self::InspectArtifact => "/portable-artifacts/inspect",
            Self::ImportArtifact => "/portable-artifacts/import",
        }
    }

    pub fn disclosure(self) -> &'static str {
        match self {
            Self::VaultExport => "Bounded desktop portable export request preparation for the existing local /vault/export runtime route; no controller, agent, planner, or orchestration behavior is included.",
            Self::InspectArtifact => "Bounded desktop portable artifact inspection request preparation for the existing local /portable-artifacts/inspect runtime route; no controller, agent, planner, or orchestration behavior is included.",
            Self::ImportArtifact => "Bounded desktop portable artifact import request preparation for the existing local /portable-artifacts/import runtime route; no controller, agent, planner, or orchestration behavior is included.",
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct DesktopPortableRequestState {
    pub mode: DesktopPortableMode,
    pub vault_path: String,
    pub vault_passphrase: String,
    pub record_ids_json: String,
    pub export_passphrase: String,
    pub export_context: String,
    pub artifact_json: String,
    pub portable_passphrase: String,
    pub destination_vault_path: String,
    pub destination_vault_passphrase: String,
    pub import_context: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopPortableValidationError {
    BlankVaultPath,
    BlankVaultPassphrase,
    BlankRecordIdsJson,
    BlankExportPassphrase,
    BlankExportContext,
    BlankArtifactJson,
    BlankPortablePassphrase,
    BlankDestinationVaultPath,
    BlankDestinationVaultPassphrase,
    BlankImportContext,
}
```

Implement `Default`, redacted `Debug`, and `try_build_request()` so the tests pass exactly.

- [ ] **Step 4: Run targeted tests to verify GREEN**

Run: `cargo test -p mdid-desktop portable_ -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Run broader desktop tests**

Run: `cargo test -p mdid-desktop`
Expected: PASS.

- [ ] **Step 6: Commit**

Run:

```bash
git add crates/mdid-desktop/src/lib.rs
git commit -m "feat(desktop): add portable artifact request helpers"
```

### Task 2: README truth-sync

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update completion table and missing items**

Adjust README completion claims based only on landed behavior and verified tests:

```markdown
| Desktop app | 35% | Bounded sensitive-workstation foundation prepares CSV, XLSX, PDF review, DICOM, vault decode/audit, and portable artifact export/inspect/import request envelopes for existing localhost runtime routes, can apply bounded CSV/XLSX/PDF/DICOM file import/export helpers, submit prepared non-vault envelopes to a localhost runtime, and render response panes with honest disclosures; deeper desktop vault browsing, decode workflow execution UX, audit investigation workflow, portable transfer execution UX, OCR, visual redaction, and full review workflow remain missing. |
| Overall | 49% | Core workspace, vault MVP, tabular path, bounded DICOM/PDF/runtime slices, conservative media/FCS domain models, adapter/application review foundation, bounded runtime metadata review/PDF review/DICOM/vault decode/audit/portable export/import entries, browser tabular/PDF review surface with bounded CSV/XLSX/PDF import/export helpers, desktop request-preparation/localhost-submit/response workbench foundation with bounded CSV/XLSX/PDF/DICOM file import/export helpers and bounded desktop vault/portable request helpers are landed. |
```

- [ ] **Step 2: Run verification**

Run: `cargo test -p mdid-desktop`
Expected: PASS.

- [ ] **Step 3: Commit**

Run:

```bash
git add README.md docs/superpowers/plans/2026-04-29-desktop-portable-artifact-workbench.md
git commit -m "docs: truth-sync desktop portable workbench completion"
```

## Self-Review

- Spec coverage: The plan targets a high-leverage desktop missing item already backed by runtime routes: portable artifact export/inspect/import request preparation.
- Placeholder scan: No TBD/TODO/fill-later placeholders are present.
- Type consistency: `DesktopPortableMode`, `DesktopPortableRequestState`, and `DesktopPortableValidationError` names are consistent across tests and implementation steps.
