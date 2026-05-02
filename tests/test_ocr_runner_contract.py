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


def test_json_output_includes_phi_safe_text_metrics(tmp_path):
    source = tmp_path / "Jane-Example-MRN-12345.png"
    source.write_bytes(b"synthetic image placeholder")
    expected = tmp_path / "synthetic_printed_phi_expected.txt"
    expected.write_text("Patient Jane Example\nMRN MRN-12345\n", encoding="utf-8")

    completed = subprocess.run(
        [sys.executable, "scripts/ocr_eval/run_small_ocr.py", "--mock", "--json", str(source)],
        cwd=REPO_ROOT,
        text=True,
        capture_output=True,
        check=True,
    )

    payload = json.loads(completed.stdout)
    assert payload["line_count"] == 2
    assert payload["normalized_char_count"] == len("Patient Jane Example MRN MRN-12345")
    assert payload["ready_for_text_pii_eval"] is True
    rendered = json.dumps(payload, sort_keys=True)
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


def test_fake_paddleocr_valueerror_kwargs_retry_is_normalized_to_plain_stdout(monkeypatch, capsys):
    runner = load_runner()

    class FakePaddleOCR:
        def __init__(self, **kwargs):
            if kwargs:
                raise ValueError('Unknown argument: rec_model_name')
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


def test_subprocess_local_paddleocr_adapter_path_emits_json_without_mock_or_source_leak(tmp_path):
    adapter_dir = tmp_path / "adapter"
    adapter_dir.mkdir()
    (adapter_dir / "paddleocr.py").write_text(
        "class PaddleOCR:\n"
        "    def __init__(self, **kwargs):\n"
        "        self.kwargs = kwargs\n"
        "    def ocr(self, image_path):\n"
        "        return [{'rec_texts': ['Patient Jane Example', 'MRN MRN-12345']}]\n",
        encoding="utf-8",
    )
    image = tmp_path / "Jane-Example-MRN-12345.png"
    image.write_bytes(b"synthetic image placeholder")
    env = {**__import__("os").environ, "PYTHONPATH": str(adapter_dir)}

    completed = subprocess.run(
        [sys.executable, "scripts/ocr_eval/run_small_ocr.py", "--json", str(image)],
        cwd=REPO_ROOT,
        text=True,
        capture_output=True,
        check=True,
        timeout=5,
        env=env,
    )

    payload = json.loads(completed.stdout)
    assert payload["engine_status"] == "local_paddleocr_execution"
    assert payload["source"] == "<redacted>"
    assert payload["ready_for_text_pii_eval"] is True
    rendered = json.dumps(payload, sort_keys=True)
    assert "Jane-Example-MRN-12345" not in rendered
    assert "Jane-Example-MRN-12345" not in completed.stderr
    assert "mock" not in completed.stderr.lower()


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


def test_batch_quality_runner_processes_multiple_mock_images_without_path_leaks(tmp_path):
    image_one = tmp_path / "Jane-Example-MRN-12345-page-1.png"
    image_two = tmp_path / "Jane-Example-MRN-12345-page-2.png"
    for image in (image_one, image_two):
        image.write_bytes(b"synthetic image placeholder")
    (tmp_path / "synthetic_printed_phi_expected.txt").write_text(
        "Patient Jane Example\nMRN MRN-12345\n", encoding="utf-8"
    )

    completed = subprocess.run(
        [
            sys.executable,
            "scripts/ocr_eval/run_ocr_batch_quality.py",
            "--mock",
            "--runner-path",
            "scripts/ocr_eval/run_small_ocr.py",
            str(image_one),
            str(image_two),
        ],
        cwd=REPO_ROOT,
        text=True,
        capture_output=True,
        check=True,
    )

    payload = json.loads(completed.stdout)
    assert payload["artifact"] == "ocr_batch_quality_summary"
    assert payload["input_count"] == 2
    assert payload["succeeded_count"] == 2
    assert payload["failed_count"] == 0
    assert payload["real_model_quality_verified"] is False
    assert payload["engine_status_counts"] == {"deterministic_synthetic_fixture_fallback": 2}
    rendered = json.dumps(payload, sort_keys=True)
    assert "Jane-Example-MRN-12345" not in rendered
    assert "Patient Jane Example" not in rendered
    assert completed.stderr == ""


