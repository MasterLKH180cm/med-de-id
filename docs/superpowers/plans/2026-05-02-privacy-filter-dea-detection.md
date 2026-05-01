# Privacy Filter DEA Detection Implementation Plan

> **For implementation workers:** REQUIRED SUB-SKILL: Use subagent-driven-development (recommended) or executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add bounded text-only DEA registration number detection to the Privacy Filter CLI/runtime fallback POC without claiming OCR, visual redaction, Browser/Desktop execution, or final PDF rewrite/export.

**Architecture:** Extend the deterministic local Privacy Filter runner with one narrow `DEA_NUMBER` detector requiring explicit DEA context and strict DEA checksum validation. Keep JSON output PHI-safe by masking the full DEA token, forcing previews to `<redacted>`, and aligning Python runner, Python validator, Rust CLI allowlists/smoke tests, and README completion arithmetic in one coherent slice.

**Tech Stack:** Python 3 runner/tests, Rust `mdid-cli` smoke tests, Cargo, README completion truth-sync.

---

## File Structure

- Modify `scripts/privacy_filter/run_privacy_filter.py`: add `DEA_NUMBER` regex/checksum helper and emit spans after `NPI`/license-like health identifiers but before generic `ID`-style masking, preserving non-overlap behavior.
- Modify `scripts/privacy_filter/validate_privacy_filter_output.py`: add `DEA_NUMBER` to the strict label allowlist.
- Modify `scripts/privacy_filter/test_run_privacy_filter.py`: add RED/GREEN unit tests for valid context-required DEA values, invalid checksums, embedded tokens, MRN/ID-prefixed values, and validator compatibility.
- Modify `crates/mdid-cli/src/main.rs`: add `DEA_NUMBER` to `is_allowed_privacy_filter_label` for `privacy-filter-text` report validation only.
- Modify `crates/mdid-cli/tests/cli_smoke.rs`: add a stdin `mdid-cli privacy-filter-text --stdin --mock` smoke test proving DEA category counts, masking, PHI/path-safe stdout/stderr/report, and `<redacted>` previews.
- Modify `README.md`: truth-sync completion evidence and arithmetic. Treat this as CLI/runtime text-only PII detection progress only; Browser/Web and Desktop completion must not increase.

### Task 1: Add bounded DEA_NUMBER detection to Privacy Filter CLI/runtime

**Files:**
- Modify: `scripts/privacy_filter/test_run_privacy_filter.py`
- Modify: `scripts/privacy_filter/run_privacy_filter.py`
- Modify: `scripts/privacy_filter/validate_privacy_filter_output.py`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
- Modify: `README.md`

- [x] **Step 1: Write failing Python tests**

Add tests to `scripts/privacy_filter/test_run_privacy_filter.py`:

```python
    def test_fallback_detects_contextual_dea_numbers_without_raw_previews(self):
        text = 'Patient Jane Example DEA AB1234563 for MRN-12345.'
        payload = detect_pii(text)
        validator = load_validator_module()

        self.assertEqual(payload['summary']['category_counts'].get('DEA_NUMBER'), 1)
        self.assertIn('[DEA_NUMBER]', payload['masked_text'])
        rendered = json.dumps(payload, sort_keys=True)
        self.assertNotIn('AB1234563', rendered)
        dea_spans = [span for span in payload['spans'] if span['label'] == 'DEA_NUMBER']
        self.assertEqual(len(dea_spans), 1)
        self.assertEqual(dea_spans[0]['preview'], '<redacted>')
        validator.validate_privacy_filter_output(payload)

    def test_fallback_rejects_invalid_or_uncontextual_dea_like_tokens(self):
        text = 'AB1234563 no context; DEA AB1234564 bad checksum; MRN AB1234563 and ID AB1234563 stay bounded.'
        payload = detect_pii(text)

        labels = [span['label'] for span in payload['spans']]
        self.assertNotIn('DEA_NUMBER', labels)
        self.assertNotIn('[DEA_NUMBER]', payload['masked_text'])
```

- [x] **Step 2: Run Python tests to verify RED**

Run:

```bash
python -m unittest scripts.privacy_filter.test_run_privacy_filter.PrivacyFilterRunnerTests.test_fallback_detects_contextual_dea_numbers_without_raw_previews scripts.privacy_filter.test_run_privacy_filter.PrivacyFilterRunnerTests.test_fallback_rejects_invalid_or_uncontextual_dea_like_tokens
```

Expected: FAIL because `DEA_NUMBER` is not detected/allowed yet.

- [x] **Step 3: Implement minimal Python runner/validator support**

In `scripts/privacy_filter/run_privacy_filter.py`, add:

