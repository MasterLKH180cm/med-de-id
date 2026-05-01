# OCR Privacy Evidence CLI Wrapper Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded `mdid-cli ocr-privacy-evidence` wrapper around the existing OCR→text-only Privacy Filter aggregate evidence runner.

**Architecture:** The Rust CLI will delegate to the existing `scripts/ocr_eval/run_ocr_privacy_evidence.py` runner, validate the aggregate-only PHI-safe evidence contract, and write a sanitized report plus PHI/path-safe stdout summary. This is CLI/runtime evidence only; it does not add Browser/Web or Desktop execution, OCR model-quality proof, visual redaction, image pixel redaction, or final PDF rewrite/export.

**Tech Stack:** Rust `mdid-cli`, Python fixture runners under `scripts/ocr_eval` and `scripts/privacy_filter`, Cargo tests, README truth-sync.

---

## File Structure

- Modify `crates/mdid-cli/src/main.rs`: add `OcrPrivacyEvidenceArgs`, parser branch, runner invocation, aggregate report validation, stale report cleanup, PHI/path-safe stdout, help text.
- Modify `crates/mdid-cli/tests/cli_smoke.rs`: add TDD smoke tests for success, stale cleanup on prerequisite failure, invalid runner output rejection, path/PHI leak safety, and help discoverability.
- Modify `scripts/ocr_eval/README.md`: document the new CLI wrapper command and its non-goals.
- Modify `README.md`: truth-sync CLI/runtime completion evidence and completion arithmetic without raising Browser/Web or Desktop.

### Task 1: Add bounded `mdid-cli ocr-privacy-evidence` wrapper

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/cli_smoke.rs`

- [x] **Step 1: Write failing smoke tests**

Add tests that invoke `mdid-cli ocr-privacy-evidence` with checked-in fixtures and assert: report exists, JSON has `artifact: "ocr_privacy_evidence"`, `ocr_scope: "printed_text_line_extraction_only"`, `privacy_scope: "text_only_pii_detection"`, `network_api_called: false`, stdout redacts `report_path`, and stdout/stderr/report omit `Jane Example`, `MRN-12345`, `jane@example.com`, `555-123-4567`, fixture filenames, and temp paths. Add tests for missing image stale cleanup, fake runner invalid JSON/schema stale cleanup, and help containing the exact usage line `mdid-cli ocr-privacy-evidence --image-path <image> --runner-path <runner.py> --output <report.json> [--python-command <cmd>] [--mock]`.

- [x] **Step 2: Run RED**

Run: `/home/azureuser/.cargo/bin/cargo test -p mdid-cli ocr_privacy_evidence -- --nocapture`
Expected: FAIL because the command is not parsed/implemented.

- [x] **Step 3: Implement minimal CLI wrapper**

In `main.rs`, add `OcrPrivacyEvidenceArgs { image_path, runner_path, output_path, python_command, mock }`, parse the command and required flags, reject missing files with fixed PHI/path-safe errors, remove stale output before running, execute `<python> <runner> --image-path <image> --output <temp/report> --mock` only when requested, suppress runner diagnostics, validate the written JSON contract with a strict allowlist, reject unsafe fields/labels/network flags/path-like strings, and print only aggregate stdout with `report_path: "<redacted>"` and `report_written: true`.

- [x] **Step 4: Run GREEN and broader CLI tests**

Run: `/home/azureuser/.cargo/bin/cargo test -p mdid-cli ocr_privacy_evidence -- --nocapture`
Expected: PASS.
Run: `/home/azureuser/.cargo/bin/cargo test -p mdid-cli -- --nocapture`
Expected: PASS.

- [x] **Step 5: Commit**

Run:
```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs docs/superpowers/plans/2026-05-01-ocr-privacy-evidence-cli-wrapper.md
git commit -m "feat(cli): add OCR privacy evidence wrapper"
```

### Task 2: Truth-sync docs and completion accounting

**Files:**
- Modify: `scripts/ocr_eval/README.md`
- Modify: `README.md`

- [ ] **Step 1: Write docs regression expectations or identify existing docs checks**

Search existing tests for README/docs assertions. If a docs test exists, add assertions that the new CLI wrapper is documented as CLI/runtime aggregate evidence only and not Browser/Web/Desktop execution. If no docs test exists, document the exact commands and verify with `git diff --check`.

- [ ] **Step 2: Update docs**

Update `scripts/ocr_eval/README.md` with the exact command:
```bash
cargo run -p mdid-cli -- ocr-privacy-evidence \
  --image-path scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png \
  --runner-path scripts/ocr_eval/run_ocr_privacy_evidence.py \
  --output /tmp/ocr-privacy-evidence-cli.json \
  --python-command python3 \
  --mock
```
State the report is aggregate-only, PHI-safe, and CLI/runtime evidence only.

Update `README.md` completion snapshot: add this new CLI/runtime requirement to numerator and denominator, conservatively floor percentages, keep Browser/Web 99% and Desktop 99%, and mark the reserved final 1% blockers unchanged. Mention Browser/Web/Desktop receive +0% this round because no user-facing surface capability changed.

- [ ] **Step 3: Verify docs and code**

Run: `/home/azureuser/.cargo/bin/cargo test -p mdid-cli ocr_privacy_evidence -- --nocapture`
Expected: PASS.
Run: `python scripts/ocr_eval/run_ocr_privacy_evidence.py --image-path scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png --output /tmp/ocr-privacy-evidence.json --mock`
Expected: PASS with PHI-safe stdout.
Run: `git diff --check`
Expected: no whitespace errors.

- [ ] **Step 4: Commit**

Run:
```bash
git add README.md scripts/ocr_eval/README.md
git commit -m "docs: truth-sync OCR privacy evidence CLI wrapper"
```
