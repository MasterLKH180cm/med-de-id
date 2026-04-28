# CLI DICOM De-identify Command Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded `mdid-cli deidentify-dicom` automation command that rewrites a local DICOM file through the existing application/vault stack and prints only a PHI-safe summary.

**Architecture:** Keep the CLI thin: parse local paths and private-tag policy, read DICOM bytes, create/unlock the local vault, delegate to `mdid_application::DicomDeidentificationService::deidentify_bytes`, write returned DICOM bytes to disk, and print a safe JSON envelope. Reuse existing DICOM application/adapter semantics; do not add OCR, browser/desktop workflow, server orchestration, or agent/controller behavior.

**Tech Stack:** Rust workspace, `mdid-cli`, `mdid-application`, `mdid-domain`, `mdid-vault`, `assert_cmd`, `serde_json`, existing DICOM adapter test fixture helpers.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `CliCommand::DeidentifyDicom` and `DeidentifyDicomArgs` without deriving `Debug`.
  - Parse command shape: `mdid-cli deidentify-dicom --dicom-path <input.dcm> --private-tag-policy <remove|review|required|keep> --vault-path <vault.json> --passphrase <passphrase> --output-path <output.dcm>`.
  - Run the command by delegating to `DicomDeidentificationService::deidentify_bytes(..., SurfaceKind::Cli, &mut vault, policy)`.
  - Write `rewritten_dicom_bytes` to `--output-path` and print a PHI-safe JSON summary with `output_path`, `sanitized_file_name`, `summary`, and `review_queue_len` only.
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
  - Add DICOM fixture builder local to the CLI smoke test or copy minimal fixture-building pattern from existing adapter/application tests.
  - Add success, invalid bytes, and PHI-safe stdout tests.
- Modify: `README.md`
  - Truth-sync completion snapshot after verified landing: CLI increases from 62% to 68%; Overall increases from 55% to 58%; Browser/web remains 34%; Desktop app remains 35%.
  - Add DICOM CLI command to current CLI narrative and missing-items list.

### Task 1: Add bounded CLI DICOM rewrite command

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
- Modify: `README.md`

- [x] **Step 1: Write failing parser test**

Add this test in the `#[cfg(test)] mod tests` block of `crates/mdid-cli/src/main.rs`:

```rust
#[test]
fn parses_deidentify_dicom_command_without_requiring_debug() {
    let args = vec![
        "deidentify-dicom".to_string(),
        "--dicom-path".to_string(),
        "input.dcm".to_string(),
        "--private-tag-policy".to_string(),
        "remove".to_string(),
        "--vault-path".to_string(),
        "vault.mdid".to_string(),
        "--passphrase".to_string(),
        "secret-passphrase".to_string(),
        "--output-path".to_string(),
        "output.dcm".to_string(),
    ];

    assert!(
        parse_command(&args)
            == Ok(CliCommand::DeidentifyDicom(DeidentifyDicomArgs {
                dicom_path: PathBuf::from("input.dcm"),
                private_tag_policy: DicomPrivateTagPolicy::Remove,
                vault_path: PathBuf::from("vault.mdid"),
                passphrase: "secret-passphrase".to_string(),
                output_path: PathBuf::from("output.dcm"),
            }))
    );
}
```

