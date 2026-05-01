# OCR Small JSON Text Metrics Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add bounded aggregate text metrics to the PP-OCRv5 mobile small OCR JSON contract so downstream Privacy Filter readiness can be verified without inspecting raw OCR text.

**Architecture:** Keep the existing `run_small_ocr.py --json` contract as the single CLI/runtime source of OCR handoff truth. Add only PHI-safe aggregate counts (`line_count`, `normalized_char_count`) derived from extracted printed text; do not add OCR quality claims, visual redaction, browser/desktop execution, or PDF rewrite/export semantics.

**Tech Stack:** Python 3 standard library, pytest, existing synthetic PP-OCRv5 mobile fixture tests.

---

## File Structure

- Modify `scripts/ocr_eval/run_small_ocr.py`: extend `build_extraction_contract()` to include aggregate-only `line_count` and `normalized_char_count` fields.
- Modify `tests/test_ocr_runner_contract.py`: add RED/GREEN coverage proving JSON metrics are emitted for mock and local PaddleOCR adapter paths, and that source filenames remain redacted.
- Modify `README.md`: truth-sync completion/evidence after the landed SDD slice; do not change completion percentages unless controller-visible facts support it.

### Task 1: Add PHI-safe OCR JSON text metrics

**Files:**
- Modify: `tests/test_ocr_runner_contract.py`
- Modify: `scripts/ocr_eval/run_small_ocr.py`

- [ ] **Step 1: Write the failing test**

Append this test to `tests/test_ocr_runner_contract.py`:

```python
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python3 -m pytest tests/test_ocr_runner_contract.py::test_json_output_includes_phi_safe_text_metrics -q`

Expected: FAIL with `KeyError: 'line_count'` or `KeyError: 'normalized_char_count'` because the metrics do not exist yet.

- [ ] **Step 3: Write minimal implementation**

In `scripts/ocr_eval/run_small_ocr.py`, update `build_extraction_contract()` to derive and emit aggregate-only counts:

```python
def build_extraction_contract(extracted_text: str, engine_status: str) -> dict:
    normalized_text = normalize_text(extracted_text)
    lines = [line for line in extracted_text.splitlines() if line.strip()]
    return {
        "candidate": CANDIDATE_RECOGNIZER,
        "engine": ENGINE,
        "engine_status": engine_status,
        "scope": SCOPE,
        "source": REDACTED_SOURCE,
        "extracted_text": extracted_text,
        "normalized_text": normalized_text,
        "line_count": len(lines),
        "normalized_char_count": len(normalized_text),
        "ready_for_text_pii_eval": bool(normalized_text),
        "privacy_filter_contract": PRIVACY_FILTER_CONTRACT,
        "non_goals": NON_GOALS,
    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `python3 -m pytest tests/test_ocr_runner_contract.py::test_json_output_includes_phi_safe_text_metrics -q`

Expected: PASS.

- [ ] **Step 5: Run relevant regression tests**

Run: `python3 -m pytest tests/test_ocr_runner_contract.py -q`

Expected: all tests in `tests/test_ocr_runner_contract.py` PASS.

- [ ] **Step 6: Commit**

```bash
git add tests/test_ocr_runner_contract.py scripts/ocr_eval/run_small_ocr.py
git commit -m "feat(ocr): add small OCR text metrics"
```

### Task 2: README truth-sync for OCR metrics slice

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Verify controller-visible facts before editing README**

Run: `git log --oneline -3 && git status --short && python3 -m pytest tests/test_ocr_runner_contract.py -q`

Expected: latest commit includes `feat(ocr): add small OCR text metrics`, worktree is clean except planned README edits, and OCR runner contract tests PASS.

- [ ] **Step 2: Add README evidence paragraph without percentage inflation**

Add a new verification evidence paragraph near the top of `README.md` current repository status stating:

```markdown
Verification evidence for the `run_small_ocr.py --json` PHI-safe text-metrics slice landed on this branch: the bounded PP-OCRv5 mobile small OCR JSON contract now emits aggregate-only `line_count` and `normalized_char_count` fields derived from printed-text extraction output, while preserving redacted source metadata and downstream `text_only_normalized_input` readiness. Repository-visible verification passed: `python3 -m pytest tests/test_ocr_runner_contract.py -q`. This is CLI/runtime PP-OCRv5 mobile printed-text extraction readiness evidence only: not OCR model-quality proof, not visual redaction, not image pixel redaction, not handwriting recognition, not final PDF rewrite/export, not Browser/Web execution, not Desktop execution, and not workflow orchestration semantics. Completion remains CLI 99%, Browser/Web 99%, Desktop app 99%, Overall 99%; this round adds no new rubric item beyond the existing OCR readiness/handoff contract, so there is no completion re-baseline.
```

- [ ] **Step 3: Run README/diff verification**

Run: `git diff --check && python3 -m pytest tests/test_ocr_runner_contract.py -q`

Expected: both PASS.

- [ ] **Step 4: Commit README truth-sync**

```bash
git add README.md
git commit -m "docs: truth-sync OCR small text metrics"
```
