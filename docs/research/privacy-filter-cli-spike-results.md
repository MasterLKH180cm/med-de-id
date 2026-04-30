# Privacy Filter CLI Spike Results

## Status
- **Contract/plumbing status:** PASS
- **Real-model status:** NOT YET VERIFIED on this machine

## What was implemented
- `scripts/privacy_filter/run_privacy_filter.py`
- `scripts/privacy_filter/validate_privacy_filter_output.py`
- synthetic input fixture
- expected output contract fixture
- bounded README with exact commands

## Verification run
### Contract fixture validation
```bash
python scripts/privacy_filter/validate_privacy_filter_output.py scripts/privacy_filter/fixtures/sample_text_expected_shape.json
```
Result: PASS

### Mock plumbing verification
```bash
python scripts/privacy_filter/run_privacy_filter.py --mock scripts/privacy_filter/fixtures/sample_text_input.txt > /tmp/privacy-filter-output.json
python scripts/privacy_filter/validate_privacy_filter_output.py /tmp/privacy-filter-output.json
```
Result: PASS

Observed mock output summary:
- `input_char_count`: 137
- `detected_span_count`: 4
- `category_counts`: `PERSON=1`, `EMAIL=1`, `PHONE=1`, `ID=1`

## Current limitation
The upstream `opf` CLI is not installed locally in this environment, so the real Privacy Filter model path currently exits with a truthful error instructing the operator to either install the upstream tool or use `--mock` for plumbing-only validation.

## Output usefulness
The bounded output contract is now explicitly and strictly validated:
- `summary.input_char_count`
- `summary.detected_span_count`
- `summary.category_counts`
- `masked_text`
- `spans[]` with `label`, `start`, `end`, `preview`
- summary counts must match span labels
- spans must be sorted and non-overlapping
- placeholder masked text is rejected

This is sufficient to support a later CLI/runtime text-only spike once the real model is installed.

## Verdict
- **Go for next step:** YES, for bounded CLI/runtime integration plumbing
- **Go for real model adoption right now:** MORE EVIDENCE NEEDED
- **Reason:** real upstream model path still needs local install + real inference verification
