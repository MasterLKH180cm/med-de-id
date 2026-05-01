import json
import subprocess
import sys
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
RUNNER = REPO_ROOT / "scripts" / "ocr_eval" / "run_ocr_privacy_evidence.py"
IMAGE = REPO_ROOT / "scripts" / "ocr_eval" / "fixtures" / "synthetic_printed_phi_line.png"
OCR_RUNNER = REPO_ROOT / "scripts" / "ocr_eval" / "run_small_ocr.py"
PRIVACY_RUNNER = REPO_ROOT / "scripts" / "privacy_filter" / "run_privacy_filter.py"
PHI_SENTINELS = [
    "Jane Example",
    "MRN-12345",
    "jane@example.com",
    "555-123-4567",
    "+1-555-123-4567",
]


EXPECTED = {
    "artifact": "ocr_privacy_evidence",
    "ocr_candidate": "PP-OCRv5_mobile_rec",
    "ocr_engine": "PP-OCRv5-mobile-bounded-spike",
    "ocr_scope": "printed_text_line_extraction_only",
    "ocr_engine_status": "deterministic_synthetic_fixture_fallback",
    "privacy_filter_engine": "fallback_synthetic_patterns",
    "privacy_filter_contract": "text_only_normalized_input",
    "privacy_scope": "text_only_pii_detection",
    "ready_for_text_pii_eval": True,
    "network_api_called": False,
    "detected_span_count": 4,
    "category_counts": {"EMAIL": 1, "MRN": 1, "NAME": 1, "PHONE": 1},
    "non_goals": [
        "browser_ui",
        "complete_ocr_pipeline",
        "desktop_ui",
        "final_pdf_rewrite_export",
        "handwriting_recognition",
        "image_pixel_redaction",
        "visual_redaction",
    ],
}


def run_evidence(output: Path, image_path: Path = IMAGE, privacy_runner: Path = PRIVACY_RUNNER):
    return subprocess.run(
        [
            sys.executable,
            str(RUNNER),
            "--image-path",
            str(image_path),
            "--ocr-runner-path",
            str(OCR_RUNNER),
            "--privacy-runner-path",
            str(privacy_runner),
            "--output",
            str(output),
            "--mock",
        ],
        cwd=REPO_ROOT,
        text=True,
        capture_output=True,
        timeout=15,
    )


def assert_no_phi(*values: str):
    combined = "\n".join(values)
    for sentinel in PHI_SENTINELS:
        assert sentinel not in combined


def test_ocr_privacy_evidence_success_path_writes_aggregate_only_report(tmp_path):
    output = tmp_path / "ocr-privacy-evidence.json"

    proc = run_evidence(output)

    assert proc.returncode == 0, proc.stderr
    assert proc.stderr == ""
    assert json.loads(output.read_text(encoding="utf-8")) == EXPECTED
    report = json.loads(output.read_text(encoding="utf-8"))
    assert "ID" not in report["category_counts"]
    assert report["detected_span_count"] == sum(report["category_counts"].values())
    assert json.loads(proc.stdout) == {
        "artifact": "ocr_privacy_evidence",
        "report_path": "<redacted>",
        "report_written": True,
    }
    assert '"report_path": "<redacted>"' in proc.stdout
    assert_no_phi(proc.stdout, proc.stderr, output.read_text(encoding="utf-8"))


def test_privacy_filter_canonical_validator_failure_is_generic_and_phi_safe(tmp_path):
    bad_privacy_runner = tmp_path / "bad_privacy_runner.py"
    bad_privacy_runner.write_text(
        """
import json
print(json.dumps({
    "metadata": {"engine": "fallback_synthetic_patterns", "network_api_called": False},
    "summary": {"detected_span_count": 4, "category_counts": {"EMAIL": 1, "MRN": 1, "NAME": 1, "PHONE": 1}},
    "masked_text": "<masked-text>",
    "spans": [],
}))
""".lstrip(),
        encoding="utf-8",
    )
    output = tmp_path / "Jane Example MRN-12345 evidence.json"

    proc = run_evidence(output, privacy_runner=bad_privacy_runner)

    assert proc.returncode == 3
    assert proc.stdout == ""
    assert proc.stderr == "OCR Privacy evidence runner failed\n"
    assert not output.exists()
    assert_no_phi(proc.stdout, proc.stderr)


def test_missing_image_removes_stale_output_without_echoing_phi_path(tmp_path):
    phi_dir = tmp_path / "Jane Example MRN-12345"
    phi_dir.mkdir()
    output = phi_dir / "ocr-privacy-evidence.json"
    output.write_text("Jane Example", encoding="utf-8")

    proc = run_evidence(output, image_path=tmp_path / "missing-image.png")

    assert proc.returncode != 0
    assert proc.stdout == ""
    assert proc.stderr == "OCR Privacy evidence input image is missing\n"
    assert not output.exists()
    assert_no_phi(proc.stdout, proc.stderr)


def test_output_directory_cleanup_failure_is_generic_and_phi_safe(tmp_path):
    output = tmp_path / "Jane-Example-MRN-12345-output.json"
    output.mkdir()

    proc = run_evidence(output)

    assert proc.returncode != 0
    assert proc.stdout == ""
    assert proc.stderr == "OCR Privacy evidence output cleanup failed\n"
    assert "Traceback" not in proc.stderr
    assert "Jane-Example-MRN-12345" not in proc.stderr
    assert "Jane-Example-MRN-12345" not in proc.stdout
    assert_no_phi(proc.stdout, proc.stderr)


def test_output_write_failure_is_generic_and_phi_safe(tmp_path):
    unwritable_dir = tmp_path / "Jane-Example-MRN-12345-output"
    unwritable_dir.mkdir()
    unwritable_dir.chmod(0o500)
    output = unwritable_dir / "ocr-privacy-evidence.json"

    try:
        proc = run_evidence(output)
    finally:
        unwritable_dir.chmod(0o700)

    assert proc.returncode != 0
    assert proc.stdout == ""
    assert proc.stderr == "OCR Privacy evidence output write failed\n"
    assert "Traceback" not in proc.stderr
    assert "Jane" not in proc.stderr
    assert "MRN-12345" not in proc.stderr
    assert str(tmp_path) not in proc.stderr
    assert "Jane" not in proc.stdout
    assert "MRN-12345" not in proc.stdout
    assert str(tmp_path) not in proc.stdout
    assert_no_phi(proc.stdout, proc.stderr)
