# Privacy Filter bounded spike

## Purpose
This directory is only for a bounded local text-only PII detection/masking spike.

## Non-goals
- not OCR
- not visual redaction
- not image/pixel redaction
- not final PDF rewrite/export
- not browser or desktop UI
- not production Privacy Filter integration

## Bootstrap
Normal invocation intentionally uses the deterministic local `fallback_synthetic_patterns` engine so contract verification remains offline and reproducible. A locally installed `opf` command is never auto-used. The runner only invokes upstream OpenAI Privacy Filter tooling (`opf`) with explicit `--use-opf`, and then normalizes its JSON into this repo's bounded text-only contract.

When explicit --use-opf is selected, PHI-bearing input text is sent to the local `opf` subprocess via stdin only, not by command-line argument or temporary input file. Both canonical span output and alternate entities-style output are normalized into the bounded text-only contract used here: `summary`, `masked_text`, redacted `spans`, and metadata with `network_api_called: false`. Span previews are redacted previews only and must not expose raw PHI.

The fallback is a synthetic plumbing/evaluation aid only. It proves output shape and downstream wiring, not real model quality. The OPF path is still a bounded CLI/runtime Privacy Filter POC and is not OCR, not visual redaction, not image/pixel redaction, not final PDF rewrite/export, not browser integration, and not desktop integration.

All successful single-text runner outputs must include:
- `summary.input_char_count`
- `summary.detected_span_count`
- `summary.category_counts`
- `masked_text`
- `spans[]` entries with `label`, `start`, `end`, and safe `preview`
- `metadata.engine`
- `metadata.network_api_called: false`
- `metadata.preview_policy`

## Commands
### Default local run with deterministic no-network fallback when `opf` is unavailable
```bash
python scripts/privacy_filter/run_privacy_filter.py scripts/privacy_filter/fixtures/sample_text_input.txt > /tmp/privacy-filter-output.json
python scripts/privacy_filter/validate_privacy_filter_output.py /tmp/privacy-filter-output.json
```

### Validate the checked-in expected contract fixture
```bash
python scripts/privacy_filter/validate_privacy_filter_output.py scripts/privacy_filter/fixtures/sample_text_expected_shape.json
```

### Force fallback/plumbing mode explicitly
```bash
python scripts/privacy_filter/run_privacy_filter.py --mock scripts/privacy_filter/fixtures/sample_text_input.txt > /tmp/privacy-filter-output-mock.json
python scripts/privacy_filter/validate_privacy_filter_output.py /tmp/privacy-filter-output-mock.json
```

### Exercise the Rust CLI wrapper against the checked-in Python runner
```bash
cargo run -p mdid-cli -- privacy-filter-text \
  --input-path scripts/privacy_filter/fixtures/sample_text_input.txt \
  --runner-path scripts/privacy_filter/run_privacy_filter.py \
  --report-path /tmp/mdid-privacy-filter-wrapper-output.json
python scripts/privacy_filter/validate_privacy_filter_output.py /tmp/mdid-privacy-filter-wrapper-output.json
```

### Run the synthetic corpus aggregate evidence
```bash
python scripts/privacy_filter/run_synthetic_corpus.py --fixture-dir scripts/privacy_filter/fixtures/corpus --output /tmp/privacy-filter-corpus.json
```

`run_synthetic_corpus.py` is synthetic text-only PII detection/masking evidence for the bounded Privacy Filter spike. It runs only checked-in synthetic corpus fixtures through the local text-only runner and writes a PHI-safe aggregate report.

The PHI-safe aggregate report must contain only counts, category coverage, fixture names, engine/scope metadata, and explicit non-goals. It must not include raw fixture text, `masked_text`, spans, raw previews, or any per-detection text payload.

Corpus-runner non-goals:
- not OCR
- not visual redaction
- not image/pixel redaction
- not final PDF rewrite/export
- not browser/desktop UI

Use of fallback, `--mock`, or the synthetic corpus proves only the output contract/pipeline shape, not real model quality.
