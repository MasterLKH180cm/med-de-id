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
fn offline_readiness_reports_cli_opf_and_ocr_without_phi_paths_or_network_claims() {
    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "offline-readiness",
            "--privacy-runner-path",
            &repo_path("scripts/privacy_filter/run_privacy_filter.py"),
            "--ocr-runner-path",
            &repo_path("scripts/ocr_eval/run_small_ocr.py"),
            "--ocr-fixture-path",
            &repo_path("scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png"),
            "--python-command",
            default_python_command(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let report: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(report["artifact"], "offline_cli_ocr_readiness");
    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["network_required"], false);
    assert_eq!(report["privacy_filter"]["opf_requires_explicit_flag"], true);
    assert_eq!(report["privacy_filter"]["network_api_called"], false);
    assert_eq!(report["ocr"]["candidate"], "PP-OCRv5_mobile_rec");
    assert_eq!(report["ocr"]["fallback_fixture_available"], true);
    assert!(!stdout.contains("synthetic_printed_phi_line.png"));
    assert!(!stdout.contains("run_small_ocr.py"));
    assert!(!stdout.contains("run_privacy_filter.py"));
    assert!(!stdout.contains("Jane Example"));
    assert!(!stdout.contains("MRN-12345"));
}

#[test]
fn offline_readiness_help_mentions_exact_usage_line() {
    Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("--help")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "mdid-cli offline-readiness --privacy-runner-path <path> --ocr-runner-path <path> --ocr-fixture-path <path>",
        ));
}

#[test]
fn ocr_small_json_runs_repo_fixture_runner_without_phi_leaks() {
    let dir = tempdir().unwrap();
    let phi_named_dir = dir
        .path()
        .join("Jane-Example-MRN-12345-jane@example.com-555-123-4567");
    fs::create_dir(&phi_named_dir).unwrap();
    let report_path = phi_named_dir.join("ocr-small-json-report.json");

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-small-json",
            "--image-path",
            &repo_path("scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png"),
            "--ocr-runner-path",
            &repo_path("scripts/ocr_eval/run_small_ocr.py"),
            "--report-path",
            report_path.to_str().unwrap(),
            "--python-command",
            default_python_command(),
            "--mock",
        ])
        .assert()
        .success()
        .get_output()
        .clone();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stdout.contains("ocr-small-json"));
    assert!(stdout.contains("\"report_path\":\"<redacted>\""));
    assert!(stdout.contains("\"report_written\":true"));
    for unsafe_text in [
        report_path.to_str().unwrap(),
        phi_named_dir.to_str().unwrap(),
        "Jane Example",
        "MRN-12345",
        "jane@example.com",
        "555-123-4567",
    ] {
        assert!(!stdout.contains(unsafe_text), "stdout leaked {unsafe_text}");
        assert!(!stderr.contains(unsafe_text), "stderr leaked {unsafe_text}");
    }
    assert!(stderr.is_empty());

    let report_text = fs::read_to_string(&report_path).unwrap();
    let report: Value = serde_json::from_str(&report_text).unwrap();
    assert_eq!(report["candidate"], "PP-OCRv5_mobile_rec");
    assert_eq!(report["engine"], "PP-OCRv5-mobile-bounded-spike");
    assert_eq!(report["scope"], "printed_text_line_extraction_only");
    assert_eq!(
        report["privacy_filter_contract"],
        "text_only_normalized_input"
    );
    assert!(report["ready_for_text_pii_eval"].is_boolean());
    assert!(report["extracted_text"].is_string());
    assert!(report["normalized_text"].is_string());
    for non_goal in [
        "visual_redaction",
        "final_pdf_rewrite_export",
        "handwriting_recognition",
        "full_page_detection_or_segmentation",
        "complete_ocr_pipeline",
    ] {
        assert!(report["non_goals"]
            .as_array()
            .unwrap()
            .contains(&Value::String(non_goal.to_string())));
    }
}

#[test]
fn ocr_small_json_redacts_phi_bearing_source_artifacts() {
    let dir = tempdir().unwrap();
    let image_path = dir.path().join("Jane-Example-MRN-12345.png");
    fs::copy(
        repo_path("scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png"),
        &image_path,
    )
    .unwrap();
    fs::copy(
        repo_path("scripts/ocr_eval/fixtures/synthetic_printed_phi_expected.txt"),
        dir.path().join("synthetic_printed_phi_expected.txt"),
    )
    .unwrap();
    let report_path = dir.path().join("ocr-small-json-report.json");
    let summary_path = dir.path().join("ocr-small-json-summary.json");

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-small-json",
            "--image-path",
            image_path.to_str().unwrap(),
            "--ocr-runner-path",
            &repo_path("scripts/ocr_eval/run_small_ocr.py"),
            "--report-path",
            report_path.to_str().unwrap(),
            "--summary-output",
            summary_path.to_str().unwrap(),
            "--python-command",
            default_python_command(),
            "--mock",
        ])
        .assert()
        .success()
        .get_output()
        .clone();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    let report_text = fs::read_to_string(&report_path).unwrap();
    let summary_text = fs::read_to_string(&summary_path).unwrap();
    let full_image_path = image_path.to_str().unwrap();
    for (artifact, text) in [
        ("stdout", stdout.as_str()),
        ("stderr", stderr.as_str()),
        ("report", report_text.as_str()),
        ("summary", summary_text.as_str()),
    ] {
        for unsafe_text in ["Jane-Example-MRN-12345", full_image_path] {
            assert!(
                !text.contains(unsafe_text),
                "{artifact} leaked source artifact {unsafe_text}"
            );
        }
    }

    let report: Value = serde_json::from_str(&report_text).unwrap();
    assert_eq!(report["source"], "<redacted>");
}

#[test]
fn ocr_small_json_writes_phi_safe_summary_output() {
    let dir = tempdir().unwrap();
    let phi_named_dir = dir
        .path()
        .join("Jane-Example-MRN-12345-jane@example.com-555-123-4567");
    fs::create_dir(&phi_named_dir).unwrap();
    let report_path = phi_named_dir.join("ocr-small-json-report.json");
    let summary_path = phi_named_dir.join("ocr-small-json-summary.json");

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-small-json",
            "--image-path",
            &repo_path("scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png"),
            "--ocr-runner-path",
            &repo_path("scripts/ocr_eval/run_small_ocr.py"),
            "--report-path",
            report_path.to_str().unwrap(),
            "--summary-output",
            summary_path.to_str().unwrap(),
            "--python-command",
            default_python_command(),
            "--mock",
        ])
        .assert()
        .success()
        .get_output()
        .clone();

    assert!(report_path.exists());
    assert!(summary_path.exists());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stdout.contains("\"summary_written\":true"));
    assert!(stdout.contains("\"report_path\":\"<redacted>\""));
    for unsafe_text in [
        report_path.to_str().unwrap(),
        summary_path.to_str().unwrap(),
        phi_named_dir.to_str().unwrap(),
        "Jane Example",
        "MRN-12345",
        "jane@example.com",
        "555-123-4567",
    ] {
        assert!(!stdout.contains(unsafe_text), "stdout leaked {unsafe_text}");
        assert!(!stderr.contains(unsafe_text), "stderr leaked {unsafe_text}");
    }

    let summary_text = fs::read_to_string(&summary_path).unwrap();
    let summary: Value = serde_json::from_str(&summary_text).unwrap();
    let summary_keys = summary
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        summary_keys,
        [
            "artifact",
            "schema_version",
            "candidate",
            "engine",
            "engine_status",
            "scope",
            "privacy_filter_contract",
            "ready_for_text_pii_eval",
            "non_goals",
        ]
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>()
    );
    assert_eq!(summary["artifact"], "ocr_small_json_summary");
    assert_eq!(summary["schema_version"], 1);
    assert_eq!(summary["candidate"], "PP-OCRv5_mobile_rec");
    assert_eq!(summary["engine"], "PP-OCRv5-mobile-bounded-spike");
    assert_eq!(summary["scope"], "printed_text_line_extraction_only");
    assert_eq!(
        summary["privacy_filter_contract"],
        "text_only_normalized_input"
    );
    assert_eq!(summary["ready_for_text_pii_eval"], true);
    for forbidden in [
        "\"source\"",
        "\"extracted_text\"",
        "\"normalized_text\"",
        "\"local\"",
        "\"path\"",
        "\"bbox\"",
        "\"image\"",
        "\"span\"",
        "\"preview\"",
        "\"masked_text\"",
        "Jane Example",
        "MRN-12345",
        "jane@example.com",
        "555-123-4567",
        "/tmp/",
        "/home/",
    ] {
        assert!(
            !summary_text.contains(forbidden),
            "summary leaked {forbidden}"
        );
    }
}

#[test]
fn ocr_small_json_local_mode_does_not_force_mock_flag() {
    let dir = tempdir().unwrap();
    let phi_named_dir = dir
        .path()
        .join("Jane-Example-MRN-12345-jane@example.com-555-123-4567");
    fs::create_dir(&phi_named_dir).unwrap();
    let image_path = phi_named_dir.join("synthetic_fixture_001.png");
    fs::write(&image_path, b"synthetic image fixture").unwrap();
    let report_path = phi_named_dir.join("ocr-small-json-report.json");
    let summary_path = phi_named_dir.join("ocr-small-json-summary.json");
    let argv_path = phi_named_dir.join("fake-runner-argv.json");
    let fake_runner_path = phi_named_dir.join("fake_ocr_runner.py");
    fs::write(
        &fake_runner_path,
        format!(
            r#"import json
import sys
from pathlib import Path

Path({argv_path:?}).write_text(json.dumps(sys.argv), encoding="utf-8")
print(json.dumps({{
    "candidate": "PP-OCRv5_mobile_rec",
    "engine": "PP-OCRv5-mobile-bounded-spike",
    "engine_status": "local_paddleocr_execution",
    "scope": "printed_text_line_extraction_only",
    "source": "<redacted>",
    "extracted_text": "LOCAL OCR PRINTED TEXT LINE READY FOR EVAL!",
    "normalized_text": "LOCAL OCR PRINTED TEXT LINE READY FOR EVAL!",
    "ready_for_text_pii_eval": True,
    "privacy_filter_contract": "text_only_normalized_input",
    "non_goals": [
        "visual_redaction",
        "pixel_redaction",
        "final_pdf_rewrite_export",
        "handwriting_recognition",
        "full_page_detection_or_segmentation",
        "complete_ocr_pipeline"
    ]
}}, sort_keys=True))
"#,
            argv_path = argv_path.to_string_lossy()
        ),
    )
    .unwrap();

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-small-json",
            "--image-path",
            image_path.to_str().unwrap(),
            "--ocr-runner-path",
            fake_runner_path.to_str().unwrap(),
            "--report-path",
            report_path.to_str().unwrap(),
            "--summary-output",
            summary_path.to_str().unwrap(),
            "--python-command",
            default_python_command(),
        ])
        .assert()
        .success()
        .get_output()
        .clone();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stdout.contains("ocr-small-json"));
    assert!(stdout.contains("\"report_path\":\"<redacted>\""));
    assert!(stdout.contains("\"summary_written\":true"));
    let runner_argv: Vec<String> =
        serde_json::from_str(&fs::read_to_string(&argv_path).unwrap()).unwrap();
    assert!(runner_argv.iter().any(|arg| arg == "--json"));
    assert!(
        !runner_argv.iter().any(|arg| arg == "--mock"),
        "fake runner argv unexpectedly included --mock: {runner_argv:?}"
    );

    let report_text = fs::read_to_string(&report_path).unwrap();
    let summary_text = fs::read_to_string(&summary_path).unwrap();
    let report: Value = serde_json::from_str(&report_text).unwrap();
    let summary: Value = serde_json::from_str(&summary_text).unwrap();
    assert_eq!(report["source"], "<redacted>");
    assert_eq!(report["candidate"], "PP-OCRv5_mobile_rec");
    assert_eq!(report["scope"], "printed_text_line_extraction_only");
    assert_eq!(report["engine_status"], "local_paddleocr_execution");
    assert_eq!(
        report["normalized_text"].as_str().unwrap().chars().count(),
        43
    );
    assert_eq!(summary["engine_status"], "local_paddleocr_execution");
    for non_goal in [
        "visual_redaction",
        "pixel_redaction",
        "final_pdf_rewrite_export",
    ] {
        assert!(report["non_goals"]
            .as_array()
            .unwrap()
            .contains(&Value::String(non_goal.to_string())));
    }
    for unsafe_text in [
        report_path.to_str().unwrap(),
        summary_path.to_str().unwrap(),
        phi_named_dir.to_str().unwrap(),
        "Jane Example",
        "MRN-12345",
        "jane@example.com",
        "555-123-4567",
    ] {
        assert!(!stdout.contains(unsafe_text), "stdout leaked {unsafe_text}");
        assert!(!stderr.contains(unsafe_text), "stderr leaked {unsafe_text}");
        assert!(
            !report_text.contains(unsafe_text),
            "report leaked {unsafe_text}"
        );
        assert!(
            !summary_text.contains(unsafe_text),
            "summary leaked {unsafe_text}"
        );
    }
}

#[test]
fn ocr_small_json_removes_stale_summary_on_missing_runner() {
    let dir = tempdir().unwrap();
    let report_path = dir.path().join("ocr-small-json-report.json");
    let summary_path = dir.path().join("ocr-small-json-summary.json");
    fs::write(&report_path, "stale raw Jane Example").unwrap();
    fs::write(&summary_path, "stale raw Jane Example").unwrap();

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-small-json",
            "--image-path",
            &repo_path("scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png"),
            "--ocr-runner-path",
            dir.path().join("missing-runner.py").to_str().unwrap(),
            "--report-path",
            report_path.to_str().unwrap(),
            "--summary-output",
            summary_path.to_str().unwrap(),
            "--python-command",
            default_python_command(),
            "--mock",
        ])
        .assert()
        .failure()
        .get_output()
        .clone();

    assert!(!report_path.exists());
    assert!(!summary_path.exists());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    for sentinel in ["Jane Example", summary_path.to_str().unwrap()] {
        assert!(!stdout.contains(sentinel));
        assert!(!stderr.contains(sentinel));
    }
}

#[test]
fn ocr_small_json_rejects_same_report_and_summary_path() {
    let dir = tempdir().unwrap();
    let shared_path = dir
        .path()
        .join("ocr-small-json-Jane-Example-MRN-12345.json");
    fs::write(&shared_path, "stale raw Jane Example").unwrap();

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-small-json",
            "--image-path",
            &repo_path("scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png"),
            "--ocr-runner-path",
            &repo_path("scripts/ocr_eval/run_small_ocr.py"),
            "--report-path",
            shared_path.to_str().unwrap(),
            "--summary-output",
            shared_path.to_str().unwrap(),
            "--python-command",
            default_python_command(),
            "--mock",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "OCR small JSON summary path must differ from report path",
        ))
        .get_output()
        .clone();

    assert!(shared_path.exists());
    assert_eq!(
        fs::read_to_string(&shared_path).unwrap(),
        "stale raw Jane Example"
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    for sentinel in [
        shared_path.to_str().unwrap(),
        dir.path().to_str().unwrap(),
        "Jane Example",
        "MRN-12345",
        "success",
    ] {
        assert!(!stdout.contains(sentinel), "stdout leaked {sentinel}");
        assert!(!stderr.contains(sentinel), "stderr leaked {sentinel}");
    }
}

#[test]
fn ocr_small_json_rejects_existing_alias_report_and_summary_path_without_cleanup() {
    let dir = tempdir().unwrap();
    let shared_path = dir
        .path()
        .join("ocr-small-json-Jane-Example-MRN-12345.json");
    let alias_dir = dir.path().join("alias");
    #[cfg(unix)]
    std::os::unix::fs::symlink(dir.path(), &alias_dir).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(dir.path(), &alias_dir).unwrap();
    let alias_path = alias_dir.join("ocr-small-json-Jane-Example-MRN-12345.json");
    fs::write(&shared_path, "stale raw Jane Example").unwrap();

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-small-json",
            "--image-path",
            &repo_path("scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png"),
            "--ocr-runner-path",
            &repo_path("scripts/ocr_eval/run_small_ocr.py"),
            "--report-path",
            shared_path.to_str().unwrap(),
            "--summary-output",
            alias_path.to_str().unwrap(),
            "--python-command",
            default_python_command(),
            "--mock",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "OCR small JSON summary path must differ from report path",
        ))
        .get_output()
        .clone();

    assert!(shared_path.exists());
    assert_eq!(
        fs::read_to_string(&shared_path).unwrap(),
        "stale raw Jane Example"
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    for sentinel in [
        shared_path.to_str().unwrap(),
        alias_path.to_str().unwrap(),
        dir.path().to_str().unwrap(),
        "Jane Example",
        "MRN-12345",
        "success",
    ] {
        assert!(!stdout.contains(sentinel), "stdout leaked {sentinel}");
        assert!(!stderr.contains(sentinel), "stderr leaked {sentinel}");
    }
}

#[test]
fn ocr_small_json_rejects_non_existing_alias_report_and_summary_path_without_cleanup() {
    let dir = tempdir().unwrap();
    let report_path = dir
        .path()
        .join("ocr-small-json-Jane-Example-MRN-12345.json");
    let alias_dir = dir.path().join("alias");
    #[cfg(unix)]
    std::os::unix::fs::symlink(dir.path(), &alias_dir).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(dir.path(), &alias_dir).unwrap();
    let summary_path = alias_dir.join("ocr-small-json-Jane-Example-MRN-12345.json");
    assert!(!report_path.exists());
    assert!(!summary_path.exists());

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-small-json",
            "--image-path",
            &repo_path("scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png"),
            "--ocr-runner-path",
            &repo_path("scripts/ocr_eval/run_small_ocr.py"),
            "--report-path",
            report_path.to_str().unwrap(),
            "--summary-output",
            summary_path.to_str().unwrap(),
            "--python-command",
            default_python_command(),
            "--mock",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "OCR small JSON summary path must differ from report path",
        ))
        .get_output()
        .clone();

    assert!(!report_path.exists());
    assert!(!summary_path.exists());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    for sentinel in [
        report_path.to_str().unwrap(),
        summary_path.to_str().unwrap(),
        dir.path().to_str().unwrap(),
        "Jane Example",
        "MRN-12345",
        "success",
    ] {
        assert!(!stdout.contains(sentinel), "stdout leaked {sentinel}");
        assert!(!stderr.contains(sentinel), "stderr leaked {sentinel}");
    }
}

