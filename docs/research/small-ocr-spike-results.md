# Small OCR Spike Results

## Status
- **Extraction/handoff plumbing status:** PASS
- **Real-model status:** NOT YET VERIFIED on this machine

## What was implemented
- `scripts/ocr_eval/run_small_ocr.py`
- `scripts/ocr_eval/validate_small_ocr_output.py`
- `scripts/ocr_eval/build_ocr_handoff.py`
- `scripts/ocr_eval/validate_ocr_handoff.py`
- synthetic text fixture
- synthetic pre-cropped line-image PNG fixture
- expected extracted text fixture
- OCR handoff expected-shape fixture
- bounded README with exact commands

## Verification run
### OCR handoff builder PHI-safe failure hardening

Task 1 landed OCR handoff builder PHI-safe failure hardening for CLI/runtime synthetic OCR handoff failures. The builder now rejects a missing OCR input file and empty OCR input text with generic PHI-safe stderr, removes stale outputs before failure, and preserves stale handoff report cleanup so callers do not accidentally consume old JSON reports after a failed build.

Evidence:

```bash
pytest tests/test_ocr_handoff_builder_failures.py -q

# OCR validator pipeline
python scripts/ocr_eval/run_small_ocr.py --mock scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png > /tmp/small-ocr-output.txt
python scripts/ocr_eval/validate_small_ocr_output.py /tmp/small-ocr-output.txt scripts/ocr_eval/fixtures/synthetic_printed_phi_expected.txt
python scripts/ocr_eval/build_ocr_handoff.py --source scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png --input /tmp/small-ocr-output.txt --output /tmp/ocr-handoff.json
python scripts/ocr_eval/validate_ocr_handoff.py /tmp/ocr-handoff.json
python - <<'PY'
import json
from pathlib import Path
handoff = json.loads(Path('/tmp/ocr-handoff.json').read_text(encoding='utf-8'))
Path('/tmp/ocr-normalized-text.txt').write_text(handoff['normalized_text'], encoding='utf-8')
PY
python scripts/privacy_filter/run_privacy_filter.py --mock /tmp/ocr-normalized-text.txt > /tmp/privacy-filter-output.json
python scripts/privacy_filter/validate_privacy_filter_output.py /tmp/privacy-filter-output.json
```

Scope: this only hardens CLI/runtime synthetic OCR handoff failures. It is not visual redaction, browser/desktop integration, handwriting recognition, or final PDF rewrite/export.

### OCR-to-Privacy-Filter chain evidence

This standalone synthetic chain can be reproduced from a clean checkout without depending on prior `/tmp` files. It creates the OCR text output, builds and validates the OCR handoff JSON, extracts `normalized_text` as the text-only Privacy Filter input, runs the Privacy Filter, and validates both JSON outputs:

```bash
python scripts/ocr_eval/run_small_ocr.py --mock scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png > /tmp/small-ocr-output.txt
python scripts/ocr_eval/validate_small_ocr_output.py /tmp/small-ocr-output.txt scripts/ocr_eval/fixtures/synthetic_printed_phi_expected.txt
python scripts/ocr_eval/build_ocr_handoff.py --source scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png --input /tmp/small-ocr-output.txt --output /tmp/ocr-handoff.json
python scripts/ocr_eval/validate_ocr_handoff.py /tmp/ocr-handoff.json
python - <<'PY'
import json
from pathlib import Path
handoff = json.loads(Path('/tmp/ocr-handoff.json').read_text(encoding='utf-8'))
Path('/tmp/ocr-normalized-text.txt').write_text(handoff['normalized_text'], encoding='utf-8')
PY
python scripts/privacy_filter/run_privacy_filter.py --mock /tmp/ocr-normalized-text.txt > /tmp/ocr-privacy-filter.json
python scripts/privacy_filter/validate_privacy_filter_output.py /tmp/ocr-privacy-filter.json
python scripts/privacy_filter/validate_privacy_filter_output.py scripts/privacy_filter/fixtures/sample_text_expected_shape.json
```

