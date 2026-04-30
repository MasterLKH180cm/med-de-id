# Browser DICOM Source-Name Download Filename Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Use a safe visible DICOM source name for browser DICOM rewritten-output download filenames when no imported browser file name exists.

**Architecture:** Keep the change inside the existing browser UI state filename suggestion helper. Reuse the existing safe filename helper pattern already used for PDF review reports, without adding any broad workflow behavior.

**Tech Stack:** Rust workspace, `mdid-browser` crate, `cargo test`, existing browser `BrowserFlowState` unit tests.

---

## File Structure

- Modify: `crates/mdid-browser/src/app.rs`
  - Responsibility: browser local-first UI state and download filename suggestion logic.
  - Add unit coverage for DICOM source-name fallback and minimal production branch in `BrowserFlowState::suggested_export_file_name`.
- Modify: `README.md`
  - Responsibility: truthful completion snapshot and verification evidence after the slice lands.

### Task 1: Browser DICOM source-name filename fallback

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`
- Test: `crates/mdid-browser/src/app.rs` existing `#[cfg(test)]` module

- [ ] **Step 1: Write the failing test**

Add this test to the existing browser `#[cfg(test)]` module near the other `suggested_export_file_name` tests:

```rust
#[test]
fn dicom_download_uses_safe_source_name_when_no_imported_file_exists() {
    let mut state = BrowserFlowState::default();
    state.input_mode = InputMode::DicomBase64;
    state.source_name = r"C:\incoming\CT Series 01.dcm".to_string();
    state.imported_file_name = None;

    assert_eq!(
        state.suggested_export_file_name(),
        "CT-Series-01-deidentified.dcm"
    );
}
```

- [ ] **Step 2: Run the targeted test to verify RED**

Run:

```bash
cargo test -p mdid-browser dicom_download_uses_safe_source_name_when_no_imported_file_exists -- --nocapture
```

Expected: FAIL with an assertion showing the current fallback is `mdid-browser-output.dcm` instead of `CT-Series-01-deidentified.dcm`.

- [ ] **Step 3: Write the minimal implementation**

In `BrowserFlowState::suggested_export_file_name`, after the imported-file-name match block and before the final `match self.input_mode`, add this DICOM-specific fallback next to the existing PDF source-name fallback:

```rust
fn sanitized_source_stem_preserving_case(file_name: &str) -> String {
    let file_name = file_name.rsplit(['/', '\\']).next().unwrap_or(file_name);
    let stem = file_name
        .rsplit_once('.')
        .map_or(file_name, |(stem, _)| stem);

    let mut sanitized = String::new();
    let mut needs_separator = false;
    for ch in stem.chars() {
        if ch.is_ascii_alphanumeric() {
            if needs_separator && !sanitized.is_empty() {
                sanitized.push('-');
            }
            sanitized.push(ch);
            needs_separator = false;
        } else {
            needs_separator = !sanitized.is_empty();
        }
    }

    if sanitized.len() > MAX_IMPORT_DERIVED_EXPORT_STEM_CHARS {
        sanitized.truncate(MAX_IMPORT_DERIVED_EXPORT_STEM_CHARS);
        while sanitized.ends_with('-') {
            sanitized.pop();
        }
    }

    if sanitized.is_empty() {
        "mdid-browser-output".to_string()
    } else {
        sanitized
    }
}

if self.input_mode == InputMode::DicomBase64 && !self.source_name.trim().is_empty() {
    let stem = sanitized_source_stem_preserving_case(&self.source_name);
    if stem != "mdid-browser-output" && stem != "local-review" {
        return format!("{stem}-deidentified.dcm");
    }
}
```

Keep the existing PDF fallback unchanged.

- [ ] **Step 4: Run the targeted test to verify GREEN**

Run:

```bash
cargo test -p mdid-browser dicom_download_uses_safe_source_name_when_no_imported_file_exists -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Run broader browser verification**

Run:

```bash
cargo test -p mdid-browser --lib
cargo fmt --check
git diff --check
```

Expected: all commands PASS with no formatting or whitespace errors.

- [ ] **Step 6: Commit**

Run:

```bash
git add crates/mdid-browser/src/app.rs
git commit -m "fix(browser): use DICOM source name for downloads"
```

Expected: one feature-branch commit containing only the browser code/test slice.

### Task 2: README truth-sync for browser DICOM source-name filename fallback

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update README completion snapshot and verification evidence**

Update the completion snapshot truth-sync date/wording to mention this browser DICOM filename slice. Keep percentages truthful: CLI remains 95%, browser/web remains 76%, desktop app remains 70%, and overall remains 93% because this is a bounded browser UX hardening slice, not a major remaining blocker such as OCR/visual redaction or full desktop workflow UX.

- [ ] **Step 2: Run docs verification**

Run:

```bash
git diff --check
```

Expected: PASS.

- [ ] **Step 3: Commit README update**

Run:

```bash
git add README.md docs/superpowers/plans/2026-04-30-browser-dicom-source-name-download.md
git commit -m "docs: truth-sync browser DICOM source filename"
```

Expected: one docs commit containing README and this plan.
