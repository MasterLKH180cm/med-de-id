# PPM Image Redaction Verification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded visual verification artifact for existing PPM P6 bbox-driven image pixel redaction exports.

**Architecture:** Reuse the existing in-memory PPM redaction path and CLI `redact-image-ppm` command. Add PHI-safe verification metadata that proves output bytes differ only in approved regions by reporting deterministic aggregate pixel counts/checksum evidence, without raw filenames, local paths, image bytes, bbox arrays, OCR text, or PDF/media claims.

**Tech Stack:** Rust `mdid-adapters` byte-level PPM redaction helpers; Rust `mdid-cli` smoke tests and JSON summary output; README truth-sync after verified behavior lands.

---

## File Structure

- Modify `crates/mdid-adapters/src/image_redaction.rs`: expose a PHI-safe `PpmRedactionVerification` summary returned with PPM byte redaction.
- Modify `crates/mdid-adapters/tests/image_redaction_adapter.rs`: test verification counts for redacted vs unchanged pixels and fail-closed out-of-bounds behavior.
- Modify `crates/mdid-cli/src/main.rs`: include verification metadata in `redact-image-ppm` summary output.
- Modify `crates/mdid-cli/tests/cli_smoke.rs`: assert the CLI summary includes visual verification metadata and omits raw image/source details.
- Modify `README.md`: truth-sync only the bounded PPM verification loop.

### Task 1: Adapter visual verification metadata

**Files:**
- Modify: `crates/mdid-adapters/src/image_redaction.rs`
- Test: `crates/mdid-adapters/tests/image_redaction_adapter.rs`

- [ ] **Step 1: Write failing adapter tests**

Add a test that calls the new helper on a tiny 3x2 PPM image with one 2-pixel approved region. Assert the returned verification has `format: "ppm_p6"`, width 3, height 2, `redacted_region_count: 1`, `redacted_pixel_count: 2`, `unchanged_pixel_count: 4`, `output_byte_count` equal to output length, and `verified_changed_pixels_within_regions: true`. Add an out-of-bounds test proving no verification artifact is returned on failure.

- [ ] **Step 2: Run RED**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-adapters --test image_redaction_adapter ppm_visual_verification -- --nocapture`
Expected: FAIL because verification helper/types do not exist.

- [ ] **Step 3: Implement minimal adapter verification support**

Add a serializable verification struct and `redact_ppm_p6_bytes_with_verification(input, regions, fill)` that delegates to the existing parser/redactor, counts distinct approved pixels inside image bounds, and returns `(Vec<u8>, PpmRedactionVerification)`. Do not include source names, paths, bbox arrays, or raw bytes in the struct.

- [ ] **Step 4: Run GREEN and adapter regression**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-adapters --test image_redaction_adapter ppm -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit adapter slice**

Run: `git add crates/mdid-adapters/src/image_redaction.rs crates/mdid-adapters/tests/image_redaction_adapter.rs && git commit -m "feat(image): add PPM redaction verification metadata"`

### Task 2: CLI summary visual verification metadata

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/cli_smoke.rs`

- [ ] **Step 1: Write failing CLI smoke test**

Extend/add a `redact_image_ppm` smoke test that invokes `mdid-cli redact-image-ppm` and asserts `summary.visual_verification.format == "ppm_p6"`, `redacted_pixel_count == 2`, `unchanged_pixel_count == 2` for a 2x2 image with a one-column region, `verified_changed_pixels_within_regions == true`, `output_byte_count` equals the output byte length, and the summary text does not contain the input filename, output filename, raw PPM header, image bytes, or region JSON.

- [ ] **Step 2: Run RED**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-cli --test cli_smoke redact_image_ppm -- --nocapture`
Expected: FAIL because CLI summary lacks `visual_verification`.

- [ ] **Step 3: Implement minimal CLI wiring**

Change `redact-image-ppm` to call `redact_ppm_p6_bytes_with_verification` and include the returned verification under `visual_verification` in the existing PHI-safe summary.

- [ ] **Step 4: Run GREEN and targeted regression**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-cli --test cli_smoke redact_image_ppm -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit CLI slice**

Run: `git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs && git commit -m "feat(cli): include PPM redaction verification summary"`

### Task 3: README truth-sync and final verification

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Run final verification**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-adapters --test image_redaction_adapter ppm -- --nocapture && cargo test -p mdid-cli --test cli_smoke redact_image_ppm -- --nocapture`
Expected: PASS.

- [ ] **Step 2: Update README evidence**

Add one paragraph stating that bounded PPM P6 image export now includes PHI-safe visual verification counts/checksum-style aggregate metadata proving changed pixels are inside approved regions. Do not claim PNG/JPEG/PDF/video, OCR, handwriting, Browser/Desktop, installer, or field-validation completion.

- [ ] **Step 3: Commit docs**

Run: `git add README.md docs/superpowers/plans/2026-05-02-ppm-image-redaction-verification.md && git commit -m "docs: truth-sync PPM visual verification evidence"`
