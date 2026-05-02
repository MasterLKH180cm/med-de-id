# Browser PPM Visual Redaction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded Browser/Web mode that submits explicit PPM P6 bytes plus approved bbox regions to the existing local `/visual-redaction/ppm` runtime endpoint and prepares a redacted PPM download with PHI-safe verification metadata.

**Architecture:** Reuse the existing runtime/application PPM visual redaction endpoint; the browser only adds input-mode plumbing, file import, request construction, clean redacted-byte download, and truthful disclosure. This is a bounded de-identification workflow surface for explicit bbox PPM redaction, not OCR, automatic detection, PDF/video redaction, or a generic orchestration platform.

**Tech Stack:** Rust, Leptos browser crate, serde_json, base64, existing `mdid-runtime` `/visual-redaction/ppm` contract.

---

## File Structure

- Modify `crates/mdid-browser/src/app.rs`: add `VisualRedactionPpm` input mode, route metadata, PPM file import behavior, JSON request builder, response download gating/decoding, and tests.
- Modify `README.md`: truth-sync current repository-visible evidence after tests pass.

### Task 1: Browser Visual Redaction PPM Mode

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`
- Test: `crates/mdid-browser/src/app.rs` existing `#[cfg(test)]` module

- [ ] **Step 1: Write failing tests**

Add tests proving: `.ppm` imports use base64/data-url mode; select value maps to the new mode; endpoint is `/visual-redaction/ppm`; disclosure states PPM-only explicit bbox scope; request JSON wraps imported base64 and bbox regions; successful responses expose a redacted PPM download; review/error responses do not expose downloads.

- [ ] **Step 2: Run RED**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-browser visual_redaction_ppm -- --nocapture`

Expected: FAIL because `VisualRedactionPpm` mode and helper functions do not exist yet.

- [ ] **Step 3: Implement minimal browser mode**

Update `InputMode`, match arms, `browser_file_read_mode`, and request-payload construction so the browser submits JSON `{ "ppm_bytes_base64": <payload>, "regions": [...] }` to the existing local runtime endpoint. Use a bounded default region JSON array suitable for explicit operator-approved bboxes, and keep disclosure truthful: PPM P6 only, explicit bbox only, no OCR/automatic visual detection/PDF/video/Desktop.

- [ ] **Step 4: Implement download gating**

Add helper to decode `rewritten_ppm_bytes_base64` only when runtime response has `verification.format == "ppm_p6"`, nonzero `redacted_region_count`, and `verified_changed_pixels_within_regions == true`. The prepared download must be `image/x-portable-pixmap`, binary, filename `mdid-browser-redacted.ppm`, and must not include bbox arrays or source names in any summary.

- [ ] **Step 5: Run targeted GREEN**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-browser visual_redaction_ppm -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Run regression**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-browser pdf_clean_text_layer_export visual_redaction_ppm -- --nocapture && cargo fmt --check && git diff --check`

Expected: PASS.

- [ ] **Step 7: Commit**

Run: `git add crates/mdid-browser/src/app.rs docs/superpowers/plans/2026-05-02-browser-ppm-visual-redaction.md && git commit -m "feat(browser): add bounded PPM visual redaction mode"`

### Task 2: README Truth Sync

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update README evidence**

Add a concise verification paragraph stating Browser/Web can now submit explicit PPM P6 bbox redaction requests to the existing local runtime and download redacted PPM bytes when verification passes. Explicitly state non-goals: no OCR, automatic visual detection, PNG/JPEG/PDF/video rewrite, Desktop execution, packaging, or model-quality proof. Do not increase displayed completion above the already capped 99%.

- [ ] **Step 2: Verify docs and tests**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-browser visual_redaction_ppm -- --nocapture && git diff --check`

Expected: PASS.

- [ ] **Step 3: Commit**

Run: `git add README.md && git commit -m "docs(browser): truth-sync PPM visual redaction mode"`

## Self-Review

- Spec coverage: Advances priority item 2 and item 8 with actual browser submission/download for bounded PPM visual redaction using already-landed runtime; does not overclaim unsupported formats.
- Placeholder scan: No TBD/TODO/placeholders.
- Type consistency: New mode name is `PpmVisualRedaction`; endpoint is `/visual-redaction/ppm`; response field is `rewritten_ppm_bytes_base64`.
