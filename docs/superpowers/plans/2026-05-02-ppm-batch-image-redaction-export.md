# PPM Batch Image Redaction Export Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded local multi-file PPM P6 bbox-driven redaction CLI export command that rewrites each image's bytes, continues after per-item failures, and writes PHI-safe aggregate evidence.

**Architecture:** Reuse the existing `redact_ppm_p6_bytes_with_verification` adapter and existing single-file `redact-image-ppm` semantics. Add one CLI command that accepts a manifest of `{input, output, regions}` items, processes them sequentially, writes successful redacted PPM bytes, records failure statuses without raw paths, and emits a PHI-safe summary.

**Tech Stack:** Rust workspace, `mdid-cli`, `mdid-adapters`, `mdid-domain`, serde_json, Cargo integration tests.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `RedactImagePpmBatchArgs` and `CliCommand::RedactImagePpmBatch`.
  - Parse `redact-image-ppm-batch --manifest-json <json> --summary-output <path>`.
  - Implement manifest validation and batch execution using existing PPM redaction adapter.
  - Summary must not include input paths, output paths, bbox arrays, source names, or image bytes.
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
  - Add RED/GREEN smoke tests for mixed success/failure continuation and PHI-safe summary.

### Task 1: CLI PPM batch redaction command

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/cli_smoke.rs`

- [x] **Step 1: Write the failing test**

Add a test named `redact_image_ppm_batch_continues_after_item_failure_without_path_leaks` to `crates/mdid-cli/tests/cli_smoke.rs`. The test should:

```rust
#[test]
fn redact_image_ppm_batch_continues_after_item_failure_without_path_leaks() {
    let temp = tempfile::tempdir().expect("tempdir");
    let input_ok = temp.path().join("patient-jane-face.ppm");
    let input_bad = temp.path().join("patient-jane-bad.ppm");
    let output_ok = temp.path().join("patient-jane-face-redacted.ppm");
    let output_bad = temp.path().join("patient-jane-bad-redacted.ppm");
    let summary = temp.path().join("patient-jane-summary.json");

    std::fs::write(&input_ok, b"P6\n2 1\n255\n\x01\x02\x03\x04\x05\x06").expect("write ok ppm");
    std::fs::write(&input_bad, b"not a ppm").expect("write bad ppm");

    let manifest = serde_json::json!([
        {
            "input": input_ok,
            "output": output_ok,
            "regions": [{"x": 0, "y": 0, "width": 1, "height": 1}]
        },
        {
            "input": input_bad,
            "output": output_bad,
            "regions": [{"x": 0, "y": 0, "width": 1, "height": 1}]
        }
    ]);

    let output = run_cli([
        "redact-image-ppm-batch",
        "--manifest-json",
        &manifest.to_string(),
        "--summary-output",
        summary.to_str().expect("summary path"),
    ]);

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let redacted = std::fs::read(&output_ok).expect("redacted output written");
    assert_eq!(&redacted[11..14], &[0, 0, 0]);
    assert!(!output_bad.exists(), "failed item must not write output");

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("image_redaction_batch_summary"));
    assert!(!stdout.contains("patient-jane"));

    let summary_text = std::fs::read_to_string(&summary).expect("summary written");
    assert!(summary_text.contains("image_redaction_batch_summary"));
    assert!(summary_text.contains("\"total_item_count\": 2"));
    assert!(summary_text.contains("\"succeeded_item_count\": 1"));
    assert!(summary_text.contains("\"failed_item_count\": 1"));
    assert!(summary_text.contains("\"status\": \"redacted\""));
    assert!(summary_text.contains("\"status\": \"failed\""));
    assert!(summary_text.contains("\"error_code\": \"invalid_ppm_redaction\""));
    assert!(!summary_text.contains("patient-jane"));
    assert!(!summary_text.contains("regions"));
    assert!(!summary_text.contains("bbox"));
    assert!(!summary_text.contains("P6"));
}
```

- [x] **Step 2: Run test to verify it fails**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-cli redact_image_ppm_batch_continues_after_item_failure_without_path_leaks --test cli_smoke -- --nocapture`

Expected: FAIL because `redact-image-ppm-batch` is an unknown command.

- [x] **Step 3: Write minimal implementation**

Implement:
- `RedactImagePpmBatchArgs { manifest_json: String, summary_output: PathBuf }`
- `ImageRedactionBatchManifestItem { input: PathBuf, output: PathBuf, regions: Vec<ImageRedactionRegion> }`
- `parse_redact_image_ppm_batch_args`
- `run_redact_image_ppm_batch`

Summary schema:

```json
{
  "artifact": "image_redaction_batch_summary",
  "schema_version": 1,
  "format": "ppm_p6",
  "total_item_count": 2,
  "succeeded_item_count": 1,
  "failed_item_count": 1,
  "raw_paths_included": false,
  "raw_regions_included": false,
  "image_bytes_included": false,
  "items": [
    {"item_index": 1, "status": "redacted", "visual_verification": {"format": "ppm_p6", "width": 2, "height": 1, "redacted_region_count": 1, "redacted_pixel_count": 1, "unchanged_pixel_count": 1, "output_byte_count": 17, "verified_changed_pixels_within_regions": true}},
    {"item_index": 2, "status": "failed", "error_code": "invalid_ppm_redaction"}
  ]
}
```

- [x] **Step 4: Run test to verify it passes**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-cli redact_image_ppm_batch_continues_after_item_failure_without_path_leaks --test cli_smoke -- --nocapture`

Expected: PASS.

- [x] **Step 5: Run regression tests**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-cli redact_image_ppm --test cli_smoke -- --nocapture`

Expected: PASS for existing single-file and new batch PPM redaction smoke tests.

- [x] **Step 6: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs docs/superpowers/plans/2026-05-02-ppm-batch-image-redaction-export.md
git commit -m "feat(cli): add ppm batch image redaction export"
```

## Self-Review

- Spec coverage: advances actual bbox-driven pixel redaction and media-byte rewrite/export for bounded PPM images with multi-file error recovery.
- Placeholder scan: no TBD/TODO placeholders.
- Type consistency: command, manifest, and summary names are consistent across tasks.
