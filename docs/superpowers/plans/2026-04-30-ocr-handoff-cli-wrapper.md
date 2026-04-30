# OCR Handoff CLI Wrapper Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded `mdid-cli ocr-handoff` command that runs the existing synthetic PP-OCRv5 mobile line-image runner, builds a validated OCR-to-text-PII handoff JSON artifact, and proves the output can feed the text-only Privacy Filter path.

**Architecture:** Keep OCR as extraction-only: the CLI wrapper composes existing local Python helpers and validates the generated handoff contract before writing the report path. The command does not add visual redaction, page detection, handwriting recognition, PDF rewrite/export, agent workflows, or browser/desktop UI. It is a CLI/runtime bridge that turns the PP-OCRv5 mobile spike into a reproducible artifact for downstream text PII evaluation.

**Tech Stack:** Rust `mdid-cli`, existing Python helpers under `scripts/ocr_eval/`, existing Privacy Filter Python runner, `assert_cmd` CLI smoke tests, JSON contract validation.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `CliCommand::OcrHandoff(OcrHandoffArgs)`.
  - Parse `ocr-handoff --image-path <path> --ocr-runner-path <path> --handoff-builder-path <path> --report-path <path> [--python-command <cmd>]`.
  - Run OCR runner with `--mock`, cap stdout at 1 MiB, write temporary OCR text next to report, run handoff builder, validate output JSON shape, remove the temporary OCR text on success, and print a JSON summary.
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
  - Add tests for help text, missing flag rejection, end-to-end mock fixture success, missing image rejection, non-JSON/invalid handoff rejection, and oversized OCR stdout rejection.
- Modify: `scripts/ocr_eval/README.md`
  - Document the CLI wrapper command and its extraction-only non-goals.
- Modify: `docs/research/small-ocr-spike-results.md`
  - Record wrapper verification evidence.
- Modify: `README.md`
  - Truth-sync completion/evidence without claiming Browser/Web or Desktop capability gains from this CLI/runtime slice.

## Task 1: Add failing CLI smoke tests for `ocr-handoff`

**Files:**
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`

- [ ] **Step 1: Write failing tests**

Append tests that expect the new command to exist and enforce the bounded OCR handoff contract:

```rust
#[test]
fn ocr_handoff_help_mentions_bounded_extraction_command() {
    cli()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("ocr-handoff"));
}

#[test]
fn ocr_handoff_rejects_missing_required_flags() {
    cli()
        .arg("ocr-handoff")
        .assert()
        .failure()
        .stderr(predicate::str::contains("missing --image-path"));
}

#[test]
fn ocr_handoff_runs_mock_fixture_and_writes_valid_handoff_report() {
    let temp = assert_fs::TempDir::new().unwrap();
    let report = temp.child("ocr-handoff.json");

    cli()
        .arg("ocr-handoff")
        .arg("--image-path")
        .arg(repo_path("scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png"))
        .arg("--ocr-runner-path")
        .arg(repo_path("scripts/ocr_eval/run_small_ocr.py"))
        .arg("--handoff-builder-path")
        .arg(repo_path("scripts/ocr_eval/build_ocr_handoff.py"))
        .arg("--report-path")
        .arg(report.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("ocr-handoff"));

    let value: serde_json::Value = serde_json::from_str(&report.read_to_string().unwrap()).unwrap();
    assert_eq!(value["source"], "synthetic_printed_phi_line.png");
    assert_eq!(value["candidate"], "PP-OCRv5_mobile_rec");
    assert_eq!(value["scope"], "printed_text_line_extraction_only");
    assert_eq!(value["privacy_filter_contract"], "text_only_normalized_input");
    assert_eq!(value["ready_for_text_pii_eval"], true);
    assert!(value["normalized_text"].as_str().unwrap().contains("Jane Doe"));
}
```

- [ ] **Step 2: Run tests to verify RED**

Run:

```bash
cargo test -p mdid-cli ocr_handoff -- --nocapture
```

Expected: FAIL because `ocr-handoff` is an unknown command or help text does not include it.

## Task 2: Implement minimal `ocr-handoff` parser and runner

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`

- [ ] **Step 1: Add command args and parser**

Add an `OcrHandoffArgs` struct, enum variant, parser branch, `parse_ocr_handoff_args`, and usage line. Required flags are exactly `--image-path`, `--ocr-runner-path`, `--handoff-builder-path`, and `--report-path`; optional `--python-command` defaults to `python3`.

- [ ] **Step 2: Implement bounded execution**

