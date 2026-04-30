# Small OCR Candidate Evaluation for `med-de-id`

## Recommendation

**Primary small OCR candidate to evaluate first: `PaddlePaddle/PP-OCRv5_mobile_rec`**

Recommend evaluating this first as the bounded primary OCR candidate for local-first `med-de-id` workflows, pending synthetic-fixture validation.

## Sources checked

- PaddleOCR upstream README: `https://github.com/PaddlePaddle/PaddleOCR`
- Paddle Hugging Face listing: `https://huggingface.co/PaddlePaddle/PP-OCRv5_mobile_rec`
- TrOCR Hugging Face listing: `https://huggingface.co/microsoft/trocr-small-printed`
- Surya upstream README: `https://github.com/VikParuchuri/surya`

## Why this is the primary candidate

1. **Small/mobile positioning**
   The model is explicitly presented as a mobile-oriented OCR recognizer in the PaddleOCR ecosystem, which is a much better fit for the request to find a small, practical local OCR candidate.

2. **Local-first practicality**
   PaddleOCR is widely used for local OCR workflows and is more directly positioned for practical CPU/laptop usage than a larger, more research-style document stack.

3. **Bounded role fit**
   `med-de-id` does not need a giant document platform first. It needs a realistic OCR extraction candidate that can help turn printed scanned-like content into text for downstream review/detection.

## Comparison baseline: `microsoft/trocr-small-printed`

### What is good about TrOCR small
- compact compared to larger OCR transformers
- accessible through Hugging Face transformers ecosystem
- good baseline to compare against for printed text OCR

### Why it is not the primary candidate here
- the transformer image-to-text path is less obviously "small local utility first" than the PP-OCR mobile direction
- for a bounded workstation OCR spike, PaddleOCR mobile is the more pragmatic first candidate
- TrOCR small still remains a useful comparison point if PP-OCR quality or integration ergonomics disappoint

## Rejected as primary candidate: Surya

Surya looks powerful, but it is not the right first answer for this specific ask because:
- it is broader/heavier document OCR tooling
- licensing/profile complexity is less aligned with a narrow small-model-first evaluation
- it would increase the risk of scope drift too early

## Intended role inside `med-de-id`

The OCR candidate is only meant to do this first:
- extract printed text from synthetic image/PDF-page fixtures locally
- feed extracted text into downstream text review / text PII detection evaluation

This is a **bounded extraction role**, not a full document-redaction solution.

## What OCR does NOT solve here

This OCR candidate must not be misrepresented as solving:
- visual redaction
- image pixel redaction
- handwritten medical OCR at production quality
- final PDF rewrite/export
- end-to-end browser/desktop workflow completion

## Best first prototype seam

### Bounded OCR extraction spike
Use synthetic printed fixtures only.

Suggested spike shape:
1. one synthetic rendered PDF page or image with printed PHI-like text
2. run local OCR on CPU
3. capture extracted text
4. measure whether the text is good enough to feed into downstream text PII detection evaluation

### Why this seam is right
It gives a clean answer to:
- setup complexity
- OCR quality on realistic printed text
- CPU usability
- downstream compatibility with text-only PII detection

without claiming that the hard visual-redaction problem is solved.

## Adoption criteria

Adopt as the primary bounded OCR candidate if it proves:
- local CPU-viable
- installable with bounded friction
- usable extraction quality on synthetic printed fixtures
- easy enough to connect to a downstream text-only PII detection experiment

## Rejection criteria

Reject or demote if it proves:
- too painful to set up for a bounded spike
- too poor at extracting synthetic printed PHI-like text
- too tightly coupled to a larger OCR platform workflow to be useful in a narrow experiment

## Comparison summary

| Candidate | Model type | Likely runtime weight | Local CPU friendliness | Fit for bounded workstation OCR spike | Browser suitability | Why consider it | Why it is or is not primary |
|---|---|---|---|---|---|---|---|
| `PaddlePaddle/PP-OCRv5_mobile_rec` | mobile-oriented OCR recognizer | likely lighter than broader document OCR stacks; intended as a small/mobile recognizer | promising for local CPU-first evaluation, pending fixture validation | strong fit for a narrow printed-text extraction spike | not a first browser pick; better as local helper/evaluation engine | practical, small-model-first, CPU/local-first friendly direction | primary candidate to test first, but not yet proven until synthetic fixtures pass |
| `microsoft/trocr-small-printed` | transformer image-to-text OCR | compact compared to larger OCR transformers, but still a transformer OCR path | plausible, but less obviously pragmatic than the PP-OCR mobile path | reasonable comparison baseline for printed text OCR | possible later browser exploration, but not the first pragmatic choice | compact HF baseline and useful comparison | not primary because the PP-OCR mobile direction is a cleaner small/local-first first bet |
| Surya | broader OCR toolkit | broader/heavier document OCR stack | less aligned with a narrow small-model-first spike | weaker fit for the very first bounded spike because it widens evaluation scope | not suitable as the first browser-oriented bounded candidate | strong OCR ecosystem/tooling | not primary because it is too broad/heavy for this specific ask |

## Recommended next step

Proceed with a bounded **small OCR extraction spike plan** using `PP-OCRv5_mobile_rec` as primary candidate and `TrOCR small` as comparison baseline only if needed.

## Current verdict

- **Adopt for bounded evaluation:** YES
- **Claim OCR solves visual redaction:** NO
- **Claim OCR solves final PDF rewrite/export:** NO
