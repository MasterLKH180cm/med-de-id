# Small OCR Spike Implementation Plan

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** Build a bounded local OCR extraction spike using `PaddlePaddle/PP-OCRv5_mobile_rec` as the primary candidate to determine whether synthetic printed text-line images can be recognized locally on CPU and handed to downstream text PII detection evaluation.

**Architecture:** Keep this spike narrow and synthetic-fixture-only. Because `PP-OCRv5_mobile_rec` is being treated here as a recognizer-first small candidate, the first honest spike must use pre-cropped synthetic printed text-line images rather than pretending the model alone performs full page detection/segmentation. Do not attempt full PDF rewrite/export, visual redaction, or browser/desktop UX rollout. Evaluate OCR as a text-extraction stage only, then explicitly test whether extracted text is good enough to pass into the Privacy Filter text pipeline.

**Tech Stack:** local Python helper scripts, PaddleOCR small/mobile candidate, synthetic printed-text fixtures, optional comparison with `microsoft/trocr-small-printed` only if needed.

---

## File Structure

- Create: `docs/research/small-ocr-spike-results.md`
- Create: `scripts/ocr_eval/README.md`
- Create: `scripts/ocr_eval/run_small_ocr.py`
- Create: `scripts/ocr_eval/validate_small_ocr_output.py`
- Create: `scripts/ocr_eval/build_ocr_handoff.py`
- Create: `scripts/ocr_eval/validate_ocr_handoff.py`
- Create: `scripts/ocr_eval/fixtures/synthetic_printed_phi.txt`
- Create: `scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png`
- Create: `scripts/ocr_eval/fixtures/synthetic_printed_phi_expected.txt`
- Create: `scripts/ocr_eval/fixtures/ocr_handoff_expected_shape.json`
- Optional create later: `scripts/ocr_eval/compare_trocr_small.py`
- Optional later create: `scripts/ocr_eval/run_page_ocr_with_detector.py` only if a separate detector stage is explicitly added

## Task 1: Build synthetic fixtures
- [ ] Create a small synthetic printed-text source fixture in `scripts/ocr_eval/fixtures/synthetic_printed_phi.txt`
- [ ] Create a pre-cropped synthetic printed text-line image in `scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png`
- [ ] Create expected extracted text reference in `scripts/ocr_eval/fixtures/synthetic_printed_phi_expected.txt`
- [ ] Ensure fixture content is synthetic only
- [ ] Do not treat this first spike as page OCR; page-like evaluation is deferred unless a separate detector stage is explicitly added later.

## Task 2: Create bounded OCR runner
- [ ] Add `scripts/ocr_eval/run_small_ocr.py`
- [ ] Add `scripts/ocr_eval/validate_small_ocr_output.py`
- [ ] Runner must:
  - accept local line-image path
  - run the small OCR candidate locally
  - emit extracted text as plain UTF-8 stdout text for the first spike
- [ ] RED command before the runner exists:
```bash
python scripts/ocr_eval/run_small_ocr.py scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png
```
- [ ] Expected RED evidence: command fails because the runner does not exist yet.
- [ ] GREEN command after implementation:
```bash
python scripts/ocr_eval/run_small_ocr.py scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png > /tmp/small-ocr-output.txt
python scripts/ocr_eval/validate_small_ocr_output.py /tmp/small-ocr-output.txt scripts/ocr_eval/fixtures/synthetic_printed_phi_expected.txt
```
- [ ] Expected GREEN evidence: validator exits zero or records bounded mismatch metrics explicitly.

## Task 3: Measure extraction usefulness
- [ ] Compare extracted text to expected synthetic text
- [ ] Record whether token fidelity is sufficient for downstream PII detection
- [ ] Explicitly record where the OCR output is weak or lossy

## Task 4: Test downstream handoff readiness
- [ ] Define a tiny handoff contract in `scripts/ocr_eval/fixtures/ocr_handoff_expected_shape.json`
- [ ] Minimum handoff shape must include:
  - `source`: fixture name
  - `extracted_text`: OCR output string
  - `normalized_text`: optional whitespace-normalized string
  - `ready_for_text_pii_eval`: boolean
- [ ] Add a helper command that builds the handoff artifact explicitly:
```bash
python scripts/ocr_eval/run_small_ocr.py scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png > /tmp/small-ocr-output.txt
python scripts/ocr_eval/build_ocr_handoff.py \
  --source scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png \
  --input /tmp/small-ocr-output.txt \
  --output /tmp/ocr-handoff.json
python scripts/ocr_eval/validate_ocr_handoff.py /tmp/ocr-handoff.json
```
- [ ] Add `scripts/ocr_eval/build_ocr_handoff.py` to File Structure if this helper is implemented.
- [ ] Do not claim end-to-end medical PDF redaction is solved.
- [ ] Only prove extraction -> downstream text evaluation is feasible or not.

## Task 5: Record spike results
- [ ] Create `docs/research/small-ocr-spike-results.md`
- [ ] Add `scripts/ocr_eval/README.md` with exact bootstrap/install steps, exact run commands, exact validator commands, and any environment assumptions needed to reproduce the spike locally.
- [ ] Capture:
  - setup/runtime friction
  - CPU practicality
  - extraction quality on the synthetic line-image fixture
  - whether a separate detection/cropping stage is now the real blocker
  - whether TrOCR small comparison is needed
  - Go / No-Go / More-Evidence verdict

## Verification
- [ ] Run OCR locally on the synthetic printed text-line fixture
- [ ] Compare output to expected text with the validator script
- [ ] If handoff is tested, record the exact extracted text or normalized output shape fed into the text PII stage
- [ ] Keep all claims bounded to extraction only
- [ ] Only widen to page-like OCR in a later plan if an explicit detector/cropping stage is added and verified
