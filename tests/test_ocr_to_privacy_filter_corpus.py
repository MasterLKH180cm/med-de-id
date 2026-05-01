import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
RUNNER = REPO_ROOT / "scripts" / "ocr_eval" / "run_ocr_to_privacy_filter_corpus.py"
FIXTURE_DIR = REPO_ROOT / "scripts" / "ocr_eval" / "fixtures" / "corpus"
OCR_RUNNER = REPO_ROOT / "scripts" / "ocr_eval" / "run_ocr_handoff_corpus.py"
PRIVACY_RUNNER = REPO_ROOT / "scripts" / "privacy_filter" / "run_privacy_filter.py"
RAW_SENTINELS = ("Jane Example", "MRN-12345", "jane@example.com", "555-123-4567")


def test_ocr_to_privacy_filter_corpus_writes_phi_safe_aggregate():
    with tempfile.TemporaryDirectory() as tmp:
        output_path = Path(tmp) / "ocr-to-privacy-filter-report.json"
        result = subprocess.run(
            [
                sys.executable,
                str(RUNNER),
                "--fixture-dir",
                str(FIXTURE_DIR),
                "--ocr-runner-path",
                str(OCR_RUNNER),
                "--privacy-runner-path",
                str(PRIVACY_RUNNER),
                "--output",
                str(output_path),
            ],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=20,
            check=False,
        )

        assert result.returncode == 0, result.stderr
        assert "ocr_to_privacy_filter_corpus" in result.stdout
        assert result.stderr == ""
        assert output_path.exists()
        report_text = output_path.read_text(encoding="utf-8")
        combined = result.stdout + result.stderr + report_text
        for sentinel in RAW_SENTINELS:
            assert sentinel not in combined

        report = json.loads(report_text)

    assert report["artifact"] == "ocr_to_privacy_filter_corpus_bridge"
    assert report["ocr_candidate"] == "PP-OCRv5_mobile_rec"
    assert report["ocr_engine"] == "PP-OCRv5-mobile-bounded-spike"
    assert report["scope"] == "printed_text_extraction_to_text_pii_detection_only"
    assert report["privacy_filter_engine"] == "fallback_synthetic_patterns"
    assert report["privacy_filter_contract"] == "text_only_normalized_input"
    assert report["fixture_count"] >= 2
    assert report["ready_fixture_count"] == report["fixture_count"]
    assert report["privacy_filter_detected_span_count"] >= 2
    assert report["privacy_filter_category_counts"]["NAME"] >= 1
    assert report["privacy_filter_category_counts"]["MRN"] >= 1
    assert report["category_counts"] == report["privacy_filter_category_counts"]
    assert "visual_redaction" in report["non_goals"]
    assert "image_pixel_redaction" in report["non_goals"]
    assert "final_pdf_rewrite_export" in report["non_goals"]

    assert len(report["fixtures"]) == report["fixture_count"]
    for fixture in report["fixtures"]:
        assert set(fixture) == {"detected_span_count", "fixture", "ready_for_text_pii_eval"}
        assert re.fullmatch(r"fixture_\d{3}", fixture["fixture"])
        assert isinstance(fixture["detected_span_count"], int)
        assert fixture["detected_span_count"] >= 0
        assert fixture["ready_for_text_pii_eval"] is True


def test_missing_privacy_runner_removes_stale_phi_and_emits_sanitized_failure():
    with tempfile.TemporaryDirectory() as tmp:
        output_path = Path(tmp) / "ocr-to-privacy-filter-report.json"
        output_path.write_text('{"stale":"Jane Example"}\n', encoding="utf-8")
        missing_runner = Path(tmp) / "missing-privacy-runner-Jane-Example.py"

        result = subprocess.run(
            [
                sys.executable,
                str(RUNNER),
                "--fixture-dir",
                str(FIXTURE_DIR),
                "--ocr-runner-path",
                str(OCR_RUNNER),
                "--privacy-runner-path",
                str(missing_runner),
                "--output",
                str(output_path),
            ],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=20,
            check=False,
        )

        assert result.returncode != 0
        assert not output_path.exists()
        combined = result.stdout + result.stderr
        assert "Jane Example" not in combined
        assert str(missing_runner) not in combined


def test_privacy_runner_sensitive_fields_do_not_pass_through_to_aggregate():
    raw_values = ("RAW_MASKED_Jane Example", "RAW_SPAN_MRN-12345", "RAW_PREVIEW_jane@example.com")
    with tempfile.TemporaryDirectory() as tmp:
        stub_path = Path(tmp) / "privacy_stub.py"
        output_path = Path(tmp) / "ocr-to-privacy-filter-report.json"
        stub_path.write_text(
            """
import json
payload = {
  "summary": {"input_char_count": 10, "detected_span_count": 2, "category_counts": {"NAME": 1, "MRN": 1}},
  "masked_text": "RAW_MASKED_Jane Example",
  "spans": [{"label": "NAME", "start": 0, "end": 4, "preview": "RAW_PREVIEW_jane@example.com", "text": "RAW_SPAN_MRN-12345"}],
  "metadata": {"engine": "stub_privacy_filter", "network_api_called": False}
}
print(json.dumps(payload))
""".lstrip(),
            encoding="utf-8",
        )

        result = subprocess.run(
            [
                sys.executable,
                str(RUNNER),
                "--fixture-dir",
                str(FIXTURE_DIR),
                "--ocr-runner-path",
                str(OCR_RUNNER),
                "--privacy-runner-path",
                str(stub_path),
                "--output",
                str(output_path),
            ],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=20,
            check=False,
        )

        assert result.returncode == 0, result.stderr
        report_text = output_path.read_text(encoding="utf-8")
        combined = result.stdout + result.stderr + report_text
        for raw_value in raw_values:
            assert raw_value not in combined
