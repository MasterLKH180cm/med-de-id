# Privacy Filter Synthetic Corpus Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the bounded CLI-first Privacy Filter text-only POC from a single synthetic text file into a small synthetic corpus runner with aggregate, PHI-safe evidence.

**Architecture:** Add a Python corpus runner that invokes the existing `run_privacy_filter.py --mock` text-only detector over `.txt` fixtures, validates each normalized output with the existing validator, and writes one aggregate JSON report containing counts and category coverage without raw PHI. This remains CLI/runtime-only; it does not add OCR, visual redaction, PDF rewrite/export, browser/desktop UI, or agent/controller semantics.

**Tech Stack:** Python stdlib scripts under `scripts/privacy_filter`, pytest, existing Privacy Filter JSON contract and validator.

---

## File Structure

- Create: `scripts/privacy_filter/run_synthetic_corpus.py` — corpus runner for `.txt` fixtures using the existing text-only Privacy Filter runner and validator.
- Create: `scripts/privacy_filter/fixtures/corpus/contact_card.txt` — synthetic contact-card PHI fixture.
- Create: `scripts/privacy_filter/fixtures/corpus/clinic_note.txt` — synthetic clinic-note PHI fixture.
- Modify: `scripts/privacy_filter/test_run_privacy_filter.py` — TDD tests for corpus aggregate output and PHI-safe report behavior.
- Modify: `scripts/privacy_filter/README.md` — usage and non-goals for corpus runner.
- Modify: `README.md` — truth-sync completion evidence without inflating Browser/Web/Desktop.

### Task 1: Add Privacy Filter synthetic corpus runner

**Files:**
- Create: `scripts/privacy_filter/run_synthetic_corpus.py`
- Create: `scripts/privacy_filter/fixtures/corpus/contact_card.txt`
- Create: `scripts/privacy_filter/fixtures/corpus/clinic_note.txt`
- Modify: `scripts/privacy_filter/test_run_privacy_filter.py`

- [ ] **Step 1: Write failing tests**

Append tests that create/read a corpus directory, invoke `run_synthetic_corpus.py`, and assert the aggregate report contains `engine: fallback_synthetic_patterns`, `fixture_count: 2`, category coverage for `NAME`, `MRN`, `EMAIL`, and `PHONE`, no raw fixture PHI (`Jane Example`, `MRN-12345`, `jane@example.test`, `555-111-2222`), and per-fixture entries with `detected_span_count` only.

- [ ] **Step 2: Run tests to verify RED**

Run: `python -m pytest scripts/privacy_filter/test_run_privacy_filter.py -q`
Expected: FAIL because `run_synthetic_corpus.py` does not exist yet.

- [ ] **Step 3: Implement minimal runner and fixtures**

Create the corpus fixtures with synthetic-only PHI. Implement `run_synthetic_corpus.py` with args `--fixture-dir`, `--output`, optional `--python-command`, and optional `--runner-path`. For each `.txt`, invoke existing `run_privacy_filter.py --mock`, parse JSON, aggregate category counts, and write a PHI-safe JSON report:

```json
{
  "engine": "fallback_synthetic_patterns",
  "scope": "text_only_synthetic_corpus",
  "fixture_count": 2,
  "total_detected_span_count": 7,
  "category_counts": {"EMAIL": 1, "MRN": 2, "NAME": 2, "PHONE": 2},
  "fixtures": [
    {"fixture": "clinic_note.txt", "detected_span_count": 3, "category_counts": {"MRN": 1, "NAME": 1, "PHONE": 1}},
    {"fixture": "contact_card.txt", "detected_span_count": 4, "category_counts": {"EMAIL": 1, "MRN": 1, "NAME": 1, "PHONE": 1}}
  ],
  "non_goals": ["ocr", "visual_redaction", "image_pixel_redaction", "final_pdf_rewrite_export", "browser_ui", "desktop_ui"]
}
```

The report must never include raw input text or masked text.

- [ ] **Step 4: Run tests to verify GREEN**

Run: `python -m pytest scripts/privacy_filter/test_run_privacy_filter.py -q`
Expected: PASS.

- [ ] **Step 5: Run direct CLI/runtime verification**

Run: `python scripts/privacy_filter/run_synthetic_corpus.py --fixture-dir scripts/privacy_filter/fixtures/corpus --output /tmp/privacy-filter-corpus.json && python -m json.tool /tmp/privacy-filter-corpus.json >/tmp/privacy-filter-corpus.pretty.json && ! grep -E 'Jane Example|MRN-12345|jane@example.test|555-111-2222' /tmp/privacy-filter-corpus.json`
Expected: all commands exit 0.

- [ ] **Step 6: Commit**

Run: `git add scripts/privacy_filter && git commit -m "feat(cli): add privacy filter synthetic corpus runner"`

### Task 2: Truth-sync docs and README completion evidence

**Files:**
- Modify: `scripts/privacy_filter/README.md`
- Modify: `README.md`

- [ ] **Step 1: Write failing docs check**

Run a Python check requiring `synthetic corpus`, `text-only`, `not OCR`, `not visual redaction`, and `run_synthetic_corpus.py` in docs.
Expected: FAIL until docs are updated.

- [ ] **Step 2: Update docs**

Document the corpus command, fixture scope, PHI-safe aggregate report, and non-goals. Update README completion evidence to mention the corpus runner. Keep Browser/Web and Desktop unchanged unless landed capability supports movement.

- [ ] **Step 3: Run docs check and verification**

Run the docs check, `python -m pytest scripts/privacy_filter/test_run_privacy_filter.py -q`, `git diff --check`.
Expected: PASS.

- [ ] **Step 4: Commit**

Run: `git add README.md scripts/privacy_filter/README.md && git commit -m "docs: truth-sync privacy filter corpus evidence"`
