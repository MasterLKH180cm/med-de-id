# Privacy Filter Corpus CLI Wrapper Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded `mdid-cli privacy-filter-corpus` wrapper that runs the landed synthetic text-only Privacy Filter corpus runner and emits a PHI-safe summary.

**Architecture:** Extend the existing CLI Privacy Filter runtime path with a second wrapper around `scripts/privacy_filter/run_synthetic_corpus.py`. The wrapper validates fixture directory and runner path, invokes Python with bounded stdout and timeout, writes the aggregate report to `--report-path`, validates only PHI-safe aggregate fields, and prints a short JSON summary; it does not add OCR, visual redaction, image redaction, PDF rewrite/export, browser/desktop UI, or any agent/controller semantics.

**Tech Stack:** Rust `mdid-cli`, `assert_cmd`, `serde_json`, Python stdlib corpus runner under `scripts/privacy_filter`.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs` — add `PrivacyFilterCorpusArgs`, parse `privacy-filter-corpus`, execute bounded Python corpus runner, validate aggregate JSON, and update usage.
- Modify: `crates/mdid-cli/tests/cli_smoke.rs` — add fixture-backed smoke coverage that proves PHI-safe CLI stdout/stderr/report behavior.
- Modify: `README.md` — truth-sync completion evidence without inflating Browser/Web or Desktop.

### Task 1: Add `mdid-cli privacy-filter-corpus` runtime wrapper

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`

- [ ] **Step 1: Write the failing test**

Add this test near the existing Privacy Filter CLI smoke tests in `crates/mdid-cli/tests/cli_smoke.rs`:

```rust
#[test]
fn cli_privacy_filter_corpus_writes_phi_safe_aggregate_summary() {
    let dir = tempdir().unwrap();
    let report_path = dir.path().join("privacy-filter-corpus.json");

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .arg("privacy-filter-corpus")
        .arg("--fixture-dir")
        .arg(repo_path("scripts/privacy_filter/fixtures/corpus"))
        .arg("--runner-path")
        .arg(repo_path("scripts/privacy_filter/run_synthetic_corpus.py"))
        .arg("--report-path")
        .arg(&report_path)
        .arg("--python-command")
        .arg(default_python_command())
        .assert()
        .success()
        .stdout(predicate::str::contains("Jane Example").not())
        .stdout(predicate::str::contains("MRN-12345").not())
        .stdout(predicate::str::contains("jane@example.test").not())
        .stdout(predicate::str::contains("555-111-2222").not())
        .stderr(predicate::str::contains("Jane Example").not())
        .stderr(predicate::str::contains("MRN-12345").not())
        .get_output()
        .stdout
        .clone();

    let summary: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(summary["command"], "privacy-filter-corpus");
    assert_eq!(summary["engine"], "fallback_synthetic_patterns");
    assert_eq!(summary["scope"], "text_only_synthetic_corpus");
    assert_eq!(summary["fixture_count"], 2);
    assert!(summary["total_detected_span_count"].as_u64().unwrap() >= 4);
    assert_eq!(summary["report_path"], report_path.to_string_lossy().to_string());

    let report = fs::read_to_string(&report_path).unwrap();
    assert!(!report.contains("Jane Example"));
    assert!(!report.contains("MRN-12345"));
    assert!(!report.contains("jane@example.test"));
    assert!(!report.contains("555-111-2222"));
    let report_json: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report_json["category_counts"]["NAME"], 2);
    assert_eq!(report_json["category_counts"]["MRN"], 2);
    assert_eq!(report_json["category_counts"]["EMAIL"], 1);
    assert_eq!(report_json["category_counts"]["PHONE"], 2);
    assert!(report_json["non_goals"].as_array().unwrap().contains(&Value::String("visual_redaction".to_string())));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p mdid-cli cli_privacy_filter_corpus_writes_phi_safe_aggregate_summary -- --nocapture`
Expected: FAIL with unknown command or missing `privacy-filter-corpus` parser/runtime.

- [ ] **Step 3: Implement minimal wrapper**

In `crates/mdid-cli/src/main.rs` add `PrivacyFilterCorpus(PrivacyFilterCorpusArgs)` to `CliCommand`, define:

