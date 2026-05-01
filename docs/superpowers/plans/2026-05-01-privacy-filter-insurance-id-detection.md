# Privacy Filter Insurance ID Detection Implementation Plan

> **For implementation workers:** REQUIRED SUB-SKILL: Use subagent-driven-development (recommended) or executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add bounded text-only synthetic health-insurance member/policy identifier detection to the Privacy Filter CLI/runtime POC.

**Architecture:** Extend the deterministic local Privacy Filter fallback runner with a narrow `INSURANCE_ID` detector for explicit insurance/member/policy context only, then align Python validation, Rust CLI validation/smoke coverage, and README completion truth-sync. This remains CLI/runtime text-only PII detection evidence and does not add OCR, visual redaction, image pixel redaction, handwriting recognition, PDF rewrite/export, Browser/Web execution, Desktop execution, or workflow orchestration semantics.

**Tech Stack:** Python 3 Privacy Filter fallback runner/tests, Rust `mdid-cli` wrapper and smoke tests, Markdown README truth-sync.

---

## File Structure

- Modify `scripts/privacy_filter/run_privacy_filter.py`: add `INSURANCE_ID` to `ALLOWED_LABELS`; add bounded context regexes; emit `INSURANCE_ID` spans after SSN/date/passport detection and before ZIP/address/MRN/ID to avoid broad identifier collisions.
- Modify `scripts/privacy_filter/validate_privacy_filter_output.py`: add `INSURANCE_ID` to the validator allowlist.
- Modify `scripts/privacy_filter/test_run_privacy_filter.py`: add RED/GREEN unit tests for positive insurance/member/policy identifiers and negative embedded/standalone tokens.
- Modify `crates/mdid-cli/src/main.rs`: add `INSURANCE_ID` to `is_allowed_privacy_filter_label`.
- Modify `crates/mdid-cli/tests/cli_smoke.rs`: add a stdin smoke test proving `mdid-cli privacy-filter-text --stdin` accepts `INSURANCE_ID`, writes a validator-compatible report, and does not leak raw synthetic identifiers or paths.
- Modify `README.md`: update the completion snapshot/current-round evidence with fraction accounting from CLI `126/131 -> 127/132 = 96%` floor, Browser/Web 99%, Desktop 99%, Overall 97%; mark Browser/Desktop +5% rule as FAIL because this is CLI/runtime-only.

### Task 1: Python Privacy Filter insurance ID detector

**Files:**
- Modify: `scripts/privacy_filter/run_privacy_filter.py`
- Modify: `scripts/privacy_filter/validate_privacy_filter_output.py`
- Test: `scripts/privacy_filter/test_run_privacy_filter.py`

- [x] **Step 1: Write the failing tests**

Add these tests to `scripts/privacy_filter/test_run_privacy_filter.py`:

```python
def test_fallback_detects_contextual_insurance_ids_without_raw_previews(self):
    text = 'Patient Jane Example insurance ID ABC1234567 and member number MBR-7654321.'
    payload = detect_pii(text)
    labels = [span['label'] for span in payload['spans']]
    self.assertEqual(labels.count('INSURANCE_ID'), 2)
    self.assertIn('[INSURANCE_ID]', payload['masked_text'])
    self.assertEqual(payload['summary']['category_counts']['INSURANCE_ID'], 2)
    for span in payload['spans']:
        self.assertEqual(span['preview'], '<redacted>')
    self.assertNotIn('ABC1234567', json.dumps(payload))
    self.assertNotIn('MBR-7654321', json.dumps(payload))


def test_fallback_does_not_detect_standalone_or_embedded_insurance_like_tokens(self):
    text = 'Standalone ABC1234567 should not match; embedded XABC1234567 and MRN ABC1234567 stay bounded.'
    payload = detect_pii(text)
    labels = [span['label'] for span in payload['spans']]
    self.assertNotIn('INSURANCE_ID', labels)
```

- [x] **Step 2: Run tests to verify RED**

Run: `python -m pytest scripts/privacy_filter/test_run_privacy_filter.py -k insurance -q`
Expected: FAIL because `INSURANCE_ID` detection is not implemented.

- [x] **Step 3: Implement minimal detector and validation allowlist**

In `scripts/privacy_filter/run_privacy_filter.py`, add:

```python
INSURANCE_ID_RE = re.compile(
    r'\b(?:insurance(?:\s+(?:id|number|policy))?|member(?:\s+(?:id|number))?|policy(?:\s+(?:id|number))?)\s+(?:ID\s+)?([A-Z]{2,4}-?\d{6,10}|[A-Z]{3}\d{6,10})(?![A-Za-z0-9-])',
    re.I,
)
```

Add `INSURANCE_ID` to `ALLOWED_LABELS`, and in `detect_pii(text)` after SSN/passport loops and before ZIP/address/MRN/ID loops:

```python
for m in INSURANCE_ID_RE.finditer(text):
    add_span(spans, 'INSURANCE_ID', m.start(1), m.end(1))
```

In `scripts/privacy_filter/validate_privacy_filter_output.py`, add `INSURANCE_ID` to its allowed-label set.

- [x] **Step 4: Run Python GREEN tests**

Run: `python -m pytest scripts/privacy_filter/test_run_privacy_filter.py -k insurance -q`
Expected: PASS.

Run: `python scripts/privacy_filter/run_privacy_filter.py --mock scripts/privacy_filter/fixtures/sample_text_input.txt > /tmp/privacy-filter-insurance-baseline.json && python scripts/privacy_filter/validate_privacy_filter_output.py /tmp/privacy-filter-insurance-baseline.json`
Expected: PASS; existing fixture remains valid.

- [x] **Step 5: Commit Python detector slice**

```bash
git add scripts/privacy_filter/run_privacy_filter.py scripts/privacy_filter/validate_privacy_filter_output.py scripts/privacy_filter/test_run_privacy_filter.py
git commit -m "feat(privacy-filter): detect insurance identifiers"
```

### Task 2: Rust CLI wrapper validation and smoke coverage

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/cli_smoke.rs`

- [x] **Step 1: Write the failing CLI smoke test**

Add a test named `cli_privacy_filter_text_accepts_insurance_id_category_without_phi_leaks` to `crates/mdid-cli/tests/cli_smoke.rs`. It should run `mdid-cli privacy-filter-text --stdin --runner-path scripts/privacy_filter/run_privacy_filter.py --report-path <temp>/insurance-report.json --python-command <default_python_command()> --mock`, submit `Patient Jane Example insurance ID ABC1234567 member number MBR-7654321`, then assert: command success; stdout contains `"report_path":"<redacted>"`; report JSON has `summary.category_counts.INSURANCE_ID == 2`; `masked_text` contains `[INSURANCE_ID]`; every span preview is `<redacted>`; stdout/stderr/report omit `ABC1234567`, `MBR-7654321`, `Jane Example`, the report path, and temp directory.

- [x] **Step 2: Run test to verify RED**

Run: `cargo test -p mdid-cli cli_privacy_filter_text_accepts_insurance_id_category_without_phi_leaks --test cli_smoke -- --nocapture`
Expected: FAIL because Rust category allowlist does not yet accept `INSURANCE_ID`.

- [x] **Step 3: Add Rust category allowlist support**

In `crates/mdid-cli/src/main.rs`, add `"INSURANCE_ID"` to `is_allowed_privacy_filter_label`.

- [x] **Step 4: Run CLI GREEN tests**

Run: `cargo test -p mdid-cli cli_privacy_filter_text_accepts_insurance_id_category_without_phi_leaks --test cli_smoke -- --nocapture`
Expected: PASS.

Run: `cargo test -p mdid-cli privacy_filter_text --test cli_smoke -- --nocapture`
Expected: PASS for all Privacy Filter text smoke tests.

- [x] **Step 5: Commit CLI wrapper slice**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs
git commit -m "test(cli): accept insurance privacy filter category"
```

### Task 3: README completion truth-sync and final verification

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/plans/2026-05-01-privacy-filter-insurance-id-detection.md`

- [x] **Step 1: Update README snapshot and evidence**

Update the completion snapshot to say this round adds and completes one CLI/runtime text-only `INSURANCE_ID` detection requirement: CLI fraction `126/131 -> 127/132 = 96%` floor, Browser/Web remains 99%, Desktop app remains 99%, Overall remains 97%. Add a verification evidence paragraph listing the Python and Rust commands run. Explicitly state Browser/Web +5% rule: FAIL/not claimed; Desktop +5% rule: FAIL/not claimed.

- [x] **Step 2: Mark this plan's completed checkboxes**

After implementation and verification, change completed checklist items in this plan from `- [ ]` to `- [x]`.

- [x] **Step 3: Run final verification**

```bash
python -m pytest scripts/privacy_filter/test_run_privacy_filter.py -k insurance -q
cargo test -p mdid-cli cli_privacy_filter_text_accepts_insurance_id_category_without_phi_leaks --test cli_smoke -- --nocapture
cargo test -p mdid-cli privacy_filter_text --test cli_smoke -- --nocapture
python scripts/privacy_filter/run_privacy_filter.py --mock scripts/privacy_filter/fixtures/sample_text_input.txt > /tmp/privacy-filter-insurance-baseline.json
python scripts/privacy_filter/validate_privacy_filter_output.py /tmp/privacy-filter-insurance-baseline.json
cargo fmt --check
git diff --check
```

Expected: all PASS with no diff whitespace errors.

- [x] **Step 4: Commit docs and plan truth-sync**

```bash
git add README.md docs/superpowers/plans/2026-05-01-privacy-filter-insurance-id-detection.md
git commit -m "docs: truth sync insurance privacy filter detection"
```
