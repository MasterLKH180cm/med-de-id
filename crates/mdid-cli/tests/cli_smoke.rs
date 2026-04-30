use assert_cmd::Command;
use dicom_core::{Tag, VR};
use dicom_object::{meta::FileMetaTableBuilder, InMemDicomObject};
use mdid_adapters::XlsxTabularAdapter;
use predicates::prelude::*;
use rust_xlsxwriter::Workbook;
use serde_json::Value;
use std::{fs, path::Path, time::Duration};

use tempfile::tempdir;

fn repo_path(relative: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
        .to_string_lossy()
        .to_string()
}

fn default_python_command() -> &'static str {
    if cfg!(windows) {
        "py"
    } else {
        "python3"
    }
}

fn write_xlsx(path: &Path) {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    worksheet.write_string(0, 0, "patient_name").unwrap();
    worksheet.write_string(0, 1, "note").unwrap();
    worksheet.write_string(1, 0, "Alice Patient").unwrap();
    worksheet.write_string(1, 1, "needs follow-up").unwrap();
    workbook.save(path).unwrap();
}

fn dicom_fixture() -> Vec<u8> {
    let mut obj = InMemDicomObject::new_empty();
    obj.put_str(Tag(0x0008, 0x0016), VR::UI, "1.2.840.10008.5.1.4.1.1.7");
    obj.put_str(
        Tag(0x0008, 0x0018),
        VR::UI,
        "2.25.123456789012345678901234567890123456",
    );
    obj.put_str(
        Tag(0x0020, 0x000D),
        VR::UI,
        "2.25.123456789012345678901234567890123457",
    );
    obj.put_str(
        Tag(0x0020, 0x000E),
        VR::UI,
        "2.25.123456789012345678901234567890123458",
    );
    obj.put_str(Tag(0x0010, 0x0010), VR::PN, "Alice^Smith");
    obj.put_str(Tag(0x0010, 0x0020), VR::LO, "MRN-001");
    obj.put_str(Tag(0x0028, 0x0301), VR::CS, "NO");

    let file_obj = obj
        .with_meta(FileMetaTableBuilder::new().transfer_syntax("1.2.840.10008.1.2.1"))
        .unwrap();
    let mut bytes = Vec::new();
    file_obj.write_all(&mut bytes).unwrap();
    bytes
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
fn cli_deidentify_dicom_writes_rewritten_dicom_and_phi_safe_summary() {
    let dir = tempdir().unwrap();
    let input_path = dir.path().join("Alice-Smith-source.dcm");
    let output_path = dir.path().join("output.dcm");
    let vault_path = dir.path().join("vault.json");
    fs::write(&input_path, dicom_fixture()).unwrap();

    let stdout = Command::cargo_bin("mdid-cli")
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
        .stdout(predicate::str::contains("MRN-001").not())
        .stdout(predicate::str::contains(input_path.to_string_lossy().as_ref()).not())
        .stdout(predicate::str::contains("correct horse battery staple").not())
        .get_output()
        .stdout
        .clone();

    assert!(output_path.exists());
    let rewritten = fs::read(&output_path).unwrap();
    assert!(!rewritten.is_empty());
    assert_ne!(rewritten, fs::read(&input_path).unwrap());

    let payload: Value = serde_json::from_slice(&stdout).unwrap();
    assert_eq!(
        payload["output_path"],
        output_path.to_string_lossy().to_string()
    );
    assert_eq!(payload["sanitized_file_name"], "dicom-output.dcm");
    assert_eq!(payload["review_queue_len"], 0);
    assert!(payload.get("summary").is_some());
    assert_eq!(payload.as_object().unwrap().len(), 4);
}

#[test]
fn cli_deidentify_dicom_rejects_invalid_dicom_without_scope_drift_terms() {
    let dir = tempdir().unwrap();
    let input_path = dir.path().join("invalid.dcm");
    let output_path = dir.path().join("output.dcm");
    let vault_path = dir.path().join("vault.json");
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

#[test]
fn verify_artifacts_exits_nonzero_when_artifact_is_missing() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let missing_path = temp_dir.path().join("missing-output.json");
    let paths_json = serde_json::to_string(&vec![missing_path.to_string_lossy().to_string()])
        .expect("paths json");

    let assert = Command::cargo_bin("mdid-cli")
        .expect("binary")
        .args(["verify-artifacts", "--artifact-paths-json", &paths_json])
        .assert()
        .failure();

    let output = assert.get_output();
    let stdout = String::from_utf8(output.stdout.clone()).expect("stdout utf8");
    let report: serde_json::Value = serde_json::from_str(stdout.trim()).expect("json report");

    assert_eq!(report["artifact_count"], 1);
    assert_eq!(report["existing_count"], 0);
    assert_eq!(report["missing_count"], 1);
    assert_eq!(report["oversized_count"], 0);
    assert_eq!(report["artifacts"][0]["index"], 0);
    assert_eq!(report["artifacts"][0]["exists"], false);
    assert!(!stdout.contains("missing-output.json"));
}

#[test]
fn verify_artifacts_exits_nonzero_when_artifact_exceeds_max_bytes() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let artifact_path = temp_dir.path().join("large-output.json");
    std::fs::write(&artifact_path, b"abcdef").expect("write artifact");
    let paths_json = serde_json::to_string(&vec![artifact_path.to_string_lossy().to_string()])
        .expect("paths json");

    let assert = Command::cargo_bin("mdid-cli")
        .expect("binary")
        .args([
            "verify-artifacts",
            "--artifact-paths-json",
            &paths_json,
            "--max-bytes",
            "3",
        ])
        .assert()
        .failure();

    let output = assert.get_output();
    let stdout = String::from_utf8(output.stdout.clone()).expect("stdout utf8");
    let report: serde_json::Value = serde_json::from_str(stdout.trim()).expect("json report");

    assert_eq!(report["artifact_count"], 1);
    assert_eq!(report["existing_count"], 1);
    assert_eq!(report["missing_count"], 0);
    assert_eq!(report["oversized_count"], 1);
    assert_eq!(report["max_bytes"], 3);
    assert_eq!(report["artifacts"][0]["byte_len"], 6);
    assert_eq!(report["artifacts"][0]["within_max_bytes"], false);
    assert!(!stdout.contains("large-output.json"));
}

