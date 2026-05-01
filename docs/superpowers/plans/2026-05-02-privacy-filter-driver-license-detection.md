# Privacy Filter Driver License Detection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add bounded CLI/runtime text-only Privacy Filter support for contextual driver license identifiers without overclaiming OCR, visual redaction, Browser/Web execution, Desktop execution, or final PDF rewrite/export.

**Architecture:** Extend the existing deterministic `scripts/privacy_filter/run_privacy_filter.py` fallback detector with one context-required `DRIVER_LICENSE` category and allow the same category through OPF normalization and Rust CLI report validation. Keep the detector bounded to explicit driver-license context so standalone ID-like strings, MRN/ID-prefixed tokens, and embedded values do not become false positives.

**Tech Stack:** Python 3 standard library regex/unittest runner; Rust `mdid-cli` validation and CLI smoke tests; existing local Privacy Filter JSON contract.

---

## File Structure

- Modify `scripts/privacy_filter/run_privacy_filter.py`: add `DRIVER_LICENSE_RE`, include `DRIVER_LICENSE` in `ALLOWED_LABELS`, and emit redacted spans from `heuristic_detect` only when explicit driver-license context is present.
- Modify `scripts/privacy_filter/test_run_privacy_filter.py`: add unit tests for positive contextual driver-license detection and negative boundedness cases.
- Modify `crates/mdid-cli/src/main.rs`: allow `DRIVER_LICENSE` in CLI privacy-filter report validation.
- Modify `crates/mdid-cli/tests/cli_smoke.rs`: add stdin smoke coverage proving `mdid-cli privacy-filter-text` accepts `DRIVER_LICENSE`, masks it, and does not leak raw driver-license text, synthetic name/MRN, report path, or PHI-bearing temp directory.
- Modify `README.md`: truth-sync completion and verification evidence after code/test/review completion.

### Task 1: Python fallback detector and OPF label allowlist

**Files:**
- Modify: `scripts/privacy_filter/run_privacy_filter.py`
- Test: `scripts/privacy_filter/test_run_privacy_filter.py`

- [ ] **Step 1: Write the failing positive detector test**

Add this method inside `PrivacyFilterRunnerTests` in `scripts/privacy_filter/test_run_privacy_filter.py`:

```python
    def test_fallback_detects_contextual_driver_license_without_raw_previews(self):
        text = 'Patient Jane Example driver license D1234567 for intake verification.'
        payload = detect_pii(text)
        validator = load_validator_module()

        self.assertEqual(payload['summary']['category_counts'].get('DRIVER_LICENSE'), 1)
        self.assertIn('[DRIVER_LICENSE]', payload['masked_text'])
        self.assertNotIn('D1234567', payload['masked_text'])
        license_spans = [span for span in payload['spans'] if span['label'] == 'DRIVER_LICENSE']
        self.assertEqual(len(license_spans), 1)
        self.assertEqual(text[license_spans[0]['start']:license_spans[0]['end']], 'D1234567')
        self.assertEqual(license_spans[0]['preview'], '<redacted>')
        self.assertNotIn('D1234567', json.dumps(payload, sort_keys=True))
        validator.validate_privacy_filter_output(payload)
```

- [ ] **Step 2: Run the positive test to verify RED**

Run: `python3 -m pytest scripts/privacy_filter/test_run_privacy_filter.py::PrivacyFilterRunnerTests::test_fallback_detects_contextual_driver_license_without_raw_previews -q`

Expected: FAIL because `DRIVER_LICENSE` is not detected and/or not allowed by the validator.

- [ ] **Step 3: Implement minimal positive detector support**

In `scripts/privacy_filter/run_privacy_filter.py`, add near the existing identifier regex constants:

```python
DRIVER_LICENSE_RE = re.compile(
    r'\b(?:driver(?:\s+license)?|drivers(?:\s+license)?|driver\'s\s+license|DL|license(?:\s+(?:number|no\.))?)\s+([A-Z]\d{7,8}|[A-Z]{1,2}-?\d{6,8})(?![A-Za-z0-9-])',
    re.I,
)
```

Add `DRIVER_LICENSE` to `ALLOWED_LABELS`:

```python
ALLOWED_LABELS = {'NAME', 'MRN', 'EMAIL', 'PHONE', 'FAX', 'ID', 'DATE', 'ADDRESS', 'SSN', 'PASSPORT', 'ZIP', 'INSURANCE_ID', 'DEA_NUMBER', 'AGE', 'FACILITY', 'NPI', 'LICENSE_PLATE', 'VIN', 'DRIVER_LICENSE', 'IP_ADDRESS', 'URL'}
```

