# Privacy Filter Date Detection Implementation Plan

> **For implementation workers:** REQUIRED SUB-SKILL: Use subagent-driven-development (recommended) or executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Improve bounded CLI/runtime text-only PII detection by adding deterministic DATE/DOB detection to the local Privacy Filter runner used by `mdid-cli privacy-filter-text`.

**Architecture:** Extend the existing deterministic local Privacy Filter fallback runner with one bounded DATE span pattern, then keep the Rust CLI wrapper's PHI-safe summary/category allowlist aligned with the runner contract. The slice remains CLI/runtime text-only PII detection evidence; it does not add OCR, visual redaction, Browser/Web execution, Desktop execution, final PDF rewrite/export, or unrelated workflow coordination semantics.

**Tech Stack:** Python Privacy Filter fallback runner, Rust `mdid-cli`, Cargo CLI smoke tests, JSON contract validator.

---

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
