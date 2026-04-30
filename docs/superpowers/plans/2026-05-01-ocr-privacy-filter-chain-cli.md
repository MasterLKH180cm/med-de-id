# OCR to Privacy Filter Chain CLI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded CLI smoke contract proving synthetic PP-OCRv5 mobile text extraction output can be fed into the text-only Privacy Filter runner without leaking fixture PHI or claiming visual/PDF redaction.

**Architecture:** This is a CLI/runtime-only integration slice that composes the already-landed `ocr-handoff` and `privacy-filter-text` commands in tests and docs. It does not add browser/desktop UI or agent/controller workflow semantics; it verifies the handoff file’s `normalized_text` is usable as Privacy Filter text input.

**Tech Stack:** Rust `mdid-cli`, `assert_cmd`, Python helper scripts under `scripts/ocr_eval` and `scripts/privacy_filter`, JSON validation with `serde_json`, markdown docs.

---

## File Structure

- Modify: `crates/mdid-cli/tests/cli_smoke.rs` — add a fixture-backed end-to-end smoke test that runs `mdid-cli ocr-handoff`, extracts `normalized_text`, runs `mdid-cli privacy-filter-text`, validates the generated Privacy Filter JSON, and asserts raw fixture PHI does not leak to command stdout/stderr or reports.
- Modify: `docs/research/small-ocr-spike-results.md` — record the bounded OCR-to-text-PII chain evidence and non-goals.
- Modify: `README.md` — truth-sync completion/evidence without inflating Browser/Web or Desktop capability.

### Task 1: Add CLI smoke coverage for OCR handoff to Privacy Filter chain

