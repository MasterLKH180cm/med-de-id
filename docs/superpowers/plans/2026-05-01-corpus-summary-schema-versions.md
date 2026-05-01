# Corpus Summary Schema Versions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add explicit `schema_version: 1` to bounded aggregate-only corpus summary artifacts for Privacy Filter, OCR handoff, and OCR-to-Privacy-Filter CLI/runtime evidence.

**Architecture:** Preserve the existing primary report contracts and stdout shapes. Only the secondary aggregate summary artifacts get a stable schema marker, with tests updated first to lock exact key allowlists and README completion truth-sync updated using numerator/denominator accounting.

**Tech Stack:** Rust `mdid-cli`, serde_json, cargo tests, Markdown README truth-sync.

---

## File Structure

- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
  - Responsibility: CLI smoke coverage for summary-output contracts and PHI/path safety.
  - Add `schema_version` assertions to existing summary-output tests for:
    - `privacy_filter_corpus_writes_phi_safe_summary_output`
    - `ocr_handoff_corpus_writes_phi_safe_summary_output`
    - `ocr_to_privacy_filter_corpus_writes_phi_safe_summary_output`
- Modify: `crates/mdid-cli/src/main.rs`
  - Responsibility: CLI command parsing, wrapper execution, and report/summary JSON builders.
  - Add `"schema_version": 1` to:
    - `build_privacy_filter_corpus_summary`
    - `build_ocr_handoff_corpus_summary`
    - `build_ocr_to_privacy_filter_corpus_summary`
- Modify: `README.md`
  - Responsibility: repository-visible completion truth-sync and evidence log.
  - Add current-round evidence for corpus summary schema-version hardening, maintain CLI/Browser/Desktop/Overall arithmetic truthfully.

---

### Task 1: Add schema_version to Privacy Filter corpus summary

**Files:**
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
- Modify: `crates/mdid-cli/src/main.rs`

- [x] **Step 1: Write the failing test**

Edit the existing `privacy_filter_corpus_writes_phi_safe_summary_output` test in `crates/mdid-cli/tests/cli_smoke.rs` so the exact summary key allowlist includes `schema_version` and the body asserts `summary["schema_version"] == 1`.

Expected test assertion shape:

```rust
let summary_keys: BTreeSet<_> = summary.as_object().unwrap().keys().map(String::as_str).collect();
assert_eq!(
    summary_keys,
    BTreeSet::from([
        "artifact",
        "schema_version",
        "engine",
        "scope",
        "fixture_count",
        "total_detected_span_count",
        "category_counts",
        "non_goals",
    ])
);
assert_eq!(summary["schema_version"], json!(1));
```

- [x] **Step 2: Run the targeted test to verify RED**

Run:

```bash
cargo test -p mdid-cli privacy_filter_corpus_writes_phi_safe_summary_output -- --nocapture
```

Expected: FAIL because the summary JSON does not yet include `schema_version`.

- [x] **Step 3: Implement minimal code**

In `crates/mdid-cli/src/main.rs`, update `build_privacy_filter_corpus_summary`:

```rust
fn build_privacy_filter_corpus_summary(value: &Value) -> Value {
    json!({
        "artifact": "privacy_filter_corpus_summary",
        "schema_version": 1,
        "engine": value["engine"],
        "scope": value["scope"],
        "fixture_count": value["fixture_count"],
        "total_detected_span_count": value["total_detected_span_count"],
        "category_counts": value["category_counts"],
        "non_goals": value["non_goals"],
    })
}
```

- [x] **Step 4: Run the targeted test to verify GREEN**

Run:

```bash
cargo test -p mdid-cli privacy_filter_corpus_writes_phi_safe_summary_output -- --nocapture
```

Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add crates/mdid-cli/tests/cli_smoke.rs crates/mdid-cli/src/main.rs
git commit -m "feat(cli): version Privacy Filter corpus summary"
```

---

### Task 2: Add schema_version to OCR handoff corpus summary

**Files:**
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
- Modify: `crates/mdid-cli/src/main.rs`

- [x] **Step 1: Write the failing test**

Edit the existing `ocr_handoff_corpus_writes_phi_safe_summary_output` test in `crates/mdid-cli/tests/cli_smoke.rs` so the exact summary key allowlist includes `schema_version` and the body asserts `summary["schema_version"] == 1`.

Expected test assertion shape:

```rust
let summary_keys: BTreeSet<_> = summary.as_object().unwrap().keys().map(String::as_str).collect();
assert_eq!(
    summary_keys,
    BTreeSet::from([
        "artifact",
        "schema_version",
        "candidate",
        "engine",
        "scope",
        "privacy_filter_contract",
        "fixture_count",
        "ready_fixture_count",
        "all_fixtures_ready_for_text_pii_eval",
        "total_char_count",
        "non_goals",
    ])
);
assert_eq!(summary["schema_version"], json!(1));
```

- [x] **Step 2: Run the targeted test to verify RED**

Run:

```bash
cargo test -p mdid-cli ocr_handoff_corpus_writes_phi_safe_summary_output -- --nocapture
```

Expected: FAIL because the summary JSON does not yet include `schema_version`.

- [x] **Step 3: Implement minimal code**

In `crates/mdid-cli/src/main.rs`, update `build_ocr_handoff_corpus_summary`:

```rust
fn build_ocr_handoff_corpus_summary(value: &Value) -> Value {
    let fixture_count = value["fixture_count"].as_u64().unwrap_or(0);
    let ready_fixture_count = value["ready_fixture_count"].as_u64().unwrap_or(0);
    json!({
        "artifact": "ocr_handoff_corpus_readiness_summary",
        "schema_version": 1,
        "candidate": "PP-OCRv5_mobile_rec",
        "engine": "PP-OCRv5-mobile-bounded-spike",
        "scope": "printed_text_line_extraction_only",
        "privacy_filter_contract": "text_only_normalized_input",
        "fixture_count": fixture_count,
        "ready_fixture_count": ready_fixture_count,
        "all_fixtures_ready_for_text_pii_eval": fixture_count > 0 && fixture_count == ready_fixture_count,
        "total_char_count": value["total_char_count"],
        "non_goals": value["non_goals"],
    })
}
```

- [x] **Step 4: Run the targeted test to verify GREEN**

Run:

```bash
cargo test -p mdid-cli ocr_handoff_corpus_writes_phi_safe_summary_output -- --nocapture
```

Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add crates/mdid-cli/tests/cli_smoke.rs crates/mdid-cli/src/main.rs
git commit -m "feat(cli): version OCR handoff corpus summary"
```

