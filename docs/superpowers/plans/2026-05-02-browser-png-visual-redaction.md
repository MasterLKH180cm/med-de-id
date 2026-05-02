# Browser PNG Visual Redaction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add bounded Browser/Web execution and download support for explicit-bbox PNG visual redaction through the existing local `/visual-redaction/png` runtime endpoint.

**Architecture:** Extend the existing Browser PPM visual-redaction mode with a sibling PNG mode rather than introducing generic media workflow semantics. The browser will import `.png` bytes as base64, submit only explicit operator-approved bbox JSON to the runtime, render PHI-safe aggregate verification, and expose a gated `mdid-browser-redacted.png` binary download only when runtime verification proves changed pixels inside explicit regions.

**Tech Stack:** Rust workspace, `mdid-browser` app helpers/tests, existing `mdid-runtime` PNG visual-redaction contract, base64 JSON payloads, strict TDD.

---

## Success Criteria / Done Contract

1. Browser input mode detects `.png` imports and routes them to a new PNG visual-redaction mode.
2. Browser request payload for PNG uses exactly `{ "png_bytes_base64": "...", "regions": [...] }` and endpoint `/visual-redaction/png`.
3. Browser visible output for PNG contains only PHI-safe aggregate verification and never exposes source filename, raw bytes, bbox arrays, or raw runtime response.
4. Browser download for PNG is available only when `verification.format == "png"`, `verified_changed_pixels_within_regions == true`, `redacted_region_count > 0`, and `rewritten_png_bytes_base64` decodes successfully.
5. Invalid/wrong-format responses do not enable download and return a PHI-safe fixed error.
6. Targeted tests pass: `cargo test -p mdid-browser visual_redaction_png -- --nocapture` and regression `cargo test -p mdid-browser visual_redaction_ppm -- --nocapture`.

## File Structure

- Modify: `crates/mdid-browser/src/app.rs`
  - Add `PngVisualRedaction` input mode and mode metadata.
  - Add PNG request builder, safe renderer, availability gate, download builder.
  - Preserve existing PPM behavior unchanged.
  - Add focused unit tests near existing PPM visual redaction tests.
- Modify: `README.md`
  - Truth-sync bounded Browser/Web PNG execution/download evidence after tests pass; explicitly keep non-goals for OCR, automatic detection, JPEG/PDF/video, Desktop PNG execution, packaging, and field validation.

---

### Task 1: Browser PNG visual redaction helpers and tests

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`

- [x] **Step 1: Write failing tests**

Add focused tests beside the existing `visual_redaction_ppm` tests:

```rust
#[test]
fn png_file_import_targets_visual_redaction_mode() {
    assert_eq!(
        InputMode::from_file_name("Patient Jane Example.PNG"),
        Some(InputMode::PngVisualRedaction)
    );
    assert_eq!(InputMode::PngVisualRedaction.endpoint_path(), "/visual-redaction/png");
    assert_eq!(InputMode::PngVisualRedaction.file_read_mode(), BrowserFileReadMode::DataUrlBase64);
}

#[test]
fn visual_redaction_png_request_uses_runtime_contract_without_source_names() {
    let payload = build_visual_redaction_png_request_payload(
        "cG5nLWJ5dGVz",
        r#"[{"x":0,"y":0,"width":1,"height":1}]"#,
    )
    .unwrap();
    let value: serde_json::Value = serde_json::from_str(&payload).unwrap();
    assert_eq!(value["png_bytes_base64"], "cG5nLWJ5dGVz");
    assert_eq!(value["regions"][0]["width"], 1);
    assert!(value.get("ppm_bytes_base64").is_none());
    assert!(!payload.contains("Patient Jane Example"));
}

#[test]
fn visual_redaction_png_safe_output_and_download_are_gated_by_verification() {
    let response = serde_json::json!({
        "rewritten_png_bytes_base64": base64::engine::general_purpose::STANDARD.encode(b"redacted-png-bytes"),
        "verification": {
            "format": "png",
            "width": 2,
            "height": 1,
            "redacted_region_count": 1,
            "redacted_pixel_count": 1,
            "unchanged_pixel_count": 1,
            "output_byte_count": 18,
            "verified_changed_pixels_within_regions": true
        },
        "source_name": "Patient Jane Example.png",
        "regions": [{"x":0,"y":0,"width":1,"height":1}]
    })
    .to_string();

    let safe = render_visual_redaction_png_safe_output(&response).unwrap();
    assert!(safe.contains("png_visual_redaction"));
    assert!(safe.contains("download_available"));
    assert!(!safe.contains("Patient Jane Example"));
    assert!(!safe.contains("redacted-png-bytes"));
    assert!(!safe.contains("\"regions\""));

    let download = build_visual_redaction_png_download(&response).unwrap();
    assert_eq!(download.file_name, "mdid-browser-redacted.png");
    assert_eq!(download.mime_type, "image/png");
    assert_eq!(download.bytes, b"redacted-png-bytes");
}