- [x] **Step 2: Run parser test to verify RED**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-cli parses_deidentify_dicom_command_without_requiring_debug -- --nocapture`
Expected: FAIL/compile error because `DeidentifyDicom`, `DeidentifyDicomArgs`, and `DicomPrivateTagPolicy` import are missing.

- [x] **Step 3: Add failing CLI smoke tests**

Add DICOM fixture imports/helpers to `crates/mdid-cli/tests/cli_smoke.rs` using existing adapter/application test fixture patterns, then add:

```rust
#[test]
fn cli_deidentify_dicom_writes_rewritten_dicom_and_phi_safe_summary() {
    let dir = tempdir().unwrap();
    let input_path = dir.path().join("alice-smith-source.dcm");
    let output_path = dir.path().join("output.dcm");
    let vault_path = dir.path().join("vault.mdid");
    fs::write(&input_path, build_dicom_fixture("NO", true)).unwrap();

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "deidentify-dicom",
            "--dicom-path",
            input_path.to_str().unwrap(),
            "--private-tag-policy",
            "remove",
            "--vault-path",
            vault_path.to_str().unwrap(),
            "--passphrase",
            "correct horse battery staple",
            "--output-path",
            output_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Alice").not())
        .stdout(predicate::str::contains("SMITH").not())
        .stdout(predicate::str::contains("correct horse battery staple").not())
        .get_output()
        .stdout
        .clone();

    assert!(output_path.exists());
    assert!(fs::metadata(&output_path).unwrap().len() > 0);
    let payload: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(payload["output_path"], output_path.to_string_lossy().to_string());
    assert_eq!(payload["sanitized_file_name"], "deidentified.dcm");
    assert!(payload["summary"]["rewritten_tags"].as_u64().unwrap() >= 1);
    assert!(payload["review_queue_len"].as_u64().unwrap() >= 1);
    assert!(payload.get("rewritten_dicom_bytes_base64").is_none());
    assert!(payload.get("review_queue").is_none());
}

#[test]
fn cli_deidentify_dicom_rejects_invalid_dicom_without_scope_drift_terms() {
    let dir = tempdir().unwrap();
    let input_path = dir.path().join("invalid.dcm");
    let output_path = dir.path().join("output.dcm");
    let vault_path = dir.path().join("vault.mdid");
    fs::write(&input_path, b"not a dicom file").unwrap();

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "deidentify-dicom",
            "--dicom-path",
            input_path.to_str().unwrap(),
            "--private-tag-policy",
            "remove",
            "--vault-path",
            vault_path.to_str().unwrap(),
            "--passphrase",
            "correct horse battery staple",
            "--output-path",
            output_path.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("failed to deidentify DICOM"))
        .stderr(predicate::str::contains("agent").not())
        .stderr(predicate::str::contains("controller").not())
        .stderr(predicate::str::contains("moat").not());

    assert!(!output_path.exists());
}
```

- [x] **Step 4: Run CLI smoke tests to verify RED**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-cli cli_deidentify_dicom -- --nocapture`
Expected: FAIL/compile error because the command is not implemented.

- [x] **Step 5: Implement minimal CLI DICOM command**

In `crates/mdid-cli/src/main.rs`:
- Import `DicomDeidentificationService` and `DicomPrivateTagPolicy`.
- Add `DeidentifyDicomArgs` with `dicom_path`, `private_tag_policy`, `vault_path`, `passphrase`, `output_path`.
- Parse `--private-tag-policy` values: `remove`, `review`/`review-required`, and `keep`.
- Delegate to existing application service and write only returned bytes.
- Print only safe aggregate JSON.

- [x] **Step 6: Run targeted tests to verify GREEN**

Run:
`source "$HOME/.cargo/env" && cargo test -p mdid-cli parses_deidentify_dicom_command_without_requiring_debug -- --nocapture`
`source "$HOME/.cargo/env" && cargo test -p mdid-cli cli_deidentify_dicom -- --nocapture`
Expected: PASS.

- [x] **Step 7: Run broader CLI verification**

Run:
`source "$HOME/.cargo/env" && cargo test -p mdid-cli --all-targets`
`source "$HOME/.cargo/env" && cargo clippy -p mdid-cli --all-targets -- -D warnings`
`git diff --check`
Expected: PASS.

- [x] **Step 8: Update README completion snapshot**

Update README based only on landed tests/features:
- CLI: `68%` with bounded `deidentify-csv`, `deidentify-xlsx`, and `deidentify-dicom`.
- Browser/web: remains `34%`.
- Desktop app: remains `35%`.
- Overall: `58%`.
- Missing items: CLI still lacks vault decode/audit/PDF/conservative-media/import-export commands; browser and desktop deeper workflows remain missing.

- [ ] **Step 9: Commit**

Run:
```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs README.md docs/superpowers/plans/2026-04-29-cli-dicom-deidentify-command.md
git commit -m "feat(cli): add bounded dicom deidentify command"
```
