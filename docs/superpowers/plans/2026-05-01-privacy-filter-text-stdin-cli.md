# Privacy Filter Text Stdin CLI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded stdin input mode to `mdid-cli privacy-filter-text` so the CLI-first Privacy Filter text-only PII detection POC can run without creating an intermediate PHI text file.

**Architecture:** Keep the existing local runner/validator contract unchanged: the Rust CLI still delegates to the configured local Python runner and validates its JSON output before writing any report or summary. The only behavior change is input-source selection: exactly one of `--input-path <path>` or `--stdin` is required, and stdin input is copied into a private temporary file that is deleted after the runner finishes. Summary/stdout remain PHI-safe and path-redacted.

**Tech Stack:** Rust CLI (`crates/mdid-cli/src/main.rs`), assert_cmd predicates smoke tests (`crates/mdid-cli/tests/cli_smoke.rs`), Python synthetic Privacy Filter runner (`scripts/privacy_filter/run_privacy_filter.py`).

---

## File Structure

- Modify `crates/mdid-cli/src/main.rs`
  - Add `PrivacyFilterTextInput` enum with `Path(PathBuf)` and `Stdin` variants.
  - Update `PrivacyFilterTextArgs` to store `input: PrivacyFilterTextInput` instead of only `input_path`.
  - Update parser so `--stdin` is a boolean flag, `--input-path` and `--stdin` are mutually exclusive, and one input source is required.
  - Update runner execution to materialize stdin into a bounded temp file before invoking the existing runner, then delete it after completion.
  - Update usage string to document `--stdin`.
- Modify `crates/mdid-cli/tests/cli_smoke.rs`
  - Add failing smoke tests for stdin success and input-source validation.
- Modify `README.md`
  - Truth-sync the Privacy Filter CLI evidence, completion arithmetic, and remaining gaps after verification.

### Task 1: Add bounded stdin input mode to Privacy Filter text CLI

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/cli_smoke.rs`
- Modify: `README.md`

- [ ] **Step 1: Write the failing stdin success test**

Append this test near the existing `privacy_filter_text` smoke tests in `crates/mdid-cli/tests/cli_smoke.rs`:

```rust
#[test]
fn privacy_filter_text_accepts_stdin_without_leaking_input_path() {
    let temp = TempDir::new().unwrap();
    let report_path = temp.path().join("stdin-report.json");
    let summary_path = temp.path().join("stdin-summary.json");

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "privacy-filter-text",
            "--stdin",
            "--runner-path",
            "scripts/privacy_filter/run_privacy_filter.py",
            "--report-path",
            report_path.to_str().unwrap(),
            "--summary-output",
            summary_path.to_str().unwrap(),
            "--mock",
        ])
        .write_stdin("Patient Jane Example has MRN-12345 and email jane@example.com.")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"command\": \"privacy-filter-text\""))
        .stdout(predicate::str::contains("\"report_path\": \"<redacted>\""))
        .stdout(predicate::str::contains("Jane").not())
        .stdout(predicate::str::contains("MRN-12345").not())
        .stdout(predicate::str::contains("jane@example.com").not());

    let report = fs::read_to_string(&report_path).unwrap();
    assert!(report.contains("\"network_api_called\": false"));
    assert!(report.contains("\"masked_text\""));

    let summary = fs::read_to_string(&summary_path).unwrap();
    assert!(summary.contains("\"artifact\": \"privacy_filter_text_summary\""));
    assert!(!summary.contains("Patient Jane Example"));
    assert!(!summary.contains("MRN-12345"));
    assert!(!summary.contains("jane@example.com"));
}
```

- [ ] **Step 2: Write failing parser validation tests**

Append these tests in the same file:

```rust
#[test]
fn privacy_filter_text_rejects_missing_input_source() {
    let temp = TempDir::new().unwrap();
    let report_path = temp.path().join("missing-input-report.json");

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "privacy-filter-text",
            "--runner-path",
            "scripts/privacy_filter/run_privacy_filter.py",
            "--report-path",
            report_path.to_str().unwrap(),
            "--mock",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("missing input source: provide exactly one of --input-path or --stdin"));
}

