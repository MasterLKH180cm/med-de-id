# PDF Export Validation Evidence Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add CLI-visible validation evidence for clean text-layer PDF byte export so PDF rewrite/export is verified after writing bytes.

**Architecture:** Keep the existing bounded clean text-layer PDF export path: only PDFs with no review candidates and all pages text-layer-present may produce output bytes. After writing `--output-pdf-path`, re-parse the written bytes with `PdfDeidentificationService` and add PHI-safe validation fields to the report/stdout; fail closed and remove the output if validation cannot prove the exported bytes remain parseable and eligible.

**Tech Stack:** Rust workspace, `mdid-cli`, `mdid-application`, `lopdf` via existing PDF adapter, serde_json, Cargo integration tests.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - In `run_deidentify_pdf`, after writing clean PDF bytes, read/re-parse the output and emit `rewrite_validation` with parseability, byte count, page count, and review-candidate count.
  - On validation failure, remove the output PDF and return a PHI-safe error.
- Modify: `crates/mdid-cli/tests/cli_pdf.rs`
  - Add a CLI integration test proving clean text-layer output bytes are written and validation evidence is present without path/PHI leaks.

### Task 1: CLI clean PDF export validation evidence

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/cli_pdf.rs`

- [x] **Step 1: Write the failing test**

Add this test to `crates/mdid-cli/tests/cli_pdf.rs`:

```rust
#[test]
fn cli_deidentify_pdf_validates_clean_text_layer_output_pdf_without_path_or_phi_leaks() {
    let dir = tempdir().unwrap();
    let pdf_path = dir.path().join("clean-record.pdf");
    let report_path = dir.path().join("clean-report.json");
    let output_pdf_path = dir.path().join("clean-output.pdf");
    let mut pdf = text_layer_pdf_fixture();
    let needle = b"Alice Smith";
    let offset = pdf
        .windows(needle.len())
        .position(|window| window == needle)
        .unwrap();
    pdf[offset..offset + needle.len()].copy_from_slice(b"ClinicNote");
    fs::write(&pdf_path, pdf).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .arg("deidentify-pdf")
        .arg("--pdf-path")
        .arg(&pdf_path)
        .arg("--source-name")
        .arg("clean-record.pdf")
        .arg("--report-path")
        .arg(&report_path)
        .arg("--output-pdf-path")
        .arg(&output_pdf_path)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output_pdf_path.exists());
    let exported = fs::read(&output_pdf_path).unwrap();
    assert!(exported.starts_with(b"%PDF"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stdout_json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(stdout_json["rewrite_status"], "clean_text_layer_pdf_bytes_available");
    assert_eq!(stdout_json["rewrite_validation"]["validated"], true);
    assert_eq!(stdout_json["rewrite_validation"]["parseable_pdf"], true);
    assert_eq!(stdout_json["rewrite_validation"]["review_queue_len"], 0);
    assert_eq!(stdout_json["rewrite_validation"]["output_byte_count"].as_u64().unwrap(), exported.len() as u64);
    assert!(!stdout.contains(output_pdf_path.to_string_lossy().as_ref()));
    assert!(!stdout.contains("clean-output.pdf"));
    assert!(!stdout.contains("ClinicNote"));
    assert!(!stdout.contains("Alice Smith"));

    let report = fs::read_to_string(&report_path).unwrap();
    let report_json: serde_json::Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report_json["rewrite_available"], true);
    assert_eq!(report_json["rewrite_validation"]["validated"], true);
    assert_eq!(report_json["rewrite_validation"]["parseable_pdf"], true);
    assert_eq!(report_json["rewrite_validation"]["review_queue_len"], 0);
    assert_eq!(report_json["rewrite_validation"]["output_byte_count"].as_u64().unwrap(), exported.len() as u64);
    assert!(!report.contains(output_pdf_path.to_string_lossy().as_ref()));
    assert!(!report.contains("clean-output.pdf"));
    assert!(!report.contains("ClinicNote"));
    assert!(!report.contains("Alice Smith"));
}
```

- [x] **Step 2: Run test to verify it fails**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-cli cli_deidentify_pdf_validates_clean_text_layer_output_pdf_without_path_or_phi_leaks --test cli_pdf -- --nocapture`

Expected: FAIL because `rewrite_validation` is absent.

- [x] **Step 3: Write minimal implementation**

In `crates/mdid-cli/src/main.rs`, add a PHI-safe validation object for successful clean PDF exports:

```json
{
  "validated": true,
  "parseable_pdf": true,
  "page_count": 1,
  "review_queue_len": 0,
  "output_byte_count": 1234
}
```

Implementation requirements:
- only validate after bytes are written;
- read the output path back from disk;
- call `PdfDeidentificationService.deidentify_bytes(&written_bytes, "exported.pdf")`;
- require `rewrite_status == clean_text_layer_pdf_bytes_available` and `review_queue.len() == 0`;
- if validation fails, remove the output file and return a PHI-safe error message containing `PDF rewrite validation failed` but no raw paths, source names, or extracted text;
- include `rewrite_validation` in both stdout JSON and report JSON for all PDF CLI runs; use `null` when no PDF bytes are exported.

- [x] **Step 4: Run test to verify it passes**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-cli cli_deidentify_pdf_validates_clean_text_layer_output_pdf_without_path_or_phi_leaks --test cli_pdf -- --nocapture`

Expected: PASS.

- [x] **Step 5: Run regression tests**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-cli --test cli_pdf -- --nocapture`

Expected: PASS.

- [x] **Step 6: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_pdf.rs docs/superpowers/plans/2026-05-02-pdf-export-validation-evidence.md
git commit -m "feat(cli): validate clean pdf export bytes"
```

## Self-Review

- Spec coverage: Advances full PDF rewrite/export by adding repository-visible byte-level export validation for the bounded clean text-layer PDF path.
- Placeholder scan: no TBD/TODO placeholders.
- Type consistency: command, report, stdout, and validation field names are consistent.
