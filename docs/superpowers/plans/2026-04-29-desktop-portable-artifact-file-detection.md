# Desktop Portable Artifact File Detection Implementation Plan

> **For implementation workers:** REQUIRED SUB-SKILL: Use subagent-driven-development (recommended) or executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Teach the desktop file import helper to route explicit portable artifact JSON files into the portable inspect workbench instead of the conservative-media metadata workflow.

**Architecture:** Keep detection in the existing `DesktopFileImportPayload::from_bytes(...)` helper so the egui file-import path can stay thin. Add a new non-PHI-bearing import target enum that can represent either workflow payloads or portable artifact payloads without leaking artifact contents in `Debug`.

**Tech Stack:** Rust workspace, `mdid-desktop`, serde_json, cargo test/clippy.

---

## File Structure

- Modify: `crates/mdid-desktop/src/lib.rs`
  - Add `DesktopFileImportTarget` enum with redacted `Debug`.
  - Add `DesktopPortableFileImportPayload` struct with redacted `Debug`.
  - Add `DesktopFileImportPayload::from_bytes_target(...)` that routes known portable artifact JSON filenames to `DesktopFileImportTarget::PortableArtifactInspect`.
  - Keep existing `DesktopFileImportPayload::from_bytes(...)` for workflow-only callers and return `UnsupportedFileType` when a known portable artifact JSON is imported through that legacy workflow-only API.
  - Add tests proving portable artifact names route to inspect mode, generic JSON remains media metadata, and artifact contents are not exposed through debug output.
- Modify: `crates/mdid-desktop/src/main.rs`
  - Update desktop import handling to use `from_bytes_target(...)` and populate `portable_request_state.artifact_json` plus `DesktopPortableMode::InspectArtifact` for portable artifacts.
- Modify: `README.md`
  - Truth-sync the completion snapshot after the landed desktop file-detection slice. Desktop app should move from 50% to 52%, overall from 82% to 83%; CLI and Browser/web remain unchanged.

## Scope Guard

This slice is desktop-only file import routing. It must not add vault browsing, decoded-value display, generalized portable transfer workflow UX, auth/session, controller/agent/orchestration behavior, OCR, visual redaction, PDF rewrite/export, or media rewrite/export.

### Task 1: Desktop portable artifact import target

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`
- Test: `crates/mdid-desktop/src/lib.rs` module tests

- [x] **Step 1: Write failing tests**

Add these tests in the existing `#[cfg(test)] mod tests` in `crates/mdid-desktop/src/lib.rs` near the file import tests:

```rust
#[test]
fn portable_artifact_json_file_import_targets_inspect_mode() {
    let imported = DesktopFileImportPayload::from_bytes_target(
        "patient-123.mdid-portable.json",
        br#"{"version":1,"artifact":{"ciphertext":"secret-patient-ciphertext"}}"#,
    )
    .expect("portable artifact json imports should be accepted");

    match imported {
        DesktopFileImportTarget::PortableArtifactInspect(payload) => {
            assert_eq!(payload.mode, DesktopPortableMode::InspectArtifact);
            assert_eq!(payload.artifact_json, r#"{"version":1,"artifact":{"ciphertext":"secret-patient-ciphertext"}}"#);
            assert_eq!(payload.source_name, "patient-123.mdid-portable.json");
        }
        other => panic!("expected portable inspect import target, got {other:?}"),
    }
}

#[test]
fn exact_browser_portable_artifact_filename_targets_inspect_mode() {
    let imported = DesktopFileImportPayload::from_bytes_target(
        "mdid-browser-portable-artifact.json",
        br#"{"artifact":{"ciphertext":"secret"}}"#,
    )
    .expect("browser portable artifact export names should be accepted");

    assert!(matches!(
        imported,
        DesktopFileImportTarget::PortableArtifactInspect(_)
    ));
}

#[test]
fn generic_json_file_import_still_uses_media_metadata_mode() {
    let imported = DesktopFileImportPayload::from_bytes_target(
        "local-media-metadata.json",
        b"{\"artifact_label\":\"scan.png\",\"format\":\"image\",\"metadata\":[]}",
    )
    .expect("generic json metadata imports should still be accepted");

    match imported {
        DesktopFileImportTarget::Workflow(payload) => {
            assert_eq!(payload.mode, DesktopWorkflowMode::MediaMetadataJson);
            assert_eq!(payload.source_name.as_deref(), Some("local-media-metadata.json"));
        }
        other => panic!("expected workflow import target, got {other:?}"),
    }
}

#[test]
fn portable_artifact_file_import_debug_redacts_artifact_contents() {
    let imported = DesktopFileImportPayload::from_bytes_target(
        "patient-123-mrn-456-m did-portable.json".replace(" ", ""),
        br#"{"artifact":{"ciphertext":"secret-patient-ciphertext"}}"#,
    )
    .expect("portable artifact json imports should be accepted");

    let debug = format!("{imported:?}");

    assert!(debug.contains("PortableArtifactInspect"));
    assert!(!debug.contains("secret-patient-ciphertext"));
    assert!(!debug.contains("patient-123"));
    assert!(!debug.contains("mrn-456"));
}
```