```python
DEA_RE = re.compile(r'(?i)\bDEA\s+([A-Z]{2}\d{7})\b')


def is_valid_dea_number(value: str) -> bool:
    if not re.fullmatch(r'[A-Z]{2}\d{7}', value):
        return False
    digits = [int(ch) for ch in value[2:]]
    check = (digits[0] + digits[2] + digits[4]) + 2 * (digits[1] + digits[3] + digits[5])
    return check % 10 == digits[6]
```

Emit a `DEA_NUMBER` span for the captured token only when `is_valid_dea_number()` is true and the token is not immediately preceded by `MRN`, `MRN-`, `ID`, or `ID-`. Keep span previews `<redacted>` and do not include the raw token anywhere in JSON.

In `scripts/privacy_filter/validate_privacy_filter_output.py`, add `DEA_NUMBER` to `ALLOWED_LABELS`.

- [x] **Step 4: Run Python tests to verify GREEN**

Run:

```bash
python -m unittest scripts.privacy_filter.test_run_privacy_filter.PrivacyFilterRunnerTests.test_fallback_detects_contextual_dea_numbers_without_raw_previews scripts.privacy_filter.test_run_privacy_filter.PrivacyFilterRunnerTests.test_fallback_rejects_invalid_or_uncontextual_dea_like_tokens
```

Expected: PASS.

- [x] **Step 5: Write failing Rust CLI smoke test**

Add a test to `crates/mdid-cli/tests/cli_smoke.rs` that pipes:

```text
Patient Jane Example DEA AB1234563 for MRN-12345.
```

through `mdid-cli privacy-filter-text --stdin --mock --runner-path scripts/privacy_filter/run_privacy_filter.py --report-path <temp>/dea-output.json` and asserts:

```rust
assert_eq!(category_counts.get("DEA_NUMBER"), Some(&json!(1)));
assert!(masked_text.contains("[DEA_NUMBER]"));
assert!(!rendered_report.contains("AB1234563"));
assert!(!stdout_text.contains("AB1234563"));
assert!(!stderr_text.contains("AB1234563"));
assert!(spans.iter().any(|span| span.get("label") == Some(&json!("DEA_NUMBER")) && span.get("preview") == Some(&json!("<redacted>"))));
```

Also deny `Jane Example`, `MRN-12345`, temp directory path, and report path from stdout/stderr/report where applicable.

- [x] **Step 6: Run Rust smoke test to verify RED**

Run:

```bash
/home/azureuser/.cargo/bin/cargo test -p mdid-cli cli_privacy_filter_text_detects_dea_number --test cli_smoke -- --nocapture
```

Expected: FAIL because Rust CLI validation has not allowed `DEA_NUMBER` yet or the test is not implemented.

- [x] **Step 7: Implement Rust CLI allowlist support**

In `crates/mdid-cli/src/main.rs`, add `DEA_NUMBER` to the existing Privacy Filter label allowlist used for `privacy-filter-text` JSON validation.

- [x] **Step 8: Run Rust smoke test to verify GREEN**

Run:

```bash
/home/azureuser/.cargo/bin/cargo test -p mdid-cli cli_privacy_filter_text_detects_dea_number --test cli_smoke -- --nocapture
```

Expected: PASS.

- [x] **Step 9: Run broader focused verification**

Run:

```bash
python -m unittest scripts.privacy_filter.test_run_privacy_filter
python -m py_compile scripts/privacy_filter/run_privacy_filter.py scripts/privacy_filter/validate_privacy_filter_output.py
/home/azureuser/.cargo/bin/cargo test -p mdid-cli privacy_filter_text --test cli_smoke -- --nocapture
git diff --check
```

Expected: all PASS and no whitespace errors. Remove `scripts/privacy_filter/__pycache__/` if generated.

- [x] **Step 10: README truth-sync**

Update `README.md` completion/evidence rows to state that this round adds one CLI/runtime text-only Privacy Filter requirement (`DEA_NUMBER`) and completes it in the same slice. Use fraction accounting from the current README baseline by adding `+1` to both numerator and denominator. Browser/Web and Desktop remain unchanged because no user-facing surface capability landed; explicitly mark their +5% round rule as FAIL/not claimed.

- [x] **Step 11: Commit**

Run:

```bash
git add scripts/privacy_filter/test_run_privacy_filter.py scripts/privacy_filter/run_privacy_filter.py scripts/privacy_filter/validate_privacy_filter_output.py crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs README.md docs/superpowers/plans/2026-05-02-privacy-filter-dea-detection.md
git commit -m "feat(privacy-filter): detect bounded DEA numbers"
```

Expected: commit succeeds.

## Self-Review

- Spec coverage: The plan covers bounded text-only DEA detection, validator/CLI allowlist alignment, Rust smoke evidence, README completion truth-sync, and no-goal boundaries.
- Placeholder scan: No TBD/TODO/fill-in placeholders remain.
- Type consistency: The label is consistently `DEA_NUMBER`, previews are consistently `<redacted>`, and completion is consistently CLI/runtime-only.
