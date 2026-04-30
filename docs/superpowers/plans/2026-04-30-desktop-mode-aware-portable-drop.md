# Desktop Mode-Aware Portable Drop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make desktop dropped portable artifact JSON files populate the currently selected portable inspect/import request mode instead of always forcing inspect.

**Architecture:** Keep this bounded to the desktop helper/UI state layer. Reuse the existing `DesktopPortableFileImportPayload::from_bytes_for_mode` validation so only portable inspect/import modes accept `.mdid-portable.json`/browser portable artifact JSON files; do not change vault semantics, portable artifact contents, runtime routes, or expose artifact payloads in UI output.

**Tech Stack:** Rust workspace, `mdid-desktop`, unit tests in `crates/mdid-desktop/src/main.rs` and helper code in `crates/mdid-desktop/src/lib.rs`, Cargo test/clippy.

---

## File Structure

- Modify: `crates/mdid-desktop/src/main.rs`
  - Add a helper on `DesktopApp` that imports portable artifact JSON using `self.portable_request_state.mode` when the current portable mode is inspect or import.
  - Keep non-portable files on the existing workflow import path.
- Test: `crates/mdid-desktop/src/main.rs`
  - Add focused unit coverage that a portable artifact dropped while the desktop portable mode is import keeps `DesktopPortableMode::ImportArtifact` and fills the import artifact JSON/source name.
- Modify: `README.md`
  - Truth-sync the desktop/browser/CLI/overall completion snapshot and verification evidence after the landed behavior is verified.

---

### Task 1: Desktop portable drops honor the selected import mode

**Files:**
- Modify: `crates/mdid-desktop/src/main.rs`
- Test: `crates/mdid-desktop/src/main.rs`

- [ ] **Step 1: Write the failing test**

Add this test inside the existing `#[cfg(test)] mod tests` in `crates/mdid-desktop/src/main.rs`:

```rust
    #[test]
    fn app_imports_dropped_portable_artifact_into_selected_import_mode() {
        let mut app = DesktopApp::default();
        app.portable_request_state.mode = DesktopPortableMode::ImportArtifact;

        app.import_file_bytes_for_current_state(
            "Clinic Bundle.mdid-portable.json".to_string(),
            br#"{"records":[{"record_id":"patient-1"}]}"#,
        );

        assert_eq!(app.portable_request_state.mode, DesktopPortableMode::ImportArtifact);
        assert_eq!(
            app.portable_request_state.artifact_json,
            r#"{"records":[{"record_id":"patient-1"}]}"#
        );
        assert_eq!(
            app.portable_response_report_source_name.as_deref(),
            Some("Clinic Bundle.mdid-portable.json")
        );
        assert!(app.response_state.error.is_none());
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
~/.cargo/bin/cargo test -p mdid-desktop app_imports_dropped_portable_artifact_into_selected_import_mode -- --nocapture
```

Expected: FAIL because the helper does not exist yet or because the existing target import path always maps portable JSON drops to `InspectArtifact`.

- [ ] **Step 3: Write minimal implementation**

In `impl DesktopApp`, add this helper and update `import_dropped_files` to call it instead of directly calling `DesktopFileImportPayload::from_bytes_target`:

```rust
    fn import_file_bytes_for_current_state(&mut self, source_name: String, bytes: &[u8]) {
        let imported = if source_name.ends_with(".mdid-portable.json")
            || source_name == "mdid-browser-portable-artifact.json"
        {
            DesktopPortableFileImportPayload::from_bytes_for_mode(
                self.portable_request_state.mode,
                source_name,
                bytes,
            )
            .map(DesktopFileImportTarget::PortableArtifactInspect)
        } else {
            DesktopFileImportPayload::from_bytes_target(source_name, bytes)
        };

        match imported {
            Ok(imported) => self.apply_file_import_target(imported),
            Err(error) => self
                .response_state
                .apply_error(format!("file import failed: {error:?}")),
        }
    }
```

Then replace the `match DesktopFileImportPayload::from_bytes_target(source_name, &bytes)` block in `import_dropped_files` with:

```rust
            self.import_file_bytes_for_current_state(source_name, &bytes);
```

If path separators or case-insensitive matching are needed, extract a tiny private helper that uses only the basename and lowercases it before checking the portable artifact filename pattern.

- [ ] **Step 4: Run targeted verification**

Run:

```bash
~/.cargo/bin/cargo test -p mdid-desktop app_imports_dropped_portable_artifact_into_selected_import_mode -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Run relevant broader verification**

Run:

```bash
~/.cargo/bin/cargo test -p mdid-desktop portable -- --nocapture
~/.cargo/bin/cargo test -p mdid-desktop --bin mdid-desktop
~/.cargo/bin/cargo clippy -p mdid-desktop --all-targets -- -D warnings
~/.cargo/bin/cargo fmt --all -- --check
git diff --check
```

Expected: all PASS with no clippy warnings or whitespace errors.

- [ ] **Step 6: Commit**

```bash
git add crates/mdid-desktop/src/main.rs
git commit -m "fix(desktop): honor portable import mode on drop"
```

---

### Task 2: README completion truth-sync

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update completion snapshot text**

Update the `Current repository status` snapshot to mention the desktop mode-aware dropped portable artifact JSON handoff. Keep CLI, Browser/web, Desktop app, and Overall percentages honest; this is a bounded workflow-polish fix, so do not raise Overall above 93% unless controller-visible landed functionality and tests support a material completion change.

- [ ] **Step 2: Verify README boundaries**

Run:

```bash
grep -n "Overall | 93%" README.md
grep -n "portable artifact" README.md
git diff -- README.md
git diff --check
```

Expected: README mentions the desktop dropped portable artifact import-mode handoff, keeps the known missing items, and does not claim OCR, visual redaction, full PDF/media rewrite/export, generalized transfer workflow UX, auth/session, or agent/controller platform behavior.

- [ ] **Step 3: Commit README truth-sync**

```bash
git add README.md docs/superpowers/plans/2026-04-30-desktop-mode-aware-portable-drop.md
git commit -m "docs: truth-sync desktop portable drop handoff"
```

---

## Self-Review

- Spec coverage: Task 1 implements and verifies mode-aware desktop portable artifact JSON drops; Task 2 updates completion status and verification evidence.
- Placeholder scan: no unresolved placeholder markers remain.
- Type consistency: `import_file_bytes_for_current_state`, `DesktopPortableMode::ImportArtifact`, and `DesktopPortableFileImportPayload::from_bytes_for_mode` are existing or introduced consistently.
