# Privacy Filter Date Detection Implementation Plan

**Goal:** Improve bounded CLI/runtime text-only PII detection by adding deterministic DATE/DOB detection to the local Privacy Filter runner used by `mdid-cli privacy-filter-text`, without adding any agent/controller/orchestration semantics.

**Why now:** Dates of birth and service dates are common PHI in OCR handoff text and typed clinical notes. The existing bounded heuristic detects names, emails, phone numbers, MRNs, and IDs, but misses common DOB/date forms.

## Task 1: Add DATE/DOB heuristic detection to text-only Privacy Filter runner

- [x] RED: Add a CLI smoke test that pipes stdin containing `DOB 1978-04-23` and `seen on 04/23/1978`, then asserts the report masks both date values, emits `DATE` category count, and does not leak raw date/PHI in stdout/stderr/report.
- [x] GREEN: Add bounded deterministic DATE regex detection to `scripts/privacy_filter/run_privacy_filter.py`.
- [x] VERIFY: Run the targeted smoke test, broader `privacy_filter_text` CLI tests, generated-report privacy filter validator, and full `mdid-cli` smoke test suite.
- [x] COMMIT: Commit test, implementation, and plan only.

## Non-goals

- No network calls or remote API behavior.
- No OCR model quality claims.
- No browser/desktop UI work.
- No visual redaction, PDF rewrite, or image-pixel redaction.
