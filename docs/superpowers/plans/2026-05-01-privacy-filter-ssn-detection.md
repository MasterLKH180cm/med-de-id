# Privacy Filter SSN Detection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the bounded CLI/runtime text-only Privacy Filter POC to detect and mask common synthetic SSN-like identifiers without broadening into OCR, visual redaction, or PDF rewrite/export.

**Architecture:** Add one deterministic fallback pattern to `scripts/privacy_filter/run_privacy_filter.py` and preserve the existing PHI-safe JSON contract: category counts, `[SSN]` masking, and `<redacted>` span previews. Keep the work CLI/runtime-only and validate it with focused runner tests plus README truth-sync evidence.

**Tech Stack:** Python 3 unittest runner, Rust `mdid-cli` smoke tests, Markdown README truth-sync.

---

## File Structure

- Modify: `scripts/privacy_filter/run_privacy_filter.py` — add a bounded `SSN_RE` and emit `SSN` spans in `heuristic_detect`.
- Modify: `scripts/privacy_filter/test_run_privacy_filter.py` — add a focused subprocess test proving SSN detection, masking, and PHI-safe previews.
- Modify: `README.md` — truth-sync current completion/evidence with the landed SSN detection slice and maintain the 99% target accounting.

### Task 1: Privacy Filter SSN fallback detection

**Files:**
- Modify: `scripts/privacy_filter/test_run_privacy_filter.py`
- Modify: `scripts/privacy_filter/run_privacy_filter.py`

- [ ] **Step 1: Write the failing test**

Add this method inside `PrivacyFilterRunnerFailureTests` after `test_stdin_mock_reads_stdin_emits_contract_and_detects_phi`:

```python
    def test_stdin_mock_detects_ssn_without_phi_previews(self):
        phi = 'Patient Jane Example has SSN 123-45-6789 for intake\n'
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
        self.assertEqual(payload['summary']['category_counts'].get('SSN'), 1)
        self.assertIn('[SSN]', payload['masked_text'])
        rendered = json.dumps(payload, sort_keys=True)
        self.assertNotIn('123-45-6789', rendered)
        self.assertTrue(all(span['preview'] == '<redacted>' for span in payload['spans']))
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
python scripts/privacy_filter/test_run_privacy_filter.py PrivacyFilterRunnerFailureTests.test_stdin_mock_detects_ssn_without_phi_previews -v
```

Expected: FAIL because `category_counts.get('SSN')` is `None` and `[SSN]` is absent.

- [ ] **Step 3: Write minimal implementation**

In `scripts/privacy_filter/run_privacy_filter.py`, add the bounded regex near the other fallback regexes:

```python
SSN_RE = re.compile(r'(?<!\d)\d{3}-\d{2}-\d{4}(?!\d)')
```

Then add this loop in `heuristic_detect` after the DATE loop and before ADDRESS detection:

```python
    for m in SSN_RE.finditer(text):
        add_span(spans, 'SSN', m.start(), m.end())
```

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
python scripts/privacy_filter/test_run_privacy_filter.py PrivacyFilterRunnerFailureTests.test_stdin_mock_detects_ssn_without_phi_previews -v
```

Expected: PASS.

- [ ] **Step 5: Run broader verification**

Run:

```bash
python scripts/privacy_filter/test_run_privacy_filter.py -v
cargo test -p mdid-cli privacy_filter_text_detects_address_category -- --nocapture
```

Expected: Python runner tests PASS; existing CLI category validation smoke remains PASS with the expanded runner category behavior.

- [ ] **Step 6: Commit**

```bash
git add scripts/privacy_filter/test_run_privacy_filter.py scripts/privacy_filter/run_privacy_filter.py docs/superpowers/plans/2026-05-01-privacy-filter-ssn-detection.md
git commit -m "feat(cli): detect ssn pii in privacy filter"
```

### Task 2: README completion and evidence truth-sync

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update README evidence**

Update the completion snapshot/evidence text to mention the landed `mdid-cli privacy-filter-text` SSN detection slice. Keep Browser/Web and Desktop at their existing target cap unless controller-visible facts prove new landed surface capability. Document that this round is CLI/runtime-only and therefore Browser/Desktop +5% rule is FAIL.

- [ ] **Step 2: Run README consistency checks**

Run:

```bash
git diff -- README.md
git diff --check
```

Expected: README diff mentions SSN detection, no whitespace errors.

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: truth-sync privacy filter ssn detection"
```

## Self-Review

- Spec coverage: The plan implements one CLI/runtime-only Privacy Filter synthetic text POC increment: SSN-like identifier detection/masking. It does not add OCR, visual redaction, image redaction, handwriting, PDF rewrite/export, browser UI, desktop UI, or agent/controller semantics.
- Placeholder scan: No TBD/TODO/implement-later placeholders remain.
- Type consistency: The new category label is consistently `SSN` in tests, runner spans, category counts, and README evidence.
