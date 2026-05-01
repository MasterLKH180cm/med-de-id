# Privacy Filter URL Detection Implementation Plan

> **For implementation workers:** REQUIRED SUB-SKILL: Use subagent-driven-development (recommended) or executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add bounded URL detection to the CLI/runtime text-only Privacy Filter POC.

**Architecture:** Extend the deterministic fallback Privacy Filter runner with a narrow `URL` label, align Python and Rust validators, and add PHI/path-safe CLI smoke coverage. This remains text-only PII detection/masking evidence and does not claim OCR, visual redaction, Browser/Desktop execution, or PDF rewrite/export.

**Tech Stack:** Python 3 scripts under `scripts/privacy_filter/`, Rust `mdid-cli`, pytest/unittest, Cargo CLI smoke tests.

---

## File Structure

- Modify: `scripts/privacy_filter/run_privacy_filter.py` — add bounded `URL` regex, emit `URL` spans, include label in allowlist.
- Modify: `scripts/privacy_filter/validate_privacy_filter_output.py` — include `URL` in validator allowlist.
- Modify: `scripts/privacy_filter/test_run_privacy_filter.py` — add positive and negative URL detection tests.
- Modify: `crates/mdid-cli/src/main.rs` — allow `URL` in Rust Privacy Filter category validation.
- Modify: `crates/mdid-cli/tests/cli_smoke.rs` — add stdin smoke test proving URL masking, redacted previews, and no raw URL/PHI/path leaks.
- Modify: `README.md` — truth-sync CLI/runtime Privacy Filter URL evidence and completion arithmetic.

### Task 1: Python Privacy Filter URL detector and validator alignment

**Files:**
- Modify: `scripts/privacy_filter/run_privacy_filter.py`
- Modify: `scripts/privacy_filter/validate_privacy_filter_output.py`
- Modify: `scripts/privacy_filter/test_run_privacy_filter.py`

- [x] **Step 1: Write failing tests**

Added tests that prove bounded `http://`/`https://` URLs are detected as `URL`, full raw URLs are absent from `masked_text`, URL span previews are `<redacted>`, and invalid/non-http/embedded tokens are rejected.

- [x] **Step 2: Run tests to verify RED**

Run:

```bash
python -m pytest scripts/privacy_filter/test_run_privacy_filter.py -q
```

Observed RED during the quality-fix loop for bare URL and invalid long-TLD/no-dot cases.

- [x] **Step 3: Implement minimal detector and allowlists**

Added a bounded `URL_RE` that only accepts `http://` or `https://`, requires a dotted host and 2-8 letter TLD, supports optional path/query/fragment, rejects adjacent alphanumeric/underscore tokens, emits `URL` spans, and aligns both Python allowlists.

- [x] **Step 4: Run tests to verify GREEN**

Run:

```bash
python -m pytest scripts/privacy_filter/test_run_privacy_filter.py -q
python -m py_compile scripts/privacy_filter/run_privacy_filter.py scripts/privacy_filter/validate_privacy_filter_output.py scripts/privacy_filter/test_run_privacy_filter.py
```

Observed GREEN: `38 passed` and py_compile passed.

- [x] **Step 5: Commit**

```bash
git commit -m "feat(privacy-filter): detect bounded URL text spans"
git commit -m "fix(privacy-filter): bound full URL query masking"
git commit -m "fix(privacy-filter): tighten bounded URL detection"
```

### Task 2: Rust CLI Privacy Filter URL validation and smoke evidence

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`

- [x] **Step 1: Write failing CLI smoke test**

Added `cli_privacy_filter_text_masks_url_without_phi_or_path_leaks`, which runs `mdid-cli privacy-filter-text --stdin --mock` with the checked-in runner and asserts URL masking plus stdout/stderr/report PHI/path safety.

- [x] **Step 2: Run test to verify RED**

Run:

```bash
cargo test -p mdid-cli cli_privacy_filter_text_masks_url_without_phi_or_path_leaks --test cli_smoke -- --nocapture
```

Observed RED: Rust validation rejected `URL` as an invalid category label.

- [x] **Step 3: Implement minimal Rust allowlist change**

Added `URL` to `is_allowed_privacy_filter_label`.

- [x] **Step 4: Run targeted and broad Privacy Filter CLI tests**

Run:

```bash
cargo test -p mdid-cli cli_privacy_filter_text_masks_url_without_phi_or_path_leaks --test cli_smoke -- --nocapture
cargo test -p mdid-cli privacy_filter_text --test cli_smoke -- --nocapture
cargo fmt --check
git diff --check
```

Observed GREEN: targeted test passed, broad `privacy_filter_text` CLI smoke passed, `cargo fmt --check` passed, and `git diff --check` passed.

- [x] **Step 5: Commit**

```bash
git commit -m "test(cli): cover privacy filter URL masking"
```

### Task 3: README completion truth-sync and verification evidence

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/plans/2026-05-02-privacy-filter-url-detection.md`

- [x] **Step 1: Update README completion evidence**

README now describes `URL` as bounded CLI/runtime text-only Privacy Filter evidence and keeps Browser/Web and Desktop unchanged at the 99% target cap.

- [x] **Step 2: Truth-sync plan checkboxes**

This plan records completed steps with `[x]`.

- [x] **Step 3: Run verification**

Final controller-visible verification commands are recorded in the round report.

- [x] **Step 4: Commit**

Committed this plan and README truth-sync after controller verification with `docs: truth-sync privacy filter URL detection`.

## Self-Review

- Spec coverage: Python detector/validator/tests, Rust allowlist/smoke, README truth-sync, and non-goals are covered.
- Placeholder scan: no TBD/TODO/fill-in-later placeholders remain.
- Type/signature consistency: `URL` is the same label across Python runner, Python validator, Rust validator, tests, and docs.
