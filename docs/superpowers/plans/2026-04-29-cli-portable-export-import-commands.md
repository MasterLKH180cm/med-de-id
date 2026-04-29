# CLI Portable Export/Import Commands Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add bounded CLI commands for local portable vault subset export and local portable artifact import, using the already-landed vault portable APIs without adding workflow/controller semantics.

**Architecture:** Extend `mdid-cli` command parsing with two narrow automation commands: `vault-export` writes an encrypted portable artifact JSON file and prints a PHI-safe summary, while `vault-import` reads an encrypted portable artifact JSON file into a local vault and prints PHI-safe imported/duplicate counts. Both commands call `LocalVaultStore` directly and use `SurfaceKind::Cli`, keeping passphrases and artifact contents out of stdout.

**Tech Stack:** Rust workspace, `mdid-cli`, `mdid-vault`, `serde_json`, existing Cargo tests.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `VaultExportArgs` and `VaultImportArgs`.
  - Add `CliCommand::VaultExport` and `CliCommand::VaultImport`.
  - Add flag parsers for `vault-export` and `vault-import`.
  - Add bounded command runners using `LocalVaultStore::open`, `export_portable`, `import_portable`, `serde_json::to_vec_pretty`, and `serde_json::from_slice`.
  - Add tests for parsing, PHI-safe output shape, missing/blank arguments, and full export/import roundtrip.
- Modify: `README.md`
  - Truth-sync CLI/browser/desktop/overall completion snapshot after the landed CLI commands.

### Task 1: Add CLI `vault-export` command

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: inline tests in `crates/mdid-cli/src/main.rs`

- [ ] **Step 1: Write failing parser and roundtrip tests**

Add tests that expect:
- `vault-export --vault-path <path> --passphrase <secret> --record-ids-json '["<uuid>"]' --export-passphrase <portable-secret> --context "handoff" --artifact-path <path>` parses into `CliCommand::VaultExport`.
- Running the command against a temporary local vault containing one encoded record writes a JSON artifact file and prints only a PHI-safe summary with `command`, `exported_records`, `artifact_path`, and `audit_event_id`.
- stdout must not contain original PHI or either passphrase.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p mdid-cli vault_export -- --nocapture`
Expected: FAIL because `VaultExport` parsing/running does not exist yet.

- [ ] **Step 3: Implement minimal command**

Implement:
- `VaultExportArgs { vault_path, passphrase, record_ids_json, export_passphrase, context, artifact_path }`
- `CliCommand::VaultExport(VaultExportArgs)`
- parser branch for `vault-export`
- `parse_vault_export_args`
- `run_vault_export`

`run_vault_export` must:
1. parse UUID list from `record_ids_json`, rejecting empty lists and invalid UUIDs with existing-style error strings;
2. open the local vault with `LocalVaultStore::open(&vault_path, &passphrase)`;
3. call `export_portable(&record_ids, &export_passphrase, SurfaceKind::Cli, &context)`;
4. serialize artifact with `serde_json::to_vec_pretty`;
5. write the artifact to `artifact_path`;
6. print JSON summary only:

```json
{
  "command": "vault-export",
  "exported_records": 1,
  "artifact_path": "/tmp/export.json",
  "audit_event_id": "<uuid>"
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p mdid-cli vault_export -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-cli/src/main.rs
git commit -m "feat(cli): add bounded portable vault export"
```

### Task 2: Add CLI `vault-import` command

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: inline tests in `crates/mdid-cli/src/main.rs`

- [ ] **Step 1: Write failing parser and import tests**

Add tests that expect:
- `vault-import --vault-path <path> --passphrase <secret> --artifact-path <path> --portable-passphrase <portable-secret> --context "import"` parses into `CliCommand::VaultImport`.
- Running export then import into a second vault prints only a PHI-safe summary with `command`, `imported_records`, `duplicate_records`, and `audit_event_id`.
- Re-running import against the same target reports duplicates without leaking token/original values.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p mdid-cli vault_import -- --nocapture`
Expected: FAIL because `VaultImport` parsing/running does not exist yet.

- [ ] **Step 3: Implement minimal command**

Implement:
- `VaultImportArgs { vault_path, passphrase, artifact_path, portable_passphrase, context }`
- `CliCommand::VaultImport(VaultImportArgs)`
- parser branch for `vault-import`
- `parse_vault_import_args`
- `run_vault_import`

`run_vault_import` must:
1. read artifact JSON bytes from `artifact_path`;
2. parse as `mdid_vault::PortableVaultArtifact`;
3. open the local vault with `LocalVaultStore::open(&vault_path, &passphrase)`;
4. call `import_portable(artifact, &portable_passphrase, SurfaceKind::Cli, &context)`;
5. print JSON summary only:

```json
{
  "command": "vault-import",
  "imported_records": 1,
  "duplicate_records": 0,
  "audit_event_id": "<uuid>"
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p mdid-cli vault_import -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Run focused CLI tests**

Run: `cargo test -p mdid-cli -- --nocapture`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/mdid-cli/src/main.rs
git commit -m "feat(cli): add bounded portable vault import"
```

### Task 3: README truth-sync for portable CLI completion

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update completion snapshot text**

Adjust the README completion snapshot to say CLI includes bounded `vault-export` and `vault-import`, and update completion numbers conservatively:
- CLI: 82%
- Browser/web: 38%
- Desktop app: 35%
- Overall: 66%

Preserve explicit gaps: no generalized transfer workflow, no controller/agent/orchestration, no OCR/visual redaction/PDF rewrite.

- [ ] **Step 2: Verify README mentions are truthful**

Run: `grep -n "vault-export\|vault-import\|Overall" README.md`
Expected: README lists the new commands and updated overall snapshot.

- [ ] **Step 3: Run regression tests**

Run: `cargo test -p mdid-cli -- --nocapture`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add README.md docs/superpowers/plans/2026-04-29-cli-portable-export-import-commands.md
git commit -m "docs: truth-sync cli portable completion"
```

## Self-Review

- Spec coverage: Covers bounded portable export/import CLI commands, tests, PHI-safe summaries, README completion maintenance, and explicitly excludes controller/agent workflow semantics.
- Placeholder scan: No TBD/TODO/fill-in placeholders remain.
- Type consistency: `VaultExportArgs`, `VaultImportArgs`, `CliCommand::VaultExport`, `CliCommand::VaultImport`, `run_vault_export`, and `run_vault_import` names are consistent across tasks.