#[test]
fn cli_review_media_writes_phi_safe_report() {
    let dir = tempdir().unwrap();
    let report_path = dir.path().join("media-review.json");

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "review-media",
            "--artifact-label",
            "patient-alice-scan.png",
            "--format",
            "image",
            "--metadata-json",
            r#"[{"key":"DeviceSerialNumber","value":"ABC123"}]"#,
            "--requires-visual-review",
            "false",
            "--unsupported-payload",
            "false",
            "--report-path",
            report_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("patient-alice").not())
        .stdout(predicate::str::contains("ABC123").not());

    let report_text = fs::read_to_string(&report_path).unwrap();
    assert!(!report_text.contains("patient-alice"));
    assert!(!report_text.contains("ABC123"));
    assert!(report_text.contains("candidate_index"));
    assert!(!report_text.contains("field_path"));
    let report: Value = serde_json::from_str(&report_text).unwrap();
    assert_eq!(report["summary"]["metadata_only_items"], 1);
    assert_eq!(report["review_queue_len"], 1);
    assert_eq!(report["review_queue"][0]["candidate_index"], 0);
    assert!(report["review_queue"][0].get("field_path").is_none());
    assert!(report["rewritten_media_bytes"].is_null());
}

#[test]
fn cli_review_media_rejects_blank_artifact_label() {
    let dir = tempdir().unwrap();
    let report_path = dir.path().join("media-review.json");

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "review-media",
            "--artifact-label",
            "   ",
            "--format",
            "image",
            "--metadata-json",
            r#"[{"key":"DeviceSerialNumber","value":"ABC123"}]"#,
            "--requires-visual-review",
            "false",
            "--unsupported-payload",
            "false",
            "--report-path",
            report_path.to_str().unwrap(),
        ])
        .assert()
        .failure();
}

#[test]
fn privacy_filter_text_runs_repo_fixture_runner_and_validator() {
    let dir = tempdir().unwrap();
    let report_path = dir.path().join("privacy-filter-report.json");
    let input_path = repo_path("scripts/privacy_filter/fixtures/sample_text_input.txt");
    let runner_path = repo_path("scripts/privacy_filter/run_privacy_filter.py");
    let validator_path = repo_path("scripts/privacy_filter/validate_privacy_filter_output.py");

    let assert = Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("privacy-filter-text")
        .arg("--input-path")
        .arg(&input_path)
        .arg("--runner-path")
        .arg(&runner_path)
        .arg("--report-path")
        .arg(&report_path)
        .arg("--python-command")
        .arg(default_python_command())
        .arg("--mock")
        .assert()
        .success();

    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let raw_fixture_values = [
        "Jane Example",
        "555-0100",
        "555-123-4567",
        "jane@example.com",
        "+1-555-123-4567",
        "MRN-12345",
    ];
    for raw_pii in raw_fixture_values {
        assert!(
            !stdout.contains(raw_pii),
            "stdout leaked raw PII: {raw_pii}"
        );
        assert!(
            !stderr.contains(raw_pii),
            "stderr leaked raw PII: {raw_pii}"
        );
    }
    assert!(report_path.exists());
    let report_text = fs::read_to_string(&report_path).unwrap();
    for raw_pii in raw_fixture_values {
        assert!(
            !report_text.contains(raw_pii),
            "report leaked raw PII: {raw_pii}"
        );
    }

    let validator = std::process::Command::new(default_python_command())
        .arg(&validator_path)
        .arg(&report_path)
        .output()
        .unwrap();
    assert!(
        validator.status.success(),
        "validator failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&validator.stdout),
        String::from_utf8_lossy(&validator.stderr)
    );
}