```rust
#[derive(Clone, PartialEq, Eq)]
struct PrivacyFilterCorpusArgs {
    fixture_dir: PathBuf,
    runner_path: PathBuf,
    report_path: PathBuf,
    python_command: String,
}
```

Parse flags `--fixture-dir`, `--runner-path`, `--report-path`, optional `--python-command`. Require `fixture_dir.is_dir()`, `runner_path` regular file, nonblank paths, and nonblank Python command. Run `<python> <runner-path> --fixture-dir <fixture-dir> --output <report-path>` with stdout/stderr suppressed or bounded the same way as the existing Privacy Filter text wrapper. After success, read `report_path`, parse JSON, validate: `engine == fallback_synthetic_patterns`, `scope == text_only_synthetic_corpus`, `fixture_count` is positive, `fixtures` is an array, `non_goals` contains `visual_redaction`, and the serialized report does not contain raw fixture PHI strings `Jane Example`, `MRN-12345`, `jane@example.test`, `555-111-2222`. Print summary JSON with `command`, `report_path`, `engine`, `scope`, `fixture_count`, and `total_detected_span_count`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p mdid-cli cli_privacy_filter_corpus_writes_phi_safe_aggregate_summary -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Run supporting verification**

Run:

```bash
cargo test -p mdid-cli cli_privacy_filter_corpus_writes_phi_safe_aggregate_summary -- --nocapture
python -m pytest scripts/privacy_filter/test_run_privacy_filter.py -q
python scripts/privacy_filter/run_synthetic_corpus.py --fixture-dir scripts/privacy_filter/fixtures/corpus --output /tmp/privacy-filter-corpus.json
python -m json.tool /tmp/privacy-filter-corpus.json >/tmp/privacy-filter-corpus.pretty.json
! grep -E 'Jane Example|MRN-12345|jane@example.test|555-111-2222' /tmp/privacy-filter-corpus.json
git diff --check
```

Expected: all commands exit 0.

- [ ] **Step 6: Commit**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs
git commit -m "feat(cli): add privacy filter corpus wrapper"
```

### Task 2: Truth-sync README completion evidence

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Write failing docs check**

Run:

```bash
python - <<'PY'
from pathlib import Path
text = Path('README.md').read_text()
required = ['mdid-cli privacy-filter-corpus', 'text-only synthetic corpus', 'not OCR', 'not visual redaction', 'not final PDF rewrite/export']
missing = [term for term in required if term not in text]
if missing:
    raise SystemExit('missing README terms: ' + ', '.join(missing))
PY
```

Expected: FAIL until README names the CLI wrapper evidence and non-goals.

- [ ] **Step 2: Update README**

Add verification evidence for `mdid-cli privacy-filter-corpus --fixture-dir scripts/privacy_filter/fixtures/corpus --runner-path scripts/privacy_filter/run_synthetic_corpus.py --report-path <report.json>`. State it remains a text-only synthetic corpus PII-detection candidate and is not OCR, visual redaction, image pixel redaction, final PDF rewrite/export, browser UI, or desktop UI. Keep completion honest: CLI 95%, Browser/Web 93%, Desktop app 93%, Overall 95% unless controller-visible facts support a different re-baseline.

- [ ] **Step 3: Run docs and verification**

Run the docs check from Step 1, then:

```bash
cargo test -p mdid-cli cli_privacy_filter_corpus_writes_phi_safe_aggregate_summary -- --nocapture
python -m pytest scripts/privacy_filter/test_run_privacy_filter.py -q
git diff --check
```

Expected: all commands exit 0.

- [ ] **Step 4: Commit**

Run:

```bash
git add README.md
git commit -m "docs: truth-sync privacy filter corpus CLI wrapper"
```

## Self-Review

Spec coverage: Task 1 adds a bounded CLI/runtime wrapper for the already-landed text-only synthetic corpus runner. Task 2 documents evidence and non-goals without claiming OCR, visual redaction, PDF rewrite/export, browser, or desktop completion.

Placeholder scan: No TBD/TODO/fill-in placeholders remain.

Type consistency: Command name `privacy-filter-corpus`, args struct `PrivacyFilterCorpusArgs`, parser `parse_privacy_filter_corpus_args`, and runtime `run_privacy_filter_corpus` are used consistently.
