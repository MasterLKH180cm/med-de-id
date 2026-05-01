# OCR Small JSON CLI Wrapper Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded `mdid-cli ocr-small-json` wrapper for the existing PP-OCRv5 mobile small-runner `--json` output so CLI automation can produce and validate the printed-text extraction handoff JSON directly.

**Architecture:** Reuse the existing `scripts/ocr_eval/run_small_ocr.py --json` runner and `scripts/ocr_eval/validate_ocr_handoff.py` contract rather than adding a new OCR engine path. The Rust CLI wrapper executes the checked-in runner with explicit `--mock`, validates the resulting JSON shape/scope/non-goals, writes a report atomically enough for this bounded spike, redacts paths in stdout, and removes stale artifacts on failures.

**Tech Stack:** Rust `mdid-cli`, Python OCR eval scripts, Cargo integration tests, synthetic-only fixtures.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs` — add `ocr-small-json` command parsing, bounded subprocess execution, JSON validation, report writing, PHI-safe stdout/error behavior, and usage text.
- Modify: `crates/mdid-cli/tests/cli_smoke.rs` — add smoke tests for success, help discoverability, invalid output/stale cleanup, missing input cleanup, and PHI/path-safe stdout/stderr.
- Modify: `README.md` — truth-sync the CLI/runtime OCR small JSON wrapper evidence and completion snapshot without raising Browser/Web or Desktop.
- Modify: `scripts/ocr_eval/README.md` — document exact local wrapper command and non-goals.

### Task 1: Add `mdid-cli ocr-small-json` bounded CLI wrapper

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/cli_smoke.rs`

- [x] **Step 1: Write failing smoke tests**

Add tests that invoke `mdid-cli ocr-small-json --image-path scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png --ocr-runner-path scripts/ocr_eval/run_small_ocr.py --report-path <tmp>/ocr-small-json.json --python-command <python> --mock` and assert:
- exit success;
- report JSON contains `candidate: PP-OCRv5_mobile_rec`, `scope: printed_text_line_extraction_only`, `privacy_filter_contract: text_only_normalized_input`, `ready_for_text_pii_eval: true`;
- stdout includes `ocr-small-json`, `report_path":"<redacted>`, and `report_written":true`;
- stdout/stderr omit the temp path and synthetic PHI sentinels (`Jane Example`, `MRN-12345`, `jane@example.com`, `555-123-4567`);
- help text mentions `ocr-small-json`.

Add failure tests with fake runner scripts that emit invalid JSON or a contract with a wrong scope. Pre-create stale report files containing `Jane Example`, run the command, assert failure, stale report removal, generic PHI-safe errors, and no path/PHI leaks.

Run: `/home/azureuser/.cargo/bin/cargo test -p mdid-cli ocr_small_json -- --nocapture`
Expected: FAIL because the command does not exist.

- [x] **Step 2: Implement minimal command**

In `crates/mdid-cli/src/main.rs`, add:
- `CliCommand::OcrSmallJson(OcrSmallJsonArgs)`;
- `struct OcrSmallJsonArgs { image_path: PathBuf, ocr_runner_path: PathBuf, report_path: PathBuf, python_command: String, mock: bool }`;
- parser for required `--image-path`, `--ocr-runner-path`, `--report-path`, optional `--python-command`, and required explicit `--mock` for this bounded synthetic spike;
- subprocess runner calling `<python> <runner> --json --mock <image>` with stdout cap and timeout;
- validator for the handoff JSON object enforcing exact candidate/engine/scope/privacy contract, bool readiness, string extracted/normalized text, required non-goals, no incompatible visual/PDF claims beyond non-goals, and no raw PHI in stdout/stderr;
- write report only after validation, remove stale report before starting and on failure;
- stdout summary JSON with `command`, `report_written`, `report_path: <redacted>`, `candidate`, `scope`, and `ready_for_text_pii_eval` only.

- [x] **Step 3: Run targeted tests**

Run: `/home/azureuser/.cargo/bin/cargo test -p mdid-cli ocr_small_json -- --nocapture`
Expected: PASS.

- [x] **Step 4: Run script-chain verification**

Run:
```bash
python scripts/ocr_eval/run_small_ocr.py --mock --json scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png > /tmp/ocr-small-json-wrapper-source.json
python scripts/ocr_eval/validate_ocr_handoff.py /tmp/ocr-small-json-wrapper-source.json
/home/azureuser/.cargo/bin/cargo run -p mdid-cli -- ocr-small-json --image-path scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png --ocr-runner-path scripts/ocr_eval/run_small_ocr.py --report-path /tmp/ocr-small-json-wrapper-report.json --python-command python --mock
python scripts/ocr_eval/validate_ocr_handoff.py /tmp/ocr-small-json-wrapper-report.json
```
Expected: all commands PASS; CLI stdout redacts report path.

- [x] **Step 5: Commit**

Run:
```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs
git commit -m "feat(cli): add bounded ocr small json wrapper"
```

### Task 2: Truth-sync docs and completion evidence

**Files:**
- Modify: `README.md`
- Modify: `scripts/ocr_eval/README.md`

- [x] **Step 1: Update docs**

Add a concise evidence paragraph saying `mdid-cli ocr-small-json` wraps the existing PP-OCRv5 mobile small-runner `--json` synthetic fixture mode, validates the same OCR handoff JSON contract, keeps stdout/errors PHI/path-safe, writes a validated OCR handoff JSON report containing normalized OCR text for downstream text-only Privacy Filter evaluation, and proves printed-text extraction output can feed downstream text-only Privacy Filter PII detection. State explicit non-goals: no OCR quality claim, visual redaction, image pixel redaction, handwriting recognition, browser/desktop execution, or final PDF rewrite/export. Do not call the report itself PHI-safe because it intentionally contains OCR text in `extracted_text` / `normalized_text`.

- [x] **Step 2: Completion truth-sync**

Keep Browser/Web at 99% and Desktop app at 99% because this is CLI/runtime only. Re-baseline CLI fraction by adding this wrapper as one new required CLI/runtime OCR automation item and completing it in the same round; floor integer remains 95% unless README rubric facts justify otherwise. Keep Overall at 97% unless the existing README fraction explicitly supports a conservative increase.

- [x] **Step 3: Verify docs and commit**

Run:
```bash
git diff --check
/home/azureuser/.cargo/bin/cargo test -p mdid-cli ocr_small_json -- --nocapture
python scripts/ocr_eval/validate_ocr_handoff.py /tmp/ocr-small-json-wrapper-report.json
```
Expected: PASS.

Commit:
```bash
git add README.md scripts/ocr_eval/README.md docs/superpowers/plans/2026-05-01-ocr-small-json-cli-wrapper.md
git commit -m "docs: truth-sync ocr small json cli wrapper"
```

## Self-Review

- Spec coverage: The plan adds a CLI/runtime wrapper for existing OCR JSON extraction and documents completion truthfully.
- Placeholder scan: No TBD/TODO placeholders remain.
- Type consistency: The command name is consistently `ocr-small-json`; report contract fields match the existing OCR handoff JSON shape.