def test_batch_quality_runner_recovers_from_one_missing_input_without_phi_leaks(tmp_path):
    image = tmp_path / "Jane-Example-MRN-12345-page-1.png"
    missing = tmp_path / "Jane-Example-MRN-12345-missing.png"
    image.write_bytes(b"synthetic image placeholder")
    (tmp_path / "synthetic_printed_phi_expected.txt").write_text(
        "Patient Jane Example\nMRN MRN-12345\n", encoding="utf-8"
    )

    completed = subprocess.run(
        [
            sys.executable,
            "scripts/ocr_eval/run_ocr_batch_quality.py",
            "--mock",
            "--runner-path",
            "scripts/ocr_eval/run_small_ocr.py",
            str(image),
            str(missing),
        ],
        cwd=REPO_ROOT,
        text=True,
        capture_output=True,
        check=True,
    )

    payload = json.loads(completed.stdout)
    assert payload["input_count"] == 2
    assert payload["succeeded_count"] == 1
    assert payload["failed_count"] == 1
    assert payload["items"][0]["status"] == "succeeded"
    assert payload["items"][1]["status"] == "failed"
    assert payload["items"][1]["error_code"] == 2
    assert payload["items"][1]["error"] == "OCR input path does not exist"
    rendered = json.dumps(payload, sort_keys=True)
    assert "Jane-Example-MRN-12345" not in rendered
    assert "Patient Jane Example" not in rendered
    assert completed.stderr == ""


def test_batch_quality_manifest_groups_multi_page_documents_without_path_or_text_leaks(tmp_path):
    page1 = tmp_path / "Jane-Example-MRN-12345-doc-a-page-1.png"
    page2 = tmp_path / "Jane-Example-MRN-12345-doc-a-page-2.png"
    page3 = tmp_path / "Jane-Example-MRN-12345-doc-b-page-1.png"
    for page in (page1, page2, page3):
        page.write_bytes(b"synthetic image placeholder")
    (tmp_path / "synthetic_printed_phi_expected.txt").write_text(
        "Patient Jane Example\nMRN MRN-12345\n", encoding="utf-8"
    )
    manifest = tmp_path / "manifest.json"
    manifest.write_text(
        json.dumps(
            {
                "documents": [
                    {
                        "document_id": "doc-a",
                        "pages": [
                            {"page_number": 1, "image_path": str(page1)},
                            {"page_number": 2, "image_path": str(page2)},
                        ],
                    },
                    {
                        "document_id": "doc-b",
                        "pages": [{"page_number": 1, "image_path": str(page3)}],
                    },
                ]
            }
        ),
        encoding="utf-8",
    )

    completed = subprocess.run(
        [
            sys.executable,
            "scripts/ocr_eval/run_ocr_batch_quality.py",
            "--mock",
            "--runner-path",
            "scripts/ocr_eval/run_small_ocr.py",
            "--manifest",
            str(manifest),
        ],
        cwd=REPO_ROOT,
        text=True,
        capture_output=True,
        check=True,
    )

    payload = json.loads(completed.stdout)
    assert payload["artifact"] == "ocr_batch_quality_summary"
    assert payload["input_count"] == 3
    assert payload["document_count"] == 2
    assert payload["succeeded_count"] == 3
    assert payload["failed_count"] == 0
    assert payload["documents"] == [
        {"document_id": "document-1", "page_count": 2, "succeeded_count": 2, "failed_count": 0},
        {"document_id": "document-2", "page_count": 1, "succeeded_count": 1, "failed_count": 0},
    ]
    rendered = json.dumps(payload, sort_keys=True)
    assert "Jane-Example-MRN-12345" not in rendered
    assert "Patient Jane Example" not in rendered
    assert completed.stderr == ""


