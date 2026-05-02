use std::{fs, process::Command};

use tempfile::tempdir;

fn text_layer_pdf_fixture() -> Vec<u8> {
    fs::read("../mdid-adapters/tests/fixtures/pdf/text-layer-minimal.pdf").unwrap()
}

fn clean_text_layer_pdf_fixture() -> Vec<u8> {
    let mut pdf = text_layer_pdf_fixture();
    let needle = b"Alice Smith";
    let offset = pdf
        .windows(needle.len())
        .position(|window| window == needle)
        .unwrap();
    pdf[offset..offset + needle.len()].copy_from_slice(b"ClinicNote ");
    pdf
}

#[test]
fn cli_deidentify_pdf_validates_clean_text_layer_output_pdf_without_path_or_phi_leaks() {
    let dir = tempdir().unwrap();
    let pdf_path = dir.path().join("clean-record.pdf");
    let report_path = dir.path().join("clean-report.json");
    let output_pdf_path = dir.path().join("clean-output.pdf");
    fs::write(&pdf_path, clean_text_layer_pdf_fixture()).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .arg("deidentify-pdf")
        .arg("--pdf-path")
        .arg(&pdf_path)
        .arg("--source-name")
        .arg("exported.pdf")
        .arg("--report-path")
        .arg(&report_path)
        .arg("--output-pdf-path")
        .arg(&output_pdf_path)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output_pdf_path.exists());
    let exported = fs::read(&output_pdf_path).unwrap();
    assert!(exported.starts_with(b"%PDF"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stdout_json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        stdout_json["rewrite_status"],
        "clean_text_layer_pdf_bytes_available"
    );
    assert_eq!(stdout_json["rewrite_validation"]["validated"], true);
    assert_eq!(stdout_json["rewrite_validation"]["parseable_pdf"], true);
    assert_eq!(stdout_json["rewrite_validation"]["page_count"], 1);
    assert_eq!(stdout_json["rewrite_validation"]["review_queue_len"], 0);
    assert_eq!(
        stdout_json["rewrite_validation"]["output_byte_count"]
            .as_u64()
            .unwrap(),
        exported.len() as u64
    );
    assert!(!stdout.contains(output_pdf_path.to_string_lossy().as_ref()));
    assert!(!stdout.contains("clean-output.pdf"));
    assert!(!stdout.contains("ClinicNote"));
    assert!(!stdout.contains("Alice Smith"));

    let report = fs::read_to_string(&report_path).unwrap();
    let report_json: serde_json::Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report_json["rewrite_available"], true);
    assert_eq!(report_json["rewrite_validation"]["validated"], true);
    assert_eq!(report_json["rewrite_validation"]["parseable_pdf"], true);
    assert_eq!(report_json["rewrite_validation"]["page_count"], 1);
    assert_eq!(report_json["rewrite_validation"]["review_queue_len"], 0);
    assert_eq!(
        report_json["rewrite_validation"]["output_byte_count"]
            .as_u64()
            .unwrap(),
        exported.len() as u64
    );
    assert!(!report.contains(output_pdf_path.to_string_lossy().as_ref()));
    assert!(!report.contains("clean-output.pdf"));
    assert!(!report.contains("ClinicNote"));
    assert!(!report.contains("Alice Smith"));
}

#[test]
fn cli_deidentify_pdf_refuses_output_pdf_for_lowercase_review_queue_candidates() {
    let dir = tempdir().unwrap();
    let pdf_path = dir.path().join("patient-alice.pdf");
    let report_path = dir.path().join("report.json");
    let output_pdf_path = dir.path().join("out.pdf");
    let mut pdf = text_layer_pdf_fixture();
    let needle = b"Alice Smith";
    let offset = pdf
        .windows(needle.len())
        .position(|window| window == needle)
        .unwrap();
    pdf[offset..offset + needle.len()].copy_from_slice(b"alice smith");
    fs::write(&pdf_path, pdf).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .arg("deidentify-pdf")
        .arg("--pdf-path")
        .arg(&pdf_path)
        .arg("--source-name")
        .arg("patient-jane.pdf")
        .arg("--report-path")
        .arg(&report_path)
        .arg("--output-pdf-path")
        .arg(&output_pdf_path)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(!output_pdf_path.exists());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("PDF rewrite/export unavailable for this input"));
    assert!(!stderr.contains("Alice Smith"));
    for forbidden in ["agent", "controller", "orchestration"] {
        assert!(!stderr.to_lowercase().contains(forbidden));
    }
}

