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
python scripts/privacy_filter/run_privacy_filter.py --mock /tmp/small-ocr-output.txt > /tmp/privacy-filter-output.json
python scripts/privacy_filter/validate_privacy_filter_output.py /tmp/privacy-filter-output.json
```
Result: PASS. This only verifies that normalized OCR text can be handed into the existing text-only privacy-filter contract; it does not make Privacy Filter an OCR engine.

## Current limitation
The real PaddleOCR stack is not installed locally in this environment, so the real OCR path currently exits with a truthful error instructing the operator to install the OCR stack or use `--mock` for plumbing-only validation.

## What this proves
- The first bounded OCR spike now has a reproducible synthetic fixture set
- The first honest OCR spike stays recognizer-first on a pre-cropped line image
- OCR output can now be normalized and handed to a downstream text-PII stage shape
- The handoff contract is explicitly validated rather than only written
- The handoff metadata truthfully names PP-OCRv5 mobile as the bounded candidate while recording the current deterministic synthetic fallback status when PP-OCRv5 is not installed/wired
- The downstream Privacy Filter check remains text-only and consumes normalized extracted text, not pixels/images

## What this does NOT prove
- real OCR quality on this machine
- full page OCR
- detector/cropping quality
- visual redaction
- final PDF rewrite/export

## Verdict
- **Go for next step:** YES, continue with real local OCR install/evaluation
- **Go for claiming OCR solved:** NO
- **Reason:** plumbing is ready, but actual local OCR inference still needs installation and verification
