# OCR Privacy Evidence Summary Output Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an optional PHI-safe aggregate `--summary-output <summary.json>` artifact to the bounded `mdid-cli ocr-privacy-evidence` CLI/runtime bridge.

**Architecture:** The Rust CLI keeps delegating to `scripts/ocr_eval/run_ocr_privacy_evidence.py`, validates the full aggregate report, then writes a second stricter summary artifact derived only from the validated report. The summary is CLI/runtime evidence only and does not add Browser/Web or Desktop execution, OCR model-quality proof, visual redaction, image pixel redaction, handwriting recognition, or final PDF rewrite/export.

**Tech Stack:** Rust `mdid-cli`, `assert_cmd` CLI smoke tests, serde_json, existing path-equivalence and PHI-safe output patterns, README truth-sync.

---

## File Structure

- Modify `crates/mdid-cli/src/main.rs`: add optional `summary_output_path` parsing for `ocr-privacy-evidence`, reject equivalent report/summary paths before cleanup, write a strict summary after validated report write, clean stale summary on failure, and update help text.
- Modify `crates/mdid-cli/tests/cli_smoke.rs`: add TDD smoke tests for success summary output, same/alias path rejection, missing image stale summary cleanup, invalid runner output stale summary cleanup, and help/docs discoverability.
- Modify `README.md`: update completion snapshot/evidence with truthful CLI/runtime-only summary-output requirement and fraction accounting.
- Modify `scripts/ocr_eval/README.md`: document the optional summary artifact and non-goals.
- Modify this plan file: check completed steps before final review.

### Task 1: Add `ocr-privacy-evidence --summary-output` CLI/runtime summary artifact

**Files:**
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
- Modify: `crates/mdid-cli/src/main.rs`

- [x] **Step 1: Write the failing success and cleanup tests**

Add tests to `crates/mdid-cli/tests/cli_smoke.rs` near the existing `ocr_privacy_evidence_*` tests:

```rust
#[test]
fn ocr_privacy_evidence_writes_phi_safe_summary_output() {
    let dir = tempdir().unwrap();
    let phi_named_dir = dir.path().join("Jane-Example-MRN-12345");
    fs::create_dir_all(&phi_named_dir).unwrap();
    let report_path = phi_named_dir.join("ocr-privacy-evidence-report.json");
    let summary_path = phi_named_dir.join("ocr-privacy-evidence-summary.json");

    let assert = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-privacy-evidence",
            "--image-path",
            &repo_path("scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png"),
            "--runner-path",
            &repo_path("scripts/ocr_eval/run_ocr_privacy_evidence.py"),
            "--output",
            report_path.to_str().unwrap(),
            "--summary-output",
            summary_path.to_str().unwrap(),
            "--python-command",
            &default_python_command(),
            "--mock",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(stdout.contains("ocr-privacy-evidence"));
    assert!(stdout.contains("\"summary_written\":true"));
    assert!(!stdout.contains(phi_named_dir.to_str().unwrap()));
    assert!(stderr.is_empty());

    let summary_text = fs::read_to_string(&summary_path).unwrap();
    let summary: serde_json::Value = serde_json::from_str(&summary_text).unwrap();
    assert_eq!(summary["artifact"], "ocr_privacy_evidence_summary");
    assert_eq!(summary["ocr_scope"], "printed_text_line_extraction_only");
    assert_eq!(summary["privacy_scope"], "text_only_pii_detection");
    assert_eq!(summary["network_api_called"], false);
    assert_eq!(summary["total_detected_span_count"], 4);
    assert_eq!(summary["category_counts"]["NAME"], 1);
    assert_eq!(summary["category_counts"]["MRN"], 1);
    assert_eq!(summary["category_counts"]["EMAIL"], 1);
    assert_eq!(summary["category_counts"]["PHONE"], 1);
    assert!(summary.get("fixtures").is_none());
    assert!(summary.get("report_path").is_none());
    assert!(summary.get("normalized_text").is_none());
    assert!(summary.get("masked_text").is_none());
    assert!(summary.get("spans").is_none());
    for forbidden in ["Jane Example", "MRN-12345", "jane@example.com", "555-123-4567", phi_named_dir.to_str().unwrap()] {
        assert!(!stdout.contains(forbidden));
        assert!(!stderr.contains(forbidden));
        assert!(!summary_text.contains(forbidden));
    }
}

#[test]
fn ocr_privacy_evidence_summary_path_must_differ_from_report_path() {
    let dir = tempdir().unwrap();
    let output_path = dir.path().join("Jane-Example-MRN-12345.json");
    fs::write(&output_path, "stale primary Jane Example").unwrap();

    let assert = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-privacy-evidence",
            "--image-path",
            &repo_path("scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png"),
            "--runner-path",
            &repo_path("scripts/ocr_eval/run_ocr_privacy_evidence.py"),
            "--output",
            output_path.to_str().unwrap(),
            "--summary-output",
            output_path.to_str().unwrap(),
            "--python-command",
            &default_python_command(),
            "--mock",
        ])
        .assert()
        .failure();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(stdout.is_empty());
    assert_eq!(stderr.trim(), "ocr privacy evidence summary path must differ from report path");
    assert_eq!(fs::read_to_string(&output_path).unwrap(), "stale primary Jane Example");
}

#[test]
fn ocr_privacy_evidence_summary_alias_path_must_differ_from_report_path() {
    let dir = tempdir().unwrap();
    let output_path = dir.path().join("report.json");
    let alias_path = dir.path().join(".").join("report.json");
    fs::write(&output_path, "stale primary Jane Example").unwrap();

    let assert = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-privacy-evidence",
            "--image-path",
            &repo_path("scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png"),
            "--runner-path",
            &repo_path("scripts/ocr_eval/run_ocr_privacy_evidence.py"),
            "--output",
            output_path.to_str().unwrap(),
            "--summary-output",
            alias_path.to_str().unwrap(),
            "--python-command",
            &default_python_command(),
            "--mock",
        ])
        .assert()
        .failure();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(stdout.is_empty());
    assert_eq!(stderr.trim(), "ocr privacy evidence summary path must differ from report path");
    assert_eq!(fs::read_to_string(&output_path).unwrap(), "stale primary Jane Example");
}

#[test]
fn ocr_privacy_evidence_missing_image_removes_stale_summary_without_leaks() {
    let dir = tempdir().unwrap();
    let report_path = dir.path().join("report.json");
    let summary_path = dir.path().join("summary-Jane-Example.json");
    fs::write(&summary_path, "Jane Example stale summary").unwrap();

    let assert = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-privacy-evidence",
            "--image-path",
            dir.path().join("missing.png").to_str().unwrap(),
            "--runner-path",
            &repo_path("scripts/ocr_eval/run_ocr_privacy_evidence.py"),
            "--output",
            report_path.to_str().unwrap(),
            "--summary-output",
            summary_path.to_str().unwrap(),
            "--python-command",
            &default_python_command(),
            "--mock",
        ])
        .assert()
        .failure();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(stdout.is_empty());
    assert!(stderr.contains("OCR privacy evidence input image is missing"));
    assert!(!summary_path.exists());
    for forbidden in ["Jane Example", "MRN-12345", dir.path().to_str().unwrap()] {
        assert!(!stdout.contains(forbidden));
        assert!(!stderr.contains(forbidden));
    }
}
```

- [x] **Step 2: Run tests to verify RED**

Run: `/home/azureuser/.cargo/bin/cargo test -p mdid-cli ocr_privacy_evidence_summary -- --nocapture`
Expected: FAIL because `--summary-output` is not yet accepted and no summary artifact is written.

- [x] **Step 3: Implement minimal CLI support**

In `crates/mdid-cli/src/main.rs`, extend `OcrPrivacyEvidenceArgs` with `summary_output_path: Option<PathBuf>`, parse `--summary-output`, update usage/help, reject equivalent paths with `paths_are_same_existing_or_lexical(&output_path, summary_path)` and fixed error `ocr privacy evidence summary path must differ from report path`, clean stale summary before prerequisites and on failures, write a summary JSON after the validated primary report:

```json
{
  "artifact": "ocr_privacy_evidence_summary",
  "ocr_scope": "printed_text_line_extraction_only",
  "privacy_scope": "text_only_pii_detection",
  "privacy_filter_contract": "text_only_pii_detection",
  "network_api_called": false,
  "fixture_count": 1,
  "ready_fixture_count": 1,
  "total_detected_span_count": 4,
  "category_counts": {"EMAIL": 1, "MRN": 1, "NAME": 1, "PHONE": 1},
  "non_goals": ["browser_ui", "desktop_ui", "visual_redaction", "image_pixel_redaction", "handwriting_recognition", "final_pdf_rewrite_export"]
}
```

Derive counts from the validated primary report instead of hardcoding them. Keep stdout PHI/path-safe and include `"summary_written": true` only when the summary file is written.

- [x] **Step 4: Run tests to verify GREEN**

Run: `/home/azureuser/.cargo/bin/cargo test -p mdid-cli ocr_privacy_evidence -- --nocapture`
Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs
git commit -m "feat(cli): add OCR privacy evidence summary output"
```

### Task 2: Documentation and completion truth-sync

**Files:**
- Modify: `README.md`
- Modify: `scripts/ocr_eval/README.md`
- Modify: `docs/superpowers/plans/2026-05-01-ocr-privacy-evidence-summary-output.md`

- [x] **Step 1: Write docs expectations before updating docs**

Add assertions to `crates/mdid-cli/tests/cli_smoke.rs` or extend existing docs test so `scripts/ocr_eval/README.md` contains:

```text
--summary-output <summary.json>
ocr_privacy_evidence_summary
CLI/runtime evidence only
not Browser/Web execution
not Desktop execution
not visual redaction
not image pixel redaction
not final PDF rewrite/export
```

- [x] **Step 2: Run docs test to verify RED**

Run: `/home/azureuser/.cargo/bin/cargo test -p mdid-cli ocr_privacy_evidence_docs -- --nocapture`
Expected: FAIL until docs mention the new summary artifact.

- [x] **Step 3: Update docs and completion truth-sync**

Update `scripts/ocr_eval/README.md` with the command:

```bash
mdid-cli ocr-privacy-evidence \
  --image-path scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png \
  --runner-path scripts/ocr_eval/run_ocr_privacy_evidence.py \
  --output /tmp/ocr-privacy-evidence.json \
  --summary-output /tmp/ocr-privacy-evidence-summary.json \
  --python-command python3 \
  --mock
```

Update `README.md` completion evidence: add one completed CLI/runtime requirement to both numerator and denominator (current CLI evidence fraction `109/114 -> 110/115 = 95%` floor if still current), keep Browser/Web 99%, Desktop 99%, Overall 97% unless actual landed arithmetic supports a different number, and explicitly state Browser/Web/Desktop receive `+0%` because this is CLI/runtime-only evidence.

- [x] **Step 4: Run final verification**

Run:

```bash
/home/azureuser/.cargo/bin/cargo test -p mdid-cli ocr_privacy_evidence -- --nocapture
python scripts/ocr_eval/run_ocr_privacy_evidence.py --image-path scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png --output /tmp/ocr-privacy-evidence.json --mock
python -m py_compile scripts/ocr_eval/run_ocr_privacy_evidence.py
rm -rf scripts/ocr_eval/__pycache__ scripts/privacy_filter/__pycache__ tests/__pycache__
git diff --check
```

Expected: all commands pass, no stale `__pycache__`, no PHI/path leaks in tested CLI outputs.

- [x] **Step 5: Commit docs**

```bash
git add README.md scripts/ocr_eval/README.md crates/mdid-cli/tests/cli_smoke.rs docs/superpowers/plans/2026-05-01-ocr-privacy-evidence-summary-output.md
git commit -m "docs: truth-sync OCR privacy evidence summary output"
```