#[test]
fn privacy_filter_text_writes_verbatim_runner_json_report() {
    let dir = tempdir().unwrap();
    let input_path = dir.path().join("synthetic-input.txt");
    let runner_path = dir.path().join("privacy_runner.py");
    let report_path = dir.path().join("privacy-report.json");
    fs::write(&input_path, "Patient Jane Example has MRN-123\n").unwrap();
    fs::write(
        &runner_path,
        r#"import json, pathlib, sys
pathlib.Path(sys.argv[1]).read_text(encoding='utf-8')
print(json.dumps({"summary":{"detected_span_count":1},"masked_text":"Patient <PERSON> has <ID>","spans":[{"label":"PERSON","start":8,"end":20,"preview":"<redacted>"}],"metadata":{"engine":"fallback_synthetic_patterns","network_api_called":False,"preview_policy":"redacted_placeholders_only"}}, indent=2))
"#,
    )
    .unwrap();

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("privacy-filter-text")
        .arg("--input-path")
        .arg(&input_path)
        .arg("--runner-path")
        .arg(&runner_path)
        .arg("--report-path")
        .arg(&report_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("privacy-filter-text"))
        .stdout(predicate::str::contains("Jane Example").not())
        .stderr(predicate::str::contains("Jane Example").not());

    let report_text = fs::read_to_string(&report_path).unwrap();
    assert!(report_text.contains("\"network_api_called\": false"));
    assert!(report_text.contains("Patient <PERSON> has <ID>"));
    assert!(!report_text.contains("Jane Example"));
}

fn write_privacy_runner(path: &Path, body: &str) {
    fs::write(path, body).unwrap();
}

#[test]
fn privacy_filter_text_uses_explicit_python_command_when_provided() {
    let dir = tempdir().unwrap();
    let input_path = dir.path().join("synthetic-input.txt");
    let runner_path = dir.path().join("privacy_runner.py");
    let report_path = dir.path().join("privacy-report.json");
    let python_command = dir.path().join("fake-python");
    let argv_path = dir.path().join("argv.txt");
    fs::write(&input_path, "Patient Jane Example has MRN-123\n").unwrap();
    fs::write(&runner_path, "not executed by a real python in this test\n").unwrap();
    fs::write(
        &python_command,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > {}\nprintf '%s\\n' '{{\"summary\":{{}},\"masked_text\":\"Patient <PERSON>\",\"spans\":[],\"metadata\":{{\"engine\":\"fallback_synthetic_patterns\",\"network_api_called\":false,\"preview_policy\":\"redacted_placeholders_only\"}}}}'\n",
            argv_path.display()
        ),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&python_command).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&python_command, perms).unwrap();
    }

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("privacy-filter-text")
        .arg("--input-path")
        .arg(&input_path)
        .arg("--runner-path")
        .arg(&runner_path)
        .arg("--report-path")
        .arg(&report_path)
        .arg("--python-command")
        .arg(&python_command)
        .assert()
        .success();

    let argv = fs::read_to_string(argv_path).unwrap();
    assert!(argv.contains(runner_path.to_str().unwrap()));
    assert!(argv.contains(input_path.to_str().unwrap()));
}

#[test]
fn privacy_filter_text_mock_flag_forwards_mock_to_runner() {
    let dir = tempdir().unwrap();
    let input_path = dir.path().join("synthetic-input.txt");
    let runner_path = dir.path().join("privacy_runner.py");
    let report_path = dir.path().join("privacy-report.json");
    let argv_path = dir.path().join("argv.txt");
    fs::write(&input_path, "Patient Jane Example has MRN-123\n").unwrap();
    fs::write(
        &runner_path,
        format!(
            r#"import json, pathlib, sys
pathlib.Path({argv_path:?}).write_text('\n'.join(sys.argv[1:]), encoding='utf-8')
print(json.dumps({{"summary":{{}},"masked_text":"Patient <PERSON>","spans":[],"metadata":{{"engine":"fallback_synthetic_patterns","network_api_called":False,"preview_policy":"redacted_placeholders_only"}}}}))
"#,
            argv_path = argv_path
        ),
    )
    .unwrap();

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("privacy-filter-text")
        .arg("--input-path")
        .arg(&input_path)
        .arg("--runner-path")
        .arg(&runner_path)
        .arg("--report-path")
        .arg(&report_path)
        .arg("--mock")
        .assert()
        .success();

    let argv = fs::read_to_string(argv_path).unwrap();
    assert!(argv.lines().any(|arg| arg == "--mock"));
    assert!(argv.lines().any(|arg| arg == input_path.to_str().unwrap()));
}

