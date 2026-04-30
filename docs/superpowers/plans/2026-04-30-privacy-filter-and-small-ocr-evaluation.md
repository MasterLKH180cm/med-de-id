# Privacy Filter + Small OCR Candidate Evaluation Plan

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** Evaluate OpenAI Privacy Filter as a local text-PII detection/masking engine candidate for `med-de-id`, and evaluate one small OCR model candidate that is realistic for local-first browser/desktop workflows without pretending either solves visual redaction or final PDF rewrite/export.

**Architecture:** Keep the current Rust-first product boundaries intact. Treat OpenAI Privacy Filter as a bounded external text-classification engine candidate for text spans only, and treat small OCR as a separate bounded document-text extraction candidate. Do not broaden scope into generalized AI orchestration, multimodal agents, or fake end-to-end PDF redaction claims. Evaluation must produce clear adoption/no-adoption evidence, integration seams, and limitations.

**Tech Stack:** Rust workspace (`mdid-domain`, `mdid-adapters`, `mdid-application`, `mdid-runtime`, `mdid-cli`, `mdid-browser`, `mdid-desktop`), local Python sidecar/prototype only if needed for evaluation, OpenAI Privacy Filter, PaddleOCR small/mobile OCR candidate.

---

## Decision Summary To Validate

### Candidate 1: OpenAI Privacy Filter
- Source: `openai/privacy-filter`
- License: Apache-2.0
- Type: token-classification / text PII detection + masking
- Useful for: plain text, structured field text, extracted PDF text layer, runtime/CLI text sanitization pipeline
- Not useful for: OCR, visual redaction, image pixel redaction, handwriting recognition, full PDF rewrite/export

### Candidate 2: Small OCR model
- Primary candidate: `PaddlePaddle/PP-OCRv5_mobile_rec`
- Why primary: explicitly small/mobile-oriented OCR line recognizer, Apache-2.0 ecosystem, laptop/CPU-friendly positioning, realistic for local-first bounded OCR evaluation
- Secondary comparison baseline: `microsoft/trocr-small-printed`
- Not selected as primary: TrOCR small is promising but heavier transformer-style image-to-text flow is less obviously suited than PP-OCR mobile for a small bounded local-first OCR spike
- Explicitly out-of-scope for primary adoption decision: Surya, because it is broader/heavier document OCR tooling and license complexity is less aligned with this narrow “small model first” ask

---

## File Structure

- Create: `docs/superpowers/plans/2026-04-30-privacy-filter-and-small-ocr-evaluation.md`
- Create: `docs/superpowers/specs/2026-04-30-privacy-filter-and-small-ocr-evaluation.md`
- Modify: `README.md`
  - Only after the evaluation lands and only to truth-sync what was actually learned or integrated
- Optional create during implementation:
  - `docs/research/privacy-filter-eval.md`
  - `docs/research/small-ocr-eval.md`
  - `tools/privacy_filter/` or `scripts/privacy_filter/` for bounded local evaluation helpers
  - `tools/ocr_eval/` or `scripts/ocr_eval/` for bounded OCR evaluation helpers
- Possible future integration touch points if a prototype is accepted:
  - `crates/mdid-domain/src/lib.rs`
  - `crates/mdid-adapters/src/`
  - `crates/mdid-application/src/`
  - `crates/mdid-runtime/src/http.rs`
  - `crates/mdid-cli/src/main.rs`

---

## Task 1: Write evaluation spec with adoption criteria

**Objective:** Create a spec that defines exactly what “usable” means for Privacy Filter and the small OCR candidate inside `med-de-id`.

**Files:**
- Create: `docs/superpowers/specs/2026-04-30-privacy-filter-and-small-ocr-evaluation.md`

### Step 1: Write the spec document

The spec must include these sections:
- Problem statement
- Why the current repo needs text PII detection and OCR evaluation
- Candidate summary
- Hard constraints
- Success criteria
- Rejection criteria
- Scope boundaries

