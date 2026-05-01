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
cargo run -p mdid-cli -- ocr-handoff \
  --image-path scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png \
  --ocr-runner-path scripts/ocr_eval/run_small_ocr.py \
  --handoff-builder-path scripts/ocr_eval/build_ocr_handoff.py \
  --report-path /tmp/ocr-handoff.json \
  --summary-output /tmp/ocr-handoff-summary.json
```

Usage: `mdid-cli ocr-handoff --image-path <path> --ocr-runner-path <path> --handoff-builder-path <path> --report-path <report.json> [--summary-output <summary.json>] [--python-command <cmd>]`.

The CLI wrapper invokes `run_small_ocr.py --mock <image>`, writes the bounded text to a temporary handoff input, invokes `build_ocr_handoff.py`, validates the JSON contract, deletes the temporary text, and prints only an aggregate/path-redacted JSON summary such as `{"command":"ocr-handoff","report_path":"<redacted>","report_written":true,...}`. Stdout never echoes the caller-supplied report path or summary path, whether or not `--summary-output` is supplied.

Use of `--mock` proves only extraction/handoff plumbing, not real model quality. The handoff artifact truthfully identifies the candidate as `PP-OCRv5_mobile_rec`, the bounded spike engine as `PP-OCRv5-mobile-bounded-spike`, and the current fallback status as `deterministic_synthetic_fixture_fallback` when real PP-OCRv5 local inference is not installed/wired.

The optional `--summary-output <summary.json>` writes a secondary `ocr_handoff_summary` artifact only after the primary OCR handoff report validates. The summary is aggregate-only and PHI-safe: it contains bounded safe metadata, readiness booleans, counts, `network_api_called: false`, and explicit non-goals, while omitting raw OCR text, normalized text, source/fixture names, local paths, bbox/image data, spans, previews, masked text, and raw PHI. On failures after validation prerequisites begin, stale report, temporary text, and summary artifacts are removed so callers cannot mistake old evidence for the current run. If `--summary-output` is the same path as `--report-path`, or an equivalent lexical/existing-file alias such as `dir/report.json` versus `dir/./report.json`, the command fails before stale output cleanup with the fixed PHI/path-safe error `OCR handoff summary path must differ from report path`, preserving stale primary bytes for diagnosis.

This wrapper is CLI/runtime evidence only. It does not add Browser/Web OCR execution, Desktop OCR execution, visual redaction, image pixel redaction, handwriting recognition, final PDF rewrite/export, OCR model-quality proof, or a complete OCR pipeline.

### JSON extraction contract mode
```bash
python scripts/ocr_eval/run_small_ocr.py --mock --json scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png > /tmp/ocr-extraction.json
```

`--json` preserves the same bounded extraction scope while emitting a strict text-only handoff object with `candidate: "PP-OCRv5_mobile_rec"`, `engine: "PP-OCRv5-mobile-bounded-spike"`, `scope: "printed_text_line_extraction_only"`, `privacy_filter_contract: "text_only_normalized_input"`, `extracted_text`, and whitespace-normalized `normalized_text`. Mock JSON uses `engine_status: "deterministic_synthetic_fixture_fallback"`; successful local PaddleOCR execution uses `engine_status: "local_paddleocr_execution"`.

### CLI bounded OCR small JSON wrapper
```bash
cargo run -p mdid-cli -- ocr-small-json \
  --image-path scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png \
  --ocr-runner-path scripts/ocr_eval/run_small_ocr.py \
  --report-path /tmp/ocr-small-json-wrapper-report.json \
  --summary-output /tmp/ocr-small-json-wrapper-summary.json \
  --python-command python \
  --mock
