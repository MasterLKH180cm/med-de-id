# PPM Image Redaction Export Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded, real byte-level image redaction/export path for binary PPM (P6) images using approved bbox regions.

**Architecture:** Extend the existing RGB bbox pixel redaction adapter with a tiny PPM P6 decoder/encoder so tests can prove actual input bytes become redacted output bytes without adding a broad image dependency. Expose it through `mdid-cli redact-image-ppm` with PHI-safe JSON summary output and fail-closed bounds validation.

**Tech Stack:** Rust workspace, mdid-adapters, mdid-cli, serde_json, existing `ImageRedactionRegion` model.

---

## File Structure

- Modify `crates/mdid-adapters/src/image_redaction.rs`: add `redact_ppm_p6_bytes(input, regions, fill)` that parses P6 header, applies `redact_rgb_regions`, and returns rewritten PPM bytes.
- Modify `crates/mdid-adapters/tests/image_redaction_adapter.rs`: add RED tests for real PPM byte redaction, preserving non-region pixels, and fail-closed out-of-bounds validation.
- Modify `crates/mdid-cli/src/main.rs`: add `redact-image-ppm --input <path> --regions-json <json> --output <path> --summary-output <path>` command.
- Modify `crates/mdid-cli/tests/cli_smoke.rs`: add CLI smoke covering redacted output bytes and PHI-safe summary.
- Modify `README.md`: truth-sync only the bounded PPM image redaction/export evidence after tests pass.

### Task 1: Adapter PPM P6 byte redaction

**Files:**
- Modify: `crates/mdid-adapters/src/image_redaction.rs`
- Test: `crates/mdid-adapters/tests/image_redaction_adapter.rs`

- [ ] **Step 1: Write failing adapter tests**

Add tests that construct a 2x2 `P6` byte image (`P6\n2 2\n255\n` + four RGB pixels), call `redact_ppm_p6_bytes`, and assert the output bytes contain black fill only in the approved bbox and unchanged bytes elsewhere. Add one test where region `(1,1,2,1)` is outside a 2x2 image and assert the function returns `RegionOutOfBounds` and no raw source names are in the error debug string.

- [ ] **Step 2: Run RED**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-adapters --test image_redaction_adapter ppm -- --nocapture`
Expected: FAIL because `redact_ppm_p6_bytes` is not defined/exported.

- [ ] **Step 3: Implement minimal adapter**

Add a small P6 parser that accepts ASCII whitespace between magic/width/height/maxval, requires maxval 255, validates exact RGB payload length, calls `redact_rgb_regions`, and returns original header plus rewritten RGB bytes. Do not support comments or other PPM variants in this slice.

- [ ] **Step 4: Run GREEN/regression**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-adapters --test image_redaction_adapter -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit**

Run: `git add crates/mdid-adapters/src/image_redaction.rs crates/mdid-adapters/tests/image_redaction_adapter.rs && git commit -m "feat(image): rewrite PPM bytes with bbox redactions"`

### Task 2: CLI redacted PPM export

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/cli_smoke.rs`

- [ ] **Step 1: Write failing CLI smoke**

Add a test that writes `patient-Jane.ppm`, invokes `mdid-cli redact-image-ppm --input <input> --regions-json '[{"x":1,"y":0,"width":1,"height":2}]' --output <output> --summary-output <summary>`, then asserts the output PPM exists, redacted pixel bytes are `[0,0,0]`, other pixels are unchanged, summary JSON has `artifact: "image_redaction_export_summary"`, `format: "ppm_p6"`, `redacted_region_count: 1`, `redacted_pixel_count: 2`, `bytes_written` equals the output length, and the rendered summary does not contain `Jane` or the raw input/output paths.

- [ ] **Step 2: Run RED**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-cli --test cli_smoke redact_image_ppm -- --nocapture`
Expected: FAIL because the command is not recognized.

- [ ] **Step 3: Implement minimal CLI command**

Add command parsing, read input bytes, parse regions with existing `ImageRedactionRegion` serde validation, call `redact_ppm_p6_bytes`, write output bytes, and write PHI-safe summary JSON to `summary-output`.

- [ ] **Step 4: Run GREEN/regression**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-cli --test cli_smoke redact_image_ppm -- --nocapture`
Expected: PASS. Then run adapter targeted test from Task 1 again.

- [ ] **Step 5: Commit**

Run: `git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs && git commit -m "feat(cli): export bbox-redacted PPM images"`

### Task 3: Truth-sync documentation and integration verification

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Run integration verification**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-adapters --test image_redaction_adapter -- --nocapture && cargo test -p mdid-cli --test cli_smoke redact_image_ppm -- --nocapture`
Expected: PASS.

- [ ] **Step 2: Update README without overclaiming**

Add a verification-evidence paragraph stating only that bounded PPM P6 image byte export now exists for explicit bbox regions; do not claim PNG/JPEG/PDF/video or full visual verification completion.

- [ ] **Step 3: Commit docs**

Run: `git add README.md docs/superpowers/plans/2026-05-02-ppm-image-redaction-export.md && git commit -m "docs: truth-sync bounded image redaction export"`