#[test]
fn privacy_filter_text_rejects_both_input_path_and_stdin() {
    let temp = TempDir::new().unwrap();
    let input_path = temp.path().join("input.txt");
    let report_path = temp.path().join("both-inputs-report.json");
    fs::write(&input_path, "Patient Jane Example has MRN-12345.").unwrap();

    Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "privacy-filter-text",
            "--input-path",
            input_path.to_str().unwrap(),
            "--stdin",
            "--runner-path",
            "scripts/privacy_filter/run_privacy_filter.py",
            "--report-path",
            report_path.to_str().unwrap(),
            "--mock",
        ])
        .write_stdin("Patient Jane Example has MRN-12345.")
        .assert()
        .failure()
        .stderr(predicate::str::contains("conflicting input sources: provide exactly one of --input-path or --stdin"));
}
```

- [ ] **Step 3: Run tests to verify RED**

Run:

```bash
cargo test -p mdid-cli --test cli_smoke privacy_filter_text_accepts_stdin_without_leaking_input_path -- --nocapture
cargo test -p mdid-cli --test cli_smoke privacy_filter_text_rejects_missing_input_source -- --nocapture
cargo test -p mdid-cli --test cli_smoke privacy_filter_text_rejects_both_input_path_and_stdin -- --nocapture
```

Expected: stdin test fails because `--stdin` is an unknown flag; missing-input test fails because the existing error says `missing --input-path`; both-inputs test fails because `--stdin` is an unknown flag.

- [ ] **Step 4: Implement input-source parsing**

In `crates/mdid-cli/src/main.rs`, replace `PrivacyFilterTextArgs` with:

```rust
#[derive(Clone, PartialEq, Eq)]
enum PrivacyFilterTextInput {
    Path(PathBuf),
    Stdin,
}