- [x] **Step 2: Run tests to verify RED**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-desktop portable_artifact_json_file_import_targets_inspect_mode -- --nocapture`

Expected: FAIL because `DesktopFileImportPayload::from_bytes_target` and `DesktopFileImportTarget` do not exist yet.

- [x] **Step 3: Implement minimal import target support**

In `crates/mdid-desktop/src/lib.rs`, add:

```rust
#[derive(Clone, PartialEq, Eq)]
pub enum DesktopFileImportTarget {
    Workflow(DesktopFileImportPayload),
    PortableArtifactInspect(DesktopPortableFileImportPayload),
}

impl std::fmt::Debug for DesktopFileImportTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Workflow(payload) => f.debug_tuple("Workflow").field(payload).finish(),
            Self::PortableArtifactInspect(payload) => f
                .debug_tuple("PortableArtifactInspect")
                .field(payload)
                .finish(),
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct DesktopPortableFileImportPayload {
    pub mode: DesktopPortableMode,
    pub artifact_json: String,
    pub source_name: String,
}

impl std::fmt::Debug for DesktopPortableFileImportPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DesktopPortableFileImportPayload")
            .field("mode", &self.mode)
            .field("artifact_json", &"<redacted>")
            .field("source_name", &"<redacted>")
            .finish()
    }
}
```

Add helper logic:

```rust
fn is_portable_artifact_json_filename(source_name: &str) -> bool {
    let filename = source_name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(source_name)
        .to_ascii_lowercase();

    filename == "mdid-browser-portable-artifact.json"
        || filename.ends_with(".mdid-portable.json")
        || filename.ends_with("-mdid-portable.json")
}
```

Add `from_bytes_target(...)` before or inside the existing `impl DesktopFileImportPayload`:

```rust
pub fn from_bytes_target(
    source_name: impl Into<String>,
    bytes: &[u8],
) -> Result<DesktopFileImportTarget, DesktopFileImportError> {
    let source_name = source_name.into();
    if is_portable_artifact_json_filename(&source_name) {
        if bytes.len() > DESKTOP_FILE_IMPORT_MAX_BYTES {
            return Err(DesktopFileImportError::FileTooLarge);
        }
        let artifact_json = std::str::from_utf8(bytes)
            .map_err(|_| DesktopFileImportError::InvalidCsvUtf8)?
            .to_string();
        return Ok(DesktopFileImportTarget::PortableArtifactInspect(
            DesktopPortableFileImportPayload {
                mode: DesktopPortableMode::InspectArtifact,
                artifact_json,
                source_name,
            },
        ));
    }

    Self::from_bytes(source_name, bytes).map(DesktopFileImportTarget::Workflow)
}
```

Update `from_bytes(...)` so workflow-only callers reject known portable artifact filenames before generic JSON handling:

```rust
let source_name = source_name.into();
if is_portable_artifact_json_filename(&source_name) {
    return Err(DesktopFileImportError::UnsupportedFileType);
}
```

- [x] **Step 4: Run targeted tests to verify GREEN**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-desktop portable_artifact -- --nocapture`

Expected: PASS for the new portable artifact file import tests and existing portable request/response tests.

- [x] **Step 5: Commit Task 1**

Run:

```bash
git add crates/mdid-desktop/src/lib.rs docs/superpowers/plans/2026-04-29-desktop-portable-artifact-file-detection.md
git commit -m "feat(desktop): detect portable artifact json imports"
```

### Task 2: Wire desktop UI import handling and README truth-sync

