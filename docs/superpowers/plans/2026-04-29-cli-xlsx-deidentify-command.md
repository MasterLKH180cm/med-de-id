# CLI XLSX Deidentify Command Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded `mdid-cli deidentify-xlsx` command that extracts a local XLSX workbook through the existing tabular adapter/application/vault path, then writes a normalized single-sheet XLSX generated from the de-identified tabular rows and prints only a PHI-safe summary.

**Architecture:** Keep the CLI thin and de-identification-only: parse local file paths and explicit field-policy JSON, read workbook bytes, use `XlsxTabularAdapter` for extraction, delegate to `TabularDeidentificationService::deidentify_extracted`, then render the returned CSV text into a bounded normalized single-sheet XLSX output. This CLI output is not workbook preservation: it does not preserve original workbook metadata, formatting, formulas, sheet names, or multiple sheets. Do not add agent/controller/moat workflow semantics, runtime networking, auth/session, vault browsing, or generalized orchestration.

**Tech Stack:** Rust, `mdid-cli`, `mdid-adapters::XlsxTabularAdapter`, `mdid-application::TabularDeidentificationService`, `mdid-vault::LocalVaultStore`, `rust_xlsxwriter` for normalized XLSX rendering and tests.

---

## File Structure

- Modify: `crates/mdid-cli/Cargo.toml` — use `rust_xlsxwriter` for normalized XLSX rendering/tests; no `base64` or `mdid-runtime` dependency is needed by the CLI command.
- Modify: `crates/mdid-cli/src/main.rs` — add `deidentify-xlsx` argument parsing, thin XLSX workflow, usage text, and parsing tests without sensitive `Debug` derives.
- Modify: `crates/mdid-cli/tests/cli_smoke.rs` — add end-to-end CLI XLSX smoke tests covering normalized valid XLSX output, encoded/removed PHI in encoded cells, review cells remaining, PHI-safe stdout, summary shape, and malformed workbook rejection.
- Modify: `README.md` — truth-sync CLI/browser/desktop/overall completion and missing items after landed verification.

## Task 1: Add bounded CLI XLSX de-identification command

**Files:**
- Modify: `crates/mdid-cli/Cargo.toml`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
- Modify: `README.md`

- [x] **Step 1: Write the failing parser test**

Add this test to `#[cfg(test)] mod tests` in `crates/mdid-cli/src/main.rs`:

```rust
#[test]
fn parses_deidentify_xlsx_command_without_requiring_debug() {
    let policies_json = r#"[{"header":"patient_name","phi_type":"NAME","action":"encode"}]"#;
    let args = vec![
        "deidentify-xlsx".to_string(),
        "--xlsx-path".to_string(),
        "input.xlsx".to_string(),
        "--policies-json".to_string(),
        policies_json.to_string(),
        "--vault-path".to_string(),
        "vault.mdid".to_string(),
        "--passphrase".to_string(),
        "secret-passphrase".to_string(),
        "--output-path".to_string(),
        "output.xlsx".to_string(),
    ];

    assert!(
        parse_command(&args)
            == Ok(CliCommand::DeidentifyXlsx(DeidentifyXlsxArgs {
                xlsx_path: PathBuf::from("input.xlsx"),
                policies_json: policies_json.to_string(),
                vault_path: PathBuf::from("vault.mdid"),
                passphrase: "secret-passphrase".to_string(),
                output_path: PathBuf::from("output.xlsx"),
            }))
    );
}
```

- [x] **Step 2: Run parser test and verify RED**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli parses_deidentify_xlsx_command_without_requiring_debug -- --nocapture
```

Expected: FAIL because `CliCommand::DeidentifyXlsx`, `DeidentifyXlsxArgs`, and parser support do not exist.

- [x] **Step 3: Write failing CLI smoke tests**

Add helpers/tests to `crates/mdid-cli/tests/cli_smoke.rs`:

```rust
use assert_cmd::Command;
use mdid_adapters::XlsxTabularAdapter;
use predicates::prelude::*;
use rust_xlsxwriter::Workbook;
use serde_json::Value;
use std::{fs, path::Path};
use tempfile::tempdir;

fn write_xlsx(path: &Path) {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    worksheet.write_string(0, 0, "patient_name").unwrap();
    worksheet.write_string(0, 1, "note").unwrap();
    worksheet.write_string(1, 0, "Alice Patient").unwrap();
    worksheet.write_string(1, 1, "needs follow-up").unwrap();
    workbook.save(path).unwrap();
}

#[test]
fn cli_deidentify_xlsx_writes_rewritten_workbook_and_phi_safe_summary() {
    let dir = tempdir().unwrap();
    let input_path = dir.path().join("input.xlsx");
    let output_path = dir.path().join("output.xlsx");
    let vault_path = dir.path().join("vault.mdid");
    write_xlsx(&input_path);

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "deidentify-xlsx",
            "--xlsx-path",
            input_path.to_str().unwrap(),
            "--policies-json",
            r#"[{"header":"patient_name","phi_type":"NAME","action":"encode"},{"header":"note","phi_type":"NOTE","action":"review"}]"#,
            "--vault-path",
            vault_path.to_str().unwrap(),
            "--passphrase",
            "correct horse battery staple",
            "--output-path",
            output_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Alice Patient").not())
        .stdout(predicate::str::contains("correct horse battery staple").not())
        .get_output()
        .stdout
        .clone();

    assert!(output_path.exists());
    let payload: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(payload["output_path"], output_path.to_string_lossy().to_string());
    assert_eq!(payload["summary"]["total_rows"], 1);
    assert_eq!(payload["summary"]["encoded_cells"], 1);
    assert_eq!(payload["summary"]["review_required_cells"], 1);
    assert!(payload["summary"].get("processed_rows").is_none());
    assert!(payload["summary"].get("review_items").is_none());
    assert_eq!(payload["review_queue_len"], 1);

    let output_bytes = fs::read(&output_path).unwrap();
    let extracted = XlsxTabularAdapter::new(Vec::new()).extract(&output_bytes).unwrap();
    assert!(extracted.rows[0][0].starts_with("tok-"));
    assert!(!extracted.rows[0][0].contains("Alice Patient"));
    assert_eq!(extracted.rows[0][1], "needs follow-up");
}