---

### Task 3: Add schema_version to OCR-to-Privacy-Filter corpus summary

**Files:**
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
- Modify: `crates/mdid-cli/src/main.rs`

- [x] **Step 1: Write the failing test**

Edit the existing `ocr_to_privacy_filter_corpus_writes_phi_safe_summary_output` test in `crates/mdid-cli/tests/cli_smoke.rs` so the exact summary key allowlist includes `schema_version` and the body asserts `summary["schema_version"] == 1`.

Expected test assertion shape:

```rust
let summary_keys: BTreeSet<_> = summary.as_object().unwrap().keys().map(String::as_str).collect();
assert_eq!(
    summary_keys,
    BTreeSet::from([
        "artifact",
        "schema_version",
        "ocr_candidate",
        "ocr_engine",
        "ocr_scope",
        "privacy_filter_engine",
        "privacy_filter_contract",
        "privacy_scope",
        "fixture_count",
        "ready_fixture_count",
        "total_detected_span_count",
        "category_counts",
        "privacy_filter_category_counts",
        "network_api_called",
        "non_goals",
    ])
);
assert_eq!(summary["schema_version"], json!(1));
```

- [x] **Step 2: Run the targeted test to verify RED**

Run:

```bash
cargo test -p mdid-cli ocr_to_privacy_filter_corpus_writes_phi_safe_summary_output -- --nocapture
```

Expected: FAIL because the summary JSON does not yet include `schema_version`.

- [x] **Step 3: Implement minimal code**

In `crates/mdid-cli/src/main.rs`, update `build_ocr_to_privacy_filter_corpus_summary`:

```rust
fn build_ocr_to_privacy_filter_corpus_summary(value: &Value) -> Value {
    json!({
        "artifact": "ocr_to_privacy_filter_corpus_summary",
        "schema_version": 1,
        "ocr_candidate": value["ocr_candidate"],
        "ocr_engine": value["ocr_engine"],
        "ocr_scope": value["ocr_scope"],
        "privacy_filter_engine": value["privacy_filter_engine"],
        "privacy_filter_contract": value["privacy_filter_contract"],
        "privacy_scope": value["privacy_scope"],
        "fixture_count": value["fixture_count"],
        "ready_fixture_count": value["ready_fixture_count"],
        "total_detected_span_count": value["total_detected_span_count"],
        "category_counts": value["category_counts"],
        "privacy_filter_category_counts": value["privacy_filter_category_counts"],
        "network_api_called": value["network_api_called"],
        "non_goals": value["non_goals"],
    })
}
```

- [x] **Step 4: Run the targeted test to verify GREEN**

Run:

```bash
cargo test -p mdid-cli ocr_to_privacy_filter_corpus_writes_phi_safe_summary_output -- --nocapture
```

Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add crates/mdid-cli/tests/cli_smoke.rs crates/mdid-cli/src/main.rs
git commit -m "feat(cli): version OCR Privacy corpus summary"
```

---

### Task 4: README completion truth-sync and final verification

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/plans/2026-05-01-corpus-summary-schema-versions.md`

- [x] **Step 1: Update README completion snapshot**

Edit `README.md` current status so it says this round adds `schema_version: 1` to three aggregate-only corpus summaries:

- `privacy_filter_corpus_summary`
- `ocr_handoff_corpus_readiness_summary`
- `ocr_to_privacy_filter_corpus_summary`

Use fraction accounting:

- CLI old: `114/119 = 95%` floor
- Add and complete three CLI/runtime summary-contract requirements: `117/122 = 95%` floor
- Browser/Web remains `99%`
- Desktop app remains `99%`
- Overall remains `97%`

Include explicit Browser/Web +5% and Desktop +5% FAIL/not claimed because this is CLI/runtime-only corpus summary-contract hardening.

- [x] **Step 2: Mark this plan complete**

Change every task checkbox in this plan from `- [ ]` to `- [x]` after implementation and verification have actually completed.

- [x] **Step 3: Run final verification**

Run:

```bash
cargo test -p mdid-cli privacy_filter_corpus_writes_phi_safe_summary_output -- --nocapture
cargo test -p mdid-cli ocr_handoff_corpus_writes_phi_safe_summary_output -- --nocapture
cargo test -p mdid-cli ocr_to_privacy_filter_corpus_writes_phi_safe_summary_output -- --nocapture
cargo test -p mdid-cli corpus -- --nocapture
cargo fmt --check
git diff --check
```

Expected: all PASS, fmt clean, diff check clean.

- [x] **Step 4: Commit docs and plan truth-sync**

```bash
git add README.md docs/superpowers/plans/2026-05-01-corpus-summary-schema-versions.md
git commit -m "docs: truth-sync corpus summary schema versions"
```
