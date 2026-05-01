# Privacy Filter Runner Stdin Implementation Plan

**Goal:** Let the bounded text-only Privacy Filter runner and `mdid-cli privacy-filter-text --stdin` stream PHI-bearing text through stdin instead of materializing a temporary runner input file.

**Architecture:** Extend the existing Python Privacy Filter runner with an exactly-one-source contract: positional UTF-8 text file or `--stdin`. Update the Rust CLI wrapper so its `--stdin` mode invokes the runner with `--stdin` and pipes the bounded captured stdin into the child process while preserving all existing JSON validation, stale artifact cleanup, timeout/stdout caps, and PHI-safe stdout/stderr behavior.

**Tech Stack:** Python 3 runner/tests, Rust `mdid-cli`, Cargo integration smoke tests, existing Privacy Filter JSON validator.

---

## File Structure

- Modify: `scripts/privacy_filter/run_privacy_filter.py` — add parser support for `--stdin`, exactly-one-source validation, and stdin text loading while preserving `--mock` and explicit `--use-opf` behavior.
- Modify: `scripts/privacy_filter/test_run_privacy_filter.py` — add Python RED tests for runner stdin success, conflicting sources, and OPF stdin plumbing.
- Modify: `crates/mdid-cli/src/main.rs` — change `privacy-filter-text --stdin` runner invocation to pass text through child stdin instead of creating a temporary input file.
- Modify: `crates/mdid-cli/tests/cli_smoke.rs` — add a fake-runner smoke test proving Rust stdin mode invokes the runner with `--stdin` and no positional input path.
- Modify: `scripts/privacy_filter/README.md` — document runner `--stdin` as text-only local PII detection input plumbing, not OCR or visual redaction.
- Modify: `README.md` — truth-sync completion/evidence after verification; add the new CLI/runtime requirement to the fraction accounting without increasing Browser/Web or Desktop.

### Task 1: Python runner accepts stdin as a first-class input source

**Files:**
- Modify: `scripts/privacy_filter/run_privacy_filter.py`
- Modify: `scripts/privacy_filter/test_run_privacy_filter.py`

- [ ] **Step 1: Write the failing Python stdin tests**

Add tests that execute the runner through its CLI entrypoint. The tests must assert that `--stdin --mock` returns the same bounded JSON contract as file input, that positional input plus `--stdin` is rejected, and that explicit OPF mode receives PHI through subprocess stdin rather than argv.

- [ ] **Step 2: Run Python RED tests**

Run: `python -m unittest scripts/privacy_filter/test_run_privacy_filter.py -v`

Expected before implementation: at least one failure because `run_privacy_filter.py` does not accept `--stdin`.

- [ ] **Step 3: Implement minimal runner stdin support**

Update `run_privacy_filter.py` so the parser accepts optional positional `input_path` and `--stdin`. Reject missing input and conflicting input with generic PHI-safe parser errors. When `--stdin` is present, read at most 1 MiB of UTF-8 text from stdin and feed it into the existing detection path. Do not pass stdin text as a command-line argument to OPF.

- [ ] **Step 4: Run Python GREEN tests**

Run: `python -m unittest scripts/privacy_filter/test_run_privacy_filter.py -v`

Expected: PASS.

- [ ] **Step 5: Commit Task 1**

```bash
git add scripts/privacy_filter/run_privacy_filter.py scripts/privacy_filter/test_run_privacy_filter.py
git commit -m "feat(privacy-filter): accept runner stdin input"
```

### Task 2: Rust CLI pipes `privacy-filter-text --stdin` to runner stdin

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`

- [ ] **Step 1: Write the failing Rust smoke test**

Add `privacy_filter_text_stdin_pipes_to_runner_without_temp_input_file`. The fake runner must fail if it receives a positional path-like argument and must succeed only when it receives `--stdin`, reads stdin, and emits a valid Privacy Filter JSON report. The test must assert no raw synthetic PHI or caller temp path appears in stdout/stderr and the report validates.

- [ ] **Step 2: Run Rust RED test**

Run: `cargo test -p mdid-cli --test cli_smoke privacy_filter_text_stdin_pipes_to_runner_without_temp_input_file -- --nocapture`

Expected before implementation: FAIL because the CLI materializes stdin to a temp input file and passes a positional input path to the runner.

- [ ] **Step 3: Implement minimal Rust stdin piping**

In `privacy-filter-text --stdin` mode, build runner args with `--stdin` instead of the temp input path, append `--mock` when requested, spawn the child with piped stdin, write the already-bounded captured stdin text to the child, close stdin, then reuse the existing bounded stdout read/timeout/validation/report write path. Preserve path mode behavior unchanged.

- [ ] **Step 4: Run Rust GREEN tests**

Run: `cargo test -p mdid-cli --test cli_smoke privacy_filter_text -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Commit Task 2**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs
git commit -m "feat(cli): pipe privacy filter stdin to runner"
```

### Task 3: Documentation, README truth-sync, and integration verification

**Files:**
- Modify: `scripts/privacy_filter/README.md`
- Modify: `README.md`

- [ ] **Step 1: Update docs after code is green**

Document that `run_privacy_filter.py --stdin` and `mdid-cli privacy-filter-text --stdin` are bounded text-only Privacy Filter input plumbing, not OCR, visual redaction, image pixel redaction, handwriting recognition, Browser/Web execution, Desktop execution, or final PDF rewrite/export.

- [ ] **Step 2: Truth-sync README completion**

Add the new CLI/runtime requirement to fraction accounting as completed in the same round. Keep conservative integer floors truthful: CLI remains 95%, Browser/Web remains 99%, Desktop app remains 99%, Overall remains 97% unless repository-visible facts support a different conservative fraction. Explicitly mark Browser/Web +5 and Desktop +5 as not achieved because this is CLI/runtime-only blocker hardening.

- [ ] **Step 3: Run final verification**

Run:

```bash
python -m unittest scripts/privacy_filter/test_run_privacy_filter.py -v
cargo test -p mdid-cli --test cli_smoke privacy_filter_text -- --nocapture
python scripts/privacy_filter/run_privacy_filter.py --stdin --mock < scripts/privacy_filter/fixtures/sample_text_input.txt > /tmp/privacy-filter-stdin-output.json
python scripts/privacy_filter/validate_privacy_filter_output.py /tmp/privacy-filter-stdin-output.json
cargo fmt --check
git diff --check
```

Expected: all commands pass.

- [ ] **Step 4: Commit Task 3**

```bash
git add scripts/privacy_filter/README.md README.md
git commit -m "docs: truth-sync privacy filter runner stdin"
```
