# Browser Portable Import Drop Mode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Preserve the browser user's selected portable import mode when a recognized portable artifact JSON file is imported, instead of always switching to inspect mode.

**Architecture:** Add a tiny browser helper that resolves the effective import mode from the current UI mode plus filename-detected mode. The helper keeps existing filename detection conservative and only preserves `PortableArtifactImport` when the detected file is a portable artifact that otherwise defaults to inspect.

**Tech Stack:** Rust, Leptos browser crate (`mdid-browser`), cargo test, existing browser state helper tests.

---

## File Structure

- Modify: `crates/mdid-browser/src/app.rs`
  - Add `BrowserFlowState::mode_for_imported_file(&self, detected_mode: InputMode) -> InputMode`.
  - Update wasm32 `read_browser_import_file` to resolve the effective import mode at load time using current state and detected filename mode.
  - Add unit tests in the existing `#[cfg(test)]` module for preserving portable import mode and defaulting portable artifact imports to inspect outside import mode.
- Modify: `README.md`
  - Truth-sync the browser/web and overall completion snapshot with this landed bounded import-mode handoff. Browser can increase from 75% to 76%; overall remains 93% because the change is upload UX depth, not one of the larger >=95% blockers.
- Modify: `docs/superpowers/plans/2026-04-30-browser-portable-import-drop-mode.md`
  - Mark plan checkboxes/evidence complete after implementation and review.

