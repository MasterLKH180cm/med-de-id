# Privacy Filter OPF Normalization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Harden the CLI-first Privacy Filter text-only POC so explicitly invoked local `opf` output is normalized into the same PHI-safe contract as fallback synthetic detection.

**Architecture:** Add pure normalization helpers inside `scripts/privacy_filter/run_privacy_filter.py` and test them with mocked `opf` subprocess output; this stays CLI/runtime-only and never adds OCR, visual redaction, PDF rewrite/export, browser/desktop UI, or agent/controller semantics. The runner should accept the already-supported canonical OPF shape (`masked_text` + `spans`) and a common alternate shape (`text` + `entities`) while preserving redacted previews only.

**Tech Stack:** Python stdlib, existing `scripts/privacy_filter` runner/validator, pytest/unittest.

---

## File Structure

- Modify: `scripts/privacy_filter/run_privacy_filter.py` — extract OPF JSON normalization into a testable helper that returns the bounded output contract.
- Modify: `scripts/privacy_filter/test_run_privacy_filter.py` — add RED/GREEN tests for explicit OPF normalization, PHI-safe stderr/stdout behavior, and alternate entity field mapping.
- Modify: `scripts/privacy_filter/README.md` — document explicit `--use-opf` normalization scope and non-goals.
- Modify: `README.md` — truth-sync CLI completion evidence without inflating Browser/Web/Desktop unless landed capability supports it.

### Task 1: Normalize explicit OPF JSON variants into the bounded Privacy Filter contract

**Files:**
- Modify: `scripts/privacy_filter/run_privacy_filter.py`
- Modify: `scripts/privacy_filter/test_run_privacy_filter.py`

- [ ] **Step 1: Write failing tests**

Append tests to `scripts/privacy_filter/test_run_privacy_filter.py` that mock `shutil.which('opf')` and `run_opf_with_stdin()` through the module object, invoke `module.main()` with a temp input path and `--use-opf`, capture stdout/stderr, and assert:

```python
def test_explicit_opf_canonical_json_is_normalized_without_phi_previews(self):
    module = load_runner_module()
    with tempfile.TemporaryDirectory() as tmp:
        input_path = Path(tmp) / 'input.txt'
        input_path.write_text('Patient Jane Example has MRN-12345\n', encoding='utf-8')
        raw_opf = json.dumps({
            'masked_text': 'Patient [NAME] has [MRN]\n',
            'spans': [
                {'label': 'NAME', 'start': 8, 'end': 20, 'preview': 'Jane Example'},
                {'label': 'MRN', 'start': 25, 'end': 34, 'text': 'MRN-12345'},
            ],
        })
        with mock.patch.object(module.shutil, 'which', return_value='/tmp/opf'), \
             mock.patch.object(module, 'run_opf_with_stdin', return_value=raw_opf), \
             mock.patch.object(sys, 'argv', ['run_privacy_filter.py', '--use-opf', str(input_path)]), \
             mock.patch('sys.stdout', new_callable=io.StringIO) as stdout, \
             mock.patch('sys.stderr', new_callable=io.StringIO) as stderr:
            module.main()
        payload = json.loads(stdout.getvalue())
    self.assertEqual(stderr.getvalue(), '')
    self.assertEqual(payload['metadata']['engine'], 'openai_privacy_filter_opf')
    self.assertEqual(payload['summary']['detected_span_count'], 2)
    self.assertEqual(payload['summary']['category_counts'], {'MRN': 1, 'NAME': 1})
    self.assertEqual(payload['spans'][0]['preview'], '<redacted>')
    self.assertNotIn('Jane Example', json.dumps(payload))
    self.assertNotIn('MRN-12345', json.dumps(payload))
```

Also add an alternate-shape test:

```python
def test_explicit_opf_entities_shape_is_normalized_to_spans(self):
    module = load_runner_module()
    raw_opf = json.dumps({
        'text': 'Patient [NAME] email [EMAIL]',
        'entities': [
            {'type': 'NAME', 'start': '8', 'end': '20', 'value': 'Jane Example'},
            {'category': 'EMAIL', 'begin': 27, 'finish': 44, 'value': 'jane@example.test'},
        ],
    })
    payload = module.normalize_opf_json(raw_opf, input_char_count=45)
    self.assertEqual(payload['masked_text'], 'Patient [NAME] email [EMAIL]')
    self.assertEqual(payload['summary']['category_counts'], {'EMAIL': 1, 'NAME': 1})
    self.assertEqual(payload['spans'][0], {'label': 'NAME', 'start': 8, 'end': 20, 'preview': '<redacted>'})
    self.assertEqual(payload['spans'][1], {'label': 'EMAIL', 'start': 27, 'end': 44, 'preview': '<redacted>'})
    self.assertNotIn('Jane Example', json.dumps(payload))
    self.assertNotIn('jane@example.test', json.dumps(payload))
```

- [ ] **Step 2: Run tests to verify RED**

Run: `python -m pytest scripts/privacy_filter/test_run_privacy_filter.py -q`
Expected: FAIL because `normalize_opf_json` does not exist and the current inline OPF path leaks raw `masked_text` if supplied by OPF.

- [ ] **Step 3: Write minimal implementation**

In `scripts/privacy_filter/run_privacy_filter.py`, add helpers:

```python
def _coerce_int(value, default=0):
    try:
        return int(value)
    except (TypeError, ValueError):
        return default


def _span_label(span):
    return str(span.get('label') or span.get('type') or span.get('category') or 'UNKNOWN')


def _span_start(span):
    return _coerce_int(span.get('start', span.get('begin', 0)))


def _span_end(span):
    return _coerce_int(span.get('end', span.get('finish', span.get('stop', 0))))


def normalize_opf_json(raw: str, input_char_count: int):
    obj = json.loads(raw)
    if not isinstance(obj, dict):
        obj = {}
    raw_spans = obj.get('spans')
    if not isinstance(raw_spans, list):
        raw_spans = obj.get('entities') if isinstance(obj.get('entities'), list) else []
    spans = []
    counts = {}
    for item in raw_spans:
        if not isinstance(item, dict):
            continue
        label = _span_label(item)
        start = _span_start(item)
        end = _span_end(item)
        counts[label] = counts.get(label, 0) + 1
        spans.append({'label': label, 'start': start, 'end': end, 'preview': '<redacted>'})
    spans.sort(key=lambda s: (s['start'], s['end'], s['label']))
    counts = {key: counts[key] for key in sorted(counts)}
    masked_text = obj.get('masked_text') if isinstance(obj.get('masked_text'), str) else obj.get('text')
    if not isinstance(masked_text, str):
        masked_text = '<missing>'
    return {
        'summary': {
            'input_char_count': input_char_count,
            'detected_span_count': len(spans),
            'category_counts': counts,
        },
        'masked_text': masked_text,
        'spans': spans,
        'metadata': real_opf_metadata(),
    }
```

Replace the inline OPF output construction in `main()` with `print(json.dumps(normalize_opf_json(raw, len(text)), ensure_ascii=False, indent=2))`.

- [ ] **Step 4: Run tests to verify GREEN**

Run: `python -m pytest scripts/privacy_filter/test_run_privacy_filter.py -q`
Expected: PASS.

- [ ] **Step 5: Run validator smoke checks**

Run:

```bash
python scripts/privacy_filter/run_privacy_filter.py --mock scripts/privacy_filter/fixtures/sample_text_input.txt > /tmp/privacy-filter-output.json
python scripts/privacy_filter/validate_privacy_filter_output.py /tmp/privacy-filter-output.json
python scripts/privacy_filter/run_synthetic_corpus.py --fixture-dir scripts/privacy_filter/fixtures/corpus --output /tmp/privacy-filter-corpus.json
python -m json.tool /tmp/privacy-filter-corpus.json >/tmp/privacy-filter-corpus.pretty.json
! grep -E 'Jane Example|MRN-12345|jane@example.test|555-111-2222' /tmp/privacy-filter-corpus.json
```

Expected: all commands exit 0.

- [ ] **Step 6: Commit**

Run:

```bash
git add scripts/privacy_filter/run_privacy_filter.py scripts/privacy_filter/test_run_privacy_filter.py
git commit -m "fix(cli): normalize explicit privacy filter opf output"
```

### Task 2: Truth-sync docs and README completion evidence for OPF normalization

**Files:**
- Modify: `scripts/privacy_filter/README.md`
- Modify: `README.md`

- [ ] **Step 1: Write failing docs check**

Run:

```bash
python - <<'PY'
from pathlib import Path
readme = Path('README.md').read_text()
privacy = Path('scripts/privacy_filter/README.md').read_text()
required = [
    'explicit --use-opf',
    'normalized into the bounded text-only contract',
    'redacted previews only',
    'not OCR',
    'not visual redaction',
]
missing = [term for term in required if term not in readme + '\n' + privacy]
if missing:
    raise SystemExit('missing docs terms: ' + ', '.join(missing))
PY
```

Expected: FAIL until docs explicitly mention the OPF normalization evidence and non-goals.

- [ ] **Step 2: Update docs**

Document explicit `--use-opf` behavior in `scripts/privacy_filter/README.md`: local `opf` is never auto-used, PHI is sent via stdin only, canonical `spans` and alternate `entities` JSON shapes normalize into this repo's bounded text-only contract, previews are redacted, and this is not OCR/visual redaction/final PDF rewrite/export/browser/desktop integration.

Update `README.md` completion evidence to mention the new CLI/runtime OPF normalization tests. Keep Browser/Web and Desktop unchanged unless controller-visible landed capability supports movement. If no new rubric requirement is introduced beyond hardening the existing Privacy Filter CLI POC, state that the round uses the existing rubric.

- [ ] **Step 3: Run docs check and verification**

Run the Python docs check from Step 1, then:

```bash
python -m pytest scripts/privacy_filter/test_run_privacy_filter.py -q
git diff --check
```

Expected: PASS.

- [ ] **Step 4: Commit**

Run:

```bash
git add README.md scripts/privacy_filter/README.md
git commit -m "docs: truth-sync privacy filter opf normalization"
```

## Self-Review

Spec coverage: Task 1 implements and tests explicit OPF JSON normalization for the existing CLI-first Privacy Filter text-only POC; Task 2 documents evidence and non-goals. No browser/desktop, OCR, visual redaction, PDF rewrite/export, or agent/controller semantics are added.

Placeholder scan: No TBD/TODO/fill-in placeholders remain.

Type consistency: Helper names and field names are defined before use; tests call `normalize_opf_json(raw, input_char_count=...)` exactly as implemented.
