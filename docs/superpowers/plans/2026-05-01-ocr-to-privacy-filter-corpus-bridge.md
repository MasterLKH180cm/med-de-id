# OCR to Privacy Filter Corpus Bridge Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded CLI/runtime corpus bridge proving PP-OCRv5 mobile synthetic OCR handoff text can feed the text-only Privacy Filter corpus contract as aggregate PHI-safe evidence.

**Architecture:** Add a Python helper that runs the existing OCR handoff corpus runner, extracts only normalized text through temporary files, runs the existing Privacy Filter runner per fixture, validates every Privacy Filter report, and writes an aggregate-only JSON bridge report. This remains CLI/runtime evidence only: no OCR model quality claims, no visual redaction, no image pixel redaction, no final PDF rewrite/export, no Browser/Desktop UI, and no unrelated workflow orchestration semantics.

**Tech Stack:** Python scripts under `scripts/ocr_eval` and `scripts/privacy_filter`, pytest, JSON schema-style validation with repository helpers, markdown README evidence.

---

## File Structure

- Create: `scripts/ocr_eval/run_ocr_to_privacy_filter_corpus.py` — aggregate-only bridge runner from OCR handoff corpus to text-only Privacy Filter reports.
- Create: `tests/test_ocr_to_privacy_filter_corpus.py` — fixture-backed tests for success, PHI-safety, validation failure cleanup, and no raw OCR text in aggregate output.
- Modify: `docs/research/small-ocr-spike-results.md` — record bridge evidence and non-goals.
- Modify: `README.md` — truth-sync completion snapshot/evidence without inflating Browser/Web or Desktop capability.

### Task 1: Add OCR-to-Privacy-Filter corpus bridge runner

**Files:**
- Create: `scripts/ocr_eval/run_ocr_to_privacy_filter_corpus.py`
- Create: `tests/test_ocr_to_privacy_filter_corpus.py`

- [ ] **Step 1: Write the failing success and PHI-safety tests**

Create `tests/test_ocr_to_privacy_filter_corpus.py` with:

```python
import json
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
SCRIPT = REPO_ROOT / "scripts" / "ocr_eval" / "run_ocr_to_privacy_filter_corpus.py"
OCR_RUNNER = REPO_ROOT / "scripts" / "ocr_eval" / "run_ocr_handoff_corpus.py"
PRIVACY_RUNNER = REPO_ROOT / "scripts" / "privacy_filter" / "run_privacy_filter.py"
FIXTURE_DIR = REPO_ROOT / "scripts" / "ocr_eval" / "fixtures" / "corpus"
RAW_SENTINELS = ["Jane Example", "MRN-12345", "jane@example.com", "555-123-4567"]


def run_bridge(output_path: Path, extra_args=None):
    args = [
        sys.executable,
        str(SCRIPT),
        "--fixture-dir",
        str(FIXTURE_DIR),
        "--ocr-runner-path",
        str(OCR_RUNNER),
        "--privacy-runner-path",
        str(PRIVACY_RUNNER),
        "--output",
        str(output_path),
    ]
    if extra_args:
        args.extend(extra_args)
    return subprocess.run(args, cwd=REPO_ROOT, text=True, capture_output=True)


def test_ocr_to_privacy_filter_corpus_writes_phi_safe_aggregate(tmp_path):
    output_path = tmp_path / "bridge.json"

    result = run_bridge(output_path)

    assert result.returncode == 0, result.stderr
    stdout_stderr = result.stdout + result.stderr
    assert "ocr_to_privacy_filter_corpus" in result.stdout
    for sentinel in RAW_SENTINELS:
        assert sentinel not in stdout_stderr
    report_text = output_path.read_text(encoding="utf-8")
    for sentinel in RAW_SENTINELS:
        assert sentinel not in report_text
    report = json.loads(report_text)
    assert report["artifact"] == "ocr_to_privacy_filter_corpus_bridge"
    assert report["ocr_candidate"] == "PP-OCRv5_mobile_rec"
    assert report["ocr_engine"] == "PP-OCRv5-mobile-bounded-spike"
    assert report["scope"] == "printed_text_extraction_to_text_pii_detection_only"
    assert report["privacy_filter_engine"] == "fallback_synthetic_patterns"
    assert report["privacy_filter_contract"] == "text_only_normalized_input"
    assert report["fixture_count"] >= 2
    assert report["ready_fixture_count"] == report["fixture_count"]
    assert report["privacy_filter_detected_span_count"] >= 2
    assert "NAME" in report["privacy_filter_category_counts"]
    assert "MRN" in report["privacy_filter_category_counts"]
    assert "fixtures" in report
    for item in report["fixtures"]:
        assert sorted(item.keys()) == ["detected_span_count", "fixture", "ready_for_text_pii_eval"]
        assert item["fixture"].startswith("fixture_")
    assert "visual_redaction" in report["non_goals"]
    assert "final_pdf_rewrite_export" in report["non_goals"]
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pytest tests/test_ocr_to_privacy_filter_corpus.py::test_ocr_to_privacy_filter_corpus_writes_phi_safe_aggregate -q`