python scripts/ocr_eval/validate_ocr_handoff.py /tmp/ocr-small-json-wrapper-report.json
```

The `mdid-cli ocr-small-json` wrapper runs the existing local PP-OCRv5 mobile small OCR candidate runner as `run_small_ocr.py --json` and adds `--mock` only when the CLI caller supplies `--mock`. It validates the same OCR handoff JSON contract, writes a validated OCR handoff JSON report, keeps stdout/errors PHI/path-safe with the report path redacted, rejects unknown extra fields, and removes stale report/summary artifacts on failure. The report is bounded to `scope: "printed_text_line_extraction_only"` and intentionally contains OCR text in `extracted_text` / `normalized_text` so downstream **text-only** Privacy Filter evaluation can consume normalized OCR text through `privacy_filter_contract: "text_only_normalized_input"`; do not treat the report itself as PHI-safe.

Omitting `--mock` attempts local PaddleOCR/PP-OCRv5 execution through `run_small_ocr.py`; this requires the PaddleOCR stack to be installed locally and remains a bounded printed-text extraction spike only. The optional `--summary-output <summary.json>` writes aggregate-only PP-OCRv5 mobile printed-text extraction readiness evidence for downstream text-only Privacy Filter evaluation. The summary omits raw OCR text, normalized text, source, local paths, bbox/image data, spans, previews, masked text, and raw PHI.

This wrapper is CLI/runtime evidence only. It is not OCR model-quality proof, not visual redaction, not image pixel redaction, not final PDF rewrite/export, not Browser/Web OCR execution, not Desktop OCR execution, and not a full OCR pipeline.

### CLI local PP-OCRv5 runtime attempt
```bash
mdid-cli ocr-small-json \
  --image-path scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png \
  --ocr-runner-path scripts/ocr_eval/run_small_ocr.py \
  --report-path /tmp/ocr-small-local.json \
  --summary-output /tmp/ocr-small-local-summary.json
```

This command intentionally omits `--mock`, so it attempts local PP-OCRv5 mobile printed-text extraction and then writes the same bounded report/summary contracts. It is not visual redaction, handwriting recognition, pixel redaction, final PDF rewrite/export, Browser/Desktop integration, or complete OCR pipeline evidence.

### Text-only Privacy Filter handoff check
```bash
python scripts/ocr_eval/run_small_ocr.py --mock scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png > /tmp/small-ocr-output.txt
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

### Aggregate-only OCR Privacy evidence runner
```bash
python scripts/ocr_eval/run_ocr_privacy_evidence.py \
  --image-path scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png \
  --ocr-runner-path scripts/ocr_eval/run_small_ocr.py \
  --privacy-runner-path scripts/privacy_filter/run_privacy_filter.py \
  --output /tmp/ocr-privacy-evidence.json \
  --mock
```

This runner composes the existing PP-OCRv5 mobile bounded printed-text OCR runner with the existing text-only Privacy Filter runner, validates their JSON with the canonical OCR handoff and Privacy Filter validators plus aggregate-only internal checks, and writes only an aggregate PHI-safe evidence artifact with safe OCR/Privacy Filter metadata, readiness, network status, detected-span counts, category counts, and explicit non-goals. For the checked-in synthetic printed PHI line, the report truthfully mirrors Privacy Filter output: 4 detected spans with `EMAIL`, `MRN`, `NAME`, and `PHONE` counts only. It suppresses child process and validator output from the terminal and emits only PHI-safe stdout with `artifact`, `report_written: true`, and a redacted `report_path`.

Non-goals: `browser_ui`, `desktop_ui`, `complete_ocr_pipeline`, `visual_redaction`, `image_pixel_redaction`, `handwriting_recognition`, and `final_pdf_rewrite_export`. This is CLI/runtime evidence only; it does not claim Browser/Desktop execution, visual/image pixel redaction, final PDF rewrite/export, handwriting recognition, or full OCR pipeline behavior.

### CLI aggregate-only OCR Privacy evidence wrapper
```bash
cargo run -p mdid-cli -- ocr-privacy-evidence \
  --image-path scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png \
  --runner-path scripts/ocr_eval/run_ocr_privacy_evidence.py \
  --output /tmp/ocr-privacy-evidence.json \
  --summary-output /tmp/ocr-privacy-evidence-summary.json \
  --python-command python3 \
  --mock
```

The `mdid-cli ocr-privacy-evidence` wrapper invokes the local aggregate-only OCR Privacy evidence runner, validates the bounded JSON report, writes the requested report, and emits only a PHI-safe CLI summary with report and summary paths redacted. The optional `--summary-output <summary.json>` writes a stricter `ocr_privacy_evidence_summary` artifact only after the primary aggregate report validates. The summary is aggregate-only, PHI-safe, and CLI/runtime evidence only: it omits raw OCR text, normalized text, masked text, spans/previews, fixture paths/filenames/IDs, local paths, raw PHI, image bytes, bbox data, and arbitrary runner payloads.

This wrapper is CLI/runtime evidence only. It is not Browser/Web execution, not Desktop execution, not OCR model-quality proof, not visual redaction, not image pixel redaction, not handwriting recognition, and not final PDF rewrite/export.

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
