# Desktop Media Review Report Download Implementation Plan

> **For implementation workers:** REQUIRED SUB-SKILL: Use subagent-driven-development (recommended) or executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a PHI-safe structured JSON download helper for already-rendered desktop conservative media metadata review responses.

**Architecture:** Keep the slice inside `mdid-desktop` helper/state code. Reuse the existing strict review-report sanitizer that is already verified for desktop PDF review reports, but expand the allowed report mode from PDF-only to bounded media metadata review responses. Update README completion truthfully after the landed helper and tests pass.

**Tech Stack:** Rust workspace, `mdid-desktop`, serde_json, cargo tests/clippy, README truth-sync.

---

## File Structure

- Modify: `crates/mdid-desktop/src/lib.rs`
  - Add tests for media review report download behavior.
  - Allow `DesktopWorkflowResponseState::review_report_download(...)` for `DesktopWorkflowMode::MediaMetadataJson` as well as `PdfBase64Review`.
  - Return a media-specific suggested filename while preserving PDF report behavior.
- Modify: `README.md`
  - Truth-sync completion snapshot after the feature lands.
  - Mention desktop PHI-safe media metadata review report JSON downloads without claiming media rewrite/export, OCR, visual redaction, or broader desktop workflow completion.

### Task 1: Desktop media review structured report helper

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`

- [x] **Step 1: Write the failing test**

Add this test near the existing desktop review report download tests in `crates/mdid-desktop/src/lib.rs`:

```rust
#[test]
fn media_review_report_download_exports_structured_json_without_metadata_phi() {
    let mut state = DesktopWorkflowResponseState::default();
    state.apply_success_json(
        DesktopWorkflowMode::MediaMetadataJson,
        serde_json::json!({
            "summary": {
                "artifact_count": 1,
                "metadata_entry_count": 2,
                "candidate_count": 1,
                "review_required_count": 1,
                "unsupported_payload_count": 0,
                "artifact_label": "Patient-Jane-Doe-face-photo.jpg",
                "free_text_note": "MRN-12345"
            },
            "review_queue": [{
                "kind": "conservative_media",
                "status": "review_required",
                "decision": "review",
                "phi_type": "name",
                "metadata_key": "PatientName",
                "artifact_label": "Patient-Jane-Doe-face-photo.jpg",
                "source_value": "Jane Doe MRN-12345"
            }],
            "rewritten_media_bytes_base64": null
        }),
    );

    let download = state
        .review_report_download(DesktopWorkflowMode::MediaMetadataJson)
        .expect("media metadata review response should produce structured report");

    assert_eq!(download.file_name, "desktop-media-review-report.json");
    let report: serde_json::Value = serde_json::from_str(&download.json).unwrap();
    assert_eq!(report["mode"], "media_metadata_json");
    assert_eq!(report["summary"]["artifact_count"], 1);
    assert_eq!(report["summary"]["candidate_count"], 1);
    assert_eq!(report["review_queue"][0]["kind"], "conservative_media");
    assert_eq!(report["review_queue"][0]["status"], "review_required");
    assert_eq!(report["review_queue"][0]["phi_type"], "name");

    let rendered = download.json;
    assert!(!rendered.contains("Jane Doe"));
    assert!(!rendered.contains("MRN-12345"));
    assert!(!rendered.contains("PatientName"));
    assert!(!rendered.contains("Patient-Jane-Doe"));
    assert!(!rendered.contains("rewritten_media_bytes"));
}
```

- [x] **Step 2: Run test to verify it fails**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-desktop media_review_report_download_exports_structured_json_without_metadata_phi -- --nocapture
```

Expected: FAIL because `review_report_download(DesktopWorkflowMode::MediaMetadataJson)` currently returns `None`.

- [x] **Step 3: Write minimal implementation**

Update `DesktopWorkflowResponseState::review_report_download(...)` in `crates/mdid-desktop/src/lib.rs` so it accepts only PDF review and media metadata review modes:

```rust
    pub fn review_report_download(
        &self,
        mode: DesktopWorkflowMode,
    ) -> Option<DesktopWorkflowReviewReportDownload> {
        if !matches!(
            mode,
            DesktopWorkflowMode::PdfBase64Review | DesktopWorkflowMode::MediaMetadataJson
        ) || self.last_success_mode != Some(mode)
            || self.error.is_some()
        {
            return None;
        }
        let response = self.last_success_response.as_ref()?;
        let report = serde_json::json!({
            "mode": mode.as_report_mode(),
            "summary": sanitize_review_report_summary(response.get("summary")),
            "review_queue": sanitize_review_report_queue(response.get("review_queue")),
        });
        let json = serde_json::to_string_pretty(&report).ok()?;
        Some(DesktopWorkflowReviewReportDownload {
            file_name: match mode {
                DesktopWorkflowMode::PdfBase64Review => "desktop-pdf-review-report.json",
                DesktopWorkflowMode::MediaMetadataJson => "desktop-media-review-report.json",
                DesktopWorkflowMode::CsvText
                | DesktopWorkflowMode::XlsxBase64
                | DesktopWorkflowMode::DicomBase64 => return None,
            },
            json,
        })
    }
```

- [x] **Step 4: Run test to verify it passes**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-desktop media_review_report_download_exports_structured_json_without_metadata_phi -- --nocapture
```

Expected: PASS.

- [x] **Step 5: Run regression tests**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-desktop review_report_download -- --nocapture
cargo test -p mdid-desktop media_metadata -- --nocapture
cargo test -p mdid-desktop
cargo clippy -p mdid-desktop --all-targets -- -D warnings
git diff --check
```

Expected: all pass and diff check is clean.

- [x] **Step 6: Commit**

Run:

```bash
git add crates/mdid-desktop/src/lib.rs docs/superpowers/plans/2026-04-30-desktop-media-review-report-download.md
git commit -m "feat(desktop): add media review report downloads"
```

### Task 2: README completion truth-sync

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/plans/2026-04-30-desktop-media-review-report-download.md`

- [ ] **Step 1: Update README completion snapshot**

Change the snapshot wording to say it is truth-synced after desktop PHI-safe structured media review JSON downloads landed and were verified. Keep CLI at 95%, Browser/web at 73%, raise Desktop app from 64% to 66%, and raise Overall from 92% to 93%. This is modest because it is a helper-layer export for already-rendered responses, not full desktop workflow completion.

Update the Desktop app row to mention PHI-safe helper-layer structured JSON downloads for both PDF review reports and media metadata review reports. Keep explicit limitations: no full desktop file picker/save workflow, no PDF/media rewrite/export, no OCR/visual redaction, no vault browsing, no decoded-value display, no auth/session, and no unrelated platform behavior.

Update the Overall row to include desktop media review report JSON downloads and keep >=95% blockers explicit.

- [ ] **Step 2: Verify README scope and completion text**

Run:

```bash
grep -n "Completion snapshot\|CLI | 95%\|Browser/web | 73%\|Desktop app | 66%\|Overall | 93%\|media review report" README.md
grep -nE "controller|agent|orchestration|planner|coder|reviewer|moat" README.md || true
git diff --check
```

Expected: completion rows show the updated truthful numbers and media report wording; any forbidden-scope grep hits are historical scope-drift warnings only or absent.

- [ ] **Step 3: Commit docs**

Run:

```bash
git add README.md docs/superpowers/plans/2026-04-30-desktop-media-review-report-download.md
git commit -m "docs: truth-sync desktop media report completion"
```

## Self-Review

- Spec coverage: Task 1 lands the bounded desktop media review report helper with PHI-safe tests. Task 2 updates README completion for CLI, browser/web, desktop app, overall, and remaining blockers.
- Placeholder scan: No TBD/TODO/fill-in placeholders remain.
- Type consistency: The plan uses existing `DesktopWorkflowMode::MediaMetadataJson`, `DesktopWorkflowResponseState::review_report_download`, and `DesktopWorkflowReviewReportDownload` names consistently.
