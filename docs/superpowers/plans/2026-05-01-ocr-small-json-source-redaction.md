# OCR Small JSON Source Redaction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Harden the bounded PP-OCRv5 mobile `ocr-small-json` CLI/runtime spike so generated OCR JSON reports do not preserve caller-controlled image filenames in the `source` field.

**Architecture:** Keep the slice inside the existing CLI/runtime PP-OCRv5 mobile printed-text extraction spike. The Python runner will emit a fixed PHI-safe `source` sentinel for both mock and local PaddleOCR JSON output, while Rust wrapper tests assert PHI-bearing image filenames do not leak to stdout, stderr, primary report, or optional summary.

**Tech Stack:** Python 3 runner/tests, Rust `mdid-cli` smoke tests, Cargo test, repository README truth-sync.

---

## File Structure

- Modify: `scripts/ocr_eval/run_small_ocr.py` — change `build_extraction_contract()` to emit a fixed safe source sentinel instead of `input_path.name`.
- Modify: `tests/test_ocr_runner_contract.py` — add Python runner coverage proving JSON output source is redacted and PHI-bearing input filenames are not emitted in JSON.
- Modify: `crates/mdid-cli/tests/cli_smoke.rs` — add CLI smoke coverage proving `ocr-small-json` report/summary/stdout/stderr omit PHI-bearing image filename components.
- Modify: `README.md` — truth-sync completion evidence and fraction accounting for a CLI/runtime PP-OCRv5 mobile source-redaction hardening requirement.

### Task 1: Python OCR runner source redaction

**Files:**
- Modify: `scripts/ocr_eval/run_small_ocr.py`
- Modify: `tests/test_ocr_runner_contract.py`

- [ ] **Step 1: Write the failing Python test**

Add this test to `tests/test_ocr_runner_contract.py`:

```python
def test_json_output_redacts_phi_bearing_source_filename(tmp_path):
    source = tmp_path / "Jane-Example-MRN-12345.png"
    source.write_bytes(b"synthetic image placeholder")
    expected = tmp_path / "synthetic_printed_phi_expected.txt"
    expected.write_text("Patient Jane Example MRN-12345\n", encoding="utf-8")

    completed = subprocess.run(
        [sys.executable, "scripts/ocr_eval/run_small_ocr.py", "--mock", "--json", str(source)],
        cwd=REPO_ROOT,
        text=True,
        capture_output=True,
        check=True,
    )

    payload = json.loads(completed.stdout)
    rendered = json.dumps(payload, sort_keys=True)
    assert payload["source"] == "<redacted>"
    assert "Jane-Example-MRN-12345" not in rendered
    assert "Jane-Example-MRN-12345" not in completed.stderr
```

- [ ] **Step 2: Run RED**

Run:

```bash
python -m pytest tests/test_ocr_runner_contract.py::test_json_output_redacts_phi_bearing_source_filename -q
```

Expected: FAIL because `payload["source"]` currently contains the input filename.

- [ ] **Step 3: Implement minimal source redaction**

In `scripts/ocr_eval/run_small_ocr.py`, add a constant near the existing constants:

```python
REDACTED_SOURCE = "<redacted>"
```

Change `build_extraction_contract()` so the `source` field is:

```python
"source": REDACTED_SOURCE,
```

- [ ] **Step 4: Run GREEN and focused regression**

Run:

```bash
python -m pytest tests/test_ocr_runner_contract.py::test_json_output_redacts_phi_bearing_source_filename -q
python -m pytest tests/test_ocr_runner_contract.py -q
python -m py_compile scripts/ocr_eval/run_small_ocr.py
```

Expected: all commands PASS.

- [ ] **Step 5: Commit Task 1**

Run:

```bash
git add scripts/ocr_eval/run_small_ocr.py tests/test_ocr_runner_contract.py
git commit -m "fix(ocr): redact small runner source filenames"
```

### Task 2: CLI wrapper source-redaction smoke coverage and README truth-sync

**Files:**
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
- Modify: `README.md`

- [x] **Step 1: Write the failing CLI smoke test**

