# Browser Vault Audit Pagination Status Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a PHI-safe browser-side vault audit pagination status helper so operators can see the requested page window and whether another page is likely available after a runtime response.

**Architecture:** Keep the feature inside `mdid-browser` state helpers, derived from existing vault audit request fields and parsed response summaries. Do not add controller/agent/workflow semantics; this is a bounded local browser UX improvement over the already-landed read-only audit endpoint.

**Tech Stack:** Rust, Leptos component state helpers, serde_json, cargo tests.

---

## File Structure

- Modify: `crates/mdid-browser/src/app.rs`
  - Add small pure helper(s) near existing vault audit request parsing.
  - Add `BrowserAppState::vault_audit_pagination_status()` for UI/test use.
  - Render the status text in the vault audit form/response area without exposing PHI values.
  - Add unit tests in the existing `#[cfg(test)]` module.
- Modify: `README.md`
  - Truth-sync completion snapshot and verification evidence after SDD review and controller-visible tests.

### Task 1: Browser vault audit pagination status helper

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`
- Test: existing unit tests in `crates/mdid-browser/src/app.rs`

- [ ] **Step 1: Write the failing test**

Add tests in the existing test module:

```rust
#[test]
fn vault_audit_pagination_status_reports_requested_window_and_next_page() {
    let mut state = BrowserAppState::default();
    state.input_mode = InputMode::VaultAuditEvents;
    state.vault_audit_limit = "25".to_string();
    state.vault_audit_offset = "50".to_string();
    state.summary = r#"{"event_count":25,"next_offset":75}"#.to_string();

    assert_eq!(
        state.vault_audit_pagination_status(),
        Some("Showing audit events 51-75. More events may be available from offset 75.".to_string())
    );
}

#[test]
fn vault_audit_pagination_status_omits_next_page_when_response_has_no_next_offset() {
    let mut state = BrowserAppState::default();
    state.input_mode = InputMode::VaultAuditEvents;
    state.vault_audit_limit = "10".to_string();
    state.vault_audit_offset = String::new();
    state.summary = r#"{"event_count":3}"#.to_string();

    assert_eq!(
        state.vault_audit_pagination_status(),
        Some("Showing audit events 1-3. No next audit page was reported.".to_string())
    );
}

#[test]
fn vault_audit_pagination_status_is_absent_outside_vault_audit_mode() {
    let mut state = BrowserAppState::default();
    state.input_mode = InputMode::CsvText;
    state.vault_audit_limit = "25".to_string();
    state.vault_audit_offset = "50".to_string();
    state.summary = r#"{"event_count":25,"next_offset":75}"#.to_string();

    assert_eq!(state.vault_audit_pagination_status(), None);
}
```

- [ ] **Step 2: Run tests to verify RED**

Run: `cargo test -p mdid-browser vault_audit_pagination_status -- --nocapture`

Expected: FAIL because `vault_audit_pagination_status` does not exist yet.

- [ ] **Step 3: Implement minimal code**

Add helper code that:
- returns `None` unless `input_mode == InputMode::VaultAuditEvents`
- parses `vault_audit_offset` as zero when blank, otherwise non-negative integer
- parses `event_count` and optional `next_offset` from `summary` JSON
- returns PHI-safe text only, never echoing vault path, actor, kind, passphrase, or event data
- renders the status near the vault audit controls/response area using existing Leptos conditional rendering patterns

- [ ] **Step 4: Run focused tests to verify GREEN**

Run: `cargo test -p mdid-browser vault_audit_pagination_status -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Run broader verification**

Run:

```bash
cargo test -p mdid-browser --lib
cargo fmt --check
git diff --check
```

Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add crates/mdid-browser/src/app.rs
git commit -m "feat(browser): show vault audit pagination status"
```

### Task 2: README truth-sync for browser vault audit pagination status

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Read current completion snapshot**

Run: `grep -n "Completion snapshot\|Browser/web\|Overall\|Verification evidence" README.md | head -40`

Expected: current browser/web completion and latest verification evidence are visible.

- [ ] **Step 2: Update README completion evidence**

Update README to state that browser/web now includes a PHI-safe vault audit pagination status helper for requested windows and next-offset guidance. Keep percentages truthful: browser/web may increase by at most one point only if the feature is landed and verified; overall likely remains unchanged unless the repo has crossed a major missing-capability threshold.

- [ ] **Step 3: Verify README text**

Run: `git diff -- README.md`

Expected: README mentions the exact branch/commit and controller-visible verification commands without overclaiming OCR, visual redaction, PDF/media rewrite, vault browsing, auth/session, or agent/controller semantics.

- [ ] **Step 4: Commit**

```bash
git add README.md
git commit -m "docs: truth-sync browser audit pagination status"
```
