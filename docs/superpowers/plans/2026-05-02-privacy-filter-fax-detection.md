# Privacy Filter FAX Detection Implementation Plan

> **For implementation workers:** REQUIRED SUB-SKILL: Use subagent-driven-development (recommended) or executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add bounded text-only FAX number detection to the Privacy Filter CLI/runtime POC without claiming OCR, visual redaction, Browser/Web execution, Desktop execution, or PDF rewrite/export.

**Architecture:** Extend the deterministic local `scripts/privacy_filter/run_privacy_filter.py` fallback detector with a narrow context-required `FAX` category, align validator and Rust CLI allowlists, and lock the new category with Python and CLI smoke tests. Update README completion truth-sync as a CLI/runtime-only requirement added to both numerator and denominator while Browser/Web and Desktop remain capped at 99% with no new surface capability.

**Tech Stack:** Python 3 Privacy Filter runner/validator, Rust `mdid-cli` smoke tests, pytest/unittest, cargo test.

---

## File Structure

- Modify `scripts/privacy_filter/run_privacy_filter.py`: add `FAX_RE`, emit `FAX` spans before generic `PHONE` spans, add `FAX` to `ALLOWED_LABELS`.
- Modify `scripts/privacy_filter/validate_privacy_filter_output.py`: add `FAX` to the strict label allowlist.
- Modify `scripts/privacy_filter/test_run_privacy_filter.py`: add positive and negative fallback detector tests for FAX numbers.
- Modify `crates/mdid-cli/src/main.rs`: add `FAX` to the Rust Privacy Filter label allowlist used by `mdid-cli privacy-filter-text` validation.
- Modify `crates/mdid-cli/tests/cli_smoke.rs`: add a stdin smoke test proving `FAX` is masked and raw FAX/PHI/path values do not leak to stdout, stderr, or report.
- Modify `README.md`: truth-sync the current round as a CLI/runtime-only FAX detection requirement, with completion fraction adding one requirement to both numerator and denominator and integer completion remaining at the 99% target cap.

### Task 1: Python Privacy Filter FAX detector and validator

**Files:**
- Modify: `scripts/privacy_filter/test_run_privacy_filter.py`
- Modify: `scripts/privacy_filter/run_privacy_filter.py`
- Modify: `scripts/privacy_filter/validate_privacy_filter_output.py`

- [x] **Step 1: Write the failing Python tests**

Add these tests to `scripts/privacy_filter/test_run_privacy_filter.py` near the other category detector tests:

```python
    def test_fallback_detects_context_required_fax_numbers(self):
        text = 'Patient Jane Example fax 555-222-3333 and fax: (555) 444-5555.'
        payload = run_privacy_filter_payload(text)

        self.assertEqual(payload['summary']['category_counts'].get('FAX'), 2)
        self.assertEqual(payload['masked_text'].count('[FAX]'), 2)
        self.assertNotIn('555-222-3333', payload['masked_text'])
        self.assertNotIn('(555) 444-5555', payload['masked_text'])
        fax_spans = [span for span in payload['spans'] if span['label'] == 'FAX']
        self.assertEqual(len(fax_spans), 2)
        self.assertTrue(all(span['preview'] == '<redacted>' for span in fax_spans))
        self.assertFalse(payload['metadata']['network_api_called'])

    def test_fallback_does_not_classify_plain_phone_or_overlong_fax_as_fax(self):
        text = 'Phone 555-222-3333. fax 555-222-333333. ID555-222-3333'
        payload = run_privacy_filter_payload(text)

        self.assertNotIn('FAX', payload['summary']['category_counts'])
        self.assertNotIn('[FAX]', payload['masked_text'])
```

- [x] **Step 2: Run the focused tests and verify RED**

Run:

```bash
python3 -m pytest scripts/privacy_filter/test_run_privacy_filter.py -q -k fax
```

Expected: FAIL because `FAX` is not detected yet and/or not accepted by the validator.

- [x] **Step 3: Implement minimal Python detector and validator allowlist**

In `scripts/privacy_filter/run_privacy_filter.py`, add the new regex near the phone regexes:

```python
FAX_RE = re.compile(r'\b(?:fax|facsimile)(?:\s+(?:number|no\.))?\s*:?\s*((?:\+\d{1,3}[-.\s]?)?(?:\d{3}[-.]\d{3}[-.]\d{4}|\(\d{3}\)\s?\d{3}[-.]\d{4}))(?![A-Za-z0-9-])', re.I)
```

Update `ALLOWED_LABELS` in both Python files to include `FAX`:

```python
ALLOWED_LABELS = {'NAME', 'MRN', 'EMAIL', 'PHONE', 'FAX', 'ID', 'DATE', 'ADDRESS', 'SSN', 'PASSPORT', 'ZIP', 'INSURANCE_ID', 'AGE', 'FACILITY', 'NPI', 'LICENSE_PLATE', 'IP_ADDRESS', 'URL'}
```

