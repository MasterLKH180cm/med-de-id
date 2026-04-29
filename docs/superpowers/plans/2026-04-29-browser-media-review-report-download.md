# Browser Media Review Report Download Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a PHI-safe structured JSON download for browser conservative media review results.

**Architecture:** Keep the slice inside `mdid-browser` helper/state code. Convert already-rendered media review summaries and review queue lines into a bounded allowlisted JSON report that never includes raw source values or media bytes, while preserving the existing browser download path.

**Tech Stack:** Rust, Leptos browser crate, serde_json, cargo test/clippy.

---

## File Structure

- Modify: `crates/mdid-browser/src/app.rs`
  - Add media-specific safe report helpers near the existing browser download helper functions.
  - Route `InputMode::MediaMetadataJson` downloads to the new structured media report helper.
  - Add unit tests in the existing `#[cfg(test)] mod tests` in the same file, following the crate's current pattern.
- Modify: `README.md`
  - Truth-sync browser/web and overall completion after verification.

### Task 1: Browser media review structured report download

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`
- Test: `crates/mdid-browser/src/app.rs` existing unit test module

- [ ] **Step 1: Write the failing test**

Add this test to the existing test module in `crates/mdid-browser/src/app.rs`:

```rust
#[test]
fn media_review_download_is_structured_and_phi_safe() {
    let mut state = BrowserFlowState::default();
    state.input_mode = InputMode::MediaMetadataJson;
    state.result_output = "Media rewrite/export unavailable: runtime returned metadata-only conservative review.".to_string();
    state.summary = "total_items: 1\nmetadata_only_items: 1\nvisual_review_required_items: 1\nunsupported_items: 0\nreview_required_candidates: 1\nrewritten_media_bytes_base64: null".to_string();
    state.review_queue = "- PatientName / image / Name / confidence 0.97 / value: <redacted>".to_string();

    let payload = state.prepared_download_payload().expect("download payload");
    let report: serde_json::Value = serde_json::from_slice(&payload.bytes).expect("report json");
    let report_text = String::from_utf8(payload.bytes).expect("report utf8");

    assert_eq!(payload.file_name, "mdid-browser-media-review-report.json");
    assert_eq!(payload.mime_type, "application/json;charset=utf-8");
    assert!(payload.is_text);
    assert_eq!(report["mode"], "media_metadata_review");
    assert_eq!(report["summary"]["total_items"], 1);
    assert_eq!(report["summary"]["metadata_only_items"], 1);
    assert_eq!(report["summary"]["visual_review_required_items"], 1);
    assert_eq!(report["summary"]["unsupported_items"], 0);
    assert_eq!(report["summary"]["review_required_candidates"], 1);
    assert_eq!(report["summary"]["rewritten_media_bytes_base64"], serde_json::Value::Null);
    assert_eq!(report["review_queue"][0]["metadata_key"], "redacted-field");
    assert_eq!(report["review_queue"][0]["format"], "image");
    assert_eq!(report["review_queue"][0]["phi_type"], "Name");
    assert_eq!(report["review_queue"][0]["confidence"], 0.97);
    assert_eq!(report["review_queue"][0]["value"], "redacted");
    assert!(!report_text.contains("Jane Patient"));
    assert!(!report_text.contains("PatientName"));
    assert!(!report_text.contains("source_value"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p mdid-browser media_review_download_is_structured_and_phi_safe -- --nocapture`

Expected: FAIL because media downloads still use the generic string report shape (`mode` is label text and summary/review_queue are strings), not the structured `media_metadata_review` JSON object.

- [ ] **Step 3: Implement minimal structured media report helper**

In `impl BrowserFlowState`, change `prepared_download_payload` so `InputMode::MediaMetadataJson` calls a new helper:

```rust
InputMode::MediaMetadataJson => Ok(BrowserDownloadPayload {
    file_name,
    mime_type: "application/json;charset=utf-8",
    bytes: self.media_review_report_download_json()?,
    is_text: true,
}),
InputMode::PdfBase64
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

Add these helpers near `review_report_download_json`:

```rust
fn media_review_report_download_json(&self) -> Result<Vec<u8>, String> {
    serde_json::to_vec_pretty(&serde_json::json!({
        "mode": "media_metadata_review",
        "summary": parse_media_summary_report(&self.summary),
        "review_queue": parse_media_review_queue_report(&self.review_queue),
        "output": "media rewrite/export unavailable",
    }))
    .map_err(|_| "Browser output download could not encode media review report JSON.".to_string())
}
```

Add standalone helpers below the `impl BrowserFlowState` block:

```rust
fn parse_media_summary_report(summary: &str) -> serde_json::Value {
    let mut object = serde_json::Map::new();
    for line in summary.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "total_items"
            | "metadata_only_items"
            | "visual_review_required_items"
            | "unsupported_items"
            | "review_required_candidates" => {
                if let Ok(parsed) = value.parse::<usize>() {
                    object.insert(key.to_string(), serde_json::json!(parsed));
                }
            }
            "rewritten_media_bytes_base64" => {
                object.insert(key.to_string(), serde_json::Value::Null);
            }
            _ => {}
        }
    }
    serde_json::Value::Object(object)
}

fn parse_media_review_queue_report(review_queue: &str) -> serde_json::Value {
    let items = review_queue
        .lines()
        .filter_map(parse_media_review_queue_line)
        .collect::<Vec<_>>();
    serde_json::Value::Array(items)
}

fn parse_media_review_queue_line(line: &str) -> Option<serde_json::Value> {
    let line = line.trim().strip_prefix("- ")?;
    let parts = line.split(" / ").collect::<Vec<_>>();
    if parts.len() != 5 {
        return None;
    }
    let confidence = parts[3]
        .strip_prefix("confidence ")
        .and_then(|value| value.parse::<f64>().ok());
    Some(serde_json::json!({
        "metadata_key": "redacted-field",
        "format": allowlisted_media_format(parts[1]),
        "phi_type": allowlisted_media_phi_type(parts[2]),
        "confidence": confidence,
        "value": "redacted",
    }))
}

fn allowlisted_media_format(value: &str) -> serde_json::Value {
    match value.trim() {
        "image" | "video" | "fcs" | "metadata" => serde_json::json!(value.trim()),
        _ => serde_json::Value::Null,
    }
}

fn allowlisted_media_phi_type(value: &str) -> serde_json::Value {
    match value.trim() {
        "Name" | "RecordId" | "Date" | "Location" | "Contact" | "FreeText" => {
            serde_json::json!(value.trim())
        }
        _ => serde_json::Value::Null,
    }
}
```

- [ ] **Step 4: Run targeted test to verify it passes**

Run: `cargo test -p mdid-browser media_review_download_is_structured_and_phi_safe -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Run broader browser verification**

Run:

```bash
cargo test -p mdid-browser --lib
cargo clippy -p mdid-browser --all-targets -- -D warnings
git diff --check
```

Expected: all PASS with no warnings or whitespace errors.

- [ ] **Step 6: Commit**

```bash
git add crates/mdid-browser/src/app.rs
git commit -m "feat(browser): add media review report downloads"
```

### Task 2: README truth-sync for browser media report download

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update completion snapshot text**

Update the README completion snapshot to mention browser PHI-safe structured media review JSON downloads. Raise browser/web only if supported by the landed feature and verification evidence; keep CLI and desktop unchanged.

- [ ] **Step 2: Run docs verification**

Run:

```bash
git diff --check
git diff -- README.md
```

Expected: README diff only changes truthful completion/evidence text.

- [ ] **Step 3: Commit**

```bash
git add README.md docs/superpowers/plans/2026-04-29-browser-media-review-report-download.md
git commit -m "docs: truth-sync browser media review downloads"
```
