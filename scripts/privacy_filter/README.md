# Privacy Filter CLI Spike

This directory contains a bounded CLI-first text PII detection/masking spike for evaluating OpenAI Privacy Filter as a future candidate in `med-de-id`.

## Scope

This spike is intentionally narrow:

- It accepts UTF-8 text input only.
- It emits a PHI-safe JSON contract with redacted previews.
- It uses synthetic fixtures in this repository for verification.
- It does not call network APIs.
- It currently uses a deterministic local fallback engine named `fallback_synthetic_patterns` so the output contract can be verified even when a real local Privacy Filter package is not installed.

## Non-goals

This spike is not OCR, visual redaction, image pixel redaction, handwriting recognition, PDF rewrite/export, browser UI, desktop UI, or a production de-identification engine.

## Commands

Run the synthetic fixture through the bounded runner:

```bash
python scripts/privacy_filter/run_privacy_filter.py scripts/privacy_filter/fixtures/sample_text_input.txt > /tmp/privacy-filter-output.json
```

Validate the generated contract:

```bash
python scripts/privacy_filter/validate_privacy_filter_output.py /tmp/privacy-filter-output.json
```

Validate the checked-in expected contract shape:

```bash
python scripts/privacy_filter/validate_privacy_filter_output.py scripts/privacy_filter/fixtures/sample_text_expected_shape.json
```

Validate via stdin mode:

```bash
python scripts/privacy_filter/run_privacy_filter.py scripts/privacy_filter/fixtures/sample_text_input.txt | python scripts/privacy_filter/validate_privacy_filter_output.py -
```

If the local execution environment blocks shell pipes into Python, use an equivalent subprocess pipeline and pass the runner stdout to validator stdin.

## Output contract

The JSON output contains:

- `summary.input_char_count`
- `summary.detected_span_count`
- `summary.category_counts`
- `masked_text`
- `spans[]` entries with `label`, `start`, `end`, and `preview`
- `metadata.engine`, currently `fallback_synthetic_patterns`
- `metadata.network_api_called`, always `false` in this spike

Offsets use start-inclusive/end-exclusive semantics. Span previews must be bracketed redacted labels such as `[NAME]`, not raw sensitive text.