#[test]
fn ocr_small_json_rejects_relative_same_parent_alias_without_cleanup() {
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join("relative-alias-Jane-Example-MRN-12345");
    fs::create_dir(&output_dir).unwrap();
    let report_path = output_dir.join("report.json");
    let summary_path = output_dir.join("report.json");
    assert!(!report_path.exists());
    assert!(!summary_path.exists());

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .current_dir(&output_dir)
        .args([
            "ocr-small-json",
            "--image-path",
            &repo_path("scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png"),
            "--ocr-runner-path",
            &repo_path("scripts/ocr_eval/run_small_ocr.py"),
            "--report-path",
            "./report.json",
            "--summary-output",
            "report.json",
            "--python-command",
            default_python_command(),
            "--mock",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "OCR small JSON summary path must differ from report path",
        ))
        .get_output()
        .clone();

    assert!(!report_path.exists());
    assert!(!summary_path.exists());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    for sentinel in [
        report_path.to_str().unwrap(),
        summary_path.to_str().unwrap(),
        output_dir.to_str().unwrap(),
        "Jane Example",
        "MRN-12345",
        "success",
    ] {
        assert!(!stdout.contains(sentinel), "stdout leaked {sentinel}");
        assert!(!stderr.contains(sentinel), "stderr leaked {sentinel}");
    }
}

#[test]
fn ocr_small_json_rejects_unsafe_engine_status_without_phi_or_path_leaks() {
    let dir = tempdir().unwrap();
    let phi_named_dir = dir.path().join("Jane-Example-MRN-12345");
    fs::create_dir(&phi_named_dir).unwrap();
    let runner_path = phi_named_dir.join("unsafe-engine-status-runner.py");
    fs::write(
        &runner_path,
        r#"import json
print(json.dumps({
    "candidate":"PP-OCRv5_mobile_rec",
    "engine":"PP-OCRv5-mobile-bounded-spike",
    "engine_status":"Jane Example /tmp/patient",
    "scope":"printed_text_line_extraction_only",
    "source":"synthetic_fixture",
    "privacy_filter_contract":"text_only_normalized_input",
    "ready_for_text_pii_eval":True,
    "extracted_text":"ok",
    "normalized_text":"ok",
    "non_goals":["visual_redaction","final_pdf_rewrite_export","handwriting_recognition","full_page_detection_or_segmentation","complete_ocr_pipeline"]
}))
"#,
    )
    .unwrap();
    let report_path = phi_named_dir.join("report.json");
    let summary_path = phi_named_dir.join("summary.json");
    fs::write(&report_path, "stale Jane Example report").unwrap();
    fs::write(&summary_path, "stale Jane Example summary").unwrap();

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-small-json",
            "--image-path",
            &repo_path("scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png"),
            "--ocr-runner-path",
            runner_path.to_str().unwrap(),
            "--report-path",
            report_path.to_str().unwrap(),
            "--summary-output",
            summary_path.to_str().unwrap(),
            "--python-command",
            default_python_command(),
            "--mock",
        ])
        .assert()
        .failure()
        .get_output()
        .clone();

    assert!(!report_path.exists());
    assert!(!summary_path.exists());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    for unsafe_text in [
        report_path.to_str().unwrap(),
        summary_path.to_str().unwrap(),
        runner_path.to_str().unwrap(),
        phi_named_dir.to_str().unwrap(),
        "Jane Example",
        "MRN-12345",
        "/tmp/patient",
    ] {
        assert!(!stdout.contains(unsafe_text), "stdout leaked {unsafe_text}");
        assert!(!stderr.contains(unsafe_text), "stderr leaked {unsafe_text}");
    }
}

#[test]
fn ocr_small_json_rejects_not_ready_report_and_removes_stale_outputs_without_leaks() {
    let dir = tempdir().unwrap();
    let phi_named_dir = dir.path().join("Jane-Example-MRN-12345");
    fs::create_dir(&phi_named_dir).unwrap();
    let runner_path = phi_named_dir.join("not-ready-runner.py");
    fs::write(
        &runner_path,
        r#"import json
print(json.dumps({
    "candidate":"PP-OCRv5_mobile_rec",
    "engine":"PP-OCRv5-mobile-bounded-spike",
    "engine_status":"deterministic_synthetic_fixture_fallback",
    "scope":"printed_text_line_extraction_only",
    "source":"synthetic_fixture",
    "privacy_filter_contract":"text_only_normalized_input",
    "ready_for_text_pii_eval":False,
    "extracted_text":"Jane Example MRN-12345",
    "normalized_text":"Jane Example MRN-12345",
    "non_goals":["visual_redaction","final_pdf_rewrite_export","handwriting_recognition","full_page_detection_or_segmentation","complete_ocr_pipeline"]
}))
"#,
    )
    .unwrap();
    let report_path = phi_named_dir.join("report.json");
    let summary_path = phi_named_dir.join("summary.json");
    fs::write(&report_path, "stale Jane Example report").unwrap();
    fs::write(&summary_path, "stale Jane Example summary").unwrap();

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-small-json",
            "--image-path",
            &repo_path("scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png"),
            "--ocr-runner-path",
            runner_path.to_str().unwrap(),
            "--report-path",
            report_path.to_str().unwrap(),
            "--summary-output",
            summary_path.to_str().unwrap(),
            "--python-command",
            default_python_command(),
            "--mock",
        ])
        .assert()
        .failure()
        .get_output()
        .clone();

    assert!(!report_path.exists());
    assert!(!summary_path.exists());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    for unsafe_text in [
        report_path.to_str().unwrap(),
        summary_path.to_str().unwrap(),
        runner_path.to_str().unwrap(),
        phi_named_dir.to_str().unwrap(),
        "Jane Example",
        "MRN-12345",
    ] {
        assert!(!stdout.contains(unsafe_text), "stdout leaked {unsafe_text}");
        assert!(!stderr.contains(unsafe_text), "stderr leaked {unsafe_text}");
    }
}

#[test]
fn ocr_small_json_help_mentions_command() {
    Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("--help")
        .assert()
        .failure()
        .stderr(predicate::str::contains("ocr-small-json"));
}

#[test]
fn ocr_small_json_rejects_phi_bearing_runner_source() {
    let dir = tempdir().unwrap();
    let phi_named_dir = dir.path().join("Jane-Example-MRN-12345");
    fs::create_dir(&phi_named_dir).unwrap();
    let image_path = phi_named_dir.join("Jane-Example-MRN-12345.png");
    fs::copy(
        repo_path("scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png"),
        &image_path,
    )
    .unwrap();
    let runner_path = phi_named_dir.join("unsafe-source-runner.py");
    fs::write(
        &runner_path,
        r#"import json
import sys
print(json.dumps({
    "candidate":"PP-OCRv5_mobile_rec",
    "engine":"PP-OCRv5-mobile-bounded-spike",
    "engine_status":"deterministic_synthetic_fixture_fallback",
    "scope":"printed_text_line_extraction_only",
    "source":"Jane-Example-MRN-12345.png",
    "privacy_filter_contract":"text_only_normalized_input",
    "ready_for_text_pii_eval":True,
    "extracted_text":"ok",
    "normalized_text":"ok",
    "non_goals":["visual_redaction","final_pdf_rewrite_export","handwriting_recognition","full_page_detection_or_segmentation","complete_ocr_pipeline"]
}))
"#,
    )
    .unwrap();
    let report_path = phi_named_dir.join("report.json");
    let summary_path = phi_named_dir.join("summary.json");
    fs::write(&report_path, "stale Jane Example report").unwrap();
    fs::write(&summary_path, "stale Jane Example summary").unwrap();

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-small-json",
            "--image-path",
            image_path.to_str().unwrap(),
            "--ocr-runner-path",
            runner_path.to_str().unwrap(),
            "--report-path",
            report_path.to_str().unwrap(),
            "--summary-output",
            summary_path.to_str().unwrap(),
            "--python-command",
            default_python_command(),
            "--mock",
        ])
        .assert()
        .failure()
        .get_output()
        .clone();

    assert!(!report_path.exists());
    assert!(!summary_path.exists());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    for unsafe_text in [
        "Jane-Example-MRN-12345",
        image_path.to_str().unwrap(),
        report_path.to_str().unwrap(),
        summary_path.to_str().unwrap(),
        runner_path.to_str().unwrap(),
    ] {
        assert!(!stdout.contains(unsafe_text), "stdout leaked {unsafe_text}");
        assert!(!stderr.contains(unsafe_text), "stderr leaked {unsafe_text}");
    }
}

#[test]
fn ocr_small_json_invalid_runner_output_removes_stale_report_without_phi_leaks() {
    let dir = tempdir().unwrap();
    let phi_named_dir = dir.path().join("Jane-Example-MRN-12345");
    fs::create_dir(&phi_named_dir).unwrap();
    let runner_path = phi_named_dir.join("bad-runner.py");
    fs::write(
        &runner_path,
        r#"import json
import sys
print(json.dumps({"candidate":"PP-OCRv5_mobile_rec","engine":"PP-OCRv5-mobile-bounded-spike","scope":"wrong_scope","privacy_filter_contract":"text_only_normalized_input","ready_for_text_pii_eval":True,"extracted_text":"ok","normalized_text":"ok","non_goals":["visual_redaction","final_pdf_rewrite_export","handwriting_recognition","full_page_detection_or_segmentation","complete_ocr_pipeline"]}))
"#,
    )
    .unwrap();
    let report_path = phi_named_dir.join("report.json");
    fs::write(&report_path, "stale Jane Example").unwrap();

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-small-json",
            "--image-path",
            &repo_path("scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png"),
            "--ocr-runner-path",
            runner_path.to_str().unwrap(),
            "--report-path",
            report_path.to_str().unwrap(),
            "--python-command",
            default_python_command(),
            "--mock",
        ])
        .assert()
        .failure()
        .get_output()
        .clone();

    assert!(!report_path.exists());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    for unsafe_text in [
        report_path.to_str().unwrap(),
        phi_named_dir.to_str().unwrap(),
        "Jane Example",
        "MRN-12345",
    ] {
        assert!(!stdout.contains(unsafe_text), "stdout leaked {unsafe_text}");
        assert!(!stderr.contains(unsafe_text), "stderr leaked {unsafe_text}");
    }
}

#[test]
fn ocr_small_json_rejects_unknown_phi_bearing_keys_without_leaks() {
    let dir = tempdir().unwrap();
    let phi_named_dir = dir.path().join("Jane-Example-MRN-12345");
    fs::create_dir(&phi_named_dir).unwrap();
    let runner_path = phi_named_dir.join("unsafe-runner.py");
    fs::write(
        &runner_path,
        r#"import json
import sys
print(json.dumps({
    "candidate":"PP-OCRv5_mobile_rec",
    "engine":"PP-OCRv5-mobile-bounded-spike",
    "engine_status":"mock_ready",
    "scope":"printed_text_line_extraction_only",
    "source":"synthetic_fixture",
    "privacy_filter_contract":"text_only_normalized_input",
    "ready_for_text_pii_eval":True,
    "extracted_text":"ok",
    "normalized_text":"ok",
    "non_goals":["visual_redaction","final_pdf_rewrite_export","handwriting_recognition","full_page_detection_or_segmentation","complete_ocr_pipeline"],
    "source_image_path":"/patients/Jane Example/MRN-12345/source.png",
    "bbox":[1,2,3,4],
    "visual_redaction":"Jane Example overlay",
    "pdf_export":"/tmp/Jane Example.pdf",
    "agent_id":"agent-Jane-Example",
    "controller_step":"controller copied MRN-12345 path",
}))
"#,
    )
    .unwrap();
    let report_path = phi_named_dir.join("report.json");
    fs::write(&report_path, "stale Jane Example").unwrap();

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-small-json",
            "--image-path",
            &repo_path("scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png"),
            "--ocr-runner-path",
            runner_path.to_str().unwrap(),
            "--report-path",
            report_path.to_str().unwrap(),
            "--python-command",
            default_python_command(),
            "--mock",
        ])
        .assert()
        .failure()
        .get_output()
        .clone();

    assert!(!report_path.exists());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    for unsafe_text in [
        report_path.to_str().unwrap(),
        runner_path.to_str().unwrap(),
        phi_named_dir.to_str().unwrap(),
        "/patients/Jane Example/MRN-12345/source.png",
        "/tmp/Jane Example.pdf",
        "Jane Example",
        "MRN-12345",
        "agent-Jane-Example",
        "controller copied",
    ] {
        assert!(!stdout.contains(unsafe_text), "stdout leaked {unsafe_text}");
        assert!(!stderr.contains(unsafe_text), "stderr leaked {unsafe_text}");
    }
}

#[test]
fn ocr_small_json_missing_input_removes_stale_report_without_phi_leaks() {
    let dir = tempdir().unwrap();
    let phi_named_dir = dir.path().join("Jane-Example-MRN-12345");
    fs::create_dir(&phi_named_dir).unwrap();
    let report_path = phi_named_dir.join("report.json");
    fs::write(&report_path, "stale Jane Example").unwrap();

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-small-json",
            "--image-path",
            phi_named_dir.join("missing.png").to_str().unwrap(),
            "--ocr-runner-path",
            &repo_path("scripts/ocr_eval/run_small_ocr.py"),
            "--report-path",
            report_path.to_str().unwrap(),
            "--python-command",
            default_python_command(),
            "--mock",
        ])
        .assert()
        .failure()
        .get_output()
        .clone();

    assert!(!report_path.exists());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    for unsafe_text in [
        report_path.to_str().unwrap(),
        phi_named_dir.to_str().unwrap(),
        "Jane Example",
        "MRN-12345",
    ] {
        assert!(!stdout.contains(unsafe_text), "stdout leaked {unsafe_text}");
        assert!(!stderr.contains(unsafe_text), "stderr leaked {unsafe_text}");
    }
}

#[test]
fn ocr_handoff_corpus_runs_repo_fixture_runner_without_phi_leaks() {
    let dir = tempdir().unwrap();
    let report_path = dir.path().join("ocr-handoff-corpus.json");

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-handoff-corpus",
            "--fixture-dir",
            &repo_path("scripts/ocr_eval/fixtures/corpus"),
            "--runner-path",
            &repo_path("scripts/ocr_eval/run_ocr_handoff_corpus.py"),
            "--report-path",
            report_path.to_str().unwrap(),
            "--python-command",
            default_python_command(),
        ])
        .assert()
        .success()
        .get_output()
        .clone();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stdout.contains("ocr-handoff-corpus"));
    assert!(stdout.contains("<redacted>"));
    for sentinel in ["Jane Example", "MRN-12345"] {
        assert!(!stdout.contains(sentinel));
        assert!(!stderr.contains(sentinel));
    }
    assert!(stderr.is_empty());

    let report_text = fs::read_to_string(&report_path).unwrap();
    for sentinel in [
        "Jane Example",
        "MRN-12345",
        "jane@example.com",
        "555-123-4567",
    ] {
        assert!(!report_text.contains(sentinel));
    }
    let report: Value = serde_json::from_str(&report_text).unwrap();
    assert_eq!(report["engine"], "PP-OCRv5-mobile-bounded-spike");
    assert_eq!(report["scope"], "printed_text_line_extraction_only");
    assert_eq!(
        report["privacy_filter_contract"],
        "text_only_normalized_input"
    );
    let fixture_count = report["fixture_count"].as_u64().unwrap();
    assert!(fixture_count >= 2);
    assert_eq!(
        report["ready_fixture_count"].as_u64().unwrap(),
        fixture_count
    );
    for fixture in report["fixtures"].as_array().unwrap() {
        assert!(fixture["id"].as_str().unwrap().starts_with("fixture_"));
    }
}

#[test]
fn ocr_handoff_corpus_writes_phi_safe_summary_output() {
    let dir = tempdir().unwrap();
    let report_path = dir.path().join("ocr-handoff-corpus.json");
    let summary_path = dir.path().join("ocr-handoff-summary.json");

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-handoff-corpus",
            "--fixture-dir",
            &repo_path("scripts/ocr_eval/fixtures/corpus"),
            "--runner-path",
            &repo_path("scripts/ocr_eval/run_ocr_handoff_corpus.py"),
            "--report-path",
            report_path.to_str().unwrap(),
            "--summary-output",
            summary_path.to_str().unwrap(),
            "--python-command",
            default_python_command(),
        ])
        .assert()
        .success();

    assert!(report_path.exists());
    assert!(summary_path.exists());
    let summary_text = fs::read_to_string(&summary_path).unwrap();
    let summary: Value = serde_json::from_str(&summary_text).unwrap();
    let summary_keys = summary
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        summary_keys,
        [
            "artifact",
            "schema_version",
            "candidate",
            "engine",
            "scope",
            "privacy_filter_contract",
            "fixture_count",
            "ready_fixture_count",
            "all_fixtures_ready_for_text_pii_eval",
            "total_char_count",
            "non_goals",
        ]
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>()
    );
    assert_eq!(summary["artifact"], "ocr_handoff_corpus_readiness_summary");
    assert_eq!(summary["schema_version"], 1);
    assert_eq!(summary["candidate"], "PP-OCRv5_mobile_rec");
    assert_eq!(summary["engine"], "PP-OCRv5-mobile-bounded-spike");
    assert_eq!(summary["scope"], "printed_text_line_extraction_only");
    assert_eq!(
        summary["privacy_filter_contract"],
        "text_only_normalized_input"
    );
    assert_eq!(summary["fixture_count"].as_u64().unwrap(), 2);
    assert_eq!(summary["ready_fixture_count"].as_u64().unwrap(), 2);
    assert_eq!(summary["all_fixtures_ready_for_text_pii_eval"], true);
    assert!(summary["total_char_count"].as_u64().unwrap() > 0);
    let non_goals = summary["non_goals"].as_array().unwrap();
    for non_goal in [
        "complete_ocr_pipeline",
        "final_pdf_rewrite_export",
        "full_page_detection_or_segmentation",
        "handwriting_recognition",
        "visual_redaction",
    ] {
        assert!(non_goals.contains(&Value::String(non_goal.to_string())));
    }

    for forbidden_key in [
        "fixtures",
        "fixture",
        "normalized_text",
        "ocr_lines",
        "bbox",
        "image_bytes",
    ] {
        assert!(!summary_text.contains(&format!("\"{forbidden_key}\"")));
    }
    for sentinel in [
        "Jane Example",
        "MRN-12345",
        "jane@example.com",
        "555-123-4567",
        "fixture_001",
        "synthetic_patient_label_",
        "/home/",
        "/tmp/",
        "fixtures/",
    ] {
        assert!(
            !summary_text.contains(sentinel),
            "summary leaked {sentinel}"
        );
    }
}

