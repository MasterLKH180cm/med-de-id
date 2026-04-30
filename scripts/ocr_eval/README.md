# Small OCR bounded spike

## Purpose
This directory is for a bounded local OCR extraction spike only.

## Non-goals
- not visual redaction
- not image/pixel redaction
- not handwritten OCR solved
- not final PDF rewrite/export
- not full page OCR unless a separate detector/cropping stage is added later

## Fixtures
The first honest spike uses a **pre-cropped synthetic printed text-line image**, not a full page.

## Bootstrap
Real model path requires PaddleOCR stack to be installed locally.
This repo also supports a bounded plumbing-only mode via `--mock` for extraction-contract validation without claiming the real OCR model ran.

## Commands
### RED (real model missing expected)
```bash
python scripts/ocr_eval/run_small_ocr.py scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png
```

### GREEN (plumbing only)
```bash
python scripts/ocr_eval/run_small_ocr.py --mock scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png > /tmp/small-ocr-output.txt
python scripts/ocr_eval/validate_small_ocr_output.py /tmp/small-ocr-output.txt scripts/ocr_eval/fixtures/synthetic_printed_phi_expected.txt
python scripts/ocr_eval/build_ocr_handoff.py --source scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png --input /tmp/small-ocr-output.txt --output /tmp/ocr-handoff.json
python scripts/ocr_eval/validate_ocr_handoff.py /tmp/ocr-handoff.json
```

Use of `--mock` proves only extraction/handoff plumbing, not real model quality.
