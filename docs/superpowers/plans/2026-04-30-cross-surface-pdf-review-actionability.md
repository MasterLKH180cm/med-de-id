# Cross-Surface PDF Review Actionability Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add PHI-safe PDF review actionability summaries to Browser/Web and Desktop PDF review report artifacts so users can understand the next safe manual action when OCR or visual-redaction blockers prevent automatic rewrite.

**Architecture:** Keep actionability generation local to each surface report sanitizer and do not change runtime/core semantics. Browser and Desktop reports will expose the same allowlisted `actionability` object with blocker counts, an `automatic_rewrite_ready` boolean, and stable PHI-free `next_steps` strings derived only from already-sanitized page/blocker metadata.

**Tech Stack:** Rust workspace, `serde_json`, existing `mdid-browser` and `mdid-desktop` report helper tests, Cargo test runner.

---

## File Structure

- Modify: `crates/mdid-browser/src/app.rs`
  - Add `sanitized_pdf_review_actionability(&serde_json::Value) -> serde_json::Value` near existing PDF report sanitizers.
  - Include `actionability` in `build_pdf_review_report_download`.
  - Add focused browser tests for blocker and rewrite-ready summaries.
- Modify: `crates/mdid-desktop/src/lib.rs`
  - Add `sanitize_desktop_pdf_review_actionability(&serde_json::Value) -> serde_json::Value` near existing desktop PDF report sanitizers.
  - Include `actionability` in desktop PDF report builders.
  - Add focused desktop tests for blocker and rewrite-ready summaries.
- Modify: `README.md`
  - Truth-sync current completion snapshot after verified Browser/Web and Desktop actionability artifacts land.

### Task 1: Browser PDF Review Actionability Report

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`
- Test: `crates/mdid-browser/src/app.rs`

- [ ] **Step 1: Write the failing browser tests**

Add these tests in the existing `#[cfg(test)]` module near the PDF review report tests:

```rust
#[test]
fn pdf_review_report_download_includes_actionability_for_blocked_pdf() {
    let response = serde_json::json!({
        "ok": true,
        "command": "pdf-review",
        "source": {"path": "/tmp/forms/intake.pdf"},
        "metadata": {
            "page_statuses": [
                {"page": 1, "status": "review_required", "requires_ocr": true, "can_rewrite": false},
                {"page": 2, "status": "visual_review_required", "requires_visual_review": true, "can_rewrite": false}
            ],
            "redaction_rewrite_available": false
        },
        "raw_text": "patient name must not be copied"
    });

    let report = build_pdf_review_report_download(response).unwrap().json;

    assert_eq!(report["actionability"]["automatic_rewrite_ready"], false);
    assert_eq!(report["actionability"]["blocked_page_count"], 2);
    assert_eq!(report["actionability"]["ocr_required_pages"], 1);
    assert_eq!(report["actionability"]["visual_review_pages"], 1);
    assert_eq!(
        report["actionability"]["next_steps"],
        serde_json::json!([
            "Run OCR outside this tool before attempting PDF rewrite.",
            "Review visual-only pages manually before exporting a redacted PDF.",
            "Keep this PHI-safe report with the case audit trail."
        ])
    );
    assert!(!serde_json::to_string(&report["actionability"]).unwrap().contains("patient name"));
}

#[test]
fn pdf_review_report_download_marks_rewrite_ready_when_no_blockers_remain() {
    let response = serde_json::json!({
        "ok": true,
        "command": "pdf-review",
        "source": {"path": "/tmp/forms/ready.pdf"},
        "metadata": {
            "page_statuses": [
                {"page": 1, "status": "rewrite_ready", "requires_ocr": false, "requires_visual_review": false, "can_rewrite": true}
            ],
            "redaction_rewrite_available": true
        }
    });

    let report = build_pdf_review_report_download(response).unwrap().json;

    assert_eq!(report["actionability"]["automatic_rewrite_ready"], true);
    assert_eq!(report["actionability"]["blocked_page_count"], 0);
    assert_eq!(
        report["actionability"]["next_steps"],
        serde_json::json!(["PDF review metadata indicates rewrite readiness; verify output before release."])
    );
}
```