Add this detector loop after the VIN loop and before IP/URL detection:

```python
    for m in DRIVER_LICENSE_RE.finditer(text):
        add_span(spans, 'DRIVER_LICENSE', m.start(1), m.end(1))
```

- [ ] **Step 4: Run the positive test to verify GREEN**

Run: `python3 -m pytest scripts/privacy_filter/test_run_privacy_filter.py::PrivacyFilterRunnerTests::test_fallback_detects_contextual_driver_license_without_raw_previews -q`

Expected: PASS.

- [ ] **Step 5: Write the failing boundedness test**

Add this method inside `PrivacyFilterRunnerTests`:

```python
    def test_fallback_does_not_detect_standalone_or_embedded_driver_license_like_tokens(self):
        text = ' '.join([
            'D1234567 appears without context.',
            'driver license XD1234567Y is embedded.',
            'MRN D1234567 stays bounded.',
            'ID D1234567 stays bounded.',
            'license ABCDEFG is not a bounded driver license token.',
        ])
        payload = detect_pii(text)

        self.assertNotIn('DRIVER_LICENSE', payload['summary']['category_counts'])
        self.assertNotIn('[DRIVER_LICENSE]', payload['masked_text'])
```

- [ ] **Step 6: Run the boundedness test to verify RED or already-GREEN from minimal bounded regex**

Run: `python3 -m pytest scripts/privacy_filter/test_run_privacy_filter.py::PrivacyFilterRunnerTests::test_fallback_does_not_detect_standalone_or_embedded_driver_license_like_tokens -q`

Expected: PASS is acceptable if Step 3's bounded regex already rejects the cases. If it FAILS, tighten `DRIVER_LICENSE_RE` so it requires context and uses the existing end boundary `(?![A-Za-z0-9-])`.

- [ ] **Step 7: Run the full Python Privacy Filter test suite**

Run: `python3 scripts/privacy_filter/test_run_privacy_filter.py -v`

Expected: PASS for all tests.

- [ ] **Step 8: Commit Task 1**

```bash
git add scripts/privacy_filter/run_privacy_filter.py scripts/privacy_filter/test_run_privacy_filter.py
git commit -m "feat(privacy-filter): detect bounded driver licenses"
```

### Task 2: Rust CLI validation and stdin smoke coverage

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`

- [ ] **Step 1: Write the failing CLI smoke test**

Add this test near the other `privacy_filter_text_detects_*` tests in `crates/mdid-cli/tests/cli_smoke.rs`:

```rust
#[test]
fn cli_privacy_filter_text_detects_driver_license_from_stdin_without_raw_value_leaks() {
    let dir = tempdir().unwrap();
    let report = dir.path().join("driver-license-report.json");
    let input = "Patient Jane Example driver license D1234567 for MRN-12345.";

    let assert = Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("privacy-filter-text")
        .arg("--stdin")
        .arg("--runner-path")
        .arg(repo_root().join("scripts/privacy_filter/run_privacy_filter.py"))
        .arg("--report-path")
        .arg(&report)
        .arg("--python-command")
        .arg(python_command())
        .write_stdin(input)
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(!stdout.contains("D1234567"));
    assert!(!stderr.contains("D1234567"));
    assert!(!stdout.contains("Patient Jane Example"));
    assert!(!stderr.contains("Patient Jane Example"));
    assert!(!stdout.contains("MRN-12345"));
    assert!(!stderr.contains("MRN-12345"));
    assert!(!stdout.contains(report.to_str().unwrap()));
    assert!(!stdout.contains(dir.path().to_str().unwrap()));

    let report_text = fs::read_to_string(&report).unwrap();
    assert!(!report_text.contains("D1234567"));
    let report_json: Value = serde_json::from_str(&report_text).unwrap();
    assert_eq!(report_json["summary"]["category_counts"]["DRIVER_LICENSE"], 1);
    assert!(report_json["masked_text"].as_str().unwrap().contains("[DRIVER_LICENSE]"));
    assert!(report_json["spans"]
        .as_array()
        .unwrap()
        .iter()
        .any(|span| span["label"] == "DRIVER_LICENSE" && span["preview"] == "<redacted>"));
    assert_eq!(report_json["metadata"]["network_api_called"], false);
}
```

- [ ] **Step 2: Run the CLI smoke test to verify RED**

Run: `cargo test -p mdid-cli cli_privacy_filter_text_detects_driver_license_from_stdin_without_raw_value_leaks --test cli_smoke -- --nocapture`

Expected: FAIL with unsupported category `DRIVER_LICENSE` from CLI report validation.

- [ ] **Step 3: Allow the new label in CLI validation**

In `crates/mdid-cli/src/main.rs`, extend the category allowlist in `is_allowed_privacy_filter_category`:

```rust
            | "LICENSE_PLATE"
            | "VIN"
            | "DRIVER_LICENSE"
            | "IP_ADDRESS"
            | "URL"
