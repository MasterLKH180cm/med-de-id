# Audit Event Export Surface Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add explicit bounded audit-events JSON export/save affordances to both Browser/Web and Desktop surfaces after successful vault audit runtime responses.

**Architecture:** Reuse existing runtime-shaped vault audit JSON responses already stored in browser/desktop state. Add separate audit-event export helpers that are distinct from existing PHI-safe response reports, require successful `vault_audit` responses, include only the `events` array plus counts/pagination metadata, and use source-derived safe filenames with static fallbacks. This is de-identification audit UX only; no agent/controller/orchestration semantics.

**Tech Stack:** Rust workspace, `mdid-browser` helper/state tests, `mdid-desktop` helper/state/UI tests, `serde_json`, Cargo test commands.

---

## File structure

- Modify: `crates/mdid-browser/src/app.rs` — add `BrowserFlowState` audit-events export helper, source-derived filename helper, availability guard, and tests.
- Modify: `crates/mdid-desktop/src/lib.rs` — add `DesktopAuditEventsExportError`, `DesktopVaultResponseState::audit_events_export_json`, `write_desktop_audit_events_json`, and tests.
- Modify: `crates/mdid-desktop/src/main.rs` — wire desktop audit-events save path/status/action beside existing vault response/decode-values save actions.
- Modify: `README.md` — truth-sync completion snapshot and verification evidence after landed implementation.

### Task 1: Browser audit-events JSON download helper

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`
- Test: inline `#[cfg(test)]` tests in `crates/mdid-browser/src/app.rs`

- [ ] **Step 1: Write failing tests**

Add tests that construct `BrowserFlowState` in `InputMode::VaultAuditEvents` with a successful JSON response containing `events`, `event_count`, `returned_event_count`, and `next_offset`, then assert `prepared_audit_events_download_payload()` returns `mode: vault_audit_events`, copies only audit events/count metadata, uses `application/json;charset=utf-8`, and uses a safe source-derived filename. Add rejection tests for non-audit mode and audit responses without `events`.

- [ ] **Step 2: Run RED**

Run: `cargo test -p mdid-browser audit_events_download -- --nocapture`
Expected: FAIL because audit-events download helper does not exist yet.

- [ ] **Step 3: Implement minimal browser helper**

Add methods on `BrowserFlowState`: `suggested_audit_events_file_name`, `audit_events_payload`, `can_export_audit_events`, and `prepared_audit_events_download_payload`. The helper must require `InputMode::VaultAuditEvents`, parse `result_output` first and fall back to `summary`, require `events` as an array, include `event_count`, `returned_event_count`, and `next_offset` when present, and avoid adding vault path/passphrase/raw request fields.

- [ ] **Step 4: Run GREEN and broader browser tests**

Run: `cargo test -p mdid-browser audit_events_download -- --nocapture`
Expected: PASS.
Run: `cargo test -p mdid-browser vault_audit -- --nocapture`
Expected: PASS.
Run: `cargo test -p mdid-browser --lib`
Expected: PASS.

- [ ] **Step 5: Commit**

Run: `git add crates/mdid-browser/src/app.rs && git commit -m "feat(browser): add audit events downloads"`

### Task 2: Desktop audit-events JSON save helper and UI wiring

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`
- Modify: `crates/mdid-desktop/src/main.rs`
- Test: inline `#[cfg(test)]` tests in `crates/mdid-desktop/src/lib.rs` and `crates/mdid-desktop/src/main.rs`

- [ ] **Step 1: Write failing tests**

Add `mdid-desktop` tests that apply a successful `DesktopVaultResponseMode::VaultAudit` response containing `events`, `event_count`, `returned_event_count`, and `next_offset`, then assert `audit_events_export_json()` returns `mode: vault_audit_events`, includes the events/count metadata, and excludes passphrase/vault path/request fields. Add a writer test for `write_desktop_audit_events_json`. Add `main.rs` tests that default/generated audit event save paths use source-derived safe stems and preserve explicit user overrides.

- [ ] **Step 2: Run RED**

Run: `cargo test -p mdid-desktop audit_events_export -- --nocapture`
Expected: FAIL because audit-events save helper/UI state does not exist yet.

- [ ] **Step 3: Implement minimal desktop helper and UI state**

In `lib.rs`, add `DesktopAuditEventsExportError`, `DesktopVaultResponseState::audit_events_export_json`, and `write_desktop_audit_events_json`. In `main.rs`, add audit-events save path/generated-path/status fields, refresh them after successful vault audit responses, render a save control only when `audit_events_export_json().is_ok()`, and write the JSON through `write_desktop_audit_events_json` with PHI-safe status strings.

- [ ] **Step 4: Run GREEN and broader desktop tests**

Run: `cargo test -p mdid-desktop audit_events_export -- --nocapture`
Expected: PASS.
Run: `cargo test -p mdid-desktop vault_audit -- --nocapture`
Expected: PASS.
Run: `cargo test -p mdid-desktop --lib`
Expected: PASS.
Run: `cargo test -p mdid-desktop --bin mdid-desktop`
Expected: PASS.

- [ ] **Step 5: Commit**

Run: `git add crates/mdid-desktop/src/lib.rs crates/mdid-desktop/src/main.rs && git commit -m "feat(desktop): add audit events saves"`

### Task 3: README truth-sync and final verification

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/plans/2026-04-30-audit-event-export-surface.md` only if implementation discoveries require scope clarification.

- [ ] **Step 1: Run final verification**

Run: `cargo test -p mdid-browser audit_events_download -- --nocapture`
Expected: PASS.
Run: `cargo test -p mdid-browser --lib`
Expected: PASS.
Run: `cargo test -p mdid-desktop audit_events_export -- --nocapture`
Expected: PASS.
Run: `cargo test -p mdid-desktop --lib`
Expected: PASS.
Run: `cargo test -p mdid-desktop --bin mdid-desktop`
Expected: PASS.
Run: `cargo fmt --check`
Expected: PASS.
Run: `git diff --check`
Expected: PASS.

- [ ] **Step 2: Update README completion snapshot**

Update the current repository status to record this branch/commit, browser/web +5 points for explicit bounded audit-events download, desktop app +5 points for explicit bounded audit-events save, and an honest overall completion increase only if controller-visible landed tests support it. Keep missing items explicit: OCR/visual redaction, PDF/media rewrite/export, richer workflows, packaging/hardening, and deeper policy/detection remain missing.

- [ ] **Step 3: Commit README truth-sync**

Run: `git add README.md docs/superpowers/plans/2026-04-30-audit-event-export-surface.md && git commit -m "docs: truth-sync audit event exports"`

- [ ] **Step 4: Final controller checks**

Run: `git status --short && git log --oneline -8`
Expected: branch clean except any intentionally uncommitted files must be explained; latest commits are browser helper, desktop helper, README truth-sync.