**Files:**
- Modify: `crates/mdid-desktop/src/main.rs`
- Modify: `README.md`
- Test: `crates/mdid-desktop/src/main.rs` module tests where practical; otherwise library tests plus clippy verify compile-time wiring.

- [x] **Step 1: Write failing UI handoff test**

If `MdidDesktopApp` import handling is testable through an existing helper, add a test that imports `mdid-browser-portable-artifact.json` and asserts `portable_request_state.mode == DesktopPortableMode::InspectArtifact` and `portable_request_state.artifact_json` contains the artifact JSON while workflow payload remains unchanged. If no helper exists, add a small private method `apply_file_import_target(...)` and test it:

```rust
#[test]
fn app_file_import_target_populates_portable_artifact_inspect_state() {
    let mut app = MdidDesktopApp::default();
    app.apply_file_import_target(DesktopFileImportTarget::PortableArtifactInspect(
        DesktopPortableFileImportPayload {
            mode: DesktopPortableMode::InspectArtifact,
            artifact_json: r#"{"artifact":{"ciphertext":"secret"}}"#.to_string(),
            source_name: "mdid-browser-portable-artifact.json".to_string(),
        },
    ));

    assert_eq!(app.portable_request_state.mode, DesktopPortableMode::InspectArtifact);
    assert_eq!(
        app.portable_request_state.artifact_json,
        r#"{"artifact":{"ciphertext":"secret"}}"#
    );
    assert_eq!(app.request_state.mode, DesktopWorkflowMode::CsvText);
}
```

- [x] **Step 2: Run test to verify RED**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-desktop app_file_import_target_populates_portable_artifact_inspect_state -- --nocapture`

Expected: FAIL because `apply_file_import_target(...)` is not wired yet.

- [x] **Step 3: Implement minimal UI wiring**

In `crates/mdid-desktop/src/main.rs`, import `DesktopFileImportTarget` and `DesktopPortableFileImportPayload` if needed. Replace file import calls that use `DesktopFileImportPayload::from_bytes(...)` with `DesktopFileImportPayload::from_bytes_target(...)`.

Add this private method on `MdidDesktopApp`:

```rust
fn apply_file_import_target(&mut self, target: DesktopFileImportTarget) {
    match target {
        DesktopFileImportTarget::Workflow(payload) => {
            self.request_state.mode = payload.mode;
            self.request_state.payload = payload.payload;
            if let Some(source_name) = payload.source_name {
                self.request_state.source_name = source_name;
            }
        }
        DesktopFileImportTarget::PortableArtifactInspect(payload) => {
            self.portable_request_state.mode = payload.mode;
            self.portable_request_state.artifact_json = payload.artifact_json;
        }
    }
}
```

Update existing workflow import assignment to call the helper.

Update README snapshot:
- Change truth-sync sentence to mention desktop portable artifact JSON file detection.
- Keep CLI at 84%.
- Keep Browser/web at 61%.
- Change Desktop app to 52%.
- Change Overall to 83%.
- Add a missing-item note that desktop still lacks full portable transfer workflow UX and vault browsing.

- [x] **Step 4: Run targeted and broad verification**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-desktop portable_artifact -- --nocapture
cargo test -p mdid-desktop file_import -- --nocapture
cargo test -p mdid-desktop
cargo clippy -p mdid-desktop --all-targets -- -D warnings
git diff --check
grep -n "Completion snapshot\|CLI | 84%\|Browser/web | 61%\|Desktop app | 52%\|Overall | 83%\|portable artifact JSON" README.md
```

Expected: all cargo commands PASS, diff check PASS, grep shows updated README snapshot.

- [x] **Step 5: Commit Task 2**

Run:

```bash
git add crates/mdid-desktop/src/main.rs README.md docs/superpowers/plans/2026-04-29-desktop-portable-artifact-file-detection.md
git commit -m "docs: truth-sync desktop portable artifact import status"
```

## Self-Review

- Spec coverage: The plan covers desktop import routing, PHI-safe debug, UI handoff, and README completion truth-sync.
- Placeholder scan: No TBD/TODO/fill-in placeholders remain.
- Type consistency: `DesktopFileImportTarget`, `DesktopPortableFileImportPayload`, `DesktopPortableMode::InspectArtifact`, and `DesktopFileImportPayload::from_bytes_target(...)` are used consistently.
