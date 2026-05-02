# OCR Manifest Document Batch Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the local OCR batch-quality runner with a PHI-safe manifest mode for multi-file/multi-page document batches while preserving per-page failure recovery.

**Architecture:** Keep OCR execution local and bounded to the existing `scripts/ocr_eval/run_small_ocr.py --json` runner. Add manifest parsing in the Python batch runner only; group items by document IDs supplied by the manifest, emit aggregate document/page counts and recovery status, and never expose source paths, filenames, OCR text, normalized text, bbox data, or raw PHI in aggregate output.

**Tech Stack:** Python 3 argparse/json/subprocess, pytest, existing OCR fixture tests.

---

## File Structure

- Modify: `scripts/ocr_eval/run_ocr_batch_quality.py`
  - Add optional `--manifest <json>` mode. The manifest shape is `{ "documents": [{ "document_id": "doc-001", "pages": [{ "page_number": 1, "image_path": "..." }] }] }`.
  - Validate bounded page records, continue after per-page failures, and emit PHI-safe document summaries.
- Modify: `tests/test_ocr_runner_contract.py`
  - Add manifest mode tests for multi-document/multi-page success plus missing-page recovery without path/PHI leaks.
- Modify: `README.md`
  - Truth-sync the completion snapshot with the new bounded manifest evidence and explicit non-goals.

### Task 1: Add OCR manifest document batch mode

**Files:**
- Modify: `scripts/ocr_eval/run_ocr_batch_quality.py`
- Modify: `tests/test_ocr_runner_contract.py`
- Modify: `README.md`

- [x] **Step 1: Write failing tests**

Add these tests to `tests/test_ocr_runner_contract.py`:

```python
def test_batch_quality_manifest_groups_multi_page_documents_without_path_or_text_leaks(tmp_path):
    page1 = tmp_path / "Jane-Example-MRN-12345-doc-a-page-1.png"
    page2 = tmp_path / "Jane-Example-MRN-12345-doc-a-page-2.png"
    page3 = tmp_path / "Jane-Example-MRN-12345-doc-b-page-1.png"
    for page in (page1, page2, page3):
        page.write_bytes(b"synthetic image placeholder")
    (tmp_path / "synthetic_printed_phi_expected.txt").write_text("Patient Jane Example\nMRN MRN-12345\n", encoding="utf-8")
    manifest = tmp_path / "manifest.json"
    manifest.write_text(json.dumps({"documents": [{"document_id": "doc-a", "pages": [{"page_number": 1, "image_path": str(page1)}, {"page_number": 2, "image_path": str(page2)}]}, {"document_id": "doc-b", "pages": [{"page_number": 1, "image_path": str(page3)}]}]}), encoding="utf-8")

    completed = subprocess.run([sys.executable, "scripts/ocr_eval/run_ocr_batch_quality.py", "--mock", "--runner-path", "scripts/ocr_eval/run_small_ocr.py", "--manifest", str(manifest)], cwd=REPO_ROOT, text=True, capture_output=True, check=True)

    payload = json.loads(completed.stdout)
    assert payload["artifact"] == "ocr_batch_quality_summary"
    assert payload["input_count"] == 3
    assert payload["document_count"] == 2
    assert payload["succeeded_count"] == 3
    assert payload["failed_count"] == 0
    assert payload["documents"] == [{"document_id": "doc-a", "page_count": 2, "succeeded_count": 2, "failed_count": 0}, {"document_id": "doc-b", "page_count": 1, "succeeded_count": 1, "failed_count": 0}]
    rendered = json.dumps(payload, sort_keys=True)
    assert "Jane-Example-MRN-12345" not in rendered
    assert "Patient Jane Example" not in rendered
    assert completed.stderr == ""


def test_batch_quality_manifest_recovers_from_missing_page_without_path_leaks(tmp_path):
    present = tmp_path / "Jane-Example-MRN-12345-doc-a-page-1.png"
    missing = tmp_path / "Jane-Example-MRN-12345-doc-a-page-2.png"
    present.write_bytes(b"synthetic image placeholder")
    (tmp_path / "synthetic_printed_phi_expected.txt").write_text("Patient Jane Example\nMRN MRN-12345\n", encoding="utf-8")
    manifest = tmp_path / "manifest.json"
    manifest.write_text(json.dumps({"documents": [{"document_id": "doc-a", "pages": [{"page_number": 1, "image_path": str(present)}, {"page_number": 2, "image_path": str(missing)}]}]}), encoding="utf-8")

    completed = subprocess.run([sys.executable, "scripts/ocr_eval/run_ocr_batch_quality.py", "--mock", "--runner-path", "scripts/ocr_eval/run_small_ocr.py", "--manifest", str(manifest)], cwd=REPO_ROOT, text=True, capture_output=True, check=True)

    payload = json.loads(completed.stdout)
    assert payload["input_count"] == 2
    assert payload["document_count"] == 1
    assert payload["succeeded_count"] == 1
    assert payload["failed_count"] == 1
    assert payload["documents"] == [{"document_id": "doc-a", "page_count": 2, "succeeded_count": 1, "failed_count": 1}]
    assert payload["items"][1]["status"] == "failed"
    assert payload["items"][1]["document_id"] == "doc-a"
    assert payload["items"][1]["page_number"] == 2
    assert payload["items"][1]["error"] == "OCR input path does not exist"
    rendered = json.dumps(payload, sort_keys=True)
    assert "Jane-Example-MRN-12345" not in rendered
    assert "Patient Jane Example" not in rendered
    assert completed.stderr == ""
```

- [x] **Step 2: Run RED**

Run: `python3 -m pytest tests/test_ocr_runner_contract.py::test_batch_quality_manifest_groups_multi_page_documents_without_path_or_text_leaks tests/test_ocr_runner_contract.py::test_batch_quality_manifest_recovers_from_missing_page_without_path_leaks -q`

Expected: FAIL because `--manifest` is not supported and `document_count`/`documents` are absent.

- [x] **Step 3: Implement minimal manifest support**

Update `scripts/ocr_eval/run_ocr_batch_quality.py` so `--manifest` and positional image paths are mutually exclusive; parse document/page records; call the existing per-image logic for each page; add `document_id` and `page_number` to each PHI-safe item; add top-level `document_count` and `documents` summaries. Reject malformed manifests with fixed PHI-safe stderr text and exit code 2.

- [x] **Step 4: Verify GREEN and regression**

Run:

```bash
python3 -m pytest tests/test_ocr_runner_contract.py::test_batch_quality_manifest_groups_multi_page_documents_without_path_or_text_leaks tests/test_ocr_runner_contract.py::test_batch_quality_manifest_recovers_from_missing_page_without_path_leaks -q
python3 -m pytest tests/test_ocr_runner_contract.py -q -k batch
python3 -m pytest tests/test_ocr_runner_contract.py tests/test_ocr_handoff_contract.py tests/test_ocr_privacy_evidence_runner.py -q
```

Expected: PASS.

- [x] **Step 5: Truth-sync README and commit**

Update the README completion snapshot to mention bounded OCR manifest document/page batch evidence while explicitly not claiming full OCR, PDF OCR, handwriting recognition, visual redaction, Browser/Desktop OCR execution, or model-quality acceptance.

Run: `git diff --check`

Commit:

```bash
git add scripts/ocr_eval/run_ocr_batch_quality.py tests/test_ocr_runner_contract.py README.md docs/superpowers/plans/2026-05-02-ocr-manifest-document-batch.md
git commit -m "feat(ocr): add manifest document batch evidence"
```
