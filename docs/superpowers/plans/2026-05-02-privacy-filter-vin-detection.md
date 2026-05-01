# Privacy Filter VIN Detection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the bounded CLI/runtime text-only Privacy Filter POC to detect context-required vehicle identification numbers (VINs) in synthetic clinical text without leaking raw values.

**Architecture:** Add one bounded heuristic detector to `scripts/privacy_filter/run_privacy_filter.py` that only labels VIN-like tokens when a vehicle/VIN context word is present, validates the 17-character VIN alphabet, rejects embedded tokens, and emits the existing redacted span contract. Keep the change CLI/runtime text-only: no OCR, image redaction, PDF rewrite/export, agent workflow, controller, or orchestration semantics.

**Tech Stack:** Python 3 standard library regex/unittest, existing privacy filter runner/validator scripts.

---

## File Structure

- Modify `scripts/privacy_filter/run_privacy_filter.py`
  - Add `VIN_RE`, include `VIN` in `ALLOWED_LABELS`, and add the detector in `heuristic_detect()`.
  - Responsibility: bounded text-only fallback detection and masking contract.
- Modify `scripts/privacy_filter/test_run_privacy_filter.py`
  - Add tests for valid contextual VIN detection and invalid/uncontextual/embedded VIN rejection.
  - Responsibility: TDD guard for text PII detection behavior and raw-value non-leakage.
- Modify `README.md`
  - Truth-sync completion/rubric narrative after the landed detector and verification evidence.
  - Responsibility: user-facing status and remaining gaps only; no scope inflation.

### Task 1: Bounded VIN detection in Privacy Filter runner

**Files:**
- Modify: `scripts/privacy_filter/test_run_privacy_filter.py`
- Modify: `scripts/privacy_filter/run_privacy_filter.py`

- [ ] **Step 1: Write the failing valid-context VIN test**

Add this test inside `PrivacyFilterRunnerTests` in `scripts/privacy_filter/test_run_privacy_filter.py` near the existing bounded identifier detector tests:

```python
    def test_fallback_detects_contextual_vin_without_raw_previews(self):
        text = 'Patient Jane Example vehicle VIN 1HGCM82633A004352 for transport billing.'
        payload = detect_pii(text)
        validator = load_validator_module()

        self.assertEqual(payload['summary']['category_counts'].get('VIN'), 1)
        self.assertIn('[VIN]', payload['masked_text'])
        self.assertNotIn('1HGCM82633A004352', payload['masked_text'])
        vin_spans = [span for span in payload['spans'] if span['label'] == 'VIN']
        self.assertEqual(len(vin_spans), 1)
        self.assertEqual(text[vin_spans[0]['start']:vin_spans[0]['end']], '1HGCM82633A004352')
        self.assertEqual(vin_spans[0]['preview'], '<redacted>')
        self.assertNotIn('1HGCM82633A004352', json.dumps(payload, sort_keys=True))
        validator.validate_privacy_filter_output(payload)
```

- [ ] **Step 2: Run the valid-context test to verify RED**

Run:

```bash
python3 scripts/privacy_filter/test_run_privacy_filter.py PrivacyFilterRunnerTests.test_fallback_detects_contextual_vin_without_raw_previews -v
```

Expected: FAIL because `VIN` is not detected and/or not allowed by the validator.

- [ ] **Step 3: Add minimal VIN detector implementation**

In `scripts/privacy_filter/run_privacy_filter.py`, add a regex near the other identifier regexes:

```python
VIN_RE = re.compile(
    r'\b(?:VIN|vehicle(?:\s+(?:id|identification(?:\s+number)?|VIN))?)\s+([A-HJ-NPR-Z0-9]{17})(?![A-Za-z0-9-])',
    re.I,
)
```

Add `VIN` to `ALLOWED_LABELS`:

```python
ALLOWED_LABELS = {'NAME', 'MRN', 'EMAIL', 'PHONE', 'FAX', 'ID', 'DATE', 'ADDRESS', 'SSN', 'PASSPORT', 'ZIP', 'INSURANCE_ID', 'DEA_NUMBER', 'AGE', 'FACILITY', 'NPI', 'LICENSE_PLATE', 'IP_ADDRESS', 'URL', 'VIN'}
```

Add this loop in `heuristic_detect()` after license plate detection and before IP/URL detection:

```python
    for m in VIN_RE.finditer(text):
        add_span(spans, 'VIN', m.start(1), m.end(1))
```

- [ ] **Step 4: Run the valid-context test to verify GREEN**

Run:

```bash
python3 scripts/privacy_filter/test_run_privacy_filter.py PrivacyFilterRunnerTests.test_fallback_detects_contextual_vin_without_raw_previews -v
```

