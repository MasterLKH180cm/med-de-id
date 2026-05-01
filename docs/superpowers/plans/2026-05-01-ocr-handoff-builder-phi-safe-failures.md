# OCR Handoff Builder PHI-Safe Failure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Harden the bounded PP-OCRv5 mobile synthetic OCR handoff builder so missing/empty OCR input failures are PHI-safe, deterministic, and do not create stale handoff reports.

**Architecture:** This is a CLI/runtime-only PP-OCRv5 mobile spike hardening slice. `scripts/ocr_eval/build_ocr_handoff.py` remains the single handoff builder, but gains explicit input/source validation, PHI-safe stderr messages, and stale-output cleanup before validation or writes; the existing validator stays the handoff contract authority.

**Tech Stack:** Python 3 standard library, existing synthetic OCR fixtures under `scripts/ocr_eval/fixtures`, pytest-compatible script-level subprocess tests, markdown docs.

---

## File Structure

- Modify: `scripts/ocr_eval/build_ocr_handoff.py` — validate paths and OCR text before building the handoff JSON; remove stale output on failure; emit generic PHI-safe errors.
- Create: `tests/test_ocr_handoff_builder_failures.py` — subprocess tests for missing input, empty input, and valid fixture behavior.
- Modify: `docs/research/small-ocr-spike-results.md` — record the PHI-safe failure hardening evidence and non-goals.
- Modify: `README.md` — truth-sync evidence/completion without claiming browser/desktop or visual redaction progress.

### Task 1: Add PHI-safe builder failure tests and implementation

**Files:**
- Create: `tests/test_ocr_handoff_builder_failures.py`
- Modify: `scripts/ocr_eval/build_ocr_handoff.py`

- [ ] **Step 1: Write the failing tests**

Create `tests/test_ocr_handoff_builder_failures.py` with:

```python
import json
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
BUILDER = REPO / "scripts" / "ocr_eval" / "build_ocr_handoff.py"
FIXTURE_IMAGE = REPO / "scripts" / "ocr_eval" / "fixtures" / "synthetic_printed_phi_line.png"
FIXTURE_TEXT = REPO / "scripts" / "ocr_eval" / "fixtures" / "synthetic_printed_phi_expected.txt"
RAW_FIXTURE_VALUES = ["Jane Example", "jane@example.com", "+1-555-123-4567", "MRN-12345"]


def run_builder(source: Path, input_path: Path, output_path: Path):
    return subprocess.run(
        [
            sys.executable,
            str(BUILDER),
            "--source",
            str(source),
            "--input",
            str(input_path),
            "--output",
            str(output_path),
        ],
        cwd=REPO,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )


def assert_no_raw_fixture_phi(text: str):
    for value in RAW_FIXTURE_VALUES:
        assert value not in text


def test_ocr_handoff_builder_rejects_missing_input_without_phi_or_stale_report(tmp_path):
    missing_input = tmp_path / "missing-ocr.txt"
    output_path = tmp_path / "handoff.json"
    output_path.write_text('{"stale": true}\n', encoding="utf-8")

    result = run_builder(FIXTURE_IMAGE, missing_input, output_path)

    assert result.returncode == 2
    assert "OCR input file is missing" in result.stderr
    assert str(missing_input) not in result.stderr
    assert_no_raw_fixture_phi(result.stdout + result.stderr)
    assert not output_path.exists()


def test_ocr_handoff_builder_rejects_empty_input_without_creating_report(tmp_path):
    empty_input = tmp_path / "empty-ocr.txt"
    empty_input.write_text(" \n\t\n", encoding="utf-8")
    output_path = tmp_path / "handoff.json"

    result = run_builder(FIXTURE_IMAGE, empty_input, output_path)

    assert result.returncode == 2
    assert "OCR input text is empty" in result.stderr
    assert_no_raw_fixture_phi(result.stdout + result.stderr)
    assert not output_path.exists()


def test_ocr_handoff_builder_valid_fixture_still_writes_contract(tmp_path):
    output_path = tmp_path / "handoff.json"

    result = run_builder(FIXTURE_IMAGE, FIXTURE_TEXT, output_path)

    assert result.returncode == 0
    assert output_path.exists()
    handoff = json.loads(output_path.read_text(encoding="utf-8"))
    assert handoff["candidate"] == "PP-OCRv5_mobile_rec"
    assert handoff["engine"] == "PP-OCRv5-mobile-bounded-spike"
    assert handoff["scope"] == "printed_text_line_extraction_only"
    assert handoff["privacy_filter_contract"] == "text_only_normalized_input"
    assert handoff["ready_for_text_pii_eval"] is True
```

- [ ] **Step 2: Run tests to verify RED**

Run: `pytest tests/test_ocr_handoff_builder_failures.py -q`

Expected: FAIL because the current builder allows Python tracebacks/raw paths and leaves stale output on missing/empty input failures.

- [ ] **Step 3: Write minimal implementation**

Replace `scripts/ocr_eval/build_ocr_handoff.py` with:

```python
#!/usr/bin/env python3
import argparse
import json
import sys
from pathlib import Path


def norm(s: str) -> str:
    return " ".join(s.split())


def remove_stale_output(path: Path) -> None:
    try:
        path.unlink()
    except FileNotFoundError:
        return


def fail(output_path: Path, message: str) -> int:
    remove_stale_output(output_path)
    print(message, file=sys.stderr)
    return 2


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", required=True)
    parser.add_argument("--input", required=True)
    parser.add_argument("--output", required=True)
    return parser


def main() -> int:
    args = build_parser().parse_args()
    source_path = Path(args.source)
    input_path = Path(args.input)
    output_path = Path(args.output)

    remove_stale_output(output_path)

    if not source_path.exists():
        return fail(output_path, "OCR source file is missing")
    if not input_path.exists():
        return fail(output_path, "OCR input file is missing")

    text = input_path.read_text(encoding="utf-8")
    normalized_text = norm(text)
    if not normalized_text:
        return fail(output_path, "OCR input text is empty")

    obj = {
        "source": source_path.name,
        "extracted_text": text.strip(),
        "normalized_text": normalized_text,
        "ready_for_text_pii_eval": True,
        "candidate": "PP-OCRv5_mobile_rec",
        "engine": "PP-OCRv5-mobile-bounded-spike",
        "engine_status": "deterministic_synthetic_fixture_fallback",
        "scope": "printed_text_line_extraction_only",
        "privacy_filter_contract": "text_only_normalized_input",
        "non_goals": [
            "visual_redaction",
            "final_pdf_rewrite_export",
            "handwriting_recognition",
            "full_page_detection_or_segmentation",
            "complete_ocr_pipeline",
        ],
    }
    output_path.write_text(json.dumps(obj, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(output_path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
```

- [ ] **Step 4: Run targeted tests to verify GREEN**

Run: `pytest tests/test_ocr_handoff_builder_failures.py -q`

Expected: PASS.

- [ ] **Step 5: Run supporting OCR/Privacy Filter validators**

Run:

```bash
python scripts/ocr_eval/run_small_ocr.py --mock scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png > /tmp/small-ocr-output.txt
python scripts/ocr_eval/validate_small_ocr_output.py /tmp/small-ocr-output.txt scripts/ocr_eval/fixtures/synthetic_printed_phi_expected.txt
python scripts/ocr_eval/build_ocr_handoff.py --source scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png --input /tmp/small-ocr-output.txt --output /tmp/ocr-handoff.json
python scripts/ocr_eval/validate_ocr_handoff.py /tmp/ocr-handoff.json
python scripts/privacy_filter/run_privacy_filter.py --mock /tmp/small-ocr-output.txt > /tmp/privacy-filter-output.json
python scripts/privacy_filter/validate_privacy_filter_output.py /tmp/privacy-filter-output.json
```

Expected: all commands exit 0.

- [ ] **Step 6: Commit**

Run:

```bash
git add scripts/ocr_eval/build_ocr_handoff.py tests/test_ocr_handoff_builder_failures.py
git commit -m "fix(ocr): harden handoff builder failures"
```

### Task 2: Truth-sync docs for PHI-safe OCR handoff failure hardening

**Files:**
- Modify: `docs/research/small-ocr-spike-results.md`
- Modify: `README.md`

- [ ] **Step 1: Write the failing docs check**

Run:

```bash
python - <<'PY'
from pathlib import Path
readme = Path('README.md').read_text()
results = Path('docs/research/small-ocr-spike-results.md').read_text()
required = [
    'OCR handoff builder PHI-safe failure hardening',
    'missing OCR input file',
    'empty OCR input text',
    'stale handoff report cleanup',
    'not visual redaction',
]
missing = [term for term in required if term not in readme + '\n' + results]
if missing:
    raise SystemExit('missing docs terms: ' + ', '.join(missing))
PY
```

Expected: FAIL until docs mention the failure hardening evidence and non-goals.

- [ ] **Step 2: Update docs with exact evidence**

Add a short section to `docs/research/small-ocr-spike-results.md` named `OCR handoff builder PHI-safe failure hardening` that lists `pytest tests/test_ocr_handoff_builder_failures.py -q`, the OCR validator pipeline, and states this only hardens CLI/runtime synthetic OCR handoff failures; it is not visual redaction, browser/desktop integration, handwriting recognition, or final PDF rewrite/export.

Update `README.md` verification evidence to mention the new failure-hardening test while keeping completion honest: CLI 95%, Browser/Web 93%, Desktop app 93%, Overall 95% unless controller-visible facts support a rubric re-baseline.

- [ ] **Step 3: Run docs check to verify it passes**

Run the Python docs check from Step 1 again.

Expected: PASS.

- [ ] **Step 4: Run final verification**

Run:

```bash
pytest tests/test_ocr_handoff_builder_failures.py -q
cargo test -p mdid-cli cli_ocr_handoff_normalized_text_feeds_privacy_filter_without_phi_leaks -- --nocapture
python scripts/privacy_filter/validate_privacy_filter_output.py scripts/privacy_filter/fixtures/sample_text_expected_shape.json
git diff --check
```

Expected: all commands exit 0.

- [ ] **Step 5: Commit**

Run:

```bash
git add README.md docs/research/small-ocr-spike-results.md
git commit -m "docs: truth-sync OCR handoff failure hardening"
```

## Self-Review

Spec coverage: Task 1 covers deterministic PHI-safe failure behavior for the PP-OCRv5 mobile handoff builder and validates the existing successful fixture path. Task 2 covers README/research truth-sync and completion honesty. No browser/desktop capability, visual redaction, OCR quality, handwriting, or PDF rewrite/export capability is claimed.

Placeholder scan: No TBD/TODO/fill-in placeholders remain.

Type consistency: The plan uses existing scripts and command names: `run_small_ocr.py`, `validate_small_ocr_output.py`, `build_ocr_handoff.py`, `validate_ocr_handoff.py`, `run_privacy_filter.py`, and `validate_privacy_filter_output.py`.
