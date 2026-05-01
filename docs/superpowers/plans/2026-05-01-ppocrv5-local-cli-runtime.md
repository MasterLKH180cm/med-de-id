# PP-OCRv5 Local CLI Runtime Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Let the existing CLI/runtime OCR wrappers exercise the local PP-OCRv5 mobile OCR runner path without `--mock`, while preserving bounded PHI-safe reports and explicit non-goals.

**Architecture:** Keep `scripts/ocr_eval/run_small_ocr.py` as the source of truth for local PaddleOCR/PP-OCRv5 invocation and adjust Rust CLI wrappers so `--mock` is optional rather than mandatory. Tests use a fake OCR runner to prove the CLI no longer forces mock mode and still validates the same bounded JSON contract; no Browser/Web, Desktop, visual redaction, agent workflow, controller, PDF rewrite/export, or image pixel redaction capability is added.

**Tech Stack:** Rust `mdid-cli`, Cargo integration tests, Python OCR runner contract scripts, serde_json.

---

## File Structure

- Modify `crates/mdid-cli/tests/cli_smoke.rs`: replace mock-only rejection expectations with fake-runner non-mock tests for `ocr-small-json` and `ocr-to-privacy-filter`.
- Modify `crates/mdid-cli/src/main.rs`: remove mock-only gates, pass `--mock` only when requested, preserve stale artifact cleanup and schema validation.
- Modify `scripts/ocr_eval/README.md`: document local non-mock CLI runtime execution as bounded local PP-OCRv5 extraction candidate evidence.
- Modify `README.md`: truth-sync completion evidence and rubric; Browser/Web and Desktop do not increase because this is CLI/runtime-only.

### Task 1: Enable `ocr-small-json` local non-mock CLI execution

**Files:**
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
- Modify: `crates/mdid-cli/src/main.rs`

- [x] **Step 1: Write the failing test**

Add this Rust smoke test in `crates/mdid-cli/tests/cli_smoke.rs` near the existing `ocr_small_json` tests:

```rust
#[test]
fn ocr_small_json_local_mode_does_not_force_mock_flag() {
    let dir = tempdir().expect("tempdir");
    let runner_path = dir.path().join("fake-local-ocr-runner.py");
    let args_path = dir.path().join("runner-args.txt");
    let report_path = dir.path().join("ocr-small-local.json");
    let summary_path = dir.path().join("ocr-small-local-summary.json");

    std::fs::write(
        &runner_path,
        format!(
            r#"#!/usr/bin/env python3
import json
import pathlib
import sys
pathlib.Path({args_path:?}).write_text("\n".join(sys.argv[1:]), encoding="utf-8")
print(json.dumps({{
    "artifact": "ocr_handoff_v1",
    "candidate": "PP-OCRv5_mobile_rec",
    "scope": "printed_text_line_extraction_only",
    "engine_status": "local_paddleocr_execution",
    "image_id": "fixture_001",
    "text_char_count": 43,
    "line_count": 1,
    "contains_text": True,
    "network_api_called": False,
    "non_goals": ["visual_redaction", "pixel_redaction", "pdf_rewrite_export"]
}}))
"#,
            args_path = args_path.to_string_lossy()
        ),
    )
    .expect("write fake runner");

    Command::cargo_bin("mdid-cli")
        .expect("binary")
        .args([
            "ocr-small-json",
            "--image-path",
            &repo_path("scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png"),
            "--ocr-runner-path",
            runner_path.to_str().expect("runner path"),
            "--report-path",
            report_path.to_str().expect("report path"),
            "--summary-output",
            summary_path.to_str().expect("summary path"),
            "--python-command",
            "python",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("ocr-small-json"))
        .stdout(predicate::str::contains("\"report_path\":\"<redacted>\""));

    let runner_args = std::fs::read_to_string(args_path).expect("runner args");
    assert!(!runner_args.lines().any(|line| line == "--mock"), "local runtime must not force --mock: {runner_args}");

    let report = std::fs::read_to_string(report_path).expect("report");
    assert!(report.contains("local_paddleocr_execution"));
    assert!(!report.contains("Jane Example"));

    let summary = std::fs::read_to_string(summary_path).expect("summary");
    assert!(summary.contains("PP-OCRv5_mobile_rec"));
    assert!(!summary.contains("Jane Example"));
}
```

