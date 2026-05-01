# OCR Runner Input Validation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Harden the bounded PP-OCRv5 mobile synthetic OCR runner so it rejects missing, directory, and non-file inputs before either mock or real OCR execution.

**Architecture:** Keep the OCR spike CLI/runtime-only and printed-text-extraction-only. Add a small shared `validate_input_path()` guard in `scripts/ocr_eval/run_small_ocr.py` that runs before mock or PaddleOCR paths, preserving existing fixture-backed mock behavior and honest dependency-missing behavior.

**Tech Stack:** Python 3 standard library, pytest, bounded OCR spike scripts under `scripts/ocr_eval/`.

---

## File Structure

- Modify `scripts/ocr_eval/run_small_ocr.py`
  - Add `validate_input_path(input_path: Path) -> None`.
  - Call it from `main()` before mock or real execution.
  - Emit PHI-safe error text for missing paths and directories.
- Modify `tests/test_ocr_runner_contract.py`
  - Add tests for missing image input and directory input.
- Modify `README.md`
  - Truth-sync verification evidence and completion arithmetic without claiming Browser/Desktop OCR execution.

### Task 1: Reject invalid OCR runner input paths

**Files:**
- Modify: `scripts/ocr_eval/run_small_ocr.py`
- Test: `tests/test_ocr_runner_contract.py`
- Modify: `README.md`

- [ ] **Step 1: Write failing tests**

Add these tests to `tests/test_ocr_runner_contract.py`:

```python
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
```

- [ ] **Step 2: Run tests to verify RED**

Run:

```bash
python -m pytest tests/test_ocr_runner_contract.py::test_ocr_runner_rejects_missing_input_path_without_phi_leak tests/test_ocr_runner_contract.py::test_ocr_runner_rejects_directory_input_without_phi_leak -q
```

Expected: FAIL because the current mock path falls through to fixture lookup for missing paths and directory inputs rather than validating the input path first.

- [ ] **Step 3: Implement minimal validation**

In `scripts/ocr_eval/run_small_ocr.py`, add:

```python
def validate_input_path(input_path: Path) -> None:
    if not input_path.exists():
        raise ValueError("OCR input path does not exist")
    if not input_path.is_file():
        raise ValueError("OCR input path must be a file")
```

Then update `main()` to validate before dispatch:

```python
def main(argv: Sequence[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    input_path = Path(args.input_path)
    try:
        validate_input_path(input_path)
    except ValueError as exc:
        print(str(exc), file=sys.stderr)
        return 2
    if args.mock:
        return run_mock(input_path)
    return run_real(input_path)
```

- [ ] **Step 4: Run targeted tests to verify GREEN**

Run:

```bash
python -m pytest tests/test_ocr_runner_contract.py::test_ocr_runner_rejects_missing_input_path_without_phi_leak tests/test_ocr_runner_contract.py::test_ocr_runner_rejects_directory_input_without_phi_leak -q
```

Expected: PASS.

- [ ] **Step 5: Run OCR runner contract tests**

Run:

```bash
python -m pytest tests/test_ocr_runner_contract.py -q
```

Expected: PASS.

- [ ] **Step 6: Run existing OCR handoff and bridge smoke tests**

Run:

```bash
python -m pytest tests/test_ocr_handoff_contract.py tests/test_ocr_handoff_corpus.py tests/test_ocr_to_privacy_filter_corpus.py -q
```

Expected: PASS.

- [ ] **Step 7: Truth-sync README completion evidence**

Update `README.md` current status/evidence to mention the new OCR runner input validation evidence. Completion should remain CLI 95%, Browser/Web 99%, Desktop app 99%, Overall 97% unless repository-visible rubric facts justify a conservative re-baseline.

- [ ] **Step 8: Commit**

Run:

```bash
git add scripts/ocr_eval/run_small_ocr.py tests/test_ocr_runner_contract.py README.md docs/superpowers/plans/2026-05-01-ocr-runner-input-validation.md
git commit -m "fix(ocr): validate small ocr runner inputs"
```