**Files:**
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`

- [ ] **Step 1: Write the failing test**

Append this test near the existing OCR/Privacy Filter smoke tests in `crates/mdid-cli/tests/cli_smoke.rs`:

```rust
#[test]
fn cli_ocr_handoff_normalized_text_feeds_privacy_filter_without_phi_leaks() {
    let dir = tempdir().unwrap();
    let handoff_report = dir.path().join("ocr-handoff.json");
    let normalized_text = dir.path().join("ocr-normalized.txt");
    let privacy_report = dir.path().join("privacy-filter.json");

    let ocr_output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("ocr-handoff")
        .arg("--image-path")
        .arg(repo_path("scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png"))
        .arg("--ocr-runner-path")
        .arg(repo_path("scripts/ocr_eval/run_small_ocr.py"))
        .arg("--handoff-builder-path")
        .arg(repo_path("scripts/ocr_eval/build_ocr_handoff.py"))
        .arg("--report-path")
        .arg(&handoff_report)
        .arg("--python-command")
        .arg(default_python_command())
        .assert()
        .success()
        .stdout(predicate::str::contains("Jane Example").not())
        .stdout(predicate::str::contains("MRN-12345").not())
        .stderr(predicate::str::contains("Jane Example").not())
        .stderr(predicate::str::contains("MRN-12345").not())
        .get_output()
        .stdout
        .clone();

    let ocr_summary: Value = serde_json::from_slice(&ocr_output).unwrap();
    assert_eq!(ocr_summary["ready_for_text_pii_eval"], true);
    assert_eq!(ocr_summary["privacy_filter_contract"], "text_only_normalized_input");

    let handoff: Value = serde_json::from_str(&fs::read_to_string(&handoff_report).unwrap()).unwrap();
    let text = handoff["normalized_text"].as_str().unwrap();
    assert!(text.contains("Jane Example"));
    assert!(text.contains("MRN-12345"));
    fs::write(&normalized_text, text).unwrap();

    let privacy_output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("privacy-filter-text")
        .arg("--input-path")
        .arg(&normalized_text)
        .arg("--runner-path")
        .arg(repo_path("scripts/privacy_filter/run_privacy_filter.py"))
        .arg("--report-path")
        .arg(&privacy_report)
        .arg("--python-command")
        .arg(default_python_command())
        .assert()
        .success()
        .stdout(predicate::str::contains("Jane Example").not())
        .stdout(predicate::str::contains("MRN-12345").not())
        .stderr(predicate::str::contains("Jane Example").not())
        .stderr(predicate::str::contains("MRN-12345").not())
        .get_output()
        .stdout
        .clone();

    let privacy_summary: Value = serde_json::from_slice(&privacy_output).unwrap();
    assert_eq!(privacy_summary["engine"], "fallback_synthetic_patterns");
    assert_eq!(privacy_summary["network_api_called"], false);
    assert!(privacy_summary["detected_span_count"].as_u64().unwrap() >= 2);

    let privacy_json = fs::read_to_string(&privacy_report).unwrap();
    assert!(!privacy_json.contains("Jane Example"));
    assert!(!privacy_json.contains("MRN-12345"));
    assert!(privacy_json.contains("[NAME]"));
    assert!(privacy_json.contains("[MRN]"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p mdid-cli cli_ocr_handoff_normalized_text_feeds_privacy_filter_without_phi_leaks -- --nocapture`

Expected: FAIL because the new test has not yet been compiled into the test file, or because the existing Privacy Filter fallback does not yet detect the OCR fixture text categories required by the chain.

- [ ] **Step 3: Write minimal implementation**

If the test fails due to missing Privacy Filter fallback patterns, update `scripts/privacy_filter/run_privacy_filter.py` so fallback detection recognizes `MRN-12345` as `MRN` and keeps previews bracket-label-only. Do not add any OCR, image redaction, PDF rewrite, browser/desktop UI, or agent/controller semantics.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p mdid-cli cli_ocr_handoff_normalized_text_feeds_privacy_filter_without_phi_leaks -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Run supporting validators**

Run:

```bash
python scripts/ocr_eval/run_small_ocr.py --mock scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png > /tmp/small-ocr-output.txt
python scripts/ocr_eval/validate_small_ocr_output.py /tmp/small-ocr-output.txt scripts/ocr_eval/fixtures/synthetic_printed_phi_expected.txt
python scripts/ocr_eval/build_ocr_handoff.py --source scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png --input /tmp/small-ocr-output.txt --output /tmp/ocr-handoff.json
python scripts/ocr_eval/validate_ocr_handoff.py /tmp/ocr-handoff.json
python - <<'PY'
import json
from pathlib import Path
obj = json.loads(Path('/tmp/ocr-handoff.json').read_text())
Path('/tmp/ocr-normalized-text.txt').write_text(obj['normalized_text'], encoding='utf-8')
PY
python scripts/privacy_filter/run_privacy_filter.py --mock /tmp/ocr-normalized-text.txt > /tmp/privacy-filter-output.json
python scripts/privacy_filter/validate_privacy_filter_output.py /tmp/privacy-filter-output.json
```

Expected: all commands exit 0.

- [ ] **Step 6: Commit**

Run:

```bash
git add crates/mdid-cli/tests/cli_smoke.rs scripts/privacy_filter/run_privacy_filter.py
git commit -m "test(cli): prove OCR text feeds privacy filter"
```

### Task 2: Truth-sync docs and completion evidence for the OCR-to-Privacy-Filter chain

**Files:**
- Modify: `docs/research/small-ocr-spike-results.md`
- Modify: `README.md`

- [ ] **Step 1: Write the failing docs check**

Run:

```bash
python - <<'PY'
from pathlib import Path
readme = Path('README.md').read_text()
results = Path('docs/research/small-ocr-spike-results.md').read_text()
required = [
    'OCR-to-Privacy-Filter chain',
    'text-only Privacy Filter',
    'printed-text extraction only',
    'not visual redaction',
    'not final PDF rewrite/export',
]
missing = [term for term in required if term not in readme + '\n' + results]
if missing:
    raise SystemExit('missing docs terms: ' + ', '.join(missing))
PY
```

Expected: FAIL until docs explicitly mention the chain evidence and non-goals.

- [ ] **Step 2: Update docs with exact evidence**

Add a short section to `docs/research/small-ocr-spike-results.md` named `OCR-to-Privacy-Filter chain evidence` that lists the exact commands from Task 1 Step 5 and states that the chain proves only synthetic printed-text extraction handoff into text-only PII detection, not visual redaction, handwriting recognition, page detection/cropping, browser/desktop integration, or PDF rewrite/export.

Update `README.md` completion evidence to mention the new CLI/runtime chain test while keeping completion honest: CLI 95%, Browser/Web 93%, Desktop app 93%, Overall 95% unless controller-visible facts support a different re-baseline.

- [ ] **Step 3: Run docs check to verify it passes**

Run the Python docs check from Step 1 again.

Expected: PASS.

- [ ] **Step 4: Run final verification**

Run:

```bash
cargo test -p mdid-cli cli_ocr_handoff_normalized_text_feeds_privacy_filter_without_phi_leaks -- --nocapture
python scripts/privacy_filter/validate_privacy_filter_output.py scripts/privacy_filter/fixtures/sample_text_expected_shape.json
git diff --check
```

Expected: all commands exit 0.

- [ ] **Step 5: Commit**

Run:

```bash
git add README.md docs/research/small-ocr-spike-results.md
git commit -m "docs: truth-sync OCR privacy filter chain evidence"
```

## Self-Review

Spec coverage: Task 1 covers the required CLI/runtime chain evidence from PP-OCRv5 mobile synthetic OCR handoff to text-only Privacy Filter detection. Task 2 covers README/research truth-sync and completion honesty. No browser/desktop capability is claimed.

Placeholder scan: No TBD/TODO/fill-in placeholders remain.

Type consistency: The plan uses existing command names `ocr-handoff` and `privacy-filter-text`, existing helper names `repo_path` and `default_python_command`, and `serde_json::Value` already imported in `cli_smoke.rs`.
