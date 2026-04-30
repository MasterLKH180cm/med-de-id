# Privacy Filter Runtime Summary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded localhost runtime endpoint that converts existing text-only Privacy Filter JSON reports into PHI-safe summaries.

**Architecture:** The runtime does not execute OpenAI Privacy Filter and never accepts raw clinical text. It accepts an existing bounded JSON report from the CLI text-only POC and returns an allowlisted summary with safe counts and explicit non-goals, matching the existing Browser/Desktop summary semantics.

**Tech Stack:** Rust, axum, serde_json, mdid-runtime HTTP tests with tower::ServiceExt.

---

## File Structure

- Modify: `crates/mdid-runtime/src/http.rs` — add request/response structs, sanitizer helper, `/privacy-filter/summary` route, and invalid request response.
- Modify: `crates/mdid-runtime/tests/runtime_http.rs` — add endpoint tests proving safe summary output, PHI stripping, and invalid payload rejection.
- Modify: `README.md` — truth-sync completion/evidence after implementation.

## Tasks

### Task 1: Runtime Privacy Filter safe summary endpoint

**Files:**
- Modify: `crates/mdid-runtime/tests/runtime_http.rs`
- Modify: `crates/mdid-runtime/src/http.rs`

- [ ] **Step 1: Write failing test for safe summary output**

Append a tokio test in `crates/mdid-runtime/tests/runtime_http.rs` that POSTs to `/privacy-filter/summary` with a synthetic report containing `input_text`, `detected_spans` with raw text, category counts, `network_api_called`, `preview_policy`, and non-goals. Assert HTTP 200; assert the response includes only `artifact: privacy_filter_summary`, `mode`, safe `engine`, `network_api_called`, `preview_policy`, numeric `input_char_count`, `detected_span_count`, `category_counts`, and `non_goals`; assert the serialized response does not contain `Alice Smith`, `MRN-001`, or `input_text`.

- [ ] **Step 2: Verify RED**

Run: `cargo test -p mdid-runtime privacy_filter_summary -- --nocapture`
Expected: FAIL because `/privacy-filter/summary` is not registered yet.

- [ ] **Step 3: Implement minimal endpoint**

In `crates/mdid-runtime/src/http.rs`, add `PrivacyFilterSummaryRequest { report: serde_json::Value }`, `PrivacyFilterSummaryResponse`, route `.route("/privacy-filter/summary", post(privacy_filter_summary))`, helper functions that read only allowlisted report fields, reject non-object reports, reject negative/non-integer count fields, and never copy `input_text`, `normalized_text`, `detected_spans`, `text`, `value`, or `preview` into the response.

- [ ] **Step 4: Verify GREEN**

Run: `cargo test -p mdid-runtime privacy_filter_summary -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Add rejection test**

Add a second tokio test that sends `{"report":"not an object"}` and expects `400 Bad Request` with `error: invalid_privacy_filter_summary_request`.

- [ ] **Step 6: Verify targeted and broader runtime tests**

Run: `cargo test -p mdid-runtime privacy_filter_summary -- --nocapture`
Expected: PASS.
Run: `cargo test -p mdid-runtime --test runtime_http privacy_filter -- --nocapture`
Expected: PASS.

- [ ] **Step 7: Commit**

Run: `git add crates/mdid-runtime/src/http.rs crates/mdid-runtime/tests/runtime_http.rs && git commit -m "feat(runtime): add privacy filter summary endpoint"`.

### Task 2: README truth-sync

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update completion snapshot evidence**

Add a verification evidence paragraph for `/privacy-filter/summary`, explicitly stating it summarizes existing text-only Privacy Filter JSON reports only, does not execute Privacy Filter, does not accept raw clinical text as an OCR/visual redaction input, and strips PHI-bearing fields.

- [ ] **Step 2: Completion numbers**

Keep CLI at 95%, Browser/Web at 99%, Desktop app at 99%, and Overall at 97% unless repository-visible facts justify a conservative increase. State this round adds runtime hardening and does not add a new Browser/Desktop capability, so Browser/Desktop +5% is FAIL.

- [ ] **Step 3: Verify docs and commit**

Run: `git diff -- README.md` and `git add README.md docs/superpowers/plans/2026-05-01-privacy-filter-runtime-summary.md && git commit -m "docs: truth-sync privacy filter runtime summary"`.