#[test]
fn ocr_handoff_corpus_rejects_identical_report_and_summary_paths_safely() {
    let dir = tempdir().unwrap();
    let shared_path = dir.path().join("ocr-handoff-Jane-Example-MRN-12345.json");
    fs::write(&shared_path, "stale raw Jane Example").unwrap();

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-handoff-corpus",
            "--fixture-dir",
            &repo_path("scripts/ocr_eval/fixtures/corpus"),
            "--runner-path",
            &repo_path("scripts/ocr_eval/run_ocr_handoff_corpus.py"),
            "--report-path",
            shared_path.to_str().unwrap(),
            "--summary-output",
            shared_path.to_str().unwrap(),
            "--python-command",
            default_python_command(),
        ])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .get_output()
        .clone();

    assert!(shared_path.exists());
    assert_eq!(
        fs::read_to_string(&shared_path).unwrap(),
        "stale raw Jane Example"
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(!stdout.contains("ocr-handoff-corpus"));
    for sentinel in [
        shared_path.to_str().unwrap(),
        dir.path().to_str().unwrap(),
        "Jane Example",
        "MRN-12345",
        "success",
    ] {
        assert!(!stdout.contains(sentinel), "stdout leaked {sentinel}");
        assert!(!stderr.contains(sentinel), "stderr leaked {sentinel}");
    }
}

#[test]
fn ocr_handoff_corpus_removes_stale_summary_on_prerequisite_failure() {
    let dir = tempdir().unwrap();
    let report_path = dir.path().join("ocr-handoff-corpus.json");
    let summary_path = dir.path().join("ocr-handoff-summary.json");
    fs::write(&report_path, "stale raw Jane Example").unwrap();
    fs::write(&summary_path, "stale raw Jane Example").unwrap();

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-handoff-corpus",
            "--fixture-dir",
            dir.path().join("missing-fixtures").to_str().unwrap(),
            "--runner-path",
            &repo_path("scripts/ocr_eval/run_ocr_handoff_corpus.py"),
            "--report-path",
            report_path.to_str().unwrap(),
            "--summary-output",
            summary_path.to_str().unwrap(),
            "--python-command",
            default_python_command(),
        ])
        .assert()
        .failure()
        .get_output()
        .clone();

    assert!(!report_path.exists());
    assert!(!summary_path.exists());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    for sentinel in ["Jane Example", summary_path.to_str().unwrap()] {
        assert!(!stdout.contains(sentinel));
        assert!(!stderr.contains(sentinel));
    }
}

#[test]
fn ocr_handoff_corpus_removes_stale_report_when_runner_fails() {
    let dir = tempdir().unwrap();
    let fixture_dir = dir.path().join("fixtures");
    fs::create_dir(&fixture_dir).unwrap();
    fs::write(fixture_dir.join("one.txt"), "synthetic printed line").unwrap();
    let runner_path = dir.path().join("fail_runner.py");
    fs::write(&runner_path, "import sys\nsys.exit(1)\n").unwrap();
    let report_path = dir.path().join("ocr-handoff-corpus.json");
    fs::write(&report_path, "stale raw Jane Example").unwrap();

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-handoff-corpus",
            "--fixture-dir",
            fixture_dir.to_str().unwrap(),
            "--runner-path",
            runner_path.to_str().unwrap(),
            "--report-path",
            report_path.to_str().unwrap(),
            "--python-command",
            default_python_command(),
        ])
        .assert()
        .failure();

    assert!(!report_path.exists());
}

#[test]
fn ocr_handoff_corpus_removes_stale_report_when_runner_path_missing() {
    let dir = tempdir().unwrap();
    let fixture_dir = dir.path().join("fixtures");
    fs::create_dir(&fixture_dir).unwrap();
    fs::write(fixture_dir.join("one.txt"), "synthetic printed line").unwrap();
    let report_path = dir.path().join("ocr-handoff-corpus.json");
    fs::write(&report_path, "stale raw Jane Example").unwrap();

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-handoff-corpus",
            "--fixture-dir",
            fixture_dir.to_str().unwrap(),
            "--runner-path",
            dir.path().join("missing_runner.py").to_str().unwrap(),
            "--report-path",
            report_path.to_str().unwrap(),
            "--python-command",
            default_python_command(),
        ])
        .assert()
        .failure();

    assert!(!report_path.exists());
}

#[test]
fn ocr_to_privacy_filter_corpus_help_mentions_command() {
    Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("--help")
        .assert()
        .failure()
        .stderr(predicate::str::contains("ocr-to-privacy-filter-corpus"));
}

#[test]
fn ocr_to_privacy_filter_single_help_mentions_command() {
    Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("--help")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "mdid-cli ocr-to-privacy-filter --image-path <path> --ocr-runner-path <path> --privacy-runner-path <path> --report-path <report.json> [--summary-output <summary.json>] [--python-command <cmd>] [--mock]",
        ));
}

#[test]
fn ocr_to_privacy_filter_single_runs_fixture_chain_without_phi_leaks() {
    let dir = tempdir().expect("tempdir");
    let report_path = dir.path().join("ocr-to-privacy-filter.json");
    let summary_path = dir.path().join("ocr-to-privacy-filter-summary.json");

    let assert = Command::cargo_bin("mdid-cli")
        .expect("binary")
        .args([
            "ocr-to-privacy-filter",
            "--image-path",
            &repo_path("scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png"),
            "--ocr-runner-path",
            &repo_path("scripts/ocr_eval/run_small_ocr.py"),
            "--privacy-runner-path",
            &repo_path("scripts/privacy_filter/run_privacy_filter.py"),
            "--report-path",
            report_path.to_str().expect("report path"),
            "--summary-output",
            summary_path.to_str().expect("summary path"),
            "--python-command",
            default_python_command(),
            "--mock",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    assert!(stdout.contains("ocr-to-privacy-filter"));
    assert!(stdout.contains("\"report_path\":\"...\""));
    assert!(!stdout.contains(report_path.to_str().expect("report path")));
    assert!(!stdout.contains("Patient Jane Example"));
    assert!(!stdout.contains("MRN-12345"));
    assert!(!stdout.contains("jane@example.com"));

    let report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report_path).expect("report file"))
            .expect("report json");
    assert_eq!(report["artifact"], "ocr_to_privacy_filter_single");
    assert_eq!(report["ocr_scope"], "printed_text_line_extraction_only");
    assert_eq!(report["privacy_scope"], "text_only_pii_detection");
    assert_eq!(report["network_api_called"], false);
    assert_eq!(report["ready_for_text_pii_eval"], true);
    assert!(
        report["privacy_filter_detected_span_count"]
            .as_u64()
            .unwrap_or(0)
            >= 3
    );
    assert!(report["privacy_filter_category_counts"]
        .get("NAME")
        .is_some());
    let report_text = report.to_string();
    assert!(!report_text.contains("Patient Jane Example"));
    assert!(!report_text.contains("MRN-12345"));
    assert!(!report_text.contains("jane@example.com"));
    assert!(!report_text.contains("normalized_text"));
    assert!(!report_text.contains("masked_text"));
    assert!(!report_text.contains("spans"));

    let summary: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&summary_path).expect("summary file"))
            .expect("summary json");
    assert_eq!(summary["artifact"], "ocr_to_privacy_filter_single_summary");
    assert_eq!(summary["ocr_scope"], "printed_text_line_extraction_only");
    assert_eq!(summary["privacy_scope"], "text_only_pii_detection");
    assert_eq!(summary["network_api_called"], false);
    let summary_text = summary.to_string();
    assert!(!summary_text.contains("Patient Jane Example"));
    assert!(!summary_text.contains("MRN-12345"));
    assert!(!summary_text.contains("jane@example.com"));
    assert!(!summary_text.contains("normalized_text"));
    assert!(!summary_text.contains("masked_text"));
    assert!(!summary_text.contains("spans"));
}

#[test]
fn ocr_to_privacy_filter_local_mode_does_not_force_mock_flag() {
    let dir = tempdir().expect("tempdir");
    let phi_named_dir = dir
        .path()
        .join("Patient-Jane-Example-MRN-12345-jane@example.com-555-123-4567");
    fs::create_dir(&phi_named_dir).expect("phi fixture dir");
    let image_path = phi_named_dir.join("synthetic_fixture.png");
    fs::write(&image_path, b"synthetic image fixture").expect("image fixture");
    let report_path = phi_named_dir.join("ocr-to-privacy-filter-report.json");
    let summary_path = phi_named_dir.join("ocr-to-privacy-filter-summary.json");
    let ocr_argv_path = phi_named_dir.join("fake-ocr-runner-argv.json");
    let privacy_argv_path = phi_named_dir.join("fake-privacy-runner-argv.json");
    let fake_runner_path = phi_named_dir.join("fake_ocr_runner.py");
    let fake_privacy_runner_path = phi_named_dir.join("fake_privacy_runner.py");
    fs::write(
        &fake_runner_path,
        format!(
            r#"import json
import sys
from pathlib import Path

Path({ocr_argv_path:?}).write_text(json.dumps(sys.argv), encoding="utf-8")
print(json.dumps({{
    "candidate": "PP-OCRv5_mobile_rec",
    "engine": "PP-OCRv5-mobile-bounded-spike",
    "engine_status": "local_paddleocr_execution",
    "scope": "printed_text_line_extraction_only",
    "source": "fixture_001",
    "extracted_text": "Patient Jane Example MRN-12345 jane@example.com 555-123-4567",
    "normalized_text": "Patient Jane Example MRN-12345 jane@example.com 555-123-4567",
    "ready_for_text_pii_eval": True,
    "privacy_filter_contract": "text_only_normalized_input",
    "non_goals": ["visual_redaction", "pixel_redaction", "final_pdf_rewrite_export"]
}}, sort_keys=True))
"#,
            ocr_argv_path = ocr_argv_path.to_string_lossy()
        ),
    )
    .expect("fake ocr runner");
    fs::write(
        &fake_privacy_runner_path,
        format!(
            r#"import argparse
import json
import sys
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument("--stdin", action="store_true")
parser.add_argument("--mock", action="store_true")
args = parser.parse_args()

Path({privacy_argv_path:?}).write_text(json.dumps(sys.argv), encoding="utf-8")
text = sys.stdin.read() if args.stdin else ""
spans = [
    {{"label": "NAME", "start": text.find("Jane Example"), "end": text.find("Jane Example") + len("Jane Example"), "preview": "<redacted>"}},
    {{"label": "MRN", "start": text.find("MRN-12345"), "end": text.find("MRN-12345") + len("MRN-12345"), "preview": "<redacted>"}},
    {{"label": "EMAIL", "start": text.find("jane@example.com"), "end": text.find("jane@example.com") + len("jane@example.com"), "preview": "<redacted>"}},
    {{"label": "PHONE", "start": text.find("555-123-4567"), "end": text.find("555-123-4567") + len("555-123-4567"), "preview": "<redacted>"}},
]
spans = [span for span in spans if span["start"] >= 0]
counts = {{}}
for span in spans:
    counts[span["label"]] = counts.get(span["label"], 0) + 1
print(json.dumps({{
    "summary": {{
        "input_char_count": len(text),
        "detected_span_count": len(spans),
        "category_counts": counts,
    }},
    "masked_text": "Patient [NAME] [MRN] [EMAIL] [PHONE]",
    "spans": spans,
    "metadata": {{
        "engine": "fallback_synthetic_patterns",
        "network_api_called": False,
        "preview_policy": "redacted_placeholders_only",
    }},
}}, sort_keys=True))
"#,
            privacy_argv_path = privacy_argv_path.to_string_lossy()
        ),
    )
    .expect("fake privacy runner");

    let assert = Command::cargo_bin("mdid-cli")
        .expect("binary")
        .args([
            "ocr-to-privacy-filter",
            "--image-path",
            image_path.to_str().expect("image path"),
            "--ocr-runner-path",
            fake_runner_path.to_str().expect("fake runner path"),
            "--privacy-runner-path",
            fake_privacy_runner_path
                .to_str()
                .expect("fake privacy runner path"),
            "--report-path",
            report_path.to_str().expect("report path"),
            "--summary-output",
            summary_path.to_str().expect("summary path"),
            "--python-command",
            default_python_command(),
        ])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("stderr utf8");
    assert!(stdout.contains("ocr-to-privacy-filter"));
    assert!(stdout.contains("\"report_path\":\"...\""));
    assert!(!stdout.contains(report_path.to_str().expect("report path")));

    let ocr_argv: Vec<String> =
        serde_json::from_str(&fs::read_to_string(&ocr_argv_path).expect("ocr argv file"))
            .expect("ocr argv json");
    assert!(ocr_argv.iter().any(|arg| arg == "--json"));
    assert!(
        !ocr_argv.iter().any(|arg| arg == "--mock"),
        "fake ocr runner argv unexpectedly included --mock: {ocr_argv:?}"
    );
    let privacy_argv: Vec<String> =
        serde_json::from_str(&fs::read_to_string(&privacy_argv_path).expect("privacy argv file"))
            .expect("privacy argv json");
    assert!(privacy_argv.iter().any(|arg| arg == "--stdin"));
    assert!(
        !privacy_argv.iter().any(|arg| arg == "--mock"),
        "fake privacy runner argv unexpectedly included --mock: {privacy_argv:?}"
    );

    let report_text = fs::read_to_string(&report_path).expect("report file");
    let summary_text = fs::read_to_string(&summary_path).expect("summary file");
    let report: Value = serde_json::from_str(&report_text).expect("report json");
    let summary: Value = serde_json::from_str(&summary_text).expect("summary json");
    assert_eq!(report["artifact"], "ocr_to_privacy_filter_single");
    assert_eq!(report["ocr_scope"], "printed_text_line_extraction_only");
    assert_eq!(report["privacy_scope"], "text_only_pii_detection");
    assert_eq!(
        report["privacy_filter_contract"],
        "text_only_normalized_input"
    );
    assert_eq!(report["network_api_called"], false);
    assert_eq!(report["ready_for_text_pii_eval"], true);
    assert!(
        report["privacy_filter_detected_span_count"]
            .as_u64()
            .unwrap_or(0)
            >= 3
    );
    assert!(report["privacy_filter_category_counts"]
        .as_object()
        .is_some());
    assert_eq!(summary["artifact"], "ocr_to_privacy_filter_single_summary");
    assert_eq!(summary["ocr_scope"], "printed_text_line_extraction_only");
    assert_eq!(summary["privacy_scope"], "text_only_pii_detection");
    assert_eq!(summary["network_api_called"], false);

    for unsafe_text in [
        report_path.to_str().expect("report path"),
        summary_path.to_str().expect("summary path"),
        phi_named_dir.to_str().expect("phi dir"),
        "Patient Jane Example",
        "MRN-12345",
        "jane@example.com",
        "555-123-4567",
        "normalized_text",
        "masked_text",
        "spans",
    ] {
        assert!(!stdout.contains(unsafe_text), "stdout leaked {unsafe_text}");
        assert!(!stderr.contains(unsafe_text), "stderr leaked {unsafe_text}");
        assert!(
            !report_text.contains(unsafe_text),
            "report leaked {unsafe_text}"
        );
        assert!(
            !summary_text.contains(unsafe_text),
            "summary leaked {unsafe_text}"
        );
    }
}

#[test]
fn ocr_to_privacy_filter_single_removes_stale_outputs_on_missing_image() {
    let dir = tempdir().expect("tempdir");
    let report_path = dir.path().join("stale-report.json");
    let summary_path = dir.path().join("stale-summary.json");
    std::fs::write(&report_path, "stale report").expect("write stale report");
    std::fs::write(&summary_path, "stale summary").expect("write stale summary");

    Command::cargo_bin("mdid-cli")
        .expect("binary")
        .args([
            "ocr-to-privacy-filter",
            "--image-path",
            dir.path()
                .join("missing.png")
                .to_str()
                .expect("missing image path"),
            "--ocr-runner-path",
            &repo_path("scripts/ocr_eval/run_small_ocr.py"),
            "--privacy-runner-path",
            &repo_path("scripts/privacy_filter/run_privacy_filter.py"),
            "--report-path",
            report_path.to_str().expect("report path"),
            "--summary-output",
            summary_path.to_str().expect("summary path"),
            "--python-command",
            default_python_command(),
            "--mock",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "ocr_to_privacy_filter single-image chain failed",
        ))
        .stderr(predicate::str::contains("missing.png").not())
        .stderr(predicate::str::contains("Patient Jane Example").not());

    assert!(
        !report_path.exists(),
        "stale report should be removed on failure"
    );
    assert!(
        !summary_path.exists(),
        "stale summary should be removed on failure"
    );
}

#[test]
fn ocr_to_privacy_filter_single_rejects_identical_report_and_summary_paths_before_cleanup() {
    let dir = tempdir().expect("tempdir");
    let output_path = dir.path().join("same-output.json");
    std::fs::write(&output_path, "stale output").expect("write stale output");

    Command::cargo_bin("mdid-cli")
        .expect("binary")
        .args([
            "ocr-to-privacy-filter",
            "--image-path",
            &repo_path("scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png"),
            "--ocr-runner-path",
            &repo_path("scripts/ocr_eval/run_small_ocr.py"),
            "--privacy-runner-path",
            &repo_path("scripts/privacy_filter/run_privacy_filter.py"),
            "--report-path",
            output_path.to_str().expect("output path"),
            "--summary-output",
            output_path.to_str().expect("output path"),
            "--python-command",
            default_python_command(),
            "--mock",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "summary path must differ from report path",
        ))
        .stderr(predicate::str::contains(output_path.to_str().expect("output path")).not())
        .stderr(predicate::str::contains("Patient Jane Example").not());

    assert_eq!(
        std::fs::read_to_string(&output_path).expect("stale output remains"),
        "stale output"
    );
}

