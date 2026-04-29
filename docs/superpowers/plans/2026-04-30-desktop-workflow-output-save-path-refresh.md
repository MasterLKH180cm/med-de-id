# Desktop Workflow Output Save Path Refresh Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refresh the desktop rewritten workflow output save-path suggestion to the bounded PHI-safe filename of the latest already-received CSV/XLSX/DICOM runtime output.

**Architecture:** Keep the slice inside `crates/mdid-desktop/src/main.rs` app-shell state. Reuse `DesktopWorkflowResponseState::workflow_output_download(mode).file_name`; do not inspect raw response bodies, source paths, PHI, vault data, portable artifacts, or add file-picker behavior.

**Tech Stack:** Rust, mdid-desktop egui app shell, cargo tests/clippy.

---

## File structure

- Modify: `crates/mdid-desktop/src/main.rs`
  - Add a default workflow output save-path constant.
  - Track the last generated workflow output save-path suggestion so explicit user overrides are preserved.
  - Refresh the generated save-path suggestion after successful CSV/XLSX/DICOM workflow runtime responses.
  - Add unit tests around the helper and disconnected submission behavior.
- Modify: `README.md`
  - Truth-sync desktop/browser/CLI/overall completion and verification evidence after the landed slice.

### Task 1: Desktop workflow output save-path refresh

**Files:**
- Modify: `crates/mdid-desktop/src/main.rs`
- Test: `crates/mdid-desktop/src/main.rs`

- [x] **Step 1: Write failing tests**

Add tests in `crates/mdid-desktop/src/main.rs` test module:

```rust
#[test]
fn workflow_output_save_path_refreshes_default_after_csv_success() {
    let mut app = DesktopApp::default();
    app.request_state.mode = DesktopWorkflowMode::CsvText;
    app.response_state.apply_success_json(
        DesktopWorkflowMode::CsvText,
        serde_json::json!({"csv": "name\nTOKEN-1\n", "summary": {}}),
    );

    app.refresh_workflow_output_save_path();

    assert_eq!(app.workflow_output_save_path, "desktop-deidentified.csv");
    assert_eq!(
        app.generated_workflow_output_save_path.as_deref(),
        Some("desktop-deidentified.csv")
    );
}

#[test]
fn workflow_output_save_path_preserves_user_override_after_dicom_success() {
    let mut app = DesktopApp::default();
    app.workflow_output_save_path = "C:\\exports\\custom-output.dcm".to_string();
    app.request_state.mode = DesktopWorkflowMode::DicomBase64;
    app.response_state.apply_success_json(
        DesktopWorkflowMode::DicomBase64,
        serde_json::json!({
            "rewritten_dicom_bytes_base64": base64::engine::general_purpose::STANDARD.encode(b"dicom"),
            "summary": {}
        }),
    );

    app.refresh_workflow_output_save_path();

    assert_eq!(app.workflow_output_save_path, "C:\\exports\\custom-output.dcm");
    assert_eq!(app.generated_workflow_output_save_path, None);
}

#[test]
fn workflow_output_save_path_resets_generated_path_when_no_binary_output() {
    let mut app = DesktopApp::default();
    app.workflow_output_save_path = "desktop-deidentified.csv".to_string();
    app.generated_workflow_output_save_path = Some("desktop-deidentified.csv".to_string());
    app.request_state.mode = DesktopWorkflowMode::PdfBase64Review;
    app.response_state.apply_success_json(
        DesktopWorkflowMode::PdfBase64Review,
        serde_json::json!({"summary": {}, "page_statuses": [], "review_queue": []}),
    );

    app.refresh_workflow_output_save_path();

    assert_eq!(app.workflow_output_save_path, DEFAULT_WORKFLOW_OUTPUT_SAVE_PATH);
    assert_eq!(app.generated_workflow_output_save_path, None);
}
```

- [x] **Step 2: Run tests to verify RED**

Run: `cargo test -p mdid-desktop workflow_output_save_path_ -- --nocapture`

Expected: FAIL because `generated_workflow_output_save_path`, `refresh_workflow_output_save_path`, and `DEFAULT_WORKFLOW_OUTPUT_SAVE_PATH` do not exist yet.

- [x] **Step 3: Implement minimal app-shell helper**

In `crates/mdid-desktop/src/main.rs`, add:

```rust
const DEFAULT_WORKFLOW_OUTPUT_SAVE_PATH: &str = "desktop-deidentified-output.bin";

fn is_replaceable_workflow_output_save_path(path: &str, generated_path: Option<&str>) -> bool {
    let path = path.trim();
    path.is_empty()
        || path == DEFAULT_WORKFLOW_OUTPUT_SAVE_PATH
        || generated_path.is_some_and(|generated| path == generated.trim())
}
```

Add `generated_workflow_output_save_path: Option<String>` to `DesktopApp`, default it to `None`, replace the hard-coded default path with `DEFAULT_WORKFLOW_OUTPUT_SAVE_PATH.to_string()`, and add:

```rust
fn refresh_workflow_output_save_path(&mut self) {
    if !is_replaceable_workflow_output_save_path(
        &self.workflow_output_save_path,
        self.generated_workflow_output_save_path.as_deref(),
    ) {
        self.generated_workflow_output_save_path = None;
        return;
    }

    let next_path = self
        .response_state
        .workflow_output_download(self.request_state.mode)
        .map(|download| download.file_name.to_string())
        .unwrap_or_else(|| DEFAULT_WORKFLOW_OUTPUT_SAVE_PATH.to_string());
    let next_generated_path =
        (next_path != DEFAULT_WORKFLOW_OUTPUT_SAVE_PATH).then(|| next_path.clone());
    self.workflow_output_save_path = next_path;
    self.generated_workflow_output_save_path = next_generated_path;
}
```

Call `self.refresh_workflow_output_save_path();` immediately after `self.response_state.apply_success_json(workflow_mode, envelope);` in the workflow success branch.

- [x] **Step 4: Run targeted tests to verify GREEN**

Run: `cargo test -p mdid-desktop workflow_output_save_path_ -- --nocapture`

Expected: PASS.

- [x] **Step 5: Run broader desktop verification**

Run:

```bash
cargo test -p mdid-desktop --lib
cargo test -p mdid-desktop --bin mdid-desktop workflow_output_save_path_ -- --nocapture
cargo clippy -p mdid-desktop --all-targets -- -D warnings
git diff --check
```

Expected: all pass with no warnings or whitespace errors.

- [x] **Step 6: Commit**

```bash
git add crates/mdid-desktop/src/main.rs docs/superpowers/plans/2026-04-30-desktop-workflow-output-save-path-refresh.md
git commit -m "fix(desktop): refresh workflow output save paths"
```

### Task 2: README truth-sync

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update completion snapshot**

Update `README.md` current repository status to mention the desktop app-shell now refreshes generated rewritten workflow output save suggestions from the latest already-received CSV/XLSX/DICOM output filename while preserving explicit user overrides. Keep CLI and browser percentages unchanged. Increase desktop only if justified by landed behavior; overall may remain unchanged if the blocker list is still dominated by larger UX gaps.

- [ ] **Step 2: Verify docs diff**

Run: `git diff -- README.md`

Expected: README claims match landed code/tests and do not overclaim file-picker UX, PDF/media rewrite/export, OCR, raw decoded value display, vault browsing, or agent/controller behavior.

- [ ] **Step 3: Commit**

```bash
git add README.md docs/superpowers/plans/2026-04-30-desktop-workflow-output-save-path-refresh.md
git commit -m "docs: truth-sync desktop workflow save path refresh"
```
