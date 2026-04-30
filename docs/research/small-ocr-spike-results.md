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
Result: PASS

## Current limitation
The real PaddleOCR stack is not installed locally in this environment, so the real OCR path currently exits with a truthful error instructing the operator to install the OCR stack or use `--mock` for plumbing-only validation.

## What this proves
- The first bounded OCR spike now has a reproducible synthetic fixture set
- The first honest OCR spike stays recognizer-first on a pre-cropped line image
- OCR output can now be normalized and handed to a downstream text-PII stage shape
- The handoff contract is explicitly validated rather than only written

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
