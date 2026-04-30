# Browser Media Source-Aware Report Filenames Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make browser conservative media metadata review report downloads use a sanitized source-derived filename when no imported browser file name is available.

**Architecture:** Reuse the existing browser filename suggestion helper in `BrowserFlowState::suggested_export_file_name`. Keep the change bounded to the browser UI helper and tests; do not alter runtime media review semantics or add media rewrite/export behavior.

**Tech Stack:** Rust workspace, `mdid-browser`, Yew/browser state helper tests, Cargo test.

---

## File Structure

- Modify: `crates/mdid-browser/src/app.rs`
  - Add one focused unit test near the existing browser download filename tests.
  - Extend `BrowserFlowState::suggested_export_file_name` with a source-name fallback for `InputMode::MediaMetadataJson` when `imported_file_name` is absent and `source_name` sanitizes to a non-default stem.
- Modify: `README.md`
  - Truth-sync the current repository completion snapshot after this browser UX-depth slice lands.

### Task 1: Browser media report filename source fallback

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`
- Test: `crates/mdid-browser/src/app.rs`

- [ ] **Step 1: Write the failing test**

Add this test in the existing `#[cfg(test)]` module near other `suggested_export_file_name` tests:

```rust
#[test]
fn media_review_download_uses_safe_source_name_when_no_imported_file_exists() {
    let state = BrowserFlowState {
        input_mode: InputMode::MediaMetadataJson,
        source_name: "C:/incoming/Patient Face Photo.JPG".to_string(),
        imported_file_name: None,
        result_output: "metadata-only review".to_string(),
        summary: "Media review summary".to_string(),
        review_queue: "No review items returned.".to_string(),
        ..BrowserFlowState::default()
    };

    let payload = state.prepared_download_payload().expect("media report download payload");

    assert_eq!(payload.file_name, "Patient-Face-Photo-media-review-report.json");
    assert_eq!(payload.mime_type, "application/json;charset=utf-8");
    assert!(payload.is_text);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p mdid-browser --lib media_review_download_uses_safe_source_name_when_no_imported_file_exists -- --nocapture`

Expected: FAIL with assertion showing the current default `mdid-browser-media-review-report.json` instead of `Patient-Face-Photo-media-review-report.json`.

- [ ] **Step 3: Write minimal implementation**

In `BrowserFlowState::suggested_export_file_name`, after the existing PDF `source_name` fallback and before the final `match`, add:

```rust
        if self.input_mode == InputMode::MediaMetadataJson && !self.source_name.trim().is_empty() {
            let stem = sanitized_import_stem(&self.source_name);
            if stem != "mdid-browser-output" {
                return format!("{stem}-media-review-report.json");
            }
        }
```

Do not change runtime media review behavior. Do not add media rewrite/export.

- [ ] **Step 4: Run targeted and broader browser tests**

Run: `cargo test -p mdid-browser --lib media_review_download_uses_safe_source_name_when_no_imported_file_exists -- --nocapture`

Expected: PASS.

Run: `cargo test -p mdid-browser --lib`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-browser/src/app.rs docs/superpowers/plans/2026-04-30-browser-media-source-aware-report-filenames.md
git commit -m "feat(browser): use media source name for report downloads"
```

### Task 2: README completion truth-sync

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update README completion snapshot**

Update only the `Current repository status` snapshot text to mention the landed browser media source-aware report filename slice. Keep CLI at 95%, browser at 75%, desktop at 69%, and overall at 93% unless new functionality removes a larger blocker. Add verification evidence for the new commit and tests. State explicitly that the percentage is unchanged because this is bounded browser filename UX depth.

- [ ] **Step 2: Run verification**

Run: `cargo test -p mdid-browser --lib media_review_download_uses_safe_source_name_when_no_imported_file_exists -- --nocapture`

Expected: PASS.

Run: `git diff --check`

Expected: no whitespace errors.

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: truth-sync browser media source filenames"
```