#[test]
fn ocr_to_privacy_filter_single_rejects_alias_report_and_summary_paths_before_cleanup() {
    let dir = tempdir().expect("tempdir");
    let output_path = dir.path().join("same-output.json");
    let alias_path = dir.path().join(".").join("same-output.json");
    std::fs::write(&output_path, "stale output").expect("write stale output");

    Command::cargo_bin("mdid-cli")
        .expect("binary")
        .args([
            "ocr-to-privacy-filter",
            "--image-path",
            &repo_path("scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png"),
            "--ocr-runner-path",
            &repo_path("scripts/ocr_eval/run_small_ocr.py"),
            "--privacy-runner-path",
            &repo_path("scripts/privacy_filter/run_privacy_filter.py"),
            "--report-path",
            output_path.to_str().expect("output path"),
            "--summary-output",
            alias_path.to_str().expect("alias output path"),
            "--python-command",
            default_python_command(),
            "--mock",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "summary path must differ from report path",
        ))
        .stderr(predicate::str::contains(output_path.to_str().expect("output path")).not())
        .stderr(predicate::str::contains(alias_path.to_str().expect("alias output path")).not())
        .stderr(predicate::str::contains("Patient Jane Example").not())
        .stderr(predicate::str::contains("MRN-12345").not());

    assert_eq!(
        std::fs::read_to_string(&output_path).expect("stale output remains"),
        "stale output"
    );
}

#[test]
fn ocr_to_privacy_filter_single_rejects_unsafe_privacy_metadata_and_removes_stale_outputs() {
    let dir = tempdir().expect("tempdir");
    let privacy_runner_path = dir.path().join("unsafe_privacy_runner.py");
    std::fs::write(
        &privacy_runner_path,
        r#"import json
print(json.dumps({"metadata":{"network_api_called":False,"engine":"/patients/Jane-Example/MRN-12345"},"summary":{"detected_span_count":1,"category_counts":{"NAME":1}}}))
"#,
    )
    .expect("write privacy runner");
    let report_path = dir.path().join("report.json");
    let summary_path = dir.path().join("summary.json");
    std::fs::write(&report_path, "stale report").expect("write stale report");
    std::fs::write(&summary_path, "stale summary").expect("write stale summary");

    let assert = Command::cargo_bin("mdid-cli")
        .expect("binary")
        .args([
            "ocr-to-privacy-filter",
            "--image-path",
            &repo_path("scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png"),
            "--ocr-runner-path",
            &repo_path("scripts/ocr_eval/run_small_ocr.py"),
            "--privacy-runner-path",
            privacy_runner_path.to_str().expect("privacy runner path"),
            "--report-path",
            report_path.to_str().expect("report path"),
            "--summary-output",
            summary_path.to_str().expect("summary path"),
            "--python-command",
            default_python_command(),
            "--mock",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "ocr_to_privacy_filter single-image chain failed",
        ))
        .stderr(predicate::str::contains("Jane-Example").not())
        .stderr(predicate::str::contains("MRN-12345").not())
        .stderr(predicate::str::contains("/patients").not());

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8");
    assert!(!stdout.contains("Jane-Example"));
    assert!(!stdout.contains("MRN-12345"));
    assert!(!report_path.exists());
    assert!(!summary_path.exists());
}

#[test]
fn ocr_to_privacy_filter_corpus_runs_repo_fixture_chain_without_phi_leaks() {
    let dir = tempdir().unwrap();
    let report_path = dir.path().join("Jane-Example-MRN-12345-report.json");

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-to-privacy-filter-corpus",
            "--fixture-dir",
            &repo_path("scripts/ocr_eval/fixtures/corpus"),
            "--ocr-runner-path",
            &repo_path("scripts/ocr_eval/run_ocr_handoff_corpus.py"),
            "--privacy-runner-path",
            &repo_path("scripts/privacy_filter/run_privacy_filter.py"),
            "--bridge-runner-path",
            &repo_path("scripts/ocr_eval/run_ocr_to_privacy_filter_corpus.py"),
            "--report-path",
            report_path.to_str().unwrap(),
            "--python-command",
            default_python_command(),
        ])
        .assert()
        .success()
        .get_output()
        .clone();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stdout.contains("ocr-to-privacy-filter-corpus"));
    assert!(stdout.contains("<redacted>"));
    assert!(stdout.contains("total_detected_span_count"));
    assert!(stdout.contains("\"network_api_called\":false"));
    for unsafe_text in [
        "Jane Example",
        "MRN-12345",
        "jane@example.com",
        "555-123-4567",
        "synthetic_patient_label_01.txt",
        report_path.to_str().unwrap(),
    ] {
        assert!(!stdout.contains(unsafe_text), "stdout leaked {unsafe_text}");
        assert!(!stderr.contains(unsafe_text), "stderr leaked {unsafe_text}");
    }

    let report_text = fs::read_to_string(&report_path).unwrap();
    for unsafe_text in [
        "Jane Example",
        "MRN-12345",
        "jane@example.com",
        "555-123-4567",
        "synthetic_patient_label_01.txt",
        "masked_text",
        "spans",
        "preview",
        "extracted_text",
        "normalized_text",
    ] {
        assert!(
            !report_text.contains(unsafe_text),
            "report leaked {unsafe_text}"
        );
    }
    let report: Value = serde_json::from_str(&report_text).unwrap();
    assert_eq!(report["artifact"], "ocr_to_privacy_filter_corpus");
    assert_eq!(report["ocr_scope"], "printed_text_line_extraction_only");
    assert_eq!(report["privacy_scope"], "text_only_pii_detection");
    assert_eq!(
        report["privacy_filter_contract"],
        "text_only_normalized_input"
    );
    assert_eq!(report["network_api_called"], false);
    assert_eq!(report["ready_fixture_count"], report["fixture_count"]);
    assert!(report["total_detected_span_count"].as_u64().unwrap() >= 4);
    assert!(report.get("scope").is_none());
    assert!(report.get("privacy_filter_detected_span_count").is_none());
    for fixture in report["fixtures"].as_array().unwrap() {
        assert!(fixture["fixture"].as_str().unwrap().starts_with("fixture_"));
    }
}

#[test]
fn ocr_to_privacy_filter_corpus_writes_phi_safe_summary_output() {
    let dir = tempdir().unwrap();
    let report_path = dir.path().join("ocr-to-privacy-filter-corpus.json");
    let summary_path = dir.path().join("ocr-to-privacy-filter-corpus-summary.json");

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-to-privacy-filter-corpus",
            "--fixture-dir",
            &repo_path("scripts/ocr_eval/fixtures/corpus"),
            "--ocr-runner-path",
            &repo_path("scripts/ocr_eval/run_ocr_handoff_corpus.py"),
            "--privacy-runner-path",
            &repo_path("scripts/privacy_filter/run_privacy_filter.py"),
            "--bridge-runner-path",
            &repo_path("scripts/ocr_eval/run_ocr_to_privacy_filter_corpus.py"),
            "--report-path",
            report_path.to_str().unwrap(),
            "--summary-output",
            summary_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let summary_text = fs::read_to_string(&summary_path).unwrap();
    let summary: serde_json::Value = serde_json::from_str(&summary_text).unwrap();

    let summary_keys = summary
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        summary_keys,
        [
            "artifact",
            "schema_version",
            "ocr_scope",
            "ocr_engine",
            "ocr_candidate",
            "privacy_scope",
            "privacy_filter_engine",
            "privacy_filter_contract",
            "network_api_called",
            "fixture_count",
            "ready_fixture_count",
            "total_detected_span_count",
            "category_counts",
            "privacy_filter_category_counts",
            "non_goals",
        ]
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>()
    );
    assert_eq!(summary["artifact"], "ocr_to_privacy_filter_corpus_summary");
    assert_eq!(summary["schema_version"], 1);
    assert_eq!(summary["ocr_scope"], "printed_text_line_extraction_only");
    assert_eq!(summary["privacy_scope"], "text_only_pii_detection");
    assert_eq!(
        summary["privacy_filter_contract"],
        "text_only_normalized_input"
    );
    assert_eq!(summary["network_api_called"], false);
    assert_eq!(summary["fixture_count"], 2);
    assert_eq!(summary["ready_fixture_count"], 2);
    assert!(summary["total_detected_span_count"].as_u64().unwrap() > 0);
    assert!(summary.get("fixtures").is_none());
    assert!(summary.get("spans").is_none());
    assert!(summary.get("masked_text").is_none());
    assert!(summary.get("normalized_text").is_none());

    for unsafe_text in [
        "Jane Example",
        "MRN-12345",
        "jane@example.com",
        "555-123-4567",
        "synthetic_patient_label_",
        "/home/",
        "/tmp/",
        "fixtures/",
    ] {
        assert!(
            !summary_text.contains(unsafe_text),
            "summary leaked {unsafe_text}"
        );
    }
}

#[test]
fn ocr_to_privacy_filter_corpus_removes_stale_summary_on_prerequisite_failure() {
    let dir = tempdir().unwrap();
    let report_path = dir.path().join("ocr-to-privacy-filter-corpus.json");
    let summary_path = dir.path().join("ocr-to-privacy-filter-corpus-summary.json");
    fs::write(&report_path, "stale raw Jane Example").unwrap();
    fs::write(&summary_path, "stale raw Jane Example").unwrap();

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-to-privacy-filter-corpus",
            "--fixture-dir",
            dir.path().join("missing-fixtures").to_str().unwrap(),
            "--ocr-runner-path",
            &repo_path("scripts/ocr_eval/run_ocr_handoff_corpus.py"),
            "--privacy-runner-path",
            &repo_path("scripts/privacy_filter/run_privacy_filter.py"),
            "--bridge-runner-path",
            &repo_path("scripts/ocr_eval/run_ocr_to_privacy_filter_corpus.py"),
            "--report-path",
            report_path.to_str().unwrap(),
            "--summary-output",
            summary_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(!report_path.exists());
    assert!(!summary_path.exists());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(!stdout.contains("Jane Example"));
    assert!(!stderr.contains("Jane Example"));
}

#[test]
fn ocr_to_privacy_filter_corpus_missing_bridge_runner_removes_stale_report_generically() {
    let dir = tempdir().unwrap();
    let report_path = dir.path().join("report.json");
    fs::write(&report_path, "stale Jane Example").unwrap();

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-to-privacy-filter-corpus",
            "--fixture-dir",
            &repo_path("scripts/ocr_eval/fixtures/corpus"),
            "--ocr-runner-path",
            &repo_path("scripts/ocr_eval/run_ocr_handoff_corpus.py"),
            "--privacy-runner-path",
            &repo_path("scripts/privacy_filter/run_privacy_filter.py"),
            "--bridge-runner-path",
            dir.path().join("missing.py").to_str().unwrap(),
            "--report-path",
            report_path.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "OCR to privacy filter corpus failed",
        ))
        .stderr(predicate::str::contains("Jane Example").not());

    assert!(!report_path.exists());
}

#[test]
fn ocr_to_privacy_filter_corpus_rejects_unsafe_bridge_report_and_removes_output() {
    let dir = tempdir().unwrap();
    let bridge_path = dir.path().join("unsafe_bridge.py");
    let report_path = dir.path().join("report.json");
    fs::write(
        &bridge_path,
        r#"
import argparse, json
parser = argparse.ArgumentParser()
parser.add_argument('--fixture-dir'); parser.add_argument('--ocr-runner-path'); parser.add_argument('--privacy-runner-path'); parser.add_argument('--output')
args = parser.parse_args()
report = {"artifact":"ocr_to_privacy_filter_corpus_bridge","ocr_candidate":"PP-OCRv5_mobile_rec","ocr_engine":"PP-OCRv5-mobile-bounded-spike","scope":"printed_text_extraction_to_text_pii_detection_only","privacy_filter_engine":"fallback_synthetic_patterns","privacy_filter_contract":"text_only_normalized_input","fixture_count":1,"ready_fixture_count":1,"privacy_filter_detected_span_count":1,"category_counts":{"NAME":1},"privacy_filter_category_counts":{"NAME":1},"fixtures":[{"fixture":"synthetic_patient_label_01.txt","ready_for_text_pii_eval":True,"detected_span_count":1,"masked_text":"Jane Example"}],"non_goals":["visual_redaction"]}
open(args.output, 'w', encoding='utf-8').write(json.dumps(report))
"#,
    )
    .unwrap();
    fs::write(&report_path, "stale Jane Example").unwrap();

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-to-privacy-filter-corpus",
            "--fixture-dir",
            &repo_path("scripts/ocr_eval/fixtures/corpus"),
            "--ocr-runner-path",
            &repo_path("scripts/ocr_eval/run_ocr_handoff_corpus.py"),
            "--privacy-runner-path",
            &repo_path("scripts/privacy_filter/run_privacy_filter.py"),
            "--bridge-runner-path",
            bridge_path.to_str().unwrap(),
            "--report-path",
            report_path.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "invalid OCR to privacy filter corpus report",
        ))
        .stderr(predicate::str::contains("Jane Example").not());

    assert!(!report_path.exists());
}

#[test]
fn ocr_to_privacy_filter_corpus_rejects_second_synthetic_fixture_filename() {
    let dir = tempdir().unwrap();
    let bridge_path = dir.path().join("unsafe_bridge.py");
    let report_path = dir.path().join("report.json");
    fs::write(
        &bridge_path,
        r#"
import argparse, json
parser = argparse.ArgumentParser()
parser.add_argument('--fixture-dir'); parser.add_argument('--ocr-runner-path'); parser.add_argument('--privacy-runner-path'); parser.add_argument('--output')
args = parser.parse_args()
report = {"artifact":"ocr_to_privacy_filter_corpus_bridge","ocr_candidate":"PP-OCRv5_mobile_rec","ocr_engine":"PP-OCRv5-mobile-bounded-spike","scope":"printed_text_extraction_to_text_pii_detection_only","privacy_filter_engine":"fallback_synthetic_patterns","privacy_filter_contract":"text_only_normalized_input","fixture_count":1,"ready_fixture_count":1,"privacy_filter_detected_span_count":1,"category_counts":{"NAME":1},"privacy_filter_category_counts":{"NAME":1},"fixtures":[{"fixture":"synthetic_patient_label_02.txt","ready_for_text_pii_eval":True,"detected_span_count":1}],"non_goals":["visual_redaction"]}
open(args.output, 'w', encoding='utf-8').write(json.dumps(report))
"#,
    )
    .unwrap();

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-to-privacy-filter-corpus",
            "--fixture-dir",
            &repo_path("scripts/ocr_eval/fixtures/corpus"),
            "--ocr-runner-path",
            &repo_path("scripts/ocr_eval/run_ocr_handoff_corpus.py"),
            "--privacy-runner-path",
            &repo_path("scripts/privacy_filter/run_privacy_filter.py"),
            "--bridge-runner-path",
            bridge_path.to_str().unwrap(),
            "--report-path",
            report_path.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "invalid OCR to privacy filter corpus report",
        ));

    assert!(!report_path.exists());
}

#[test]
fn ocr_to_privacy_filter_corpus_rejects_path_like_engine_values_without_leaking_them() {
    for unsafe_engine in [
        r"C:\patients\Jane Example\opf.exe",
        "/var/patient/report.json",
        "fixtures/foo.txt",
        "reports/patient.json",
        "models/runner.py",
        "/opt/patient/report.json",
        "/etc/local/file",
    ] {
        let dir = tempdir().unwrap();
        let bridge_path = dir.path().join("unsafe_bridge.py");
        let report_path = dir.path().join("report.json");
        fs::write(
            &bridge_path,
            format!(
                r#"
import argparse, json
parser = argparse.ArgumentParser()
parser.add_argument('--fixture-dir'); parser.add_argument('--ocr-runner-path'); parser.add_argument('--privacy-runner-path'); parser.add_argument('--output')
args = parser.parse_args()
report = {{"artifact":"ocr_to_privacy_filter_corpus_bridge","ocr_candidate":"PP-OCRv5_mobile_rec","ocr_engine":"PP-OCRv5-mobile-bounded-spike","scope":"printed_text_extraction_to_text_pii_detection_only","privacy_filter_engine":{unsafe_engine:?},"privacy_filter_contract":"text_only_normalized_input","fixture_count":1,"ready_fixture_count":1,"privacy_filter_detected_span_count":1,"category_counts":{{"NAME":1}},"privacy_filter_category_counts":{{"NAME":1}},"fixtures":[{{"fixture":"fixture_001","ready_for_text_pii_eval":True,"detected_span_count":1}}],"non_goals":["visual_redaction"]}}
open(args.output, 'w', encoding='utf-8').write(json.dumps(report))
"#
            ),
        )
        .unwrap();

        Command::cargo_bin("mdid-cli")
            .unwrap()
            .args([
                "ocr-to-privacy-filter-corpus",
                "--fixture-dir",
                &repo_path("scripts/ocr_eval/fixtures/corpus"),
                "--ocr-runner-path",
                &repo_path("scripts/ocr_eval/run_ocr_handoff_corpus.py"),
                "--privacy-runner-path",
                &repo_path("scripts/privacy_filter/run_privacy_filter.py"),
                "--bridge-runner-path",
                bridge_path.to_str().unwrap(),
                "--report-path",
                report_path.to_str().unwrap(),
            ])
            .assert()
            .failure()
            .stderr(predicate::str::contains(
                "invalid OCR to privacy filter corpus report",
            ))
            .stdout(predicate::str::contains(unsafe_engine).not())
            .stderr(predicate::str::contains(unsafe_engine).not());

        assert!(!report_path.exists());
    }
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
fn cli_privacy_filter_corpus_writes_phi_safe_aggregate_summary() {
    let dir = tempdir().unwrap();
    let report_path = dir.path().join("Alice-MRN-99999-report.json");

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("privacy-filter-corpus")
        .arg("--fixture-dir")
        .arg(repo_path("scripts/privacy_filter/fixtures/corpus"))
        .arg("--runner-path")
        .arg(repo_path("scripts/privacy_filter/run_synthetic_corpus.py"))
        .arg("--report-path")
        .arg(&report_path)
        .arg("--python-command")
        .arg(default_python_command())
        .assert()
        .success()
        .stdout(predicate::str::contains("Jane Example").not())
        .stdout(predicate::str::contains("MRN-12345").not())
        .stdout(predicate::str::contains("jane@example.test").not())
        .stdout(predicate::str::contains("555-111-2222").not())
        .stdout(predicate::str::contains("Alice-MRN-99999").not())
        .stderr(predicate::str::contains("Jane Example").not())
        .stderr(predicate::str::contains("MRN-12345").not())
        .stderr(predicate::str::contains("jane@example.test").not())
        .stderr(predicate::str::contains("555-111-2222").not())
        .get_output()
        .stdout
        .clone();

    let summary: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(summary["command"], "privacy-filter-corpus");
    assert_eq!(summary["engine"], "fallback_synthetic_patterns");
    assert_eq!(summary["scope"], "text_only_synthetic_corpus");
    assert_eq!(summary["fixture_count"], 2);
    assert!(summary["total_detected_span_count"].as_u64().unwrap() >= 4);
    assert_eq!(summary["report_path"], "<redacted>");

    let report = fs::read_to_string(&report_path).unwrap();
    assert!(!report.contains("Jane Example"));
    assert!(!report.contains("MRN-12345"));
    assert!(!report.contains("jane@example.test"));
    assert!(!report.contains("555-111-2222"));
    let report_json: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report_json["category_counts"]["NAME"], 2);
    assert_eq!(report_json["category_counts"]["MRN"], 2);
    assert_eq!(report_json["category_counts"]["EMAIL"], 1);
    assert_eq!(report_json["category_counts"]["PHONE"], 2);
    assert!(report_json["non_goals"]
        .as_array()
        .unwrap()
        .contains(&Value::String("visual_redaction".to_string())));
}

