# CLI Portable Inspect Command Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded `mdid-cli vault-inspect-artifact` command that inspects encrypted portable vault artifacts locally and prints only PHI-safe record counts/previews.

**Architecture:** Extend the existing `mdid-cli` portable command family with a read-only inspect command that reuses the landed bounded artifact reader and the current `PortableVaultArtifact::unlock` inspection path. The command must parse an artifact file plus portable passphrase, decrypt only enough metadata for an inspection summary, and keep passphrases, tokens, original values, vault paths, and artifact ciphertext out of stdout.

**Tech Stack:** Rust workspace, `mdid-cli`, `mdid-vault`, `serde_json`, existing Cargo tests.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `VaultInspectArtifactArgs`.
  - Add `CliCommand::VaultInspectArtifact`.
  - Add parser branch for `vault-inspect-artifact`.
  - Add bounded `run_vault_inspect_artifact_for_summary` and `run_vault_inspect_artifact` helpers.
  - Add tests for parser behavior, PHI-safe output shape, bounded artifact read reuse, and inspect count.
- Modify: `README.md`
  - Truth-sync CLI/browser/desktop/overall completion snapshot after the landed command.

### Task 1: Add CLI `vault-inspect-artifact` command

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: inline tests in `crates/mdid-cli/src/main.rs`

- [x] **Step 1: Write failing parser and PHI-safe inspect tests**

Add tests to `crates/mdid-cli/src/main.rs` that expect this command to parse:

```rust
let command = CliCommand::parse(&[
    "vault-inspect-artifact".to_string(),
    "--artifact-path".to_string(),
    "portable-export.json".to_string(),
    "--portable-passphrase".to_string(),
    "portable-secret".to_string(),
])
.expect("parse vault inspect artifact");
assert_eq!(
    command,
    CliCommand::VaultInspectArtifact(VaultInspectArtifactArgs {
        artifact_path: PathBuf::from("portable-export.json"),
        portable_passphrase: "portable-secret".to_string(),
    })
);
```

Add a roundtrip test that creates a local vault, encodes one known PHI value, exports it with `run_vault_export_for_summary`, then calls `run_vault_inspect_artifact_for_summary(VaultInspectArtifactArgs { artifact_path, portable_passphrase })`. Assert parsed stdout contains:

```json
{
  "command": "vault-inspect-artifact",
  "record_count": 1
}
```

Assert stdout does not contain the PHI value, vault passphrase, portable passphrase, token, vault path, `ciphertext_b64`, `nonce_b64`, or `salt_b64`.

- [x] **Step 2: Run test to verify it fails**

Run: `cargo test -p mdid-cli vault_inspect_artifact -- --nocapture`
Expected: FAIL because `VaultInspectArtifact` parsing/running does not exist yet.

- [x] **Step 3: Implement minimal command**

Implement:

```rust
#[derive(Debug, Clone, Eq, PartialEq)]
struct VaultInspectArtifactArgs {
    artifact_path: PathBuf,
    portable_passphrase: String,
}
```

Add `CliCommand::VaultInspectArtifact(VaultInspectArtifactArgs)`, parser branch for `vault-inspect-artifact`, and `parse_vault_inspect_artifact_args` accepting only:
- `--artifact-path <path>`
- `--portable-passphrase <value>`

Add:

```rust
fn run_vault_inspect_artifact(args: VaultInspectArtifactArgs) -> Result<(), String> {
    println!("{}", run_vault_inspect_artifact_for_summary(args)?);
    Ok(())
}

fn run_vault_inspect_artifact_for_summary(args: VaultInspectArtifactArgs) -> Result<String, String> {
    let artifact_bytes = read_bounded_portable_artifact(&args.artifact_path)?;
    let artifact: PortableVaultArtifact = serde_json::from_slice(&artifact_bytes)
        .map_err(|err| format!("failed to parse portable artifact: {err}"))?;
    let artifact = PortableVaultArtifact::unlock(&artifact, &args.portable_passphrase)
        .map_err(|err| format!("failed to inspect portable artifact: {err}"))?;
    let stdout = json!({
        "command": "vault-inspect-artifact",
        "record_count": artifact.records.len(),
    });
    serde_json::to_string(&stdout).map_err(|err| format!("failed to render portable inspect summary: {err}"))
}
```

If the exact artifact inspection API changes in the future, inspect `mdid-vault` and use the already-landed runtime/desktop pattern while preserving the same PHI-safe stdout contract.

- [x] **Step 4: Run test to verify it passes**

Run: `cargo test -p mdid-cli vault_inspect_artifact -- --nocapture`
Expected: PASS.

- [x] **Step 5: Run focused CLI tests**

Run: `cargo test -p mdid-cli -- --nocapture`
Expected: PASS.

- [x] **Step 6: Commit**

```bash
git add crates/mdid-cli/src/main.rs docs/superpowers/plans/2026-04-29-cli-portable-inspect-command.md
git commit -m "feat(cli): add bounded portable artifact inspect"
```

### Task 2: README truth-sync for CLI portable inspect

**Files:**
- Modify: `README.md`

- [x] **Step 1: Update completion snapshot text**

Update the completion snapshot to mention `vault-inspect-artifact` as a landed CLI portable artifact inspection command. Conservatively raise CLI completion only if tests pass and the command is committed; browser/web and desktop percentages should remain unchanged unless this task lands additional browser/desktop code.

Use these completion numbers if Task 1 lands and verification passes:
- CLI: 93%
- Browser/web: 61%
- Desktop app: 58%
- Overall: 86%

Preserve explicit gaps: no OCR/visual redaction/PDF rewrite, no generalized transfer workflow UX, no vault browsing, no auth/session, no controller/agent/orchestration semantics.

- [x] **Step 2: Verify README mentions are truthful**

Run: `grep -n "vault-inspect-artifact\|Completion snapshot\|Overall" README.md`
Expected: README lists the new command and the updated snapshot date/context.

- [x] **Step 3: Run regression tests**

Run: `cargo test -p mdid-cli -- --nocapture`
Expected: PASS.

- [x] **Step 4: Commit**

```bash
git add README.md
git commit -m "docs: truth-sync cli portable inspect completion"
```

## Self-Review

- Spec coverage: Covers bounded CLI portable artifact inspection, parser, PHI-safe summary, bounded artifact reads, README completion maintenance, and explicit exclusion of workflow/controller/agent semantics.
- Placeholder scan: No TBD/TODO/fill-in placeholders remain.
- Type consistency: `VaultInspectArtifactArgs`, `CliCommand::VaultInspectArtifact`, `parse_vault_inspect_artifact_args`, `run_vault_inspect_artifact`, and `run_vault_inspect_artifact_for_summary` are consistent across tasks.
