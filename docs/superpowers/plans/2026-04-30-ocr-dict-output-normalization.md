# OCR Dict Output Normalization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Strengthen the bounded PP-OCRv5 mobile OCR runner so newer dict-style OCR outputs with `rec_texts` normalize to plain text for downstream text-only PII evaluation.

**Architecture:** Keep the existing `scripts/ocr_eval/run_small_ocr.py` runner narrow: it still emits UTF-8 text only, makes no visual-redaction/PDF-rewrite claims, and only improves local result normalization. The change is a focused parser enhancement plus fixture-free unit coverage that proves a PaddleOCR-like dict result can feed the existing OCR handoff and Privacy Filter text contract.

**Tech Stack:** Python runner scripts, pytest, synthetic OCR contract tests, existing Privacy Filter text-only mock/validator.

---

## File Structure

- Modify: `tests/test_ocr_runner_contract.py` — add a RED test for dict-style `rec_texts` outputs returned by newer PaddleOCR APIs.
- Modify: `scripts/ocr_eval/run_small_ocr.py` — extend `iter_text_fragments()` to yield strings from dict values named `rec_texts` while preserving existing behavior for strings, tuples, lists, `text`, `transcription`, and `rec_text`.
- Modify: `docs/research/small-ocr-spike-results.md` — record that dict-style result normalization is now covered locally without claiming real PP-OCRv5 model-quality verification.
- Modify: `README.md` — truth-sync verification evidence and completion notes; keep Browser/Web and Desktop percentages unchanged unless a landed surface capability exists.

### Task 1: Add dict-style OCR output normalization

**Files:**
- Modify: `tests/test_ocr_runner_contract.py`
- Modify: `scripts/ocr_eval/run_small_ocr.py`
- Modify: `docs/research/small-ocr-spike-results.md`
- Modify: `README.md`

- [x] **Step 1: Write the failing test**

Append this test to `tests/test_ocr_runner_contract.py`:

```python
def test_fake_paddleocr_dict_rec_texts_result_is_normalized(monkeypatch, capsys):
    runner = load_runner()

    class FakePaddleOCR:
        def __init__(self, **kwargs):
            self.kwargs = kwargs

        def ocr(self, image_path, **kwargs):
            assert image_path == str(FIXTURE_IMAGE)
            return [{"rec_texts": ["Patient Jane Example", "MRN MRN-12345"]}]

    class FakePaddleModule:
        PaddleOCR = FakePaddleOCR

    monkeypatch.setitem(sys.modules, "paddleocr", FakePaddleModule)

    code = runner.main([str(FIXTURE_IMAGE)])

    captured = capsys.readouterr()
    assert code == 0
    assert captured.out == "Patient Jane Example\nMRN MRN-12345\n"
    assert captured.err == ""
```

- [x] **Step 2: Run test to verify it fails**

Run:

```bash
python -m pytest tests/test_ocr_runner_contract.py::test_fake_paddleocr_dict_rec_texts_result_is_normalized -q
```

Expected: FAIL because the runner currently ignores dict-style `rec_texts` arrays and emits empty stdout.

- [x] **Step 3: Write minimal implementation**

In `scripts/ocr_eval/run_small_ocr.py`, change the dict branch of `iter_text_fragments()` from:

```python
    if isinstance(node, dict):
        for key in ("text", "transcription", "rec_text"):
            value = node.get(key)
            if isinstance(value, str):
                yield value
        return
```

to:

```python
    if isinstance(node, dict):
        for key in ("text", "transcription", "rec_text"):
            value = node.get(key)
            if isinstance(value, str):
                yield value
        rec_texts = node.get("rec_texts")
        if isinstance(rec_texts, list):
            for value in rec_texts:
                if isinstance(value, str):
                    yield value
        return
```

- [x] **Step 4: Run targeted tests to verify pass**

Run:

```bash
python -m pytest tests/test_ocr_runner_contract.py -q
```

Expected: PASS for all OCR runner contract tests.

- [x] **Step 5: Verify OCR-to-Privacy-Filter chain still passes**

Run:

```bash
python scripts/ocr_eval/run_small_ocr.py --mock scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png > /tmp/small-ocr-output.txt
python scripts/ocr_eval/validate_small_ocr_output.py /tmp/small-ocr-output.txt scripts/ocr_eval/fixtures/synthetic_printed_phi_expected.txt
python scripts/ocr_eval/build_ocr_handoff.py \
  --source scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png \
  --input /tmp/small-ocr-output.txt \
  --output /tmp/ocr-handoff.json
python scripts/ocr_eval/validate_ocr_handoff.py /tmp/ocr-handoff.json
python - <<'PY'
import json
from pathlib import Path
obj = json.loads(Path('/tmp/ocr-handoff.json').read_text())
Path('/tmp/ocr-normalized-text.txt').write_text(obj['normalized_text'], encoding='utf-8')
PY
python scripts/privacy_filter/run_privacy_filter.py --mock /tmp/ocr-normalized-text.txt > /tmp/privacy-filter-output.json
python scripts/privacy_filter/validate_privacy_filter_output.py /tmp/privacy-filter-output.json
```

Expected: every command exits zero; the chain remains text-only and synthetic-fixture bounded.

- [x] **Step 6: Update docs truthfully**

Add this bullet under the verification/results section of `docs/research/small-ocr-spike-results.md`:

```markdown
- Dict-style OCR output normalization: `tests/test_ocr_runner_contract.py::test_fake_paddleocr_dict_rec_texts_result_is_normalized` covers newer PaddleOCR-like `{"rec_texts": [...]}` outputs and confirms they normalize to plain UTF-8 lines for downstream text-only PII evaluation. This is parser compatibility evidence only; it does not verify real PP-OCRv5 model weights, page detection, visual redaction, or PDF rewrite/export.
```

In `README.md`, append one sentence to the bounded PP-OCRv5 evidence paragraph:

```markdown
A follow-up OCR runner hardening test covers newer dict-style `rec_texts` OCR results and verifies they normalize into plain text for the same downstream Privacy Filter contract; this remains parser compatibility evidence, not real model-quality or visual-redaction evidence.
```

- [x] **Step 7: Run final verification**

Run:

```bash
python -m pytest tests/test_ocr_runner_contract.py tests/test_ocr_handoff_contract.py -q
python scripts/privacy_filter/run_privacy_filter.py --mock scripts/privacy_filter/fixtures/sample_text_input.txt > /tmp/privacy-filter-output.json
python scripts/privacy_filter/validate_privacy_filter_output.py /tmp/privacy-filter-output.json
cargo test -p mdid-cli ocr_handoff -- --nocapture
git diff --check
```

Expected: all tests/validators pass and `git diff --check` reports no whitespace errors.

- [x] **Step 8: Commit**

Run:

```bash
git add README.md docs/research/small-ocr-spike-results.md docs/superpowers/plans/2026-04-30-ocr-dict-output-normalization.md scripts/ocr_eval/run_small_ocr.py tests/test_ocr_runner_contract.py
git commit -m "fix(ocr): normalize dict-style OCR text outputs"
```

Expected: commit succeeds on the current feature branch.

## Self-Review

- Spec coverage: the plan directly advances the PP-OCRv5 mobile synthetic OCR extraction spike by making the local runner compatible with a common newer PaddleOCR output shape and preserving the downstream Privacy Filter text-only handoff.
- Placeholder scan: no TBD/TODO/implement-later placeholders remain.
- Type consistency: the test and implementation both use `rec_texts` and preserve `main(argv) -> int`, `iter_text_fragments(node) -> Iterable[str]`, and plain stdout text contracts.
