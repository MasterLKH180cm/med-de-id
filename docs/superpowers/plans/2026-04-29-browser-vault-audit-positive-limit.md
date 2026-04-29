# Browser Vault Audit Positive Limit Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Align the browser vault audit form with the existing runtime/CLI/desktop contract by rejecting zero audit limits before localhost submission.

**Architecture:** Keep the change bounded to browser request construction and README truth-sync. The browser already builds a JSON request for `/vault/audit/events`; this plan tightens its validation so optional `limit` must be a positive integer when present, matching the landed runtime and desktop behavior.

**Tech Stack:** Rust workspace, `mdid-browser` crate, Yew-facing state helpers, `serde_json`, Cargo tests.

---

## File Structure

- Modify: `crates/mdid-browser/src/app.rs`
  - Responsibility: browser flow state and local request payload builders. Add one RED test for zero-limit rejection and update `build_vault_audit_request_payload` validation to require parsed limit > 0.
- Modify: `README.md`
  - Responsibility: completion snapshot truth-sync for this landed browser validation slice; completion numbers may remain unchanged unless the evidence warrants a truthful bump.

### Task 1: Browser vault audit limit validation parity

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`
- Test: `crates/mdid-browser/src/app.rs`

- [ ] **Step 1: Write the failing test**

Add this test immediately after `vault_audit_payload_rejects_invalid_limit` in `crates/mdid-browser/src/app.rs`:

```rust
    #[test]
    fn vault_audit_payload_rejects_zero_limit() {
        let error = build_vault_audit_request_payload(
            "/tmp/local-vault",
            "passphrase kept local",
            "decode",
            "browser",
            "0",
        )
        .expect_err("zero limit must be rejected before localhost submission");

        assert!(error.contains("positive"));
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test -p mdid-browser vault_audit_payload_rejects_zero_limit -- --nocapture
```

Expected: FAIL because the current browser payload builder accepts `0` as `limit`.

- [ ] **Step 3: Write minimal implementation**

In `build_vault_audit_request_payload`, replace the existing limit parse/insert block:

```rust
    if !limit.is_empty() {
        let parsed_limit = limit
            .parse::<usize>()
            .map_err(|_| "Vault audit limit must be a non-negative integer.".to_string())?;
        object.insert("limit".to_string(), serde_json::json!(parsed_limit));
    }
```

with:

```rust
    if !limit.is_empty() {
        let parsed_limit = limit
            .parse::<usize>()
            .map_err(|_| "Vault audit limit must be a positive integer.".to_string())?;
        if parsed_limit == 0 {
            return Err("Vault audit limit must be a positive integer.".to_string());
        }
        object.insert("limit".to_string(), serde_json::json!(parsed_limit));
    }
```

- [ ] **Step 4: Run targeted tests to verify it passes**

Run:

```bash
cargo test -p mdid-browser vault_audit_payload -- --nocapture
```

Expected: PASS for valid limit, blank optional filters, invalid non-number limit, and zero-limit rejection.

- [ ] **Step 5: Run relevant crate regression tests**

Run:

```bash
cargo test -p mdid-browser
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/mdid-browser/src/app.rs
git commit -m "fix(browser): require positive vault audit limit"
```

### Task 2: README completion truth-sync

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update completion snapshot wording**

Update the `Completion snapshot` sentence and Browser/web row in `README.md` to mention that browser vault audit limit validation now rejects zero locally and remains aligned with the runtime/CLI/desktop positive-limit contract. Do not inflate percentages unless supported by landed functionality; if numbers remain unchanged, keep them unchanged.

- [ ] **Step 2: Run documentation sanity check**

Run:

```bash
git diff -- README.md
```

Expected: diff only changes completion wording for this landed validation slice.

- [ ] **Step 3: Commit**

```bash
git add README.md docs/superpowers/plans/2026-04-29-browser-vault-audit-positive-limit.md
git commit -m "docs: truth-sync browser audit limit parity"
```

---

## Self-Review

1. Spec coverage: The plan implements browser positive-limit parity and updates README truthfully.
2. Placeholder scan: No TBD/TODO placeholders or vague implementation steps remain.
3. Type consistency: The function name `build_vault_audit_request_payload` and fields `limit`, `vault_path`, `vault_passphrase`, `kind`, and `actor` match the existing browser helper/test naming.
