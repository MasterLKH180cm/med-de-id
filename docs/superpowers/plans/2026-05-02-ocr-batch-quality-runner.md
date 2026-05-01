# OCR Batch Quality Runner Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a local OCR batch/quality runner that can process multiple image files, recover from per-file failures, and emit PHI-safe aggregate quality metrics without claiming full OCR model quality when only fixtures are used.

**Architecture:** Extend the existing `scripts/ocr_eval/run_small_ocr.py` single-image runner with a new Python batch runner that shells out to that local runner for each input and records per-item success/failure. Add tests first for multi-file success, per-file recovery, and explicit real-mode status accounting. Keep the surface bounded to printed text extraction and truthfully mark fixture/mock quality as not real model-quality proof.

**Tech Stack:** Python 3 subprocess/json/argparse, pytest, existing PP-OCRv5 mobile spike fixtures.

---

## File Structure

- Create `scripts/ocr_eval/run_ocr_batch_quality.py`: CLI batch runner for multiple image paths using existing `run_small_ocr.py --json`; emits aggregate JSON with `artifact: ocr_batch_quality_summary`, item statuses, counts, engine-status counts, and explicit non-goals.
- Modify `tests/test_ocr_runner_contract.py`: add tests for batch success, failure recovery, and real local adapter status aggregation.

### Task 1: Add multi-file batch quality runner

**Files:**
- Create: `scripts/ocr_eval/run_ocr_batch_quality.py`
- Modify: `tests/test_ocr_runner_contract.py`

- [ ] **Step 1: Write the failing tests**

```python

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
```

- [ ] **Step 2: Run tests to verify RED**

Run: `python3 -m pytest tests/test_ocr_runner_contract.py::test_batch_quality_runner_processes_multiple_mock_images_without_path_leaks tests/test_ocr_runner_contract.py::test_batch_quality_runner_recovers_from_one_missing_input_without_phi_leaks tests/test_ocr_runner_contract.py::test_batch_quality_runner_counts_real_local_adapter_status -q`
Expected: FAIL because `scripts/ocr_eval/run_ocr_batch_quality.py` does not exist.

- [ ] **Step 3: Implement minimal runner**

Create `scripts/ocr_eval/run_ocr_batch_quality.py` with argparse for `--mock`, `--runner-path`, and image paths; run the existing runner once per input with `--json`; never include raw paths or OCR text in the aggregate; emit successful and failed item summaries.

- [ ] **Step 4: Run targeted tests to verify GREEN**

Run: `python3 -m pytest tests/test_ocr_runner_contract.py::test_batch_quality_runner_processes_multiple_mock_images_without_path_leaks tests/test_ocr_runner_contract.py::test_batch_quality_runner_recovers_from_one_missing_input_without_phi_leaks tests/test_ocr_runner_contract.py::test_batch_quality_runner_counts_real_local_adapter_status -q`
Expected: PASS.

- [ ] **Step 5: Run broader OCR regression**

Run: `python3 -m pytest tests/test_ocr_runner_contract.py tests/test_ocr_handoff_contract.py tests/test_ocr_privacy_evidence_runner.py -q`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add scripts/ocr_eval/run_ocr_batch_quality.py tests/test_ocr_runner_contract.py docs/superpowers/plans/2026-05-02-ocr-batch-quality-runner.md
git commit -m "feat(ocr): add batch quality runner"
```