- [x] **Step 2: Run test to verify RED**

Run: `/home/azureuser/.cargo/bin/cargo test -p mdid-cli ocr_small_json_local_mode_does_not_force_mock_flag -- --nocapture`

Expected: FAIL with `OCR small JSON requires mock mode` or with the fake runner args showing `--mock` was forced.

- [x] **Step 3: Write minimal implementation**

In `crates/mdid-cli/src/main.rs`, remove the `if !args.mock { return Err("OCR small JSON requires mock mode"...) }` gate in `run_ocr_small_json`. In `run_ocr_small_json_inner`, push `--mock` only when `args.mock` is true:

```rust
if args.mock {
    command.arg("--mock");
}
```

Keep the existing stale report removal, bounded subprocess execution, JSON validation, and summary writing unchanged.

- [x] **Step 4: Run test to verify GREEN**

Run: `/home/azureuser/.cargo/bin/cargo test -p mdid-cli ocr_small_json_local_mode_does_not_force_mock_flag -- --nocapture`

Expected: PASS.

- [x] **Step 5: Run focused regression tests**

Run: `/home/azureuser/.cargo/bin/cargo test -p mdid-cli ocr_small_json -- --nocapture`

Expected: PASS.

### Task 2: Enable `ocr-to-privacy-filter` local non-mock chain execution

**Files:**
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
- Modify: `crates/mdid-cli/src/main.rs`

- [x] **Step 1: Write the failing test**

Add this Rust smoke test near the existing `ocr_to_privacy_filter` tests:

```rust
#[test]
fn ocr_to_privacy_filter_local_mode_does_not_force_mock_flag() {
    let dir = tempdir().expect("tempdir");
    let ocr_runner_path = dir.path().join("fake-local-ocr-runner.py");
    let args_path = dir.path().join("ocr-runner-args.txt");
    let report_path = dir.path().join("ocr-to-privacy-filter-local.json");
    let summary_path = dir.path().join("ocr-to-privacy-filter-local-summary.json");

    std::fs::write(
        &ocr_runner_path,
        format!(
            r#"#!/usr/bin/env python3
import pathlib
import sys
pathlib.Path({args_path:?}).write_text("\n".join(sys.argv[1:]), encoding="utf-8")
print("Patient Jane Example MRN-12345 jane@example.com 555-123-4567")
"#,
            args_path = args_path.to_string_lossy()
        ),
    )
    .expect("write fake runner");

    Command::cargo_bin("mdid-cli")
        .expect("binary")
        .args([
            "ocr-to-privacy-filter",
            "--image-path",
            &repo_path("scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png"),
            "--ocr-runner-path",
            ocr_runner_path.to_str().expect("runner path"),
            "--privacy-runner-path",
            &repo_path("scripts/privacy_filter/run_privacy_filter.py"),
            "--report-path",
            report_path.to_str().expect("report path"),
            "--summary-output",
            summary_path.to_str().expect("summary path"),
            "--python-command",
            "python",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("ocr-to-privacy-filter"))
        .stdout(predicate::str::contains("\"report_path\":\"...\""));

    let runner_args = std::fs::read_to_string(args_path).expect("runner args");
    assert!(!runner_args.lines().any(|line| line == "--mock"), "local runtime must not force --mock: {runner_args}");

    let report = std::fs::read_to_string(report_path).expect("report");
    assert!(report.contains("ocr_to_privacy_filter_single"));
    assert!(report.contains("text_only_pii_detection"));
    assert!(!report.contains("Jane Example"));
    assert!(!report.contains("MRN-12345"));
    assert!(!report.contains("jane@example.com"));
    assert!(!report.contains("555-123-4567"));

    let summary = std::fs::read_to_string(summary_path).expect("summary");
    assert!(summary.contains("detected_span_count"));
    assert!(!summary.contains("Jane Example"));
}
```

