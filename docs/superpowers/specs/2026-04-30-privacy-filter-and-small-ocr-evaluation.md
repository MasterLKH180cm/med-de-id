# Privacy Filter and Small OCR Evaluation Spec

## Goal

Evaluate whether OpenAI Privacy Filter can serve as a bounded local text-PII detection/masking engine inside `med-de-id`, and whether a small OCR model can serve as a bounded local OCR candidate for printed-document text extraction that feeds downstream de-identification workflows.

## Why this evaluation exists

`med-de-id` already aims at bounded local flows for CSV/XLSX/DICOM/PDF review, conservative media metadata review, vault/decode/audit, and portable artifact handling, as reflected in the current repo docs and surface code. The repo still has real gaps in:

- robust text-span PII detection beyond existing bounded rules/flows
- OCR for scanned/bitmap-heavy documents
- cross-surface browser/desktop workflow depth for text extraction + review

This evaluation exists to identify realistic local-first model candidates without pretending that model availability alone solves OCR, visual redaction, or final PDF rewrite/export.

## Candidates in scope

### Candidate A — OpenAI Privacy Filter
- Source: `openai/privacy-filter`
- License: Apache-2.0
- Category: token-classification model for text PII detection/masking
- Intended role: text-only PII span detection and masking candidate for CLI/runtime-first evaluation

### Candidate B — Small OCR candidate
- Primary candidate: `PaddlePaddle/PP-OCRv5_mobile_rec`
- Comparison baseline: `microsoft/trocr-small-printed`
- Intended role: bounded local OCR candidate for extracting printed text from synthetic scanned-style document fixtures so extracted text can flow into downstream text review or PII detection

## Hard constraints

1. Local-first only.
2. No SaaS/network dependency in the eventual product path.
3. No claim that Privacy Filter solves OCR, visual redaction, image redaction, handwriting recognition, or final PDF rewrite/export.
4. No claim that the OCR candidate solves visual redaction, final PDF rewrite/export, or full browser/desktop UX depth.
5. Evaluation must preserve `med-de-id` identity as a de-identification tool, not an AI platform or orchestration product.
6. Initial integration target must stay bounded: CLI/runtime first, browser/desktop later only if justified by evidence.
7. Any evaluation or prototype data must be synthetic, sanitized, or already-safe fixtures only.

## Success criteria

### Privacy Filter success criteria
1. The model can be installed and run locally with reproducible steps.
2. It can detect and mask representative PII spans in synthetic text.
3. It has a credible integration seam for `mdid-cli` and/or `mdid-runtime` text flows.
4. It can produce outputs that map cleanly to `med-de-id` review/report concepts such as summary counts, detected spans, and redacted output.
5. The evaluation can describe operating-point tradeoffs (precision/recall / masking strictness) in a way usable by local workflows.

### Small OCR success criteria
1. The candidate can run locally on CPU with bounded setup complexity.
2. It can extract printed text from representative synthetic document images or rendered PDF pages.
3. The extracted text is good enough to feed downstream text PII detection evaluation.
4. The candidate fits a bounded workstation/runtime OCR spike without requiring a broad platform rewrite.

## Rejection criteria

### Privacy Filter rejection criteria
- Requires non-local serving for practical use.
- Integration would force a broad architecture rewrite before any bounded value appears.
- Output structure is too brittle to map into `med-de-id` summaries/reports.
- Model size/runtime characteristics are not credible for local-first bounded use.

### Small OCR rejection criteria
- Setup/runtime burden is too high for a "small model" local-first spike.
- Extracted text quality is too poor even on synthetic printed-text fixtures.
- Candidate pushes the team into heavy document platform work before bounded value is proven.

## Scope boundaries

### In scope
- Text-only PII detection/masking evaluation
- OCR candidate selection for printed text extraction
- CLI/runtime-first prototype planning
- README truth-sync only after evidence actually lands

### Out of scope
- Full OCR production rollout
- Visual redaction implementation
- Image pixel redaction
- Handwriting OCR
- Final PDF rewrite/export
- Broad browser/desktop product claims beyond bounded prototype evidence

## Preferred first prototype shape

1. Privacy Filter CLI or runtime text-only spike
2. OCR synthetic fixture extraction spike
3. Optional pipeline handoff: OCR output text -> Privacy Filter evaluation path

## Deliverables required by this spec

1. Technical evaluation note for OpenAI Privacy Filter
2. Technical evaluation note for the chosen small OCR candidate with at least one comparison model
3. Bounded spike plans for:
   - text-only Privacy Filter integration
   - small OCR extraction evaluation
4. README truth-sync only if and when evidence actually lands
