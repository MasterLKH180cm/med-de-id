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
fn cli_prints_ready_banner_with_no_args() {
    let mut cmd = Command::cargo_bin("mdid-cli").unwrap();

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("med-de-id CLI ready"));
}

#[test]
fn cli_prints_status_banner() {
    let mut cmd = Command::cargo_bin("mdid-cli").unwrap();

    cmd.arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("med-de-id CLI ready"));
}

#[test]
fn cli_deidentify_csv_writes_rewritten_csv_and_phi_safe_summary() {
    let dir = tempdir().unwrap();
    let input_path = dir.path().join("input.csv");
    let output_path = dir.path().join("output.csv");
    let vault_path = dir.path().join("vault.json");
    fs::write(&input_path, "name,notes\nAlice,called\nAlice,follow up\n").unwrap();

    let assert = Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("deidentify-csv")
        .arg("--csv-path")
        .arg(&input_path)
        .arg("--policies-json")
        .arg(r#"[{"header":"name","phi_type":"name","action":"encode"}]"#)
        .arg("--vault-path")
        .arg(&vault_path)
        .arg("--passphrase")
        .arg("correct horse battery staple")
        .arg("--output-path")
        .arg(&output_path)
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""total_rows":2"#))
        .stdout(predicate::str::contains(r#""encoded_cells":2"#))
        .stdout(predicate::str::contains(r#""review_queue_len":0"#))
        .stdout(predicate::str::contains("Alice").not());

    let _ = assert;
    let output_csv = fs::read_to_string(&output_path).unwrap();
    assert!(output_csv.contains("tok-"));
    assert!(!output_csv.contains("Alice"));
    assert!(vault_path.exists());
}

#[test]
fn cli_deidentify_csv_reports_review_queue_count_without_printing_phi() {
    let dir = tempdir().unwrap();
    let input_path = dir.path().join("input.csv");
    let output_path = dir.path().join("output.csv");
    let vault_path = dir.path().join("vault.json");
    fs::write(&input_path, "name,notes\nBob,Call Alice today\n").unwrap();

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("deidentify-csv")
        .arg("--csv-path")
        .arg(&input_path)
        .arg("--policies-json")
        .arg(r#"[{"header":"notes","phi_type":"note","action":"review"}]"#)
        .arg("--vault-path")
        .arg(&vault_path)
        .arg("--passphrase")
        .arg("correct horse battery staple")
        .arg("--output-path")
        .arg(&output_path)
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""review_queue_len":1"#))
        .stdout(predicate::str::contains("Alice").not())
        .stdout(predicate::str::contains("Bob").not());

    let output_csv = fs::read_to_string(&output_path).unwrap();
    assert!(output_csv.contains("Call Alice today"));
}

#[test]
fn cli_deidentify_csv_rejects_malformed_policy_json_without_scope_drift_terms() {
    let dir = tempdir().unwrap();
    let input_path = dir.path().join("input.csv");
    let output_path = dir.path().join("output.csv");
    let vault_path = dir.path().join("vault.json");
    fs::write(&input_path, "name\nAlice\n").unwrap();

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("deidentify-csv")
        .arg("--csv-path")
        .arg(&input_path)
        .arg("--policies-json")
        .arg(r#"[{"header":"name","phi_type":"name","action":"delete"}]"#)
        .arg("--vault-path")
        .arg(&vault_path)
        .arg("--passphrase")
        .arg("correct horse battery staple")
        .arg("--output-path")
        .arg(&output_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid policies JSON"))
        .stderr(predicate::str::contains("moat").not())
        .stderr(predicate::str::contains("controller").not())
        .stderr(predicate::str::contains("agent").not());
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
    assert_eq!(
        payload["output_path"],
        output_path.to_string_lossy().to_string()
    );
    assert_eq!(payload["summary"]["total_rows"], 1);
    assert_eq!(payload["summary"]["encoded_cells"], 1);
    assert_eq!(payload["summary"]["review_required_cells"], 1);
    assert!(payload["summary"].get("processed_rows").is_none());
    assert!(payload["summary"].get("review_items").is_none());
    assert_eq!(payload["review_queue_len"], 1);

    let output_bytes = fs::read(&output_path).unwrap();
    let extracted = XlsxTabularAdapter::new(Vec::new())
        .extract(&output_bytes)
        .unwrap();
    assert_eq!(extracted.columns[0].name, "patient_name");
    assert_eq!(extracted.columns[1].name, "note");
    assert_eq!(extracted.rows.len(), 1);
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

#[test]
fn cli_usage_stays_deidentification_scoped() {
    let mut cmd = Command::cargo_bin("mdid-cli").unwrap();

    cmd.arg("--help")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage: mdid-cli [status]"))
        .stderr(predicate::str::contains(
            "local de-identification automation",
        ))
        .stderr(predicate::str::contains("moat").not())
        .stderr(predicate::str::contains("controller").not())
        .stderr(predicate::str::contains("agent").not());
}

#[test]
fn cli_rejects_scope_drift_controller_commands() {
    for args in [
        vec!["moat"],
        vec!["moat", "controller-plan", "--history-path", "history.json"],
        vec![
            "moat",
            "controller-step",
            "--history-path",
            "history.json",
            "--agent-id",
            "agent-1",
        ],
        vec!["controller-step"],
        vec!["claim"],
        vec!["complete_command"],
    ] {
        let mut cmd = Command::cargo_bin("mdid-cli").unwrap();

        cmd.args(args)
            .assert()
            .failure()
            .stderr(predicate::str::contains("unknown command"))
            .stderr(predicate::str::contains("Usage: mdid-cli [status]"))
            .stderr(predicate::str::contains("moat").not())
            .stderr(predicate::str::contains("controller").not())
            .stderr(predicate::str::contains("agent").not());
    }
}
