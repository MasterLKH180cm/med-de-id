# PDF Clean Text-Layer Export Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded CLI/application PDF rewrite/export path that writes validated PDF bytes only when text-layer extraction proves there are no review candidates or OCR/visual blockers.

**Architecture:** Extend `PdfDeidentificationService` to return copied `rewritten_pdf_bytes` for candidate-free text-layer PDFs with no OCR-required pages; keep candidate-bearing/scanned PDFs review-only. Extend `mdid-cli deidentify-pdf` with optional `--output-pdf-path` that writes those bytes and reports PHI-safe rewrite validation metadata.

**Tech Stack:** Rust workspace (`mdid-application`, `mdid-cli`), serde JSON reports, cargo integration tests.

---

## File Structure

- Modify `crates/mdid-application/src/lib.rs`: set `rewrite_status`, `no_rewritten_pdf`, `review_only`, and `rewritten_pdf_bytes` for clean text-layer PDFs only.
- Modify `crates/mdid-cli/src/main.rs`: add `output_pdf_path: Option<PathBuf>` parsing and write export bytes when available; fail closed when `--output-pdf-path` is requested but rewrite bytes are unavailable.
- Modify `crates/mdid-cli/tests/cli_pdf.rs`: add strict TDD tests for clean PDF byte export and blocked candidate-bearing export.

### Task 1: Bounded clean PDF byte export

**Files:**
- Modify: `crates/mdid-application/src/lib.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/cli_pdf.rs`

- [ ] **Step 1: Write failing tests**

Add tests that create a minimal clean PDF byte fixture inside the test, invoke `mdid-cli deidentify-pdf --output-pdf-path`, and assert the output PDF exists, starts with `%PDF`, report has `rewrite_available: true`, `no_rewritten_pdf: false`, `review_only: false`, `rewritten_pdf_bytes: "<written-to-output-pdf-path>"`, and no raw source path/PHI leaks. Add a second test using the existing PHI-bearing `text-layer-minimal.pdf` fixture and `--output-pdf-path`, asserting the command fails closed, does not create output bytes, and stderr says `PDF rewrite/export unavailable for this input` without raw PHI or agent/controller/orchestration terms.

- [ ] **Step 2: Run RED**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-cli --test cli_pdf cli_deidentify_pdf_exports_clean_text_layer_pdf_bytes cli_deidentify_pdf_refuses_output_pdf_for_review_queue_candidates -- --nocapture`
Expected: FAIL because `--output-pdf-path` is unknown and export behavior is not implemented.

- [ ] **Step 3: Implement minimal application behavior**

In `PdfDeidentificationService::deidentify_bytes`, after extraction, compute rewrite availability as candidate-free and no page status requiring OCR/visual review. For that case only, return the original bytes as `rewritten_pdf_bytes`, set `rewrite_status` to an existing serialized status appropriate for rewrite-ready if available or add a narrowly named enum variant in the domain if required, `no_rewritten_pdf: false`, and `review_only: false`. Otherwise keep the current review-only values.

- [ ] **Step 4: Implement CLI export**

Add `--output-pdf-path` parsing. If provided and `rewritten_pdf_bytes` is `Some`, write the bytes to that path and set report/stdout rewrite metadata. If provided and bytes are unavailable, return `Err("PDF rewrite/export unavailable for this input")` before creating the output file. Do not serialize raw PDF bytes into JSON.

- [ ] **Step 5: Run GREEN and regression**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-cli --test cli_pdf -- --nocapture`
Expected: PASS.

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-application -p mdid-cli --tests`
Expected: PASS or document environmental failures truthfully.

- [ ] **Step 6: Commit**

Run:
```bash
git add crates/mdid-application/src/lib.rs crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_pdf.rs docs/superpowers/plans/2026-05-02-pdf-clean-text-layer-export.md
git commit -m "feat(pdf): export clean text-layer PDF bytes"
```