#[test]
fn cli_deidentify_pdf_writes_phi_safe_review_report() {
    let dir = tempdir().unwrap();
    let pdf_path = dir.path().join("patient-jane.pdf");
    let report_path = dir.path().join("report.json");
    fs::write(&pdf_path, text_layer_pdf_fixture()).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .arg("deidentify-pdf")
        .arg("--pdf-path")
        .arg(&pdf_path)
        .arg("--source-name")
        .arg("patient-jane.pdf")
        .arg("--report-path")
        .arg(&report_path)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("Jane Patient"));
    assert!(!stdout.contains("MRN123"));
    assert!(stdout.contains("review_queue_len"));
    assert!(stdout.contains("review_only_no_rewritten_pdf"));

    let report = fs::read_to_string(&report_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&report).unwrap();
    assert_eq!(json["rewrite_available"], false);
    assert_eq!(json["rewrite_status"], "review_only_no_rewritten_pdf");
    assert_eq!(json["no_rewritten_pdf"], true);
    assert_eq!(json["review_only"], true);
    assert_eq!(json["rewritten_pdf_bytes"], serde_json::Value::Null);
    assert!(json["summary"].is_object());
    assert!(json["page_statuses"].is_array());
    assert!(json["review_queue_len"].as_u64().unwrap() >= 1);
    assert!(!report.contains("Jane Patient"));
    assert!(!report.contains("MRN123"));
}

#[test]
fn cli_deidentify_pdf_redacts_report_path_in_stdout() {
    let dir = tempdir().unwrap();
    let pdf_path = dir.path().join("patient-jane.pdf");
    let report_path = dir.path().join("sensitive-output-location.json");
    fs::write(&pdf_path, text_layer_pdf_fixture()).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .arg("deidentify-pdf")
        .arg("--pdf-path")
        .arg(&pdf_path)
        .arg("--source-name")
        .arg("patient-jane.pdf")
        .arg("--report-path")
        .arg(&report_path)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains(report_path.to_string_lossy().as_ref()));
    assert!(!stdout.contains("sensitive-output-location.json"));
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["report_path"], "<redacted>");
}

#[test]
fn cli_deidentify_pdf_rejects_same_report_and_output_pdf_path_without_creating_file() {
    let dir = tempdir().unwrap();
    let pdf_path = dir.path().join("patient-jane.pdf");
    let colliding_path = dir.path().join("collision.pdf");
    fs::write(&pdf_path, text_layer_pdf_fixture()).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .arg("deidentify-pdf")
        .arg("--pdf-path")
        .arg(&pdf_path)
        .arg("--source-name")
        .arg("patient-jane.pdf")
        .arg("--report-path")
        .arg(&colliding_path)
        .arg("--output-pdf-path")
        .arg(dir.path().join(".").join("collision.pdf"))
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(!colliding_path.exists());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("report and output PDF paths must be different"));
    assert!(!stderr.contains(colliding_path.to_string_lossy().as_ref()));
    assert!(!stderr.contains("patient-jane"));
}

#[test]
fn cli_deidentify_pdf_rejects_invalid_pdf_without_scope_drift_terms() {
    let dir = tempdir().unwrap();
    let pdf_path = dir.path().join("bad.pdf");
    let report_path = dir.path().join("report.json");
    fs::write(&pdf_path, b"not a pdf").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .arg("deidentify-pdf")
        .arg("--pdf-path")
        .arg(&pdf_path)
        .arg("--source-name")
        .arg("bad.pdf")
        .arg("--report-path")
        .arg(&report_path)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed to review PDF"));
    for forbidden in ["moat", "controller", "agent", "orchestration"] {
        assert!(!stderr.to_lowercase().contains(forbidden));
    }
    assert!(!report_path.exists());
}
