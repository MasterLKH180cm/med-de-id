# Desktop Conservative Media Review Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded desktop media metadata review mode that prepares and submits metadata-only JSON to the existing local conservative media runtime route without adding OCR, media-byte upload, rewrite/export, or orchestration semantics.

**Architecture:** Extend the existing focused `mdid-desktop` workflow mode enum and request builder so media metadata JSON behaves like the already-landed browser/runtime media review slice. Reuse the existing desktop response workbench and add tests that prove route selection, validation, file import, copy, and PHI-safe response rendering stay bounded.

**Tech Stack:** Rust workspace, `mdid-desktop` crate, Leptos desktop UI, `cargo test -p mdid-desktop`.

---

## File Structure

- Modify: `crates/mdid-desktop/src/lib.rs`
  - Add `DesktopWorkflowMode::MediaMetadataJson`.
  - Include it in `DesktopWorkflowMode::ALL`.
  - Add label, payload hint, disclosure, and route `/media/conservative/deidentify`.
  - Make `.json` file imports load UTF-8 text into media metadata mode with a source name.
  - Make request building validate that media metadata payloads are JSON objects and use the raw JSON object body, not field policies.
  - Make response parsing/rendering redact any returned `rewritten_media_bytes_base64` and render summary/review queue details only.
- Modify: `crates/mdid-desktop/src/main.rs`
  - Ensure the UI treats media metadata JSON like a source-named non-tabular payload, hiding tabular field policies and showing the bounded disclosure.
- Modify: `README.md`
  - Truth-sync completion table and status bullets for the new desktop media metadata review mode.

### Task 1: Desktop Media Metadata Request Mode

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`
- Modify: `crates/mdid-desktop/src/main.rs`
- Test: inline unit tests in `crates/mdid-desktop/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Add tests under the existing `#[cfg(test)]` module in `crates/mdid-desktop/src/lib.rs`:

```rust
#[test]
fn media_metadata_mode_uses_bounded_runtime_route_and_copy() {
    assert!(DesktopWorkflowMode::ALL.contains(&DesktopWorkflowMode::MediaMetadataJson));
    assert_eq!(DesktopWorkflowMode::MediaMetadataJson.label(), "Media metadata JSON");
    assert_eq!(DesktopWorkflowMode::MediaMetadataJson.route(), "/media/conservative/deidentify");
    assert!(DesktopWorkflowMode::MediaMetadataJson.payload_hint().contains("media metadata JSON"));
    assert!(DesktopWorkflowMode::MediaMetadataJson.disclosure().contains("metadata-only"));
    assert!(DesktopWorkflowMode::MediaMetadataJson.disclosure().contains("does not upload media bytes"));
    assert!(DesktopWorkflowMode::MediaMetadataJson.disclosure().contains("no OCR"));
}

#[test]
fn json_file_import_uses_media_metadata_mode_without_media_bytes() {
    let imported = DesktopFileImportPayload::from_bytes(
        "local-media-metadata.json",
        br#"{"artifact_label":"scan.png","format":"image","metadata":[{"key":"PatientName","value":"Jane Patient"}],"ocr_or_visual_review_required":true}"#,
    )
    .expect("json metadata imports should be accepted");

    assert_eq!(imported.mode, DesktopWorkflowMode::MediaMetadataJson);
    assert_eq!(imported.source_name.as_deref(), Some("local-media-metadata.json"));
    assert!(imported.payload.contains("PatientName"));
}

#[test]
fn media_metadata_request_uses_raw_json_body_and_rejects_non_objects() {
    let valid = DesktopWorkflowRequestState {
        mode: DesktopWorkflowMode::MediaMetadataJson,
        payload: r#"{"artifact_label":"scan.png","format":"image","metadata":[],"ocr_or_visual_review_required":false}"#.to_string(),
        field_policy_json: r#"{"PatientName":"redact"}"#.to_string(),
        source_name: "local-media-metadata.json".to_string(),
    };

    let request = valid.try_build_request().expect("valid metadata object should build");
    assert_eq!(request.endpoint, "/media/conservative/deidentify");
    assert!(request.body.contains(r#""artifact_label":"scan.png""#));
    assert!(!request.body.contains("field_policies"));

    let invalid = DesktopWorkflowRequestState {
        mode: DesktopWorkflowMode::MediaMetadataJson,
        payload: "[]".to_string(),
        field_policy_json: "{}".to_string(),
        source_name: "local-media-metadata.json".to_string(),
    };

    assert_eq!(
        invalid.try_build_request(),
        Err(DesktopWorkflowValidationError::InvalidMediaMetadataJson)
    );
}
```