#[test]
fn visual_redaction_png_download_rejects_wrong_format_or_unverified_response_phi_safely() {
    for response in [
        serde_json::json!({
            "rewritten_png_bytes_base64": base64::engine::general_purpose::STANDARD.encode(b"redacted-png-bytes"),
            "verification": {"format":"ppm_p6","redacted_region_count":1,"verified_changed_pixels_within_regions":true}
        }),
        serde_json::json!({
            "rewritten_png_bytes_base64": "not base64 Patient Jane Example.png",
            "verification": {"format":"png","redacted_region_count":1,"verified_changed_pixels_within_regions":true}
        }),
        serde_json::json!({
            "rewritten_png_bytes_base64": base64::engine::general_purpose::STANDARD.encode(b"redacted-png-bytes"),
            "verification": {"format":"png","redacted_region_count":0,"verified_changed_pixels_within_regions":true}
        }),
        serde_json::json!({
            "rewritten_png_bytes_base64": base64::engine::general_purpose::STANDARD.encode(b"redacted-png-bytes"),
            "verification": {"format":"png","redacted_region_count":1,"verified_changed_pixels_within_regions":false}
        }),
    ] {
        let error = build_visual_redaction_png_download(&response.to_string()).unwrap_err();
        assert!(error.contains("PNG visual redaction download is only available"));
        assert!(!error.contains("Patient Jane Example"));
        assert!(!error.contains("not base64"));
    }
}
```

- [x] **Step 2: Run tests to verify RED**

Run: `cargo test -p mdid-browser visual_redaction_png -- --nocapture`

Expected: FAIL because `PngVisualRedaction` and PNG helper functions do not exist yet.

- [x] **Step 3: Implement minimal browser PNG support**

Update `crates/mdid-browser/src/app.rs` by mirroring only the existing PPM pattern:

```rust
// Add enum variant near PpmVisualRedaction:
PngVisualRedaction,

// Route .png imports:
} else if file_name.ends_with(".png") {
    Some(Self::PngVisualRedaction)
}

// Add mode id/label/prompt/warning/endpoint/payload/read-mode branches:
Self::PngVisualRedaction => "png-visual-redaction",
Self::PngVisualRedaction => "PNG visual redaction",
Self::PngVisualRedaction => "Import PNG bytes as base64 and paste explicit bbox regions JSON here",
Self::PngVisualRedaction => Some("PNG visual redaction mode is bounded to PNG only with explicit bbox regions approved by the user. No OCR or automatic visual detection is performed, and this browser flow does not support JPEG, PDF, video, or Desktop capture."),
Self::PngVisualRedaction => "/visual-redaction/png",
Self::PngVisualRedaction => "bbox regions JSON",
Self::PngVisualRedaction => BrowserFileReadMode::DataUrlBase64,
```

Add helper functions:

```rust
fn build_visual_redaction_png_request_payload(
    png_bytes_base64: &str,
    regions_json: &str,
) -> Result<String, String> {
    let regions: serde_json::Value = serde_json::from_str(regions_json)
        .map_err(|_| "PNG visual redaction requires valid bbox regions JSON.".to_string())?;
    serde_json::to_string(&serde_json::json!({
        "png_bytes_base64": png_bytes_base64,
        "regions": regions,
    }))
    .map_err(|_| "PNG visual redaction request could not be prepared.".to_string())
}

fn visual_redaction_png_download_available(response_json: &str) -> bool {
    let Ok(response) = serde_json::from_str::<serde_json::Value>(response_json) else {
        return false;
    };
    let Some(verification) = response.get("verification") else {
        return false;
    };
    verification.get("format").and_then(serde_json::Value::as_str) == Some("png")
        && verification
            .get("verified_changed_pixels_within_regions")
            .and_then(serde_json::Value::as_bool)
            == Some(true)
        && verification
            .get("redacted_region_count")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
            > 0
        && response
            .get("rewritten_png_bytes_base64")
            .and_then(serde_json::Value::as_str)
            .and_then(|bytes| base64::engine::general_purpose::STANDARD.decode(bytes).ok())
            .is_some()
}