Expected: FAIL because `scripts/ocr_eval/run_ocr_to_privacy_filter_corpus.py` does not exist yet.

- [ ] **Step 3: Write minimal implementation**

Create `scripts/ocr_eval/run_ocr_to_privacy_filter_corpus.py` with a CLI that:

```python
#!/usr/bin/env python3
"""Aggregate-only PP-OCRv5 mobile OCR handoff to text Privacy Filter bridge."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tempfile
from pathlib import Path

NON_GOALS = [
    "ocr_model_quality_benchmark",
    "visual_redaction",
    "image_pixel_redaction",
    "handwriting_recognition",
    "final_pdf_rewrite_export",
    "browser_ui",
    "desktop_ui",
]
SAFE_CATEGORY_LABELS = {"NAME", "MRN", "EMAIL", "PHONE", "ID"}


def load_json(path: Path) -> dict:
    with path.open("r", encoding="utf-8") as handle:
        value = json.load(handle)
    if not isinstance(value, dict):
        raise ValueError(f"{path} must contain a JSON object")
    return value


def run_json_command(args: list[str], output_path: Path) -> dict:
    completed = subprocess.run(args, text=True, capture_output=True)
    if completed.returncode != 0:
        raise RuntimeError("helper command failed")
    return load_json(output_path)


def safe_fixture_id(index: int) -> str:
    return f"fixture_{index:03d}"


def extract_texts(ocr_report: dict) -> list[str]:
    fixtures = ocr_report.get("fixtures")
    if not isinstance(fixtures, list) or not fixtures:
        raise ValueError("OCR handoff corpus report must contain fixtures")
    texts: list[str] = []
    for fixture in fixtures:
        if not isinstance(fixture, dict):
            raise ValueError("OCR fixture entry must be an object")
        text = fixture.get("normalized_text") or fixture.get("extracted_text") or fixture.get("text")
        if not isinstance(text, str) or not text.strip():
            raise ValueError("OCR fixture entry must include normalized text")
        texts.append(text)
    return texts


def add_counts(total: dict[str, int], partial: dict) -> None:
    for label, value in partial.items():
        if label not in SAFE_CATEGORY_LABELS or not isinstance(value, int) or value < 0:
            raise ValueError("Privacy Filter category counts are outside the safe aggregate contract")
        total[label] = total.get(label, 0) + value


def build_bridge_report(args: argparse.Namespace) -> dict:
    with tempfile.TemporaryDirectory(prefix="mdid-ocr-privacy-bridge-") as tmp:
        tmpdir = Path(tmp)
        ocr_output = tmpdir / "ocr-corpus.json"
        ocr_report = run_json_command([
            sys.executable,
            str(args.ocr_runner_path),
            "--fixture-dir",
            str(args.fixture_dir),
            "--output",
            str(ocr_output),
        ], ocr_output)
        texts = extract_texts(ocr_report)
        category_counts: dict[str, int] = {}
        fixtures = []
        detected_total = 0
        privacy_engine = None
        for index, text in enumerate(texts, start=1):
            input_path = tmpdir / f"{safe_fixture_id(index)}.txt"
            privacy_output = tmpdir / f"{safe_fixture_id(index)}-privacy.json"
            input_path.write_text(text, encoding="utf-8")
            privacy_report = run_json_command([
                sys.executable,
                str(args.privacy_runner_path),
                str(input_path),
            ], privacy_output)
            summary = privacy_report.get("summary")
            metadata = privacy_report.get("metadata")
            if not isinstance(summary, dict) or not isinstance(metadata, dict):
                raise ValueError("Privacy Filter report missing summary or metadata")
            if metadata.get("network_api_called") is not False:
                raise ValueError("Privacy Filter bridge requires local no-network reports")
            engine = metadata.get("engine")
            if not isinstance(engine, str) or not engine:
                raise ValueError("Privacy Filter report missing safe engine")
            privacy_engine = privacy_engine or engine
            if privacy_engine != engine:
                raise ValueError("Privacy Filter reports used inconsistent engines")
            detected = summary.get("detected_span_count")
            if not isinstance(detected, int) or detected < 0:
                raise ValueError("Privacy Filter detected span count must be nonnegative integer")
            counts = summary.get("category_counts")
            if not isinstance(counts, dict):
                raise ValueError("Privacy Filter category counts must be an object")
            add_counts(category_counts, counts)
            detected_total += detected
            fixtures.append({
                "fixture": safe_fixture_id(index),
                "ready_for_text_pii_eval": detected > 0,
                "detected_span_count": detected,
            })
        return {
            "artifact": "ocr_to_privacy_filter_corpus_bridge",
            "ocr_candidate": "PP-OCRv5_mobile_rec",
            "ocr_engine": "PP-OCRv5-mobile-bounded-spike",
            "scope": "printed_text_extraction_to_text_pii_detection_only",
            "privacy_filter_engine": privacy_engine,
            "privacy_filter_contract": "text_only_normalized_input",
            "fixture_count": len(fixtures),
            "ready_fixture_count": sum(1 for item in fixtures if item["ready_for_text_pii_eval"]),
            "privacy_filter_detected_span_count": detected_total,
            "privacy_filter_category_counts": dict(sorted(category_counts.items())),
            "fixtures": fixtures,
            "non_goals": NON_GOALS,
        }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run bounded OCR-to-Privacy-Filter corpus bridge")
    parser.add_argument("--fixture-dir", type=Path, required=True)
    parser.add_argument("--ocr-runner-path", type=Path, required=True)
    parser.add_argument("--privacy-runner-path", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        args.output.unlink(missing_ok=True)
        if not args.fixture_dir.is_dir():
            raise ValueError("fixture dir is missing")
        if not args.ocr_runner_path.is_file():
            raise ValueError("OCR runner is missing")
        if not args.privacy_runner_path.is_file():
            raise ValueError("Privacy Filter runner is missing")
        report = build_bridge_report(args)
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        print(json.dumps({"artifact": "ocr_to_privacy_filter_corpus", "report_written": True, "fixture_count": report["fixture_count"]}))
        return 0
    except Exception as exc:
        args.output.unlink(missing_ok=True)
        print(f"error: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
```

