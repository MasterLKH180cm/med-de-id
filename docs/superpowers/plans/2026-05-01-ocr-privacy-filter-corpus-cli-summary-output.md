# OCR Privacy Filter Corpus CLI Summary Output Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an optional PHI-safe `--summary-output` artifact to `mdid-cli ocr-to-privacy-filter-corpus` so the bounded PP-OCRv5 mobile synthetic OCR-to-text-PII chain can emit a compact downstream-readiness summary without raw OCR text, masked text, spans, paths, or fixture filenames.

**Architecture:** Extend the existing Rust CLI wrapper only; keep the Python bridge unchanged. The wrapper will continue to execute and validate the existing aggregate bridge report, normalize the primary report, and, only after successful validation, optionally write a second allowlisted summary JSON derived from the wrapper report. Failure paths must remove stale primary and summary artifacts.

**Tech Stack:** Rust `mdid-cli`, `serde_json`, existing Python synthetic OCR/Privacy Filter runners, Cargo test smoke coverage.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `summary_output: Option<PathBuf>` to `OcrToPrivacyFilterCorpusArgs`.
  - Parse optional `--summary-output <path>`.
  - Remove stale summary output before prerequisites and on every failure path.
  - Add `build_ocr_to_privacy_filter_corpus_summary(&Value) -> Value` producing a strict PHI-safe summary.
  - Write the summary only after the primary wrapper report validates and is written.
  - Update usage text to mention `--summary-output`.
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
  - Add a smoke test that successful CLI execution writes the summary output, verifies bounded fields, and denies raw synthetic PHI and paths.
  - Add a failure test proving stale summary output is removed when prerequisites fail.
- Modify: `README.md`
  - Truth-sync the current evidence paragraph for `mdid-cli ocr-to-privacy-filter-corpus` to mention the optional PHI-safe summary artifact.
  - Keep completion arithmetic honest: CLI remains 95%, Browser/Web 99%, Desktop app 99%, Overall 97% unless repository-visible fraction accounting is separately changed.

### Task 1: Add optional CLI summary artifact for OCR-to-Privacy-Filter corpus

**Files:**
- Modify: `crates/mdid-cli/src/main.rs:141-148`
- Modify: `crates/mdid-cli/src/main.rs:594-643`
- Modify: `crates/mdid-cli/src/main.rs:1111-1198`
- Modify: `crates/mdid-cli/src/main.rs:2746-2748`
- Test: `crates/mdid-cli/tests/cli_smoke.rs`

- [ ] **Step 1: Write the failing success-path smoke test**

Add this test to `crates/mdid-cli/tests/cli_smoke.rs` near the existing `ocr-to-privacy-filter-corpus` smoke tests:

```rust
#[test]
fn ocr_to_privacy_filter_corpus_writes_phi_safe_summary_output() {
    let dir = tempdir().unwrap();
    let report_path = dir.path().join("ocr-to-privacy-filter-corpus.json");
    let summary_path = dir.path().join("ocr-to-privacy-filter-corpus-summary.json");

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-to-privacy-filter-corpus",
            "--fixture-dir",
            &repo_path("scripts/ocr_eval/fixtures/corpus"),
            "--ocr-runner-path",
            &repo_path("scripts/ocr_eval/run_ocr_handoff_corpus.py"),
            "--privacy-runner-path",
            &repo_path("scripts/privacy_filter/run_privacy_filter.py"),
            "--bridge-runner-path",
            &repo_path("scripts/ocr_eval/run_ocr_to_privacy_filter_corpus.py"),
            "--report-path",
            report_path.to_str().unwrap(),
            "--summary-output",
            summary_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let summary_text = fs::read_to_string(&summary_path).unwrap();
    let summary: serde_json::Value = serde_json::from_str(&summary_text).unwrap();

    assert_eq!(summary["artifact"], "ocr_to_privacy_filter_corpus_summary");
    assert_eq!(summary["ocr_scope"], "printed_text_line_extraction_only");
    assert_eq!(summary["privacy_scope"], "text_only_pii_detection");
    assert_eq!(summary["privacy_filter_contract"], "text_only_normalized_input");
    assert_eq!(summary["network_api_called"], false);
    assert_eq!(summary["fixture_count"], 2);
    assert_eq!(summary["ready_fixture_count"], 2);
    assert!(summary["total_detected_span_count"].as_u64().unwrap() > 0);
    assert!(summary.get("fixtures").is_none());
    assert!(summary.get("spans").is_none());
    assert!(summary.get("masked_text").is_none());
    assert!(summary.get("normalized_text").is_none());

    for unsafe_text in [
        "Jane Example",
        "MRN-12345",
        "jane@example.com",
        "555-123-4567",
        "synthetic_patient_label_",
        "/home/",
        "/tmp/",
        "fixtures/",
    ] {
        assert!(!summary_text.contains(unsafe_text), "summary leaked {unsafe_text}");
    }
}
```