Result: PASS in the bounded synthetic chain validators. This OCR-to-Privacy-Filter chain proves printed-text extraction only from the synthetic pre-cropped line fixture can be normalized and handed into the existing text-only Privacy Filter / text-only PII detection contract without PHI leaks in the wrapper output. It is not visual redaction, not final PDF rewrite/export, not handwriting recognition, not page detection/segmentation, not browser or desktop workflow capability, and not a complete OCR pipeline.

### OCR handoff synthetic corpus evidence

The bounded CLI/runtime synthetic corpus runner can be reproduced with:

```bash
python scripts/ocr_eval/run_ocr_handoff_corpus.py --fixture-dir scripts/ocr_eval/fixtures/corpus --output /tmp/ocr-handoff-corpus.json
```

Result: PASS for aggregate fixture readiness. This OCR handoff synthetic corpus evidence proves only that checked-in synthetic printed-text fixtures can be normalized into aggregate, PHI-safe readiness metadata for the downstream text-only Privacy Filter input contract. It is printed-text extraction only and is not OCR quality evidence, not visual redaction, not final PDF rewrite/export, not handwriting recognition, not browser/desktop integration, and not a complete OCR pipeline.

### OCR handoff synthetic corpus CLI wrapper evidence

The bounded CLI wrapper can be reproduced with:

```bash
mdid-cli ocr-handoff-corpus --fixture-dir scripts/ocr_eval/fixtures/corpus --runner-path scripts/ocr_eval/run_ocr_handoff_corpus.py --report-path <report.json>
python scripts/ocr_eval/run_ocr_handoff_corpus.py --fixture-dir scripts/ocr_eval/fixtures/corpus --output /tmp/ocr-handoff-corpus.json
cargo test -p mdid-cli ocr_handoff_corpus -- --nocapture
```

Result: PASS for the PP-OCRv5 mobile handoff corpus wrapper and direct runner. This documents printed-text extraction readiness only: the CLI validates aggregate-only PHI-safe readiness metadata for checked-in synthetic printed-text fixtures, the downstream text-only Privacy Filter input contract, strict report shape, sanitized fixture IDs, redacted report-path stdout, and stale-report cleanup on failure. It is not visual redaction, not image pixel redaction, not final PDF rewrite/export, not handwriting recognition, not full page detection, not browser integration, not desktop integration, not model-quality evidence, and not a complete OCR pipeline. Browser/Web +5 target: FAIL; Desktop app +5 target: FAIL because this round lands CLI/runtime blocker progress only, not browser/desktop capability. It adds no Browser/Desktop +5 surface progress and does not change completion: CLI 95%, Browser/Web 93%, Desktop app 93%, Overall 95%.

### OCR-to-Privacy-Filter corpus bridge evidence

The bounded CLI/runtime corpus bridge can be reproduced with:

```bash
python scripts/ocr_eval/run_ocr_to_privacy_filter_corpus.py --fixture-dir scripts/ocr_eval/fixtures/corpus --ocr-runner-path scripts/ocr_eval/run_ocr_handoff_corpus.py --privacy-runner-path scripts/privacy_filter/run_privacy_filter.py --output /tmp/ocr-to-privacy-filter-corpus.json
```

Result: PASS for the bounded PP-OCRv5 mobile synthetic handoff corpus composed with the existing text-only Privacy Filter runner. The bridge writes an aggregate-only PHI-safe report containing sanitized fixture IDs, readiness counts, Privacy Filter detected-span counts, Privacy Filter category counts, and explicit non-goals only.

The report intentionally omits raw OCR text, normalized text, masked text, raw spans, raw previews, fixture filenames, fixture paths, image data, bbox data, and raw synthetic PHI. This is CLI/runtime evidence for aggregate OCR handoff readiness flowing into text-only Privacy Filter detection only. It is not Browser/Web execution, not Desktop execution, not visual redaction, not image pixel redaction, not handwriting recognition, not final PDF rewrite/export, not OCR production readiness, and not model-quality benchmark evidence.

### CLI wrapper RED/GREEN evidence
```bash
cargo test -p mdid-cli ocr_handoff -- --nocapture
```
RED result before implementation: FAIL with `unknown command` for `ocr-handoff` and missing help text, proving no CLI handoff wrapper existed.

