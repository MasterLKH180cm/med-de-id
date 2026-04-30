# Privacy Filter CLI Spike Results

Date: 2026-04-30
Branch: `feat/privacy-filter-cli-spike-cron-2117`

## Goal

Evaluate a bounded CLI-first text-only PII detection/masking contract for possible future OpenAI Privacy Filter integration in `med-de-id`.

## What landed in this spike

- Synthetic text fixture with fake patient-style identifiers.
- Bounded JSON output contract with summary counts, masked text, and redacted span previews.
- Local validator that reads a JSON file or stdin via `-`.
- Local runner that accepts a text file and emits the bounded contract.
- Deterministic fallback engine named `fallback_synthetic_patterns` for local, no-network verification.

## Verification evidence

Controller-visible commands passed on `feat/privacy-filter-cli-spike-cron-2117`:

```bash
python scripts/privacy_filter/run_privacy_filter.py scripts/privacy_filter/fixtures/sample_text_input.txt > /tmp/privacy-filter-output.json
python scripts/privacy_filter/validate_privacy_filter_output.py /tmp/privacy-filter-output.json
python scripts/privacy_filter/validate_privacy_filter_output.py scripts/privacy_filter/fixtures/sample_text_expected_shape.json
python -m py_compile scripts/privacy_filter/run_privacy_filter.py scripts/privacy_filter/validate_privacy_filter_output.py
```

The SDD spec review returned PASS. The SDD quality review returned APPROVED after fixing stdin validation support and removing import-time package probing side effects.

The direct shell pipeline form below is the intended stdin validation command, but this environment's security scanner may block `python | python` pipelines:

```bash
python scripts/privacy_filter/run_privacy_filter.py scripts/privacy_filter/fixtures/sample_text_input.txt | python scripts/privacy_filter/validate_privacy_filter_output.py -
```

A subprocess-equivalent stdin verification was run by the implementer/reviewer and passed.

## Output usefulness

The output contract is useful as a handoff shape for future text PII detection evaluation because it separates:

- aggregate counts in `summary`,
- safe masked content in `masked_text`, and
- span offsets/categories in `spans` without raw sensitive previews.

The fallback engine is not production detection quality. It only proves the CLI shape, masking contract, validator behavior, and local verification loop on synthetic text.

## Install/runtime friction

No external runtime install is required for the fallback path. The runner uses only Python standard library modules. In `--engine auto`, it does not import or execute a Privacy Filter package; it only preserves a safe future extension point and returns the deterministic fallback engine.

## Boundaries and non-goals

This is text-only PII detection/masking evaluation. It is not OCR, visual redaction, image pixel redaction, handwriting recognition, PDF rewrite/export, browser UI, desktop UI, or an end-to-end production medical de-identification pipeline.

## Verdict

More-Evidence.

The bounded CLI contract and verification loop are useful enough to justify a later local Privacy Filter package integration experiment if a local package can be installed without network calls at runtime. The current fallback does not justify raising Browser/Web or Desktop app completion, and it should not be described as OCR or visual redaction progress.
