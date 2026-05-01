import importlib.util
import json
import subprocess
import sys
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[1]
REPO_ROOT = REPO
FIXTURE_IMAGE = REPO / "scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png"
EXPECTED_TEXT = REPO / "scripts/ocr_eval/fixtures/synthetic_printed_phi_expected.txt"
RUNNER = REPO / "scripts/ocr_eval/run_small_ocr.py"


def load_runner():
    spec = importlib.util.spec_from_file_location("run_small_ocr_under_test", RUNNER)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


def test_mock_outputs_synthetic_fixture_text(capsys):
    runner = load_runner()

    code = runner.main(["--mock", str(FIXTURE_IMAGE)])

    captured = capsys.readouterr()
    assert code == 0
    assert captured.out == EXPECTED_TEXT.read_text(encoding="utf-8")
    assert captured.err == ""


def test_ocr_runner_rejects_missing_input_path_without_phi_leak(tmp_path):
    missing = tmp_path / "missing-line.png"

    proc = subprocess.run(
        [sys.executable, "scripts/ocr_eval/run_small_ocr.py", "--mock", str(missing)],
        cwd=REPO_ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )

    assert proc.returncode == 2
    assert proc.stdout == ""
    assert "OCR input path does not exist" in proc.stderr
    assert "Patient Jane Example" not in proc.stderr
    assert "MRN-12345" not in proc.stderr


def test_ocr_runner_rejects_directory_input_without_phi_leak(tmp_path):
    proc = subprocess.run(
        [sys.executable, "scripts/ocr_eval/run_small_ocr.py", "--mock", str(tmp_path)],
        cwd=REPO_ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )

    assert proc.returncode == 2
    assert proc.stdout == ""
    assert "OCR input path must be a file" in proc.stderr
    assert "Patient Jane Example" not in proc.stderr
    assert "MRN-12345" not in proc.stderr


def test_json_output_redacts_phi_bearing_source_filename(tmp_path):
    source = tmp_path / "Jane-Example-MRN-12345.png"
    source.write_bytes(b"synthetic image placeholder")
    expected = tmp_path / "synthetic_printed_phi_expected.txt"
    expected.write_text("Patient Jane Example MRN-12345\n", encoding="utf-8")

    completed = subprocess.run(
        [sys.executable, "scripts/ocr_eval/run_small_ocr.py", "--mock", "--json", str(source)],
        cwd=REPO_ROOT,
        text=True,
        capture_output=True,
        check=True,
    )

    payload = json.loads(completed.stdout)
    rendered = json.dumps(payload, sort_keys=True)
    assert payload["source"] == "<redacted>"
    assert "Jane-Example-MRN-12345" not in rendered
    assert "Jane-Example-MRN-12345" not in completed.stderr


def test_missing_paddleocr_without_mock_exits_3_without_fixture_text(monkeypatch, capsys):
    runner = load_runner()
    monkeypatch.setitem(sys.modules, "paddleocr", None)

    code = runner.main([str(FIXTURE_IMAGE)])

    captured = capsys.readouterr()
    assert code == 3
    assert "PaddleOCR is not installed locally" in captured.err
    assert EXPECTED_TEXT.read_text(encoding="utf-8").strip() not in captured.out


def test_fake_paddleocr_result_is_normalized_to_plain_stdout(monkeypatch, capsys):
    runner = load_runner()

    class FakePaddleOCR:
        def __init__(self, **kwargs):
            self.kwargs = kwargs

        def ocr(self, image_path, **kwargs):
            assert image_path == str(FIXTURE_IMAGE)
            return [
                [
                    [None, ("Patient Jane Doe", 0.99)],
                    [None, ("DOB 1970-01-02", 0.98)],
                ]
            ]

    class FakePaddleModule:
        PaddleOCR = FakePaddleOCR

    monkeypatch.setitem(sys.modules, "paddleocr", FakePaddleModule)

    code = runner.main([str(FIXTURE_IMAGE)])

    captured = capsys.readouterr()
    assert code == 0
    assert captured.out == "Patient Jane Doe\nDOB 1970-01-02\n"
    assert captured.err == ""


def test_fake_paddleocr_dict_rec_texts_result_is_normalized(monkeypatch, capsys):
    runner = load_runner()

    class FakePaddleOCR:
        def __init__(self, **kwargs):
            self.kwargs = kwargs

        def ocr(self, image_path, **kwargs):
            assert image_path == str(FIXTURE_IMAGE)
            return [{"rec_texts": ["Patient Jane Example", "MRN MRN-12345"]}]

    class FakePaddleModule:
        PaddleOCR = FakePaddleOCR

    monkeypatch.setitem(sys.modules, "paddleocr", FakePaddleModule)

    code = runner.main([str(FIXTURE_IMAGE)])

    captured = capsys.readouterr()
    assert code == 0
    assert captured.out == "Patient Jane Example\nMRN MRN-12345\n"
    assert captured.err == ""
