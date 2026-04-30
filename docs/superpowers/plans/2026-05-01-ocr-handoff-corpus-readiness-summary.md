# OCR Handoff Corpus Readiness Summary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded PHI-safe `--summary-output` artifact to the PP-OCRv5 mobile synthetic OCR handoff corpus runner so downstream Privacy Filter text-only evaluation can consume readiness evidence without raw OCR text.

**Architecture:** Extend the existing `scripts/ocr_eval/run_ocr_handoff_corpus.py` runner with an optional second JSON artifact that contains only aggregate readiness fields and explicit downstream contract metadata. Keep the full corpus report contract unchanged, and prove the new artifact never contains raw synthetic PHI, OCR lines, filenames, paths, spans, or image data.

**Tech Stack:** Python 3 standard library, pytest, existing synthetic OCR corpus fixtures, README truth-sync.

---

## File Structure

- Modify: `scripts/ocr_eval/run_ocr_handoff_corpus.py` — parse optional `--summary-output`, remove stale summary files, build a strict allowlisted downstream-readiness summary from the already-built aggregate report, and write it only after the full report succeeds.
- Modify: `tests/test_ocr_handoff_corpus.py` — add TDD coverage for successful summary output and stale-summary cleanup on failures.
- Modify: `README.md` — truth-sync the PP-OCRv5 mobile corpus evidence and completion snapshot without overclaiming OCR quality, visual redaction, PDF rewrite/export, Browser/Web execution, or Desktop execution.

### Task 1: Add OCR handoff corpus downstream readiness summary artifact

**Files:**
- Modify: `tests/test_ocr_handoff_corpus.py`
- Modify: `scripts/ocr_eval/run_ocr_handoff_corpus.py`

- [ ] **Step 1: Write the failing successful-summary test**

Add this test after `test_corpus_report_contains_only_aggregate_phi_safe_fields` in `tests/test_ocr_handoff_corpus.py`:

```python
def test_corpus_summary_output_contains_only_downstream_readiness_fields(tmp_path):
    output = tmp_path / "report.json"
    summary_output = tmp_path / "summary.json"

    result = run_corpus(
        "--fixture-dir",
        FIXTURE_DIR,
        "--output",
        output,
        "--summary-output",
        summary_output,
    )

    assert result.returncode == 0, result.stderr
    report_text = output.read_text(encoding="utf-8")
    summary_text = summary_output.read_text(encoding="utf-8")
    for forbidden in [*FORBIDDEN_SYNTHETIC_PHI, "fixture_001", "fixture_002", "sample", "ocr", "handoff"]:
        assert forbidden not in summary_text
    summary = json.loads(summary_text)
    assert summary == {
        "artifact": "ocr_handoff_corpus_readiness_summary",
        "candidate": "PP-OCRv5_mobile_rec",
        "engine": "PP-OCRv5-mobile-bounded-spike",
        "scope": "printed_text_line_extraction_only",
        "privacy_filter_contract": "text_only_normalized_input",
        "fixture_count": 2,
        "ready_fixture_count": 2,
        "all_fixtures_ready_for_text_pii_eval": True,
        "total_char_count": 219,
        "non_goals": [
            "complete_ocr_pipeline",
            "final_pdf_rewrite_export",
            "full_page_detection_or_segmentation",
            "handwriting_recognition",
            "visual_redaction",
        ],
    }
    assert "fixtures" in report_text
```

- [ ] **Step 2: Run the target test to verify RED**

Run: `python -m pytest tests/test_ocr_handoff_corpus.py::test_corpus_summary_output_contains_only_downstream_readiness_fields -q`

Expected: FAIL because `run_ocr_handoff_corpus.py` does not accept `--summary-output` yet.

- [ ] **Step 3: Write the failing stale-summary cleanup test**

Add this test after `test_missing_fixture_dir_fails_without_leaving_report` in `tests/test_ocr_handoff_corpus.py`:

```python
def test_missing_fixture_dir_fails_without_leaving_summary_output(tmp_path):
    output = tmp_path / "report.json"
    summary_output = tmp_path / "summary.json"
    output.write_text("stale report Jane Example", encoding="utf-8")
    summary_output.write_text("stale summary MRN-12345", encoding="utf-8")

    result = run_corpus(
        "--fixture-dir",
        tmp_path / "missing",
        "--output",
        output,
        "--summary-output",
        summary_output,
    )

    assert result.returncode != 0
    assert not output.exists()
    assert not summary_output.exists()
```

- [ ] **Step 4: Run the cleanup test to verify RED**

Run: `python -m pytest tests/test_ocr_handoff_corpus.py::test_missing_fixture_dir_fails_without_leaving_summary_output -q`

Expected: FAIL because `run_ocr_handoff_corpus.py` does not accept `--summary-output` yet.

- [ ] **Step 5: Implement the minimal summary-output support**

In `scripts/ocr_eval/run_ocr_handoff_corpus.py`, replace the existing `fail`, `build_report`, `parse_args`, and `main` functions with this code, and add the two helper functions shown here above `build_report`:

```python
def fail(output: Path, message: str, summary_output: Path | None = None) -> int:
    remove_stale_output(output)
    if summary_output is not None:
        remove_stale_output(summary_output)
    print(message, file=sys.stderr)
    return 1


def build_readiness_summary(report: dict) -> dict:
    return {
        "artifact": "ocr_handoff_corpus_readiness_summary",
        "candidate": "PP-OCRv5_mobile_rec",
        "engine": report["engine"],
        "scope": report["scope"],
        "privacy_filter_contract": report["privacy_filter_contract"],
        "fixture_count": report["fixture_count"],
        "ready_fixture_count": report["ready_fixture_count"],
        "all_fixtures_ready_for_text_pii_eval": report["fixture_count"] == report["ready_fixture_count"],
        "total_char_count": report["total_char_count"],
        "non_goals": report["non_goals"],
    }


def write_json_file(path: Path, payload: dict) -> bool:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return True


def build_report(fixture_dir: Path, output: Path, summary_output: Path | None = None) -> int:
    remove_stale_output(output)
    if summary_output is not None:
        remove_stale_output(summary_output)

    try:
        fixture_dir_exists = fixture_dir.is_dir()
    except OSError:
        fixture_dir_exists = False
    if not fixture_dir_exists:
        return fail(output, "fixture dir does not exist or is not a directory", summary_output)

    try:
        fixture_paths = sorted(fixture_dir.glob("*.txt"))
    except OSError:
        return fail(output, "fixture dir could not be read", summary_output)
    if not fixture_paths:
        return fail(output, "fixture dir contains no .txt fixtures", summary_output)
    if len(fixture_paths) > MAX_FIXTURE_COUNT:
        return fail(output, "fixture corpus exceeds maximum fixture count", summary_output)

    fixtures = []
    total_char_count = 0
    for index, fixture_path in enumerate(fixture_paths, start=1):
        text, error_code = read_fixture_text(fixture_path, index, output)
        if error_code is not None:
            if summary_output is not None:
                remove_stale_output(summary_output)
            return error_code
        assert text is not None
        normalized = normalize_whitespace(text)
        if not normalized:
            return fail(output, f"{fixture_id(index)} is empty after whitespace normalization", summary_output)
        char_count = len(normalized)
        total_char_count += char_count
        fixtures.append(
            {
                "id": fixture_id(index),
                "char_count": char_count,
                "ready_for_text_pii_eval": True,
            }
        )

    report = {
        "engine": ENGINE,
        "scope": SCOPE,
        "fixture_count": len(fixtures),
        "ready_fixture_count": sum(1 for fixture in fixtures if fixture["ready_for_text_pii_eval"]),
        "total_char_count": total_char_count,
        "fixtures": fixtures,
        "non_goals": NON_GOALS,
        "privacy_filter_contract": PRIVACY_FILTER_CONTRACT,
    }

    try:
        write_json_file(output, report)
        if summary_output is not None:
            write_json_file(summary_output, build_readiness_summary(report))
    except OSError:
        remove_stale_output(summary_output) if summary_output is not None else None
        return fail(output, "report output could not be written", summary_output)
    return 0


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixture-dir", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--summary-output", type=Path)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    return build_report(args.fixture_dir, args.output, args.summary_output)
```

- [ ] **Step 6: Run the target tests to verify GREEN**

Run: `python -m pytest tests/test_ocr_handoff_corpus.py -q`

Expected: PASS with all OCR handoff corpus tests passing.

- [ ] **Step 7: Run static syntax verification**

Run: `python -m py_compile scripts/ocr_eval/run_ocr_handoff_corpus.py tests/test_ocr_handoff_corpus.py`

Expected: no output and exit code 0.

- [ ] **Step 8: Commit**

```bash
git add scripts/ocr_eval/run_ocr_handoff_corpus.py tests/test_ocr_handoff_corpus.py
git commit -m "feat(ocr): add corpus readiness summary"
```

### Task 2: Truth-sync README completion and evidence

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Run verification commands before editing README**

Run:

```bash
python -m pytest tests/test_ocr_handoff_corpus.py -q
python scripts/ocr_eval/run_ocr_handoff_corpus.py --fixture-dir scripts/ocr_eval/fixtures/corpus --output /tmp/ocr-handoff-corpus.json --summary-output /tmp/ocr-handoff-corpus-summary.json
python -m json.tool /tmp/ocr-handoff-corpus-summary.json >/tmp/ocr-handoff-corpus-summary.pretty.json
```

Expected: pytest passes, runner exits 0, and `json.tool` exits 0.

- [ ] **Step 2: Update README completion snapshot and OCR corpus evidence**

In `README.md`, update the completion snapshot date/evidence paragraph to say the PP-OCRv5 mobile OCR handoff corpus runner now writes an optional PHI-safe `--summary-output` readiness artifact. Keep CLI at 95%, Browser/Web at 99%, Desktop app at 99%, and Overall at 97% unless the controller-visible worktree contains another landed capability that justifies a new rubric re-baseline. State that this is CLI/runtime OCR-to-text-PII readiness evidence only and does not add OCR execution to Browser/Web or Desktop.

- [ ] **Step 3: Run README evidence search**

Run: `python - <<'PY'\nfrom pathlib import Path\ntext=Path('README.md').read_text()\nfor required in ['--summary-output', 'ocr_handoff_corpus_readiness_summary', 'CLI | 95%', 'Browser/Web | 99%', 'Desktop app | 99%', 'Overall | 97%']:\n    assert required in text, required\nprint('README OCR readiness summary evidence present')\nPY`

Expected: prints `README OCR readiness summary evidence present`.

- [ ] **Step 4: Commit**

```bash
git add README.md
git commit -m "docs: truth-sync OCR readiness summary"
```

## Self-Review

- Spec coverage: The plan implements a bounded PP-OCRv5 mobile synthetic OCR handoff readiness summary and README truth-sync. It does not add Browser/Web or Desktop execution and does not claim visual redaction, image pixel redaction, handwritten OCR, or PDF rewrite/export.
- Placeholder scan: No TBD/TODO/implement later placeholders are present.
- Type consistency: `--summary-output`, `build_readiness_summary`, `ocr_handoff_corpus_readiness_summary`, and the expected JSON keys are named consistently across tests, runner, and README instructions.