#[test]
fn privacy_filter_corpus_writes_phi_safe_summary_output() {
    let dir = tempdir().unwrap();
    let report_path = dir.path().join("privacy-filter-corpus.json");
    let summary_path = dir.path().join("privacy-filter-corpus-summary.json");

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("privacy-filter-corpus")
        .arg("--fixture-dir")
        .arg(repo_path("scripts/privacy_filter/fixtures/corpus"))
        .arg("--runner-path")
        .arg(repo_path("scripts/privacy_filter/run_synthetic_corpus.py"))
        .arg("--report-path")
        .arg(&report_path)
        .arg("--summary-output")
        .arg(&summary_path)
        .arg("--python-command")
        .arg(default_python_command())
        .assert()
        .success();

    let summary_text = fs::read_to_string(&summary_path).unwrap();
    for sentinel in [
        "Jane Example",
        "Alice Smith",
        "MRN-12345",
        "MRN-001",
        "jane@example.com",
        "555-123-4567",
        "synthetic_patient_label_",
        "/home/",
        "/tmp/",
        "fixtures/",
    ] {
        assert!(
            !summary_text.contains(sentinel),
            "summary leaked {sentinel}"
        );
    }
    let summary: Value = serde_json::from_str(&summary_text).unwrap();
    let mut actual_keys = summary
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>();
    actual_keys.sort_unstable();
    let mut expected_keys = vec![
        "artifact",
        "category_counts",
        "engine",
        "fixture_count",
        "network_api_called",
        "non_goals",
        "schema_version",
        "scope",
        "total_detected_span_count",
    ];
    expected_keys.sort_unstable();
    assert_eq!(actual_keys, expected_keys);
    assert_eq!(summary["artifact"], "privacy_filter_corpus_summary");
    assert_eq!(summary["schema_version"], 1);
    assert_eq!(summary["engine"], "fallback_synthetic_patterns");
    assert_eq!(summary["scope"], "text_only_synthetic_corpus");
    assert_eq!(summary["network_api_called"], false);
    assert_eq!(summary["fixture_count"], 2);
    assert!(summary["total_detected_span_count"].as_u64().unwrap() > 0);
    assert!(summary["category_counts"]["NAME"].as_u64().unwrap() > 0);
    let non_goals = summary["non_goals"].as_array().unwrap();
    assert!(non_goals.contains(&Value::String("ocr".to_string())));
    assert!(non_goals.contains(&Value::String("visual_redaction".to_string())));
    for omitted in ["fixtures", "masked_text", "spans", "preview"] {
        assert!(summary.get(omitted).is_none(), "summary included {omitted}");
    }
}

#[test]
fn privacy_filter_corpus_rejects_same_report_and_summary_path_before_cleanup() {
    let dir = tempdir().unwrap();
    let report_path = dir.path().join("Jane-Example-MRN-12345-report.json");
    fs::write(&report_path, "stale Jane Example MRN-12345").unwrap();

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("privacy-filter-corpus")
        .arg("--fixture-dir")
        .arg(repo_path("scripts/privacy_filter/fixtures/corpus"))
        .arg("--runner-path")
        .arg(repo_path("scripts/privacy_filter/run_synthetic_corpus.py"))
        .arg("--report-path")
        .arg(&report_path)
        .arg("--summary-output")
        .arg(&report_path)
        .arg("--python-command")
        .arg(default_python_command())
        .assert()
        .failure()
        .get_output()
        .clone();

    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("privacy filter corpus summary path must differ from report path"));
    assert!(!stderr.contains("Jane Example"));
    assert!(!stderr.contains("MRN-12345"));
    assert!(!stderr.contains(report_path.to_string_lossy().as_ref()));
    assert_eq!(
        fs::read_to_string(&report_path).unwrap(),
        "stale Jane Example MRN-12345"
    );
}

#[test]
fn privacy_filter_corpus_rejects_alias_report_and_summary_path_before_cleanup() {
    let dir = tempdir().unwrap();
    let report_path = dir.path().join("Jane-Example-MRN-12345-report.json");
    let alias_path = dir
        .path()
        .join(".")
        .join("Jane-Example-MRN-12345-report.json");
    fs::write(&report_path, "stale Jane Example MRN-12345").unwrap();

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("privacy-filter-corpus")
        .arg("--fixture-dir")
        .arg(repo_path("scripts/privacy_filter/fixtures/corpus"))
        .arg("--runner-path")
        .arg(repo_path("scripts/privacy_filter/run_synthetic_corpus.py"))
        .arg("--report-path")
        .arg(&report_path)
        .arg("--summary-output")
        .arg(&alias_path)
        .arg("--python-command")
        .arg(default_python_command())
        .assert()
        .failure()
        .get_output()
        .clone();

    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("privacy filter corpus summary path must differ from report path"));
    assert!(!stderr.contains("Jane Example"));
    assert!(!stderr.contains("MRN-12345"));
    assert!(!stderr.contains(report_path.to_string_lossy().as_ref()));
    assert!(!stderr.contains(alias_path.to_string_lossy().as_ref()));
    assert_eq!(
        fs::read_to_string(&report_path).unwrap(),
        "stale Jane Example MRN-12345"
    );
}

#[test]
fn privacy_filter_corpus_removes_stale_summary_on_prerequisite_failure() {
    let dir = tempdir().unwrap();
    let report_path = dir.path().join("privacy-filter-corpus.json");
    let summary_path = dir.path().join("privacy-filter-corpus-summary.json");
    fs::write(&summary_path, "Patient Jane Example MRN-12345").unwrap();

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("privacy-filter-corpus")
        .arg("--fixture-dir")
        .arg(dir.path().join("missing-fixtures"))
        .arg("--runner-path")
        .arg(repo_path("scripts/privacy_filter/run_synthetic_corpus.py"))
        .arg("--report-path")
        .arg(&report_path)
        .arg("--summary-output")
        .arg(&summary_path)
        .arg("--python-command")
        .arg(default_python_command())
        .assert()
        .failure()
        .get_output()
        .clone();

    assert!(!summary_path.exists());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(!stdout.contains("Jane Example"));
    assert!(!stderr.contains("Jane Example"));
}

#[test]
fn cli_privacy_filter_corpus_rejects_fake_canonical_report_with_single_fixture() {
    let dir = tempdir().unwrap();
    let runner_path = dir.path().join("fake_corpus_runner.py");
    let report_path = dir.path().join("privacy-filter-corpus.json");
    fs::write(
        &runner_path,
        r#"
import argparse
import json

parser = argparse.ArgumentParser()
parser.add_argument("--fixture-dir", required=True)
parser.add_argument("--output", required=True)
args = parser.parse_args()

report = {
    "engine": "fallback_synthetic_patterns",
    "scope": "text_only_synthetic_corpus",
    "fixture_count": 1,
    "total_detected_span_count": 3,
    "fixtures": [
        {
            "fixture": "fake.txt",
            "detected_span_count": 3,
            "category_counts": {"NAME": 1, "MRN": 1, "EMAIL": 0, "PHONE": 1},
        }
    ],
    "category_counts": {"NAME": 1, "MRN": 1, "EMAIL": 0, "PHONE": 1},
    "non_goals": ["visual_redaction"],
}
with open(args.output, "w", encoding="utf-8") as handle:
    json.dump(report, handle)
"#,
    )
    .unwrap();

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("privacy-filter-corpus")
        .arg("--fixture-dir")
        .arg(repo_path("scripts/privacy_filter/fixtures/corpus"))
        .arg("--runner-path")
        .arg(&runner_path)
        .arg("--report-path")
        .arg(&report_path)
        .arg("--python-command")
        .arg(default_python_command())
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "invalid privacy filter corpus report",
        ));

    assert!(!report_path.exists(), "invalid report should be removed");
}

#[test]
fn cli_privacy_filter_corpus_sanitizes_phi_bearing_fixture_names() {
    let dir = tempdir().unwrap();
    let fixture_dir = dir.path().join("fixtures");
    fs::create_dir(&fixture_dir).unwrap();
    fs::write(
        fixture_dir.join("Alice-MRN-99999.txt"),
        "Patient Alice Example MRN-99999 phone 555-999-0000",
    )
    .unwrap();
    let report_path = dir.path().join("privacy-filter-corpus.json");

    let assert = Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("privacy-filter-corpus")
        .arg("--fixture-dir")
        .arg(&fixture_dir)
        .arg("--runner-path")
        .arg(repo_path("scripts/privacy_filter/run_synthetic_corpus.py"))
        .arg("--report-path")
        .arg(&report_path)
        .arg("--python-command")
        .arg(default_python_command())
        .assert()
        .success();

    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let report = fs::read_to_string(&report_path).unwrap();
    for raw_phi in [
        "Alice-MRN-99999",
        "Alice Example",
        "MRN-99999",
        "555-999-0000",
    ] {
        assert!(
            !stdout.contains(raw_phi),
            "stdout leaked raw PHI: {raw_phi}"
        );
        assert!(
            !stderr.contains(raw_phi),
            "stderr leaked raw PHI: {raw_phi}"
        );
        assert!(
            !report.contains(raw_phi),
            "report leaked raw PHI: {raw_phi}"
        );
    }

    let report_json: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report_json["fixtures"][0]["fixture"], "fixture_001");
}

#[test]
fn cli_privacy_filter_corpus_rejects_unexpected_phi_bearing_fields() {
    let dir = tempdir().unwrap();
    let fixture_dir = dir.path().join("fixtures");
    fs::create_dir(&fixture_dir).unwrap();
    fs::write(fixture_dir.join("one.txt"), "synthetic fixture").unwrap();
    let runner_path = dir.path().join("fake_corpus_runner.py");
    let report_path = dir.path().join("privacy-filter-corpus.json");
    fs::write(
        &runner_path,
        r#"
import argparse
import json

parser = argparse.ArgumentParser()
parser.add_argument("--fixture-dir", required=True)
parser.add_argument("--output", required=True)
args = parser.parse_args()

report = {
    "engine": "fallback_synthetic_patterns",
    "scope": "text_only_synthetic_corpus",
    "fixture_count": 1,
    "total_detected_span_count": 3,
    "category_counts": {"NAME": 1, "MRN": 1, "PHONE": 1},
    "fixtures": [
        {
            "fixture": "one.txt",
            "detected_span_count": 3,
            "category_counts": {"NAME": 1, "MRN": 1, "PHONE": 1},
            "preview": "Alice Example",
        }
    ],
    "non_goals": ["visual_redaction"],
    "raw_text": "Alice Example MRN-99999",
}
with open(args.output, "w", encoding="utf-8") as handle:
    json.dump(report, handle)
"#,
    )
    .unwrap();

    let assert = Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("privacy-filter-corpus")
        .arg("--fixture-dir")
        .arg(&fixture_dir)
        .arg("--runner-path")
        .arg(&runner_path)
        .arg("--report-path")
        .arg(&report_path)
        .arg("--python-command")
        .arg(default_python_command())
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "invalid privacy filter corpus report",
        ));

    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    for raw_phi in ["Alice Example", "MRN-99999"] {
        assert!(
            !stdout.contains(raw_phi),
            "stdout leaked raw PHI: {raw_phi}"
        );
        assert!(
            !stderr.contains(raw_phi),
            "stderr leaked raw PHI: {raw_phi}"
        );
    }
    assert!(!report_path.exists(), "invalid report should be removed");
}

#[test]
fn cli_privacy_filter_corpus_rejects_phi_bearing_category_count_keys() {
    let dir = tempdir().unwrap();
    let fixture_dir = dir.path().join("fixtures");
    fs::create_dir(&fixture_dir).unwrap();
    fs::write(fixture_dir.join("one.txt"), "synthetic fixture").unwrap();
    let runner_path = dir.path().join("fake_corpus_runner.py");
    let report_path = dir.path().join("privacy-filter-corpus.json");
    fs::write(
        &runner_path,
        r#"
import argparse
import json

parser = argparse.ArgumentParser()
parser.add_argument("--fixture-dir", required=True)
parser.add_argument("--output", required=True)
args = parser.parse_args()

report = {
    "engine": "fallback_synthetic_patterns",
    "scope": "text_only_synthetic_corpus",
    "fixture_count": 1,
    "total_detected_span_count": 3,
    "category_counts": {"NAME": 1, "MRN-99999": 1, "PHONE": 1},
    "fixtures": [
        {
            "fixture": "one.txt",
            "detected_span_count": 3,
            "category_counts": {"NAME": 1, "Alice Example": 1, "PHONE": 1},
        }
    ],
    "non_goals": ["visual_redaction"],
}
with open(args.output, "w", encoding="utf-8") as handle:
    json.dump(report, handle)
"#,
    )
    .unwrap();

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("privacy-filter-corpus")
        .arg("--fixture-dir")
        .arg(&fixture_dir)
        .arg("--runner-path")
        .arg(&runner_path)
        .arg("--report-path")
        .arg(&report_path)
        .arg("--python-command")
        .arg(default_python_command())
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "invalid privacy filter corpus report",
        ));

    assert!(!report_path.exists(), "invalid report should be removed");
}

#[test]
fn cli_privacy_filter_corpus_rejects_phi_bearing_non_goals() {
    let dir = tempdir().unwrap();
    let fixture_dir = dir.path().join("fixtures");
    fs::create_dir(&fixture_dir).unwrap();
    fs::write(fixture_dir.join("one.txt"), "synthetic fixture").unwrap();
    let runner_path = dir.path().join("fake_corpus_runner.py");
    let report_path = dir.path().join("privacy-filter-corpus.json");
    fs::write(
        &runner_path,
        r#"
import argparse
import json

parser = argparse.ArgumentParser()
parser.add_argument("--fixture-dir", required=True)
parser.add_argument("--output", required=True)
args = parser.parse_args()

report = {
    "engine": "fallback_synthetic_patterns",
    "scope": "text_only_synthetic_corpus",
    "fixture_count": 1,
    "total_detected_span_count": 3,
    "category_counts": {"NAME": 1, "MRN": 1, "PHONE": 1},
    "fixtures": [
        {
            "fixture": "one.txt",
            "detected_span_count": 3,
            "category_counts": {"NAME": 1, "MRN": 1, "PHONE": 1},
        }
    ],
    "non_goals": ["visual_redaction", "MRN-99999"],
}
with open(args.output, "w", encoding="utf-8") as handle:
    json.dump(report, handle)
"#,
    )
    .unwrap();

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("privacy-filter-corpus")
        .arg("--fixture-dir")
        .arg(&fixture_dir)
        .arg("--runner-path")
        .arg(&runner_path)
        .arg("--report-path")
        .arg(&report_path)
        .arg("--python-command")
        .arg(default_python_command())
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "invalid privacy filter corpus report",
        ));

    assert!(!report_path.exists(), "invalid report should be removed");
}

#[test]
fn cli_privacy_filter_corpus_rejects_network_api_called_true() {
    let dir = tempdir().unwrap();
    let fixture_dir = dir.path().join("fixtures");
    fs::create_dir(&fixture_dir).unwrap();
    fs::write(fixture_dir.join("one.txt"), "synthetic fixture").unwrap();
    let runner_path = dir.path().join("fake_corpus_runner.py");
    let report_path = dir.path().join("privacy-filter-corpus.json");
    fs::write(
        &runner_path,
        r#"
import argparse
import json

parser = argparse.ArgumentParser()
parser.add_argument("--fixture-dir", required=True)
parser.add_argument("--output", required=True)
args = parser.parse_args()

report = {
    "engine": "fallback_synthetic_patterns",
    "scope": "text_only_synthetic_corpus",
    "fixture_count": 1,
    "total_detected_span_count": 3,
    "category_counts": {"NAME": 1, "MRN": 1, "PHONE": 1},
    "fixtures": [
        {
            "fixture": "one.txt",
            "detected_span_count": 3,
            "category_counts": {"NAME": 1, "MRN": 1, "PHONE": 1},
        }
    ],
    "non_goals": ["visual_redaction"],
    "network_api_called": True,
}
with open(args.output, "w", encoding="utf-8") as handle:
    json.dump(report, handle)
"#,
    )
    .unwrap();

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("privacy-filter-corpus")
        .arg("--fixture-dir")
        .arg(&fixture_dir)
        .arg("--runner-path")
        .arg(&runner_path)
        .arg("--report-path")
        .arg(&report_path)
        .arg("--python-command")
        .arg(default_python_command())
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "invalid privacy filter corpus report",
        ));

    assert!(!report_path.exists(), "networked report should be removed");
}

