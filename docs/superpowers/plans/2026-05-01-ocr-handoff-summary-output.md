# OCR Handoff Summary Output Implementation Plan

> **Implementation note:** Implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an optional PHI-safe aggregate-only `--summary-output <summary.json>` artifact to `mdid-cli ocr-handoff` for bounded PP-OCRv5 mobile single-image OCR handoff evidence.

**Architecture:** Keep the existing primary `ocr-handoff` report unchanged because it intentionally carries extracted/normalized text for downstream text-only PII detection. Add a secondary summary artifact only after the primary OCR handoff report is successfully built, validated, and written; the summary is strict allowlist JSON with no raw OCR text, source paths, fixture filenames, bbox/image data, or visual/PDF claims.

**Tech Stack:** Rust CLI (`crates/mdid-cli/src/main.rs`), Rust smoke tests (`crates/mdid-cli/tests/cli_smoke.rs`), existing Python OCR fixtures/runners under `scripts/ocr_eval/`, README truth-sync.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `summary_output: Option<PathBuf>` to `OcrHandoffArgs`.
  - Parse `--summary-output` for `ocr-handoff`.
  - Reject same/equivalent report and summary paths before stale cleanup.
  - Clean stale summary artifacts before prerequisites and on failure.
  - Build/write strict `ocr_handoff_summary` JSON only after the primary report is validated and written.
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
  - Add smoke tests for successful PHI-safe summary output, same-path rejection, stale summary cleanup on prerequisite failure, and no raw PHI/path leaks.
- Modify: `README.md`
  - Truth-sync the current completion snapshot and evidence with CLI/runtime-only PP-OCRv5 mobile `ocr-handoff --summary-output` support.
  - Recompute CLI fraction by adding one new completed CLI/runtime requirement to numerator and denominator.
- Create: `docs/superpowers/plans/2026-05-01-ocr-handoff-summary-output.md`
  - This implementation plan.

---

### Task 1: Add `ocr-handoff --summary-output` PHI-safe summary artifact

**Files:**
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
- Modify: `crates/mdid-cli/src/main.rs`

- [x] **Step 1: Write the failing smoke test for successful PHI-safe summary output**

Add this test near the existing `ocr_handoff` smoke tests in `crates/mdid-cli/tests/cli_smoke.rs`:

```rust
#[test]
fn ocr_handoff_writes_phi_safe_summary_output() {
    let dir = tempdir().unwrap();
    let report_path = dir.path().join("ocr-handoff-report.json");
    let summary_path = dir.path().join("ocr-handoff-summary.json");

    let output = Command::new(cargo_bin())
        .args([
            "ocr-handoff",
            "--image-path",
            "scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png",
            "--ocr-runner-path",
            "scripts/ocr_eval/run_small_ocr.py",
            "--handoff-builder-path",
            "scripts/ocr_eval/build_ocr_handoff.py",
            "--report-path",
            report_path.to_str().unwrap(),
            "--summary-output",
            summary_path.to_str().unwrap(),
            "--python-command",
            default_python_command(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stdout.contains(summary_path.to_str().unwrap()));
    assert!(!stderr.contains(summary_path.to_str().unwrap()));

    let summary_text = fs::read_to_string(&summary_path).unwrap();
    let summary: serde_json::Value = serde_json::from_str(&summary_text).unwrap();
    assert_eq!(summary["artifact"], "ocr_handoff_summary");
    assert_eq!(summary["schema_version"], 1);
    assert_eq!(summary["candidate"], "PP-OCRv5_mobile_rec");
    assert_eq!(summary["engine"], "PP-OCRv5-mobile-bounded-spike");
    assert_eq!(summary["scope"], "printed_text_line_extraction_only");
    assert_eq!(summary["privacy_filter_contract"], "text_only_normalized_input");
    assert_eq!(summary["ready_for_text_pii_eval"], true);
    assert!(summary["line_count"].as_u64().unwrap() >= 1);
    assert!(summary["char_count"].as_u64().unwrap() >= 1);
    assert_eq!(summary["network_api_called"], false);
    assert!(summary["non_goals"].as_array().unwrap().contains(&serde_json::Value::String("visual_redaction".to_string())));
    assert!(summary["non_goals"].as_array().unwrap().contains(&serde_json::Value::String("image_pixel_redaction".to_string())));
    assert!(summary["non_goals"].as_array().unwrap().contains(&serde_json::Value::String("final_pdf_rewrite_export".to_string())));

    let keys: std::collections::BTreeSet<_> = summary.as_object().unwrap().keys().cloned().collect();
    assert_eq!(
        keys,
        [
            "artifact",
            "candidate",
            "char_count",
            "engine",
            "line_count",
            "network_api_called",
            "non_goals",
            "privacy_filter_contract",
            "ready_for_text_pii_eval",
            "schema_version",
            "scope",
        ]
        .into_iter()
        .map(String::from)
        .collect()
    );

    for forbidden in [
        "Jane Example",
        "MRN-12345",
        "jane@example.com",
        "555-123-4567",
        "synthetic_printed_phi_line.png",
        "extracted_text",
        "normalized_text",
        "bbox",
        "image_bytes",
        report_path.to_str().unwrap(),
        summary_path.to_str().unwrap(),
    ] {
        assert!(!summary_text.contains(forbidden), "summary leaked {forbidden}");
        assert!(!stdout.contains(forbidden), "stdout leaked {forbidden}");
        assert!(!stderr.contains(forbidden), "stderr leaked {forbidden}");
    }
}
```