Implement `run_ocr_handoff(args)` by reusing the privacy-filter stdout-cap pattern: validate image/runner/builder are regular files, spawn Python runner with `--mock <image-path>`, read at most `1 MiB + 1`, reject oversized/non-UTF-8/non-zero output, write OCR text to `<report-path>.ocr-text.tmp`, spawn builder with `--source`, `--input`, `--output`, validate generated JSON fields, delete temp OCR text, and print:

```json
{"command":"ocr-handoff","report_path":"<path>"}
```

- [ ] **Step 3: Run tests to verify GREEN**

Run:

```bash
cargo test -p mdid-cli ocr_handoff -- --nocapture
```

Expected: PASS.

## Task 3: Add robustness tests and validation details

**Files:**
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
- Modify: `crates/mdid-cli/src/main.rs`

- [ ] **Step 1: Add failing robustness tests**

Add tests that assert missing image returns `missing image file`, a fake builder that writes `{}` is rejected with `OCR handoff missing required field`, and a fake OCR runner that emits more than 1 MiB is rejected with `OCR runner output exceeded limit` and does not leave the final report.

- [ ] **Step 2: Run tests to verify RED**

Run:

```bash
cargo test -p mdid-cli ocr_handoff -- --nocapture
```

Expected: FAIL until validation/error handling is complete.

- [ ] **Step 3: Implement validations**

Validate required fields: `source` string, `extracted_text` string, `normalized_text` string, `ready_for_text_pii_eval` bool, `candidate == "PP-OCRv5_mobile_rec"`, `engine == "PP-OCRv5-mobile-bounded-spike"`, `scope == "printed_text_line_extraction_only"`, `privacy_filter_contract == "text_only_normalized_input"`, and `non_goals` includes `visual_redaction`, `final_pdf_rewrite_export`, `handwriting_recognition`, `full_page_detection_or_segmentation`, and `complete_ocr_pipeline`.

- [ ] **Step 4: Run tests to verify GREEN**

Run:

```bash
cargo test -p mdid-cli ocr_handoff -- --nocapture
```

Expected: PASS.

## Task 4: Documentation and truthful verification evidence

**Files:**
- Modify: `scripts/ocr_eval/README.md`
- Modify: `docs/research/small-ocr-spike-results.md`
- Modify: `README.md`

- [ ] **Step 1: Update OCR README**

Add the exact wrapper command:

```bash
cargo run -p mdid-cli -- ocr-handoff \
  --image-path scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png \
  --ocr-runner-path scripts/ocr_eval/run_small_ocr.py \
  --handoff-builder-path scripts/ocr_eval/build_ocr_handoff.py \
  --report-path /tmp/mdid-ocr-handoff.json
python scripts/ocr_eval/validate_ocr_handoff.py /tmp/mdid-ocr-handoff.json
python scripts/privacy_filter/run_privacy_filter.py --mock /tmp/mdid-ocr-handoff-text.txt
```

Also state that this is printed-text line extraction only, not visual redaction or PDF rewrite/export.

- [ ] **Step 2: Update results and top-level README**

Record verification commands and completion truth-sync. Keep CLI at 95% unless evidence supports a truthful change; Browser/Web and Desktop remain unchanged because no browser/desktop capability landed.

- [ ] **Step 3: Run final verification**

Run:

```bash
cargo test -p mdid-cli ocr_handoff -- --nocapture
python scripts/ocr_eval/run_small_ocr.py --mock scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png > /tmp/small-ocr-output.txt
python scripts/ocr_eval/build_ocr_handoff.py --source scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png --input /tmp/small-ocr-output.txt --output /tmp/ocr-handoff.json
python scripts/ocr_eval/validate_ocr_handoff.py /tmp/ocr-handoff.json
python scripts/privacy_filter/run_privacy_filter.py --mock /tmp/small-ocr-output.txt > /tmp/ocr-privacy-filter.json
python scripts/privacy_filter/validate_privacy_filter_output.py /tmp/ocr-privacy-filter.json
git diff --check
```

Expected: all commands pass.

- [ ] **Step 4: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs scripts/ocr_eval/README.md docs/research/small-ocr-spike-results.md README.md docs/superpowers/plans/2026-04-30-ocr-handoff-cli-wrapper.md
git commit -m "feat(cli): add bounded OCR handoff wrapper"
```

## Self-Review

- Spec coverage: the plan adds a bounded CLI/runtime OCR handoff wrapper, validates the handoff, and records evidence without claiming visual redaction/PDF rewrite/browser/desktop completion.
- Placeholder scan: no TBD/TODO/implement-later placeholders remain.
- Type consistency: command name is consistently `ocr-handoff`; args are consistently `OcrHandoffArgs`; output summary uses `command: ocr-handoff`.
