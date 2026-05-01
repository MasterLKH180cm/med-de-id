# Privacy Filter CLI Spike Implementation Plan

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** Build a bounded local CLI-first spike that proves OpenAI Privacy Filter can classify and mask PII spans in synthetic text for `med-de-id` without widening into OCR, visual redaction, or browser/desktop UI work.

**Architecture:** Keep this spike outside the core Rust path at first by using a small local Python helper under a clearly scoped tools/scripts directory, then invoke it from a bounded `mdid-cli` command or evaluation harness only if the local install and output contract prove useful. Use synthetic fixtures only and emit a PHI-safe structured result contract.

**Tech Stack:** Rust workspace for eventual CLI wrapper, local Python helper for the model runner, OpenAI Privacy Filter upstream package, synthetic JSON/text fixtures.

---

## File Structure

- Create: `docs/research/privacy-filter-cli-spike-results.md`
- Create: `scripts/privacy_filter/README.md`
- Create: `scripts/privacy_filter/run_privacy_filter.py`
- Create: `scripts/privacy_filter/validate_privacy_filter_output.py`
- Create: `scripts/privacy_filter/fixtures/sample_text_input.txt`
- Create: `scripts/privacy_filter/fixtures/sample_text_expected_shape.json`
- Optional later modify: `crates/mdid-cli/src/main.rs`
- Optional later test: `crates/mdid-cli/tests/cli_smoke.rs`

## Task 1: Define bounded output contract
- [ ] Write the fixture contract first in `scripts/privacy_filter/fixtures/sample_text_expected_shape.json`
- [ ] Contract must include exact top-level keys:
  - `summary.input_char_count`
  - `summary.detected_span_count`
  - `summary.category_counts`
  - `masked_text`
  - `spans[]` with `label`, `start`, `end`, `preview`
- [ ] `start` is inclusive and `end` is exclusive.
- [ ] `preview` must be redacted or fixture-safe only.
- [ ] Add `scripts/privacy_filter/validate_privacy_filter_output.py` that reads JSON from stdin or a file path and exits non-zero when the shape is missing required keys.
- [ ] RED command before the runner exists:
```bash
python scripts/privacy_filter/validate_privacy_filter_output.py scripts/privacy_filter/fixtures/sample_text_expected_shape.json
python scripts/privacy_filter/run_privacy_filter.py scripts/privacy_filter/fixtures/sample_text_input.txt
```
- [ ] Expected RED evidence: validator may pass against the fixture contract itself, but the runner command must fail because `run_privacy_filter.py` does not exist yet.

## Task 2: Create local runner wrapper
- [ ] Add `scripts/privacy_filter/run_privacy_filter.py`
- [ ] The runner should:
  - accept text input file path
  - invoke local Privacy Filter
  - normalize model output into the bounded JSON contract
  - avoid logging raw sensitive content except synthetic fixture text already controlled in-repo
- [ ] GREEN command after implementation:
```bash
python scripts/privacy_filter/run_privacy_filter.py scripts/privacy_filter/fixtures/sample_text_input.txt > /tmp/privacy-filter-output.json
python scripts/privacy_filter/validate_privacy_filter_output.py /tmp/privacy-filter-output.json
```
- [ ] Expected GREEN evidence: validator exits zero and the output contains the required contract fields.

## Task 3: Add reproducible local usage docs
- [ ] Add `scripts/privacy_filter/README.md`
- [ ] Include exact install/bootstrap steps
- [ ] Include exact command lines for:
  - one-shot run on synthetic fixture
  - contract validation
  - optional operating-point/config variation if the upstream tool exposes it in the local setup
- [ ] Include explicit non-goals: not OCR, not visual redaction, not PDF rewrite/export

## Task 4: Record spike results
- [ ] Create `docs/research/privacy-filter-cli-spike-results.md`
- [ ] Capture:
  - install/runtime friction
  - output usefulness
  - summary/category shape
  - whether a later `mdid-cli` integration is justified
- [ ] Include a final Go / No-Go / More-Evidence verdict

## Task 5: Optional Rust CLI wrapper only if the Python spike is useful
- [ ] If Tasks 1-4 are successful, add a bounded `mdid-cli` helper path or wrapper plan
- [ ] Keep the wrapper synthetic-fixture-only at first
- [ ] Do not widen into runtime/browser/desktop until the CLI spike proves stable

## Verification
- [ ] Run the local Privacy Filter spike on synthetic text
- [ ] Validate output shape against the expected JSON contract with `validate_privacy_filter_output.py`
- [ ] Record any operating-point / masking-strictness configuration tried during the spike
- [ ] Re-run after any refactor to prove stability
- [ ] If a Rust wrapper is added, run targeted `mdid-cli` tests
