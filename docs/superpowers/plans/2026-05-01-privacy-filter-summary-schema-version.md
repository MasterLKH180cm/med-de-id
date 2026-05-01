# Privacy Filter Summary Schema Version Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a PHI-safe `schema_version` field to the bounded `privacy-filter-text --summary-output` artifact so downstream CLI/runtime/Browser/Desktop consumers can rely on a stable text-only Privacy Filter summary contract.

**Architecture:** Keep the change inside the existing CLI summary artifact path. The primary runner report remains unchanged; only the aggregate, PHI-safe summary artifact emitted by `mdid-cli privacy-filter-text --summary-output <summary.json>` gains `schema_version: 1`.

**Tech Stack:** Rust CLI crate (`crates/mdid-cli`), Cargo tests, existing Python Privacy Filter mock runner fixture.

---

## File Structure

- Modify `crates/mdid-cli/src/main.rs`
  - Add `schema_version: 1` to `privacy_filter_text_summary_artifact`.
  - Strengthen the existing summary-output test to assert the field.
- Modify `README.md`
  - Truth-sync the completion snapshot and verification evidence after tests pass.

### Task 1: Privacy Filter text summary schema version

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: existing `privacy_filter_text_writes_summary_output_without_phi` test in `crates/mdid-cli/src/main.rs`

- [ ] **Step 1: Write the failing test**

Add this assertion to the existing `privacy_filter_text_writes_summary_output_without_phi` test after the summary JSON is parsed:

```rust
assert_eq!(summary["schema_version"], 1);
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p mdid-cli privacy_filter_text_writes_summary_output_without_phi -- --nocapture`

Expected: FAIL because `summary["schema_version"]` is currently null, not `1`.

- [ ] **Step 3: Write minimal implementation**

Update `privacy_filter_text_summary_artifact` in `crates/mdid-cli/src/main.rs` so the JSON object starts with:

```rust
json!({
    "artifact": "privacy_filter_text_summary",
    "schema_version": 1,
    "scope": "text_only_single_report_summary",
```

Do not change the primary runner report shape and do not add any OCR, visual redaction, image pixel redaction, browser UI, desktop UI, controller, agent, or orchestration semantics.

- [ ] **Step 4: Run targeted test to verify it passes**

Run: `cargo test -p mdid-cli privacy_filter_text_writes_summary_output_without_phi -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Run relevant regression tests**

Run: `cargo test -p mdid-cli privacy_filter_text -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/mdid-cli/src/main.rs docs/superpowers/plans/2026-05-01-privacy-filter-summary-schema-version.md
git commit -m "feat(cli): version Privacy Filter text summary artifact"
```

### Task 2: README completion truth-sync

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update README evidence**

Update the current repository status to mention that `mdid-cli privacy-filter-text --summary-output <summary.json>` now emits `schema_version: 1` in its PHI-safe aggregate summary artifact.

Keep completion honest:
- CLI remains 95% unless controller-visible landed functionality justifies a rubric change.
- Browser/Web remains 99%.
- Desktop app remains 99%.
- Overall remains 97%.

- [ ] **Step 2: Verify README contains the schema-version evidence**

Run: `grep -n "schema_version" README.md`

Expected: at least one line documents `schema_version: 1` for the Privacy Filter text summary artifact.

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: truth-sync Privacy Filter summary schema version"
```
