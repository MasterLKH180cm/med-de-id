# Privacy Filter ZIP Detection Implementation Plan

> **For implementation workers:** REQUIRED SUB-SKILL: Use subagent-driven-development (recommended) or executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]` / `- [x]`) syntax for tracking.

**Goal:** Add bounded ZIP/postal-code text-only PII detection to the Privacy Filter CLI/runtime POC without claiming OCR, visual redaction, image pixel redaction, browser execution, desktop execution, or PDF rewrite/export.

**Architecture:** Extend the deterministic local text-only Privacy Filter fallback with a narrow `ZIP` category for US ZIP and ZIP+4 forms, then align the Python validator, Rust CLI allowlist, CLI smoke coverage, and README completion arithmetic. The implementation remains CLI/runtime only and supports the downstream PP-OCRv5 mobile OCR handoff by improving the text PII contract that normalized OCR text can feed.

**Tech Stack:** Python 3 runner/tests, Rust `mdid-cli`, Cargo CLI smoke tests, repository README truth-sync.

---

## File Structure

- Modify `scripts/privacy_filter/run_privacy_filter.py`: add a bounded ZIP regex, emit `ZIP` spans in deterministic order, and include `ZIP` in the allowed label set for OPF normalization.
- Modify `scripts/privacy_filter/validate_privacy_filter_output.py`: include `ZIP` in the strict category/span label allowlist.
- Modify `scripts/privacy_filter/test_run_privacy_filter.py`: add RED/GREEN subprocess tests for positive ZIP/ZIP+4 detection and negative embedded ZIP-like tokens.
- Modify `crates/mdid-cli/src/main.rs`: include `ZIP` in `is_allowed_privacy_filter_label` so `mdid-cli privacy-filter-text` validates runner output.
- Modify `crates/mdid-cli/tests/cli_smoke.rs`: add a stdin smoke test that asserts `ZIP` is accepted and raw ZIP values do not leak to stdout/stderr/report.
- Modify `README.md`: truth-sync the completion snapshot/current evidence with CLI fraction `124/129 -> 125/130 = 96%` floor, Browser/Web unchanged 99%, Desktop unchanged 99%, Overall unchanged 97%, and explicit Browser/Desktop +5 FAIL.

### Task 1: Python Privacy Filter ZIP fallback detection

**Files:**
- Modify: `scripts/privacy_filter/run_privacy_filter.py`
- Modify: `scripts/privacy_filter/validate_privacy_filter_output.py`
- Test: `scripts/privacy_filter/test_run_privacy_filter.py`

- [x] **Step 1: Write the failing positive ZIP test**

Add this method inside `PrivacyFilterRunnerFailureTests` in `scripts/privacy_filter/test_run_privacy_filter.py`:

```python
    def test_stdin_mock_detects_zip_codes_without_phi_previews(self):
        phi = 'Patient Jane Example lives in ZIP 02139 and alternate 02139-4307\n'
        result = subprocess.run(
            [sys.executable, str(RUNNER), '--stdin', '--mock'],
            input=phi,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=5,
            check=False,
        )

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stderr, '')
        payload = json.loads(result.stdout)
        self.assertEqual(payload['metadata']['engine'], 'fallback_synthetic_patterns')
        self.assertEqual(payload['metadata']['network_api_called'], False)
        self.assertEqual(payload['summary']['category_counts'].get('ZIP'), 2)
        self.assertIn('[ZIP]', payload['masked_text'])
        rendered = json.dumps(payload, sort_keys=True)
        self.assertNotIn('02139', rendered)
        self.assertNotIn('02139-4307', rendered)
        self.assertTrue(all(span['preview'] == '<redacted>' for span in payload['spans']))
```

- [x] **Step 2: Run the positive ZIP test to verify RED**

Run:

```bash
python3 -m pytest scripts/privacy_filter/test_run_privacy_filter.py::PrivacyFilterRunnerFailureTests::test_stdin_mock_detects_zip_codes_without_phi_previews -q
```

Expected: FAIL because `category_counts.get('ZIP')` is `None` before implementation.

- [x] **Step 3: Write the failing embedded-token negative test**

Add this method beside the positive ZIP test:

```python
    def test_stdin_mock_does_not_detect_embedded_zip_like_tokens(self):
        phi = 'Codes A02139 02139B 02139-4307-extra and ID02139 remain ordinary text\n'
        result = subprocess.run(
            [sys.executable, str(RUNNER), '--stdin', '--mock'],
            input=phi,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=5,
            check=False,
        )

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stderr, '')
        payload = json.loads(result.stdout)
        self.assertNotIn('ZIP', payload['summary']['category_counts'])
        self.assertNotIn('[ZIP]', payload['masked_text'])
