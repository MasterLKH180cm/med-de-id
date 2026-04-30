import importlib.util
import sys
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[1]
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
