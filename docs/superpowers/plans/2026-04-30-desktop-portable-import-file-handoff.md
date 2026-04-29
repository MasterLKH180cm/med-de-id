# Desktop Portable Import File Handoff Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded desktop helper that can load an encrypted portable artifact JSON into the portable import request state, not only the inspect request state.

**Architecture:** Keep this as a small `mdid-desktop` helper-layer improvement: a portable artifact JSON filename is still detected by the existing allowlist, UTF-8 and size checks remain unchanged, and the selected desktop portable mode determines whether the artifact JSON is applied to inspect or import. The feature must not add vault browsing, decoded-value display, transfer orchestration, background workflow behavior, or any agent/controller semantics.

**Tech Stack:** Rust workspace, `mdid-desktop` library tests, Cargo.

---

## File Structure

- Modify: `crates/mdid-desktop/src/lib.rs`
  - Add a helper on `DesktopPortableFileImportPayload` that accepts the current `DesktopPortableMode`, a source filename, and bytes, then returns a payload for either `InspectArtifact` or `ImportArtifact`.
  - Reuse the same filename allowlist, max-size check, UTF-8 decoding, redacted debug behavior, and error enum used by the existing file import target helper.
- Modify: `README.md`
  - Truth-sync the completion snapshot to mention the bounded desktop portable import file handoff and keep overall completion honest.

### Task 1: Desktop portable import file handoff helper

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`
- Test: `crates/mdid-desktop/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Add these tests to the existing `#[cfg(test)]` module in `crates/mdid-desktop/src/lib.rs`:

```rust
#[test]
fn desktop_portable_file_import_payload_supports_import_mode() {
    let payload = DesktopPortableFileImportPayload::from_bytes_for_mode(
        DesktopPortableMode::ImportArtifact,
        "handoff.mdid-portable.json",
        br#"{"version":1,"records":[]}"#,
    )
    .expect("portable import handoff should accept artifact json");

    assert_eq!(payload.mode, DesktopPortableMode::ImportArtifact);
    assert_eq!(payload.artifact_json, r#"{"version":1,"records":[]}"#);
    assert_eq!(payload.source_name, "handoff.mdid-portable.json");
}

#[test]
fn desktop_portable_file_import_payload_rejects_export_mode() {
    let error = DesktopPortableFileImportPayload::from_bytes_for_mode(
        DesktopPortableMode::VaultExport,
        "handoff.mdid-portable.json",
        br#"{"version":1}"#,
    )
    .expect_err("vault export is not an artifact-consuming mode");

    assert_eq!(error, DesktopFileImportError::UnsupportedFileType);
}
```

- [ ] **Step 2: Run tests to verify RED**

Run: `cargo test -p mdid-desktop desktop_portable_file_import_payload -- --nocapture`

Expected: FAIL because `DesktopPortableFileImportPayload::from_bytes_for_mode` is not defined.

- [ ] **Step 3: Implement the minimal helper**

Add this implementation near the existing `impl DesktopFileImportPayload` block:

```rust
impl DesktopPortableFileImportPayload {
    pub fn from_bytes_for_mode(
        mode: DesktopPortableMode,
        source_name: impl Into<String>,
        bytes: &[u8],
    ) -> Result<Self, DesktopFileImportError> {
        let source_name = source_name.into();
        if !matches!(mode, DesktopPortableMode::InspectArtifact | DesktopPortableMode::ImportArtifact) {
            return Err(DesktopFileImportError::UnsupportedFileType);
        }
        if !is_portable_artifact_json_filename(&source_name) {
            return Err(DesktopFileImportError::UnsupportedFileType);
        }
        if bytes.len() > DESKTOP_FILE_IMPORT_MAX_BYTES {
            return Err(DesktopFileImportError::FileTooLarge);
        }
        let artifact_json = std::str::from_utf8(bytes)
            .map_err(|_| DesktopFileImportError::InvalidCsvUtf8)?
            .to_string();

        Ok(Self {
            mode,
            artifact_json,
            source_name,
        })
    }
}
```

- [ ] **Step 4: Run targeted tests to verify GREEN**

Run: `cargo test -p mdid-desktop desktop_portable_file_import_payload -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Run broader desktop tests**

Run: `cargo test -p mdid-desktop --lib`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/mdid-desktop/src/lib.rs
git commit -m "feat(desktop): support portable import file handoff"
```

### Task 2: README completion truth-sync

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update README completion snapshot**

Update the current repository status snapshot to state that desktop now includes a bounded portable artifact JSON handoff helper for import as well as inspect. Keep CLI and browser numbers unchanged. Desktop may increase only modestly from 67% to 68%; overall remains 93% unless controller-visible verification justifies otherwise.

- [ ] **Step 2: Verify docs diff**

Run: `git diff -- README.md`

Expected: Diff mentions only the bounded desktop portable import file handoff and no unsupported claims.

- [ ] **Step 3: Commit**

```bash
git add README.md docs/superpowers/plans/2026-04-30-desktop-portable-import-file-handoff.md
git commit -m "docs: truth-sync desktop portable import handoff"
```

## Self-Review

Spec coverage: Task 1 adds the helper and tests for import-mode acceptance and export-mode rejection. Task 2 updates README completion truthfully.

Placeholder scan: No TBD/TODO/fill-in placeholders are present.

Type consistency: The plan consistently uses `DesktopPortableFileImportPayload::from_bytes_for_mode`, `DesktopPortableMode::{InspectArtifact, ImportArtifact, VaultExport}`, and `DesktopFileImportError::UnsupportedFileType`.
