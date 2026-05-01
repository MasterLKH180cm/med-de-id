# Privacy Filter Passport Detection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add bounded synthetic passport-number detection to the existing text-only Privacy Filter CLI/runtime POC.

**Architecture:** Extend the deterministic fallback text detector with one narrow `PASSPORT` span category and keep all downstream validators aligned. The slice stays CLI/runtime text-only and does not add OCR, visual redaction, PDF rewrite/export, browser UI, desktop UI, or workflow orchestration behavior.

**Tech Stack:** Python 3 scripts under `scripts/privacy_filter/`, Rust `mdid-cli`, pytest/unittest, Cargo smoke tests, Markdown README truth-sync.

---

## File Structure

- Modify `scripts/privacy_filter/run_privacy_filter.py`: add a bounded passport-number regex, emit `PASSPORT` spans, and keep masking deterministic.
- Modify `scripts/privacy_filter/validate_privacy_filter_output.py`: allow the new `PASSPORT` category in the checked-in output contract validator.
- Modify `scripts/privacy_filter/test_run_privacy_filter.py`: add RED/GREEN tests for positive passport detection, embedded-token negatives, redacted previews, and validator compatibility.
- Modify `crates/mdid-cli/src/main.rs`: allow `PASSPORT` in CLI-side Privacy Filter contract validation.
- Modify `crates/mdid-cli/tests/cli_smoke.rs`: add a stdin smoke proving `mdid-cli privacy-filter-text` accepts the new category and leaks no raw passport value.
- Modify `README.md`: truth-sync completion and evidence with conservative fraction accounting.

### Task 1: Python Privacy Filter PASSPORT Detection

**Files:**
- Modify: `scripts/privacy_filter/test_run_privacy_filter.py`
- Modify: `scripts/privacy_filter/run_privacy_filter.py`
- Modify: `scripts/privacy_filter/validate_privacy_filter_output.py`

- [x] **Step 1: Write the failing passport detection tests**

Add this test method to `PrivacyFilterRunnerTests` in `scripts/privacy_filter/test_run_privacy_filter.py`:

```python
    def test_passport_numbers_are_masked_without_overmatching_embedded_tokens(self):
        text = 'Patient Jane Example passport X12345678 reference AX12345678 and X123456789'
        payload = run_text(text)

        self.assertEqual(payload['summary']['category_counts'].get('PASSPORT'), 1)
        self.assertIn('[PASSPORT]', payload['masked_text'])
        self.assertNotIn('X12345678', payload['masked_text'])
        self.assertIn('AX12345678', payload['masked_text'])
        self.assertIn('X123456789', payload['masked_text'])
        passport_spans = [span for span in payload['spans'] if span['label'] == 'PASSPORT']
        self.assertEqual(len(passport_spans), 1)
        self.assertEqual(passport_spans[0]['preview'], '<redacted>')
```

- [x] **Step 2: Run the focused test to verify RED**

Run: `python -m pytest scripts/privacy_filter/test_run_privacy_filter.py::PrivacyFilterRunnerTests::test_passport_numbers_are_masked_without_overmatching_embedded_tokens -q`

Expected: FAIL because `PASSPORT` is not detected or is not allowed by the validator.

- [x] **Step 3: Implement the minimal Python detector and validator alignment**

Add this near the other regex constants in `scripts/privacy_filter/run_privacy_filter.py`:

```python
PASSPORT_RE = re.compile(r'(?<![A-Za-z0-9])(?:[A-Z]\d{8}|\d{9})(?![A-Za-z0-9])')
```

Update the allowlist in both `scripts/privacy_filter/run_privacy_filter.py` and `scripts/privacy_filter/validate_privacy_filter_output.py`:

```python
ALLOWED_LABELS = {'NAME', 'MRN', 'EMAIL', 'PHONE', 'ID', 'DATE', 'ADDRESS', 'SSN', 'ZIP', 'PASSPORT'}
```

Add this detection loop after the SSN loop and before ZIP/address loops in `scripts/privacy_filter/run_privacy_filter.py`:

```python
    for m in PASSPORT_RE.finditer(text):
        add_span(spans, 'PASSPORT', m.start(), m.end())
```

- [x] **Step 4: Run focused and contract tests to verify GREEN**

Run:

```bash
python -m pytest scripts/privacy_filter/test_run_privacy_filter.py::PrivacyFilterRunnerTests::test_passport_numbers_are_masked_without_overmatching_embedded_tokens -q
python -m pytest scripts/privacy_filter/test_run_privacy_filter.py -q
python -m py_compile scripts/privacy_filter/run_privacy_filter.py scripts/privacy_filter/validate_privacy_filter_output.py
```

Expected: all PASS.

- [x] **Step 5: Commit Task 1**

```bash
git add scripts/privacy_filter/run_privacy_filter.py scripts/privacy_filter/validate_privacy_filter_output.py scripts/privacy_filter/test_run_privacy_filter.py
git commit -m "feat(privacy-filter): detect bounded passport identifiers"
```