- [x] **Step 2: Run test to verify RED**

Run: `/home/azureuser/.cargo/bin/cargo test -p mdid-cli ocr_to_privacy_filter_local_mode_does_not_force_mock_flag -- --nocapture`

Expected: FAIL with `ocr_to_privacy_filter single-image chain requires mock mode`.

- [x] **Step 3: Write minimal implementation**

In `crates/mdid-cli/src/main.rs`, remove the `if !args.mock { return Err("ocr_to_privacy_filter single-image chain requires mock mode"...) }` gate in `run_ocr_to_privacy_filter`. Leave the existing conditional `if args.mock { ocr_command.arg("--mock"); }` unchanged.

- [x] **Step 4: Run test to verify GREEN**

Run: `/home/azureuser/.cargo/bin/cargo test -p mdid-cli ocr_to_privacy_filter_local_mode_does_not_force_mock_flag -- --nocapture`

Expected: PASS.

- [x] **Step 5: Run focused regression tests**

Run: `/home/azureuser/.cargo/bin/cargo test -p mdid-cli ocr_to_privacy_filter -- --nocapture`

Expected: PASS.

### Task 3: Documentation, completion truth-sync, and final verification

**Files:**
- Modify: `scripts/ocr_eval/README.md`
- Modify: `README.md`
- Modify: `docs/superpowers/plans/2026-05-01-ppocrv5-local-cli-runtime.md`

- [x] **Step 1: Update OCR docs with exact local runtime command**

Add this command to `scripts/ocr_eval/README.md`:

```bash
mdid-cli ocr-small-json \
  --image-path scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png \
  --ocr-runner-path scripts/ocr_eval/run_small_ocr.py \
  --report-path /tmp/ocr-small-local.json \
  --summary-output /tmp/ocr-small-local-summary.json
```

State that omitting `--mock` attempts local PaddleOCR/PP-OCRv5 execution only; it is a printed-text extraction spike and not visual redaction, handwriting recognition, pixel redaction, final PDF rewrite/export, or Browser/Desktop integration.

- [x] **Step 2: Update README completion truth-sync**

Update `README.md` current repository status to mention the local non-mock PP-OCRv5 CLI/runtime path. Treat this as a newly completed CLI/runtime requirement added to the rubric in the same round. Use fraction accounting conservatively: CLI old `95/100`, add one required item and complete it as `96/101`, floored completion remains `95%`; Browser/Web remains `99%`; Desktop app remains `99%`; Overall remains `97%` unless landed repository evidence justifies a floored fraction increase.

- [x] **Step 3: Run verification**

Run:

```bash
/home/azureuser/.cargo/bin/cargo test -p mdid-cli ocr_small_json -- --nocapture
/home/azureuser/.cargo/bin/cargo test -p mdid-cli ocr_to_privacy_filter -- --nocapture
python -m py_compile scripts/ocr_eval/run_small_ocr.py scripts/privacy_filter/run_privacy_filter.py
git diff --check
```

Expected: all PASS.

- [x] **Step 4: Commit**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs scripts/ocr_eval/README.md README.md docs/superpowers/plans/2026-05-01-ppocrv5-local-cli-runtime.md
git commit -m "feat(cli): enable local ppocrv5 runtime path"
```

---

## Self-Review

- Spec coverage: The plan targets the highest-leverage PP-OCRv5 mobile gap by allowing existing CLI/runtime wrappers to exercise local OCR extraction without mock mode, while preserving bounded text handoff to Privacy Filter.
- Placeholder scan: No TBD/TODO/fill-in placeholders remain.
- Type consistency: Command names, args, report fields, and engine status strings match existing code contracts.
