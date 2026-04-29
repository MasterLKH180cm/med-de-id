# Browser JSON Review Downloads Implementation Plan

> **For implementation workers:** REQUIRED SUB-SKILL: Use subagent-driven-development or executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`/`- [x]`) syntax for tracking.

**Goal:** Add PHI-safe JSON downloads for browser review/metadata response modes so browser users can save structured review artifacts instead of text-only panes.

**Architecture:** Keep `mdid-browser` thin and local-first: reuse the already-parsed browser state (`result_output`, `summary`, and `review_queue`) and generate a sanitized JSON envelope at download time for review-only modes. Binary rewritten XLSX/DICOM downloads remain unchanged; CSV remains CSV text; vault export continues to save only the encrypted portable artifact object.

**Tech Stack:** Rust, Leptos browser app, serde_json, existing `mdid-browser` unit tests.

---

## File Structure

- Modify: `crates/mdid-browser/src/app.rs`
  - Add `BrowserFlowState::review_report_download_json()` helper that serializes only mode label, summary, review_queue, and rendered output text for safe review modes.
  - Change suggested filenames for PDF/media/portable inspect/import review downloads from `.txt` to `.json`.
  - Change `prepared_download_payload()` for PDF/media/vault audit/vault decode/portable inspect/import modes to emit JSON with `application/json;charset=utf-8`.
  - Preserve existing CSV text, XLSX binary, DICOM binary, and vault export artifact JSON behavior.
- Modify: `README.md`
  - Truth-sync completion snapshot and browser status after tests land.

### Task 1: Browser PHI-safe JSON review downloads

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`
- Test: existing unit test module in `crates/mdid-browser/src/app.rs`

- [x] **Step 1: Write the failing tests**

Add these tests near the existing browser output download tests in `crates/mdid-browser/src/app.rs`:

```rust
#[test]
fn pdf_review_download_exports_structured_json_report() {
    let mut state = BrowserFlowState {
        input_mode: InputMode::PdfBase64,
        result_output: "PDF rewrite/export unavailable: runtime returned review-only PDF analysis.".to_string(),
        summary: "total_pages: 1\nocr_required_pages: 0".to_string(),
        review_queue: "- page 1 / patient_name / confidence 20 / review: <redacted>".to_string(),
        ..BrowserFlowState::default()
    };
    state.imported_file_name = Some("Patient Doe.pdf".to_string());

    let payload = state.prepared_download_payload().expect("download payload");
    let json: serde_json::Value = serde_json::from_slice(&payload.bytes).expect("json report");

    assert_eq!(payload.file_name, "patient-doe-review-report.json");
    assert_eq!(payload.mime_type, "application/json;charset=utf-8");
    assert!(payload.is_text);
    assert_eq!(json["mode"], "PDF base64");
    assert_eq!(json["summary"], "total_pages: 1\nocr_required_pages: 0");
    assert!(json["output"].as_str().unwrap().contains("review-only PDF analysis"));
}

#[test]
fn portable_review_download_exports_json_without_raw_runtime_body() {
    let state = BrowserFlowState {
        input_mode: InputMode::PortableArtifactInspect,
        result_output: "Portable artifact contains 2 record(s). Artifact contents are hidden.".to_string(),
        summary: "2 portable record(s) available for import.".to_string(),
        review_queue: "Portable artifact inspection completed without rendering original values or tokens.".to_string(),
        ..BrowserFlowState::default()
    };

    let payload = state.prepared_download_payload().expect("download payload");
    let text = std::str::from_utf8(&payload.bytes).expect("utf8 json");
    let json: serde_json::Value = serde_json::from_str(text).expect("json report");

    assert_eq!(payload.file_name, "mdid-browser-portable-artifact-inspect.json");
    assert_eq!(payload.mime_type, "application/json;charset=utf-8");
    assert!(payload.is_text);
    assert_eq!(json["mode"], "Portable artifact inspect");
    assert!(!text.contains("artifact_json"));
    assert!(!text.contains("original_value"));
    assert!(!text.contains("token"));
}
```

