# Browser Conservative Media Review Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded browser mode for conservative image/video/FCS metadata review using the existing localhost runtime route.

**Architecture:** Extend the existing `mdid-browser` single-page bounded flow with one additional input mode that submits JSON metadata to the existing `/media/conservative/deidentify` route and renders the existing runtime summary/review queue. Keep the browser surface honest: metadata-only review, no OCR, no visual redaction, no media rewrite/export, no vault browsing, and no workflow/controller semantics.

**Tech Stack:** Rust, Leptos, serde/serde_json, existing mdid-runtime `/media/conservative/deidentify` contract, Cargo tests.

---

## File Structure

- Modify: `crates/mdid-browser/src/app.rs`
  - Add `InputMode::MediaMetadataJson`.
  - Add request/response structs for `/media/conservative/deidentify`.
  - Add validation/build/parse/render support for the new mode.
  - Add bounded UI copy, select option, accepted `.json` import support, and safe report export filename.
  - Add tests first for request building, response rendering, disclosure, import selection, and unsupported claims.
- Modify: `README.md`
  - Truth-sync browser/web and overall completion after the landed feature and tests.
  - Mention that browser media review is metadata-only and does not rewrite media.

### Task 1: Browser conservative media metadata review mode

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`
- Test: inline unit tests in `crates/mdid-browser/src/app.rs`

- [ ] **Step 1: Write failing tests**

Add tests in the existing `#[cfg(test)] mod tests`:

```rust
#[test]
fn media_metadata_mode_uses_json_text_and_bounded_runtime_route() {
    assert_eq!(InputMode::from_select_value("media-metadata-json"), InputMode::MediaMetadataJson);
    assert_eq!(InputMode::MediaMetadataJson.select_value(), "media-metadata-json");
    assert_eq!(InputMode::MediaMetadataJson.endpoint(), "/media/conservative/deidentify");
    assert_eq!(InputMode::MediaMetadataJson.browser_file_read_mode(), BrowserFileReadMode::Text);
    assert_eq!(InputMode::from_file_name("metadata.JSON"), Some(InputMode::MediaMetadataJson));
}

#[test]
fn media_metadata_mode_builds_runtime_request_without_field_policies() {
    let request = build_submit_request(
        InputMode::MediaMetadataJson,
        r#"{"artifact_label":"local-media-metadata.json","format":"image","metadata":[{"key":"PatientName","value":"Jane Patient"}],"ocr_or_visual_review_required":true}"#,
        "local-media-metadata.json",
        "",
    )
    .unwrap();

    assert_eq!(request.endpoint, "/media/conservative/deidentify");
    let body: serde_json::Value = serde_json::from_str(&request.body_json).unwrap();
    assert_eq!(body["media_type"], "image");
    assert_eq!(body["metadata"]["PatientName"], "Jane Patient");
    assert!(body.get("policies").is_none());
    assert!(body.get("field_policies").is_none());
}

#[test]
fn media_metadata_mode_rejects_non_object_payload() {
    let error = build_submit_request(InputMode::MediaMetadataJson, "[]", "metadata.json", "").unwrap_err();

    assert_eq!(error, "Media metadata JSON must be a JSON object accepted by the local media review runtime route.");
}

#[test]
fn parse_media_review_success_renders_phi_safe_summary_and_redacted_queue() {
    let response = parse_runtime_success(
        InputMode::MediaMetadataJson,
        &json!({
            "summary": {
                "media_type": "image",
                "metadata_fields": 3,
                "review_required_fields": 1,
                "unsupported_fields": 0,
                "ocr_required": true,
                "visual_redaction_required": true,
                "rewritten_media_bytes_base64": null
            },
            "review_queue": [
                {
                    "field_path": "metadata.PatientName",
                    "phi_type": "patient_name",
                    "decision": "needs_review",
                    "value": "Jane Patient"
                }
            ]
        })
        .to_string(),
    )
    .unwrap();

    assert_eq!(
        response.rewritten_output,
        "Media rewrite/export unavailable: runtime returned metadata-only conservative review."
    );
    assert!(response.summary.contains("media_type: image"));
    assert!(response.summary.contains("metadata_fields: 3"));
    assert!(response.summary.contains("ocr_required: true"));
    assert_eq!(
        response.review_queue,
        "- metadata.PatientName / patient_name / needs_review / value: <redacted>"
    );
    assert!(!response.review_queue.contains("Jane Patient"));
}

#[test]
fn media_metadata_mode_discloses_no_ocr_or_rewrite_claims() {
    let copy = InputMode::MediaMetadataJson.disclosure_copy().unwrap();

    assert!(copy.contains("metadata-only"));
    assert!(copy.contains("does not perform OCR"));
    assert!(copy.contains("visual redaction"));
    assert_eq!(
        BrowserFlowState { input_mode: InputMode::MediaMetadataJson, ..BrowserFlowState::default() }.suggested_export_file_name(),
        "mdid-browser-media-review-report.txt"
    );
}
```