- [ ] **Step 2: Run tests to verify RED**

Run: `cargo test -p mdid-desktop media_metadata -- --nocapture`

Expected: FAIL because `DesktopWorkflowMode::MediaMetadataJson` and `InvalidMediaMetadataJson` do not exist yet.

- [ ] **Step 3: Implement minimal code**

Update `DesktopWorkflowMode` with the new mode, copy, and route. Update JSON file import to use UTF-8 text and preserve source name. Update `DesktopWorkflowRequestState::try_build_request` so media metadata mode trims and validates payload with `serde_json::from_str::<serde_json::Value>`, requires `Value::Object`, and uses the original payload as `body`. Add `DesktopWorkflowValidationError::InvalidMediaMetadataJson` and display text `Media metadata JSON must be a JSON object accepted by the local media review runtime route.` In `main.rs`, hide tabular field policy inputs for media metadata mode and show source name input like PDF/DICOM.

- [ ] **Step 4: Run tests to verify GREEN**

Run: `cargo test -p mdid-desktop media_metadata -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Run broader desktop tests**

Run: `cargo test -p mdid-desktop`

Expected: PASS.

- [ ] **Step 6: Commit**

Run:

```bash
git add crates/mdid-desktop/src/lib.rs crates/mdid-desktop/src/main.rs
git commit -m "feat(desktop): add bounded media metadata review mode"
```

### Task 2: README Completion Truth-Sync

**Files:**
- Modify: `README.md`
- Test: documentation plus controller-visible verification evidence

- [ ] **Step 1: Inspect current status**

Run: `git status --short && git log --oneline -5 && cargo test -p mdid-desktop`

Expected: clean or only planned README changes after edit; tests PASS.

- [ ] **Step 2: Update completion snapshot**

Update the README completion table to reflect that desktop now includes bounded media metadata JSON review preparation/submission/PHI-safe response rendering. Raise Desktop app from 38% to 41% and Overall from 70% to 71% only if Task 1 is landed and `cargo test -p mdid-desktop` passes. Keep Browser/web at 42% and CLI at 84% unless new landed facts justify a change. Keep missing items honest: media remains metadata-only with no OCR, no visual redaction, no media-byte upload/rewrite/export.

- [ ] **Step 3: Verify docs mention no orchestration drift**

Run: `grep -n "agent_id\|claim\|complete_command\|controller-step\|planner-coder-reviewer" README.md || true`

Expected: no new product-roadmap claims using those forbidden orchestration terms.

- [ ] **Step 4: Commit**

Run:

```bash
git add README.md docs/superpowers/plans/2026-04-29-desktop-conservative-media-review.md
git commit -m "docs: truth-sync desktop media review completion"
```

## Self-Review

- Spec coverage: The plan covers desktop workflow mode exposure, file import, runtime request body, bounded disclosures, no OCR/media rewrite claims, response safety through existing workbench behavior, tests, README truth-sync, and commit points.
- Placeholder scan: No TBD/TODO/fill-later placeholders remain.
- Type consistency: The plan consistently uses `DesktopWorkflowMode::MediaMetadataJson`, `/media/conservative/deidentify`, and `DesktopWorkflowValidationError::InvalidMediaMetadataJson`.