#[test]
fn cli_privacy_filter_corpus_rejects_oversized_report_before_read() {
    let dir = tempdir().unwrap();
    let fixture_dir = dir.path().join("fixtures");
    fs::create_dir(&fixture_dir).unwrap();
    fs::write(fixture_dir.join("one.txt"), "synthetic fixture").unwrap();
    let runner_path = dir.path().join("fake_corpus_runner.py");
    let report_path = dir.path().join("privacy-filter-corpus.json");
    fs::write(
        &runner_path,
        r#"
import argparse
import json

parser = argparse.ArgumentParser()
parser.add_argument("--fixture-dir", required=True)
parser.add_argument("--output", required=True)
args = parser.parse_args()

report = {
    "engine": "fallback_synthetic_patterns",
    "scope": "text_only_synthetic_corpus",
    "fixture_count": 1,
    "total_detected_span_count": 3,
    "category_counts": {"NAME": 1, "MRN": 1, "PHONE": 1},
    "fixtures": [
        {
            "fixture": "one.txt",
            "detected_span_count": 3,
            "category_counts": {"NAME": 1, "MRN": 1, "PHONE": 1},
        }
    ],
    "non_goals": ["visual_redaction"],
    "padding": "x" * (1024 * 1024 + 1),
}
with open(args.output, "w", encoding="utf-8") as handle:
    json.dump(report, handle)
"#,
    )
    .unwrap();

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("privacy-filter-corpus")
        .arg("--fixture-dir")
        .arg(&fixture_dir)
        .arg("--runner-path")
        .arg(&runner_path)
        .arg("--report-path")
        .arg(&report_path)
        .arg("--python-command")
        .arg(default_python_command())
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "privacy filter corpus report exceeded limit",
        ));

    assert!(!report_path.exists(), "oversized report should be removed");
}

#[test]
fn privacy_filter_text_redacts_report_path_in_stdout_summary() {
    let dir = tempdir().unwrap();
    let phi_named_dir = dir.path().join("Jane-Example-MRN-12345");
    fs::create_dir(&phi_named_dir).unwrap();
    let report_path = phi_named_dir.join("privacy-filter-report.json");
    let input_path = repo_path("scripts/privacy_filter/fixtures/sample_text_input.txt");
    let runner_path = repo_path("scripts/privacy_filter/run_privacy_filter.py");

    let output = Command::cargo_bin("mdid-cli")
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
        .success()
        .get_output()
        .clone();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stdout.contains("privacy-filter-text"));
    assert!(stdout.contains("<redacted>"));
    assert!(stdout.contains("\"report_written\":true"));
    assert!(!stdout.contains(report_path.to_str().unwrap()));
    assert!(!stdout.contains("Jane-Example-MRN-12345"));
    assert!(!stderr.contains(report_path.to_str().unwrap()));
    assert!(!stderr.contains("Jane-Example-MRN-12345"));
    assert!(stderr.is_empty());
    assert!(report_path.exists());

    let report_text = fs::read_to_string(&report_path).unwrap();
    assert!(report_text.contains("fallback_synthetic_patterns"));
    assert!(!report_text.contains("Jane-Example-MRN-12345"));
}

#[test]
fn cli_privacy_filter_text_summary_output_is_phi_safe() {
    let dir = tempdir().unwrap();
    let phi_named_dir = dir
        .path()
        .join("Jane-Example-MRN-12345-jane@example.com-555-123-4567");
    fs::create_dir(&phi_named_dir).unwrap();
    let report_path = phi_named_dir.join("privacy-filter-report.json");
    let summary_path = phi_named_dir.join("privacy-filter-summary.json");

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("privacy-filter-text")
        .arg("--input-path")
        .arg(repo_path(
            "scripts/privacy_filter/fixtures/sample_text_input.txt",
        ))
        .arg("--runner-path")
        .arg(repo_path("scripts/privacy_filter/run_privacy_filter.py"))
        .arg("--report-path")
        .arg(&report_path)
        .arg("--summary-output")
        .arg(&summary_path)
        .arg("--python-command")
        .arg(default_python_command())
        .arg("--mock")
        .assert()
        .success()
        .get_output()
        .clone();

    assert!(summary_path.exists());
    let summary_text = fs::read_to_string(&summary_path).unwrap();
    let summary: Value = serde_json::from_str(&summary_text).unwrap();
    let keys: Vec<&str> = summary
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    for expected in [
        "artifact",
        "schema_version",
        "scope",
        "engine",
        "network_api_called",
        "preview_policy",
        "input_char_count",
        "detected_span_count",
        "category_counts",
        "non_goals",
    ] {
        assert!(keys.contains(&expected), "summary missing {expected}");
    }
    assert_eq!(keys.len(), 10);
    assert_eq!(summary["artifact"], "privacy_filter_text_summary");
    assert_eq!(summary["schema_version"], 1);
    assert_eq!(summary["scope"], "text_only_single_report_summary");
    assert_eq!(summary["engine"], "fallback_synthetic_patterns");
    assert_eq!(summary["network_api_called"], false);
    assert_eq!(summary["preview_policy"], "redacted_placeholders_only");
    assert_eq!(summary["input_char_count"], 137);
    assert_eq!(summary["detected_span_count"], 5);
    assert_eq!(summary["category_counts"]["NAME"], 1);
    assert_eq!(summary["category_counts"]["DATE"], 1);
    for non_goal in [
        "ocr",
        "visual_redaction",
        "image_pixel_redaction",
        "final_pdf_rewrite_export",
        "browser_ui",
        "desktop_ui",
    ] {
        assert!(summary["non_goals"]
            .as_array()
            .unwrap()
            .contains(&Value::String(non_goal.to_string())));
    }

    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stdout.contains("<redacted>"));
    assert!(!stdout.contains(summary_path.to_str().unwrap()));
    for unsafe_text in [
        "Jane Example",
        "jane@example.com",
        "+1-555-123-4567",
        "555-123-4567",
        "MRN-12345",
        "2026-04-30",
        "masked_text",
        "spans",
        "\"preview\"",
        phi_named_dir.to_str().unwrap(),
    ] {
        assert!(
            !summary_text.contains(unsafe_text),
            "summary leaked {unsafe_text}"
        );
        assert!(!stdout.contains(unsafe_text), "stdout leaked {unsafe_text}");
        assert!(!stderr.contains(unsafe_text), "stderr leaked {unsafe_text}");
    }
}

#[test]
fn privacy_filter_text_rejects_same_report_and_summary_path_before_cleanup() {
    let bin = assert_cmd::cargo::cargo_bin("mdid-cli");
    let dir = tempfile::tempdir().expect("tempdir");
    let output_path = dir.path().join("Jane-Example-MRN-12345-output.json");
    fs::write(&output_path, "stale Jane Example MRN-12345").expect("write stale output");

    Command::new(&bin)
        .args([
            "privacy-filter-text",
            "--input-path",
            repo_path("scripts/privacy_filter/fixtures/sample_text_input.txt").as_str(),
            "--runner-path",
            repo_path("scripts/privacy_filter/run_privacy_filter.py").as_str(),
            "--report-path",
        ])
        .arg(&output_path)
        .arg("--summary-output")
        .arg(&output_path)
        .args(["--python-command", default_python_command(), "--mock"])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "privacy filter summary path must differ from report path",
        ))
        .stderr(predicate::str::contains("Jane Example").not())
        .stderr(predicate::str::contains("MRN-12345").not())
        .stderr(predicate::str::contains(output_path.to_string_lossy().as_ref()).not());

    assert_eq!(
        fs::read_to_string(&output_path).expect("stale output retained"),
        "stale Jane Example MRN-12345"
    );
}

#[test]
fn privacy_filter_text_rejects_alias_report_and_summary_path_before_cleanup() {
    let bin = assert_cmd::cargo::cargo_bin("mdid-cli");
    let dir = tempfile::tempdir().expect("tempdir");
    let output_path = dir.path().join("privacy-filter-report.json");
    let alias_path = dir.path().join(".").join("privacy-filter-report.json");
    fs::write(&output_path, "stale Jane Example MRN-12345").expect("write stale output");

    Command::new(&bin)
        .args([
            "privacy-filter-text",
            "--input-path",
            repo_path("scripts/privacy_filter/fixtures/sample_text_input.txt").as_str(),
            "--runner-path",
            repo_path("scripts/privacy_filter/run_privacy_filter.py").as_str(),
            "--report-path",
        ])
        .arg(&output_path)
        .arg("--summary-output")
        .arg(&alias_path)
        .args(["--python-command", default_python_command(), "--mock"])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "privacy filter summary path must differ from report path",
        ))
        .stderr(predicate::str::contains("Jane Example").not())
        .stderr(predicate::str::contains("MRN-12345").not())
        .stderr(predicate::str::contains(output_path.to_string_lossy().as_ref()).not())
        .stderr(predicate::str::contains(alias_path.to_string_lossy().as_ref()).not());

    assert_eq!(
        fs::read_to_string(&output_path).expect("stale output retained"),
        "stale Jane Example MRN-12345"
    );
}

#[test]
fn cli_privacy_filter_text_summary_output_removes_stale_file_on_prerequisite_failure() {
    let dir = tempdir().unwrap();
    let phi_named_dir = dir
        .path()
        .join("Jane-Example-MRN-12345-jane@example.com-555-123-4567");
    fs::create_dir(&phi_named_dir).unwrap();
    let report_path = phi_named_dir.join("privacy-filter-report.json");
    let summary_path = phi_named_dir.join("privacy-filter-summary.json");
    fs::write(&summary_path, "stale Jane Example").unwrap();

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("privacy-filter-text")
        .arg("--input-path")
        .arg(phi_named_dir.join("missing-input.txt"))
        .arg("--runner-path")
        .arg(repo_path("scripts/privacy_filter/run_privacy_filter.py"))
        .arg("--report-path")
        .arg(&report_path)
        .arg("--summary-output")
        .arg(&summary_path)
        .assert()
        .failure()
        .get_output()
        .clone();

    assert!(!summary_path.exists());
    assert!(!report_path.exists());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    for unsafe_text in ["Jane Example", phi_named_dir.to_str().unwrap()] {
        assert!(!stdout.contains(unsafe_text));
        assert!(!stderr.contains(unsafe_text));
    }
}

#[test]
fn cli_privacy_filter_text_summary_rejects_bad_counts_and_removes_stale_files() {
    for (case_name, category_counts) in [
        (
            "unsafe-summary-count-key",
            r#"{"NAME": 1, "PATIENT_JANE_EXAMPLE": 1}"#,
        ),
        ("malformed-summary-count", r#"{"NAME": "one"}"#),
    ] {
        let dir = tempdir().unwrap();
        let phi_named_dir = dir
            .path()
            .join(format!("{case_name}-Jane-Example-MRN-12345"));
        fs::create_dir(&phi_named_dir).unwrap();
        let input_path = phi_named_dir.join("input.txt");
        let runner_path = phi_named_dir.join("privacy_runner.py");
        let report_path = phi_named_dir.join("privacy-filter-report.json");
        let summary_path = phi_named_dir.join("privacy-filter-summary.json");
        fs::write(&input_path, "Jane Example MRN-12345").unwrap();
        fs::write(&report_path, "stale Jane Example report").unwrap();
        fs::write(&summary_path, "stale Jane Example summary").unwrap();
        fs::write(
            &runner_path,
            format!(
                r#"
import json
import sys

_ = sys.argv[1]
report = {{
    "metadata": {{
        "engine": "fallback_synthetic_patterns",
        "network_api_called": False,
        "preview_policy": "redacted_placeholders_only",
    }},
    "summary": {{
        "input_char_count": 23,
        "detected_span_count": 2,
        "category_counts": {category_counts},
    }},
    "masked_text": "[NAME] [MRN]",
    "spans": [],
}}
print(json.dumps(report))
"#
            ),
        )
        .unwrap();

        let output = Command::cargo_bin("mdid-cli")
            .unwrap()
            .arg("privacy-filter-text")
            .arg("--input-path")
            .arg(&input_path)
            .arg("--runner-path")
            .arg(&runner_path)
            .arg("--report-path")
            .arg(&report_path)
            .arg("--summary-output")
            .arg(&summary_path)
            .arg("--python-command")
            .arg(default_python_command())
            .assert()
            .failure()
            .get_output()
            .clone();

        assert!(!report_path.exists(), "invalid report should be removed");
        assert!(!summary_path.exists(), "invalid summary should be removed");
        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        for unsafe_text in [
            "Jane Example",
            "MRN-12345",
            "PATIENT_JANE_EXAMPLE",
            phi_named_dir.to_str().unwrap(),
        ] {
            assert!(!stdout.contains(unsafe_text), "stdout leaked {unsafe_text}");
            assert!(!stderr.contains(unsafe_text), "stderr leaked {unsafe_text}");
        }
    }
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
fn privacy_filter_text_detects_dates_from_stdin_without_raw_date_leaks() {
    let dir = tempdir().unwrap();
    let report_path = dir.path().join("privacy-filter-date-report.json");
    let runner_path = repo_path("scripts/privacy_filter/run_privacy_filter.py");
    let stdin_phi = "Patient Jane Example DOB 1978-04-23 seen on 04/23/1978 MRN-12345\n";

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("privacy-filter-text")
        .arg("--stdin")
        .arg("--runner-path")
        .arg(&runner_path)
        .arg("--report-path")
        .arg(&report_path)
        .arg("--python-command")
        .arg(default_python_command())
        .arg("--mock")
        .write_stdin(stdin_phi)
        .assert()
        .success()
        .get_output()
        .clone();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    let report_text = fs::read_to_string(&report_path).unwrap();
    let report: Value = serde_json::from_str(&report_text).unwrap();
    assert_eq!(report["summary"]["category_counts"]["DATE"], 2);
    assert_eq!(report["summary"]["category_counts"]["NAME"], 1);
    assert_eq!(report["summary"]["category_counts"]["MRN"], 1);
    assert!(report["masked_text"].as_str().unwrap().contains("[DATE]"));
    for unsafe_text in [
        "Jane Example",
        "1978-04-23",
        "04/23/1978",
        "MRN-12345",
        report_path.to_str().unwrap(),
        dir.path().to_str().unwrap(),
    ] {
        assert!(!stdout.contains(unsafe_text), "stdout leaked {unsafe_text}");
        assert!(!stderr.contains(unsafe_text), "stderr leaked {unsafe_text}");
        assert!(
            !report_text.contains(unsafe_text),
            "report leaked {unsafe_text}"
        );
    }
}

#[test]
fn privacy_filter_text_detects_addresses_from_stdin_without_raw_address_leaks() {
    let dir = tempdir().unwrap();
    let report_path = dir.path().join("privacy-filter-address-report.json");
    let runner_path = repo_path("scripts/privacy_filter/run_privacy_filter.py");
    let stdin_phi =
        "Patient Jane Example lives at 123 Main St and follow-up mail goes to 456 Oak Avenue.\n";

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("privacy-filter-text")
        .arg("--stdin")
        .arg("--runner-path")
        .arg(&runner_path)
        .arg("--report-path")
        .arg(&report_path)
        .arg("--python-command")
        .arg(default_python_command())
        .arg("--mock")
        .write_stdin(stdin_phi)
        .assert()
        .success()
        .get_output()
        .clone();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    let report_text = fs::read_to_string(&report_path).unwrap();
    let report: Value = serde_json::from_str(&report_text).unwrap();
    assert_eq!(report["summary"]["category_counts"]["ADDRESS"], 2);
    assert!(report["masked_text"]
        .as_str()
        .unwrap()
        .contains("[ADDRESS]"));
    for span in report["spans"].as_array().unwrap() {
        assert_eq!(span["preview"], "<redacted>");
    }
    for unsafe_text in ["123 Main St", "456 Oak Avenue"] {
        assert!(!stdout.contains(unsafe_text), "stdout leaked {unsafe_text}");
        assert!(!stderr.contains(unsafe_text), "stderr leaked {unsafe_text}");
        assert!(
            !report_text.contains(unsafe_text),
            "report leaked {unsafe_text}"
        );
    }
}

#[test]
fn privacy_filter_text_accepts_stdin_without_leaking_input_path() {
    let dir = tempdir().unwrap();
    let report_path = dir.path().join("privacy-filter-report.json");
    let runner_path = repo_path("scripts/privacy_filter/run_privacy_filter.py");
    let stdin_phi = "Patient Jane Example MRN-12345 phone 555-123-4567\n";

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("privacy-filter-text")
        .arg("--stdin")
        .arg("--runner-path")
        .arg(&runner_path)
        .arg("--report-path")
        .arg(&report_path)
        .arg("--python-command")
        .arg(default_python_command())
        .arg("--mock")
        .write_stdin(stdin_phi)
        .assert()
        .success()
        .get_output()
        .clone();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stdout.contains("privacy-filter-text"));
    assert!(stdout.contains("<redacted>"));
    assert!(stdout.contains("\"report_written\":true"));
    for unsafe_text in [
        "Jane Example",
        "MRN-12345",
        "555-123-4567",
        report_path.to_str().unwrap(),
        dir.path().to_str().unwrap(),
        "stdin",
        "input_path",
    ] {
        assert!(!stdout.contains(unsafe_text), "stdout leaked {unsafe_text}");
        assert!(!stderr.contains(unsafe_text), "stderr leaked {unsafe_text}");
    }
    assert!(report_path.exists());
}

#[test]
fn privacy_filter_text_stdin_pipes_to_runner_without_temp_input_file() {
    let dir = tempdir().unwrap();
    let runner_path = dir.path().join("stdin_only_privacy_runner.py");
    let report_path = dir.path().join("Alice-MRN-424242-report.json");
    let marker_path = dir.path().join("runner-proved-stdin.txt");
    fs::write(
        &runner_path,
        format!(
            r#"
import json
import pathlib
import sys

args = sys.argv[1:]
if args != ["--mock", "--stdin"]:
    print("expected --mock --stdin with no positional input path", file=sys.stderr)
    sys.exit(42)
text = sys.stdin.read()
if "Jane Sentinel" not in text or "MRN-424242" not in text:
    print("missing stdin sentinel", file=sys.stderr)
    sys.exit(43)
pathlib.Path({marker:?}).write_text("stdin-only", encoding="utf-8")
json.dump({{
    "summary": {{
        "input_char_count": len(text),
        "detected_span_count": 2,
        "category_counts": {{"NAME": 1, "MRN": 1}}
    }},
    "masked_text": "Patient [NAME] [MRN]",
    "spans": [
        {{"start": 8, "end": 21, "text": "[REDACTED]", "category": "NAME"}},
        {{"start": 22, "end": 32, "text": "[REDACTED]", "category": "MRN"}}
    ],
    "metadata": {{
        "engine": "fake_stdin_only_runner",
        "network_api_called": False,
        "preview_policy": "masked"
    }}
}}, sys.stdout)
"#,
            marker = marker_path.to_string_lossy()
        ),
    )
    .unwrap();
    let stdin_phi = "Patient Jane Sentinel MRN-424242 phone 555-424-2424\n";

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("privacy-filter-text")
        .arg("--stdin")
        .arg("--runner-path")
        .arg(&runner_path)
        .arg("--report-path")
        .arg(&report_path)
        .arg("--python-command")
        .arg(default_python_command())
        .arg("--mock")
        .write_stdin(stdin_phi)
        .assert()
        .success()
        .get_output()
        .clone();

    assert_eq!(fs::read_to_string(&marker_path).unwrap(), "stdin-only");
    let report_text = fs::read_to_string(&report_path).unwrap();
    let report: Value = serde_json::from_str(&report_text).unwrap();
    assert_eq!(report["metadata"]["engine"], "fake_stdin_only_runner");
    assert_eq!(report["summary"]["detected_span_count"], 2);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    for unsafe_text in [
        "Jane Sentinel",
        "MRN-424242",
        "555-424-2424",
        report_path.to_str().unwrap(),
        dir.path().to_str().unwrap(),
        "mdid-privacy-filter-stdin-",
    ] {
        assert!(!stdout.contains(unsafe_text), "stdout leaked {unsafe_text}");
        assert!(!stderr.contains(unsafe_text), "stderr leaked {unsafe_text}");
    }
}

#[test]
fn privacy_filter_text_does_not_materialize_stdin_temp_file_on_runner_failure_without_leaking_phi()
{
    let dir = tempdir().unwrap();
    let temp_root = dir.path().join("isolated-tmp");
    fs::create_dir(&temp_root).unwrap();
    let runner_path = dir.path().join("failing_privacy_runner.py");
    let report_path = dir.path().join("privacy-filter-report.json");
    let marker_path = dir.path().join("runner-input-mode.txt");
    fs::write(
        &runner_path,
        format!(
            "import pathlib, sys\nassert sys.argv[1:] == ['--stdin']\ntext = sys.stdin.read()\nassert 'Jane Example' in text\npathlib.Path({marker:?}).write_text('stdin', encoding='utf-8')\nsys.exit(42)\n",
            marker = marker_path.to_string_lossy()
        ),
    )
    .unwrap();
    let stdin_phi = "Patient Jane Example MRN-12345 phone 555-123-4567\n";

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("privacy-filter-text")
        .arg("--stdin")
        .arg("--runner-path")
        .arg(&runner_path)
        .arg("--report-path")
        .arg(&report_path)
        .arg("--python-command")
        .arg(default_python_command())
        .env("TMPDIR", &temp_root)
        .env("TEMP", &temp_root)
        .env("TMP", &temp_root)
        .write_stdin(stdin_phi)
        .assert()
        .failure()
        .get_output()
        .clone();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    for unsafe_text in ["Jane Example", "MRN-12345", "555-123-4567"] {
        assert!(!stdout.contains(unsafe_text), "stdout leaked {unsafe_text}");
        assert!(!stderr.contains(unsafe_text), "stderr leaked {unsafe_text}");
    }
    assert_eq!(fs::read_to_string(&marker_path).unwrap(), "stdin");
    let leaked_temp_files = fs::read_dir(&temp_root).unwrap().collect::<Vec<_>>();
    assert!(
        leaked_temp_files.is_empty(),
        "stdin temp files were created: {leaked_temp_files:?}"
    );
}

#[test]
fn privacy_filter_text_rejects_empty_stdin() {
    let dir = tempdir().unwrap();
    let report_path = dir.path().join("privacy-filter-report.json");

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("privacy-filter-text")
        .arg("--stdin")
        .arg("--runner-path")
        .arg(repo_path("scripts/privacy_filter/run_privacy_filter.py"))
        .arg("--report-path")
        .arg(&report_path)
        .write_stdin("")
        .assert()
        .failure()
        .stderr(predicate::str::contains("missing stdin input"));
}

#[test]
fn privacy_filter_text_rejects_oversized_stdin() {
    let dir = tempdir().unwrap();
    let report_path = dir.path().join("privacy-filter-report.json");
    let oversized_stdin = "x".repeat(1024 * 1024 + 1);

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("privacy-filter-text")
        .arg("--stdin")
        .arg("--runner-path")
        .arg(repo_path("scripts/privacy_filter/run_privacy_filter.py"))
        .arg("--report-path")
        .arg(&report_path)
        .write_stdin(oversized_stdin)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "stdin input exceeds 1048576 byte limit",
        ));
}

