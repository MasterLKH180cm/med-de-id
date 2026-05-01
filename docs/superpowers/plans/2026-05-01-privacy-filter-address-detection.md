# Privacy Filter Address Detection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use subagent-driven-development (recommended) or executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Improve bounded CLI/runtime text-only PII detection by adding deterministic street-address detection to the local Privacy Filter runner used by `mdid-cli privacy-filter-text`.

**Architecture:** Extend the existing deterministic local Privacy Filter fallback runner with one bounded street-address span pattern and keep Rust CLI validation/summary allowlists aligned with the runner contract. This remains CLI/runtime text-only PII detection evidence only; it does not add OCR, visual redaction, image pixel redaction, Browser/Web execution, Desktop execution, final PDF rewrite/export, or workflow orchestration semantics.

**Tech Stack:** Python Privacy Filter fallback runner, Rust `mdid-cli`, Cargo CLI smoke tests, JSON contract validator.

---

**Why now:** Street addresses are common patient identifiers in clinical notes and OCR normalized text. The current bounded synthetic Privacy Filter POC detects names, MRNs, emails, phones, IDs, and dates, but still misses common address forms, which weakens the CLI-first text PII detection POC.

## File Structure

- Modify `scripts/privacy_filter/run_privacy_filter.py`: add a bounded `ADDRESS_RE` and emit `ADDRESS` spans from `heuristic_detect`.
- Modify `crates/mdid-cli/src/main.rs`: allow `ADDRESS` in Privacy Filter category-count validation and PHI-safe summary extraction where current text-only Privacy Filter categories are allowlisted.
- Modify `crates/mdid-cli/tests/cli_smoke.rs`: add a stdin smoke test proving address detection/masking and no raw-address leaks in stdout/stderr/report previews.
- Modify `README.md`: truth-sync completion evidence and rubric arithmetic after the verified landed slice.

## Task 1: Add ADDRESS heuristic detection to text-only Privacy Filter runner and CLI contract

**Files:**
- Modify: `scripts/privacy_filter/run_privacy_filter.py`
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/cli_smoke.rs`
- Modify: `README.md`

- [ ] **Step 1: Write the failing CLI smoke test**

Add a test named `privacy_filter_text_detects_addresses_from_stdin_without_raw_address_leaks` in `crates/mdid-cli/tests/cli_smoke.rs`. The test must pipe this exact synthetic text through `mdid-cli privacy-filter-text --stdin`:

```text
Patient Jane Example lives at 123 Main St and follow-up mail goes to 456 Oak Avenue.
```

The test must assert:

```rust
assert!(stdout.status.success());
assert!(!stdout_text.contains("123 Main St"));
assert!(!stdout_text.contains("456 Oak Avenue"));
assert!(!stderr_text.contains("123 Main St"));
assert!(!stderr_text.contains("456 Oak Avenue"));
assert!(!report_text.contains("123 Main St"));
assert!(!report_text.contains("456 Oak Avenue"));
assert_eq!(report["summary"]["category_counts"]["ADDRESS"], 2);
assert!(report["masked_text"].as_str().unwrap().contains("[ADDRESS]"));
assert!(report["spans"].as_array().unwrap().iter().all(|span| span["preview"] == "<redacted>"));
```

- [ ] **Step 2: Run the failing test and verify RED**

Run:

```bash
cargo test -p mdid-cli privacy_filter_text_detects_addresses_from_stdin_without_raw_address_leaks --test cli_smoke -- --nocapture
```

Expected: FAIL because the current runner does not emit `ADDRESS` and/or CLI validation rejects the new category.

- [ ] **Step 3: Implement bounded ADDRESS detection and CLI allowlist alignment**

In `scripts/privacy_filter/run_privacy_filter.py`, add a regex that detects common synthetic street-address forms with a street number, one to four capitalized street words, and a suffix from `St`, `Street`, `Ave`, `Avenue`, `Rd`, `Road`, `Blvd`, `Boulevard`, `Dr`, `Drive`, `Ln`, `Lane`, `Ct`, or `Court`. Emit spans with label `ADDRESS` in `heuristic_detect` after phone/date detection and before MRN/ID detection.

In `crates/mdid-cli/src/main.rs`, update current text-only Privacy Filter category allowlists so `ADDRESS` counts are accepted and preserved in PHI-safe summaries. Do not add unbounded category passthrough.

- [ ] **Step 4: Run targeted and broader verification**

Run:

```bash
cargo test -p mdid-cli privacy_filter_text_detects_addresses_from_stdin_without_raw_address_leaks --test cli_smoke -- --nocapture
cargo test -p mdid-cli privacy_filter_text --test cli_smoke -- --nocapture
cargo test -p mdid-cli --test cli_smoke
python3 scripts/privacy_filter/validate_privacy_filter_output.py <generated-address-report-path>
git diff --check
```

Expected: all commands PASS. The generated report path may be the test temp file if retained or a new `/tmp/privacy-filter-address-output.json` generated with the same stdin text.

- [ ] **Step 5: Commit**

```bash
git add scripts/privacy_filter/run_privacy_filter.py crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs README.md docs/superpowers/plans/2026-05-01-privacy-filter-address-detection.md
git commit -m "feat(cli): detect address pii in privacy filter"
```

## Non-goals

- No network calls or remote API behavior.
- No OCR model-quality claim.
- No Browser/Web or Desktop execution.
- No visual redaction, image pixel redaction, handwriting recognition, final PDF rewrite/export, or workflow orchestration semantics.

## Self-Review

- Spec coverage: the task covers test-first address detection, runner implementation, CLI validation/summary alignment, verification, README truth-sync, and commit.
- Placeholder scan: no TBD/TODO/implement-later placeholders are present.
- Type consistency: the single new category label is consistently `ADDRESS` across tests, runner output, validation, summaries, and README evidence.