- [x] **Step 2: Run RED test**

Run:

```bash
cargo test -p mdid-cli ocr_handoff_writes_phi_safe_summary_output --test cli_smoke -- --nocapture
```

Expected: FAIL because `ocr-handoff` does not yet accept `--summary-output` and does not write the secondary summary artifact.

- [x] **Step 3: Implement minimal CLI support and summary builder**

In `crates/mdid-cli/src/main.rs`:

1. Add `summary_output: Option<PathBuf>` to `OcrHandoffArgs`.
2. In `parse_ocr_handoff_args`, parse `--summary-output` with `non_blank_path(value, "--summary-output")?`.
3. In `run_ocr_handoff`, if `summary_output` exists and is the same/equivalent path as `report_path`, return fixed error `OCR handoff summary path must differ from report path` before cleanup.
4. Remove stale summary before prerequisites; remove stale summary on every error path.
5. After the primary report validates and writes, call `build_ocr_handoff_summary(&report)` and write it to `summary_output`.
6. Summary JSON must be exactly:

```json
{
  "artifact": "ocr_handoff_summary",
  "schema_version": 1,
  "candidate": "PP-OCRv5_mobile_rec",
  "engine": "PP-OCRv5-mobile-bounded-spike",
  "scope": "printed_text_line_extraction_only",
  "privacy_filter_contract": "text_only_normalized_input",
  "ready_for_text_pii_eval": true,
  "line_count": <nonnegative integer from validated report>,
  "char_count": <nonnegative integer from validated report>,
  "network_api_called": false,
  "non_goals": ["ocr_model_quality_proof", "visual_redaction", "image_pixel_redaction", "handwriting_recognition", "final_pdf_rewrite_export", "browser_ui", "desktop_ui"]
}
```

- [x] **Step 4: Run GREEN tests**

Run:

```bash
cargo test -p mdid-cli ocr_handoff_writes_phi_safe_summary_output --test cli_smoke -- --nocapture
cargo test -p mdid-cli ocr_handoff --test cli_smoke -- --nocapture
```

Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs docs/superpowers/plans/2026-05-01-ocr-handoff-summary-output.md
git commit -m "feat(cli): add ocr handoff summary output"
```

---

### Task 2: Harden failure cleanup and same-path rejection

**Files:**
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
- Modify: `crates/mdid-cli/src/main.rs`

- [ ] **Step 1: Write failing tests for same-path rejection and stale summary cleanup**

Add two tests near `ocr_handoff_writes_phi_safe_summary_output`:

```rust
#[test]
fn ocr_handoff_summary_output_rejects_same_report_and_summary_before_cleanup() {
    let dir = tempdir().unwrap();
    let report_path = dir.path().join("ocr-handoff-report.json");
    fs::write(&report_path, "stale Jane Example report").unwrap();

    let output = Command::new(cargo_bin())
        .args([
            "ocr-handoff",
            "--image-path",
            "scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png",
            "--ocr-runner-path",
            "scripts/ocr_eval/run_small_ocr.py",
            "--handoff-builder-path",
            "scripts/ocr_eval/build_ocr_handoff.py",
            "--report-path",
            report_path.to_str().unwrap(),
            "--summary-output",
            report_path.to_str().unwrap(),
            "--python-command",
            default_python_command(),
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("OCR handoff summary path must differ from report path"));
    assert_eq!(fs::read_to_string(&report_path).unwrap(), "stale Jane Example report");
    assert!(String::from_utf8_lossy(&output.stdout).is_empty());
    assert!(!String::from_utf8_lossy(&output.stderr).contains(report_path.to_str().unwrap()));
}

#[test]
fn ocr_handoff_summary_output_missing_image_removes_stale_summary_without_leaks() {
    let dir = tempdir().unwrap();
    let report_path = dir.path().join("ocr-handoff-report.json");
    let summary_path = dir.path().join("ocr-handoff-summary.json");
    fs::write(&summary_path, "stale Jane Example summary").unwrap();

    let output = Command::new(cargo_bin())
        .args([
            "ocr-handoff",
            "--image-path",
            "scripts/ocr_eval/fixtures/missing-patient-Jane-Example.png",
            "--ocr-runner-path",
            "scripts/ocr_eval/run_small_ocr.py",
            "--handoff-builder-path",
            "scripts/ocr_eval/build_ocr_handoff.py",
            "--report-path",
            report_path.to_str().unwrap(),
            "--summary-output",
            summary_path.to_str().unwrap(),
            "--python-command",
            default_python_command(),
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(!summary_path.exists());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    for forbidden in ["Jane Example", "missing-patient-Jane-Example.png", summary_path.to_str().unwrap()] {
        assert!(!stdout.contains(forbidden));
        assert!(!stderr.contains(forbidden));
    }
}
```

- [ ] **Step 2: Run RED tests**

Run:

```bash
cargo test -p mdid-cli ocr_handoff_summary_output --test cli_smoke -- --nocapture
```

Expected: FAIL if cleanup/same-path handling is incomplete.

- [ ] **Step 3: Implement hardening**

Ensure `run_ocr_handoff` uses the same path-equivalence helper already used by adjacent commands. Cleanup must remove stale summary before prerequisite checks and after any failure from prerequisites, runner execution, JSON parsing, validation, primary report write, or secondary summary write.

- [ ] **Step 4: Run GREEN tests**

Run:

```bash
cargo test -p mdid-cli ocr_handoff_summary_output --test cli_smoke -- --nocapture
cargo test -p mdid-cli ocr_handoff --test cli_smoke -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs
git commit -m "test(cli): harden ocr handoff summary cleanup"
```

---

### Task 3: README completion truth-sync

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/plans/2026-05-01-ocr-handoff-summary-output.md`

- [ ] **Step 1: Update README current snapshot**

Update the completion snapshot to state this round adds CLI/runtime-only `mdid-cli ocr-handoff --summary-output` aggregate summary evidence. Use the current README baseline `CLI 120/125 = 96%`; add and complete one new CLI/runtime requirement, yielding `121/126 = 96%` floor. Keep Browser/Web `99%`, Desktop app `99%`, and Overall `97%` unless new product-visible evidence justifies a change.

Add a verification paragraph:

```markdown
Verification evidence for the `mdid-cli ocr-handoff --summary-output` slice landed on this branch: the PP-OCRv5 mobile single-image OCR handoff CLI can now write a secondary aggregate-only `ocr_handoff_summary` artifact with `schema_version: 1`, bounded OCR scope, text-only Privacy Filter readiness metadata, numeric counts, `network_api_called: false`, and explicit non-goals. The primary handoff report remains unchanged for downstream text-only PII detection, while the summary omits raw OCR text, normalized text, source paths, fixture filenames, bbox/image data, raw synthetic PHI, visual redaction, image pixel redaction, handwriting recognition, final PDF rewrite/export, Browser/Web execution, and Desktop execution. Repository-visible verification: `cargo test -p mdid-cli ocr_handoff_summary_output --test cli_smoke -- --nocapture`; broader smoke: `cargo test -p mdid-cli ocr_handoff --test cli_smoke -- --nocapture`.
```

- [ ] **Step 2: Run verification**

Run:

```bash
cargo test -p mdid-cli ocr_handoff_summary_output --test cli_smoke -- --nocapture
cargo test -p mdid-cli ocr_handoff --test cli_smoke -- --nocapture
git diff --check
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add README.md docs/superpowers/plans/2026-05-01-ocr-handoff-summary-output.md
git commit -m "docs: truth-sync ocr handoff summary output"
```