Required hard constraints:
- Local-first only
- No SaaS dependency in production flow
- No fake claim that Privacy Filter solves OCR/visual redaction
- No fake claim that small OCR solves visual redaction or PDF rewrite/export
- Must preserve current product identity as a de-identification tool

Required success criteria for Privacy Filter:
- Can run locally with reproducible install steps
- Can detect and mask text spans from representative synthetic samples
- Produces output that can be mapped to `med-de-id`-style summary/reporting concepts
- Has a credible integration seam for CLI/runtime first

Required success criteria for OCR candidate:
- Can run locally on CPU with bounded setup
- Can extract text from representative synthetic document images/PDF page renders
- Produces text that can flow into downstream text PII detection
- Does not require adopting a huge platform rewrite to be useful

### Step 2: Commit the spec

```bash
git add docs/superpowers/specs/2026-04-30-privacy-filter-and-small-ocr-evaluation.md
git commit -m "docs: add privacy filter and small ocr evaluation spec"
```

---

## Task 2: Privacy Filter technical evaluation note

**Objective:** Produce a concrete technical evaluation of OpenAI Privacy Filter for `med-de-id` text flows.

**Files:**
- Create: `docs/research/privacy-filter-eval.md`

### Step 1: Record the grounded facts

The note must explicitly capture:
- Apache-2.0 license
- On-prem/local-first suitability
- token-classification pipeline
- browser/laptop positioning from upstream docs
- eval/train/CLI availability

### Step 2: Map the candidate to current product seams

Write sections for:
- CLI usage seam
- runtime seam
- browser seam
- desktop seam
- PDF text-layer seam
- structured field seam

Must explicitly say:
- Best first integration target is CLI/runtime text path
- It is not an OCR or visual redaction engine
- It should be treated as a text-span classifier sidecar/prototype first

### Step 3: Define minimum prototype

Prototype target must be one of:
- CLI command that sends plain text to a local Privacy Filter sidecar and returns masked spans/summary
- runtime text-only endpoint for synthetic payload evaluation

Prefer the CLI-first prototype if both are equal.

### Step 4: Commit the note

```bash
git add docs/research/privacy-filter-eval.md
git commit -m "docs: evaluate privacy filter for text pii workflows"
```

---

## Task 3: Small OCR model evaluation note

**Objective:** Pick a small OCR model candidate honestly and document why it is the right bounded choice.

**Files:**
- Create: `docs/research/small-ocr-eval.md`

### Step 1: Evaluate the primary candidate

Primary candidate: `PaddlePaddle/PP-OCRv5_mobile_rec`

Record:
- why it counts as small/mobile-oriented
- local CPU viability
- license/ecosystem suitability
- expected role in a bounded OCR stage

### Step 2: Compare against at least one alternative

Compare against:
- `microsoft/trocr-small-printed`

The comparison table must include:
- model type
- likely runtime weight
- local CPU friendliness
- fit for bounded workstation OCR spike
- browser suitability
- why it is or is not the primary candidate

### Step 3: State strict boundaries

The note must explicitly say:
- OCR candidate extracts text; it does not perform visual redaction
- OCR candidate alone does not deliver final PDF rewrite/export
- OCR evaluation is about enabling downstream text review/detection, not pretending to finish the full PDF problem

### Step 4: Commit the note

```bash
git add docs/research/small-ocr-eval.md
git commit -m "docs: evaluate small ocr candidate for local workflows"
```

---

## Task 4: Build bounded Privacy Filter prototype plan

**Objective:** Define the smallest implementation spike worth doing next if Privacy Filter passes evaluation.

**Files:**
- Modify: `docs/research/privacy-filter-eval.md`
- Optional create: `docs/superpowers/plans/2026-04-30-privacy-filter-cli-spike.md`

### Step 1: Write failing acceptance tests on paper first

