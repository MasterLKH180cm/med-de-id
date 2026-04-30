# OpenAI Privacy Filter Evaluation for `med-de-id`

## Recommendation

**Recommend as a strong bounded candidate for text-only PII detection/masking evaluation.**

This is a good fit for local-first evaluation in `med-de-id` because it is Apache-2.0 licensed, explicitly intended for on-prem data sanitization workflows, and focused on token-classification for text PII spans rather than general generation.

## Sources checked

- iThome summary: `https://www.ithome.com.tw/news/175246`
- Upstream repo README: `https://github.com/openai/privacy-filter`
- Raw README used for local review: `https://raw.githubusercontent.com/openai/privacy-filter/main/README.md`
- Hugging Face model card/API listing: `https://huggingface.co/openai/privacy-filter`

## Grounded upstream facts

The following points are grounded in the upstream README/model metadata reviewed during this evaluation:

- Project: `openai/privacy-filter`
- License: Apache-2.0
- Type: bidirectional token-classification model for PII detection and masking in text
- Positioning: local/on-prem high-throughput sanitization workflows
- Runtime shape: upstream README states it can run in a web browser or on a laptop
- Operational knobs: upstream README describes operating points for precision/recall and detected span length behavior
- Supporting tooling: upstream README documents local CLI (`opf`), eval flow, and fine-tuning flow

## Why it fits `med-de-id`

### 1. Product alignment
`med-de-id` is a de-identification tool. Privacy Filter directly aligns with text-based de-identification because it detects and masks sensitive spans in text without requiring a generalized agent/controller platform.

### 2. Local-first alignment
The upstream positioning explicitly matches local/on-prem use. That is compatible with this repo's local-first runtime, CLI, browser, and desktop direction.

### 3. Integration seam clarity
Privacy Filter is easiest to introduce in **text-only surfaces first**, where the repo already has bounded workflows and reporting concepts.

## Best integration seams

### Best first seam: CLI
A bounded `mdid-cli` text evaluation command or helper is the cleanest first spike because it:
- avoids browser/desktop UI churn
- minimizes cross-layer blast radius
- produces easy-to-review synthetic evidence
- supports local fixture-based regression checks

### Second seam: runtime
A bounded localhost runtime endpoint for synthetic text-only sanitization would make sense only after the CLI spike proves useful.

### Later seams: browser/desktop
Browser and desktop should come later, only after the text-only path proves useful and output/report semantics are stable.

## Likely `med-de-id` integration mapping

### `mdid-domain`
Could eventually define neutral shared types for:
- text input scope metadata
- detected PII span summaries
- masked text result summaries
- confidence or policy operating-point metadata if needed

### `mdid-adapters`
Could host the boundary that turns local text into candidate spans/masked text via a bounded sidecar or wrapper.

### `mdid-application`
Could orchestrate request/response shape and domain-safe disclosures while keeping the model-specific wiring out of surface code.

### `mdid-runtime`
Could expose a bounded text-only localhost endpoint later if the CLI spike proves stable.

### `mdid-cli`
Best first adoption point. A small spike can validate:
- local invocation
- summary output shape
- masked result structure
- fixture-driven regression

## What it does NOT solve

Privacy Filter must not be misrepresented as solving:
- OCR
- visual redaction
- image/pixel redaction
- handwriting recognition
- final PDF rewrite/export
- scanned-document extraction by itself

Its role is **text-span classification and masking only**.

## Minimum useful prototype

### Proposed bounded spike
Create a CLI-first synthetic-text prototype that:
1. accepts a local synthetic text fixture
2. runs it through a local Privacy Filter evaluation path
3. emits:
   - masked text
   - detected span summary
   - category counts
   - span records with at least `label`, `start`, `end`, and a redacted or fixture-safe preview field
4. avoids echoing raw sensitive text in logs/reports unless explicitly required by safe fixture setup

### Minimum expected output contract

A minimal useful output shape for the bounded spike is:

```json
{
  "summary": {
    "input_char_count": 0,
    "detected_span_count": 0,
    "category_counts": {}
  },
  "masked_text": "<redacted-or-masked-text>",
  "spans": [
    {
      "label": "EMAIL",
      "start": 0,
      "end": 0,
      "preview": "<redacted>"
    }
  ]
}
```

This is not a final product contract; it is the smallest report-friendly shape that would let `mdid-cli` or a bounded runtime prototype prove usefulness.

### Why CLI first
CLI first gives the fastest honest answer about:
- install complexity
- runtime cost
- output usefulness
- reporting fit
without prematurely widening browser/desktop/runtime responsibilities.

## Risks

1. **Operational complexity risk**
   A Python/transformers sidecar may complicate a Rust-first repo if adopted too early.

2. **False-scope risk**
   Teams may over-read "privacy filter" as solving OCR/PDF redaction. This must be aggressively prevented in docs and completion language.

3. **Output taxonomy mismatch risk**
   The model's categories may not line up perfectly with medical-domain review/report expectations, requiring an explicit mapping layer.

## Recommended next step

Proceed with a bounded **CLI-first Privacy Filter spike plan** using synthetic fixtures only.

If the spike shows:
- reproducible local install
- usable text masking output
- report-friendly span summaries

then promote the next step to a runtime-side bounded evaluation surface.

## Current verdict

- **Adopt for bounded evaluation:** YES
- **Adopt immediately into browser/desktop UX:** NO
- **Treat as OCR/visual-redaction solution:** NO
