# Browser PDF Review Flow Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded browser PDF review mode that submits base64 PDF bytes to the existing local runtime `/pdf/deidentify` entry and renders the honest review-only PDF response.

**Architecture:** Extend the existing `mdid-browser` single-page local-first flow rather than adding a new workflow builder. The browser remains a thin runtime client: it builds JSON requests, parses runtime success/error envelopes, and renders summary/page status/review queue without performing OCR, visual redaction, PDF rewrite/export, auth, persistence, or orchestration.

**Tech Stack:** Rust, Leptos, serde/serde_json, existing `mdid-runtime` HTTP JSON contract, Cargo tests.

---

## File Structure

- Modify: `crates/mdid-browser/src/app.rs`
  - Add `InputMode::PdfBase64` with `/pdf/deidentify`, payload hints, and disclosure copy.
  - Add request/response DTOs for the PDF runtime endpoint.
  - Generalize submission validation so field policy JSON is required only for tabular CSV/XLSX modes, while PDF mode requires a base64 PDF payload and a non-blank source name.
  - Render the existing field-policy textarea only for tabular modes and render a source-name input for PDF mode.
  - Parse PDF runtime responses into a browser envelope with honest `rewritten_output` text stating that PDF rewrite/export is unavailable.
  - Format PDF summary/page statuses/review queue without claiming OCR/redaction/rewrite.
- Modify: `README.md`
  - Truth-sync Browser/web completion and current status to include bounded browser PDF review flow.
  - Keep missing items explicit: no OCR, visual redaction, handwriting handling, PDF rewrite/export, browser upload UX, desktop PDF flow, auth/session, or generalized workflow orchestration.

### Task 1: Browser PDF Runtime Review Mode

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`
- Test: `crates/mdid-browser/src/app.rs` unit tests module

- [x] **Step 1: Write failing tests for PDF mode request building and parsing**

Add these tests to the existing `#[cfg(test)] mod tests` in `crates/mdid-browser/src/app.rs`:

```rust
#[test]
fn pdf_mode_disclosure_matches_review_only_runtime_limits() {
    assert_eq!(InputMode::PdfBase64.payload_hint(), "Paste base64-encoded PDF content here");
    assert_eq!(
        InputMode::PdfBase64.disclosure_copy(),
        Some("PDF mode is review-only: it reports text-layer candidates and OCR-required pages, but does not perform OCR, visual redaction, handwriting handling, or PDF rewrite/export.")
    );
    assert_eq!(InputMode::PdfBase64.endpoint(), "/pdf/deidentify");
}

#[test]
fn build_submit_request_targets_pdf_endpoint_without_field_policies() {
    let request = build_submit_request(
        InputMode::PdfBase64,
        "JVBERi0xLjQK...\n",
        "Ignored Report.pdf",
        "",
    )
    .unwrap();

    assert_eq!(request.endpoint, "/pdf/deidentify");
    let body: serde_json::Value = serde_json::from_str(&request.body_json).unwrap();
    assert_eq!(body["pdf_bytes_base64"], "JVBERi0xLjQK...");
    assert_eq!(body["source_name"], "Ignored Report.pdf");
    assert!(body.get("policies").is_none());
    assert!(body.get("field_policies").is_none());
}

#[test]
fn pdf_submit_requires_source_name_before_runtime_request() {
    let mut state = TabularFlowState {
        input_mode: InputMode::PdfBase64,
        payload: "JVBERi0xLjQK".to_string(),
        source_name: "   ".to_string(),
        ..TabularFlowState::default()
    };

    let result = state.begin_submit();

    assert!(result.is_err());
    assert_eq!(
        state.error_banner.as_deref(),
        Some("PDF source name is required before submitting.")
    );
}

#[test]
fn parse_pdf_runtime_success_renders_review_only_summary_and_page_statuses() {
    let response = parse_runtime_success(
        InputMode::PdfBase64,
        &json!({
            "summary": {
                "total_pages": 2,
                "text_layer_pages": 1,
                "ocr_required_pages": 1,
                "extracted_candidates": 1,
                "review_required_candidates": 1
            },
            "page_statuses": [
                {"page": {"source_label": "radiology/report.pdf", "page_number": 1}, "status": "text_layer_present"},
                {"page": {"source_label": "radiology/report.pdf", "page_number": 2}, "status": "ocr_required"}
            ],
            "review_queue": [
                {
                    "page": {"source_label": "radiology/report.pdf", "page_number": 1},
                    "source_text": "Alice Smith",
                    "phi_type": "patient_name",
                    "confidence": 0.2,
                    "review_required": true
                }
            ],
            "rewritten_pdf_bytes_base64": null
        })
        .to_string(),
    )
    .unwrap();

    assert_eq!(
        response.rewritten_output,
        "PDF rewrite/export unavailable: runtime returned review-only PDF analysis."
    );
    assert!(response.summary.contains("total_pages: 2"));
    assert!(response.summary.contains("ocr_required_pages: 1"));
    assert!(response.summary.contains("page_statuses:"));
    assert!(response.summary.contains("- page 1 (radiology/report.pdf): text_layer_present"));
    assert!(response.summary.contains("- page 2 (radiology/report.pdf): ocr_required"));
    assert_eq!(
        response.review_queue,
        "- page 1 / patient_name / confidence 0.2: Alice Smith"
    );
}
```

