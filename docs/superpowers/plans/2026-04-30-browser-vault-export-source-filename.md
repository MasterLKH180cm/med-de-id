# Browser Vault Export Source Filename Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make browser vault export artifact downloads use a sanitized vault/source-derived filename when an imported vault/source name is available.

**Architecture:** Keep the change inside the existing `BrowserFlowState::suggested_export_file_name()` filename policy. Do not alter vault export payload contents, runtime routes, encryption semantics, or portable artifact structure.

**Tech Stack:** Rust workspace, `mdid-browser`, existing unit tests in `crates/mdid-browser/src/app.rs`, Cargo test runner.

---

## File Structure

- Modify: `crates/mdid-browser/src/app.rs`
  - Extend `BrowserFlowState::suggested_export_file_name()` so `InputMode::VaultExport` returns `<safe-stem>-portable-artifact.json` when `imported_file_name` is present and sanitizes to a non-default stem.
  - Add a focused unit test near existing browser vault/portable filename tests.
- Modify: `README.md`
  - Truth-sync current completion snapshot after landing and verification.

### Task 1: Browser vault export source-aware artifact filename

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`
- Test: `crates/mdid-browser/src/app.rs`

- [ ] **Step 1: Write the failing test**

Add this test near `browser_vault_response_downloads_use_safe_source_filenames`:

```rust
#[test]
fn browser_vault_export_download_uses_safe_source_filename() {
    let state = BrowserFlowState {
        input_mode: InputMode::VaultExport,
        imported_file_name: Some("Clinic Vault Backup 2026.vault".to_string()),
        ..BrowserFlowState::default()
    };

    assert_eq!(
        state.suggested_export_file_name(),
        "Clinic_Vault_Backup_2026-portable-artifact.json"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p mdid-browser --lib browser_vault_export_download_uses_safe_source_filename -- --nocapture`
Expected: FAIL because the current implementation returns `mdid-browser-portable-artifact.json` for `InputMode::VaultExport` even when `imported_file_name` is present.

- [ ] **Step 3: Write minimal implementation**

In `BrowserFlowState::suggested_export_file_name()`, replace the empty `InputMode::VaultExport => {}` arm inside the `if let Some(file_name) = &self.imported_file_name` block with:

```rust
InputMode::VaultExport => {
    return format!("{stem}-portable-artifact.json");
}
```

- [ ] **Step 4: Run targeted test to verify it passes**

Run: `cargo test -p mdid-browser --lib browser_vault_export_download_uses_safe_source_filename -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Run related browser filename tests**

Run: `cargo test -p mdid-browser --lib source_filename -- --nocapture`
Expected: PASS for matching tests, or if the filter matches no tests, run `cargo test -p mdid-browser --lib browser_vault -- --nocapture` and expect PASS.

- [ ] **Step 6: Run browser lib tests**

Run: `cargo test -p mdid-browser --lib`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/mdid-browser/src/app.rs docs/superpowers/plans/2026-04-30-browser-vault-export-source-filename.md
git commit -m "feat(browser): use vault source name for export artifacts"
```

### Task 2: README truth-sync for browser export filename slice

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update completion snapshot text**

Update the current repository status snapshot to mention the landed browser vault export source-aware artifact filename. Keep CLI at 95%, Browser/web at 75%, Desktop app at 69%, Overall at 93% unless verified landed functionality justifies a higher number; this slice should not claim a percentage increase by itself.

- [ ] **Step 2: Run docs verification commands**

Run:

```bash
cargo test -p mdid-browser --lib browser_vault_export_download_uses_safe_source_filename -- --nocapture
cargo test -p mdid-browser --lib
git diff --check
```

Expected: all PASS / no whitespace errors.

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: truth-sync browser vault export filenames"
```