```

- [x] **Step 4: Run the embedded-token test to verify current behavior**

Run:

```bash
python3 -m pytest scripts/privacy_filter/test_run_privacy_filter.py::PrivacyFilterRunnerFailureTests::test_stdin_mock_does_not_detect_embedded_zip_like_tokens -q
```

Expected: PASS before implementation, confirming the negative boundary is locked.

- [x] **Step 5: Implement minimal ZIP detection and validation allowlist**

In `scripts/privacy_filter/run_privacy_filter.py`, add the ZIP regex and label:

```python
ZIP_RE = re.compile(r'(?<![A-Za-z0-9-])\d{5}(?:-\d{4})?(?![A-Za-z0-9-])')
ALLOWED_LABELS = {'NAME', 'MRN', 'EMAIL', 'PHONE', 'ID', 'DATE', 'ADDRESS', 'SSN', 'ZIP'}
```

Add ZIP detection after `SSN_RE` and before `ADDRESS_RE`:

```python
    for m in ZIP_RE.finditer(text):
        add_span(spans, 'ZIP', m.start(), m.end())
```

In `scripts/privacy_filter/validate_privacy_filter_output.py`, update the allowed labels set to include `ZIP`:

```python
ALLOWED_LABELS = {'NAME', 'MRN', 'EMAIL', 'PHONE', 'ID', 'DATE', 'ADDRESS', 'SSN', 'ZIP'}
```

- [x] **Step 6: Run targeted Python tests to verify GREEN**

Run:

```bash
python3 -m pytest scripts/privacy_filter/test_run_privacy_filter.py::PrivacyFilterRunnerFailureTests::test_stdin_mock_detects_zip_codes_without_phi_previews scripts/privacy_filter/test_run_privacy_filter.py::PrivacyFilterRunnerFailureTests::test_stdin_mock_does_not_detect_embedded_zip_like_tokens -q
```

Expected: PASS.

- [x] **Step 7: Run the broader Privacy Filter Python tests**

Run:

```bash
python3 -m pytest scripts/privacy_filter/test_run_privacy_filter.py -q
```

Expected: PASS.

- [x] **Step 8: Commit Task 1**

```bash
git add scripts/privacy_filter/run_privacy_filter.py scripts/privacy_filter/validate_privacy_filter_output.py scripts/privacy_filter/test_run_privacy_filter.py
git commit -m "feat(privacy-filter): detect zip pii in text runner"
```

### Task 2: Rust CLI ZIP validation and smoke coverage

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`

- [x] **Step 1: Write the failing CLI smoke test**

Add this Rust test near the existing `privacy_filter_text_detects_*` tests in `crates/mdid-cli/tests/cli_smoke.rs`:

```rust
#[test]
fn privacy_filter_text_detects_zips_from_stdin_without_raw_zip_leaks() {
    let tmp = TempDir::new().expect("temp dir");
    let report_path = tmp.path().join("privacy-filter-zip-report.json");
    let input = "Patient Jane Example lives in ZIP 02139 and alternate 02139-4307\n";

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_mdid-cli"));
    cmd.arg("privacy-filter-text")
        .arg("--stdin")
        .arg("--runner-path")
        .arg(repo_path("scripts/privacy_filter/run_privacy_filter.py"))
        .arg("--report-path")
        .arg(&report_path)
        .arg("--python-command")
        .arg(default_python_command())
        .arg("--mock")
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"report_path\":\"<redacted>\""))
        .stdout(predicate::str::contains("\"report_written\":true"))
        .stdout(predicate::str::contains("\"detected_span_count\":"))
        .stdout(predicate::str::contains("02139").not())
        .stdout(predicate::str::contains("02139-4307").not())
        .stderr(predicate::str::is_empty());

    let report = fs::read_to_string(&report_path).expect("read report");
    assert!(report.contains("\"ZIP\""), "report should include ZIP category: {report}");
    assert!(report.contains("[ZIP]"), "masked text should include ZIP placeholder: {report}");
    assert!(!report.contains("02139"), "report leaked ZIP text: {report}");
    assert!(!report.contains("02139-4307"), "report leaked ZIP+4 text: {report}");
    assert!(!report.contains("Jane Example"), "report leaked name: {report}");
    assert!(report.contains("\"preview\": \"<redacted>\""));
}
```

- [x] **Step 2: Run the CLI smoke test to verify RED**

Run:

```bash
cargo test -p mdid-cli privacy_filter_text_detects_zips_from_stdin_without_raw_zip_leaks --test cli_smoke -- --nocapture
```

Expected: FAIL before Rust allowlist update because the CLI rejects `ZIP` as an invalid category label.

- [x] **Step 3: Update Rust category allowlist**

In `crates/mdid-cli/src/main.rs`, update `is_allowed_privacy_filter_label`:

```rust
fn is_allowed_privacy_filter_label(label: &str) -> bool {
    matches!(
        label,
        "NAME" | "MRN" | "EMAIL" | "PHONE" | "ID" | "DATE" | "ADDRESS" | "SSN" | "ZIP"
    )
}
```