fn render_visual_redaction_png_safe_output(response_json: &str) -> Result<String, String> {
    let response: serde_json::Value = serde_json::from_str(response_json)
        .map_err(|_| "PNG visual redaction response was not valid JSON.".to_string())?;
    let verification = response
        .get("verification")
        .ok_or_else(|| "PNG visual redaction response did not include verification.".to_string())?;
    let safe = serde_json::json!({
        "format": verification.get("format").and_then(serde_json::Value::as_str).unwrap_or("unknown"),
        "width": verification.get("width").and_then(serde_json::Value::as_u64).unwrap_or(0),
        "height": verification.get("height").and_then(serde_json::Value::as_u64).unwrap_or(0),
        "redacted_region_count": verification.get("redacted_region_count").and_then(serde_json::Value::as_u64).unwrap_or(0),
        "redacted_pixel_count": verification.get("redacted_pixel_count").and_then(serde_json::Value::as_u64).unwrap_or(0),
        "unchanged_pixel_count": verification.get("unchanged_pixel_count").and_then(serde_json::Value::as_u64).unwrap_or(0),
        "output_byte_count": verification.get("output_byte_count").and_then(serde_json::Value::as_u64).unwrap_or(0),
        "verified_changed_pixels_within_regions": verification.get("verified_changed_pixels_within_regions").and_then(serde_json::Value::as_bool).unwrap_or(false),
    });
    serde_json::to_string_pretty(&serde_json::json!({
        "png_visual_redaction": {
            "download_available": visual_redaction_png_download_available(response_json),
            "verification": safe,
        }
    }))
    .map_err(|_| "PNG visual redaction safe output could not be rendered.".to_string())
}

fn build_visual_redaction_png_download(response_json: &str) -> Result<BrowserDownloadPayload, String> {
    const ERROR: &str = "PNG visual redaction download is only available after verified png redaction with changed pixels inside explicit regions.";
    if !visual_redaction_png_download_available(response_json) {
        return Err(ERROR.to_string());
    }
    let response: serde_json::Value = serde_json::from_str(response_json).map_err(|_| ERROR.to_string())?;
    let bytes = response
        .get("rewritten_png_bytes_base64")
        .and_then(serde_json::Value::as_str)
        .and_then(|encoded| base64::engine::general_purpose::STANDARD.decode(encoded).ok())
        .ok_or_else(|| ERROR.to_string())?;
    Ok(BrowserDownloadPayload {
        file_name: "mdid-browser-redacted.png".to_string(),
        mime_type: "image/png",
        bytes,
    })
}
```

Wire submit/render/download dispatch wherever the existing PPM mode is dispatched:
- `PngVisualRedaction` should call `build_visual_redaction_png_request_payload`.
- It should store raw runtime response in the same redacted internal response slot used for visual redaction.
- Visible output should come from `render_visual_redaction_png_safe_output`.
- Download should call `build_visual_redaction_png_download`.

- [x] **Step 4: Run targeted tests to verify GREEN**

Run: `cargo test -p mdid-browser visual_redaction_png -- --nocapture`

Expected: PASS.

- [x] **Step 5: Run PPM regression tests**

Run: `cargo test -p mdid-browser visual_redaction_ppm -- --nocapture`

Expected: PASS; existing PPM flow unchanged.

- [x] **Step 6: Commit**

```bash
git add crates/mdid-browser/src/app.rs docs/superpowers/plans/2026-05-02-browser-png-visual-redaction.md
git commit -m "feat(browser): add png visual redaction flow"
```

### Task 2: README truth-sync for Browser PNG visual redaction

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Write/update documentation after code verification**

Add a concise verification evidence paragraph stating:

```markdown
Verification evidence for the bounded Browser/Web PNG visual redaction mode landed on this branch: Browser/Web now accepts local `.png` imports as base64/data-url payloads, submits explicit operator-provided bbox regions to the existing local `/visual-redaction/png` runtime endpoint using the runtime contract `{ "png_bytes_base64": "...", "regions": [...] }`, renders only PHI-safe aggregate verification fields in the visible UI, and exposes a binary `mdid-browser-redacted.png` download only when the runtime returns verified `png` redaction bytes with changed pixels inside approved regions. Repository-visible verification passed: `cargo test -p mdid-browser visual_redaction_png -- --nocapture` and `cargo test -p mdid-browser visual_redaction_ppm -- --nocapture`. This advances Browser/Web workflow depth and actual visual/image pixel redaction only for bounded PNG explicit bbox byte rewrite/download; it does not claim OCR, automatic visual detection, handwriting recognition, JPEG rewrite, PDF page redaction/rewrite, video/FCS media-byte export, Desktop PNG execution, packaging, or field validation.
```

Also update the current snapshot Image row and Browser/Web row to include Browser/Web PNG execution/download without changing the 99% cap.

- [ ] **Step 2: Verify README truthfulness**

Run: `git diff -- README.md`

Expected: Diff mentions bounded Browser/Web PNG only and preserves explicit non-goals.

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs(readme): truth-sync browser png visual redaction"
```
