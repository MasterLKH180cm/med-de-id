# PNG Image Redaction Export Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add bounded actual PNG byte-level redaction/export for explicit bbox regions, reusing the existing visual redaction verification contract and failing closed for unsupported/malformed images.

**Architecture:** Keep the first slice adapter/CLI-scoped and local-only: mdid-adapters decodes PNG bytes, applies existing `ImageRedactionRegion` bbox fills to RGBA pixels, re-encodes PNG, and returns PHI-safe aggregate verification. mdid-cli exposes a `redact-image-png` automation command parallel to existing `redact-image-ppm`, with no OCR, automatic detection, JPEG/PDF/video claims, or raw bbox/source leaks in the summary.

**Tech Stack:** Rust workspace, `image` crate for PNG decode/encode, mdid-domain bbox model, mdid-adapters image redaction helpers, mdid-cli clap-style command parser, cargo tests.

---

## File Structure

- Modify `Cargo.toml`: add workspace dependency `image = { version = "0.25", default-features = false, features = ["png"] }`.
- Modify `crates/mdid-adapters/Cargo.toml`: depend on workspace `image`.
- Modify `crates/mdid-adapters/src/image_redaction.rs`: add PNG decode/redact/re-encode helper and tests.
- Modify `crates/mdid-adapters/src/lib.rs`: export PNG helper types/functions if needed.
- Modify `crates/mdid-cli/src/main.rs`: add `redact-image-png` command parallel to `redact-image-ppm`.
- Modify `crates/mdid-cli/tests/cli_smoke.rs`: add CLI smoke tests for PNG output bytes and PHI-safe summary.
- Modify `README.md`: truth-sync the Images row and current status with bounded PNG explicit-bbox byte export, without claiming JPEG/PDF/video/OCR/automatic detection.

### Task 1: Adapter PNG explicit-bbox byte rewrite

**Files:**
- Modify: `Cargo.toml`
- Modify: `crates/mdid-adapters/Cargo.toml`
- Modify: `crates/mdid-adapters/src/image_redaction.rs`
- Test: `crates/mdid-adapters/src/image_redaction.rs`

- [ ] **Step 1: Write failing adapter tests**

Add tests that construct a 2x1 PNG in memory, redact bbox `{ x: 0, y: 0, width: 1, height: 1 }`, decode output, and assert only the approved pixel changes. Add malformed PNG and out-of-bounds tests.

- [ ] **Step 2: Run RED**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-adapters png --test image_redaction_adapter -- --nocapture`
Expected: FAIL because PNG helper/summary does not exist yet.

- [ ] **Step 3: Implement minimal adapter support**

Use `image::load_from_memory_with_format(..., image::ImageFormat::Png)` or equivalent, convert to RGBA8, apply existing region validation/mask logic, re-encode as PNG, and return the same verification style with `format: "png"`, dimensions, redacted region count, distinct redacted pixel count, unchanged pixel count, output byte count, and `verified_changed_pixels_within_regions: true`.

- [ ] **Step 4: Run GREEN**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-adapters png --test image_redaction_adapter -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Broader adapter regression**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-adapters --test image_redaction_adapter -- --nocapture`
Expected: PASS.

- [ ] **Step 6: Commit**

Run: `git add Cargo.toml crates/mdid-adapters/Cargo.toml crates/mdid-adapters/src/image_redaction.rs crates/mdid-adapters/src/lib.rs && git commit -m "feat(adapters): add bounded png image redaction export"`

### Task 2: CLI PNG redaction command and truth-sync

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
- Modify: `README.md`

- [ ] **Step 1: Write failing CLI smoke test**

Add a test that writes a tiny PNG fixture to a tempdir, invokes `mdid-cli redact-image-png --input <png> --regions <regions.json> --output <out.png> --summary-output <summary.json>`, decodes output PNG, verifies changed pixel and unchanged pixel, and asserts summary contains no input path/filename/raw bbox arrays.

- [ ] **Step 2: Run RED**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-cli redact_image_png --test cli_smoke -- --nocapture`
Expected: FAIL because command is not implemented.

- [ ] **Step 3: Implement minimal CLI command**

Mirror `redact-image-ppm` argument handling and summary-writing behavior, but call the new PNG adapter helper and emit `artifact: image_redaction_export_summary`, `format: png`, aggregate counts, and explicit non-goals.

- [ ] **Step 4: Run GREEN**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-cli redact_image_png --test cli_smoke -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Regression**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-cli --test cli_smoke redact_image -- --nocapture && cargo test -p mdid-adapters --test image_redaction_adapter -- --nocapture && cargo fmt --check && git diff --check`
Expected: PASS.

- [ ] **Step 6: Commit**

Run: `git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs README.md && git commit -m "feat(cli): export bounded png redaction bytes"`

## Self-Review

Spec coverage: addresses priority item 4 media-byte rewrite/export and item 2 actual visual/image pixel redaction by adding real PNG byte rewrite/export for explicit bbox regions. Placeholder scan: no TBD/TODO/fill-in placeholders. Type consistency: uses existing `ImageRedactionRegion` and existing PPM command/summary patterns.