#[test]
fn privacy_filter_text_rejects_oversized_runner_stdout_without_writing_report() {
    let dir = tempdir().unwrap();
    let input_path = dir.path().join("synthetic-input.txt");
    let runner_path = dir.path().join("privacy_runner.py");
    let report_path = dir.path().join("privacy-report.json");
    fs::write(&input_path, "Patient Jane Example has MRN-123\n").unwrap();
    write_privacy_runner(
        &runner_path,
        "import sys, time\nsys.stdout.write('x' * (1024 * 1024 + 1))\nsys.stdout.flush()\ntime.sleep(10)\n",
    );

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .timeout(Duration::from_secs(3))
        .arg("privacy-filter-text")
        .arg("--input-path")
        .arg(&input_path)
        .arg("--runner-path")
        .arg(&runner_path)
        .arg("--report-path")
        .arg(&report_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("runner output exceeded limit"))
        .stderr(predicate::str::contains("Jane Example").not())
        .stdout(predicate::str::contains("Jane Example").not());

    assert!(!report_path.exists());
}

#[test]
fn privacy_filter_text_times_out_silent_hanging_runner_and_removes_stale_report() {
    let dir = tempdir().unwrap();
    let input_path = dir.path().join("synthetic-input.txt");
    let runner_path = dir.path().join("privacy_runner.py");
    let report_path = dir.path().join("privacy-report.json");
    fs::write(&input_path, "Patient Jane Example has MRN-123\n").unwrap();
    fs::write(&report_path, "stale report must be removed").unwrap();
    write_privacy_runner(&runner_path, "import time\ntime.sleep(30)\n");

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .timeout(Duration::from_secs(5))
        .arg("privacy-filter-text")
        .arg("--input-path")
        .arg(&input_path)
        .arg("--runner-path")
        .arg(&runner_path)
        .arg("--report-path")
        .arg(&report_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("privacy filter runner timed out"))
        .stderr(predicate::str::contains("Jane Example").not())
        .stdout(predicate::str::contains("Jane Example").not());

    assert!(!report_path.exists());
}

fn assert_privacy_filter_rejects(
    input_path: &Path,
    runner_path: &Path,
    report_path: &Path,
    expected_error: &str,
) {
    Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("privacy-filter-text")
        .arg("--input-path")
        .arg(input_path)
        .arg("--runner-path")
        .arg(runner_path)
        .arg("--report-path")
        .arg(report_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains(expected_error))
        .stderr(predicate::str::contains("Jane Example").not())
        .stdout(predicate::str::contains("Jane Example").not());

    assert!(!report_path.exists());
}

#[test]
fn privacy_filter_text_rejects_missing_required_flags_and_blank_paths() {
    let dir = tempdir().unwrap();
    let input_path = dir.path().join("synthetic-input.txt");
    let runner_path = dir.path().join("privacy_runner.py");
    let report_path = dir.path().join("privacy-report.json");
    fs::write(&input_path, "Patient Jane Example has MRN-123\n").unwrap();
    write_privacy_runner(&runner_path, "print('{}')\n");

    for (args, expected_error) in [
        (
            vec![
                "privacy-filter-text",
                "--runner-path",
                runner_path.to_str().unwrap(),
                "--report-path",
                report_path.to_str().unwrap(),
            ],
            "missing --input-path",
        ),
        (
            vec![
                "privacy-filter-text",
                "--input-path",
                input_path.to_str().unwrap(),
                "--report-path",
                report_path.to_str().unwrap(),
            ],
            "missing --runner-path",
        ),
        (
            vec![
                "privacy-filter-text",
                "--input-path",
                input_path.to_str().unwrap(),
                "--runner-path",
                runner_path.to_str().unwrap(),
            ],
            "missing --report-path",
        ),
        (
            vec![
                "privacy-filter-text",
                "--input-path",
                "   ",
                "--runner-path",
                runner_path.to_str().unwrap(),
                "--report-path",
                report_path.to_str().unwrap(),
            ],
            "missing --input-path",
        ),
        (
            vec![
                "privacy-filter-text",
                "--input-path",
                input_path.to_str().unwrap(),
                "--runner-path",
                "   ",
                "--report-path",
                report_path.to_str().unwrap(),
            ],
            "missing --runner-path",
        ),
        (
            vec![
                "privacy-filter-text",
                "--input-path",
                input_path.to_str().unwrap(),
                "--runner-path",
                runner_path.to_str().unwrap(),
                "--report-path",
                "   ",
            ],
            "missing --report-path",
        ),
    ] {
        Command::cargo_bin("mdid-cli")
            .unwrap()
            .args(args)
            .assert()
            .failure()
            .stderr(predicate::str::contains(expected_error))
            .stderr(predicate::str::contains("Jane Example").not())
            .stdout(predicate::str::contains("Jane Example").not());
    }

    assert!(!report_path.exists());
}