- [ ] **Step 4: Run test to verify it passes**

Run: `pytest tests/test_ocr_to_privacy_filter_corpus.py::test_ocr_to_privacy_filter_corpus_writes_phi_safe_aggregate -q`

Expected: PASS.

- [ ] **Step 5: Commit**

Run:

```bash
git add scripts/ocr_eval/run_ocr_to_privacy_filter_corpus.py tests/test_ocr_to_privacy_filter_corpus.py
git commit -m "feat(ocr): bridge ocr corpus to privacy filter"
```

### Task 2: Add failure cleanup coverage and docs truth-sync

**Files:**
- Modify: `tests/test_ocr_to_privacy_filter_corpus.py`
- Modify: `docs/research/small-ocr-spike-results.md`
- Modify: `README.md`

- [ ] **Step 1: Write failing cleanup test**

Append to `tests/test_ocr_to_privacy_filter_corpus.py`:

```python
def test_ocr_to_privacy_filter_corpus_removes_stale_output_on_missing_runner(tmp_path):
    output_path = tmp_path / "bridge.json"
    output_path.write_text("stale Jane Example", encoding="utf-8")

    result = run_bridge(output_path, ["--privacy-runner-path", str(tmp_path / "missing.py")])

    assert result.returncode != 0
    assert not output_path.exists()
    assert "Jane Example" not in result.stdout
    assert "Jane Example" not in result.stderr
```

- [ ] **Step 2: Run test to verify it fails or passes for the intended reason**

Run: `pytest tests/test_ocr_to_privacy_filter_corpus.py::test_ocr_to_privacy_filter_corpus_removes_stale_output_on_missing_runner -q`

Expected: PASS if Task 1 already removed stale output during prerequisite failures; otherwise FAIL with stale output still present.

- [ ] **Step 3: Update docs evidence**

Add to `docs/research/small-ocr-spike-results.md`:

```markdown
## OCR-to-Privacy-Filter corpus bridge evidence

`python scripts/ocr_eval/run_ocr_to_privacy_filter_corpus.py --fixture-dir scripts/ocr_eval/fixtures/corpus --ocr-runner-path scripts/ocr_eval/run_ocr_handoff_corpus.py --privacy-runner-path scripts/privacy_filter/run_privacy_filter.py --output /tmp/ocr-to-privacy-filter-corpus.json` composes the bounded PP-OCRv5 mobile synthetic handoff corpus with the text-only Privacy Filter runner and writes an aggregate-only PHI-safe bridge report. The report includes safe fixture IDs, readiness counts, Privacy Filter detected-span/category counts, and explicit non-goals only; it omits raw OCR text, masked text, spans, previews, fixture filenames, paths, image data, bbox data, and raw synthetic PHI.

This remains CLI/runtime evidence for printed-text extraction feeding text-only PII detection. It is not visual redaction, image pixel redaction, handwriting recognition, final PDF rewrite/export, Browser/Web execution, or Desktop execution.
```

Add a README evidence paragraph that states the same bounded claim and keeps Browser/Web and Desktop completion unchanged.

- [ ] **Step 4: Run verification**

Run:

```bash
pytest tests/test_ocr_to_privacy_filter_corpus.py -q
python scripts/ocr_eval/run_ocr_to_privacy_filter_corpus.py --fixture-dir scripts/ocr_eval/fixtures/corpus --ocr-runner-path scripts/ocr_eval/run_ocr_handoff_corpus.py --privacy-runner-path scripts/privacy_filter/run_privacy_filter.py --output /tmp/ocr-to-privacy-filter-corpus.json
python -m json.tool /tmp/ocr-to-privacy-filter-corpus.json >/tmp/ocr-to-privacy-filter-corpus.pretty.json
git diff --check
```

Expected: all commands PASS.

- [ ] **Step 5: Commit**

Run:

```bash
git add tests/test_ocr_to_privacy_filter_corpus.py docs/research/small-ocr-spike-results.md README.md
git commit -m "docs: record ocr privacy filter corpus bridge"
```
