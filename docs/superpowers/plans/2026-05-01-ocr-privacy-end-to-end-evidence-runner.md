# OCR Privacy End-to-End Evidence Runner Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a bounded CLI/runtime evidence runner that composes the existing PP-OCRv5 mobile synthetic printed-text OCR handoff path with the text-only Privacy Filter path and emits an aggregate-only PHI-safe verification artifact.

**Architecture:** Add a small Python evidence runner under `scripts/ocr_eval/` that invokes existing checked-in scripts rather than reimplementing OCR or Privacy Filter logic. The runner writes temporary OCR handoff and Privacy Filter artifacts internally, validates them with the existing validators, then writes a strict aggregate evidence JSON that contains only safe counts, candidate/scope metadata, network status, and non-goals. This is CLI/runtime evidence only and must not claim Browser/Desktop execution, visual redaction, image pixel redaction, handwriting recognition, or final PDF rewrite/export.

**Tech Stack:** Python stdlib (`argparse`, `json`, `subprocess`, `tempfile`, `pathlib`), existing `scripts/ocr_eval/run_small_ocr.py`, `scripts/ocr_eval/validate_ocr_handoff.py`, `scripts/privacy_filter/run_privacy_filter.py`, `scripts/privacy_filter/validate_privacy_filter_output.py`, pytest.

---

## File Structure

- Create: `scripts/ocr_eval/run_ocr_privacy_evidence.py` — CLI runner that composes checked-in OCR and Privacy Filter scripts and writes aggregate-only PHI-safe evidence JSON.
- Create: `tests/test_ocr_privacy_evidence_runner.py` — subprocess tests for success, stale-output cleanup, missing image failure, and PHI-safety of stdout/stderr/report.
- Modify: `scripts/ocr_eval/README.md` — document the evidence runner, exact command, non-goals, and downstream Privacy Filter boundary.
- Modify: `README.md` — truth-sync the current completion snapshot with the new CLI/runtime evidence requirement and verification commands; keep Browser/Web and Desktop unchanged at 99% and mark this as CLI/runtime-only.

### Task 1: Add aggregate-only OCR Privacy evidence runner

**Files:**
- Create: `scripts/ocr_eval/run_ocr_privacy_evidence.py`
- Create: `tests/test_ocr_privacy_evidence_runner.py`
- Modify: `scripts/ocr_eval/README.md`
- Modify: `README.md`

- [ ] **Step 1: Write the failing success-path test**

Add this test file:

```python
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RUNNER = ROOT / "scripts" / "ocr_eval" / "run_ocr_privacy_evidence.py"
FIXTURE = ROOT / "scripts" / "ocr_eval" / "fixtures" / "synthetic_printed_phi_line.png"
OCR_RUNNER = ROOT / "scripts" / "ocr_eval" / "run_small_ocr.py"
PRIVACY_RUNNER = ROOT / "scripts" / "privacy_filter" / "run_privacy_filter.py"
SENTINELS = ["Jane Example", "MRN-12345", "jane@example.com", "555-123-4567", "+1-555-123-4567"]


def run_evidence(output: Path, image: Path = FIXTURE):
    return subprocess.run(
        [
            sys.executable,
            str(RUNNER),
            "--image-path",
            str(image),
            "--ocr-runner-path",
            str(OCR_RUNNER),
            "--privacy-runner-path",
            str(PRIVACY_RUNNER),
            "--output",
            str(output),
            "--mock",
        ],
        cwd=ROOT,
        text=True,
        capture_output=True,
        timeout=15,
    )


def test_evidence_runner_writes_aggregate_only_phi_safe_report(tmp_path):
    output = tmp_path / "ocr-privacy-evidence.json"

    result = run_evidence(output)

    assert result.returncode == 0, result.stderr
    assert result.stderr == ""
    assert '"report_path": "<redacted>"' in result.stdout
    report_text = output.read_text(encoding="utf-8")
    for sentinel in SENTINELS:
        assert sentinel not in result.stdout
        assert sentinel not in result.stderr
        assert sentinel not in report_text
    report = json.loads(report_text)
    assert report == {
        "artifact": "ocr_privacy_evidence",
        "ocr_candidate": "PP-OCRv5_mobile_rec",
        "ocr_engine": "PP-OCRv5-mobile-bounded-spike",
        "ocr_scope": "printed_text_line_extraction_only",
        "ocr_engine_status": "deterministic_synthetic_fixture_fallback",
        "privacy_filter_engine": "fallback_synthetic_patterns",
        "privacy_filter_contract": "text_only_normalized_input",
        "privacy_scope": "text_only_pii_detection",
        "ready_for_text_pii_eval": True,
        "network_api_called": False,
        "detected_span_count": 5,
        "category_counts": {"EMAIL": 1, "MRN": 1, "NAME": 1, "PHONE": 1, "ID": 1},
        "non_goals": [
            "browser_ui",
            "complete_ocr_pipeline",
            "desktop_ui",
            "final_pdf_rewrite_export",
            "handwriting_recognition",
            "image_pixel_redaction",
            "visual_redaction",
        ],
    }
```