- [x] **Step 2: Run tests to verify RED**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-browser pdf_review_download_exports_structured_json_report portable_review_download_exports_json_without_raw_runtime_body -- --nocapture
```

Expected: FAIL because review-mode downloads still use `.txt` text payloads and no structured JSON report helper exists. If Cargo rejects multiple test filters, run each test separately:

```bash
cargo test -p mdid-browser pdf_review_download_exports_structured_json_report -- --nocapture
cargo test -p mdid-browser portable_review_download_exports_json_without_raw_runtime_body -- --nocapture
```

- [x] **Step 3: Implement minimal code**

In `BrowserFlowState`, add:

```rust
fn review_report_download_json(&self) -> Result<Vec<u8>, String> {
    serde_json::to_vec_pretty(&serde_json::json!({
        "mode": self.input_mode.label(),
        "summary": self.summary,
        "review_queue": self.review_queue,
        "output": self.result_output,
    }))
    .map_err(|error| format!("Failed to prepare browser review report JSON: {error}"))
}
```

Update `suggested_export_file_name()` so PDF/media/portable review files end in `.json`:

```rust
InputMode::PdfBase64 => return format!("{stem}-review-report.json"),
InputMode::MediaMetadataJson => return format!("{stem}-media-review-report.json"),
...
InputMode::PdfBase64 => "mdid-browser-review-report.json",
InputMode::MediaMetadataJson => "mdid-browser-media-review-report.json",
InputMode::PortableArtifactInspect => "mdid-browser-portable-artifact-inspect.json",
InputMode::PortableArtifactImport => "mdid-browser-portable-artifact-import.json",
```

Update `prepared_download_payload()` with a match arm before the default text arm:

```rust
InputMode::PdfBase64
| InputMode::MediaMetadataJson
| InputMode::VaultAuditEvents
| InputMode::VaultDecode
| InputMode::PortableArtifactInspect
| InputMode::PortableArtifactImport => Ok(BrowserDownloadPayload {
    file_name,
    mime_type: "application/json;charset=utf-8",
    bytes: self.review_report_download_json()?,
    is_text: true,
}),
```

Do not include `InputMode::VaultExport` in this arm; it must keep exporting only the encrypted portable artifact JSON object already in `result_output`.

- [x] **Step 4: Run targeted tests to verify GREEN**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-browser pdf_review_download_exports_structured_json_report -- --nocapture
cargo test -p mdid-browser portable_review_download_exports_json_without_raw_runtime_body -- --nocapture
```

Expected: both PASS.

- [x] **Step 5: Run broader browser verification**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-browser output_download -- --nocapture
cargo test -p mdid-browser --lib
cargo clippy -p mdid-browser --all-targets -- -D warnings
git diff --check
```

Expected: all PASS, no warnings, no whitespace errors.

- [x] **Step 6: Commit**

```bash
git add crates/mdid-browser/src/app.rs docs/superpowers/plans/2026-04-29-browser-json-review-downloads.md
git commit -m "feat(browser): export structured review reports"
```

### Task 2: README completion truth-sync

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/plans/2026-04-29-browser-json-review-downloads.md`

- [x] **Step 1: Write the docs update**

Update `README.md` completion snapshot to mention structured JSON downloads for browser review/metadata response modes. Because this is useful browser download-depth polish but not full upload/download workflow completion, set:

```markdown
| CLI | 95% | unchanged |
| Browser/web | 72% | ... includes real binary XLSX/DICOM downloads and structured PHI-safe JSON downloads for browser review/metadata response modes ... |
| Desktop app | 62% | unchanged |
| Overall | 91% | ... browser helper surfaces including binary rewritten XLSX/DICOM output downloads and structured review report JSON downloads ... |
```

Also update verification evidence to list the exact commands run in Task 1.

- [x] **Step 2: Verify README truth-sync text**

Run:

```bash
grep -n "Completion snapshot\|Browser/web | 72%\|Overall | 91%\|structured" README.md
grep -nE "agent workflow|controller loop|planner-coder-reviewer|complete_command|agent_id|claim" README.md || true
git diff --check
```

Expected: completion snapshot lines are present; any forbidden-term hits are only negative limitation text, not roadmap claims; no whitespace errors.

- [x] **Step 3: Mark this plan completed**

Add a completion evidence section at the end of this plan with:

```markdown
## Completion Evidence

- Landed branch: `feature/browser-file-import-download-polish`
- Task 1 commit: `<commit>`
- Task 2 commit: `<commit>`
- Verification: `<commands and PASS results>`
```

- [x] **Step 4: Commit docs truth-sync**

```bash
git add README.md docs/superpowers/plans/2026-04-29-browser-json-review-downloads.md
git commit -m "docs: truth-sync browser review downloads completion"
```

## Self-Review

- Spec coverage: Task 1 adds structured downloads for browser review modes while preserving existing binary/text behavior. Task 2 updates README completion and missing-items narrative.
- Placeholder scan: no TBD/TODO/fill-in-later language remains.
- Type consistency: all code references use existing `BrowserFlowState`, `InputMode`, and `BrowserDownloadPayload` names.

## Completion Evidence

- Landed branch: `feature/browser-file-import-download-polish`
- Task 1 commit: `a762a02 feat(browser): export structured review reports`
- Quality fix commit: `57f1164 fix(browser): preserve safe review report text`
- Task 2 commit: `2158b65 docs: truth-sync browser review downloads completion`
- Verification: `cargo test -p mdid-browser pdf_review_download_exports_structured_json_report -- --nocapture` PASS; `cargo test -p mdid-browser portable_review_download_exports_json_without_raw_runtime_body -- --nocapture` PASS; `cargo test -p mdid-browser output_download -- --nocapture` PASS; `cargo test -p mdid-browser --lib` PASS; `cargo clippy -p mdid-browser --all-targets -- -D warnings` PASS; `git diff --check` PASS.
- Reviews: Task 1 spec review PASS after quality fix; Task 1 quality review APPROVED after removing the brittle scrubber and preserving already-safe rendered review text; Task 2 docs spec/quality review found stale plan evidence and README wording, fixed in follow-up commit.