#[test]
fn privacy_filter_text_rejects_missing_files_runner_failure_and_invalid_json_without_raw_text() {
    let dir = tempdir().unwrap();
    let input_path = dir.path().join("synthetic-input.txt");
    let missing_input_path = dir.path().join("missing-input.txt");
    let missing_runner_path = dir.path().join("missing-runner.py");
    let failing_runner_path = dir.path().join("failing-runner.py");
    let bad_runner_path = dir.path().join("bad-runner.py");
    let report_path = dir.path().join("privacy-report.json");
    fs::write(&input_path, "Patient Jane Example has MRN-123\n").unwrap();

    assert_privacy_filter_rejects(
        &missing_input_path,
        &bad_runner_path,
        &report_path,
        "missing input file",
    );

    assert_privacy_filter_rejects(
        &input_path,
        &missing_runner_path,
        &report_path,
        "missing runner file",
    );

    write_privacy_runner(&failing_runner_path, "import sys\nsys.exit(7)\n");
    fs::write(&report_path, "stale report must be removed").unwrap();
    assert_privacy_filter_rejects(
        &input_path,
        &failing_runner_path,
        &report_path,
        "privacy filter runner failed",
    );

    write_privacy_runner(&bad_runner_path, "print('not json')\n");
    assert_privacy_filter_rejects(
        &input_path,
        &bad_runner_path,
        &report_path,
        "runner returned non-JSON output",
    );
}

#[test]
fn privacy_filter_text_rejects_incomplete_or_invalid_runner_payloads() {
    let dir = tempdir().unwrap();
    let input_path = dir.path().join("synthetic-input.txt");
    let runner_path = dir.path().join("privacy_runner.py");
    let report_path = dir.path().join("privacy-report.json");
    fs::write(&input_path, "Patient Jane Example has MRN-123\n").unwrap();

    let valid_payload = r#"{"summary":{},"masked_text":"Patient <PERSON>","spans":[],"metadata":{"engine":"fallback_synthetic_patterns","network_api_called":false,"preview_policy":"redacted_placeholders_only"}}"#;
    for (payload, expected_error) in [
        (
            r#"{"masked_text":"x","spans":[],"metadata":{"engine":"fallback_synthetic_patterns","network_api_called":false,"preview_policy":"redacted_placeholders_only"}}"#,
            "privacy filter output missing required field",
        ),
        (
            r#"{"summary":{},"spans":[],"metadata":{"engine":"fallback_synthetic_patterns","network_api_called":false,"preview_policy":"redacted_placeholders_only"}}"#,
            "privacy filter output missing required field",
        ),
        (
            r#"{"summary":{},"masked_text":"x","metadata":{"engine":"fallback_synthetic_patterns","network_api_called":false,"preview_policy":"redacted_placeholders_only"}}"#,
            "privacy filter output missing required field",
        ),
        (
            r#"{"summary":{},"masked_text":"x","spans":[]}"#,
            "privacy filter output missing required field",
        ),
        (
            r#"{"summary":{},"masked_text":"x","spans":[],"metadata":{"engine":"fallback_synthetic_patterns","network_api_called":true,"preview_policy":"redacted_placeholders_only"}}"#,
            "privacy filter output indicates network API use",
        ),
        (r#"[]"#, "privacy filter output must be a JSON object"),
        (
            r#"{"summary":[],"masked_text":"x","spans":[],"metadata":{"engine":"fallback_synthetic_patterns","network_api_called":false,"preview_policy":"redacted_placeholders_only"}}"#,
            "privacy filter output has invalid required field shape",
        ),
        (
            r#"{"summary":{},"masked_text":[],"spans":[],"metadata":{"engine":"fallback_synthetic_patterns","network_api_called":false,"preview_policy":"redacted_placeholders_only"}}"#,
            "privacy filter output has invalid required field shape",
        ),
        (
            r#"{"summary":{},"masked_text":"x","spans":{},"metadata":{"engine":"fallback_synthetic_patterns","network_api_called":false,"preview_policy":"redacted_placeholders_only"}}"#,
            "privacy filter output has invalid required field shape",
        ),
        (
            r#"{"summary":{},"masked_text":"x","spans":[],"metadata":[]}"#,
            "privacy filter output has invalid required field shape",
        ),
    ] {
        let python_payload = serde_json::to_string(payload).unwrap();
        write_privacy_runner(&runner_path, &format!("print({python_payload})\n"));
        assert_privacy_filter_rejects(&input_path, &runner_path, &report_path, expected_error);
    }

    let python_payload = serde_json::to_string(valid_payload).unwrap();
    write_privacy_runner(&runner_path, &format!("print({python_payload})\n"));
    Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("privacy-filter-text")
        .arg("--input-path")
        .arg(&input_path)
        .arg("--runner-path")
        .arg(&runner_path)
        .arg("--report-path")
        .arg(&report_path)
        .assert()
        .success();
}

#[test]
fn ocr_handoff_help_mentions_command() {
    Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("--help")
        .assert()
        .failure()
        .stderr(predicate::str::contains("ocr-handoff"));
}

#[test]
fn ocr_handoff_missing_flags_and_files_are_rejected() {
    Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("ocr-handoff")
        .assert()
        .failure()
        .stderr(predicate::str::contains("missing --image-path"));

    let dir = tempdir().unwrap();
    let runner_path = dir.path().join("runner.py");
    let builder_path = dir.path().join("builder.py");
    let report_path = dir.path().join("handoff.json");
    fs::write(&runner_path, "print('Jane Doe')\n").unwrap();
    fs::write(&builder_path, "print('ok')\n").unwrap();

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-handoff",
            "--image-path",
            dir.path().join("missing.png").to_str().unwrap(),
            "--ocr-runner-path",
            runner_path.to_str().unwrap(),
            "--handoff-builder-path",
            builder_path.to_str().unwrap(),
            "--report-path",
            report_path.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("missing image file"));
}

