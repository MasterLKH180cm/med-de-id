# OCR to Privacy Filter Corpus CLI Wrapper Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded `mdid-cli ocr-to-privacy-filter-corpus` wrapper that runs the existing synthetic PP-OCRv5 mobile OCR handoff corpus bridge into the text-only Privacy Filter runner and writes a PHI-safe aggregate report.

**Architecture:** Reuse the existing Python bridge at `scripts/ocr_eval/run_ocr_to_privacy_filter_corpus.py` as the source of truth, and add a Rust CLI wrapper in `crates/mdid-cli/src/main.rs` that validates a strict aggregate JSON contract before writing the requested report. The wrapper must remain CLI/runtime-only: no Browser/Web or Desktop execution, no visual redaction, no image pixel redaction, no final PDF rewrite/export, and no workflow orchestration semantics.

**Tech Stack:** Rust `mdid-cli`, `serde_json`, existing subprocess runner helpers, Python synthetic OCR/Privacy Filter scripts, Cargo tests.

---

## File Structure

- Modify `crates/mdid-cli/src/main.rs`: add `OcrToPrivacyFilterCorpusArgs`, parser branch, runner invocation, strict aggregate validator, stale report cleanup, PHI-safe stdout summary, and usage text.
- Modify `crates/mdid-cli/tests/cli_smoke.rs`: add TDD smoke tests for happy path, help discoverability, missing runner cleanup, invalid/unsafe aggregate rejection, and PHI leak prevention.
- Modify `README.md`: truth-sync completion evidence for the bounded CLI/runtime wrapper; Browser/Web and Desktop remain unchanged because no new user-facing surface capability lands.
- Modify `scripts/ocr_eval/README.md`: add the wrapper command as the CLI/runtime way to run the existing OCR-to-Privacy-Filter corpus bridge.

### Task 1: Add `mdid-cli ocr-to-privacy-filter-corpus` bounded wrapper

**Files:**
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
- Modify: `crates/mdid-cli/src/main.rs`

- [ ] **Step 1: Write the failing help/discoverability and happy-path tests**

Add tests equivalent to:

```rust
#[test]
fn cli_help_mentions_ocr_to_privacy_filter_corpus() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .arg("--help")
        .output()
        .expect("run help");
    let combined = format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    assert!(combined.contains("ocr-to-privacy-filter-corpus"));
}

#[test]
fn cli_ocr_to_privacy_filter_corpus_writes_safe_aggregate_report() {
    let temp = tempfile::tempdir().expect("tempdir");
    let report_path = temp.path().join("ocr-to-privacy-filter-corpus.json");
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "ocr-to-privacy-filter-corpus",
            "--fixture-dir", "scripts/ocr_eval/fixtures/corpus",
            "--ocr-runner-path", "scripts/ocr_eval/run_ocr_handoff_corpus.py",
            "--privacy-runner-path", "scripts/privacy_filter/run_privacy_filter.py",
            "--bridge-runner-path", "scripts/ocr_eval/run_ocr_to_privacy_filter_corpus.py",
            "--report-path",
        ])
        .arg(&report_path)
        .output()
        .expect("run wrapper");
    assert!(output.status.success(), "stderr={} stdout={}", String::from_utf8_lossy(&output.stderr), String::from_utf8_lossy(&output.stdout));
    let report = std::fs::read_to_string(&report_path).expect("report");
    assert!(report.contains("ocr_to_privacy_filter_corpus"));
    assert!(report.contains("PP-OCRv5_mobile_rec"));
    assert!(report.contains("text_only_pii_detection"));
    for forbidden in ["Jane Example", "MRN-12345", "jane@example.com", "555-123-4567", "synthetic_patient_label_01.txt"] {
        assert!(!report.contains(forbidden), "report leaked {forbidden}: {report}");
        assert!(!String::from_utf8_lossy(&output.stdout).contains(forbidden));
        assert!(!String::from_utf8_lossy(&output.stderr).contains(forbidden));
    }
    assert!(String::from_utf8_lossy(&output.stdout).contains("\"report_path\":\"<redacted>\""));
}
```

- [ ] **Step 2: Run tests to verify RED**

Run: `/home/azureuser/.cargo/bin/cargo test -p mdid-cli ocr_to_privacy_filter_corpus -- --nocapture`

Expected: FAIL because the command is unknown and help text does not mention it.

- [ ] **Step 3: Implement minimal parser, enum, args, runner, and validator**

Add a `CliCommand::OcrToPrivacyFilterCorpus(OcrToPrivacyFilterCorpusArgs)` variant; parse flags `--fixture-dir`, `--ocr-runner-path`, `--privacy-runner-path`, `--bridge-runner-path`, `--report-path`, and optional `--python-command` using the existing default Python helper. Run the bridge runner with those paths, cap stdout/stderr using existing bounded subprocess helper patterns, remove stale `--report-path` before execution, validate the generated JSON strict allowlist, reject raw synthetic sentinels and unsafe fields, then write a PHI-safe stdout summary:

```json
{
  "command": "ocr-to-privacy-filter-corpus",
  "report_path": "<redacted>",
  "fixture_count": 2,
  "ready_fixture_count": 2,
  "total_detected_span_count": 8,
  "network_api_called": false
}
```

