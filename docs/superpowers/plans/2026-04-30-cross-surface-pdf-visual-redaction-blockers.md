# Cross-Surface PDF Visual Redaction Blockers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add PHI-safe visual-redaction blocker metadata to Browser/Web and Desktop PDF review report artifacts, and truth-sync README completion percentages away from the previous false 100% surface claims.

**Architecture:** Reuse the existing PDF review report sanitization path in `mdid-browser` and `mdid-desktop`. Add a new allowlisted `visual_redaction_blockers` object derived only from primitive/null `page_statuses` metadata and existing no-rewrite status, without raw text, PDF bytes/base64, bbox arrays, or nested payloads.

**Tech Stack:** Rust workspace, Leptos browser crate, desktop helper crate, serde_json, Cargo tests.

---

## File Structure

- Modify: `crates/mdid-browser/src/app.rs` — browser PDF review report JSON builder and tests.
- Modify: `crates/mdid-desktop/src/lib.rs` — desktop PDF review report save helper and tests.
- Modify: `README.md` — completion rubric truth-sync and verification evidence.

### Task 1: Browser PDF visual-redaction blocker report metadata

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`

- [ ] **Step 1: Write the failing browser test**

Add a test near the existing browser PDF review report tests:

```rust
#[test]
fn pdf_review_report_download_includes_visual_redaction_blockers_without_sensitive_payloads() {
    let response = serde_json::json!({
        "source": {"kind": "pdf", "status": "review_required"},
        "page_statuses": [
            {"page": 1, "status": "ok", "requires_ocr": false, "candidate_count": 2, "raw_text": "Patient Alice"},
            {"page": 2, "status": "visual_review_required", "requires_ocr": false, "candidate_count": 0, "bbox": [1, 2, 3, 4]},
            {"page": 3, "status": "requires_ocr", "requires_ocr": true, "candidate_count": 0, "pdf_bytes_base64": "JVBERi0="}
        ],
        "review_queue": [{"page": 2, "kind": "visual", "status": "review_required", "raw_text": "Alice"}],
        "no_rewritten_pdf": true,
        "pdf_bytes_base64": "JVBERi0="
    });

    let payload = build_pdf_review_report_download(&response.to_string(), Some("scan.pdf")).unwrap();
    let report: serde_json::Value = serde_json::from_slice(&payload.bytes).unwrap();

    assert_eq!(report["visual_redaction_blockers"]["visual_review_pages"], 1);
    assert_eq!(report["visual_redaction_blockers"]["ocr_required_pages"], 1);
    assert_eq!(report["visual_redaction_blockers"]["blocked_page_count"], 2);
    assert_eq!(report["visual_redaction_blockers"]["redaction_rewrite_available"], false);
    assert_eq!(report["visual_redaction_blockers"]["status"], "blocked_pending_visual_redaction_or_ocr");

    let encoded = String::from_utf8(payload.bytes).unwrap();
    assert!(!encoded.contains("Patient Alice"));
    assert!(!encoded.contains("pdf_bytes_base64"));
    assert!(!encoded.contains("bbox"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p mdid-browser pdf_review_report_download_includes_visual_redaction_blockers_without_sensitive_payloads -- --nocapture`

Expected: FAIL because `visual_redaction_blockers` is missing.

- [ ] **Step 3: Implement minimal browser metadata builder**

Add a helper near `sanitized_pdf_ocr_blockers`:

```rust
fn sanitized_pdf_visual_redaction_blockers(response: &serde_json::Value) -> serde_json::Value {
    let mut visual_review_pages = 0_u64;
    let mut ocr_required_pages = 0_u64;
    let mut blocked_page_count = 0_u64;

    if let Some(statuses) = response.get("page_statuses").and_then(serde_json::Value::as_array) {
        for status in statuses.iter().filter_map(serde_json::Value::as_object) {
            let requires_ocr = status
                .get("requires_ocr")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            let visual_review_required = status
                .get("status")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|value| value.eq_ignore_ascii_case("visual_review_required"));

            if visual_review_required {
                visual_review_pages += 1;
            }
            if requires_ocr {
                ocr_required_pages += 1;
            }
            if visual_review_required || requires_ocr {
                blocked_page_count += 1;
            }
        }
    }

    serde_json::json!({
        "visual_review_pages": visual_review_pages,
        "ocr_required_pages": ocr_required_pages,
        "blocked_page_count": blocked_page_count,
        "redaction_rewrite_available": response
            .get("no_rewritten_pdf")
            .and_then(serde_json::Value::as_bool)
            == Some(false),
        "status": if blocked_page_count == 0 {
            "no_visual_redaction_blockers_detected"
        } else {
            "blocked_pending_visual_redaction_or_ocr"
        },
    })
}
```

Then include it in `build_pdf_review_report_download`:

```rust
"visual_redaction_blockers": sanitized_pdf_visual_redaction_blockers(&response),
```

- [ ] **Step 4: Run browser tests**

Run: `cargo test -p mdid-browser pdf_review_report_download_includes_visual_redaction_blockers_without_sensitive_payloads -- --nocapture`

Expected: PASS.

Run: `cargo test -p mdid-browser pdf_review_report -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Commit browser slice**

```bash
git add crates/mdid-browser/src/app.rs
git commit -m "feat(browser): add pdf visual redaction blockers"
```

### Task 2: Desktop PDF visual-redaction blocker report metadata

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`

- [ ] **Step 1: Write the failing desktop test**

Add a test near existing desktop PDF review report save tests:

```rust
#[test]
fn pdf_review_report_save_includes_visual_redaction_blockers_without_sensitive_payloads() {
    let response = serde_json::json!({
        "source": {"kind": "pdf", "status": "review_required"},
        "page_statuses": [
            {"page": 1, "status": "ok", "requires_ocr": false, "candidate_count": 2, "raw_text": "Patient Alice"},
            {"page": 2, "status": "visual_review_required", "requires_ocr": false, "candidate_count": 0, "bbox": [1, 2, 3, 4]},
            {"page": 3, "status": "requires_ocr", "requires_ocr": true, "candidate_count": 0, "pdf_bytes_base64": "JVBERi0="}
        ],
        "review_queue": [{"page": 2, "kind": "visual", "status": "review_required", "raw_text": "Alice"}],
        "no_rewritten_pdf": true,
        "pdf_bytes_base64": "JVBERi0="
    });

    let report = build_pdf_review_report_save_bytes(&response.to_string()).unwrap();
    let report: serde_json::Value = serde_json::from_slice(&report).unwrap();

    assert_eq!(report["visual_redaction_blockers"]["visual_review_pages"], 1);
    assert_eq!(report["visual_redaction_blockers"]["ocr_required_pages"], 1);
    assert_eq!(report["visual_redaction_blockers"]["blocked_page_count"], 2);
    assert_eq!(report["visual_redaction_blockers"]["redaction_rewrite_available"], false);
    assert_eq!(report["visual_redaction_blockers"]["status"], "blocked_pending_visual_redaction_or_ocr");

    let encoded = serde_json::to_string(&report).unwrap();
    assert!(!encoded.contains("Patient Alice"));
    assert!(!encoded.contains("pdf_bytes_base64"));
    assert!(!encoded.contains("bbox"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p mdid-desktop pdf_review_report_save_includes_visual_redaction_blockers_without_sensitive_payloads -- --nocapture`

Expected: FAIL because `visual_redaction_blockers` is missing.

- [ ] **Step 3: Implement minimal desktop metadata builder**

Add the same `sanitized_pdf_visual_redaction_blockers` helper next to the desktop PDF report sanitizers, and include this field in the desktop PDF report object:

```rust
"visual_redaction_blockers": sanitized_pdf_visual_redaction_blockers(&response),
```

- [ ] **Step 4: Run desktop tests**

Run: `cargo test -p mdid-desktop pdf_review_report_save_includes_visual_redaction_blockers_without_sensitive_payloads -- --nocapture`

Expected: PASS.

Run: `cargo test -p mdid-desktop pdf_review_report -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Commit desktop slice**

```bash
git add crates/mdid-desktop/src/lib.rs
git commit -m "feat(desktop): add pdf visual redaction blockers"
```

### Task 3: README completion truth-sync

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update completion snapshot honestly**

Replace the completion snapshot so Browser/Web and Desktop are no longer claimed as 100% while OCR, visual redaction, fuller file-picker/upload/download UX, vault UX, and portable transfer UX remain incomplete. Use these percentages for this round:

```markdown
| CLI | 95% |
| Browser/web | 88% |
| Desktop app | 88% |
| Overall | 94% |
```

Explain that prior 100% claims were invalid because report-artifact metadata did not equal full browser/desktop completion, and the new rubric requires real workflow completion including OCR/visual redaction/rewrite/export and fuller vault/portable UX.

- [ ] **Step 2: Add verification evidence**

Add a new evidence paragraph naming this branch and the browser/desktop visual-redaction blocker commits. State that both surfaces now add PHI-safe `visual_redaction_blockers` report metadata, but this is still blocker evidence and not actual OCR, visual redaction, or rewritten PDF export.

- [ ] **Step 3: Run formatting and focused tests**

Run:

```bash
cargo test -p mdid-browser pdf_review_report -- --nocapture
cargo test -p mdid-desktop pdf_review_report -- --nocapture
cargo fmt --check
git diff --check
```

Expected: all PASS.

- [ ] **Step 4: Commit README truth-sync**

```bash
git add README.md docs/superpowers/plans/2026-04-30-cross-surface-pdf-visual-redaction-blockers.md
git commit -m "docs: truth-sync visual redaction blocker progress"
```
