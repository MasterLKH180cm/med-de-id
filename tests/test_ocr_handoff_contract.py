import json
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
FIXTURE_IMAGE = REPO / "scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png"
EXPECTED_TEXT = REPO / "scripts/ocr_eval/fixtures/synthetic_printed_phi_expected.txt"
RUNNER = REPO / "scripts/ocr_eval/run_small_ocr.py"
BUILDER = REPO / "scripts/ocr_eval/build_ocr_handoff.py"
VALIDATOR = REPO / "scripts/ocr_eval/validate_ocr_handoff.py"
PRIVACY_FILTER = REPO / "scripts/privacy_filter/run_privacy_filter.py"


def run(*args, **kwargs):
    return subprocess.run(
        [sys.executable, *map(str, args)],
        cwd=REPO,
        text=True,
        capture_output=True,
        **kwargs,
    )


def test_ppocrv5_mobile_bounded_spike_handoff_metadata_and_privacy_filter_text_contract(tmp_path):
    ocr_text = tmp_path / "ocr.txt"
    handoff = tmp_path / "handoff.json"

    ocr = run(RUNNER, "--mock", FIXTURE_IMAGE)
    assert ocr.returncode == 0, ocr.stderr
    ocr_text.write_text(ocr.stdout, encoding="utf-8")
    assert ocr.stdout.strip() == EXPECTED_TEXT.read_text(encoding="utf-8").strip()

    built = run(BUILDER, "--source", FIXTURE_IMAGE, "--input", ocr_text, "--output", handoff)
    assert built.returncode == 0, built.stderr

    validated = run(VALIDATOR, handoff)
    assert validated.returncode == 0, validated.stderr

    obj = json.loads(handoff.read_text(encoding="utf-8"))
    assert obj["source"] == "synthetic_printed_phi_line.png"
    assert obj["candidate"] == "PP-OCRv5_mobile_rec"
    assert obj["engine"] == "PP-OCRv5-mobile-bounded-spike"
    assert obj["engine_status"] == "deterministic_synthetic_fixture_fallback"
    assert obj["scope"] == "printed_text_line_extraction_only"
    assert obj["ready_for_text_pii_eval"] is True
    assert obj["privacy_filter_contract"] == "text_only_normalized_input"
    assert "visual_redaction" in obj["non_goals"]
    assert "final_pdf_rewrite_export" in obj["non_goals"]
    assert "handwriting_recognition" in obj["non_goals"]
    assert obj["normalized_text"] == " ".join(obj["extracted_text"].split())

    privacy_input = tmp_path / "privacy-input.txt"
    privacy_input.write_text(obj["normalized_text"], encoding="utf-8")
    pii = run(PRIVACY_FILTER, "--mock", privacy_input)
    assert pii.returncode == 0, pii.stderr
    pii_obj = json.loads(pii.stdout)
    assert pii_obj["summary"]["detected_span_count"] >= 1
    assert pii_obj["metadata"]["network_api_called"] is False
