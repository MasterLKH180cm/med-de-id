# OCR Synthetic Corpus Handoff Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded CLI/runtime synthetic OCR corpus handoff proof that multiple printed-text fixture samples can be normalized into text-only Privacy Filter inputs without claiming visual redaction or PDF rewrite/export.

**Architecture:** This is a CLI/runtime-only spike under `scripts/ocr_eval/` that aggregates synthetic printed-text fixture `.txt` files into PHI-safe corpus evidence and validates each sample through the existing OCR handoff contract. It does not add browser/desktop UI, agent/controller workflow semantics, visual redaction, handwriting recognition, page detection, or final PDF rewrite/export.

**Tech Stack:** Python helpers and pytest, existing OCR handoff validator, markdown evidence in README/research docs.

---

## File Structure

- Create: `scripts/ocr_eval/fixtures/corpus/synthetic_patient_label_01.txt` — synthetic printed-text OCR sample text.
- Create: `scripts/ocr_eval/fixtures/corpus/synthetic_patient_label_02.txt` — second synthetic printed-text OCR sample text.
- Create: `scripts/ocr_eval/run_ocr_handoff_corpus.py` — local aggregate corpus runner that validates each fixture as an OCR handoff candidate and writes PHI-safe aggregate JSON.
- Create: `tests/test_ocr_handoff_corpus.py` — TDD coverage for aggregate success and malformed/unsafe cases.
- Modify: `docs/research/small-ocr-spike-results.md` — add corpus evidence and non-goals.
- Modify: `README.md` — truth-sync evidence without inflating Browser/Web/Desktop completion.

### Task 1: Add OCR handoff synthetic corpus runner

**Files:**
- Create: `scripts/ocr_eval/fixtures/corpus/synthetic_patient_label_01.txt`
- Create: `scripts/ocr_eval/fixtures/corpus/synthetic_patient_label_02.txt`
- Create: `scripts/ocr_eval/run_ocr_handoff_corpus.py`
- Create: `tests/test_ocr_handoff_corpus.py`

- [ ] **Step 1: Write the failing tests**

Create `tests/test_ocr_handoff_corpus.py` with tests that call `scripts/ocr_eval/run_ocr_handoff_corpus.py` through subprocess. The success test must assert the report contains only aggregate fields (`engine`, `scope`, `fixture_count`, `ready_fixture_count`, `total_char_count`, `fixtures`, `non_goals`, `privacy_filter_contract`) and does not contain raw synthetic PHI such as `Jane Example`, `MRN-12345`, `jane@example.com`, `John Sample`, or `MRN-67890`. Failure tests must assert a missing fixture directory, empty directory, and empty fixture file exit nonzero and do not leave the requested report path behind.

- [ ] **Step 2: Run tests to verify RED**

Run: `pytest tests/test_ocr_handoff_corpus.py -q`
Expected: FAIL because `scripts/ocr_eval/run_ocr_handoff_corpus.py` does not exist yet.

- [ ] **Step 3: Add fixtures and minimal runner**

Create two synthetic-only fixture text files under `scripts/ocr_eval/fixtures/corpus/`. Implement `run_ocr_handoff_corpus.py` with `argparse --fixture-dir --output`, sorted `*.txt` fixtures, whitespace normalization, non-empty validation, PHI-safe fixture IDs (`fixture_001`, `fixture_002`), aggregate counts only, required non-goals (`visual_redaction`, `final_pdf_rewrite_export`, `handwriting_recognition`, `full_page_detection_or_segmentation`, `complete_ocr_pipeline`), and stale output cleanup before every run and on failure. The runner must not include raw text, masked text, spans, previews, or source filenames in the report.

- [ ] **Step 4: Run tests to verify GREEN**

Run: `pytest tests/test_ocr_handoff_corpus.py -q`
Expected: PASS.

- [ ] **Step 5: Run supporting verification**

Run:
```bash
python scripts/ocr_eval/run_ocr_handoff_corpus.py --fixture-dir scripts/ocr_eval/fixtures/corpus --output /tmp/ocr-handoff-corpus.json
python -m py_compile scripts/ocr_eval/run_ocr_handoff_corpus.py
python - <<'PY'
import json
from pathlib import Path
report = Path('/tmp/ocr-handoff-corpus.json').read_text(encoding='utf-8')
for forbidden in ['Jane Example', 'MRN-12345', 'jane@example.com', 'John Sample', 'MRN-67890']:
    assert forbidden not in report, forbidden
obj = json.loads(report)
assert obj['fixture_count'] == 2
assert obj['ready_fixture_count'] == 2
assert obj['privacy_filter_contract'] == 'text_only_normalized_input'
PY
git diff --check
```
Expected: all commands exit 0.

- [ ] **Step 6: Commit**

Run:
```bash
git add scripts/ocr_eval/fixtures/corpus scripts/ocr_eval/run_ocr_handoff_corpus.py tests/test_ocr_handoff_corpus.py
git commit -m "feat(ocr): add synthetic handoff corpus runner"
```

### Task 2: Truth-sync OCR corpus docs and README evidence

**Files:**
- Modify: `docs/research/small-ocr-spike-results.md`
- Modify: `README.md`

- [ ] **Step 1: Write failing docs check**

Run:
```bash
python - <<'PY'
from pathlib import Path
combined = Path('README.md').read_text(encoding='utf-8') + '\n' + Path('docs/research/small-ocr-spike-results.md').read_text(encoding='utf-8')
required = [
    'OCR handoff synthetic corpus',
    'text-only Privacy Filter input contract',
    'printed-text extraction only',
    'not visual redaction',
    'not final PDF rewrite/export',
]
missing = [term for term in required if term not in combined]
if missing:
    raise SystemExit('missing docs terms: ' + ', '.join(missing))
PY
```
Expected: FAIL until docs explicitly mention the corpus evidence and non-goals.

- [ ] **Step 2: Update docs**

Add a short `OCR handoff synthetic corpus evidence` section to `docs/research/small-ocr-spike-results.md` with the exact corpus runner command and state it proves only aggregate fixture readiness for downstream text-only Privacy Filter input, not OCR quality, visual redaction, handwriting recognition, browser/desktop integration, or PDF rewrite/export. Update README evidence to mention the new CLI/runtime corpus runner while keeping completion honest at CLI 95%, Browser/Web 93%, Desktop app 93%, Overall 95% unless the controller-visible rubric facts justify a separate re-baseline.

- [ ] **Step 3: Run docs check to verify GREEN**

Run the Python docs check from Step 1 again.
Expected: PASS.

- [ ] **Step 4: Run final verification**

Run:
```bash
pytest tests/test_ocr_handoff_corpus.py -q
python scripts/ocr_eval/run_ocr_handoff_corpus.py --fixture-dir scripts/ocr_eval/fixtures/corpus --output /tmp/ocr-handoff-corpus.json
python -m py_compile scripts/ocr_eval/run_ocr_handoff_corpus.py
git diff --check
```
Expected: all commands exit 0.

- [ ] **Step 5: Commit**

Run:
```bash
git add README.md docs/research/small-ocr-spike-results.md
git commit -m "docs: truth-sync OCR handoff corpus evidence"
```

## Self-Review

Spec coverage: Task 1 implements the bounded PP-OCRv5 mobile synthetic fixture corpus handoff evidence and Task 2 truth-syncs docs/README without browser/desktop completion inflation.

Placeholder scan: No TBD, TODO, fill-in, or similar placeholders remain.

Type consistency: The runner and tests use `fixture_count`, `ready_fixture_count`, `privacy_filter_contract`, and the required non-goals consistently across tasks.
