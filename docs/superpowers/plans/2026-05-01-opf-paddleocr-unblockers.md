# OPF + PaddleOCR Runtime-Path Unblockers Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or equivalent SDD review. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the recurring release blocker where `opf` and PaddleOCR/PP-OCRv5 are reported as unverified solely because the current environment lacks those external dependencies. This slice does not prove real OPF model quality or real PP-OCRv5 model quality. It proves deterministic local executable/adapter runtime paths and keeps CLI/runtime contracts PHI-safe.

**Current discovery:**

- `opf` command: not installed in this environment.
- Python packages: `openai_privacy_filter`, `opf`, `paddleocr`, `paddlepaddle`, and `paddle` are not importable in this environment.
- `paddleocr` exists on PyPI, but installing the full stack during cron is heavy and not required to unblock the repo contract.

## Completed tasks

- [x] Add Python runner test proving `run_small_ocr.py --mock --json` redacts PHI-bearing input filenames.
- [x] Change `build_extraction_contract()` to emit `source: "<redacted>"`.
- [x] Add Rust CLI smoke coverage proving `ocr-small-json` artifacts/stdout/stderr do not leak PHI-bearing image filenames.
- [x] Add a subprocess-level OPF test using `run_privacy_filter.py --stdin --use-opf` with a fake `opf` executable on `PATH`.
- [x] Assert PHI is passed via stdin only, stdout is normalized to the repo Privacy Filter contract, previews are redacted, and stderr does not leak raw OPF diagnostics.
- [x] Add a subprocess-level PaddleOCR-like adapter test using a fake local `paddleocr.py` on `PYTHONPATH`, non-mock `run_small_ocr.py --json`, and `engine_status: local_paddleocr_execution`.
- [x] Truth-sync README: actual external model quality remains unverified; Browser/Web and Desktop completion do not increase.

## Verification plan

- [x] `python3 -m pytest scripts/privacy_filter/test_run_privacy_filter.py::PrivacyFilterRunnerFailureTests::test_explicit_opf_subprocess_path_is_verified_with_local_fake_binary -q`
- [x] `python3 -m pytest tests/test_ocr_runner_contract.py::test_subprocess_local_paddleocr_adapter_path_emits_json_without_mock_or_source_leak -q`
- [x] `python3 -m pytest tests/test_ocr_runner_contract.py scripts/privacy_filter/test_run_privacy_filter.py -q`
- [x] `cargo test -p mdid-cli ocr_small_json -- --nocapture`
- [x] `cargo test -p mdid-cli offline_readiness -- --nocapture`
- [ ] SDD spec compliance review
- [ ] SDD code quality review
- [ ] Full release gates before promotion
