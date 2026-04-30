# CLI Portable Blank Context Validation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `mdid-cli vault-export` and `mdid-cli vault-import` reject blank `--context` values before portable vault operations run.

**Architecture:** Keep the validation in the existing CLI argument parsers so invalid portable-operation context is rejected before vault/application calls. Use the same PHI-safe missing-context message for absent and blank contexts, without echoing user input.

**Tech Stack:** Rust, Cargo tests, existing `mdid-cli` parser tests in `crates/mdid-cli/src/main.rs`.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add parser-level blank context checks in `parse_vault_export_args` and `parse_vault_import_args`.
  - Add focused unit tests beside existing CLI parser tests.
- Modify: `README.md`
  - Truth-sync the completion snapshot and verification evidence after the landed slice. Completion percentages remain CLI 95%, browser/web 76%, desktop app 70%, overall 93% because this is bounded CLI safety hardening, not a major missing capability.

### Task 1: Reject blank portable operation context in CLI parsers

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/src/main.rs`

- [x] **Step 1: Write the failing tests**

Add these tests near the existing CLI parser tests:

```rust
#[test]
fn parse_vault_export_rejects_blank_context_without_echoing_value() {
    let error = parse_vault_export_args(&[
        "--vault-path".to_string(),
        "vault.json".to_string(),
        "--passphrase".to_string(),
        "secret".to_string(),
        "--record-ids-json".to_string(),
        "[\"record-1\"]".to_string(),
        "--export-passphrase".to_string(),
        "portable-secret".to_string(),
        "--context".to_string(),
        "   ".to_string(),
        "--artifact-path".to_string(),
        "artifact.json".to_string(),
    ])
    .expect_err("blank export context should be rejected");

    assert_eq!(error, "missing --context");
    assert!(!error.contains("record-1"));
    assert!(!error.contains("portable-secret"));
}

#[test]
fn parse_vault_import_rejects_blank_context_without_echoing_value() {
    let error = parse_vault_import_args(&[
        "--vault-path".to_string(),
        "vault.json".to_string(),
        "--passphrase".to_string(),
        "secret".to_string(),
        "--artifact-path".to_string(),
        "artifact.json".to_string(),
        "--portable-passphrase".to_string(),
        "portable-secret".to_string(),
        "--context".to_string(),
        "\t".to_string(),
    ])
    .expect_err("blank import context should be rejected");

    assert_eq!(error, "missing --context");
    assert!(!error.contains("artifact.json"));
    assert!(!error.contains("portable-secret"));
}
```

- [x] **Step 2: Run tests to verify RED**

Run: `cargo test -p mdid-cli blank_context -- --nocapture`

Expected: FAIL because blank `--context` is currently accepted by the parsers.

- [x] **Step 3: Write minimal implementation**

In `parse_vault_export_args`, replace the direct `context: context.ok_or_else(...)?,` assignment with:

```rust
let context = context.ok_or_else(|| "missing --context".to_string())?;
if context.trim().is_empty() {
    return Err("missing --context".to_string());
}

Ok(VaultExportArgs {
    vault_path: vault_path.ok_or_else(|| "missing --vault-path".to_string())?,
    passphrase: passphrase.ok_or_else(|| "missing --passphrase".to_string())?,
    record_ids_json: record_ids_json.ok_or_else(|| "missing --record-ids-json".to_string())?,
    export_passphrase: export_passphrase
        .ok_or_else(|| "missing --export-passphrase".to_string())?,
    context,
    artifact_path: artifact_path.ok_or_else(|| "missing --artifact-path".to_string())?,
})
```

In `parse_vault_import_args`, replace the direct `context: context.ok_or_else(...)?,` assignment with:

```rust
let context = context.ok_or_else(|| "missing --context".to_string())?;
if context.trim().is_empty() {
    return Err("missing --context".to_string());
}

Ok(VaultImportArgs {
    vault_path: vault_path.ok_or_else(|| "missing --vault-path".to_string())?,
    passphrase: passphrase.ok_or_else(|| "missing --passphrase".to_string())?,
    artifact_path: artifact_path.ok_or_else(|| "missing --artifact-path".to_string())?,
    portable_passphrase: portable_passphrase
        .ok_or_else(|| "missing --portable-passphrase".to_string())?,
    context,
})
```

- [x] **Step 4: Run targeted and broader verification**

Run: `cargo test -p mdid-cli blank_context -- --nocapture`
Expected: PASS.

Run: `cargo test -p mdid-cli vault_export -- --nocapture && cargo test -p mdid-cli vault_import -- --nocapture`
Expected: PASS.

Run: `cargo fmt --check && git diff --check`
Expected: PASS.

- [x] **Step 5: Commit implementation**

```bash
git add crates/mdid-cli/src/main.rs docs/superpowers/plans/2026-04-30-cli-portable-blank-context-validation.md
git commit -m "fix(cli): reject blank portable context"
```

### Task 2: README completion truth-sync

**Files:**
- Modify: `README.md`

- [x] **Step 1: Update completion snapshot**

Update `Current repository status` to mention the blank portable context parser hardening. Keep CLI 95%, browser/web 76%, desktop app 70%, and overall 93%.

- [x] **Step 2: Run docs verification**

Run: `cargo fmt --check && git diff --check`
Expected: PASS.

- [x] **Step 3: Commit docs truth-sync**

```bash
git add README.md docs/superpowers/plans/2026-04-30-cli-portable-blank-context-validation.md
git commit -m "docs: truth-sync portable context validation"
```
