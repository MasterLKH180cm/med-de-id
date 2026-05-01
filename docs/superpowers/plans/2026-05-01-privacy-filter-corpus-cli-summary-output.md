# Privacy Filter Corpus CLI Summary Output Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an optional PHI-safe `--summary-output` artifact to `mdid-cli privacy-filter-corpus` so the existing synthetic text-only Privacy Filter corpus POC can emit a compact aggregate readiness summary without raw fixture text, spans, previews, paths, or fixture filenames.

**Architecture:** Extend only the existing Rust CLI wrapper around `scripts/privacy_filter/run_synthetic_corpus.py`. The command will continue to execute and validate the aggregate corpus report, sanitize fixture IDs in the primary report, and, only after successful validation, optionally write a second allowlisted summary JSON derived from the validated wrapper report. Failure paths must remove stale primary and summary artifacts.

**Tech Stack:** Rust `mdid-cli`, `serde_json`, existing Python Privacy Filter synthetic corpus runner, Cargo smoke tests.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `summary_output: Option<PathBuf>` to `PrivacyFilterCorpusArgs`.
  - Parse optional `--summary-output <path>` in `parse_privacy_filter_corpus_args`.
  - Remove stale summary output before prerequisite checks and on every failure path.
  - Add `build_privacy_filter_corpus_summary(&Value) -> Value` producing a strict PHI-safe aggregate summary.
  - Write the summary only after the primary corpus report validates and is written.
  - Update command usage text to mention `--summary-output`.
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
  - Add a success smoke test proving the summary output is written, bounded, aggregate-only, and free of raw synthetic PHI/paths/fixture filenames.
  - Add a failure smoke test proving stale summary output is removed when prerequisites fail.
- Modify: `README.md`
  - Truth-sync the current `privacy-filter-corpus` evidence to mention the optional PHI-safe summary artifact.
  - Re-evaluate completion fractions honestly: this adds one CLI/runtime Privacy Filter corpus artifact requirement and completes it in the same round; integer CLI/Browser/Desktop/Overall may remain unchanged after conservative floor arithmetic.

### Task 1: Add optional PHI-safe summary artifact for `privacy-filter-corpus`

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
- Modify: `README.md`

- [ ] **Step 1: Write the failing success-path smoke test**

Add this test near the existing `privacy_filter_corpus` tests in `crates/mdid-cli/tests/cli_smoke.rs`:

```rust
#[test]
fn privacy_filter_corpus_writes_phi_safe_summary_output() {
    let dir = tempdir().unwrap();
    let report_path = dir.path().join("privacy-filter-corpus.json");
    let summary_path = dir.path().join("privacy-filter-corpus-summary.json");

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "privacy-filter-corpus",
            "--fixture-dir",
            &repo_path("scripts/privacy_filter/fixtures/corpus"),
            "--runner-path",
            &repo_path("scripts/privacy_filter/run_synthetic_corpus.py"),
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

    assert_eq!(summary["artifact"], "privacy_filter_corpus_summary");
    assert_eq!(summary["engine"], "fallback_synthetic_patterns");
    assert_eq!(summary["scope"], "text_only_synthetic_corpus");
    assert_eq!(summary["network_api_called"], false);
    assert_eq!(summary["fixture_count"], 2);
    assert!(summary["total_detected_span_count"].as_u64().unwrap() > 0);
    assert!(summary["category_counts"]["NAME"].as_u64().unwrap() > 0);
    assert!(summary["non_goals"].as_array().unwrap().contains(&serde_json::json!("ocr")));
    assert!(summary["non_goals"].as_array().unwrap().contains(&serde_json::json!("visual_redaction")));
    assert!(summary.get("fixtures").is_none());
    assert!(summary.get("masked_text").is_none());
    assert!(summary.get("spans").is_none());
    assert!(summary.get("preview").is_none());

    for unsafe_text in [
        "Jane Example",
        "Alice Smith",
        "MRN-12345",
        "MRN-001",
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
/home/azureuser/.cargo/bin/cargo test -p mdid-cli privacy_filter_corpus_writes_phi_safe_summary_output -- --nocapture
```

Expected: FAIL with `unknown flag` or missing summary output because `--summary-output` is not implemented.

- [ ] **Step 3: Write the failing stale-summary cleanup test**

Add this test near the new success test:

```rust
#[test]
fn privacy_filter_corpus_removes_stale_summary_on_prerequisite_failure() {
    let dir = tempdir().unwrap();
    let report_path = dir.path().join("privacy-filter-corpus.json");
    let summary_path = dir.path().join("privacy-filter-corpus-summary.json");
    fs::write(&summary_path, "Patient Jane Example MRN-12345").unwrap();

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "privacy-filter-corpus",
            "--fixture-dir",
            dir.path().join("missing-fixtures").to_str().unwrap(),
            "--runner-path",
            &repo_path("scripts/privacy_filter/run_synthetic_corpus.py"),
            "--report-path",
            report_path.to_str().unwrap(),
            "--summary-output",
            summary_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(!summary_path.exists());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("Jane Example"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("Jane Example"));
}
```

- [ ] **Step 4: Run the cleanup test and verify RED**

Run:

```bash
/home/azureuser/.cargo/bin/cargo test -p mdid-cli privacy_filter_corpus_removes_stale_summary_on_prerequisite_failure -- --nocapture
```