#[test]
fn ocr_handoff_success_with_synthetic_fixture() {
    let dir = tempdir().unwrap();
    let report_path = dir.path().join("ocr-handoff.json");

    let stdout = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-handoff",
            "--image-path",
            &repo_path("scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png"),
            "--ocr-runner-path",
            &repo_path("scripts/ocr_eval/run_small_ocr.py"),
            "--handoff-builder-path",
            &repo_path("scripts/ocr_eval/build_ocr_handoff.py"),
            "--report-path",
            report_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("ocr-handoff"))
        .get_output()
        .stdout
        .clone();
    let summary: Value = serde_json::from_slice(&stdout).unwrap();
    assert_eq!(summary["command"], "ocr-handoff");
    assert_eq!(
        summary["report_path"],
        report_path.to_string_lossy().to_string()
    );

    let report: Value = serde_json::from_str(&fs::read_to_string(&report_path).unwrap()).unwrap();
    assert_eq!(report["source"], "synthetic_printed_phi_line.png");
    assert_eq!(report["candidate"], "PP-OCRv5_mobile_rec");
    assert_eq!(report["scope"], "printed_text_line_extraction_only");
    assert_eq!(
        report["privacy_filter_contract"],
        "text_only_normalized_input"
    );
    assert_eq!(report["ready_for_text_pii_eval"], true);
    assert!(report["normalized_text"]
        .as_str()
        .unwrap()
        .contains("Jane Example"));
    assert!(!report_path.with_extension("json.ocr-text.tmp").exists());
}

