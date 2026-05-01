# Browser Clean PDF Export Download Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded Browser/Web download path for clean text-layer PDF export evidence when the runtime returns rewritten PDF bytes.

**Architecture:** Reuse the existing `/pdf/deidentify` runtime response shape and keep the existing PDF review report download separate. Browser state preserves clean/exportable runtime JSON internally for sanitized export only, hides raw JSON from visible UI, and gates the clean export download on an empty review queue plus explicit rewrite availability flags.

**Tech Stack:** Rust workspace (`mdid-browser`), serde JSON payloads, existing browser unit tests.

---

## File Structure

- Modify: `crates/mdid-browser/src/app.rs` — add clean text-layer PDF export evidence builder, browser-state availability gate, dedicated download handler/button, PDF UI PHI-safety sanitation, and regression tests.
- Test: `crates/mdid-browser/src/app.rs` test module — cover clean export payloads, blocked export states, runtime-to-browser state preservation, button gating, and PHI-safe visible PDF UI fields.

### Task 1: Browser clean text-layer PDF export download

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`
- Test: `crates/mdid-browser/src/app.rs`

- [x] **Step 1: Write failing tests for clean export payload and blockers**

Add tests named:
- `browser_pdf_clean_text_layer_export_includes_rewritten_pdf_bytes`
- `browser_pdf_clean_text_layer_export_blocks_review_queue`
- `pdf_clean_text_layer_export_blocks_no_rewrite_review_only_missing_and_null_bytes`

The tests construct runtime-shaped JSON with `review_queue`, `no_rewritten_pdf`, `review_only`, and `rewritten_pdf_bytes_base64` and assert rewritten bytes are included only for clean/exportable responses.

- [x] **Step 2: Run RED**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-browser pdf_clean_text_layer_export -- --nocapture`

Expected: FAIL before implementation because the clean export helper and browser-state route do not exist.

- [x] **Step 3: Implement clean export evidence builder**

Add `build_pdf_clean_text_layer_export_download(response_json: &str)` that returns a fixed-name PHI-safe JSON download payload. The output allowlists only artifact/mode, safe aggregate summary fields, `rewrite_available`, `review_only`, `no_rewritten_pdf`, `review_queue_empty`, and `rewritten_pdf_bytes_base64` when clean/exportable.

- [x] **Step 4: Wire actual browser download flow**

Add browser state helpers for clean export availability and payload preparation, plus a dedicated PDF-mode UI download button. Preserve existing PDF review report download behavior.

- [x] **Step 5: Preserve clean runtime response without UI PHI exposure**

Update PDF runtime parsing/state display so clean/exportable JSON is internally available for sanitized export, while browser-visible PDF JSON is withheld. Ensure summary/page status/review queue formatting does not display source labels, filenames, raw `source_text`, or raw OCR/PHI.

- [x] **Step 6: Run GREEN and regression**

Run:

```bash
source "$HOME/.cargo/env" && cargo test -p mdid-browser pdf_clean_text_layer_export -- --nocapture
source "$HOME/.cargo/env" && cargo test -p mdid-browser pdf_ -- --nocapture
source "$HOME/.cargo/env" && cargo fmt --check
git diff --check
```

Expected: PASS.

- [x] **Step 7: Review and commit**

SDD spec review: PASS.  
SDD quality review: APPROVED.

Commits:
- `7fd4a86 feat(browser): add clean PDF export download evidence`
- `0e81ab2 fix(browser): wire clean PDF export download`
- `8204053 fix(browser): preserve clean PDF export runtime response`
- `972526e fix(browser): sanitize PDF export UI state`
