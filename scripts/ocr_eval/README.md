# Small OCR bounded spike

## Purpose
This directory is for a bounded local OCR extraction spike only.

## Non-goals
- `visual_redaction`
- `final_pdf_rewrite_export`
- `handwriting_recognition`
- `full_page_detection_or_segmentation`
- `complete_ocr_pipeline`

This spike does not perform image/pixel redaction, browser UI work, desktop UI work, visual redaction, final PDF rewrite/export, handwriting recognition, full-page detection/segmentation, or a complete OCR pipeline.

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

### JSON extraction contract mode
```bash
python scripts/ocr_eval/run_small_ocr.py --mock --json scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png > /tmp/ocr-extraction.json
```

`--json` preserves the same bounded extraction scope while emitting a strict text-only handoff object with `candidate: "PP-OCRv5_mobile_rec"`, `engine: "PP-OCRv5-mobile-bounded-spike"`, `scope: "printed_text_line_extraction_only"`, `privacy_filter_contract: "text_only_normalized_input"`, `extracted_text`, and whitespace-normalized `normalized_text`. Mock JSON uses `engine_status: "deterministic_synthetic_fixture_fallback"`; successful local PaddleOCR execution uses `engine_status: "local_paddleocr_execution"`.

### Text-only Privacy Filter handoff check
```bash
python scripts/ocr_eval/run_small_ocr.py --mock --json scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png > /tmp/ocr-extraction.json
python - <<'PY'
import json
from pathlib import Path
handoff = json.loads(Path('/tmp/ocr-extraction.json').read_text(encoding='utf-8'))
Path('/tmp/ocr-normalized-text.txt').write_text(handoff['normalized_text'], encoding='utf-8')
PY
python scripts/privacy_filter/run_privacy_filter.py --mock /tmp/ocr-normalized-text.txt > /tmp/privacy-filter-output.json
python scripts/privacy_filter/validate_privacy_filter_output.py /tmp/privacy-filter-output.json
```

### OCR-to-Privacy-Filter corpus wrapper evidence
```bash
cargo run -p mdid-cli -- ocr-to-privacy-filter-corpus \
  --fixture-dir scripts/ocr_eval/fixtures/corpus \
  --ocr-runner-path scripts/ocr_eval/run_ocr_handoff_corpus.py \
  --privacy-runner-path scripts/privacy_filter/run_privacy_filter.py \
  --bridge-runner-path scripts/ocr_eval/run_ocr_to_privacy_filter_corpus.py \
  --report-path /tmp/ocr-to-privacy-filter-corpus.json
```

The CLI wrapper delegates to the local bridge runner, validates its PHI-safe aggregate output, and writes the wrapper contract with `artifact: ocr_to_privacy_filter_corpus`, `ocr_scope: printed_text_line_extraction_only`, `privacy_scope: text_only_pii_detection`, `total_detected_span_count`, and `network_api_called: false`. The stdout summary redacts the report path and includes only aggregate fields. This is CLI/runtime evidence only; Browser/Web and Desktop remain unchanged at 99% and do not run OCR or Privacy Filter from this wrapper.

The Privacy Filter remains a downstream **text-only** PII detection/masking check. This OCR spike and CLI wrapper do not perform visual redaction, image/pixel redaction, final PDF rewrite/export, handwriting recognition, page detection/segmentation, browser UI, desktop UI, or a complete OCR pipeline.
