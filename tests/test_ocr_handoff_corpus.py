import json
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
RUNNER = REPO / "scripts/ocr_eval/run_ocr_handoff_corpus.py"
FIXTURE_DIR = REPO / "scripts/ocr_eval/fixtures/corpus"

EXPECTED_KEYS = {
    "engine",
    "scope",
    "fixture_count",
    "ready_fixture_count",
    "total_char_count",
    "fixtures",
    "non_goals",
    "privacy_filter_contract",
}
FORBIDDEN_SYNTHETIC_PHI = [
    "Jane Example",
    "MRN-12345",
    "jane@example.com",
    "John Sample",
    "MRN-67890",
]
FORBIDDEN_FAILURE_TOKENS = [
    "empty.txt",
    "oversized.txt",
    "invalid_utf8.txt",
    "fixtures",
    "/",
    "Traceback",
]
REQUIRED_NON_GOALS = {
    "visual_redaction",
    "final_pdf_rewrite_export",
    "handwriting_recognition",
    "full_page_detection_or_segmentation",
    "complete_ocr_pipeline",
}


def run_corpus(*args):
    return subprocess.run(
        [sys.executable, str(RUNNER), *map(str, args)],
        cwd=REPO,
        text=True,
        capture_output=True,
        timeout=10,
    )


def test_corpus_report_contains_only_aggregate_phi_safe_fields(tmp_path):
    output = tmp_path / "report.json"

    result = run_corpus("--fixture-dir", FIXTURE_DIR, "--output", output)

    assert result.returncode == 0, result.stderr
    report_text = output.read_text(encoding="utf-8")
    for forbidden in FORBIDDEN_SYNTHETIC_PHI:
        assert forbidden not in report_text
    report = json.loads(report_text)
    assert set(report) == EXPECTED_KEYS
    assert report["engine"] == "PP-OCRv5-mobile-bounded-spike"
    assert report["scope"] == "printed_text_line_extraction_only"
    assert report["fixture_count"] == 2
    assert report["ready_fixture_count"] == 2
    assert report["total_char_count"] == 219
    assert report["privacy_filter_contract"] == "text_only_normalized_input"
    assert REQUIRED_NON_GOALS <= set(report["non_goals"])
    assert report["fixtures"] == [
        {"id": "fixture_001", "char_count": report["fixtures"][0]["char_count"], "ready_for_text_pii_eval": True},
        {"id": "fixture_002", "char_count": report["fixtures"][1]["char_count"], "ready_for_text_pii_eval": True},
    ]
    assert all(set(fixture) == {"id", "char_count", "ready_for_text_pii_eval"} for fixture in report["fixtures"])
    assert [fixture["char_count"] for fixture in report["fixtures"]] == [111, 108]


def test_missing_fixture_dir_fails_without_leaving_report(tmp_path):
    output = tmp_path / "report.json"
    output.write_text("stale", encoding="utf-8")

    result = run_corpus("--fixture-dir", tmp_path / "missing", "--output", output)

    assert result.returncode != 0
    assert not output.exists()


def test_empty_fixture_dir_fails_without_leaving_report(tmp_path):
    fixture_dir = tmp_path / "empty"
    fixture_dir.mkdir()
    output = tmp_path / "report.json"
    output.write_text("stale", encoding="utf-8")

    result = run_corpus("--fixture-dir", fixture_dir, "--output", output)

    assert result.returncode != 0
    assert not output.exists()


def test_empty_fixture_file_fails_without_leaving_report(tmp_path):
    fixture_dir = tmp_path / "fixtures"
    fixture_dir.mkdir()
    (fixture_dir / "empty.txt").write_text("  \n\t  ", encoding="utf-8")
    output = tmp_path / "report.json"
    output.write_text("stale", encoding="utf-8")

    result = run_corpus("--fixture-dir", fixture_dir, "--output", output)

    assert result.returncode != 0
    assert not output.exists()
    combined_output = result.stdout + result.stderr
    assert "fixture_001" in combined_output
    assert "empty after whitespace normalization" in combined_output
    for forbidden in FORBIDDEN_FAILURE_TOKENS:
        assert forbidden not in combined_output


def test_invalid_utf8_fixture_fails_without_filename_or_leaving_report(tmp_path):
    fixture_dir = tmp_path / "fixtures"
    fixture_dir.mkdir()
    (fixture_dir / "invalid_utf8.txt").write_bytes(b"valid prefix \xff")
    output = tmp_path / "report.json"
    output.write_text("stale", encoding="utf-8")

    result = run_corpus("--fixture-dir", fixture_dir, "--output", output)

    assert result.returncode != 0
    assert not output.exists()
    combined_output = result.stdout + result.stderr
    assert "fixture_001" in combined_output
    assert "valid UTF-8" in combined_output
    for forbidden in FORBIDDEN_FAILURE_TOKENS:
        assert forbidden not in combined_output


def test_oversized_fixture_fails_without_filename_or_report(tmp_path):
    fixture_dir = tmp_path / "fixtures"
    fixture_dir.mkdir()
    (fixture_dir / "oversized.txt").write_bytes(b"x" * (1024 * 1024 + 1))
    output = tmp_path / "report.json"

    result = run_corpus("--fixture-dir", fixture_dir, "--output", output)

    assert result.returncode != 0
    assert not output.exists()
    combined_output = result.stdout + result.stderr
    assert "fixture_001" in combined_output
    assert "exceeds maximum fixture size" in combined_output
    for forbidden in FORBIDDEN_FAILURE_TOKENS:
        assert forbidden not in combined_output
