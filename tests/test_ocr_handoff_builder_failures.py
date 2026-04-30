import json
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
BUILDER = REPO / "scripts/ocr_eval/build_ocr_handoff.py"
FIXTURE_IMAGE = REPO / "scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png"
EXPECTED_TEXT = REPO / "scripts/ocr_eval/fixtures/synthetic_printed_phi_expected.txt"
RAW_FIXTURE_VALUES = (
    "Jane Example",
    "jane@example.com",
    "+1-555-123-4567",
    "MRN-12345",
)


def run_builder(*args):
    return subprocess.run(
        [sys.executable, str(BUILDER), *map(str, args)],
        cwd=REPO,
        text=True,
        capture_output=True,
        timeout=10,
    )


def assert_no_raw_fixture_phi(*streams):
    combined = "\n".join(streams)
    for value in RAW_FIXTURE_VALUES:
        assert value not in combined


def test_missing_ocr_input_fails_phi_safely_and_removes_stale_output(tmp_path):
    missing_input = tmp_path / "Jane Example jane@example.com +1-555-123-4567 MRN-12345.txt"
    output = tmp_path / "handoff.json"
    output.write_text("stale PHI Jane Example", encoding="utf-8")

    result = run_builder(
        "--source", FIXTURE_IMAGE,
        "--input", missing_input,
        "--output", output,
    )

    assert result.returncode == 2
    assert "OCR input file is missing" in result.stderr
    assert str(missing_input) not in result.stderr
    assert_no_raw_fixture_phi(result.stdout, result.stderr)
    assert not output.exists()


def test_empty_ocr_input_fails_phi_safely_without_report(tmp_path):
    empty_input = tmp_path / "ocr.txt"
    empty_input.write_text(" \n\t\n", encoding="utf-8")
    output = tmp_path / "handoff.json"

    result = run_builder(
        "--source", FIXTURE_IMAGE,
        "--input", empty_input,
        "--output", output,
    )

    assert result.returncode == 2
    assert "OCR input text is empty" in result.stderr
    assert_no_raw_fixture_phi(result.stdout, result.stderr)
    assert not output.exists()


def test_valid_fixture_still_writes_existing_handoff_contract(tmp_path):
    ocr_input = tmp_path / "ocr.txt"
    ocr_input.write_text(EXPECTED_TEXT.read_text(encoding="utf-8"), encoding="utf-8")
    output = tmp_path / "handoff.json"

    result = run_builder(
        "--source", FIXTURE_IMAGE,
        "--input", ocr_input,
        "--output", output,
    )

    assert result.returncode == 0, result.stderr
    obj = json.loads(output.read_text(encoding="utf-8"))
    assert obj["candidate"] == "PP-OCRv5_mobile_rec"
    assert obj["engine"] == "PP-OCRv5-mobile-bounded-spike"
    assert obj["scope"] == "printed_text_line_extraction_only"
    assert obj["privacy_filter_contract"] == "text_only_normalized_input"
    assert obj["ready_for_text_pii_eval"] is True