#[derive(Clone, PartialEq, Eq)]
struct PrivacyFilterTextArgs {
    input: PrivacyFilterTextInput,
    runner_path: PathBuf,
    report_path: PathBuf,
    summary_output: Option<PathBuf>,
    python_command: String,
    mock: bool,
}
```

Update `parse_privacy_filter_text_args` so:

```rust
let mut input_path = None;
let mut read_stdin = false;
```

Add flag handling before value-taking:

```rust
if flag == "--stdin" {
    read_stdin = true;
    index += 1;
    continue;
}
```

Build the input source before returning:

```rust
let input = match (input_path, read_stdin) {
    (Some(path), false) => PrivacyFilterTextInput::Path(path),
    (None, true) => PrivacyFilterTextInput::Stdin,
    (None, false) => {
        return Err("missing input source: provide exactly one of --input-path or --stdin".to_string())
    }
    (Some(_), true) => {
        return Err("conflicting input sources: provide exactly one of --input-path or --stdin".to_string())
    }
};
```

Then return `PrivacyFilterTextArgs { input, runner_path: ..., ... }`.

- [ ] **Step 5: Implement bounded stdin materialization**

In `run_privacy_filter_text`, replace direct input file validation with input-source handling:

```rust
let result = (|| {
    let materialized_stdin;
    let input_path = match &args.input {
        PrivacyFilterTextInput::Path(path) => {
            require_regular_file(path, "missing input file")?;
            path.clone()
        }
        PrivacyFilterTextInput::Stdin => {
            materialized_stdin = materialize_privacy_filter_stdin()?;
            materialized_stdin.clone()
        }
    };
    require_regular_file(&args.runner_path, "missing runner file")?;
    run_privacy_filter_text_inner(&args, &input_path)
})();
```

Change the inner function signature and runner argument:

```rust
fn run_privacy_filter_text_inner(args: &PrivacyFilterTextArgs, input_path: &Path) -> Result<(), String> {
    let mut command = std::process::Command::new(&args.python_command);
    command.arg(&args.runner_path);
    if args.mock {
        command.arg("--mock");
    }
    command.arg(input_path);
```

Add helper near the existing Privacy Filter helpers:

```rust
fn materialize_privacy_filter_stdin() -> Result<PathBuf, String> {
    let mut buffer = Vec::new();
    io::stdin()
        .take(PRIVACY_FILTER_RUNNER_STDOUT_MAX_BYTES as u64 + 1)
        .read_to_end(&mut buffer)
        .map_err(|err| format!("failed to read stdin: {err}"))?;
    if buffer.is_empty() {
        return Err("missing stdin input".to_string());
    }
    if buffer.len() > PRIVACY_FILTER_RUNNER_STDOUT_MAX_BYTES {
        return Err("stdin input exceeds 1048576 byte limit".to_string());
    }
    let path = std::env::temp_dir().join(format!(
        "mdid-privacy-filter-stdin-{}-{}.txt",
        std::process::id(),
        unique_temp_suffix()
    ));
    fs::write(&path, buffer).map_err(|err| format!("failed to materialize stdin input: {err}"))?;
    Ok(path)
}

fn unique_temp_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}
```

If `io::Read` or `Path` are not already imported, add them to the top-level imports.

- [ ] **Step 6: Ensure stdin temp file cleanup**

Update the stdin branch in `run_privacy_filter_text` so cleanup runs after `run_privacy_filter_text_inner`, even on validation failure:

```rust
PrivacyFilterTextInput::Stdin => {
    let input_path = materialize_privacy_filter_stdin()?;
    require_regular_file(&args.runner_path, "missing runner file")?;
    let run_result = run_privacy_filter_text_inner(&args, &input_path);
    let _ = fs::remove_file(&input_path);
    run_result
}
```

Keep path mode unchanged except for passing `&path` into the inner function.

- [ ] **Step 7: Update usage text**

In `usage()`, change the Privacy Filter text line to document both input modes:

```text
mdid-cli privacy-filter-text (--input-path <text> | --stdin) --runner-path <runner.py> --report-path <report.json> [--summary-output <summary.json>] [--python-command <python>] [--mock]
```

- [ ] **Step 8: Run targeted tests to verify GREEN**

Run:

```bash
cargo test -p mdid-cli --test cli_smoke privacy_filter_text_accepts_stdin_without_leaking_input_path -- --nocapture
cargo test -p mdid-cli --test cli_smoke privacy_filter_text_rejects_missing_input_source -- --nocapture
cargo test -p mdid-cli --test cli_smoke privacy_filter_text_rejects_both_input_path_and_stdin -- --nocapture
cargo test -p mdid-cli privacy_filter_text --test cli_smoke -- --nocapture
```

Expected: all targeted Privacy Filter text tests pass.

- [ ] **Step 9: Run broader verification**

Run:

```bash
cargo test -p mdid-cli --test cli_smoke
python -m py_compile scripts/privacy_filter/run_privacy_filter.py scripts/privacy_filter/validate_privacy_filter_output.py
git diff --check
```

Expected: all commands pass.

- [ ] **Step 10: README truth-sync**

Update `README.md` completion snapshot to add one landed CLI/runtime requirement: stdin input mode for `mdid-cli privacy-filter-text`. Completion arithmetic should remain conservative: CLI was `97/102 = 95%`; adding and completing this requirement makes it `98/103 = 95%` floor. Browser/Web remains 99%, Desktop app remains 99%, Overall remains 97%. Include explicit non-goals: text-only PII detection/masking candidate; not OCR, not visual redaction, not image pixel redaction, not handwriting recognition, not final PDF rewrite/export, and not browser/desktop integration.

- [ ] **Step 11: Commit**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs README.md docs/superpowers/plans/2026-05-01-privacy-filter-text-stdin-cli.md
git commit -m "feat(cli): add privacy filter stdin input"
```