Document these prototype acceptance cases:
- plain text with name/email/phone returns redaction spans
- structured field text returns masked output without echoing raw sensitive content in logs/reports
- synthetic PDF text-layer sample can be passed through the same classifier path after extraction

### Step 2: Define exact prototype boundary

Prototype must be bounded to:
- synthetic or sanitized test fixtures only
- CLI first, runtime second
- no browser/desktop UI integration yet unless CLI/runtime spike proves useful

### Step 3: Specify verification commands

List likely commands such as:
- local environment bootstrap
- one-shot inference on sample text
- regression verification against saved synthetic fixtures

### Step 4: Commit any new plan

```bash
git add docs/superpowers/plans/2026-04-30-privacy-filter-cli-spike.md docs/research/privacy-filter-eval.md
git commit -m "docs: plan bounded privacy filter spike"
```

---

## Task 5: Build bounded OCR prototype plan

**Objective:** Define the smallest OCR spike worth doing next if the small OCR candidate passes evaluation.

**Files:**
- Modify: `docs/research/small-ocr-eval.md`
- Optional create: `docs/superpowers/plans/2026-04-30-small-ocr-spike.md`

### Step 1: Define OCR spike inputs

Use bounded synthetic fixtures only:
- one synthetic scanned PDF page render
- one synthetic image containing printed PHI-like text
- optional multilingual sample only if it does not widen scope too much

### Step 2: Define expected outputs

The OCR spike is successful if it can:
- extract text locally on CPU
- preserve enough token fidelity for downstream PII detection evaluation
- output text that can be handed into the Privacy Filter prototype path

### Step 3: State non-goals

The OCR spike must not claim:
- visual redaction done
- rewritten PDF generated
- desktop/browser full UX completed
- handwritten medical OCR solved

### Step 4: Commit any new plan

```bash
git add docs/superpowers/plans/2026-04-30-small-ocr-spike.md docs/research/small-ocr-eval.md
git commit -m "docs: plan bounded small ocr spike"
```

---

## Task 6: Truth-sync README only after evidence exists

**Objective:** Update README only after evaluation/prototype evidence actually lands.

**Files:**
- Modify: `README.md`

### Step 1: Inspect landed evidence first

Run:
```bash
git status --short
git log --oneline -10
```

### Step 2: Update README carefully

Only add claims that are actually true, for example:
- “Privacy Filter evaluation completed as a candidate for text-only PII span detection”
- “Small OCR candidate selected for bounded local evaluation”

Do **not** claim:
- OCR solved
- visual redaction solved
- PDF rewrite/export solved
- browser/desktop full workflow complete

### Step 3: Verification

Run the exact evaluation/prototype verification commands documented by the landed spike before changing completion percentages.

### Step 4: Commit the truth-sync

```bash
git add README.md
git commit -m "docs: truth-sync privacy filter and ocr evaluation status"
```

---

## Recommended execution order

1. Task 1 — evaluation spec
2. Task 2 — Privacy Filter evaluation
3. Task 3 — small OCR evaluation
4. Task 4 — Privacy Filter bounded spike plan
5. Task 5 — OCR bounded spike plan
6. Task 6 — README truth-sync

---

## Verification checklist

Before calling the work complete:
- [ ] The spec clearly distinguishes text PII detection vs OCR vs visual redaction
- [ ] Privacy Filter note names concrete integration seams and limitations
- [ ] OCR note names a primary small model and at least one comparison model
- [ ] No artifact falsely claims OCR/visual redaction/full PDF rewrite is solved
- [ ] Any README update is backed by landed evidence
- [ ] Plans are executable by subagent-driven-development without guessing

---

## Expected recommendation if research remains consistent

Unless later evidence contradicts it, the expected current recommendation is:
- **Adopt OpenAI Privacy Filter as a strong text-only PII detection candidate for bounded local evaluation**
- **Adopt PaddleOCR mobile OCR as the primary small OCR evaluation candidate**
- **Do not claim either candidate closes the visual-redaction / final-PDF-rewrite gap**