- [ ] **Step 2: Run tests to verify RED**

Run: `cargo test -p mdid-browser media_metadata -- --nocapture`
Expected: FAIL because `InputMode::MediaMetadataJson` and media response parsing do not exist.

- [ ] **Step 3: Implement minimal browser media mode**

In `crates/mdid-browser/src/app.rs`:
- Add `MediaMetadataJson` to `InputMode`.
- Recognize `.json` file imports as text.
- Add select value `media-metadata-json`, label `Media metadata JSON`, payload hint `Paste media metadata JSON here`, endpoint `/media/conservative/deidentify`, bounded disclosure copy, no source name required, no field policy required.
- Add `MediaSubmitRequest = serde_json::Value` handling by parsing payload as a JSON object and serializing it unchanged.
- Add response structs:

```rust
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
struct MediaReviewSuccessResponse {
    summary: MediaReviewSummary,
    review_queue: Vec<MediaReviewCandidate>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
struct MediaReviewSummary {
    media_type: String,
    metadata_fields: usize,
    review_required_fields: usize,
    unsupported_fields: usize,
    ocr_required: bool,
    visual_redaction_required: bool,
    rewritten_media_bytes_base64: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
struct MediaReviewCandidate {
    field_path: String,
    phi_type: String,
    decision: String,
    #[allow(dead_code)]
    value: String,
}
```

Render summary as:

```text
media_type: image
metadata_fields: 3
review_required_fields: 1
unsupported_fields: 0
ocr_required: true
visual_redaction_required: true
rewritten_media_bytes_base64: null
```

Render review queue as redacted values only:

```text
- metadata.PatientName / patient_name / needs_review / value: <redacted>
```

Update UI option and import accept/copy to include metadata JSON without claiming media bytes upload or rewrite.

- [ ] **Step 4: Run targeted tests to verify GREEN**

Run: `cargo test -p mdid-browser media_metadata -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Run broader browser tests**

Run: `cargo test -p mdid-browser -- --nocapture`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/mdid-browser/src/app.rs
git commit -m "feat(browser): add conservative media metadata review mode"
```

### Task 2: README truth-sync for browser media review

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update README completion and scope text**

Change the completion table to:
- Browser/web: `42%`, mentioning bounded CSV/XLSX/PDF/DICOM plus metadata-only image/video/FCS review JSON mode.
- Overall: `70%`, mentioning the browser media metadata review mode as landed.

Update missing items to keep >90% gap honest: still missing OCR/visual redaction, media rewrite/export, deeper desktop vault browsing, full desktop decode execution UX, audit investigation polish, and richer browser UX.

- [ ] **Step 2: Run README grep verification**

Run: `grep -n "Browser/web\|Overall\|media metadata\|metadata-only" README.md`
Expected: Shows the updated browser/overall rows and metadata-only wording.

- [ ] **Step 3: Commit**

```bash
git add README.md docs/superpowers/plans/2026-04-29-browser-conservative-media-review.md
git commit -m "docs: truth-sync browser media review completion"
```
