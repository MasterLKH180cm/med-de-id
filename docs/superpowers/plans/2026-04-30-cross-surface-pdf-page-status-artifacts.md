# Cross-Surface PDF Page Status Artifacts Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add PHI-safe per-page PDF review status metadata to Browser/Web downloaded PDF review reports and Desktop saved PDF review reports.

**Architecture:** Reuse the existing PDF review report sanitizer path in each surface and add a separate `page_statuses` array containing only primitive/null allowlisted fields. Keep this separate from raw PDF bytes, raw text, bounding boxes, and nested candidate payloads so the artifact remains PHI-safe and review-only.

**Tech Stack:** Rust workspace, `mdid-browser`, `mdid-desktop`, `serde_json`, Cargo tests.

---

## File Structure

- Modify: `crates/mdid-browser/src/app.rs`
  - Add `sanitized_pdf_page_statuses(response: &serde_json::Value) -> serde_json::Value` near the existing PDF review report sanitizer helpers.
  - Include `page_statuses` in `build_pdf_review_report_download`.
  - Add tests under the existing `#[cfg(test)]` module.
- Modify: `crates/mdid-desktop/src/lib.rs`
  - Add `sanitize_desktop_pdf_page_statuses(value: Option<&serde_json::Value>) -> serde_json::Value` near existing PDF review report sanitizer helpers.
  - Include `page_statuses` in `build_desktop_pdf_review_report_save`.
  - Add tests under the existing `#[cfg(test)]` module.
- Modify: `README.md`
  - Truth-sync completion snapshot and verification evidence after implementation and review.

