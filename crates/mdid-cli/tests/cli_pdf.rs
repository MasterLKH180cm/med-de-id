use std::{fs, process::Command};

use tempfile::tempdir;

fn text_layer_pdf_fixture() -> Vec<u8> {
    fs::read("../mdid-adapters/tests/fixtures/pdf/text-layer-minimal.pdf").unwrap()
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

    let report = fs::read_to_string(&report_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&report).unwrap();
    assert_eq!(json["rewrite_available"], false);
    assert_eq!(json["rewritten_pdf_bytes"], serde_json::Value::Null);
    assert!(json["summary"].is_object());
    assert!(json["page_statuses"].is_array());
    assert!(json["review_queue_len"].as_u64().unwrap() >= 1);
    assert!(!report.contains("Jane Patient"));
    assert!(!report.contains("MRN123"));
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