Add a test near existing `ocr_small_json` smoke tests in `crates/mdid-cli/tests/cli_smoke.rs`:

```rust
#[test]
fn ocr_small_json_redacts_phi_bearing_image_filename_from_artifacts() {
    let temp = tempfile::tempdir().expect("tempdir");
    let fixture_dir = temp.path();
    let image_path = fixture_dir.join("Jane-Example-MRN-12345.png");
    fs::copy(
        repo_root().join("scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png"),
        &image_path,
    )
    .expect("copy fixture image");
    fs::write(
        fixture_dir.join("synthetic_printed_phi_expected.txt"),
        "Patient Jane Example MRN-12345\n",
    )
    .expect("write expected fixture text");
    let report_path = temp.path().join("ocr-small-report.json");
    let summary_path = temp.path().join("ocr-small-summary.json");

    let output = Command::new(cli_bin())
        .arg("ocr-small-json")
        .arg("--image-path")
        .arg(&image_path)
        .arg("--ocr-runner-path")
        .arg(repo_root().join("scripts/ocr_eval/run_small_ocr.py"))
        .arg("--report-path")
        .arg(&report_path)
        .arg("--summary-output")
        .arg(&summary_path)
        .arg("--python-command")
        .arg(default_python_command())
        .arg("--mock")
        .output()
        .expect("run ocr-small-json");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let report = fs::read_to_string(&report_path).expect("report");
    let summary = fs::read_to_string(&summary_path).expect("summary");
    for rendered in [&stdout, &stderr, &report, &summary] {
        assert!(!rendered.contains("Jane-Example-MRN-12345"));
        assert!(!rendered.contains(image_path.to_string_lossy().as_ref()));
    }
    let report_json: serde_json::Value = serde_json::from_str(&report).expect("report json");
    assert_eq!(report_json["source"], "<redacted>");
}
```

- [x] **Step 2: Run RED**

Run:

```bash
cargo test -p mdid-cli ocr_small_json_redacts_phi_bearing_image_filename_from_artifacts -- --nocapture
```

Expected: FAIL before Task 1 implementation is present in this branch, because the primary report preserves the PHI-bearing filename. If Task 1 is already committed, this may pass; record that as test coverage confirming the already-implemented production change.

Observed in Task 2: PASS as test-only hardening coverage because prior production changes already fixed the source metadata behavior.

- [x] **Step 3: Run GREEN/regression**

Run:

```bash
cargo test -p mdid-cli ocr_small_json_redacts_phi_bearing_image_filename_from_artifacts -- --nocapture
cargo test -p mdid-cli ocr_small_json -- --nocapture
cargo fmt --check
git diff --check
```

Expected: all commands PASS.

- [x] **Step 4: README truth-sync**

Update `README.md` current repository status to state this round adds/completes one CLI/runtime PP-OCRv5 mobile source-redaction hardening requirement:

- CLI fraction changes from `117/122 = 95%` to `118/123 = 95%` floor.
- Browser/Web remains `99%` and Desktop app remains `99%` because no surface capability landed.
- Overall remains `97%`.
- Browser/Web +5 and Desktop +5 are `FAIL/not claimed` because this is CLI/runtime PP-OCRv5 mobile source-artifact hardening only.
- Explicitly state it is not OCR quality proof, not visual redaction, not image pixel redaction, not handwriting recognition, not final PDF rewrite/export, not Browser/Web execution, not Desktop execution, and not workflow orchestration semantics.

- [x] **Step 5: Commit Task 2**

Run:

```bash
git add crates/mdid-cli/tests/cli_smoke.rs README.md
git commit -m "test(ocr): lock PHI-safe small JSON source artifacts"
```

## Self-Review

- Spec coverage: The plan covers Python runner emission, CLI wrapper artifacts/stdout/stderr, verification, and README completion truth-sync.
- Placeholder scan: No TBD/TODO/fill-in-later placeholders are present.
- Type consistency: The fixed source sentinel is consistently `"<redacted>"` across Python output and Rust smoke assertions.
