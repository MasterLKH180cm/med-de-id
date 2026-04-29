# Browser Portable Artifact File Detection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the browser file import helper recognize explicit portable artifact JSON filenames so exported artifacts can be re-opened for inspect/import flows without being misclassified as media metadata JSON.

**Architecture:** Keep the slice narrow in `mdid-browser`: filename-to-mode inference remains a pure helper on `InputMode`, and portable artifact files are still text-only JSON payloads submitted to existing localhost runtime routes. The change must not add vault browsing, decoded-value rendering, controller/agent semantics, auth/session, generalized transfer orchestration, or media/PDF/DICOM rewrite behavior.

**Tech Stack:** Rust, Leptos browser crate, `cargo test -p mdid-browser`, `cargo clippy -p mdid-browser --all-targets -- -D warnings`.

---

## File Structure

- Modify: `crates/mdid-browser/src/app.rs`
  - Responsibility: browser UI state helpers and pure request/response helpers. This task only touches `InputMode::from_file_name` tests and the filename inference implementation.
- Modify: `README.md`
  - Responsibility: truthful completion/status snapshot. Update browser/web and overall status text only if the landed behavior changes the snapshot; otherwise explicitly state the completion percentage remains unchanged.

### Task 1: Portable artifact filename inference

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`
- Test: existing `#[cfg(test)]` module in `crates/mdid-browser/src/app.rs`

- [ ] **Step 1: Write the failing test**

Add these test functions inside the existing `#[cfg(test)] mod tests` in `crates/mdid-browser/src/app.rs`:

```rust
#[test]
fn portable_artifact_json_filenames_select_inspect_mode() {
    assert_eq!(
        InputMode::from_file_name("mdid-browser-portable-artifact.json"),
        Some(InputMode::PortableArtifactInspect)
    );
    assert_eq!(
        InputMode::from_file_name("clinic-export.MDID-PORTABLE.JSON"),
        Some(InputMode::PortableArtifactInspect)
    );
}

#[test]
fn ordinary_json_filenames_still_select_media_metadata_mode() {
    assert_eq!(
        InputMode::from_file_name("media-metadata.json"),
        Some(InputMode::MediaMetadataJson)
    );
    assert_eq!(
        InputMode::from_file_name("portable-not-artifact.json"),
        Some(InputMode::MediaMetadataJson)
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test -p mdid-browser portable_artifact_json_filenames_select_inspect_mode -- --nocapture
```

Expected: FAIL because `mdid-browser-portable-artifact.json` currently maps to `MediaMetadataJson` instead of `PortableArtifactInspect`.

- [ ] **Step 3: Write minimal implementation**

In `InputMode::from_file_name`, check explicit portable artifact JSON filename patterns before the generic `.json` fallback:

```rust
    fn from_file_name(file_name: &str) -> Option<Self> {
        let file_name = file_name.to_lowercase();

        if file_name.ends_with(".csv") {
            Some(Self::CsvText)
        } else if file_name.ends_with(".xlsx") {
            Some(Self::XlsxBase64)
        } else if file_name.ends_with(".pdf") {
            Some(Self::PdfBase64)
        } else if file_name.ends_with(".dcm") || file_name.ends_with(".dicom") {
            Some(Self::DicomBase64)
        } else if file_name.ends_with("mdid-browser-portable-artifact.json")
            || file_name.ends_with(".mdid-portable.json")
            || file_name.ends_with("-mdid-portable.json")
        {
            Some(Self::PortableArtifactInspect)
        } else if file_name.ends_with(".json") {
            Some(Self::MediaMetadataJson)
        } else {
            None
        }
    }
```

- [ ] **Step 4: Run targeted tests to verify pass**

Run:

```bash
cargo test -p mdid-browser portable_artifact_json_filenames_select_inspect_mode ordinary_json_filenames_still_select_media_metadata_mode -- --nocapture
```

If Cargo rejects multiple filters, run them separately:

```bash
cargo test -p mdid-browser portable_artifact_json_filenames_select_inspect_mode -- --nocapture
cargo test -p mdid-browser ordinary_json_filenames_still_select_media_metadata_mode -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Run broader browser verification**

Run:

```bash
cargo test -p mdid-browser
cargo clippy -p mdid-browser --all-targets -- -D warnings
```

Expected: both PASS with no warnings.

- [ ] **Step 6: Update README truthfully**

Update `README.md` current repository status to mention explicit portable artifact JSON filename detection in the browser/web row and current-feature bullets. Do not inflate completion numbers unless the repo-visible landed behavior justifies it; this small UX hardening is expected to keep Browser/web at 61% and Overall at 82%.

- [ ] **Step 7: Commit**

```bash
git add crates/mdid-browser/src/app.rs README.md docs/superpowers/plans/2026-04-29-browser-portable-artifact-file-detection.md
git commit -m "feat(browser): detect portable artifact json imports"
```