def test_batch_quality_manifest_generates_phi_safe_document_ids(tmp_path):
    page = tmp_path / "Jane-Example-MRN-12345-page-1.png"
    page.write_bytes(b"synthetic image placeholder")
    (tmp_path / "synthetic_printed_phi_expected.txt").write_text(
        "Patient Jane Example\nMRN MRN-12345\n", encoding="utf-8"
    )
    manifest = tmp_path / "manifest.json"
    manifest.write_text(
        json.dumps(
            {
                "documents": [
                    {
                        "document_id": "Jane-Example-MRN-12345",
                        "pages": [{"page_number": 1, "image_path": str(page)}],
                    }
                ]
            }
        ),
        encoding="utf-8",
    )

    completed = subprocess.run(
        [
            sys.executable,
            "scripts/ocr_eval/run_ocr_batch_quality.py",
            "--mock",
            "--runner-path",
            "scripts/ocr_eval/run_small_ocr.py",
            "--manifest",
            str(manifest),
        ],
        cwd=REPO_ROOT,
        text=True,
        capture_output=True,
        check=True,
    )

    payload = json.loads(completed.stdout)
    assert payload["documents"] == [
        {"document_id": "document-1", "page_count": 1, "succeeded_count": 1, "failed_count": 0}
    ]
    assert payload["items"][0]["document_id"] == "document-1"
    rendered = json.dumps(payload, sort_keys=True)
    assert "Jane-Example-MRN-12345" not in rendered
    assert "Patient Jane Example" not in rendered
    assert completed.stderr == ""


def test_batch_quality_manifest_duplicate_raw_document_ids_do_not_merge_or_leak(tmp_path):
    page1 = tmp_path / "duplicate-patient-id-page-1.png"
    page2 = tmp_path / "duplicate-patient-id-page-2.png"
    for page in (page1, page2):
        page.write_bytes(b"synthetic image placeholder")
    (tmp_path / "synthetic_printed_phi_expected.txt").write_text(
        "Patient Jane Example\nMRN MRN-12345\n", encoding="utf-8"
    )
    manifest = tmp_path / "manifest.json"
    manifest.write_text(
        json.dumps(
            {
                "documents": [
                    {
                        "document_id": "duplicate-patient-id",
                        "pages": [{"page_number": 1, "image_path": str(page1)}],
                    },
                    {
                        "document_id": "duplicate-patient-id",
                        "pages": [{"page_number": 1, "image_path": str(page2)}],
                    },
                ]
            }
        ),
        encoding="utf-8",
    )

    completed = subprocess.run(
        [
            sys.executable,
            "scripts/ocr_eval/run_ocr_batch_quality.py",
            "--mock",
            "--runner-path",
            "scripts/ocr_eval/run_small_ocr.py",
            "--manifest",
            str(manifest),
        ],
        cwd=REPO_ROOT,
        text=True,
        capture_output=True,
        check=True,
    )

    payload = json.loads(completed.stdout)
    assert payload["document_count"] == 2
    assert payload["documents"] == [
        {"document_id": "document-1", "page_count": 1, "succeeded_count": 1, "failed_count": 0},
        {"document_id": "document-2", "page_count": 1, "succeeded_count": 1, "failed_count": 0},
    ]
    assert [item["document_id"] for item in payload["items"]] == ["document-1", "document-2"]
    rendered = json.dumps(payload, sort_keys=True)
    assert "duplicate-patient-id" not in rendered
    assert completed.stderr == ""


def test_batch_quality_manifest_recovers_from_missing_page_without_path_leaks(tmp_path):
    present = tmp_path / "Jane-Example-MRN-12345-doc-a-page-1.png"
    missing = tmp_path / "Jane-Example-MRN-12345-doc-a-page-2.png"
    present.write_bytes(b"synthetic image placeholder")
    (tmp_path / "synthetic_printed_phi_expected.txt").write_text(
        "Patient Jane Example\nMRN MRN-12345\n", encoding="utf-8"
    )
    manifest = tmp_path / "manifest.json"
    manifest.write_text(
        json.dumps(
            {
                "documents": [
                    {
                        "document_id": "doc-a",
                        "pages": [
                            {"page_number": 1, "image_path": str(present)},
                            {"page_number": 2, "image_path": str(missing)},
                        ],
                    }
                ]
            }
        ),
        encoding="utf-8",
    )

    completed = subprocess.run(
        [
            sys.executable,
            "scripts/ocr_eval/run_ocr_batch_quality.py",
            "--mock",
            "--runner-path",
            "scripts/ocr_eval/run_small_ocr.py",
            "--manifest",
            str(manifest),
        ],
        cwd=REPO_ROOT,
        text=True,
        capture_output=True,
        check=True,
    )

    payload = json.loads(completed.stdout)
    assert payload["input_count"] == 2
    assert payload["document_count"] == 1
    assert payload["succeeded_count"] == 1
    assert payload["failed_count"] == 1
    assert payload["documents"] == [
        {"document_id": "document-1", "page_count": 2, "succeeded_count": 1, "failed_count": 1}
    ]
    assert payload["items"][1]["status"] == "failed"
    assert payload["items"][1]["document_id"] == "document-1"
    assert payload["items"][1]["page_number"] == 2
    assert payload["items"][1]["error"] == "OCR input path does not exist"
    rendered = json.dumps(payload, sort_keys=True)
    assert "Jane-Example-MRN-12345" not in rendered
    assert "Patient Jane Example" not in rendered
    assert completed.stderr == ""