Validator requirements: top-level object only; require artifact `ocr_to_privacy_filter_corpus`, candidate `PP-OCRv5_mobile_rec`, OCR scope `printed_text_line_extraction_only`, privacy scope `text_only_pii_detection`, `network_api_called == false`, integer nonnegative counts, safe known categories only (`NAME`, `MRN`, `EMAIL`, `PHONE`, `ID`), safe fixture IDs only (`fixture_001` style), no `masked_text`, no `spans`, no `preview`, no `extracted_text`, no `normalized_text`, no local paths, no fixture filenames, no raw synthetic PHI, no visual redaction/PDF export claims except explicit non-goals.

- [ ] **Step 4: Run tests to verify GREEN**

Run: `/home/azureuser/.cargo/bin/cargo test -p mdid-cli ocr_to_privacy_filter_corpus -- --nocapture`

Expected: PASS.

### Task 2: Harden wrapper failure cleanup and unsafe-output rejection

**Files:**
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
- Modify: `crates/mdid-cli/src/main.rs`

- [ ] **Step 1: Write failing tests for stale report cleanup and unsafe aggregate rejection**

Add tests that create a fake bridge runner which writes an unsafe aggregate containing a raw PHI sentinel or a forbidden field such as `masked_text`, then assert the command fails with a generic error and removes a stale report containing `Jane Example`. Add a missing bridge runner test that also verifies stale report removal.

- [ ] **Step 2: Run tests to verify RED**

Run: `/home/azureuser/.cargo/bin/cargo test -p mdid-cli ocr_to_privacy_filter_corpus -- --nocapture`

Expected: FAIL until cleanup and unsafe-output validation are implemented.

- [ ] **Step 3: Implement cleanup and rejection paths**

Best-effort remove the target report before prerequisites, on runner failure, on non-JSON output, on schema validation failure, and on unsafe content detection. Use generic PHI-safe errors such as `OCR to Privacy Filter corpus report failed validation`; do not echo user paths, raw runner stderr, or fixture text.

- [ ] **Step 4: Run tests to verify GREEN**

Run: `/home/azureuser/.cargo/bin/cargo test -p mdid-cli ocr_to_privacy_filter_corpus -- --nocapture`

Expected: PASS.

### Task 3: Truth-sync documentation and completion snapshot

**Files:**
- Modify: `README.md`
- Modify: `scripts/ocr_eval/README.md`

- [ ] **Step 1: Update docs with bounded wrapper evidence**

Add the exact command:

```bash
mdid-cli ocr-to-privacy-filter-corpus \
  --fixture-dir scripts/ocr_eval/fixtures/corpus \
  --ocr-runner-path scripts/ocr_eval/run_ocr_handoff_corpus.py \
  --privacy-runner-path scripts/privacy_filter/run_privacy_filter.py \
  --bridge-runner-path scripts/ocr_eval/run_ocr_to_privacy_filter_corpus.py \
  --report-path /tmp/ocr-to-privacy-filter-corpus.json
```

State that it proves only CLI/runtime aggregate evidence that the PP-OCRv5 mobile synthetic OCR handoff corpus can feed text-only Privacy Filter detection. Explicitly state it is not OCR model-quality evidence, visual redaction, image pixel redaction, handwriting recognition, Browser/Web integration, Desktop integration, or final PDF rewrite/export.

- [ ] **Step 2: Update completion truth-sync**

Keep Browser/Web at 99% and Desktop app at 99% because this is CLI/runtime-only. Keep CLI at 95% unless repository-visible rubric fraction accounting justifies a conservative increase. If adding the wrapper as a new necessary CLI/runtime rubric item and completing it in the same round, document old fraction, new fraction, and conservatively floored percentage.

- [ ] **Step 3: Run docs/code verification**

Run:

```bash
/home/azureuser/.cargo/bin/cargo test -p mdid-cli ocr_to_privacy_filter_corpus -- --nocapture
python scripts/ocr_eval/run_ocr_to_privacy_filter_corpus.py --fixture-dir scripts/ocr_eval/fixtures/corpus --ocr-runner-path scripts/ocr_eval/run_ocr_handoff_corpus.py --privacy-runner-path scripts/privacy_filter/run_privacy_filter.py --output /tmp/ocr-to-privacy-filter-corpus.json
python -m py_compile scripts/ocr_eval/run_ocr_to_privacy_filter_corpus.py scripts/ocr_eval/run_ocr_handoff_corpus.py scripts/privacy_filter/run_privacy_filter.py
git diff --check
```

Expected: all PASS.

- [ ] **Step 4: Commit**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs README.md scripts/ocr_eval/README.md docs/superpowers/plans/2026-05-01-ocr-privacy-filter-corpus-cli-wrapper.md
git commit -m "feat(cli): wrap ocr privacy filter corpus bridge"
```

---

## Self-Review

- Spec coverage: The plan adds the requested highest-leverage OCR mainline progress by turning the existing synthetic PP-OCRv5 mobile OCR-to-text-PII bridge into a bounded CLI/runtime wrapper with PHI-safe validation and evidence.
- Placeholder scan: No TBD/TODO/fill-in placeholders remain.
- Type consistency: Command name, args, report path, and validation vocabulary are consistent across tasks.
