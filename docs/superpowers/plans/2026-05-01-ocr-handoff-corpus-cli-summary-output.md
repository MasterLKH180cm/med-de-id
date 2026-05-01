# OCR Handoff Corpus CLI Summary Output Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an optional PHI-safe `--summary-output` artifact to `mdid-cli ocr-handoff-corpus` so the bounded PP-OCRv5 mobile synthetic OCR handoff corpus can emit compact downstream-readiness evidence for text-only PII detection without raw OCR text, fixture details, paths, spans, or image data.

**Architecture:** Extend only the existing Rust CLI wrapper for the already-landed Python OCR handoff corpus runner. The command will continue to validate and write the existing aggregate primary report, and only after success optionally write a second allowlisted readiness summary derived from the validated primary report; every failure path must remove stale primary and summary artifacts.

**Tech Stack:** Rust `mdid-cli`, `serde_json`, `assert_cmd`, existing Python synthetic OCR handoff corpus runner, README truth-sync.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `summary_output: Option<PathBuf>` to `OcrHandoffCorpusArgs`.
  - Replace the current pre-parse `--summary-output` rejection with normal parsing.
  - Remove stale summary files before prerequisite checks and on every failure path.
  - Add `build_ocr_handoff_corpus_summary(&Value) -> Value` with strict aggregate-only fields.
  - Write the summary only after the primary report has validated and been written.
  - Update usage text to show `[--summary-output <summary.json>]` for `ocr-handoff-corpus`.
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
  - Add success smoke coverage proving summary output is written, aggregate-only, and PHI/path safe.
  - Add failure smoke coverage proving stale summary output is removed on prerequisite failure.
- Modify: `README.md`
  - Truth-sync evidence for the optional `mdid-cli ocr-handoff-corpus --summary-output <summary.json>` artifact.
  - Keep completion arithmetic honest: CLI remains 95%, Browser/Web remains 99%, Desktop app remains 99%, Overall remains 97% because this is CLI/runtime aggregate evidence only.

### Task 1: Add optional OCR handoff corpus summary artifact to CLI

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
- Modify: `README.md`

- [ ] **Step 1: Write the failing success-path smoke test**

Add this test near existing `ocr_handoff_corpus` tests in `crates/mdid-cli/tests/cli_smoke.rs`:

```rust
#[test]
fn ocr_handoff_corpus_writes_phi_safe_summary_output() {
    let dir = tempdir().unwrap();
    let report_path = dir.path().join("ocr-handoff-corpus.json");
    let summary_path = dir.path().join("ocr-handoff-corpus-summary.json");

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-handoff-corpus",
            "--fixture-dir",
            &repo_path("scripts/ocr_eval/fixtures/corpus"),
            "--runner-path",
            &repo_path("scripts/ocr_eval/run_ocr_handoff_corpus.py"),
            "--report-path",
            report_path.to_str().unwrap(),
            "--summary-output",
            summary_path.to_str().unwrap(),
            "--python-command",
            &default_python_command(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(report_path.exists());
    assert!(summary_path.exists());

    let summary_text = fs::read_to_string(&summary_path).unwrap();
    let summary: serde_json::Value = serde_json::from_str(&summary_text).unwrap();
    assert_eq!(summary["artifact"], "ocr_handoff_corpus_readiness_summary");
    assert_eq!(summary["candidate"], "PP-OCRv5_mobile_rec");
    assert_eq!(summary["engine"], "PP-OCRv5-mobile-bounded-spike");
    assert_eq!(summary["scope"], "printed_text_line_extraction_only");
    assert_eq!(summary["privacy_filter_contract"], "text_only_normalized_input");
    assert_eq!(summary["fixture_count"], 2);
    assert_eq!(summary["ready_fixture_count"], 2);
    assert!(summary["all_fixtures_ready_for_text_pii_eval"].as_bool().unwrap());
    assert!(summary["total_char_count"].as_u64().unwrap() > 0);
    assert!(summary.get("fixtures").is_none());
    assert!(summary.get("fixture").is_none());
    assert!(summary.get("normalized_text").is_none());
    assert!(summary.get("ocr_lines").is_none());
    assert!(summary.get("bbox").is_none());
    assert!(summary.get("image_bytes").is_none());

    for unsafe_text in [
        "Jane Example",
        "MRN-12345",
        "jane@example.com",
        "555-123-4567",
        "fixture_001",
        "synthetic_patient_label_",
        "/home/",
        "/tmp/",
        "fixtures/",
    ] {
        assert!(!summary_text.contains(unsafe_text), "summary leaked {unsafe_text}");
    }
}
```

- [ ] **Step 2: Run test to verify RED**

Run: `cargo test -p mdid-cli ocr_handoff_corpus_writes_phi_safe_summary_output -- --nocapture`
Expected: FAIL because `ocr-handoff-corpus --summary-output` is currently rejected as an unknown flag.

- [ ] **Step 3: Implement minimal summary-output support**

In `crates/mdid-cli/src/main.rs`, add `summary_output: Option<PathBuf>` to `OcrHandoffCorpusArgs`, parse `--summary-output` with `non_blank_path`, and remove stale primary/summary outputs before prerequisites and on failures:

```rust
let _ = fs::remove_file(&args.report_path);
if let Some(summary_output) = &args.summary_output {
    let _ = fs::remove_file(summary_output);
}
```

Add this helper near the OCR handoff corpus validation code:

```rust
fn build_ocr_handoff_corpus_summary(report: &Value) -> Value {
    let fixture_count = report["fixture_count"].as_u64().unwrap_or(0);
    let ready_fixture_count = report["ready_fixture_count"].as_u64().unwrap_or(0);
    json!({
        "artifact": "ocr_handoff_corpus_readiness_summary",
        "candidate": "PP-OCRv5_mobile_rec",
        "engine": report["engine"],
        "scope": report["scope"],
        "privacy_filter_contract": report["privacy_filter_contract"],
        "fixture_count": fixture_count,
        "ready_fixture_count": ready_fixture_count,
        "all_fixtures_ready_for_text_pii_eval": fixture_count > 0 && fixture_count == ready_fixture_count,
        "total_char_count": report["total_char_count"],
        "non_goals": report["non_goals"],
    })
}
```

After the primary report validates and is written, write the optional summary:

```rust
if let Some(summary_output) = &args.summary_output {
    let summary = build_ocr_handoff_corpus_summary(&report_json);
    let summary_text = serde_json::to_string_pretty(&summary)
        .map_err(|err| format!("failed to render OCR handoff corpus summary: {err}"))?;
    if summary_text.len() > PRIVACY_FILTER_RUNNER_STDOUT_MAX_BYTES {
        return Err("OCR handoff corpus summary exceeded limit".to_string());
    }
    fs::write(summary_output, format!("{summary_text}\n"))
        .map_err(|_| "OCR handoff corpus failed".to_string())?;
}
```

Update usage for this command to include `[--summary-output <summary.json>]`.

- [ ] **Step 4: Run targeted test to verify GREEN**

Run: `cargo test -p mdid-cli ocr_handoff_corpus_writes_phi_safe_summary_output -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Add stale summary cleanup failure test**

Add this test:

```rust
#[test]
fn ocr_handoff_corpus_removes_stale_summary_on_prerequisite_failure() {
    let dir = tempdir().unwrap();
    let missing_fixture_dir = dir.path().join("missing-fixtures");
    let report_path = dir.path().join("ocr-handoff-corpus.json");
    let summary_path = dir.path().join("ocr-handoff-corpus-summary.json");
    fs::write(&summary_path, "stale raw Jane Example").unwrap();

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-handoff-corpus",
            "--fixture-dir",
            missing_fixture_dir.to_str().unwrap(),
            "--runner-path",
            &repo_path("scripts/ocr_eval/run_ocr_handoff_corpus.py"),
            "--report-path",
            report_path.to_str().unwrap(),
            "--summary-output",
            summary_path.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Jane Example").not())
        .stderr(predicate::str::contains("Jane Example").not())
        .stderr(predicate::str::contains(summary_path.to_string_lossy().as_ref()).not());

    assert!(!report_path.exists());
    assert!(!summary_path.exists());
}
```

- [ ] **Step 6: Run targeted and supporting tests**

Run:

```bash
cargo test -p mdid-cli ocr_handoff_corpus -- --nocapture
python scripts/ocr_eval/run_ocr_handoff_corpus.py --fixture-dir scripts/ocr_eval/fixtures/corpus --output /tmp/ocr-handoff-corpus.json --summary-output /tmp/ocr-handoff-corpus-summary.json
python - <<'PY'
import json
from pathlib import Path
summary = json.loads(Path('/tmp/ocr-handoff-corpus-summary.json').read_text())
assert summary['artifact'] == 'ocr_handoff_corpus_readiness_summary'
assert summary['scope'] == 'printed_text_line_extraction_only'
text = Path('/tmp/ocr-handoff-corpus-summary.json').read_text()
for unsafe in ['Jane Example','MRN-12345','jane@example.com','555-123-4567','fixture_001','/home/','/tmp/','fixtures/']:
    assert unsafe not in text, unsafe
print('ocr_handoff_summary_ok')
PY
git diff --check
```

Expected: all pass.

- [ ] **Step 7: Update README truth-sync**

Update the current completion snapshot/evidence paragraph to mention that `mdid-cli ocr-handoff-corpus` now accepts optional `--summary-output <summary.json>` and writes an aggregate-only readiness summary after primary report validation. State explicitly that this is CLI/runtime evidence only, not Browser/Web or Desktop OCR execution, not model-quality proof, not visual redaction, not image pixel redaction, and not final PDF rewrite/export. Completion stays CLI 95%, Browser/Web 99%, Desktop app 99%, Overall 97%; no new rubric denominator remains open because the new optional summary requirement is added and completed in the same round.

- [ ] **Step 8: Commit**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs README.md docs/superpowers/plans/2026-05-01-ocr-handoff-corpus-cli-summary-output.md
git commit -m "feat(cli): add OCR handoff corpus summary output"
```