def test_batch_quality_runner_counts_real_local_adapter_status(tmp_path):
    adapter_dir = tmp_path / "adapter"
    adapter_dir.mkdir()
    (adapter_dir / "paddleocr.py").write_text(
        "class PaddleOCR:\n"
        "    def __init__(self, **kwargs):\n"
        "        self.kwargs = kwargs\n"
        "    def ocr(self, image_path):\n"
        "        return [{'rec_texts': ['Patient Jane Example', 'MRN MRN-12345']}]\n",
        encoding="utf-8",
    )
    image = tmp_path / "Jane-Example-MRN-12345.png"
    image.write_bytes(b"synthetic image placeholder")
    env = {**__import__("os").environ, "PYTHONPATH": str(adapter_dir)}

    completed = subprocess.run(
        [
            sys.executable,
            "scripts/ocr_eval/run_ocr_batch_quality.py",
            "--runner-path",
            "scripts/ocr_eval/run_small_ocr.py",
            str(image),
        ],
        cwd=REPO_ROOT,
        text=True,
        capture_output=True,
        check=True,
        env=env,
    )

    payload = json.loads(completed.stdout)
    assert payload["succeeded_count"] == 1
    assert payload["failed_count"] == 0
    assert payload["engine_status_counts"] == {"local_paddleocr_execution": 1}
    assert payload["real_model_quality_verified"] is True
    assert payload["quality_scope"] == "local_real_ocr_execution_aggregate_text_metrics"


def test_batch_quality_runner_does_not_verify_partial_real_adapter_batch(tmp_path):
    adapter_dir = tmp_path / "adapter"
    adapter_dir.mkdir()
    (adapter_dir / "paddleocr.py").write_text(
        "class PaddleOCR:\n"
        "    def __init__(self, **kwargs):\n"
        "        self.kwargs = kwargs\n"
        "    def ocr(self, image_path):\n"
        "        return [{'rec_texts': ['Patient Jane Example', 'MRN MRN-12345']}]\n",
        encoding="utf-8",
    )
    image = tmp_path / "Jane-Example-MRN-12345.png"
    missing = tmp_path / "Jane-Example-MRN-12345-missing.png"
    image.write_bytes(b"synthetic image placeholder")
    env = {**__import__("os").environ, "PYTHONPATH": str(adapter_dir)}

    completed = subprocess.run(
        [
            sys.executable,
            "scripts/ocr_eval/run_ocr_batch_quality.py",
            "--runner-path",
            "scripts/ocr_eval/run_small_ocr.py",
            str(image),
            str(missing),
        ],
        cwd=REPO_ROOT,
        text=True,
        capture_output=True,
        check=True,
        env=env,
    )

    payload = json.loads(completed.stdout)
    assert payload["input_count"] == 2
    assert payload["succeeded_count"] == 1
    assert payload["failed_count"] == 1
    assert payload["engine_status_counts"] == {"local_paddleocr_execution": 1}
    assert payload["real_model_quality_verified"] is False
    assert payload["quality_scope"] == "fixture_or_mixed_execution_aggregate_text_metrics"
    rendered = json.dumps(payload, sort_keys=True)
    assert "Jane-Example-MRN-12345" not in rendered
    assert "Patient Jane Example" not in rendered
    assert completed.stderr == ""
