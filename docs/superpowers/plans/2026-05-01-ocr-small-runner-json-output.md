# OCR Small Runner JSON Output Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded JSON output mode to the PP-OCRv5 mobile small OCR runner so its extraction evidence can be validated and fed into downstream text-only Privacy Filter evaluation without relying on ad hoc stdout text parsing.

**Architecture:** Extend `scripts/ocr_eval/run_small_ocr.py` with an optional `--json` flag that emits a strict local-only OCR extraction object for both mock and real execution paths. Add tests that verify the JSON contract is PHI-safe in metadata, remains printed-text-line-only, and preserves the existing text stdout mode for compatibility.

**Tech Stack:** Python 3 standard library, pytest, existing `scripts/ocr_eval` synthetic fixtures, existing Privacy Filter Python runner.

---

## File Structure

- Modify: `scripts/ocr_eval/run_small_ocr.py` — add `--json`, output construction, and deterministic status metadata.
- Modify: `tests/test_ocr_handoff_contract.py` — add RED/GREEN coverage for JSON mode and downstream Privacy Filter compatibility.
- Modify: `scripts/ocr_eval/README.md` — document JSON mode as OCR extraction evidence only, not visual redaction or PDF export.
- Modify: `README.md` — truth-sync the completion/evidence snapshot after the landed CLI/runtime OCR extraction improvement.

### Task 1: Add JSON output mode to the bounded small OCR runner

**Files:**
- Modify: `tests/test_ocr_handoff_contract.py`
- Modify: `scripts/ocr_eval/run_small_ocr.py`
- Modify: `scripts/ocr_eval/README.md`

- [x] **Step 1: Write the failing test**

Add this test to `tests/test_ocr_handoff_contract.py` after `test_ppocrv5_mobile_bounded_spike_handoff_metadata_and_privacy_filter_text_contract`:

```python
def test_small_ocr_json_mode_emits_bounded_extraction_contract_and_feeds_privacy_filter(tmp_path):
    ocr = run(RUNNER, "--mock", "--json", FIXTURE_IMAGE)

    assert ocr.returncode == 0, ocr.stderr
    payload = json.loads(ocr.stdout)
    assert payload == {
        "candidate": "PP-OCRv5_mobile_rec",
        "engine": "PP-OCRv5-mobile-bounded-spike",
        "engine_status": "deterministic_synthetic_fixture_fallback",
        "scope": "printed_text_line_extraction_only",
        "source": "synthetic_printed_phi_line.png",
        "extracted_text": EXPECTED_TEXT.read_text(encoding="utf-8"),
        "normalized_text": " ".join(EXPECTED_TEXT.read_text(encoding="utf-8").split()),
        "ready_for_text_pii_eval": True,
        "privacy_filter_contract": "text_only_normalized_input",
        "non_goals": sorted(REQUIRED_NON_GOALS),
    }
    assert "visual_redaction" in payload["non_goals"]
    assert "final_pdf_rewrite_export" in payload["non_goals"]

    privacy_input = tmp_path / "privacy-input.txt"
    privacy_input.write_text(payload["normalized_text"], encoding="utf-8")
    pii = run(PRIVACY_FILTER, "--mock", privacy_input)

    assert pii.returncode == 0, pii.stderr
    pii_obj = json.loads(pii.stdout)
    assert pii_obj["summary"]["detected_span_count"] >= 1
    assert pii_obj["metadata"]["network_api_called"] is False
```

- [x] **Step 2: Run test to verify it fails**

Run: `pytest tests/test_ocr_handoff_contract.py::test_small_ocr_json_mode_emits_bounded_extraction_contract_and_feeds_privacy_filter -q`

Expected: FAIL because `run_small_ocr.py` does not accept `--json` yet.

- [x] **Step 3: Write minimal implementation**

In `scripts/ocr_eval/run_small_ocr.py`:

1. Import `json`.
2. Add parser flag:

```python
parser.add_argument(
    "--json",
    action="store_true",
    help="Emit bounded OCR extraction JSON instead of raw extracted text",
)
```

3. Add constants and helper:

```python
ENGINE = "PP-OCRv5-mobile-bounded-spike"
SCOPE = "printed_text_line_extraction_only"
PRIVACY_FILTER_CONTRACT = "text_only_normalized_input"
REQUIRED_NON_GOALS = {
    "visual_redaction",
    "final_pdf_rewrite_export",
    "handwriting_recognition",
    "full_page_detection_or_segmentation",
    "complete_ocr_pipeline",
}


def normalize_text(text: str) -> str:
    return " ".join(text.split())


def build_output_payload(input_path: Path, extracted_text: str, engine_status: str) -> dict:
    normalized_text = normalize_text(extracted_text)
    return {
        "candidate": CANDIDATE_RECOGNIZER,
        "engine": ENGINE,
        "engine_status": engine_status,
        "scope": SCOPE,
        "source": input_path.name,
        "extracted_text": extracted_text,
        "normalized_text": normalized_text,
        "ready_for_text_pii_eval": bool(normalized_text),
        "privacy_filter_contract": PRIVACY_FILTER_CONTRACT,
        "non_goals": sorted(REQUIRED_NON_GOALS),
    }
```

4. Refactor `run_mock` and `run_real` to return `(return_code, extracted_text, engine_status)` internally, and make `main()` either print raw text or `json.dumps(payload, indent=2, sort_keys=True) + "\n"`.

- [x] **Step 4: Run test to verify it passes**

Run: `pytest tests/test_ocr_handoff_contract.py::test_small_ocr_json_mode_emits_bounded_extraction_contract_and_feeds_privacy_filter -q`

Expected: PASS.

- [x] **Step 5: Run broader tests**

Run: `pytest tests/test_ocr_handoff_contract.py -q`

Expected: PASS.

Run: `python -m py_compile scripts/ocr_eval/run_small_ocr.py`

Expected: PASS.

Run: `git diff --check`

Expected: PASS.

- [x] **Step 6: Update local OCR README**

Add a JSON mode example to `scripts/ocr_eval/README.md`:

```markdown
### Bounded JSON extraction mode

The small OCR runner also supports a local JSON output mode for PP-OCRv5 mobile spike plumbing:

```bash
python scripts/ocr_eval/run_small_ocr.py --mock --json scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png
```

This emits a bounded printed-text-line extraction object with `candidate: PP-OCRv5_mobile_rec`, `scope: printed_text_line_extraction_only`, normalized text for downstream text-only Privacy Filter evaluation, and explicit non-goals. It is not visual redaction, image pixel redaction, handwriting recognition, full OCR quality proof, browser/desktop OCR execution, or final PDF rewrite/export.
```

- [x] **Step 7: Commit**

```bash
git add tests/test_ocr_handoff_contract.py scripts/ocr_eval/run_small_ocr.py scripts/ocr_eval/README.md
git commit -m "feat(ocr): add bounded small runner json output"
```

### Task 2: README truth-sync and verification evidence

**Files:**
- Modify: `README.md`

- [x] **Step 1: Update README completion evidence**

Update the current repository status text to mention that the PP-OCRv5 mobile small OCR runner now has a bounded JSON output mode for printed-text-line extraction evidence and downstream text-only Privacy Filter input. Keep completion percentages honest: CLI remains 95%, Browser/Web remains 99%, Desktop remains 99%, Overall remains 97% unless repository-visible landed facts justify a new integer change.

- [x] **Step 2: Verify README and tests**

Run: `pytest tests/test_ocr_handoff_contract.py -q`

Expected: PASS.

Run: `python -m py_compile scripts/ocr_eval/run_small_ocr.py`

Expected: PASS.

Run: `git diff --check`

Expected: PASS.

- [x] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: truth-sync ocr json output evidence"
```

## Self-Review

- Spec coverage: Task 1 implements the CLI/runtime JSON output mode for PP-OCRv5 mobile printed-text extraction and verifies downstream Privacy Filter compatibility; Task 2 truth-syncs README completion evidence.
- Placeholder scan: no TBD/TODO/fill-in placeholders are present.
- Type consistency: `candidate`, `engine`, `engine_status`, `scope`, `source`, `extracted_text`, `normalized_text`, `ready_for_text_pii_eval`, `privacy_filter_contract`, and `non_goals` match the existing OCR handoff contract names.
