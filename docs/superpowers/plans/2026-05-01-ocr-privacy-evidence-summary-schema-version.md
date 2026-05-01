# OCR Privacy Evidence Summary Schema Version Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `schema_version: 1` to the aggregate-only `mdid-cli ocr-privacy-evidence --summary-output` artifact so downstream CLI/runtime consumers can rely on a stable bounded OCR-to-text-PII evidence summary contract.

**Architecture:** Keep the primary `ocr_privacy_evidence` report unchanged because it may contain aggregate evidence fields already validated by the CLI wrapper. Add the schema version only to the secondary PHI-safe `ocr_privacy_evidence_summary` artifact after the primary report validates, preserving the same strict allowlist and no Browser/Web or Desktop execution claims.

**Tech Stack:** Rust `mdid-cli`, `serde_json`, Rust smoke tests in `crates/mdid-cli/tests/cli_smoke.rs`, repository README truth-sync.

---

## File Structure

- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
  - Strengthen the existing `ocr_privacy_evidence_writes_phi_safe_summary_output` smoke test to require `schema_version: 1` and update the exact summary key allowlist.
- Modify: `crates/mdid-cli/src/main.rs`
  - Add `schema_version: 1` to `build_ocr_privacy_evidence_summary()` only; do not change primary report or stdout shapes.
- Modify: `README.md`
  - Truth-sync the current completion snapshot and evidence paragraph for this CLI/runtime-only contract-stability requirement with conservative fraction accounting.

### Task 1: Version the OCR Privacy Evidence summary artifact

**Files:**
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
- Modify: `crates/mdid-cli/src/main.rs`

- [x] **Step 1: Write the failing smoke test**

In `crates/mdid-cli/tests/cli_smoke.rs`, update the existing `ocr_privacy_evidence_writes_phi_safe_summary_output` assertions so the summary allowlist includes `schema_version` and requires value `1`:

```rust
    let summary_keys = summary.as_object().unwrap().keys().cloned().collect::<Vec<_>>();
    assert_eq!(
        summary_keys,
        vec![
            "artifact".to_string(),
            "category_counts".to_string(),
            "network_api_called".to_string(),
            "non_goals".to_string(),
            "ocr_scope".to_string(),
            "privacy_filter_contract".to_string(),
            "privacy_scope".to_string(),
            "ready_for_text_pii_eval".to_string(),
            "schema_version".to_string(),
            "total_detected_span_count".to_string(),
        ]
    );
    assert_eq!(summary["schema_version"], 1);
```

- [x] **Step 2: Run the targeted test to verify RED**

Run:

```bash
cargo test -p mdid-cli ocr_privacy_evidence_writes_phi_safe_summary_output -- --nocapture
```

Expected: FAIL because the current `ocr_privacy_evidence_summary` artifact does not include `schema_version`.

- [x] **Step 3: Implement minimal GREEN code**

In `crates/mdid-cli/src/main.rs`, update only `build_ocr_privacy_evidence_summary()` to include the fixed schema version:

```rust
    let summary = json!({
        "artifact": "ocr_privacy_evidence_summary",
        "schema_version": 1,
        "ocr_scope": report["ocr_scope"],
        "privacy_scope": report["privacy_scope"],
        "privacy_filter_contract": report["privacy_filter_contract"],
        "network_api_called": false,
        "ready_for_text_pii_eval": report["ready_for_text_pii_eval"],
        "total_detected_span_count": report["detected_span_count"],
        "category_counts": Value::Object(category_counts),
        "non_goals": [
            "browser_ui",
            "desktop_ui",
            "visual_redaction",
            "image_pixel_redaction",
            "handwriting_recognition",
            "final_pdf_rewrite_export"
        ]
    });
```

- [x] **Step 4: Run the targeted test to verify GREEN**

Run:

```bash
cargo test -p mdid-cli ocr_privacy_evidence_writes_phi_safe_summary_output -- --nocapture
```

Expected: PASS.

- [x] **Step 5: Run focused regression for the command family**

Run:

```bash
cargo test -p mdid-cli ocr_privacy_evidence -- --nocapture
```

Expected: PASS for the OCR Privacy Evidence command tests, including summary output, path alias rejection, help discoverability, invalid report cleanup, and checked-in fixture smoke coverage.

- [x] **Step 6: Commit the code/test change**

```bash
git add crates/mdid-cli/tests/cli_smoke.rs crates/mdid-cli/src/main.rs docs/superpowers/plans/2026-05-01-ocr-privacy-evidence-summary-schema-version.md
git commit -m "feat(cli): version OCR privacy evidence summary"
```

### Task 2: Truth-sync README completion and evidence

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/plans/2026-05-01-ocr-privacy-evidence-summary-schema-version.md`

- [ ] **Step 1: Update README current snapshot**

Update `README.md` current status text so the top snapshot says this round adds `schema_version: 1` to `ocr_privacy_evidence_summary`. Use this completion arithmetic:

```text
CLI 113/118 -> 114/119 = 95% floor
Browser/Web remains 99%
Desktop app remains 99%
Overall remains 97%
```

The README must say this is CLI/runtime summary-contract hardening only and not Browser/Web execution, Desktop execution, OCR model-quality proof, visual redaction, image pixel redaction, handwriting recognition, or final PDF rewrite/export.

- [ ] **Step 2: Mark plan checkboxes complete**

In this plan file, change every completed checkbox from `- [ ]` to `- [x]` after the matching code/test/docs step is actually done.

- [ ] **Step 3: Run formatting and verification**

Run:

```bash
cargo fmt --check
cargo test -p mdid-cli ocr_privacy_evidence -- --nocapture
git diff --check
```

Expected: all PASS / no output from `git diff --check`.

- [ ] **Step 4: Commit README truth-sync**

```bash
git add README.md docs/superpowers/plans/2026-05-01-ocr-privacy-evidence-summary-schema-version.md
git commit -m "docs: truth-sync OCR privacy evidence summary schema version"
```

### Task 3: Final integration review and branch publication

**Files:**
- Read/verify: `crates/mdid-cli/src/main.rs`
- Read/verify: `crates/mdid-cli/tests/cli_smoke.rs`
- Read/verify: `README.md`
- Read/verify: `docs/superpowers/plans/2026-05-01-ocr-privacy-evidence-summary-schema-version.md`

- [ ] **Step 1: Run final verification**

Run:

```bash
cargo fmt --check
cargo test -p mdid-cli ocr_privacy_evidence -- --nocapture
git diff --check
git status --short
```

Expected: formatting passes, focused tests pass, whitespace check passes, and the worktree is clean after commits.

- [ ] **Step 2: Push branch**

```bash
git push -u origin feat/ocr-privacy-evidence-summary-1840
```

Expected: remote branch is updated and ready for PR/merge review.

- [ ] **Step 3: Final SDD integration review**

Review the landed branch against this plan and confirm:

```text
Spec compliance: PASS only if summary artifact has schema_version: 1, primary report/stdout shapes are unchanged, README arithmetic is truthful, and no Browser/Desktop capability is claimed.
Quality: APPROVED only if tests cover the summary key allowlist/value, PHI/path leak assertions remain intact, docs are internally consistent, and no workflow orchestration/product scope drift was introduced.
```
