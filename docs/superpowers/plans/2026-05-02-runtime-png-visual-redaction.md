# Runtime PNG Visual Redaction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expose the already-landed bounded PNG explicit-bbox byte rewrite/export through the application service and local runtime HTTP surface.

**Architecture:** Reuse the existing PPM visual redaction service/runtime route pattern, but add PNG-specific request/response fields and strict verification gating with `format: "png"`. This is local-only explicit bbox redaction for operator-provided regions; it must not claim OCR, automatic visual detection, JPEG/PDF/video rewrite, Browser/Desktop execution, packaging, or field validation.

**Tech Stack:** Rust workspace, mdid-domain `ImageRedactionRegion`, mdid-adapters PNG redaction helper, mdid-application service layer, mdid-runtime Axum HTTP tests.

---

## File Structure

- Modify `crates/mdid-application/src/lib.rs`: add `VisualRedactionService::redact_png_bytes` parallel to `redact_ppm_p6_bytes`, mapping empty regions/malformed PNG/out-of-bounds to existing PHI-safe error taxonomy.
- Modify `crates/mdid-application/tests/visual_redaction.rs`: add PNG service RED/GREEN tests using in-memory tiny PNG bytes.
- Modify `crates/mdid-runtime/src/http.rs`: add `POST /visual-redaction/png` accepting `{ "png_bytes_base64": "...", "regions": [...] }` and returning `{ "rewritten_png_bytes_base64": "...", "verification": ... }`.
- Modify `crates/mdid-runtime/tests/runtime_http.rs`: add endpoint success and PHI-safe rejection tests.
- Modify `README.md`: truth-sync only after tests pass, stating runtime PNG route exists and remains bounded.

### Task 1: Application service PNG visual redaction

**Files:**
- Modify: `crates/mdid-application/src/lib.rs`
- Test: `crates/mdid-application/tests/visual_redaction.rs`

- [x] **Step 1: Write failing application tests**

Add tests that construct a 2x1 PNG in memory, call `VisualRedactionService.redact_png_bytes(&png_bytes, &[region])`, assert `verification.format == "png"`, output bytes are non-empty and not equal to input, and malformed PNG text containing `Patient Jane Example.png` maps to a PHI-safe visual redaction error without echoing the name.

- [x] **Step 2: Run RED**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-application png --test visual_redaction -- --nocapture`
Expected: FAIL because `redact_png_bytes` is not implemented.

- [x] **Step 3: Implement minimal service method**

Call the existing adapter PNG helper (`redact_png_bytes_with_verification`) with opaque black fill, copy the verification fields into `VisualRedactionVerification`, reject empty regions before adapter calls, and keep `Debug` redacting output bytes.

- [x] **Step 4: Run GREEN**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-application png --test visual_redaction -- --nocapture`
Expected: PASS.

- [x] **Step 5: Commit**

Run: `git add crates/mdid-application/src/lib.rs crates/mdid-application/tests/visual_redaction.rs && git commit -m "feat(application): add png visual redaction service"`

### Task 2: Runtime PNG visual redaction endpoint and README truth-sync

**Files:**
- Modify: `crates/mdid-runtime/src/http.rs`
- Test: `crates/mdid-runtime/tests/runtime_http.rs`
- Modify: `README.md`

- [x] **Step 1: Write failing runtime tests**

Add a success test for `POST /visual-redaction/png` with base64 tiny PNG bytes and one bbox region. Assert status 200, `rewritten_png_bytes_base64` decodes to non-empty bytes different from input, `verification.format == "png"`, and the response JSON does not contain source names, bbox arrays, or raw PHI sentinels. Add rejection cases for malformed base64 and malformed PNG bytes containing `Patient Jane Example.png`; both must return `422 invalid_visual_redaction` without echoing the payload.

- [x] **Step 2: Run RED**

Run: `source "$HOME/.cargo/env" && CARGO_BUILD_JOBS=1 cargo test -p mdid-runtime visual_redaction_png -- --nocapture`
Expected: FAIL because route/request/response are not implemented.

- [x] **Step 3: Implement minimal endpoint**

Add PNG request/response structs, register `.route("/visual-redaction/png", post(visual_redaction_png))`, decode base64 using the same engine as PPM, call `VisualRedactionService.redact_png_bytes`, return JSON only on success, and use existing `invalid_visual_redaction_response()` for all invalid inputs.

- [x] **Step 4: Run GREEN and regressions**

Run: `source "$HOME/.cargo/env" && CARGO_BUILD_JOBS=1 cargo test -p mdid-runtime visual_redaction_png -- --nocapture && cargo test -p mdid-application --test visual_redaction -- --nocapture && cargo fmt --check && git diff --check`
Expected: PASS.

- [x] **Step 5: README truth-sync**

Update the Images row/current status with one bounded sentence: runtime now exposes local `POST /visual-redaction/png` for explicit bbox PNG byte rewrite/export. Explicitly preserve non-goals: no OCR, automatic visual detection, JPEG/PDF/video rewrite, Browser/Desktop PNG execution, packaging, or field validation.

- [x] **Step 6: Commit**

Run: `git add crates/mdid-runtime/src/http.rs crates/mdid-runtime/tests/runtime_http.rs README.md docs/superpowers/plans/2026-05-02-runtime-png-visual-redaction.md && git commit -m "feat(runtime): expose png visual redaction endpoint"`

## Self-Review

Spec coverage: advances priority items 2, 4, and 8/9 foundation by making existing PNG byte rewrite/export available through application/runtime, but not Browser/Desktop execution yet. Placeholder scan: no TBD/TODO/fill-in placeholders. Type consistency: `VisualRedactionVerification`, `ImageRedactionRegion`, `invalid_visual_redaction`, and runtime base64 route names match existing PPM patterns.