#[test]
fn privacy_filter_text_rejects_missing_input_source() {
    let dir = tempdir().unwrap();
    let report_path = dir.path().join("privacy-filter-report.json");

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("privacy-filter-text")
        .arg("--runner-path")
        .arg(repo_path("scripts/privacy_filter/run_privacy_filter.py"))
        .arg("--report-path")
        .arg(&report_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "missing input source: provide exactly one of --input-path or --stdin",
        ));
}

#[test]
fn privacy_filter_text_rejects_both_input_path_and_stdin() {
    let dir = tempdir().unwrap();
    let input_path = dir.path().join("synthetic-input.txt");
    let report_path = dir.path().join("privacy-filter-report.json");
    fs::write(&input_path, "Patient Jane Example has MRN-123\n").unwrap();

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("privacy-filter-text")
        .arg("--input-path")
        .arg(&input_path)
        .arg("--stdin")
        .arg("--runner-path")
        .arg(repo_path("scripts/privacy_filter/run_privacy_filter.py"))
        .arg("--report-path")
        .arg(&report_path)
        .write_stdin("Patient Jane Example MRN-12345\n")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "conflicting input sources: provide exactly one of --input-path or --stdin",
        ));
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

#[test]
fn privacy_filter_text_stdin_fails_when_runner_exits_before_reading_stdin_without_false_success() {
    let dir = tempdir().unwrap();
    let phi_named_dir = dir
        .path()
        .join("Jane-Example-MRN-12345-jane@example.com-555-123-4567");
    fs::create_dir(&phi_named_dir).unwrap();
    let runner_path = phi_named_dir.join("privacy_runner.py");
    let report_path = phi_named_dir.join("privacy-report.json");
    fs::write(&report_path, "stale report must be removed").unwrap();
    write_privacy_runner(
        &runner_path,
        r#"import json
import sys
json.dump({
    "summary": {"input_char_count": 0, "detected_span_count": 0, "category_counts": {}},
    "masked_text": "",
    "spans": [],
    "metadata": {"engine": "fake_quick_no_stdin_runner", "network_api_called": False, "preview_policy": "masked"}
}, sys.stdout)
"#,
    );
    let stdin_text = format!(
        "Patient Jane Example has MRN-12345. {}\n",
        "x".repeat(1024 * 1024 - 42)
    );

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("privacy-filter-text")
        .arg("--stdin")
        .arg("--runner-path")
        .arg(&runner_path)
        .arg("--report-path")
        .arg(&report_path)
        .write_stdin(stdin_text)
        .assert()
        .failure()
        .get_output()
        .clone();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(!stdout.contains("report_written"));
    assert!(!stdout.contains("fake_quick_no_stdin_runner"));
    for unsafe_text in ["Jane Example", "MRN-12345", phi_named_dir.to_str().unwrap()] {
        assert!(!stdout.contains(unsafe_text), "stdout leaked {unsafe_text}");
        assert!(!stderr.contains(unsafe_text), "stderr leaked {unsafe_text}");
    }
    assert!(!report_path.exists());
}

#[test]
fn privacy_filter_text_stdin_times_out_when_runner_does_not_read_stdin_and_removes_stale_report() {
    let dir = tempdir().unwrap();
    let phi_named_dir = dir
        .path()
        .join("Jane-Example-MRN-12345-jane@example.com-555-123-4567");
    fs::create_dir(&phi_named_dir).unwrap();
    let runner_path = phi_named_dir.join("privacy_runner.py");
    let report_path = phi_named_dir.join("privacy-report.json");
    fs::write(&report_path, "stale report must be removed").unwrap();
    write_privacy_runner(&runner_path, "import time\ntime.sleep(30)\n");
    let stdin_text = format!(
        "Patient Jane Example has MRN-12345. {}\n",
        "x".repeat(1024 * 1024 - 42)
    );

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .timeout(Duration::from_secs(5))
        .arg("privacy-filter-text")
        .arg("--stdin")
        .arg("--runner-path")
        .arg(&runner_path)
        .arg("--report-path")
        .arg(&report_path)
        .write_stdin(stdin_text)
        .assert()
        .failure()
        .stderr(predicate::str::contains("privacy filter runner timed out"))
        .stderr(predicate::str::contains("Jane Example").not())
        .stderr(predicate::str::contains(phi_named_dir.to_str().unwrap()).not())
        .stdout(predicate::str::contains("Jane Example").not())
        .stdout(predicate::str::contains(phi_named_dir.to_str().unwrap()).not());

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
            "missing input source: provide exactly one of --input-path or --stdin",
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
    let expected_usage = "mdid-cli ocr-handoff --image-path <path> --ocr-runner-path <path> --handoff-builder-path <path> --report-path <report.json> [--summary-output <summary.json>] [--python-command <cmd>]";
    Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("--help")
        .assert()
        .failure()
        .stderr(predicate::str::contains(expected_usage));
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
    assert_eq!(summary["report_path"], "<redacted>");
    assert_eq!(summary["report_written"], true);
    assert!(!String::from_utf8_lossy(&stdout).contains(report_path.to_str().unwrap()));

    let rendered_report = fs::read_to_string(&report_path).unwrap();
    assert!(!rendered_report.contains("synthetic_printed_phi_line.png"));
    let report: Value = serde_json::from_str(&rendered_report).unwrap();
    assert_eq!(report["source"], "<redacted>");
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
fn ocr_handoff_writes_phi_safe_summary_output() {
    let dir = tempdir().unwrap();
    let report_path = dir.path().join("ocr-handoff-report.json");
    let summary_path = dir.path().join("ocr-handoff-summary.json");

    let output = Command::cargo_bin("mdid-cli")
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
            "--summary-output",
            summary_path.to_str().unwrap(),
            "--python-command",
            default_python_command(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stdout.contains(summary_path.to_str().unwrap()));
    assert!(!stderr.contains(summary_path.to_str().unwrap()));

    let summary_text = fs::read_to_string(&summary_path).unwrap();
    let summary: serde_json::Value = serde_json::from_str(&summary_text).unwrap();
    assert_eq!(summary["artifact"], "ocr_handoff_summary");
    assert_eq!(summary["schema_version"], 1);
    assert_eq!(summary["candidate"], "PP-OCRv5_mobile_rec");
    assert_eq!(summary["engine"], "PP-OCRv5-mobile-bounded-spike");
    assert_eq!(summary["scope"], "printed_text_line_extraction_only");
    assert_eq!(
        summary["privacy_filter_contract"],
        "text_only_normalized_input"
    );
    assert_eq!(summary["ready_for_text_pii_eval"], true);
    assert!(summary["line_count"].as_u64().unwrap() >= 1);
    assert!(summary["char_count"].as_u64().unwrap() >= 1);
    assert_eq!(summary["network_api_called"], false);
    assert!(summary["non_goals"]
        .as_array()
        .unwrap()
        .contains(&serde_json::Value::String("visual_redaction".to_string())));
    assert!(summary["non_goals"]
        .as_array()
        .unwrap()
        .contains(&serde_json::Value::String(
            "image_pixel_redaction".to_string()
        )));
    assert!(summary["non_goals"]
        .as_array()
        .unwrap()
        .contains(&serde_json::Value::String(
            "final_pdf_rewrite_export".to_string()
        )));

    let keys: std::collections::BTreeSet<_> =
        summary.as_object().unwrap().keys().cloned().collect();
    assert_eq!(
        keys,
        [
            "artifact",
            "candidate",
            "char_count",
            "engine",
            "line_count",
            "network_api_called",
            "non_goals",
            "privacy_filter_contract",
            "ready_for_text_pii_eval",
            "schema_version",
            "scope",
        ]
        .into_iter()
        .map(String::from)
        .collect()
    );

    for forbidden in [
        "Jane Example",
        "MRN-12345",
        "jane@example.com",
        "555-123-4567",
        "synthetic_printed_phi_line.png",
        "extracted_text",
        "normalized_text",
        "bbox",
        "image_bytes",
        report_path.to_str().unwrap(),
        summary_path.to_str().unwrap(),
    ] {
        assert!(
            !summary_text.contains(forbidden),
            "summary leaked {forbidden}"
        );
        assert!(!stdout.contains(forbidden), "stdout leaked {forbidden}");
        assert!(!stderr.contains(forbidden), "stderr leaked {forbidden}");
    }
}

#[test]
fn ocr_handoff_summary_output_rejects_same_report_and_summary_before_cleanup() {
    let dir = tempdir().unwrap();
    let image_path = dir.path().join("image.png");
    let runner_path = dir.path().join("runner.py");
    let builder_path = dir.path().join("builder.py");
    let report_path = dir.path().join("handoff.json");
    let summary_alias_path = dir.path().join(".").join("handoff.json");
    fs::write(&image_path, b"png").unwrap();
    fs::write(&runner_path, "print('Jane Example MRN-12345')\n").unwrap();
    fs::write(&builder_path, "print('not reached')\n").unwrap();
    fs::write(&report_path, "stale Jane Example report").unwrap();

    let output = Command::cargo_bin("mdid-cli")
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
            "--summary-output",
            summary_alias_path.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .get_output()
        .clone();

    assert_eq!(
        fs::read_to_string(&report_path).unwrap(),
        "stale Jane Example report"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.is_empty());
    assert_eq!(
        stderr.trim(),
        "OCR handoff summary path must differ from report path"
    );
    for forbidden in [
        "Jane Example",
        "MRN-12345",
        image_path.to_str().unwrap(),
        runner_path.to_str().unwrap(),
        builder_path.to_str().unwrap(),
        report_path.to_str().unwrap(),
    ] {
        assert!(!stdout.contains(forbidden), "stdout leaked {forbidden}");
        assert!(!stderr.contains(forbidden), "stderr leaked {forbidden}");
    }
}

#[test]
fn ocr_handoff_summary_output_missing_image_removes_stale_summary_without_leaks() {
    let dir = tempdir().unwrap();
    let runner_path = dir.path().join("runner.py");
    let builder_path = dir.path().join("builder.py");
    let report_path = dir.path().join("handoff.json");
    let summary_path = dir.path().join("summary.json");
    let missing_image_path = dir.path().join("Jane-Example-MRN-12345.png");
    fs::write(&runner_path, "print('Jane Example MRN-12345')\n").unwrap();
    fs::write(&builder_path, "print('not reached')\n").unwrap();
    fs::write(&report_path, "stale Jane Example report").unwrap();
    fs::write(&summary_path, "stale Jane Example summary").unwrap();

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-handoff",
            "--image-path",
            missing_image_path.to_str().unwrap(),
            "--ocr-runner-path",
            runner_path.to_str().unwrap(),
            "--handoff-builder-path",
            builder_path.to_str().unwrap(),
            "--report-path",
            report_path.to_str().unwrap(),
            "--summary-output",
            summary_path.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .get_output()
        .clone();

    assert!(!report_path.exists());
    assert!(!summary_path.exists());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("missing image file"));
    for forbidden in [
        "Jane Example",
        "MRN-12345",
        missing_image_path.to_str().unwrap(),
        runner_path.to_str().unwrap(),
        builder_path.to_str().unwrap(),
        report_path.to_str().unwrap(),
        summary_path.to_str().unwrap(),
    ] {
        assert!(!stdout.contains(forbidden), "stdout leaked {forbidden}");
        assert!(!stderr.contains(forbidden), "stderr leaked {forbidden}");
    }
}

#[test]
fn ocr_handoff_rejects_not_ready_builder_report_without_stale_outputs_or_leaks() {
    let dir = tempdir().unwrap();
    let image_path = dir.path().join("image.png");
    let runner_path = dir.path().join("runner.py");
    let builder_path = dir.path().join("builder.py");
    let report_path = dir.path().join("handoff.json");
    let summary_path = dir.path().join("summary.json");
    fs::write(&image_path, b"png").unwrap();
    fs::write(&runner_path, "print('Jane Example MRN-12345')\n").unwrap();
    fs::write(
        &builder_path,
        r#"import argparse, json
parser = argparse.ArgumentParser()
parser.add_argument("--source")
parser.add_argument("--input")
parser.add_argument("--output")
args = parser.parse_args()
with open(args.output, "w", encoding="utf-8") as handle:
    json.dump({
        "source": args.source,
        "extracted_text": "Jane Example MRN-12345",
        "normalized_text": "Jane Example MRN-12345",
        "ready_for_text_pii_eval": False,
        "candidate": "PP-OCRv5_mobile_rec",
        "engine": "PP-OCRv5-mobile-bounded-spike",
        "scope": "printed_text_line_extraction_only",
        "privacy_filter_contract": "text_only_normalized_input",
        "non_goals": ["visual_redaction", "final_pdf_rewrite_export", "handwriting_recognition", "full_page_detection_or_segmentation", "complete_ocr_pipeline"]
    }, handle)
"#,
    )
    .unwrap();
    fs::write(&report_path, "stale Jane Example report").unwrap();
    fs::write(&summary_path, "stale Jane Example summary").unwrap();

    let output = Command::cargo_bin("mdid-cli")
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
            "--summary-output",
            summary_path.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .get_output()
        .clone();

    assert!(!report_path.exists());
    assert!(!summary_path.exists());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("OCR handoff has invalid readiness"));
    for forbidden in [
        "Jane Example",
        "MRN-12345",
        image_path.to_str().unwrap(),
        runner_path.to_str().unwrap(),
        builder_path.to_str().unwrap(),
        report_path.to_str().unwrap(),
        summary_path.to_str().unwrap(),
    ] {
        assert!(!stdout.contains(forbidden), "stdout leaked {forbidden}");
        assert!(!stderr.contains(forbidden), "stderr leaked {forbidden}");
    }
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
fn ocr_handoff_times_out_silent_hanging_builder_and_removes_stale_outputs() {
    let dir = tempdir().unwrap();
    let image_path = dir.path().join("image.png");
    let runner_path = dir.path().join("runner.py");
    let builder_path = dir.path().join("builder.py");
    let report_path = dir.path().join("handoff.json");
    let temp_path = report_path.with_extension("json.ocr-text.tmp");
    fs::write(&image_path, b"png").unwrap();
    fs::write(&runner_path, "print('Jane Doe')\n").unwrap();
    fs::write(&builder_path, "import time\ntime.sleep(30)\n").unwrap();
    fs::write(&report_path, "stale report must be removed").unwrap();
    fs::write(&temp_path, "stale temp must be removed").unwrap();

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
        .stderr(predicate::str::contains("OCR handoff builder timed out"));
    assert!(!report_path.exists());
    assert!(!temp_path.exists());
}