- [ ] **Step 2: Run browser tests to verify RED**

Run: `cargo test -p mdid-browser pdf_review_report_download_includes_actionability -- --nocapture`

Expected: FAIL because `report["actionability"]` is missing/null.

- [ ] **Step 3: Implement minimal browser actionability sanitizer**

Add this helper near `sanitized_pdf_visual_redaction_blockers` and include it in the report object:

```rust
fn sanitized_pdf_review_actionability(response: &serde_json::Value) -> serde_json::Value {
    let ocr_blockers = sanitized_pdf_ocr_blockers(response);
    let visual_blockers = sanitized_pdf_visual_redaction_blockers(response);
    let ocr_required_pages = ocr_blockers["requires_ocr_pages"].as_u64().unwrap_or(0);
    let visual_review_pages = visual_blockers["visual_review_pages"].as_u64().unwrap_or(0);
    let blocked_page_count = ocr_required_pages + visual_review_pages;
    let automatic_rewrite_ready = response
        .pointer("/metadata/redaction_rewrite_available")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
        && blocked_page_count == 0;

    let mut next_steps = Vec::new();
    if ocr_required_pages > 0 {
        next_steps.push("Run OCR outside this tool before attempting PDF rewrite.");
    }
    if visual_review_pages > 0 {
        next_steps.push("Review visual-only pages manually before exporting a redacted PDF.");
    }
    if automatic_rewrite_ready {
        next_steps.push("PDF review metadata indicates rewrite readiness; verify output before release.");
    } else {
        next_steps.push("Keep this PHI-safe report with the case audit trail.");
    }

    serde_json::json!({
        "automatic_rewrite_ready": automatic_rewrite_ready,
        "blocked_page_count": blocked_page_count,
        "ocr_required_pages": ocr_required_pages,
        "visual_review_pages": visual_review_pages,
        "next_steps": next_steps,
    })
}
```

In `build_pdf_review_report_download`, add:

```rust
"actionability": sanitized_pdf_review_actionability(&response),
```

- [ ] **Step 4: Run browser tests to verify GREEN**

Run: `cargo test -p mdid-browser pdf_review_report_download_includes_actionability -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Run broader browser PDF report tests**

Run: `cargo test -p mdid-browser pdf_review_report -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Commit browser slice**

Run:

```bash
git add crates/mdid-browser/src/app.rs docs/superpowers/plans/2026-04-30-cross-surface-pdf-review-actionability.md
git commit -m "feat(browser): add pdf review actionability"
```

### Task 2: Desktop PDF Review Actionability Report

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`
- Test: `crates/mdid-desktop/src/lib.rs`

- [ ] **Step 1: Write the failing desktop tests**

Add these tests in the existing `#[cfg(test)]` module near desktop PDF review report tests:

```rust
#[test]
fn pdf_review_report_save_includes_actionability_for_blocked_pdf() {
    let response = serde_json::json!({
        "ok": true,
        "command": "pdf-review",
        "source": {"path": "/tmp/forms/intake.pdf"},
        "metadata": {
            "page_statuses": [
                {"page": 1, "status": "review_required", "requires_ocr": true, "can_rewrite": false},
                {"page": 2, "status": "visual_review_required", "requires_visual_review": true, "can_rewrite": false}
            ],
            "redaction_rewrite_available": false
        },
        "raw_text": "patient name must not be copied"
    });

    let report = build_desktop_pdf_review_report(&response, Some("intake")).unwrap();

    assert_eq!(report["actionability"]["automatic_rewrite_ready"], false);
    assert_eq!(report["actionability"]["blocked_page_count"], 2);
    assert_eq!(report["actionability"]["ocr_required_pages"], 1);
    assert_eq!(report["actionability"]["visual_review_pages"], 1);
    assert_eq!(
        report["actionability"]["next_steps"],
        serde_json::json!([
            "Run OCR outside this tool before attempting PDF rewrite.",
            "Review visual-only pages manually before exporting a redacted PDF.",
            "Keep this PHI-safe report with the case audit trail."
        ])
    );
    assert!(!serde_json::to_string(&report["actionability"]).unwrap().contains("patient name"));
}

#[test]
fn pdf_review_report_save_marks_rewrite_ready_when_no_blockers_remain() {
    let response = serde_json::json!({
        "ok": true,
        "command": "pdf-review",
        "source": {"path": "/tmp/forms/ready.pdf"},
        "metadata": {
            "page_statuses": [
                {"page": 1, "status": "rewrite_ready", "requires_ocr": false, "requires_visual_review": false, "can_rewrite": true}
            ],
            "redaction_rewrite_available": true
        }
    });

    let report = build_desktop_pdf_review_report(&response, Some("ready")).unwrap();

    assert_eq!(report["actionability"]["automatic_rewrite_ready"], true);
    assert_eq!(report["actionability"]["blocked_page_count"], 0);
    assert_eq!(
        report["actionability"]["next_steps"],
        serde_json::json!(["PDF review metadata indicates rewrite readiness; verify output before release."])
    );
}
```

