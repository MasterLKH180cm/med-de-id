# OCR Handoff Corpus CLI Wrapper Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded CLI wrapper for the synthetic PP-OCRv5 mobile handoff corpus runner so checked-in OCR text fixtures can be verified through a PHI-safe aggregate CLI/runtime contract.

**Architecture:** This adds a CLI/runtime-only `mdid-cli ocr-handoff-corpus` command that runs the existing Python corpus runner, validates its aggregate JSON contract, redacts report paths in stdout, and removes stale reports on failure. It is printed-text extraction readiness evidence only; it is not OCR model-quality proof, visual redaction, PDF rewrite/export, browser UI, desktop UI, or unrelated workflow orchestration semantics.

**Tech Stack:** Rust `mdid-cli`, `assert_cmd`, `serde_json`, Python helper `scripts/ocr_eval/run_ocr_handoff_corpus.py`, markdown docs.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs` — add `ocr-handoff-corpus` argument parsing, runner invocation, aggregate report validation, stale-report cleanup, and PHI-safe stdout summary.
- Modify: `crates/mdid-cli/tests/cli_smoke.rs` — add fixture-backed smoke tests for success and failure/stale cleanup.
- Modify: `README.md` — truth-sync OCR corpus CLI wrapper evidence and completion without inflating Browser/Web or Desktop.
- Modify: `docs/research/small-ocr-spike-results.md` — record exact verification commands and non-goals.

### Task 1: Add bounded `mdid-cli ocr-handoff-corpus` wrapper

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`

- [ ] **Step 1: Write the failing success smoke test**

Add a test named `ocr_handoff_corpus_runs_repo_fixture_runner_without_phi_leaks` to `crates/mdid-cli/tests/cli_smoke.rs`. It should run:

```rust
Command::cargo_bin("mdid-cli")
    .unwrap()
    .arg("ocr-handoff-corpus")
    .arg("--fixture-dir")
    .arg(repo_path("scripts/ocr_eval/fixtures/corpus"))
    .arg("--runner-path")
    .arg(repo_path("scripts/ocr_eval/run_ocr_handoff_corpus.py"))
    .arg("--report-path")
    .arg(&report_path)
    .arg("--python-command")
    .arg(default_python_command())
    .assert()
    .success()
    .stdout(predicate::str::contains("ocr-handoff-corpus"))
    .stdout(predicate::str::contains("<redacted>"))
    .stdout(predicate::str::contains("Jane Example").not())
    .stdout(predicate::str::contains("MRN-12345").not())
    .stderr(predicate::str::is_empty());
```

Then read the report JSON and assert `engine == "PP-OCRv5-mobile-bounded-spike"`, `scope == "printed_text_line_extraction_only"`, `privacy_filter_contract == "text_only_normalized_input"`, `fixture_count >= 2`, `ready_fixture_count == fixture_count`, every fixture id starts with `fixture_`, and the report does not contain `Jane Example`, `MRN-12345`, `jane@example.com`, or `555-123-4567`.

- [ ] **Step 2: Run the test to verify RED**

Run: `cargo test -p mdid-cli ocr_handoff_corpus_runs_repo_fixture_runner_without_phi_leaks -- --nocapture`
Expected: FAIL because `ocr-handoff-corpus` is not yet a supported command.

- [ ] **Step 3: Write the minimal wrapper implementation**

Update `crates/mdid-cli/src/main.rs` to parse `ocr-handoff-corpus --fixture-dir <dir> --runner-path <file> --report-path <file> [--python-command <cmd>]`, run the Python runner with `--fixture-dir` and `--output`, suppress runner stdout/stderr, validate the report is aggregate-only, allow only these top-level keys: `engine`, `scope`, `fixture_count`, `ready_fixture_count`, `total_char_count`, `fixtures`, `non_goals`, `privacy_filter_contract`; require engine/scope/contract strings above; require nonnegative integer counts; require fixtures array entries to contain only `id`, `char_count`, and `ready_for_text_pii_eval`; require every fixture id to match `fixture_###`; require non-goals to include `visual_redaction` and `final_pdf_rewrite_export`; reject raw synthetic PHI sentinels in report text; remove stale report before run and after any failure; print a summary with `report_path: "<redacted>"`.