#[test]
fn docs_ocr_privacy_chain_uses_handoff_normalized_text_file() {
    let repo_readme = fs::read_to_string(repo_path("README.md")).unwrap();
    let ocr_readme = fs::read_to_string(repo_path("scripts/ocr_eval/README.md")).unwrap();
    let research_results =
        fs::read_to_string(repo_path("docs/research/small-ocr-spike-results.md")).unwrap();

    for docs in [&repo_readme, &ocr_readme, &research_results] {
        assert!(docs.contains("/tmp/ocr-normalized-text.txt"));
        assert!(docs.contains("Path('/tmp/ocr-handoff.json')"));
        assert!(docs.contains("['normalized_text']"));
        assert!(docs.contains("python scripts/privacy_filter/run_privacy_filter.py --mock /tmp/ocr-normalized-text.txt"));
    }
    assert!(!repo_readme.contains("run_privacy_filter.py --mock /tmp/small-ocr-output.txt"));
    assert!(!ocr_readme.contains("run_privacy_filter.py --mock /tmp/small-ocr-output.txt"));
    assert!(!research_results.contains("run_privacy_filter.py --mock /tmp/small-ocr-output.txt"));
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

#[test]
fn ocr_privacy_evidence_docs_include_exact_wrapper_command_and_scope_limits() {
    let ocr_readme = fs::read_to_string(repo_path("scripts/ocr_eval/README.md")).unwrap();
    let repo_readme = fs::read_to_string(repo_path("README.md")).unwrap();
    assert!(ocr_readme.contains(r#"cargo run -p mdid-cli -- ocr-privacy-evidence \"#));
    assert!(ocr_readme
        .contains(r#"  --image-path scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png \"#));
    assert!(
        ocr_readme.contains(r#"  --runner-path scripts/ocr_eval/run_ocr_privacy_evidence.py \"#)
    );
    assert!(ocr_readme.contains(r#"  --output /tmp/ocr-privacy-evidence.json \"#));
    assert!(ocr_readme.contains(r#"  --summary-output /tmp/ocr-privacy-evidence-summary.json \"#));
    assert!(ocr_readme.contains("--summary-output <summary.json>"));
    assert!(ocr_readme.contains("ocr_privacy_evidence_summary"));
    assert!(ocr_readme.contains(r#"  --python-command python3 \"#));
    assert!(ocr_readme.contains("  --mock"));
    assert!(ocr_readme.contains("aggregate-only, PHI-safe, and CLI/runtime evidence only"));
    assert!(ocr_readme.contains("omits raw OCR text, normalized text, masked text, spans/previews, fixture paths/filenames/IDs, local paths"));
    assert!(ocr_readme.contains("not Browser/Web execution"));
    assert!(ocr_readme.contains("not Desktop execution"));
    assert!(ocr_readme.contains("not OCR model-quality proof"));
    assert!(ocr_readme.contains("not visual redaction"));
    assert!(ocr_readme.contains("not image pixel redaction"));
    assert!(ocr_readme.contains("not handwriting recognition"));
    assert!(ocr_readme.contains("not final PDF rewrite/export"));
    assert!(repo_readme.contains("ocr_privacy_evidence_summary"));
    assert!(repo_readme.contains("CLI `109/114 -> 110/115 = 95%` floor"));
    assert!(repo_readme.contains("Browser/Web +0% and Desktop +0%"));
    assert!(repo_readme.contains("no new Browser/Desktop capability landed"));
}

#[test]
fn ocr_privacy_evidence_help_contains_exact_usage_line_and_command_detail() {
    Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("--help")
        .assert()
        .failure()
        .stderr(predicate::str::contains("mdid-cli ocr-privacy-evidence --image-path <image> --runner-path <runner.py> --output <report.json> [--summary-output <summary.json>] [--python-command <cmd>] [--mock]"))
        .stderr(predicate::str::contains("Commands:\n  status"))
        .stderr(predicate::str::contains("ocr-privacy-evidence Run local OCR privacy evidence and write a bounded PHI-safe JSON report."));
}

#[test]
fn ocr_privacy_evidence_runs_checked_in_fixture_without_phi_or_path_leaks() {
    let dir = tempdir().unwrap();
    let phi_named_dir = dir
        .path()
        .join("Jane-Example-MRN-12345-jane@example.com-555-123-4567");
    fs::create_dir(&phi_named_dir).unwrap();
    let report_path = phi_named_dir.join("ocr-privacy-evidence-report.json");
    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-privacy-evidence",
            "--image-path",
            &repo_path("scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png"),
            "--runner-path",
            &repo_path("scripts/ocr_eval/run_ocr_privacy_evidence.py"),
            "--output",
            report_path.to_str().unwrap(),
            "--python-command",
            default_python_command(),
            "--mock",
        ])
        .assert()
        .success()
        .get_output()
        .clone();
    assert!(report_path.exists());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stdout.contains("ocr-privacy-evidence"));
    assert!(stdout.contains("\"report_path\":\"<redacted>\""));
    assert!(stdout.contains("\"report_written\":true"));
    assert!(stderr.is_empty());
    let report_text = fs::read_to_string(&report_path).unwrap();
    let report: Value = serde_json::from_str(&report_text).unwrap();
    assert_eq!(report["artifact"], "ocr_privacy_evidence");
    assert_eq!(report["ocr_scope"], "printed_text_line_extraction_only");
    assert_eq!(report["privacy_scope"], "text_only_pii_detection");
    assert_eq!(report["network_api_called"], false);
    for unsafe_text in [
        report_path.to_str().unwrap(),
        phi_named_dir.to_str().unwrap(),
        "Jane Example",
        "MRN-12345",
        "jane@example.com",
        "555-123-4567",
        "synthetic_printed_phi_line.png",
        "run_ocr_privacy_evidence.py",
        "/tmp/",
    ] {
        assert!(!stdout.contains(unsafe_text), "stdout leaked {unsafe_text}");
        assert!(!stderr.contains(unsafe_text), "stderr leaked {unsafe_text}");
        assert!(
            !report_text.contains(unsafe_text),
            "report leaked {unsafe_text}"
        );
    }
}

#[test]
fn ocr_privacy_evidence_writes_phi_safe_summary_output() {
    let dir = tempdir().unwrap();
    let phi_named_dir = dir
        .path()
        .join("Jane-Example-MRN-12345-jane@example.com-555-123-4567");
    fs::create_dir(&phi_named_dir).unwrap();
    let report_path = phi_named_dir.join("ocr-privacy-evidence-report.json");
    let summary_path = phi_named_dir.join("ocr-privacy-evidence-summary.json");
    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-privacy-evidence",
            "--image-path",
            &repo_path("scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png"),
            "--runner-path",
            &repo_path("scripts/ocr_eval/run_ocr_privacy_evidence.py"),
            "--output",
            report_path.to_str().unwrap(),
            "--summary-output",
            summary_path.to_str().unwrap(),
            "--python-command",
            default_python_command(),
            "--mock",
        ])
        .assert()
        .success()
        .get_output()
        .clone();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stdout.contains("\"summary_written\":true"));
    assert!(stderr.is_empty());
    let summary_text = fs::read_to_string(&summary_path).unwrap();
    let summary: Value = serde_json::from_str(&summary_text).unwrap();
    let mut actual_keys = summary
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>();
    actual_keys.sort_unstable();
    let mut expected_keys = vec![
        "artifact",
        "category_counts",
        "network_api_called",
        "non_goals",
        "ocr_scope",
        "privacy_filter_contract",
        "privacy_scope",
        "ready_for_text_pii_eval",
        "schema_version",
        "total_detected_span_count",
    ];
    expected_keys.sort_unstable();
    assert_eq!(actual_keys, expected_keys);
    assert_eq!(summary["artifact"], "ocr_privacy_evidence_summary");
    assert_eq!(summary["schema_version"], 1);
    assert_eq!(summary["ocr_scope"], "printed_text_line_extraction_only");
    assert_eq!(summary["privacy_scope"], "text_only_pii_detection");
    assert_eq!(summary["network_api_called"], false);
    assert_eq!(summary["category_counts"]["NAME"], 1);
    assert_eq!(summary["category_counts"]["MRN"], 1);
    assert_eq!(summary["category_counts"]["EMAIL"], 1);
    assert_eq!(summary["category_counts"]["PHONE"], 1);
    for forbidden_key in [
        "fixtures",
        "report_path",
        "normalized_text",
        "masked_text",
        "spans",
        "raw_ocr_text",
        "raw_text",
        "text",
        "preview",
        "previews",
        "fixture_ids",
        "fixture_filenames",
        "filenames",
        "paths",
        "image_path",
        "runner_path",
        "output",
        "summary_output",
    ] {
        assert!(
            summary.get(forbidden_key).is_none(),
            "summary included {forbidden_key}"
        );
    }
    for unsafe_text in [
        report_path.to_str().unwrap(),
        summary_path.to_str().unwrap(),
        phi_named_dir.to_str().unwrap(),
        "Jane Example",
        "MRN-12345",
        "jane@example.com",
        "555-123-4567",
        "synthetic_printed_phi_line.png",
        "run_ocr_privacy_evidence.py",
        "/tmp/",
    ] {
        assert!(!stdout.contains(unsafe_text), "stdout leaked {unsafe_text}");
        assert!(!stderr.contains(unsafe_text), "stderr leaked {unsafe_text}");
        assert!(
            !summary_text.contains(unsafe_text),
            "summary leaked {unsafe_text}"
        );
    }
}

#[test]
fn ocr_privacy_evidence_summary_output_rejects_same_report_and_summary_before_cleanup() {
    let dir = tempdir().unwrap();
    let report_path = dir.path().join("report.json");
    fs::write(&report_path, "stale primary report bytes").unwrap();
    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-privacy-evidence",
            "--image-path",
            &repo_path("scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png"),
            "--runner-path",
            &repo_path("scripts/ocr_eval/run_ocr_privacy_evidence.py"),
            "--output",
            report_path.to_str().unwrap(),
            "--summary-output",
            report_path.to_str().unwrap(),
            "--python-command",
            default_python_command(),
            "--mock",
        ])
        .assert()
        .failure()
        .get_output()
        .clone();
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "");
    assert_eq!(
        String::from_utf8(output.stderr).unwrap().trim_end(),
        "ocr privacy evidence summary path must differ from report path"
    );
    assert_eq!(
        fs::read_to_string(&report_path).unwrap(),
        "stale primary report bytes"
    );
}

#[test]
fn ocr_privacy_evidence_summary_output_rejects_alias_report_and_summary_before_cleanup() {
    let dir = tempdir().unwrap();
    let report_path = dir.path().join("report.json");
    fs::write(&report_path, "stale primary report bytes").unwrap();
    let alias = dir.path().join(".").join("report.json");
    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-privacy-evidence",
            "--image-path",
            &repo_path("scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png"),
            "--runner-path",
            &repo_path("scripts/ocr_eval/run_ocr_privacy_evidence.py"),
            "--output",
            report_path.to_str().unwrap(),
            "--summary-output",
            alias.to_str().unwrap(),
            "--python-command",
            default_python_command(),
            "--mock",
        ])
        .assert()
        .failure()
        .get_output()
        .clone();
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "");
    assert_eq!(
        String::from_utf8(output.stderr).unwrap().trim_end(),
        "ocr privacy evidence summary path must differ from report path"
    );
    assert_eq!(
        fs::read_to_string(&report_path).unwrap(),
        "stale primary report bytes"
    );
}

#[test]
fn ocr_privacy_evidence_summary_output_missing_image_removes_stale_summary_without_leaks() {
    let dir = tempdir().unwrap();
    let phi_named_dir = dir.path().join("Jane-Example-MRN-12345");
    fs::create_dir(&phi_named_dir).unwrap();
    let report_path = phi_named_dir.join("report.json");
    let summary_path = phi_named_dir.join("summary.json");
    fs::write(&summary_path, "stale Jane Example MRN-12345").unwrap();
    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-privacy-evidence",
            "--image-path",
            phi_named_dir.join("missing.png").to_str().unwrap(),
            "--runner-path",
            &repo_path("scripts/ocr_eval/run_ocr_privacy_evidence.py"),
            "--output",
            report_path.to_str().unwrap(),
            "--summary-output",
            summary_path.to_str().unwrap(),
            "--python-command",
            default_python_command(),
            "--mock",
        ])
        .assert()
        .failure()
        .get_output()
        .clone();
    assert!(!summary_path.exists());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    for unsafe_text in [
        summary_path.to_str().unwrap(),
        phi_named_dir.to_str().unwrap(),
        "Jane Example",
        "MRN-12345",
        "missing.png",
    ] {
        assert!(!stdout.contains(unsafe_text), "stdout leaked {unsafe_text}");
        assert!(!stderr.contains(unsafe_text), "stderr leaked {unsafe_text}");
    }
}

#[test]
fn ocr_privacy_evidence_missing_image_removes_stale_output_without_leaks() {
    let dir = tempdir().unwrap();
    let phi_named_dir = dir.path().join("Jane-Example-MRN-12345");
    fs::create_dir(&phi_named_dir).unwrap();
    let report_path = phi_named_dir.join("report.json");
    fs::write(&report_path, "stale Jane Example MRN-12345").unwrap();
    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-privacy-evidence",
            "--image-path",
            phi_named_dir.join("missing.png").to_str().unwrap(),
            "--runner-path",
            &repo_path("scripts/ocr_eval/run_ocr_privacy_evidence.py"),
            "--output",
            report_path.to_str().unwrap(),
            "--python-command",
            default_python_command(),
            "--mock",
        ])
        .assert()
        .failure()
        .get_output()
        .clone();
    assert!(!report_path.exists());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    for unsafe_text in [
        report_path.to_str().unwrap(),
        phi_named_dir.to_str().unwrap(),
        "Jane Example",
        "MRN-12345",
        "missing.png",
    ] {
        assert!(!stdout.contains(unsafe_text), "stdout leaked {unsafe_text}");
        assert!(!stderr.contains(unsafe_text), "stderr leaked {unsafe_text}");
    }
}

#[test]
fn ocr_privacy_evidence_rejects_syntactically_invalid_json_and_removes_output_without_leaks() {
    let dir = tempdir().unwrap();
    let phi_named_dir = dir.path().join("Jane-Example-MRN-12345");
    fs::create_dir(&phi_named_dir).unwrap();
    let image_path = phi_named_dir.join("fixture.png");
    fs::write(&image_path, b"image").unwrap();
    let runner_path = phi_named_dir.join("invalid_json_runner.py");
    fs::write(
        &runner_path,
        "import argparse\nparser = argparse.ArgumentParser(); parser.add_argument('--image-path'); parser.add_argument('--output'); parser.add_argument('--mock', action='store_true')\nargs = parser.parse_args()\nopen(args.output, 'w', encoding='utf-8').write('{\\\"artifact\\\": \\\"ocr_privacy_evidence\\\",')\nprint('Jane Example MRN-12345 /tmp/leak')\n",
    )
    .unwrap();
    let report_path = phi_named_dir.join("report.json");
    fs::write(&report_path, "stale Jane Example MRN-12345").unwrap();
    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-privacy-evidence",
            "--image-path",
            image_path.to_str().unwrap(),
            "--runner-path",
            runner_path.to_str().unwrap(),
            "--output",
            report_path.to_str().unwrap(),
            "--python-command",
            default_python_command(),
            "--mock",
        ])
        .assert()
        .failure()
        .get_output()
        .clone();
    assert!(!report_path.exists());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stdout.is_empty());
    assert!(stderr.contains("OCR privacy evidence failed"));
    for unsafe_text in [
        report_path.to_str().unwrap(),
        phi_named_dir.to_str().unwrap(),
        "Jane Example",
        "MRN-12345",
        "fixture.png",
        "invalid_json_runner.py",
        "/tmp/leak",
        "expected value",
        "line 1 column",
    ] {
        assert!(!stdout.contains(unsafe_text), "stdout leaked {unsafe_text}");
        assert!(!stderr.contains(unsafe_text), "stderr leaked {unsafe_text}");
    }
}

#[test]
fn ocr_privacy_evidence_rejects_invalid_runner_report_and_removes_stale_output() {
    let dir = tempdir().unwrap();
    let phi_named_dir = dir.path().join("Jane-Example-MRN-12345");
    fs::create_dir(&phi_named_dir).unwrap();
    let image_path = phi_named_dir.join("fixture.png");
    fs::write(&image_path, b"image").unwrap();
    let runner_path = phi_named_dir.join("bad_runner.py");
    fs::write(&runner_path, "import argparse, json\nparser = argparse.ArgumentParser(); parser.add_argument('--image-path'); parser.add_argument('--output'); parser.add_argument('--mock', action='store_true')\nargs = parser.parse_args()\nopen(args.output, 'w', encoding='utf-8').write(json.dumps({'artifact':'ocr_privacy_evidence','ocr_scope':'printed_text_line_extraction_only','privacy_scope':'text_only_pii_detection','network_api_called': True, 'raw_text':'Jane Example'}))\nprint('Jane Example MRN-12345 /tmp/leak')\n").unwrap();
    let report_path = phi_named_dir.join("report.json");
    fs::write(&report_path, "stale Jane Example").unwrap();
    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "ocr-privacy-evidence",
            "--image-path",
            image_path.to_str().unwrap(),
            "--runner-path",
            runner_path.to_str().unwrap(),
            "--output",
            report_path.to_str().unwrap(),
            "--python-command",
            default_python_command(),
            "--mock",
        ])
        .assert()
        .failure()
        .get_output()
        .clone();
    assert!(!report_path.exists());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    for unsafe_text in [
        report_path.to_str().unwrap(),
        phi_named_dir.to_str().unwrap(),
        "Jane Example",
        "MRN-12345",
        "/tmp/leak",
    ] {
        assert!(!stdout.contains(unsafe_text), "stdout leaked {unsafe_text}");
        assert!(!stderr.contains(unsafe_text), "stderr leaked {unsafe_text}");
    }
}