- [ ] **Step 2: Run desktop tests to verify RED**

Run: `cargo test -p mdid-desktop pdf_review_report_save_includes_actionability -- --nocapture`

Expected: FAIL because `report["actionability"]` is missing/null.

- [ ] **Step 3: Implement minimal desktop actionability sanitizer**

Add this helper near `sanitize_desktop_pdf_visual_redaction_blockers` and include it in the desktop PDF review report object:

```rust
fn sanitize_desktop_pdf_review_actionability(response: &serde_json::Value) -> serde_json::Value {
    let ocr_blockers = sanitize_desktop_pdf_ocr_blockers(response);
    let visual_blockers = sanitize_desktop_pdf_visual_redaction_blockers(response);
    let ocr_required_pages = ocr_blockers["requires_ocr_pages"].as_u64().unwrap_or(0);
    let visual_review_pages = visual_blockers["visual_review_pages"].as_u64().unwrap_or(0);
    let blocked_page_count = ocr_required_pages + visual_review_pages;
    let automatic_rewrite_ready = response
        .pointer("/metadata/redaction_rewrite_available")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
        && blocked_page_count == 0;

    let mut next_steps = Vec::new();
    if ocr_required_pages > 0 {
        next_steps.push("Run OCR outside this tool before attempting PDF rewrite.");
    }
    if visual_review_pages > 0 {
        next_steps.push("Review visual-only pages manually before exporting a redacted PDF.");
    }
    if automatic_rewrite_ready {
        next_steps.push("PDF review metadata indicates rewrite readiness; verify output before release.");
    } else {
        next_steps.push("Keep this PHI-safe report with the case audit trail.");
    }

    serde_json::json!({
        "automatic_rewrite_ready": automatic_rewrite_ready,
        "blocked_page_count": blocked_page_count,
        "ocr_required_pages": ocr_required_pages,
        "visual_review_pages": visual_review_pages,
        "next_steps": next_steps,
    })
}
```

Add to `build_desktop_pdf_review_report` JSON:

```rust
"actionability": sanitize_desktop_pdf_review_actionability(response),
```

- [ ] **Step 4: Run desktop tests to verify GREEN**

Run: `cargo test -p mdid-desktop pdf_review_report_save_includes_actionability -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Run broader desktop PDF report tests**

Run: `cargo test -p mdid-desktop pdf_review_report -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Commit desktop slice**

Run:

```bash
git add crates/mdid-desktop/src/lib.rs
git commit -m "feat(desktop): add pdf review actionability"
```

### Task 3: README Completion Truth-Sync

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update current completion snapshot**

Edit the top completion snapshot to state that Browser/Web and Desktop gained PDF review actionability artifacts. Use these current percentages after verification:

```markdown
| Browser/web | 93% | Browser/web now includes PHI-safe PDF review actionability summaries in successful PDF review report downloads ... |
| Desktop app | 93% | Desktop now includes matching PHI-safe PDF review actionability summaries in PDF review report save helpers ... |
| Overall | 95% | ... |
```

- [ ] **Step 2: Run documentation and workspace checks**

Run:

```bash
cargo fmt --check
git diff --check
```

Expected: PASS.

- [ ] **Step 3: Commit README truth-sync**

Run:

```bash
git add README.md
git commit -m "docs: truth-sync pdf review actionability progress"
```
