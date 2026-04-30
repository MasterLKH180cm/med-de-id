# Privacy Filter bounded spike

## Purpose
This directory is only for a bounded local text-only PII detection/masking spike.

## Non-goals
- not OCR
- not visual redaction
- not image/pixel redaction
- not final PDF rewrite/export

## Bootstrap
Real model path requires upstream OpenAI Privacy Filter tooling (`opf`) to be installed locally.
This repo also supports a bounded plumbing-only mode via `--mock` for contract verification without claiming the real model ran.

## Commands
### RED (real model missing expected)
```bash
python scripts/privacy_filter/run_privacy_filter.py scripts/privacy_filter/fixtures/sample_text_input.txt
```

### GREEN (contract plumbing only)
```bash
python scripts/privacy_filter/run_privacy_filter.py --mock scripts/privacy_filter/fixtures/sample_text_input.txt > /tmp/privacy-filter-output.json
python scripts/privacy_filter/validate_privacy_filter_output.py /tmp/privacy-filter-output.json
```

### Optional variation
```bash
python scripts/privacy_filter/run_privacy_filter.py --mock scripts/privacy_filter/fixtures/sample_text_input.txt
```

Use of `--mock` proves only the output contract/pipeline shape, not real model quality.