- [ ] **Step 4: Run the success smoke test to verify GREEN**

Run: `cargo test -p mdid-cli ocr_handoff_corpus_runs_repo_fixture_runner_without_phi_leaks -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Add failure/stale cleanup smoke test**

Add `ocr_handoff_corpus_removes_stale_report_when_runner_fails`. It should create a temp fixture directory with one `.txt` fixture, create a fake Python runner that exits 1 without writing a report, pre-write `stale raw Jane Example` to the report path, run `mdid-cli ocr-handoff-corpus`, assert failure, and assert the report path no longer exists.

- [ ] **Step 6: Run targeted and supporting tests**

Run:

```bash
cargo test -p mdid-cli ocr_handoff_corpus -- --nocapture
python scripts/ocr_eval/run_ocr_handoff_corpus.py --fixture-dir scripts/ocr_eval/fixtures/corpus --output /tmp/ocr-handoff-corpus.json
python - <<'PY'
import json
from pathlib import Path
obj=json.loads(Path('/tmp/ocr-handoff-corpus.json').read_text())
assert obj['engine'] == 'PP-OCRv5-mobile-bounded-spike'
assert obj['scope'] == 'printed_text_line_extraction_only'
assert obj['privacy_filter_contract'] == 'text_only_normalized_input'
assert obj['ready_fixture_count'] == obj['fixture_count']
print('ocr corpus aggregate ok')
PY
git diff --check
```

Expected: all commands exit 0.

- [ ] **Step 7: Commit**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs
git commit -m "feat(cli): add OCR handoff corpus wrapper"
```

### Task 2: Truth-sync OCR corpus CLI wrapper docs and completion

**Files:**
- Modify: `README.md`
- Modify: `docs/research/small-ocr-spike-results.md`

- [ ] **Step 1: Write the failing docs check**

Run:

```bash
python - <<'PY'
from pathlib import Path
text = Path('README.md').read_text() + '\n' + Path('docs/research/small-ocr-spike-results.md').read_text()
required = [
    'ocr-handoff-corpus',
    'PP-OCRv5 mobile handoff corpus',
    'printed-text extraction readiness',
    'not visual redaction',
    'not final PDF rewrite/export',
]
missing = [term for term in required if term not in text]
if missing:
    raise SystemExit('missing docs terms: ' + ', '.join(missing))
PY
```

Expected: FAIL until docs mention the wrapper evidence and non-goals.

- [ ] **Step 2: Update docs with exact evidence**

Add a concise evidence paragraph to `docs/research/small-ocr-spike-results.md` with the `mdid-cli ocr-handoff-corpus` command and the Python runner validation command. Update `README.md` current status/evidence to mention the wrapper as CLI/runtime evidence only. Keep completion honest: CLI 95%, Browser/Web 93%, Desktop app 93%, Overall 95% unless repository-visible landed facts support a different re-baseline. State this is not browser/desktop capability and does not satisfy +5 Browser/Desktop surface progress.

- [ ] **Step 3: Run docs check to verify GREEN**

Run the Python docs check from Step 1 again.
Expected: PASS.

- [ ] **Step 4: Run final verification**

Run:

```bash
cargo test -p mdid-cli ocr_handoff_corpus -- --nocapture
python scripts/ocr_eval/run_ocr_handoff_corpus.py --fixture-dir scripts/ocr_eval/fixtures/corpus --output /tmp/ocr-handoff-corpus.json
git diff --check
```

Expected: all commands exit 0.

- [ ] **Step 5: Commit**

Run:

```bash
git add README.md docs/research/small-ocr-spike-results.md
git commit -m "docs: truth-sync OCR handoff corpus CLI wrapper"
```

## Self-Review

Spec coverage: Task 1 adds the bounded CLI/runtime wrapper for the synthetic OCR handoff corpus and validates PHI-safe aggregate-only semantics. Task 2 updates research and README evidence without claiming browser/desktop progress.

Placeholder scan: No TBD, TODO, fill-in, or undefined later work remains.

Type consistency: The command name is consistently `ocr-handoff-corpus`; existing helpers are `repo_path` and `default_python_command`; the Python runner contract fields match `scripts/ocr_eval/run_ocr_handoff_corpus.py`.