- [x] **Step 4: Run focused CLI smoke to verify GREEN**

Run:

```bash
cargo test -p mdid-cli privacy_filter_text_detects_zips_from_stdin_without_raw_zip_leaks --test cli_smoke -- --nocapture
```

Expected: PASS.

- [x] **Step 5: Run broader Privacy Filter CLI smoke regression**

Run:

```bash
cargo test -p mdid-cli privacy_filter_text --test cli_smoke -- --nocapture
```

Expected: PASS.

- [x] **Step 6: Commit Task 2**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs
git commit -m "test(cli): accept zip privacy filter category"
```

### Task 3: README truth-sync and final verification

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/plans/2026-05-01-privacy-filter-zip-detection.md`

- [x] **Step 1: Update README completion snapshot**

Replace the snapshot/current-round wording so it says this round is `mdid-cli privacy-filter-text` ZIP PII detection, not the prior SSN round. Keep Browser/Web and Desktop at 99%, Overall at 97%, and update CLI raw accounting from `124/129 = 96%` to `125/130 = 96%` floor. Explicitly state Browser/Web +5% FAIL and Desktop +5% FAIL because this is CLI/runtime-only text PII detection and both surfaces are already capped at the 99% target.

- [x] **Step 2: Add ZIP evidence paragraph**

Add this paragraph near the existing SSN/ADDRESS/DATE Privacy Filter evidence:

```markdown
Verification evidence for the `mdid-cli privacy-filter-text` ZIP detection slice landed on this branch: the bounded local text-only Privacy Filter runner now detects common synthetic US ZIP and ZIP+4 forms such as `02139` and `02139-4307` as `ZIP`, masks them as `[ZIP]`, emits only `<redacted>` span previews, and keeps `metadata.network_api_called: false`. The detector is bounded against adjacent alphanumeric or hyphen characters, with regression coverage proving embedded/extended tokens such as `A02139`, `02139B`, `02139-4307-extra`, and `ID02139` remain ordinary text instead of overbroad ZIP detections. Rust CLI stdin smoke coverage proves stdout, stderr, and report contents do not leak raw ZIP values, synthetic name, report path, or temp directory. Repository-visible verification: `python3 -m pytest scripts/privacy_filter/test_run_privacy_filter.py -q`, `cargo test -p mdid-cli privacy_filter_text_detects_zips_from_stdin_without_raw_zip_leaks --test cli_smoke -- --nocapture`, `cargo test -p mdid-cli privacy_filter_text --test cli_smoke -- --nocapture`, `cargo fmt --check`, and `git diff --check` passed. This is CLI/runtime text-only PII detection coverage: not OCR, not visual redaction, not image pixel redaction, not handwriting recognition, not final PDF rewrite/export, not Browser/Desktop execution, and not workflow orchestration. Fraction accounting starts from the prior current CLI `124/129 = 96%` floor and adds/completes one CLI/runtime ZIP text-PII requirement in this round: CLI `125/130 = 96%` floor, Browser/Web remains 99%, Desktop app remains 99%, and Overall remains 97%, with Browser/Web +5 and Desktop +5 FAIL/not claimed because no Browser/Web or Desktop surface capability landed and both are already at the 99% target cap.
```

- [x] **Step 3: Mark plan checkboxes complete**

Update this plan file so completed steps are checked (`- [x]`) before final integration review.

- [x] **Step 4: Run final verification**

Run:

```bash
python3 -m pytest scripts/privacy_filter/test_run_privacy_filter.py -q
cargo test -p mdid-cli privacy_filter_text_detects_zips_from_stdin_without_raw_zip_leaks --test cli_smoke -- --nocapture
cargo test -p mdid-cli privacy_filter_text --test cli_smoke -- --nocapture
cargo fmt --check
git diff --check
```

Expected: all PASS, no formatting or whitespace errors.

- [x] **Step 5: Commit Task 3**

```bash
git add README.md docs/superpowers/plans/2026-05-01-privacy-filter-zip-detection.md
git commit -m "docs: truth-sync privacy filter zip detection"
```

- [ ] **Step 6: Push branch**

```bash
git status --short
git push -u origin feat/privacy-filter-zip-detection-cron-2223
```

Expected: working tree clean before push; branch pushed to origin.

## Self-Review

- Spec coverage: The plan covers Python fallback detection, Python validator allowlist, Rust CLI validation, CLI smoke, README completion accounting, and final verification. It explicitly excludes OCR, visual redaction, image pixel redaction, Browser/Desktop execution, and PDF rewrite/export.
- Placeholder scan: No TBD/TODO/fill-in placeholders remain; every code-changing step includes concrete snippets and commands.
- Type/signature consistency: The new label is consistently named `ZIP` across Python runner, Python validator, Rust allowlist, Rust smoke test, README, and completion accounting.