- [ ] **Step 2: Run the targeted test and verify RED**

Run:

```bash
cargo test -p mdid-cli ocr_to_privacy_filter_corpus_writes_phi_safe_summary_output -- --nocapture
```

Expected: FAIL with `unknown flag` or no summary file because `--summary-output` is not implemented yet.

- [ ] **Step 3: Implement minimal summary-output support**

In `crates/mdid-cli/src/main.rs`, make these edits:

```rust
struct OcrToPrivacyFilterCorpusArgs {
    fixture_dir: PathBuf,
    ocr_runner_path: PathBuf,
    privacy_runner_path: PathBuf,
    bridge_runner_path: PathBuf,
    report_path: PathBuf,
    summary_output: Option<PathBuf>,
    python_command: String,
}
```

Update `parse_ocr_to_privacy_filter_corpus_args`:

```rust
let mut summary_output = None;
```

Add this match arm:

```rust
"--summary-output" => summary_output = Some(non_blank_path(value, "--summary-output")?),
```

Include it in the returned struct:

```rust
summary_output,
```

Update `run_ocr_to_privacy_filter_corpus` so stale summary files are removed before prerequisites and on failure:

```rust
let _ = fs::remove_file(&args.report_path);
if let Some(summary_output) = &args.summary_output {
    let _ = fs::remove_file(summary_output);
}
```

and in the failure block:

```rust
if result.is_err() {
    let _ = fs::remove_file(&args.report_path);
    if let Some(summary_output) = &args.summary_output {
        let _ = fs::remove_file(summary_output);
    }
}
```

Add this helper near `normalize_ocr_to_privacy_filter_corpus_report`:

```rust
fn build_ocr_to_privacy_filter_corpus_summary(wrapper_report: &Value) -> Value {
    json!({
        "artifact": "ocr_to_privacy_filter_corpus_summary",
        "ocr_candidate": wrapper_report["ocr_candidate"],
        "ocr_engine": wrapper_report["ocr_engine"],
        "ocr_scope": wrapper_report["ocr_scope"],
        "privacy_filter_engine": wrapper_report["privacy_filter_engine"],
        "privacy_filter_contract": wrapper_report["privacy_filter_contract"],
        "privacy_scope": wrapper_report["privacy_scope"],
        "fixture_count": wrapper_report["fixture_count"],
        "ready_fixture_count": wrapper_report["ready_fixture_count"],
        "total_detected_span_count": wrapper_report["total_detected_span_count"],
        "category_counts": wrapper_report["category_counts"],
        "privacy_filter_category_counts": wrapper_report["privacy_filter_category_counts"],
        "network_api_called": false,
        "non_goals": wrapper_report["non_goals"],
    })
}
```

After writing the primary wrapper report, write the optional summary:

```rust
if let Some(summary_output) = &args.summary_output {
    let summary_output_value = build_ocr_to_privacy_filter_corpus_summary(&wrapper_report);
    let summary_text = serde_json::to_string_pretty(&summary_output_value)
        .map_err(|err| format!("failed to render summary output: {err}"))?;
    if summary_text.len() > PRIVACY_FILTER_RUNNER_STDOUT_MAX_BYTES {
        return Err("OCR to privacy filter corpus summary exceeded limit".to_string());
    }
    fs::write(summary_output, format!("{summary_text}\n"))
        .map_err(|_| "OCR to privacy filter corpus failed".to_string())?;
}
```

Update the usage string line for this command to include:

```text
[--summary-output <summary.json>]
```

- [ ] **Step 4: Run targeted test and verify GREEN**

Run:

```bash
cargo test -p mdid-cli ocr_to_privacy_filter_corpus_writes_phi_safe_summary_output -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit Task 1**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs
git commit -m "feat(cli): add ocr privacy filter corpus summary output"
```

### Task 2: Harden summary-output stale cleanup and docs truth-sync