```

- [ ] **Step 4: Run targeted CLI smoke test to verify GREEN**

Run: `cargo test -p mdid-cli cli_privacy_filter_text_detects_driver_license_from_stdin_without_raw_value_leaks --test cli_smoke -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Run Privacy Filter CLI regression tests**

Run: `cargo test -p mdid-cli privacy_filter_text --test cli_smoke -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Commit Task 2**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs
git commit -m "fix(cli): accept privacy filter driver license category"
```

### Task 3: README truth-sync and final verification

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Run final verification before editing README**

Run these commands:

```bash
python3 scripts/privacy_filter/test_run_privacy_filter.py -v
cargo test -p mdid-cli cli_privacy_filter_text_detects_driver_license_from_stdin_without_raw_value_leaks --test cli_smoke -- --nocapture
cargo test -p mdid-cli privacy_filter_text --test cli_smoke -- --nocapture
git diff --check
```

Expected: all commands PASS.

- [ ] **Step 2: Update README current snapshot and evidence**

Update `README.md` so the current repository status states that the new bounded CLI/runtime text-only Privacy Filter `DRIVER_LICENSE` requirement was added and completed. Use conservative fraction accounting from the current README baseline: `136/141 -> 137/142 = 96%` floor. Keep displayed completion at CLI 99%, Browser/Web 99%, Desktop app 99%, and Overall 99%; explicitly say Browser/Web +5 and Desktop +5 are FAIL/not claimed because this is CLI/runtime text-only Privacy Filter evidence and those surfaces are already capped at the 99% target.

Add a verification paragraph immediately after the VIN paragraph:

```markdown
Verification evidence for the `mdid-cli privacy-filter-text` driver license detection slice landed on this branch: the bounded local text-only Privacy Filter runner now detects explicitly contextual driver-license identifiers such as `driver license D1234567` as `DRIVER_LICENSE`, masks the raw token as `[DRIVER_LICENSE]`, emits only `<redacted>` span previews, and keeps `metadata.network_api_called: false`. The detector rejects standalone, embedded/unbounded, MRN-context, ID-context, and non-identifier license-like tokens. The validator and Rust CLI validation now accept `DRIVER_LICENSE`. Repository-visible verification passed with targeted driver-license Python tests, the full `python3 scripts/privacy_filter/test_run_privacy_filter.py -v` suite, the targeted `cargo test -p mdid-cli cli_privacy_filter_text_detects_driver_license_from_stdin_without_raw_value_leaks --test cli_smoke -- --nocapture` smoke test, and the `cargo test -p mdid-cli privacy_filter_text --test cli_smoke -- --nocapture` regression suite. Completion accounting adds one requirement to both denominator and numerator (`136/141 -> 137/142 = 96%` floor), leaving displayed completion at CLI 99%, Browser/Web 99%, Desktop app 99%, and Overall 99%; Browser/Web +5 and Desktop +5 are FAIL/not claimed because this is CLI/runtime text-only Privacy Filter evidence and those surfaces are already capped at 99%. It is not OCR, visual redaction, image pixel redaction, handwriting recognition, final PDF rewrite/export, Browser/Web execution, Desktop execution, or model-quality proof.
```

- [ ] **Step 3: Run README/diff checks**

Run:

```bash
git diff --check
git status --short
```

Expected: no whitespace errors; only planned README changes are dirty if not committed.

- [ ] **Step 4: Commit Task 3**

```bash
git add README.md docs/superpowers/plans/2026-05-02-privacy-filter-driver-license-detection.md
git commit -m "docs: truth-sync privacy filter driver license detection"
```

## Self-Review

1. **Spec coverage:** The plan covers Python fallback detection, OPF/validator label allowlisting, Rust CLI report validation, stdin smoke coverage, full regression verification, and README completion truth-sync. It explicitly preserves text-only scope and non-goals.
2. **Placeholder scan:** No TBD/TODO/fill-in placeholders remain; concrete test bodies, regex, commands, and README text are included.
3. **Type consistency:** The category name is consistently `DRIVER_LICENSE` across Python detector, JSON labels, Rust validation, CLI smoke test, and README evidence.