#[test]
fn cli_ocr_handoff_normalized_text_feeds_privacy_filter_without_phi_leaks() {
    let dir = tempdir().unwrap();
    let handoff_report = dir.path().join("ocr-handoff.json");
    let normalized_text = dir.path().join("ocr-normalized.txt");
    let privacy_report = dir.path().join("privacy-filter.json");

    let raw_fixture_values = [
        "Jane Example",
        "jane@example.com",
        "+1-555-123-4567",
        "MRN-12345",
    ];

    let ocr_output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("ocr-handoff")
        .arg("--image-path")
        .arg(repo_path(
            "scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png",
        ))
        .arg("--ocr-runner-path")
        .arg(repo_path("scripts/ocr_eval/run_small_ocr.py"))
        .arg("--handoff-builder-path")
        .arg(repo_path("scripts/ocr_eval/build_ocr_handoff.py"))
        .arg("--report-path")
        .arg(&handoff_report)
        .arg("--python-command")
        .arg(default_python_command())
        .assert()
        .success()
        .get_output()
        .clone();

    let ocr_stdout = String::from_utf8_lossy(&ocr_output.stdout);
    let ocr_stderr = String::from_utf8_lossy(&ocr_output.stderr);
    for raw_pii in raw_fixture_values {
        assert!(
            !ocr_stdout.contains(raw_pii),
            "OCR stdout leaked raw PII: {raw_pii}"
        );
        assert!(
            !ocr_stderr.contains(raw_pii),
            "OCR stderr leaked raw PII: {raw_pii}"
        );
    }
    let ocr_output = ocr_output.stdout.clone();

    let ocr_summary: Value = serde_json::from_slice(&ocr_output).unwrap();
    assert_eq!(ocr_summary["ready_for_text_pii_eval"], true);
    assert_eq!(
        ocr_summary["privacy_filter_contract"],
        "text_only_normalized_input"
    );

    let handoff: Value =
        serde_json::from_str(&fs::read_to_string(&handoff_report).unwrap()).unwrap();
    let text = handoff["normalized_text"].as_str().unwrap();
    assert!(text.contains("Jane Example"));
    assert!(text.contains("MRN-12345"));
    fs::write(&normalized_text, text).unwrap();

    let privacy_output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("privacy-filter-text")
        .arg("--input-path")
        .arg(&normalized_text)
        .arg("--runner-path")
        .arg(repo_path("scripts/privacy_filter/run_privacy_filter.py"))
        .arg("--report-path")
        .arg(&privacy_report)
        .arg("--python-command")
        .arg(default_python_command())
        .assert()
        .success()
        .get_output()
        .clone();

    let privacy_stdout = String::from_utf8_lossy(&privacy_output.stdout);
    let privacy_stderr = String::from_utf8_lossy(&privacy_output.stderr);
    for raw_pii in raw_fixture_values {
        assert!(
            !privacy_stdout.contains(raw_pii),
            "Privacy Filter stdout leaked raw PII: {raw_pii}"
        );
        assert!(
            !privacy_stderr.contains(raw_pii),
            "Privacy Filter stderr leaked raw PII: {raw_pii}"
        );
    }
    let privacy_output = privacy_output.stdout.clone();

    let privacy_summary: Value = serde_json::from_slice(&privacy_output).unwrap();
    assert_eq!(privacy_summary["engine"], "fallback_synthetic_patterns");
    assert_eq!(privacy_summary["network_api_called"], false);
    assert!(privacy_summary["detected_span_count"].as_u64().unwrap() >= 2);

    let privacy_json = fs::read_to_string(&privacy_report).unwrap();
    for raw_pii in raw_fixture_values {
        assert!(
            !privacy_json.contains(raw_pii),
            "Privacy Filter report leaked raw PII: {raw_pii}"
        );
    }
    assert!(privacy_json.contains("[NAME]"));
    assert!(privacy_json.contains("[EMAIL]"));
    assert!(privacy_json.contains("[PHONE]"));
    assert!(privacy_json.contains("[MRN]"));
}

#[test]
fn ocr_handoff_removes_stale_report_when_ocr_runner_fails() {
    let dir = tempdir().unwrap();
    let image_path = dir.path().join("image.png");
    let runner_path = dir.path().join("runner.py");
    let builder_path = dir.path().join("builder.py");
    let report_path = dir.path().join("handoff.json");
    fs::write(&image_path, b"png").unwrap();
    fs::write(&runner_path, "import sys\nsys.exit(7)\n").unwrap();
    fs::write(&builder_path, "print('not reached')\n").unwrap();
    fs::write(&report_path, r#"{"stale":true}"#).unwrap();

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-handoff",
            "--image-path",
            image_path.to_str().unwrap(),
            "--ocr-runner-path",
            runner_path.to_str().unwrap(),
            "--handoff-builder-path",
            builder_path.to_str().unwrap(),
            "--report-path",
            report_path.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("OCR runner failed"));
    assert!(!report_path.exists());
}

#[test]
fn ocr_handoff_removes_stale_report_when_ocr_runner_emits_non_utf8() {
    let dir = tempdir().unwrap();
    let image_path = dir.path().join("image.png");
    let runner_path = dir.path().join("runner.py");
    let builder_path = dir.path().join("builder.py");
    let report_path = dir.path().join("handoff.json");
    fs::write(&image_path, b"png").unwrap();
    fs::write(
        &runner_path,
        "import sys\nsys.stdout.buffer.write(b'\\xff\\xfe')\n",
    )
    .unwrap();
    fs::write(&builder_path, "print('not reached')\n").unwrap();
    fs::write(&report_path, "stale report").unwrap();

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-handoff",
            "--image-path",
            image_path.to_str().unwrap(),
            "--ocr-runner-path",
            runner_path.to_str().unwrap(),
            "--handoff-builder-path",
            builder_path.to_str().unwrap(),
            "--report-path",
            report_path.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "OCR runner returned non-UTF-8 output",
        ));
    assert!(!report_path.exists());
}