Expected: FAIL because `--summary-output` is not parsed and/or stale summary cleanup is not implemented.

- [ ] **Step 5: Implement minimal CLI support**

In `crates/mdid-cli/src/main.rs`, update `PrivacyFilterCorpusArgs` to include:

```rust
summary_output: Option<PathBuf>,
```

In `parse_privacy_filter_corpus_args`, initialize:

```rust
let mut summary_output = None;
```

Add this flag parser arm:

```rust
"--summary-output" => summary_output = Some(non_blank_path(value, "--summary-output")?),
```

Return it in `PrivacyFilterCorpusArgs`.

In `run_privacy_filter_corpus`, remove stale summary output before prerequisite checks and on any error:

```rust
if let Some(summary_output) = &args.summary_output {
    let _ = fs::remove_file(summary_output);
}
```

After `validate_privacy_filter_corpus_report(...)` succeeds in `run_privacy_filter_corpus_inner`, write:

```rust
if let Some(summary_output) = &args.summary_output {
    let summary_report = build_privacy_filter_corpus_summary(&value);
    let summary_text = serde_json::to_string_pretty(&summary_report)
        .map_err(|err| format!("failed to render privacy filter corpus summary: {err}"))?;
    let summary_text_with_newline = format!("{summary_text}\n");
    if summary_text_with_newline.len() > PRIVACY_FILTER_RUNNER_STDOUT_MAX_BYTES {
        return Err("privacy filter corpus summary exceeded limit".to_string());
    }
    fs::write(summary_output, summary_text_with_newline)
        .map_err(|err| format!("failed to write privacy filter corpus summary: {err}"))?;
}
```

Add the helper:

```rust
fn build_privacy_filter_corpus_summary(value: &Value) -> Value {
    json!({
        "artifact": "privacy_filter_corpus_summary",
        "engine": value["engine"],
        "scope": value["scope"],
        "fixture_count": value["fixture_count"],
        "total_detected_span_count": value["total_detected_span_count"],
        "category_counts": value["category_counts"],
        "network_api_called": false,
        "non_goals": value["non_goals"],
    })
}
```

Update help/usage text for `privacy-filter-corpus` to include `[--summary-output <summary.json>]`.

- [ ] **Step 6: Run targeted tests and verify GREEN**

Run:

```bash
/home/azureuser/.cargo/bin/cargo test -p mdid-cli privacy_filter_corpus_writes_phi_safe_summary_output -- --nocapture
/home/azureuser/.cargo/bin/cargo test -p mdid-cli privacy_filter_corpus_removes_stale_summary_on_prerequisite_failure -- --nocapture
```

Expected: both PASS.

- [ ] **Step 7: Run broader relevant CLI tests**

Run:

```bash
/home/azureuser/.cargo/bin/cargo test -p mdid-cli privacy_filter_corpus -- --nocapture
```

Expected: PASS; existing `privacy-filter-corpus` behavior remains compatible.

- [ ] **Step 8: Truth-sync README completion and evidence**

Update `README.md` current status/evidence so it states:

```markdown
Verification evidence for the Privacy Filter corpus CLI wrapper truth-sync landed on this branch: `mdid-cli privacy-filter-corpus --fixture-dir scripts/privacy_filter/fixtures/corpus --runner-path scripts/privacy_filter/run_synthetic_corpus.py --report-path <report.json> [--summary-output <summary.json>]` is a bounded CLI wrapper around the existing synthetic text-only corpus runner. The wrapper executes only that local runner, validates the strict aggregate allowlist/schema and size bounds, writes the report JSON to the requested path, and can optionally write a PHI-safe aggregate-only `privacy_filter_corpus_summary` artifact derived from the validated wrapper report. The primary report uses sanitized fixture IDs; the optional summary omits fixture arrays, raw fixture text, masked text, spans, previews, paths, and fixture filenames. This remains CLI/runtime text-only PII detection evidence only; it is not OCR, visual redaction, image pixel redaction, browser UI, desktop UI, or final PDF rewrite/export.
```

Completion arithmetic in README should remain conservative unless the existing fraction table is explicitly updated. If adding this requirement to the CLI rubric and completing it in the same round, use fraction accounting such as `CLI 95/100 -> 96/101 = 95% floor`, `Overall 97/100 -> 98/101 = 97% floor`, with Browser/Web and Desktop unchanged at 99%.

- [ ] **Step 9: Final verification and commit**

Run:

```bash
/home/azureuser/.cargo/bin/cargo test -p mdid-cli privacy_filter_corpus -- --nocapture
python scripts/privacy_filter/run_synthetic_corpus.py --fixture-dir scripts/privacy_filter/fixtures/corpus --output /tmp/privacy-filter-corpus.json
python -m py_compile scripts/privacy_filter/run_synthetic_corpus.py scripts/privacy_filter/run_privacy_filter.py scripts/privacy_filter/validate_privacy_filter_output.py
git diff --check
rm -rf scripts/privacy_filter/__pycache__ tests/__pycache__
git status --short
```

Expected: tests and compile pass; only intended files are dirty before commit.

Commit:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs README.md docs/superpowers/plans/2026-05-01-privacy-filter-corpus-cli-summary-output.md
git commit -m "feat(cli): add privacy filter corpus summary output"
```