In `heuristic_detect`, before `PHONE_OVERLONG_EXTENSION_RE`, add:

```python
    for m in FAX_RE.finditer(text):
        add_span(spans, 'FAX', m.start(1), m.end(1))
        occupied_phone_ranges.append((m.start(1), m.end(1)))
```

Keep detection text-only and deterministic. Do not add OCR, Browser/Web, Desktop, PDF, or network behavior.

- [x] **Step 4: Run Python tests and validators**

Run:

```bash
python3 -m pytest scripts/privacy_filter/test_run_privacy_filter.py -q -k fax
python3 scripts/privacy_filter/run_privacy_filter.py --mock --stdin <<'EOF' > /tmp/privacy-filter-fax.json
Patient Jane Example fax 555-222-3333 MRN MRN-12345
EOF
python3 scripts/privacy_filter/validate_privacy_filter_output.py /tmp/privacy-filter-fax.json
python3 -m py_compile scripts/privacy_filter/run_privacy_filter.py scripts/privacy_filter/validate_privacy_filter_output.py
```

Expected: all commands PASS, and `/tmp/privacy-filter-fax.json` validates.

- [x] **Step 5: Commit Task 1**

```bash
git add scripts/privacy_filter/run_privacy_filter.py scripts/privacy_filter/validate_privacy_filter_output.py scripts/privacy_filter/test_run_privacy_filter.py
git commit -m "feat(privacy-filter): detect bounded fax numbers"
```

### Task 2: Rust CLI validation and smoke coverage

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`

- [x] **Step 1: Write the failing CLI smoke test**

Add this test near the other `privacy-filter-text` category smoke tests in `crates/mdid-cli/tests/cli_smoke.rs`:

```rust
#[test]
fn cli_privacy_filter_text_detects_fax_without_phi_or_path_leaks() {
    let dir = tempdir().unwrap();
    let phi_named_dir = dir.path().join("Jane-Example-MRN-12345-fax");
    fs::create_dir(&phi_named_dir).unwrap();
    let report_path = phi_named_dir.join("fax-report.json");
    let raw_fax = "555-222-3333";
    let input = format!("Patient Jane Example fax {raw_fax} MRN MRN-12345");

    let output = Command::cargo_bin("mdid-cli")
        .unwrap()
        .args([
            "privacy-filter-text",
            "--stdin",
            "--runner-path",
            &repo_path("scripts/privacy_filter/run_privacy_filter.py"),
            "--python-command",
            default_python_command(),
            "--report-path",
            report_path.to_str().unwrap(),
            "--mock",
        ])
        .write_stdin(input)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.is_empty());
    assert!(stdout.contains("\"report_path\":\"<redacted>\""));
    assert!(!stdout.contains(raw_fax));
    assert!(!stdout.contains("Jane Example"));
    assert!(!stdout.contains("MRN-12345"));
    assert!(!stdout.contains(report_path.to_str().unwrap()));
    assert!(!stdout.contains(phi_named_dir.to_str().unwrap()));

    let report_text = fs::read_to_string(&report_path).unwrap();
    assert!(!report_text.contains(raw_fax));
    assert!(!report_text.contains("Jane Example"));
    assert!(!report_text.contains("MRN-12345"));
    assert!(!report_text.contains(report_path.to_str().unwrap()));
    assert!(!report_text.contains(phi_named_dir.to_str().unwrap()));
    let report_json: Value = serde_json::from_str(&report_text).unwrap();
    assert_eq!(report_json["summary"]["category_counts"]["FAX"], 1);
    assert!(report_json["masked_text"].as_str().unwrap().contains("[FAX]"));
    assert_eq!(report_json["metadata"]["network_api_called"], false);
    for span in report_json["spans"].as_array().unwrap() {
        if span["label"] == "FAX" {
            assert_eq!(span["preview"], "<redacted>");
        }
    }
}
```

- [x] **Step 2: Run the focused CLI smoke and verify RED**

Run:

```bash
/home/azureuser/.cargo/bin/cargo test -p mdid-cli cli_privacy_filter_text_detects_fax_without_phi_or_path_leaks --test cli_smoke -- --nocapture
```

Expected: FAIL before Rust allowlist alignment because the CLI rejects unsupported `FAX` labels or the test is not yet implemented.

- [x] **Step 3: Add `FAX` to Rust Privacy Filter label allowlist**

Find `fn is_allowed_privacy_filter_label` in `crates/mdid-cli/src/main.rs` and add `"FAX"` to the matches list:

```rust
matches!(
    label,
    "NAME"
        | "MRN"
        | "EMAIL"
        | "PHONE"
        | "FAX"
        | "ID"
        | "DATE"
        | "ADDRESS"
        | "SSN"
        | "PASSPORT"
        | "ZIP"
        | "INSURANCE_ID"
        | "AGE"
        | "FACILITY"
        | "NPI"
        | "LICENSE_PLATE"
        | "IP_ADDRESS"
        | "URL"
)
```

- [x] **Step 4: Run focused and category-broad CLI tests**

Run:

```bash
/home/azureuser/.cargo/bin/cargo test -p mdid-cli cli_privacy_filter_text_detects_fax_without_phi_or_path_leaks --test cli_smoke -- --nocapture
/home/azureuser/.cargo/bin/cargo test -p mdid-cli privacy_filter_text --test cli_smoke -- --nocapture
/home/azureuser/.cargo/bin/cargo fmt --check
git diff --check
```

Expected: all commands PASS.

- [x] **Step 5: Commit Task 2**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs
git commit -m "test(cli): accept privacy filter fax category"
```