### Task 1: Browser portable artifact import mode handoff

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`
- Test: `crates/mdid-browser/src/app.rs` existing unit-test module

- [x] **Step 1: Write the failing tests**

Add these tests to the existing browser test module in `crates/mdid-browser/src/app.rs`:

```rust
#[test]
fn imported_portable_artifact_preserves_selected_import_mode() {
    let mut state = BrowserFlowState {
        input_mode: InputMode::PortableArtifactImport,
        ..BrowserFlowState::default()
    };

    let detected_mode = InputMode::from_file_name("clinic-mapping.mdid-portable.json")
        .expect("portable artifact filename should be recognized");
    let resolved_mode = state.mode_for_imported_file(detected_mode);
    state.apply_imported_file(
        "clinic-mapping.mdid-portable.json",
        r#"{"version":1}"#,
        resolved_mode,
    );

    assert_eq!(state.input_mode, InputMode::PortableArtifactImport);
    assert_eq!(state.payload, r#"{"version":1}"#);
    assert_eq!(
        state.suggested_export_file_name(),
        "clinic-mapping-mdid-portable-portable-artifact-import.json"
    );
}

#[test]
fn imported_portable_artifact_defaults_to_inspect_outside_import_mode() {
    let mut state = BrowserFlowState {
        input_mode: InputMode::CsvText,
        ..BrowserFlowState::default()
    };

    let detected_mode = InputMode::from_file_name("clinic-mapping.mdid-portable.json")
        .expect("portable artifact filename should be recognized");
    let resolved_mode = state.mode_for_imported_file(detected_mode);
    state.apply_imported_file(
        "clinic-mapping.mdid-portable.json",
        r#"{"version":1}"#,
        resolved_mode,
    );

    assert_eq!(state.input_mode, InputMode::PortableArtifactInspect);
    assert_eq!(state.payload, r#"{"version":1}"#);
    assert_eq!(
        state.suggested_export_file_name(),
        "clinic-mapping-mdid-portable-portable-artifact-inspect.json"
    );
}
```

- [x] **Step 2: Run the tests to verify RED**

Run:

```bash
cargo test -p mdid-browser portable_artifact -- --nocapture
```

Expected: FAIL with a compile error like `no method named mode_for_imported_file found for struct BrowserFlowState`.

Evidence: `cargo test -p mdid-browser portable_artifact -- --nocapture` failed with `error[E0599]: no method named mode_for_imported_file found for struct BrowserFlowState` at the two new test call sites.

- [x] **Step 3: Implement the minimal browser helper and wasm import use**

Add this helper inside `impl BrowserFlowState` near `apply_imported_file`:

```rust
    #[cfg_attr(not(test), allow(dead_code))]
    fn mode_for_imported_file(&self, detected_mode: InputMode) -> InputMode {
        if self.input_mode == InputMode::PortableArtifactImport
            && detected_mode == InputMode::PortableArtifactInspect
        {
            InputMode::PortableArtifactImport
        } else {
            detected_mode
        }
    }
```

Then change the wasm32 file-reader `onload` success branch from:

```rust
        match payload {
            Some(payload) => load_state.update(|state| {
                state.apply_imported_file(&load_file_name, &payload, input_mode);
            }),
```

to:

```rust
        match payload {
            Some(payload) => load_state.update(|state| {
                let effective_mode = state.mode_for_imported_file(input_mode);
                state.apply_imported_file(&load_file_name, &payload, effective_mode);
            }),
```

- [x] **Step 4: Run targeted browser tests to verify GREEN**

Run:

```bash
cargo test -p mdid-browser portable_artifact -- --nocapture
```

Expected: PASS, including both new tests.

Evidence: `cargo test -p mdid-browser portable_artifact -- --nocapture` passed with 8 passed, including `imported_portable_artifact_preserves_selected_import_mode` and `imported_portable_artifact_defaults_to_inspect_outside_import_mode`.

- [x] **Step 5: Run broader browser tests and formatting check**

Run:

```bash
cargo test -p mdid-browser --lib
cargo fmt --check
git diff --check
```

Expected: all PASS, no formatting or whitespace errors.

Evidence: `cargo test -p mdid-browser --lib` passed with 89 passed; `cargo fmt --check` passed; `git diff --check` passed.

- [x] **Step 6: Commit**

```bash
git add crates/mdid-browser/src/app.rs docs/superpowers/plans/2026-04-30-browser-portable-import-drop-mode.md
git commit -m "fix(browser): preserve portable import mode on artifact drop"
```

### Task 2: README completion truth-sync

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/plans/2026-04-30-browser-portable-import-drop-mode.md`

- [x] **Step 1: Update README completion snapshot**

Update `README.md` Current repository status so it states:

- Browser/web is 76%.
- Browser/web now includes preserving selected portable import mode when a recognized portable artifact JSON file is imported, while still defaulting portable artifacts to inspect mode outside import mode.
- Overall remains 93% because this removes a bounded browser upload UX gap but does not remove larger >=95% blockers such as richer workflows, OCR/visual redaction, PDF/media rewrite/export, full vault browsing/execution UX, packaging/hardening, or deeper policy/detection.
- Verification evidence references the commit from Task 1 and the commands `cargo test -p mdid-browser portable_artifact -- --nocapture`, `cargo test -p mdid-browser --lib`, `cargo fmt --check`, and `git diff --check`.

Evidence: `README.md` Current repository status now shows Browser/web `76%`, Overall `93%`, the selected portable import mode preservation/default inspect wording, and verification evidence for `fe7e001` with `cargo test -p mdid-browser portable_artifact -- --nocapture`, `cargo test -p mdid-browser --lib`, `cargo fmt --check`, and `git diff --check`.

- [x] **Step 2: Verify README contains the truthful percentages**

Run:

```bash
grep -n "Completion snapshot\|Browser/web | 76%\|Overall | 93%\|portable import mode" README.md
```

Expected: matching lines show Browser/web `76%`, Overall `93%`, and the portable import mode wording.

Evidence: `grep -n "Completion snapshot\|Browser/web | 76%\|Overall | 93%\|portable import mode" README.md` matched the completion snapshot, Browser/web `76%`, Overall `93%`, and portable import mode evidence lines; `git diff --check` passed.

- [x] **Step 3: Commit docs truth-sync**

```bash
git add README.md docs/superpowers/plans/2026-04-30-browser-portable-import-drop-mode.md
git commit -m "docs: truth-sync browser portable import handoff"
```

## Self-Review

- Spec coverage: Task 1 preserves selected browser portable import mode for recognized portable artifact JSON and keeps default inspect behavior outside import mode. Task 2 updates README completion status and evidence.
- Placeholder scan: No TBD/TODO/fill-in placeholders remain; all commands and snippets are concrete.
- Type consistency: `BrowserFlowState::mode_for_imported_file`, `InputMode::PortableArtifactImport`, and `InputMode::PortableArtifactInspect` are named consistently across tests and implementation.