GREEN result after cleanup hardening: PASS, 8 targeted tests. The wrapper now validates missing flags/files, removes stale reports on OCR runner failure and non-UTF-8 OCR output, rejects oversized OCR runner stdout before writing a final report, rejects invalid or malformed handoff JSON contracts while removing the bad report, cleans up temporary OCR text, and succeeds end-to-end on `synthetic_printed_phi_line.png` with normalized text containing `Jane Doe`.

### Strict TDD RED evidence
```bash
python -m pytest tests/test_ocr_handoff_contract.py -q
```
Result before implementation: FAIL with `KeyError: 'candidate'`, proving the existing OCR handoff did not identify the PP-OCRv5 mobile candidate/engine/fallback metadata required for downstream text PII handoff.

### Mock extraction verification
```bash
python scripts/ocr_eval/run_small_ocr.py --mock scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png > /tmp/small-ocr-output.txt
python scripts/ocr_eval/validate_small_ocr_output.py /tmp/small-ocr-output.txt scripts/ocr_eval/fixtures/synthetic_printed_phi_expected.txt
```
Result: PASS

### Handoff verification
```bash
python scripts/ocr_eval/build_ocr_handoff.py --source scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png --input /tmp/small-ocr-output.txt --output /tmp/ocr-handoff.json
python scripts/ocr_eval/validate_ocr_handoff.py /tmp/ocr-handoff.json
```
Result: PASS. The JSON handoff now includes `candidate: PP-OCRv5_mobile_rec`, `engine: PP-OCRv5-mobile-bounded-spike`, `engine_status: deterministic_synthetic_fixture_fallback`, `scope: printed_text_line_extraction_only`, `privacy_filter_contract: text_only_normalized_input`, and explicit non-goals for visual redaction, final PDF rewrite/export, handwriting recognition, page detection/segmentation, and a complete OCR pipeline.

### Downstream text-only Privacy Filter handoff check
```bash
python - <<'PY'
import json
from pathlib import Path
handoff = json.loads(Path('/tmp/ocr-handoff.json').read_text(encoding='utf-8'))
Path('/tmp/ocr-normalized-text.txt').write_text(handoff['normalized_text'], encoding='utf-8')
PY
python scripts/privacy_filter/run_privacy_filter.py --mock /tmp/ocr-normalized-text.txt > /tmp/privacy-filter-output.json
python scripts/privacy_filter/validate_privacy_filter_output.py /tmp/privacy-filter-output.json
```
Result: PASS. This verifies that normalized OCR handoff text can be handed into the existing text-only privacy-filter contract; it does not make Privacy Filter an OCR engine.

## Current limitation
The real PaddleOCR stack is not installed locally in this environment, so the real OCR path currently exits with a truthful error instructing the operator to install the OCR stack or use `--mock` for plumbing-only validation.

## What this proves
- The first bounded OCR spike now has a reproducible synthetic fixture set
- The first honest OCR spike stays recognizer-first on a pre-cropped line image
- OCR output can now be normalized and handed to a downstream text-PII stage shape
- The OCR-to-Privacy-Filter chain proves printed-text extraction only into text-only Privacy Filter detection on synthetic normalized text
- The handoff contract is explicitly validated rather than only written
- The handoff metadata truthfully names PP-OCRv5 mobile as the bounded candidate while recording the current deterministic synthetic fallback status when PP-OCRv5 is not installed/wired
- The downstream Privacy Filter check remains text-only and consumes normalized extracted text, not pixels/images
- Dict-style OCR output normalization: `tests/test_ocr_runner_contract.py::test_fake_paddleocr_dict_rec_texts_result_is_normalized` covers newer PaddleOCR-like `{"rec_texts": [...]}` outputs and confirms they normalize to plain UTF-8 lines for downstream text-only PII evaluation. This is parser compatibility evidence only; it does not verify real PP-OCRv5 model weights, page detection, visual redaction, or PDF rewrite/export.

## What this does NOT prove
- real OCR quality on this machine
- full page OCR
- detector/cropping quality
- visual redaction
- final PDF rewrite/export
- handwriting recognition, page detection/segmentation, browser workflow capability, or desktop workflow capability

## Verdict
- **Go for next step:** YES, continue with real local OCR install/evaluation
- **Go for claiming OCR solved:** NO
- **Reason:** plumbing is ready, but actual local OCR inference still needs installation and verification