- [ ] **Step 2: Run the success-path test to verify RED**

Run: `python -m pytest tests/test_ocr_privacy_evidence_runner.py::test_evidence_runner_writes_aggregate_only_phi_safe_report -q`
Expected: FAIL because `scripts/ocr_eval/run_ocr_privacy_evidence.py` does not exist.

- [ ] **Step 3: Write the minimal evidence runner**

Create `scripts/ocr_eval/run_ocr_privacy_evidence.py` with complete code that parses the documented flags, removes stale output before work, runs `run_small_ocr.py --json --mock <image>`, validates/extracts normalized text, runs `run_privacy_filter.py --stdin --mock`, validates the Privacy Filter output, writes only the aggregate allowlisted JSON shown in the test, prints `{"artifact":"ocr_privacy_evidence","report_path":"<redacted>","report_written":true}` on success, and emits generic PHI-safe errors on failure.

- [ ] **Step 4: Run the success-path test to verify GREEN**

Run: `python -m pytest tests/test_ocr_privacy_evidence_runner.py::test_evidence_runner_writes_aggregate_only_phi_safe_report -q`
Expected: PASS.

- [ ] **Step 5: Add failing failure-path tests**

Extend `tests/test_ocr_privacy_evidence_runner.py` with tests that a missing image returns nonzero, removes a stale output containing `Jane Example`, emits no stdout, and stderr contains exactly `OCR Privacy evidence input image is missing`; and that a PHI-bearing output directory name is never echoed in stdout/stderr.

- [ ] **Step 6: Run failure-path tests to verify RED/GREEN loop as needed**

Run: `python -m pytest tests/test_ocr_privacy_evidence_runner.py -q`
Expected: PASS after minimal implementation fixes.

- [ ] **Step 7: Update docs and completion truth-sync**

Update `scripts/ocr_eval/README.md` with the exact command and non-goals. Update `README.md` completion snapshot by adding one completed CLI/runtime evidence requirement to the fraction accounting (`107/112 -> 108/113 = 95%` floor), keeping CLI 95%, Browser/Web 99%, Desktop 99%, and Overall 97%. State this is CLI/runtime-only and Browser/Desktop +5 are FAIL/not claimed.

- [ ] **Step 8: Verify targeted and integration commands**

Run:

```bash
python -m pytest tests/test_ocr_privacy_evidence_runner.py -q
python scripts/ocr_eval/run_ocr_privacy_evidence.py \
  --image-path scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png \
  --ocr-runner-path scripts/ocr_eval/run_small_ocr.py \
  --privacy-runner-path scripts/privacy_filter/run_privacy_filter.py \
  --output /tmp/ocr-privacy-evidence.json \
  --mock
python -m py_compile scripts/ocr_eval/run_ocr_privacy_evidence.py
cargo fmt --check
git diff --check
```

Expected: all pass.

- [ ] **Step 9: Commit**

```bash
git add README.md scripts/ocr_eval/README.md scripts/ocr_eval/run_ocr_privacy_evidence.py tests/test_ocr_privacy_evidence_runner.py docs/superpowers/plans/2026-05-01-ocr-privacy-end-to-end-evidence-runner.md
git commit -m "feat(cli): add OCR privacy evidence runner"
```

## Self-Review

Spec coverage: This plan covers a bounded CLI/runtime PP-OCRv5 mobile synthetic printed-text extraction evidence runner and downstream text-only Privacy Filter validation. It explicitly excludes OCR quality, visual redaction, image pixel redaction, Browser/Desktop execution, handwriting, and final PDF rewrite/export.

Placeholder scan: No TBD/TODO/fill-in placeholders are present; implementation steps include concrete expected behavior, exact files, commands, and output contracts.

Type consistency: Artifact names, metadata field names, category labels, and non-goals match across tests, runner contract, docs, and README truth-sync.
