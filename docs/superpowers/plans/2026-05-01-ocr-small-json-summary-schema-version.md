# OCR Small JSON Summary Schema Version Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a PHI-safe `schema_version` field to the bounded `ocr-small-json --summary-output` artifact so downstream CLI/runtime/Browser/Desktop consumers can rely on a stable PP-OCRv5 mobile printed-text extraction summary contract.

**Architecture:** Keep the change inside the existing CLI summary artifact path. The primary OCR handoff JSON report remains unchanged and may still contain OCR text for downstream text-only Privacy Filter evaluation; only the aggregate PHI-safe summary artifact emitted by `mdid-cli ocr-small-json --summary-output <summary.json>` gains `schema_version: 1`.

**Tech Stack:** Rust CLI crate (`crates/mdid-cli`), Cargo tests, existing Python PP-OCRv5 mobile small-runner mock fixture.

---

## File Structure

- Modify `crates/mdid-cli/tests/cli_smoke.rs`
  - Strengthen `ocr_small_json_writes_phi_safe_summary_output` to require `schema_version` in the summary key allowlist and assert `schema_version == 1`.
- Modify `crates/mdid-cli/src/main.rs`
  - Add `schema_version: 1` to `build_ocr_small_json_summary`.
- Modify `README.md`
  - Truth-sync the current repository status and verification evidence after implementation/review passes.

### Task 1: OCR small JSON summary schema version

**Files:**
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/cli_smoke.rs`

- [ ] **Step 1: Write the failing test**

Update `ocr_small_json_writes_phi_safe_summary_output` in `crates/mdid-cli/tests/cli_smoke.rs` so the expected `summary_keys` set includes `"schema_version"`, and add this assertion after `assert_eq!(summary["artifact"], "ocr_small_json_summary");`:

```rust
assert_eq!(summary["schema_version"], 1);
```

The expected key list should be exactly:

```rust
[
    "artifact",
    "schema_version",
    "candidate",
    "engine",
    "engine_status",
    "scope",
    "privacy_filter_contract",
    "ready_for_text_pii_eval",
    "non_goals",
]
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p mdid-cli ocr_small_json_writes_phi_safe_summary_output -- --nocapture`

Expected: FAIL because the existing `ocr_small_json_summary` artifact does not include `schema_version` and `summary["schema_version"]` is null.

- [ ] **Step 3: Write minimal implementation**

Update `build_ocr_small_json_summary` in `crates/mdid-cli/src/main.rs` so the JSON object starts with:

```rust
json!({
    "artifact": "ocr_small_json_summary",
    "schema_version": 1,
    "candidate": "PP-OCRv5_mobile_rec",
```

Do not change the primary OCR handoff report shape, do not remove OCR text from the primary report, and do not add OCR execution to Browser/Web or Desktop. Do not add visual redaction, image pixel redaction, handwriting recognition, final PDF rewrite/export, controller, agent, claim, planner, or orchestration semantics.

- [ ] **Step 4: Run targeted test to verify it passes**

Run: `cargo test -p mdid-cli ocr_small_json_writes_phi_safe_summary_output -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Run relevant regression tests**

Run: `cargo test -p mdid-cli ocr_small_json -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs docs/superpowers/plans/2026-05-01-ocr-small-json-summary-schema-version.md
git commit -m "feat(cli): version OCR small summary artifact"
```

### Task 2: README completion truth-sync

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update README evidence**

Update the current repository status to mention that `mdid-cli ocr-small-json --summary-output <summary.json>` now emits `schema_version: 1` in its PHI-safe aggregate `ocr_small_json_summary` artifact for PP-OCRv5 mobile printed-text extraction readiness evidence.

Keep completion honest:
- CLI remains 95% unless controller-visible landed functionality justifies a rubric change.
- Browser/Web remains 99%.
- Desktop app remains 99%.
- Overall remains 97%.
- This is CLI/runtime OCR summary-contract hardening only; Browser/Web +5 and Desktop +5 are FAIL/not claimed because no new Browser/Web or Desktop capability lands.

- [ ] **Step 2: Verify README contains the schema-version evidence**

Run: `grep -n "ocr_small_json_summary\|schema_version" README.md`

Expected: at least one current-status line documents `schema_version: 1` for the OCR small JSON summary artifact.

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: truth-sync OCR small summary schema version"
```
