# Privacy Filter CLI gap closure batch (2026-05-01)

## Scope

CLI/runtime text-only Privacy Filter gap closure for five bounded categories:

- `INSURANCE_ID` (already started on this branch)
- `AGE`
- `FACILITY`
- `NPI`
- `LICENSE_PLATE`

This plan is limited to local CLI text processing and JSON contract validation. It is not Browser/Web execution, Desktop execution, OCR, visual redaction, image pixel redaction, handwriting recognition, final PDF rewrite/export, real OPF model-quality proof, or workflow orchestration.

## TDD checklist

- [x] Add Python RED tests in `scripts/privacy_filter/test_run_privacy_filter.py` for positive detection and false positives for the remaining bounded categories.
- [x] Implement bounded fallback regex/checksum logic in `scripts/privacy_filter/run_privacy_filter.py`.
- [x] Extend Python validator allowed labels in `scripts/privacy_filter/validate_privacy_filter_output.py`.
- [x] Extend Rust CLI Privacy Filter label allowlist in `crates/mdid-cli/src/main.rs`.
- [x] Add CLI smoke coverage in `crates/mdid-cli/tests/cli_smoke.rs` proving the new labels are accepted and stdout/stderr/report avoid raw PHI and path leaks.
- [x] Update README truth snapshot to reflect CLI `126/131 -> 131/136 = 96%`, Browser/Web 99%, Desktop 99%, Overall 97%.

## Detection bounds

- `AGE`: explicit age context only, numeric 0-120.
- `FACILITY`: explicit facility/hospital/clinic/site/location context with bounded capitalized synthetic facility names ending in facility-type suffixes.
- `NPI`: explicit NPI context only, 10 digits, Luhn-valid with the NPI `80840` prefix.
- `LICENSE_PLATE`: explicit license-plate/plate context only, bounded alphanumeric hyphenated synthetic plate values.

## Verification commands

- `python3 -m pytest scripts/privacy_filter/test_run_privacy_filter.py -q`
- `cargo test -p mdid-cli privacy_filter --test cli_smoke`
- `git diff --check`
- `cargo fmt --all -- --check`