### Task 2: Rust CLI Contract and Smoke Coverage

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`

- [x] **Step 1: Write the failing CLI smoke test**

Add this test near the other `privacy_filter_text` category smoke tests in `crates/mdid-cli/tests/cli_smoke.rs`:

```rust
#[test]
fn cli_privacy_filter_text_detects_passport_without_phi_leaks() {
    let bin = assert_cmd::cargo::cargo_bin("mdid-cli");
    let temp = tempfile::tempdir().expect("tempdir");
    let report_path = temp.path().join("passport-report.json");
    let stdin_phi = "Patient Jane Example passport X12345678 MRN-12345\n";

    let output = Command::new(&bin)
        .arg("privacy-filter-text")
        .arg("--stdin")
        .arg("--runner-path")
        .arg("scripts/privacy_filter/run_privacy_filter.py")
        .arg("--report-path")
        .arg(&report_path)
        .arg("--mock")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            child.stdin.as_mut().expect("stdin").write_all(stdin_phi.as_bytes())?;
            child.wait_with_output()
        })
        .expect("run mdid-cli privacy-filter-text");

    assert!(output.status.success(), "stderr={}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("\"report_path\":\"<redacted>\""));
    for forbidden in ["X12345678", "Jane Example", "MRN-12345", report_path.to_string_lossy().as_ref()] {
        assert!(!stdout.contains(forbidden), "stdout leaked {forbidden}: {stdout}");
        assert!(!stderr.contains(forbidden), "stderr leaked {forbidden}: {stderr}");
    }

    let report_text = fs::read_to_string(&report_path).expect("report");
    for forbidden in ["X12345678", "Jane Example", "MRN-12345"] {
        assert!(!report_text.contains(forbidden), "report leaked {forbidden}: {report_text}");
    }
    let report: serde_json::Value = serde_json::from_str(&report_text).expect("json");
    assert_eq!(report["summary"]["category_counts"]["PASSPORT"], 1);
    assert!(report["masked_text"].as_str().unwrap().contains("[PASSPORT]"));
    for span in report["spans"].as_array().unwrap() {
        assert_eq!(span["preview"], "<redacted>");
    }
}
```

- [x] **Step 2: Run the focused smoke test to verify RED**

Run: `cargo test -p mdid-cli cli_privacy_filter_text_detects_passport_without_phi_leaks --test cli_smoke -- --nocapture`

Expected: FAIL because the Rust validator rejects `PASSPORT` or the Python runner has not yet emitted it in this task sequence.

- [x] **Step 3: Align Rust CLI validation**

Update `is_allowed_privacy_filter_label` in `crates/mdid-cli/src/main.rs` so the match arm includes `PASSPORT`:

```rust
        "NAME" | "MRN" | "EMAIL" | "PHONE" | "ID" | "DATE" | "ADDRESS" | "SSN" | "ZIP" | "PASSPORT"
```

- [x] **Step 4: Run targeted and broader CLI checks**

Run:

```bash
cargo test -p mdid-cli cli_privacy_filter_text_detects_passport_without_phi_leaks --test cli_smoke -- --nocapture
cargo test -p mdid-cli privacy_filter_text --test cli_smoke -- --nocapture
cargo fmt --check
git diff --check
```

Expected: all PASS.

- [x] **Step 5: Commit Task 2**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs
git commit -m "test(cli): accept passport privacy filter category"
```

### Task 3: README Completion Truth-Sync and Final Verification

**Files:**
- Modify: `README.md`

- [x] **Step 1: Update README completion evidence and fraction accounting**

Update the completion snapshot to state this round adds and completes one CLI/runtime Privacy Filter text-only rubric item: bounded synthetic passport identifier detection. Use conservative fraction accounting from the current CLI floor: `125/130 -> 126/131 = 96%` floor. Keep Browser/Web at `99%`, Desktop app at `99%`, and Overall at `97%` unless repository-visible evidence supports a different integer floor. Explicitly state Browser/Web +5% and Desktop +5% are `FAIL/not claimed` because this is CLI/runtime text-only evidence only.

- [x] **Step 2: Run README/spec checks and final verification**

Run:

```bash
python -m pytest scripts/privacy_filter/test_run_privacy_filter.py -q
cargo test -p mdid-cli privacy_filter_text --test cli_smoke -- --nocapture
cargo fmt --check
git diff --check
```

Expected: all PASS.

- [x] **Step 3: Commit Task 3**

```bash
git add README.md docs/superpowers/plans/2026-05-01-privacy-filter-passport-detection.md
git commit -m "docs: truth sync privacy filter passport detection"
```

## Self-Review

- Spec coverage: This plan covers Python detection, Python validator, Rust CLI validation, smoke tests, README completion truth-sync, and final verification for a bounded text-only Privacy Filter POC category.
- Placeholder scan: No TBD/TODO/implement later placeholders remain.
- Type consistency: The new label is consistently spelled `PASSPORT` in Python runner, Python validator, Rust validator, tests, and README.
