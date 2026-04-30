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

### CLI bounded OCR handoff wrapper
```bash
cargo run -p mdid-cli -- ocr-handoff --image-path scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png --ocr-runner-path scripts/ocr_eval/run_small_ocr.py --handoff-builder-path scripts/ocr_eval/build_ocr_handoff.py --report-path /tmp/ocr-handoff.json
```

The CLI wrapper invokes `run_small_ocr.py --mock <image>`, writes the bounded text to a temporary handoff input, invokes `build_ocr_handoff.py`, validates the JSON contract, deletes the temporary text, and prints a JSON summary containing `command: "ocr-handoff"` plus the report path.

Use of `--mock` proves only extraction/handoff plumbing, not real model quality. The handoff artifact truthfully identifies the candidate as `PP-OCRv5_mobile_rec`, the bounded spike engine as `PP-OCRv5-mobile-bounded-spike`, and the current fallback status as `deterministic_synthetic_fixture_fallback` when real PP-OCRv5 local inference is not installed/wired.

### Text-only Privacy Filter handoff check
```bash
python scripts/ocr_eval/run_small_ocr.py --mock scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png > /tmp/small-ocr-output.txt
python scripts/ocr_eval/build_ocr_handoff.py --source scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png --input /tmp/small-ocr-output.txt --output /tmp/ocr-handoff.json
python scripts/ocr_eval/validate_ocr_handoff.py /tmp/ocr-handoff.json
python scripts/privacy_filter/run_privacy_filter.py --mock /tmp/small-ocr-output.txt > /tmp/privacy-filter-output.json
python scripts/privacy_filter/validate_privacy_filter_output.py /tmp/privacy-filter-output.json
```

The Privacy Filter remains a downstream **text-only** PII detection/masking check. This OCR spike and CLI wrapper do not perform visual redaction, image/pixel redaction, final PDF rewrite/export, handwriting recognition, page detection/segmentation, browser UI, desktop UI, or a complete OCR pipeline.