### Task 1: Browser PDF review report page statuses

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`
- Test: `crates/mdid-browser/src/app.rs`

- [ ] **Step 1: Write the failing test**

Add this test in the existing browser test module near the PDF review report tests:

```rust
#[test]
fn pdf_review_report_download_includes_sanitized_page_statuses() {
    let response = serde_json::json!({
        "summary": {"total_pages": 2},
        "page_statuses": [
            {
                "page": 1,
                "status": "text_layer_reviewed",
                "requires_ocr": false,
                "candidate_count": 2,
                "raw_text": "Patient Alice",
                "bbox": [1, 2, 3, 4]
            },
            {
                "page": 2,
                "status": "ocr_required",
                "requires_ocr": true,
                "candidate_count": 0,
                "nested": {"patient": "Alice"}
            },
            "skip-me"
        ]
    });

    let download = build_pdf_review_report_download(&response.to_string(), Some("scan.pdf")).unwrap();
    let report: serde_json::Value = serde_json::from_slice(&download.bytes).unwrap();

    assert_eq!(report["page_statuses"][0]["page"], 1);
    assert_eq!(report["page_statuses"][0]["status"], "text_layer_reviewed");
    assert_eq!(report["page_statuses"][0]["requires_ocr"], false);
    assert_eq!(report["page_statuses"][0]["candidate_count"], 2);
    assert!(report["page_statuses"][0].get("raw_text").is_none());
    assert!(report["page_statuses"][0].get("bbox").is_none());
    assert_eq!(report["page_statuses"][1]["page"], 2);
    assert!(report["page_statuses"][1].get("nested").is_none());
    assert_eq!(report["page_statuses"].as_array().unwrap().len(), 2);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p mdid-browser pdf_review_report_download_includes_sanitized_page_statuses -- --nocapture`

Expected: FAIL because `page_statuses` is not present in the PDF review report JSON.

- [ ] **Step 3: Write minimal implementation**

Add this helper near `sanitized_pdf_review_queue` and include it in the report:

```rust
fn sanitized_pdf_page_statuses(response: &serde_json::Value) -> serde_json::Value {
    let statuses = response
        .get("page_statuses")
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let object = item.as_object()?;
                    let mut sanitized = serde_json::Map::new();
                    for key in ["page", "status", "requires_ocr", "candidate_count"] {
                        if let Some(value) = object.get(key).and_then(pdf_review_report_primitive) {
                            sanitized.insert(key.to_string(), value);
                        }
                    }
                    Some(serde_json::Value::Object(sanitized))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    serde_json::Value::Array(statuses)
}
```

Change the report object to:

```rust
let report = serde_json::json!({
    "mode": "pdf_review_report",
    "summary": sanitized_pdf_review_summary(&response),
    "review_queue": sanitized_pdf_review_queue(&response),
    "page_statuses": sanitized_pdf_page_statuses(&response),
});
```

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test -p mdid-browser pdf_review_report_download -- --nocapture`

Expected: PASS for all PDF review report download tests.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-browser/src/app.rs
git commit -m "feat(browser): add pdf page statuses to reports"
```

### Task 2: Desktop PDF review report page statuses

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`
- Test: `crates/mdid-desktop/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Add this test near the existing desktop PDF review report tests:

```rust
#[test]
fn desktop_pdf_review_report_save_includes_sanitized_page_statuses() {
    let response = serde_json::json!({
        "summary": {"total_pages": 2},
        "page_statuses": [
            {
                "page": 1,
                "status": "text_layer_reviewed",
                "requires_ocr": false,
                "candidate_count": 3,
                "raw_text": "Patient Alice",
                "bbox": [1, 2, 3, 4]
            },
            {
                "page": 2,
                "status": "ocr_required",
                "requires_ocr": true,
                "candidate_count": 0,
                "nested": {"patient": "Alice"}
            },
            42
        ]
    });

    let save = build_desktop_pdf_review_report_save(&response.to_string(), Some("scan.pdf")).unwrap();
    let report: serde_json::Value = serde_json::from_str(&save.contents).unwrap();

    assert_eq!(report["page_statuses"][0]["page"], 1);
    assert_eq!(report["page_statuses"][0]["status"], "text_layer_reviewed");
    assert_eq!(report["page_statuses"][0]["requires_ocr"], false);
    assert_eq!(report["page_statuses"][0]["candidate_count"], 3);
    assert!(report["page_statuses"][0].get("raw_text").is_none());
    assert!(report["page_statuses"][0].get("bbox").is_none());
    assert_eq!(report["page_statuses"][1]["page"], 2);
    assert!(report["page_statuses"][1].get("nested").is_none());
    assert_eq!(report["page_statuses"].as_array().unwrap().len(), 2);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p mdid-desktop desktop_pdf_review_report_save_includes_sanitized_page_statuses -- --nocapture`

Expected: FAIL because `page_statuses` is not present in the PDF review report JSON.

- [ ] **Step 3: Write minimal implementation**

Add this helper near `sanitize_desktop_pdf_review_report_queue`:

```rust
fn sanitize_desktop_pdf_page_statuses(value: Option<&serde_json::Value>) -> serde_json::Value {
    let Some(serde_json::Value::Array(items)) = value else {
        return serde_json::json!([]);
    };
    serde_json::Value::Array(
        items
            .iter()
            .filter_map(|item| {
                let object = item.as_object()?;
                let mut sanitized = serde_json::Map::new();
                for key in ["page", "status", "requires_ocr", "candidate_count"] {
                    if let Some(value) = object.get(key).filter(|value| is_json_primitive(value)) {
                        sanitized.insert(key.to_string(), value.clone());
                    }
                }
                Some(serde_json::Value::Object(sanitized))
            })
            .collect(),
    )
}
```

Change the report object to:

```rust
let report = serde_json::json!({
    "mode": "pdf_review_report",
    "summary": sanitize_desktop_pdf_review_report_summary(object.get("summary")),
    "review_queue": sanitize_desktop_pdf_review_report_queue(object.get("review_queue")),
    "page_statuses": sanitize_desktop_pdf_page_statuses(object.get("page_statuses")),
});
```

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test -p mdid-desktop pdf_review_report_save -- --nocapture`

Expected: PASS for all PDF review report save tests.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-desktop/src/lib.rs
git commit -m "feat(desktop): add pdf page statuses to reports"
```

### Task 3: README truth-sync and final verification

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Run focused and hygiene verification**

Run:

```bash
cargo test -p mdid-browser pdf_review_report_download -- --nocapture
cargo test -p mdid-desktop pdf_review_report_save -- --nocapture
cargo fmt --check
git diff --check
```

Expected: all commands pass.

- [ ] **Step 2: Update README completion snapshot**

Update the completion snapshot to describe the newly landed page status metadata in Browser/Web and Desktop PDF review artifacts. Because Browser/Web is already at 100%, Browser/Web remains 100%; Desktop app moves from 92% to 97%; Overall moves from 97% to 98%.

- [ ] **Step 3: Commit README truth-sync**

```bash
git add README.md docs/superpowers/plans/2026-04-30-cross-surface-pdf-page-status-artifacts.md
git commit -m "docs: truth-sync pdf page status artifacts"
```