Expected: PASS.

- [ ] **Step 5: Write the failing rejection test**

Add this test inside `PrivacyFilterRunnerTests`:

```python
    def test_fallback_does_not_detect_invalid_uncontextual_or_embedded_vin_like_tokens(self):
        text = ' '.join([
            '1HGCM82633A004352 appears without context.',
            'VIN 1HGCM82633A00435I uses forbidden I.',
            'VIN X1HGCM82633A004352Y is embedded.',
            'MRN 1HGCM82633A004352 stays bounded.',
        ])
        payload = detect_pii(text)

        self.assertNotIn('VIN', payload['summary']['category_counts'])
        self.assertNotIn('[VIN]', payload['masked_text'])
```

- [ ] **Step 6: Run the rejection test to verify RED or guard current behavior**

Run:

```bash
python3 scripts/privacy_filter/test_run_privacy_filter.py PrivacyFilterRunnerTests.test_fallback_does_not_detect_invalid_uncontextual_or_embedded_vin_like_tokens -v
```

Expected: PASS if the minimal detector is already bounded enough; if it FAILS because embedded tokens are detected, update the implementation in Step 7.

- [ ] **Step 7: Harden embedded-token rejection if needed**

If Step 6 fails, change `VIN_RE` to require an alphanumeric boundary before the captured VIN by keeping the context token separate and the existing `(?![A-Za-z0-9-])` suffix:

```python
VIN_RE = re.compile(
    r'\b(?:VIN|vehicle(?:\s+(?:id|identification(?:\s+number)?|VIN))?)\s+([A-HJ-NPR-Z0-9]{17})(?![A-Za-z0-9-])',
    re.I,
)
```

- [ ] **Step 8: Run targeted Privacy Filter tests**

Run:

```bash
python3 scripts/privacy_filter/test_run_privacy_filter.py PrivacyFilterRunnerTests.test_fallback_detects_contextual_vin_without_raw_previews PrivacyFilterRunnerTests.test_fallback_does_not_detect_invalid_uncontextual_or_embedded_vin_like_tokens -v
```

Expected: both tests PASS.

- [ ] **Step 9: Run broader Privacy Filter runner tests**

Run:

```bash
python3 scripts/privacy_filter/test_run_privacy_filter.py -v
```

Expected: PASS.

- [ ] **Step 10: Commit**

```bash
git add scripts/privacy_filter/test_run_privacy_filter.py scripts/privacy_filter/run_privacy_filter.py
git commit -m "feat(privacy-filter): detect bounded VINs"
```

### Task 2: README completion truth-sync for VIN detector

**Files:**
- Modify: `README.md`

- [x] **Step 1: Inspect README completion section**

Run:

```bash
grep -n "Completion\|complete\|Privacy Filter\|99%" README.md | head -80
```

Expected: locate the existing completion/rubric lines before editing.

- [x] **Step 2: Update README status without inflating surfaces**

Update README so it states:

```markdown
- Privacy Filter CLI/runtime text-only PII detection now includes bounded VIN detection in addition to previously landed synthetic categories; this is still text-only detection/masking and is not OCR, visual redaction, image pixel redaction, handwriting recognition, or PDF rewrite/export.
- Current completion baseline remains CLI 95%, Browser/Web 88%, Desktop app 88%, Overall 94% unless controller-visible landed functionality justifies a recalculation. VIN detection is a CLI/runtime POC increment and does not by itself create Browser/Web or Desktop app landed capability.
- Target is 99% for CLI, Browser/Web, Desktop app, and Overall; the final 1% is reserved for non-core polish/packaging/governance/hardening/edge-case verification.
```

- [x] **Step 3: Verify README mentions no forbidden scope**

Run:

```bash
grep -niE "agent workflow|controller loop|planner|coder|reviewer|visual redaction|pixel redaction|PDF rewrite" README.md || true
```

Expected: no new wording that claims Privacy Filter is OCR/visual/PDF redaction or an agent/controller platform.

- [x] **Step 4: Commit README truth-sync**

```bash
git add README.md
git commit -m "docs: truth-sync privacy filter VIN detection"
```

## Self-Review

- Spec coverage: Task 1 adds bounded, text-only VIN detection with raw-value non-leakage and validator compatibility. Task 2 truth-syncs README completion narrative without claiming browser/desktop progress or forbidden OCR/visual/PDF capabilities.
- Placeholder scan: no TBD/TODO/fill-in/similar placeholders remain.
- Type consistency: label name is consistently `VIN`, detector variable is `VIN_RE`, and output uses the existing span contract.