- [x] **Step 2: Run tests to verify RED**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-browser pdf -- --nocapture`

Expected: FAIL because `InputMode::PdfBase64`, `source_name`, and PDF response parsing do not exist yet.

- [x] **Step 3: Implement minimal PDF browser mode**

In `crates/mdid-browser/src/app.rs`, make the minimal production changes required by the tests:

```rust
// Add PdfBase64 to InputMode and wire select_value/label/payload_hint/disclosure_copy/endpoint.
// Add source_name: String to TabularFlowState defaulting to "local-review.pdf".
// Change build_submit_request signature to:
fn build_submit_request(
    input_mode: InputMode,
    payload: &str,
    source_name: &str,
    field_policy_json: &str,
) -> Result<TabularSubmitRequest, String>
// For PdfBase64, require non-blank source_name and serialize:
// {"pdf_bytes_base64": payload.trim(), "source_name": source_name.trim()}
// Add PdfRuntimeSuccessResponse, PdfExtractionSummary, PdfPageStatusResponse,
// PdfPageRef, PdfReviewCandidate DTOs matching the existing runtime JSON.
// In parse_runtime_success, map PdfBase64 to RuntimeResponseEnvelope with summary/review_queue strings.
// Update the Leptos view with a <option value="pdf-base64">"PDF base64"</option>,
// a source-name input shown only in PDF mode, and hide field-policy JSON for PDF mode.
```

- [x] **Step 4: Run targeted browser tests**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-browser pdf -- --nocapture`

Expected: PASS.

- [x] **Step 5: Run full browser crate tests**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-browser`

Expected: PASS.

- [x] **Step 6: Commit**

```bash
git add crates/mdid-browser/src/app.rs docs/superpowers/plans/2026-04-28-browser-pdf-review-flow.md
git commit -m "feat(browser): add bounded pdf review flow"
```

### Task 2: README Completion Truth Sync

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update README completion snapshot**

Update the completion table and current-status bullets to truthfully include the browser PDF review mode. Use these completion numbers unless controller-visible implementation differs:

```markdown
| Browser/web | 30% | Bounded localhost tabular de-identification page plus bounded PDF review mode backed by local runtime routes; not a broader browser governance workspace. |
| Overall | 39% | Core workspace, vault MVP, tabular path, bounded DICOM/PDF/runtime slices, conservative media/FCS domain models, adapter/application review foundation, bounded runtime metadata review and PDF review entries, browser tabular/PDF review surface, and local CLI foundations are present; major workflow depth and surface parity remain missing; scope-drift controller/moat CLI wording is not counted as core product progress. |
```

Also update the missing-items sentence to keep `browser upload UX`, `desktop PDF flow`, `OCR`, `visual redaction`, `handwriting handling`, and `full PDF rewrite/export` listed as missing.

- [ ] **Step 2: Verify README wording and no scope-drift expansion**

Run: `grep -nE "Browser/web|Overall|mdid-browser|PDF|moat|controller|agent|orchestration" README.md`

Expected: Browser/web is 30%, Overall is 39%, PDF browser mode is review-only, and any moat/controller/agent/orchestration wording is negative/scope-drift only.

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: update browser pdf completion snapshot"
```

### Task 3: Integration Review and Merge

**Files:**
- Verify: `crates/mdid-browser/src/app.rs`, `README.md`

- [ ] **Step 1: Run final verification**

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-browser
cargo test -p mdid-runtime --test runtime_http pdf -- --nocapture
git diff --check
git status --short
```

Expected: tests pass, diff check passes, and only expected branch commits exist.

- [ ] **Step 2: Merge to develop**

```bash
git checkout develop
git merge --no-ff feature/browser-pdf-review-flow -m "merge: add browser pdf review flow"
```

- [ ] **Step 3: Verify develop state**

```bash
git branch --show-current
git status --short
git log --oneline -8 --decorate
```

Expected: on `develop`, clean worktree, merge commit at HEAD.

---

## Self-Review

- Spec coverage: This plan covers the high-leverage browser PDF flow gap by reusing the already-landed runtime PDF review endpoint and updating README completion. It intentionally does not implement OCR, visual redaction, rewrite/export, desktop PDF flow, auth/session, uploads, or generalized workflow orchestration.
- Placeholder scan: No TBD/TODO/fill-in-later placeholders remain; code-facing steps include exact tests, commands, and expected results.
- Type consistency: The plan consistently uses `InputMode::PdfBase64`, `source_name`, `/pdf/deidentify`, `pdf_bytes_base64`, and `rewritten_pdf_bytes_base64` matching the existing runtime contract.