### Task 3: README completion truth-sync and final verification

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/plans/2026-05-02-privacy-filter-fax-detection.md`

- [x] **Step 1: Update README current status**

In `README.md`, update the completion snapshot paragraph and CLI status/evidence text to mention bounded `FAX` detection as a CLI/runtime text-only Privacy Filter requirement. Use conservative fraction accounting:

- Previous CLI fraction: `133/138 = 96% floor`, capped in README as `99%` target threshold.
- New requirement added: bounded context-required `FAX` detection.
- New fraction after completion: `134/139 = 96% floor`, still capped in README as `99%` because the repository has already reached the explicit 99% target threshold.
- Browser/Web: unchanged at 99%; `+0%`, `+5% rule FAIL/not claimed because this is CLI/runtime-only and already at target cap`.
- Desktop app: unchanged at 99%; `+0%`, `+5% rule FAIL/not claimed because this is CLI/runtime-only and already at target cap`.
- Overall: unchanged at 99%; remaining 1% reserved for external real-model quality benchmarking, distribution packaging outside this repo, and field validation.

Add an evidence paragraph:

```markdown
Verification evidence for the `mdid-cli privacy-filter-text` FAX detection slice landed on this branch: the bounded local text-only Privacy Filter runner now detects explicitly contextual fax numbers such as `fax 555-222-3333` and `fax: (555) 444-5555` as `FAX`, masks them as `[FAX]`, emits only `<redacted>` span previews, and keeps `metadata.network_api_called: false`. The detector requires fax/facsimile context and does not classify plain phone numbers or overlong fax-like values as `FAX`. Rust CLI validation now accepts `FAX`, and stdin smoke coverage proves stdout, stderr, and report contents do not leak the raw fax number, synthetic patient name/MRN, report path, or PHI-bearing temp directory. This is CLI/runtime text-only Privacy Filter evidence only; it is not OCR, visual redaction, image pixel redaction, Browser/Web execution, Desktop execution, or final PDF rewrite/export.
```

- [x] **Step 2: Mark this implementation plan complete**

Change all task checkboxes in this plan from open to checked after code, tests, and README verification land.

- [x] **Step 3: Run final verification**

Run:

```bash
python3 -m pytest scripts/privacy_filter/test_run_privacy_filter.py -q -k 'fax or phone_extension or ip_address or url'
python3 scripts/privacy_filter/run_privacy_filter.py --mock --stdin <<'EOF' > /tmp/privacy-filter-fax.json
Patient Jane Example fax 555-222-3333 MRN MRN-12345
EOF
python3 scripts/privacy_filter/validate_privacy_filter_output.py /tmp/privacy-filter-fax.json
/home/azureuser/.cargo/bin/cargo test -p mdid-cli privacy_filter_text --test cli_smoke -- --nocapture
/home/azureuser/.cargo/bin/cargo fmt --check
git diff --check
rm -rf scripts/privacy_filter/__pycache__ tests/__pycache__
git status --short
```

Expected: all tests/checks PASS and only intentional tracked changes remain before the docs commit.

- [ ] **Step 4: Commit Task 3**

```bash
git add README.md docs/superpowers/plans/2026-05-02-privacy-filter-fax-detection.md
git commit -m "docs: truth-sync privacy filter fax detection"
```

- [ ] **Step 5: Merge to develop and push**

```bash
git checkout develop
git merge --ff-only feat/privacy-filter-fax-detection
git push origin develop
```

Expected: `develop` fast-forwards to include all three commits. If push or auth fails, leave the branch committed locally and report the exact failure.

## Self-Review

- Spec coverage: The plan implements bounded text-only FAX detection, validator alignment, Rust CLI validation, smoke tests, README completion truth-sync, final verification, and merge/push.
- Placeholder scan: No TBD/TODO/placeholders remain; code snippets, paths, commands, and expected outcomes are explicit.
- Type/signature consistency: New label is consistently `FAX` across Python runner, Python validator, Rust CLI allowlist, tests, README, and completion accounting.
