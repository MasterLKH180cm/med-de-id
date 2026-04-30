# Cross-Surface PDF OCR Blocker Evidence Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add PHI-safe OCR/visual-review blocker evidence to Browser/Web and Desktop PDF review report artifacts so users can see why PDF review remains review-only without exposing raw PDF text or payloads.

**Architecture:** Build a small allowlisted `ocr_blockers` report object from existing PDF runtime response fields and already-sanitized page statuses. Browser and Desktop keep separate helper implementations in their existing report builders, but use the same JSON shape: `ocr_blockers: { requires_ocr_pages, visual_review_pages, blocked_page_count, rewrite_available }`. The slice only enriches already-successful PDF review reports; it does not add OCR, visual redaction, PDF rewrite/export, or broader workflow behavior.

**Tech Stack:** Rust workspace; `serde_json`; existing `mdid-browser` and `mdid-desktop` crate unit tests; Cargo test/clippy verification.

---

## File Structure

- Modify: `crates/mdid-browser/src/app.rs`
  - Add a pure helper near `sanitized_pdf_page_statuses` that derives `ocr_blockers` from a PDF runtime response object.
  - Include the helper output in `build_pdf_review_report_download`.
  - Add browser unit tests near existing PDF review report download tests.
- Modify: `crates/mdid-desktop/src/lib.rs`
  - Add a pure helper near `sanitize_desktop_pdf_page_statuses` that derives the same `ocr_blockers` JSON shape.
  - Include the helper output in `build_desktop_pdf_review_report`.
  - Add desktop unit tests near existing review report download tests.
- Modify: `README.md`
  - Truth-sync completion snapshot and missing items after verified cross-surface report enrichment.

### Task 1: Browser PDF OCR blocker evidence in report downloads

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`

- [x] **Step 1: Write the failing browser tests**

Add tests that create a successful PDF review response containing `page_statuses`, `summary`, raw sensitive fields, and `no_rewritten_pdf: true`, then call the existing PDF review report download path. The expected report must include:

```json
"ocr_blockers": {
  "requires_ocr_pages": 1,
  "visual_review_pages": 1,
  "blocked_page_count": 2,
  "rewrite_available": false
}
```

Also assert the serialized report does not contain raw text, bounding boxes, PDF bytes, filenames, or arbitrary nested sensitive fields.

- [x] **Step 2: Run browser RED verification**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-browser pdf_review_report_ocr_blockers -- --nocapture`

Expected: FAIL because `ocr_blockers` is not present in the generated PDF review report JSON.

- [x] **Step 3: Implement minimal browser helper and report field**

Add a helper with this behavior:

```rust
fn sanitized_pdf_ocr_blockers(response: &serde_json::Value) -> serde_json::Value {
    let page_statuses = response
        .get("page_statuses")
        .and_then(serde_json::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let requires_ocr_pages = page_statuses
        .iter()
        .filter(|item| item.get("requires_ocr").and_then(serde_json::Value::as_bool) == Some(true))
        .count();
    let visual_review_pages = page_statuses
        .iter()
        .filter(|item| {
            item.get("status")
                .and_then(serde_json::Value::as_str)
                .map(|status| status.eq_ignore_ascii_case("visual_review_required"))
                == Some(true)
        })
        .count();
    serde_json::json!({
        "requires_ocr_pages": requires_ocr_pages,
        "visual_review_pages": visual_review_pages,
        "blocked_page_count": distinct_blocked_page_count(page_statuses),
        "rewrite_available": response.get("no_rewritten_pdf").and_then(serde_json::Value::as_bool) == Some(false),
    })
}
```

Then add `"ocr_blockers": sanitized_pdf_ocr_blockers(&response)` to the PDF review report JSON.

- [x] **Step 4: Run browser GREEN verification**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-browser pdf_review_report_ocr_blockers -- --nocapture`

Expected: PASS.

- [x] **Step 5: Run browser broader verification**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-browser pdf_review_report -- --nocapture`

Expected: PASS.

- [x] **Step 6: Commit browser slice**

Run:

```bash
git add crates/mdid-browser/src/app.rs docs/superpowers/plans/2026-04-30-cross-surface-pdf-ocr-blocker-evidence.md
git commit -m "feat(browser): add pdf ocr blocker report evidence"
```

### Task 2: Desktop PDF OCR blocker evidence in report saves

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`

- [x] **Step 1: Write the failing desktop tests**

Add tests that apply a successful `DesktopWorkflowMode::PdfBase64Review` response containing `page_statuses`, `summary`, raw sensitive fields, and `no_rewritten_pdf: true`, then call the existing review report download/save helper. The expected report must include the same PHI-safe `ocr_blockers` object as Browser/Web and must omit raw text, bounding boxes, PDF bytes, filenames, passphrases, and arbitrary nested sensitive fields.

- [x] **Step 2: Run desktop RED verification**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-desktop pdf_review_report_ocr_blockers -- --nocapture`

Expected: FAIL because `ocr_blockers` is not present in the generated PDF review report JSON.

- [x] **Step 3: Implement minimal desktop helper and report field**

Add a helper near `sanitize_desktop_pdf_page_statuses` that derives the exact same JSON shape and values as the browser helper from the response object. Then add `"ocr_blockers": sanitize_desktop_pdf_ocr_blockers(Some(response))` to the desktop PDF review report JSON.

- [x] **Step 4: Run desktop GREEN verification**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-desktop pdf_review_report_ocr_blockers -- --nocapture`

Expected: PASS.

- [x] **Step 5: Run desktop broader verification**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-desktop review_report_download -- --nocapture`

Expected: PASS.

- [x] **Step 6: Commit desktop slice**

Run:

```bash
git add crates/mdid-desktop/src/lib.rs
git commit -m "feat(desktop): add pdf ocr blocker report evidence"
```

### Task 3: README truth-sync and cross-surface verification

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/plans/2026-04-30-cross-surface-pdf-ocr-blocker-evidence.md`

- [x] **Step 1: Update README completion snapshot**

Update the current repository status to state that Browser/Web and Desktop PDF review report artifacts now include `ocr_blockers` evidence. Keep completion honest: Browser/Web was already at 100%, so it remains 100%; Desktop moves from 97% to 100% if the desktop slice and verification land; Overall moves from 98% to 99% because PDF review artifact evidence is improved but full OCR, visual redaction, and rewritten PDF export remain missing. Keep CLI at 95%.

- [x] **Step 2: Mark this plan's completed checkboxes**

Change completed task checkboxes from `- [x]` to `- [x]` only for steps actually completed and verified.

- [x] **Step 3: Run final verification**

Run:

```bash
source "$HOME/.cargo/env" && cargo test -p mdid-browser pdf_review_report_ocr_blockers -- --nocapture
source "$HOME/.cargo/env" && cargo test -p mdid-desktop pdf_review_report_ocr_blockers -- --nocapture
source "$HOME/.cargo/env" && cargo test -p mdid-browser pdf_review_report -- --nocapture
source "$HOME/.cargo/env" && cargo test -p mdid-desktop review_report_download -- --nocapture
source "$HOME/.cargo/env" && cargo fmt --check
git diff --check
```

Expected: all commands PASS.

- [x] **Step 4: Commit docs truth-sync**

Run:

```bash
git add README.md docs/superpowers/plans/2026-04-30-cross-surface-pdf-ocr-blocker-evidence.md
git commit -m "docs: truth-sync pdf ocr blocker evidence"
```