#[test]
fn cli_deidentify_xlsx_rejects_invalid_workbook_without_scope_drift_terms() {
    let dir = tempdir().unwrap();
    let input_path = dir.path().join("invalid.xlsx");
    let output_path = dir.path().join("output.xlsx");
    let vault_path = dir.path().join("vault.mdid");
    fs::write(&input_path, b"not an xlsx workbook").unwrap();

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "deidentify-xlsx",
            "--xlsx-path",
            input_path.to_str().unwrap(),
            "--policies-json",
            r#"[{"header":"patient_name","phi_type":"NAME","action":"encode"}]"#,
            "--vault-path",
            vault_path.to_str().unwrap(),
            "--passphrase",
            "correct horse battery staple",
            "--output-path",
            output_path.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("failed to read XLSX workbook"))
        .stderr(predicate::str::contains("agent").not())
        .stderr(predicate::str::contains("controller").not())
        .stderr(predicate::str::contains("moat").not());

    assert!(!output_path.exists());
}
```

- [x] **Step 4: Run smoke tests and verify RED**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli cli_deidentify_xlsx_writes_rewritten_workbook_and_phi_safe_summary cli_deidentify_xlsx_rejects_invalid_workbook_without_scope_drift_terms -- --nocapture
```

Expected: FAIL because the command and test dev-dependency are not wired yet.

- [x] **Step 5: Implement minimal command**

Implement exactly these behavior points:

```rust
use mdid_adapters::XlsxTabularAdapter;

#[derive(Clone, PartialEq, Eq)]
struct DeidentifyXlsxArgs {
    xlsx_path: PathBuf,
    policies_json: String,
    vault_path: PathBuf,
    passphrase: String,
    output_path: PathBuf,
}
```

- Add `CliCommand::DeidentifyXlsx(DeidentifyXlsxArgs)` without deriving `Debug` on sensitive argument structs.
- Add `parse_deidentify_xlsx_args(...)` accepting only `--xlsx-path`, `--policies-json`, `--vault-path`, `--passphrase`, and `--output-path`, with missing-flag errors matching the flag names.
- Add `run_deidentify_xlsx(args)` that:
  1. parses policies with existing `parse_policies`,
  2. reads workbook bytes with `fs::read`, returning `failed to read XLSX workbook: {err}`,
  3. opens or creates `LocalVaultStore` exactly like the CSV command,
  4. extracts with `XlsxTabularAdapter::new(policies).extract(&workbook_bytes)`, returning `failed to read XLSX workbook: {err}`,
  5. delegates to `TabularDeidentificationService.deidentify_extracted(extracted, &mut vault, SurfaceKind::Cli)`, returning `failed to deidentify XLSX: {err}`,
  6. renders `output.csv` (CSV text from `deidentify_extracted`) into a normalized single-sheet XLSX and writes those bytes to `--output-path`, returning `failed to render XLSX output: {err}` on render failure and `failed to write output XLSX: {err}` on write failure,
  7. prints existing PHI-safe JSON summary via `print_summary`.
- Update `usage()` to include `mdid-cli deidentify-xlsx --xlsx-path <path> --policies-json <json> --vault-path <path> --passphrase <value> --output-path <path>` and a command line `deidentify-xlsx  Rewrite a local XLSX using explicit field policies.`
- Do not add `base64` or `mdid-runtime` dependencies to `crates/mdid-cli`; the command renders CSV text into XLSX locally and does not call runtime routes.

Review/fix evidence (2026-04-29 spec review follow-up): smoke test was hardened to read the CLI output with `XlsxTabularAdapter`, verify the encoded cell no longer contains PHI and starts with `tok-`, verify the review cell remains, and verify summary remains `{ output_path, summary, review_queue_len }` with the aggregate `BatchSummary` directly under `summary` and no alias keys.

- [x] **Step 6: Run targeted tests and verify GREEN**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli parses_deidentify_xlsx_command_without_requiring_debug -- --nocapture
cargo test -p mdid-cli cli_deidentify_xlsx_writes_rewritten_workbook_and_phi_safe_summary cli_deidentify_xlsx_rejects_invalid_workbook_without_scope_drift_terms -- --nocapture
```

Expected: PASS.

- [x] **Step 7: Run broader CLI verification**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --all-targets
cargo clippy -p mdid-cli --all-targets -- -D warnings
git diff --check
```

Expected: PASS.

- [x] **Step 8: Update README completion truthfully**

Update `README.md` completion table:
- CLI: 55% -> 62%, because CLI now has both CSV and XLSX local tabular rewrite commands through explicit policies and the vault stack.
- Browser/web: keep at 34% unless this task changes browser code.
- Desktop app: keep at 35% unless this task changes desktop code.
- Overall: 52% -> 55%, because the automation surface gained a second real tabular workflow.
- Missing items must still list CLI vault/decode/audit/PDF/DICOM/conservative-media commands, deeper browser upload/download UX, and deeper desktop workflows.

- [x] **Step 9: Commit**

Run:

```bash
git add crates/mdid-cli/Cargo.toml crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs README.md docs/superpowers/plans/2026-04-29-cli-xlsx-deidentify-command.md Cargo.lock
git commit -m "feat(cli): add bounded xlsx deidentify command"
```