#[test]
fn ocr_handoff_rejects_invalid_builder_contract_and_removes_report() {
    let dir = tempdir().unwrap();
    let image_path = dir.path().join("image.png");
    let runner_path = dir.path().join("runner.py");
    let builder_path = dir.path().join("builder.py");
    let report_path = dir.path().join("handoff.json");
    fs::write(&image_path, b"png").unwrap();
    fs::write(&runner_path, "print('Jane Doe')\n").unwrap();
    fs::write(&builder_path, "import pathlib, sys\npathlib.Path(sys.argv[sys.argv.index('--output')+1]).write_text('{}')\n").unwrap();

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-handoff",
            "--image-path",
            image_path.to_str().unwrap(),
            "--ocr-runner-path",
            runner_path.to_str().unwrap(),
            "--handoff-builder-path",
            builder_path.to_str().unwrap(),
            "--report-path",
            report_path.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "OCR handoff missing required field",
        ));
    assert!(!report_path.exists());
}

#[test]
fn ocr_handoff_rejects_malformed_builder_report_and_removes_report() {
    let dir = tempdir().unwrap();
    let image_path = dir.path().join("image.png");
    let runner_path = dir.path().join("runner.py");
    let builder_path = dir.path().join("builder.py");
    let report_path = dir.path().join("handoff.json");
    fs::write(&image_path, b"png").unwrap();
    fs::write(&runner_path, "print('Jane Doe')\n").unwrap();
    fs::write(&builder_path, "import pathlib, sys\npathlib.Path(sys.argv[sys.argv.index('--output')+1]).write_text('not json')\n").unwrap();

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-handoff",
            "--image-path",
            image_path.to_str().unwrap(),
            "--ocr-runner-path",
            runner_path.to_str().unwrap(),
            "--handoff-builder-path",
            builder_path.to_str().unwrap(),
            "--report-path",
            report_path.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "OCR handoff report is not valid JSON",
        ));
    assert!(!report_path.exists());
}

#[test]
fn ocr_handoff_rejects_oversized_ocr_stdout_without_writing_report() {
    let dir = tempdir().unwrap();
    let image_path = dir.path().join("image.png");
    let runner_path = dir.path().join("runner.py");
    let builder_path = dir.path().join("builder.py");
    let report_path = dir.path().join("handoff.json");
    fs::write(&image_path, b"png").unwrap();
    fs::write(&runner_path, "import sys, time\nsys.stdout.write('x' * (1024 * 1024 + 1))\nsys.stdout.flush()\ntime.sleep(10)\n").unwrap();
    fs::write(&builder_path, "print('not reached')\n").unwrap();

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .timeout(Duration::from_secs(3))
        .args([
            "ocr-handoff",
            "--image-path",
            image_path.to_str().unwrap(),
            "--ocr-runner-path",
            runner_path.to_str().unwrap(),
            "--handoff-builder-path",
            builder_path.to_str().unwrap(),
            "--report-path",
            report_path.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("OCR runner output exceeded limit"));
    assert!(!report_path.exists());
}

#[test]
fn ocr_handoff_times_out_silent_hanging_runner_and_removes_stale_report() {
    let dir = tempdir().unwrap();
    let image_path = dir.path().join("image.png");
    let runner_path = dir.path().join("runner.py");
    let builder_path = dir.path().join("builder.py");
    let report_path = dir.path().join("handoff.json");
    fs::write(&image_path, b"png").unwrap();
    fs::write(&runner_path, "import time\ntime.sleep(30)\n").unwrap();
    fs::write(&builder_path, "print('not reached')\n").unwrap();
    fs::write(&report_path, "stale report must be removed").unwrap();

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .timeout(Duration::from_secs(5))
        .args([
            "ocr-handoff",
            "--image-path",
            image_path.to_str().unwrap(),
            "--ocr-runner-path",
            runner_path.to_str().unwrap(),
            "--handoff-builder-path",
            builder_path.to_str().unwrap(),
            "--report-path",
            report_path.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("OCR runner timed out"));
    assert!(!report_path.exists());
}

#[test]
fn docs_ocr_privacy_chain_uses_handoff_normalized_text_file() {
    let repo_readme = fs::read_to_string(repo_path("README.md")).unwrap();
    let ocr_readme = fs::read_to_string(repo_path("scripts/ocr_eval/README.md")).unwrap();

    for docs in [&repo_readme, &ocr_readme] {
        assert!(docs.contains("/tmp/ocr-normalized-text.txt"));
        assert!(docs.contains("Path('/tmp/ocr-handoff.json')"));
        assert!(docs.contains("['normalized_text']"));
        assert!(docs.contains("python scripts/privacy_filter/run_privacy_filter.py --mock /tmp/ocr-normalized-text.txt"));
    }
    assert!(!repo_readme.contains("run_privacy_filter.py --mock /tmp/small-ocr-output.txt"));
    assert!(!ocr_readme.contains("run_privacy_filter.py --mock /tmp/small-ocr-output.txt"));
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
        .stderr(predicate::str::contains("review-media"))
        .stderr(predicate::str::contains("privacy-filter-text"))
        .stderr(predicate::str::contains("ocr-handoff"))
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
