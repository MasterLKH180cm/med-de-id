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
    kwargs.setdefault("timeout", 10)
    return subprocess.run(
        [sys.executable, *map(str, args)],
        cwd=REPO,
        text=True,
        capture_output=True,
        **kwargs,
    )


REQUIRED_NON_GOALS = {
    "visual_redaction",
    "final_pdf_rewrite_export",
    "handwriting_recognition",
    "full_page_detection_or_segmentation",
    "complete_ocr_pipeline",
}


def valid_handoff(**overrides):
    obj = {
        "source": "synthetic_printed_phi_line.png",
        "extracted_text": "Patient Jane Doe\nDOB 1970-01-02",
        "normalized_text": "Patient Jane Doe DOB 1970-01-02",
        "ready_for_text_pii_eval": True,
        "candidate": "PP-OCRv5_mobile_rec",
        "engine": "PP-OCRv5-mobile-bounded-spike",
        "engine_status": "deterministic_synthetic_fixture_fallback",
        "scope": "printed_text_line_extraction_only",
        "privacy_filter_contract": "text_only_normalized_input",
        "non_goals": sorted(REQUIRED_NON_GOALS),
    }
    obj.update(overrides)
    return obj


def write_handoff(tmp_path, obj):
    handoff = tmp_path / "handoff.json"
    handoff.write_text(json.dumps(obj), encoding="utf-8")
    return handoff


def test_run_helper_sets_short_subprocess_timeout(monkeypatch):
    captured = {}

    def fake_run(*args, **kwargs):
        captured.update(kwargs)
        return subprocess.CompletedProcess(args, 0, "", "")

    monkeypatch.setattr(subprocess, "run", fake_run)

    run(VALIDATOR, "handoff.json")

    assert captured["timeout"] == 10


def test_validator_requires_every_documented_non_goal(tmp_path):
    for missing_non_goal in REQUIRED_NON_GOALS:
        non_goals = sorted(REQUIRED_NON_GOALS - {missing_non_goal})
        handoff = write_handoff(tmp_path, valid_handoff(non_goals=non_goals))

        validated = run(VALIDATOR, handoff)

        assert validated.returncode != 0
        assert missing_non_goal in validated.stderr


def test_validator_fails_cleanly_without_path():
    validated = run(VALIDATOR)

    assert validated.returncode != 0
    assert "usage" in validated.stderr.lower() or "path" in validated.stderr.lower()


def test_validator_requires_normalized_text_to_match_whitespace_normalized_extracted_text(tmp_path):
    handoff = write_handoff(
        tmp_path,
        valid_handoff(extracted_text="Alpha\n\tBeta", normalized_text="Alpha Beta extra"),
    )

    validated = run(VALIDATOR, handoff)

    assert validated.returncode != 0
    assert "normalized_text" in validated.stderr


def test_validator_requires_ready_flag_to_match_normalized_text_truthiness(tmp_path):
    handoff = write_handoff(
        tmp_path,
        valid_handoff(extracted_text="", normalized_text="", ready_for_text_pii_eval=True),
    )

    validated = run(VALIDATOR, handoff)

    assert validated.returncode != 0
    assert "ready_for_text_pii_eval" in validated.stderr


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
    assert REQUIRED_NON_GOALS <= set(obj["non_goals"])
    assert obj["normalized_text"] == " ".join(obj["extracted_text"].split())

    privacy_input = tmp_path / "privacy-input.txt"
    privacy_input.write_text(obj["normalized_text"], encoding="utf-8")
    pii = run(PRIVACY_FILTER, "--mock", privacy_input)
    assert pii.returncode == 0, pii.stderr
    pii_obj = json.loads(pii.stdout)
    assert pii_obj["summary"]["detected_span_count"] >= 1
    assert pii_obj["metadata"]["network_api_called"] is False


def test_small_ocr_json_mode_emits_bounded_extraction_contract_and_feeds_privacy_filter(tmp_path):
    ocr = run(RUNNER, "--mock", "--json", FIXTURE_IMAGE)
    assert ocr.returncode == 0, ocr.stderr

    obj = json.loads(ocr.stdout)
    expected_text = EXPECTED_TEXT.read_text(encoding="utf-8")
    assert obj == {
        "candidate": "PP-OCRv5_mobile_rec",
        "engine": "PP-OCRv5-mobile-bounded-spike",
        "engine_status": "deterministic_synthetic_fixture_fallback",
        "scope": "printed_text_line_extraction_only",
        "source": "<redacted>",
        "extracted_text": expected_text,
        "normalized_text": " ".join(expected_text.split()),
        "ready_for_text_pii_eval": True,
        "privacy_filter_contract": "text_only_normalized_input",
        "non_goals": sorted(REQUIRED_NON_GOALS),
    }
    assert "synthetic_printed_phi_line.png" not in ocr.stdout

    privacy_input = tmp_path / "privacy-input.txt"
    privacy_input.write_text(obj["normalized_text"], encoding="utf-8")
    pii = run(PRIVACY_FILTER, "--mock", privacy_input)
    assert pii.returncode == 0, pii.stderr
    pii_obj = json.loads(pii.stdout)
    assert pii_obj["summary"]["detected_span_count"] >= 1
    assert pii_obj["metadata"]["network_api_called"] is False