**Files:**
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
- Modify: `README.md:64-92`

- [ ] **Step 1: Write the failing stale-summary cleanup test**

Add this test to `crates/mdid-cli/tests/cli_smoke.rs` near the other `ocr-to-privacy-filter-corpus` failure tests:

```rust
#[test]
fn ocr_to_privacy_filter_corpus_removes_stale_summary_on_prerequisite_failure() {
    let dir = tempdir().unwrap();
    let report_path = dir.path().join("ocr-to-privacy-filter-corpus.json");
    let summary_path = dir.path().join("ocr-to-privacy-filter-corpus-summary.json");
    fs::write(&report_path, "stale raw Jane Example").unwrap();
    fs::write(&summary_path, "stale raw Jane Example").unwrap();

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-to-privacy-filter-corpus",
            "--fixture-dir",
            dir.path().join("missing-fixtures").to_str().unwrap(),
            "--ocr-runner-path",
            &repo_path("scripts/ocr_eval/run_ocr_handoff_corpus.py"),
            "--privacy-runner-path",
            &repo_path("scripts/privacy_filter/run_privacy_filter.py"),
            "--bridge-runner-path",
            &repo_path("scripts/ocr_eval/run_ocr_to_privacy_filter_corpus.py"),
            "--report-path",
            report_path.to_str().unwrap(),
            "--summary-output",
            summary_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(!report_path.exists());
    assert!(!summary_path.exists());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(!stdout.contains("Jane Example"));
    assert!(!stderr.contains("Jane Example"));
}
```

- [ ] **Step 2: Run the stale-cleanup test and verify RED or existing GREEN**

Run:

```bash
cargo test -p mdid-cli ocr_to_privacy_filter_corpus_removes_stale_summary_on_prerequisite_failure -- --nocapture
```

Expected: If Task 1 already removed stale summary files before prerequisites, this may PASS immediately; that is acceptable because Task 1 intentionally included the cleanup behavior. If it fails, implement the cleanup shown in Task 1.

- [ ] **Step 3: Update README evidence without changing completion percentages**

In `README.md`, update the OCR-to-Privacy-Filter corpus bridge evidence paragraph to include this exact bounded claim:

```markdown
The wrapper also accepts optional `--summary-output <summary.json>` and, only after the primary bridge report validates, writes a second PHI-safe summary artifact with aggregate readiness counts, category counts, bounded OCR/Privacy Filter scope metadata, `network_api_called: false`, and explicit non-goals. The summary omits raw OCR text, normalized text, masked text, spans/previews, fixture IDs, fixture filenames, and local paths.
```

Do not raise Browser/Web or Desktop completion. Keep completion snapshot at CLI 95%, Browser/Web 99%, Desktop app 99%, Overall 97% unless separate fraction accounting justifies a change.

- [ ] **Step 4: Run relevant verification**

Run:

```bash
cargo test -p mdid-cli ocr_to_privacy_filter_corpus -- --nocapture
python scripts/ocr_eval/run_ocr_to_privacy_filter_corpus.py --fixture-dir scripts/ocr_eval/fixtures/corpus --ocr-runner-path scripts/ocr_eval/run_ocr_handoff_corpus.py --privacy-runner-path scripts/privacy_filter/run_privacy_filter.py --output /tmp/ocr-to-privacy-filter-corpus-bridge.json
python -m py_compile scripts/ocr_eval/run_ocr_to_privacy_filter_corpus.py scripts/privacy_filter/run_privacy_filter.py
git diff --check
```

Expected: all commands PASS. The Python bridge output remains the existing aggregate bridge contract; the new summary artifact is produced by the Rust CLI wrapper only.

- [ ] **Step 5: Commit Task 2**

Run:

```bash
git add crates/mdid-cli/tests/cli_smoke.rs README.md
git commit -m "test(cli): harden ocr privacy filter corpus summary cleanup"
```

## Self-Review

- Spec coverage: This plan covers optional CLI summary artifact creation, PHI-safe fields, stale artifact cleanup, usage discoverability, README truth-sync, and verification. It does not add Browser/Web/Desktop execution, OCR quality claims, visual redaction, image pixel redaction, handwriting recognition, final PDF rewrite/export, or workflow orchestration semantics.
- Placeholder scan: No TBD/TODO/implement-later placeholders remain.
- Type consistency: `summary_output: Option<PathBuf>` is used consistently in the args struct, parser, runner cleanup, and optional write path.
