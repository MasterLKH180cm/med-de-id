# Privacy Filter IP Address Detection Implementation Plan

> **For implementation workers:** REQUIRED SUB-SKILL: Use subagent-driven-development (recommended) or executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add bounded text-only IPv4 address detection to the Privacy Filter CLI/runtime synthetic PII detector and keep README completion accounting truthful.

**Architecture:** Extend the existing deterministic `scripts/privacy_filter/run_privacy_filter.py` fallback detector with an `IP_ADDRESS` label for syntactically valid IPv4 addresses only. Align Python validator, Rust CLI allowlist, CLI smoke coverage, and README completion arithmetic in one coherent CLI/runtime-only slice; do not claim OCR, visual redaction, image pixel redaction, handwriting recognition, Browser/Desktop execution, or PDF rewrite/export.

**Tech Stack:** Python 3 scripts/tests, Rust `mdid-cli`, Cargo tests, repository README completion truth-sync.

---

## File Structure

- Modify `scripts/privacy_filter/run_privacy_filter.py`: add bounded IPv4 regex, octet validation, `IP_ADDRESS` span emission, and allowlist entry.
- Modify `scripts/privacy_filter/validate_privacy_filter_output.py`: add `IP_ADDRESS` to strict output label allowlist.
- Modify `scripts/privacy_filter/test_run_privacy_filter.py`: add positive/negative TDD coverage for IPv4 detection and overbroad cases.
- Modify `crates/mdid-cli/src/main.rs`: add `IP_ADDRESS` to `is_allowed_privacy_filter_label` so CLI wrapper validation accepts the new category.
- Modify `crates/mdid-cli/tests/cli_smoke.rs`: add stdin smoke coverage that proves `mdid-cli privacy-filter-text --stdin --mock` detects `IP_ADDRESS` and leaks no raw IP/path/PHI to stdout, stderr, or report.
- Modify `README.md`: truth-sync current repository status and completion fraction accounting for one new completed CLI/runtime text-only PII detection requirement.

### Task 1: Python Privacy Filter IPv4 detection

**Files:**
- Modify: `scripts/privacy_filter/run_privacy_filter.py`
- Modify: `scripts/privacy_filter/validate_privacy_filter_output.py`
- Test: `scripts/privacy_filter/test_run_privacy_filter.py`

- [x] **Step 1: Write failing Python tests for IPv4 detection and bounds**

Append these tests to `scripts/privacy_filter/test_run_privacy_filter.py` inside the main test class that already exercises `heuristic_detect`:

```python
    def test_detects_context_free_ipv4_address_as_ip_address(self):
        payload = run_privacy_filter_text('Patient Jane Example remote login from 192.168.10.42 for MRN-12345')

        self.assertEqual(payload['summary']['category_counts'].get('IP_ADDRESS'), 1)
        self.assertIn('[IP_ADDRESS]', payload['masked_text'])
        self.assertNotIn('192.168.10.42', payload['masked_text'])
        ip_spans = [span for span in payload['spans'] if span['label'] == 'IP_ADDRESS']
        self.assertEqual(len(ip_spans), 1)
        self.assertEqual(ip_spans[0]['preview'], '<redacted>')

    def test_rejects_invalid_or_embedded_ipv4_like_tokens(self):
        payload = run_privacy_filter_text(
            'Ignore 999.168.10.42 and host192.168.10.42 and 192.168.10.42extra and 1.2.3'
        )

        self.assertNotIn('IP_ADDRESS', payload['summary']['category_counts'])
        self.assertNotIn('[IP_ADDRESS]', payload['masked_text'])
```

- [x] **Step 2: Run tests to verify RED**

Run: `python -m pytest scripts/privacy_filter/test_run_privacy_filter.py -q`
Expected: FAIL because `IP_ADDRESS` is not detected yet.

- [x] **Step 3: Implement minimal IPv4 detection and validator allowlist**

In `scripts/privacy_filter/run_privacy_filter.py`, add near other regex constants:

```python
IP_ADDRESS_RE = re.compile(r'(?<![A-Za-z0-9.])(?:\d{1,3}\.){3}\d{1,3}(?![A-Za-z0-9.])')
```

Add `IP_ADDRESS` to `ALLOWED_LABELS`.

Add helper:

```python
def _is_valid_ipv4(value: str) -> bool:
    parts = value.split('.')
    if len(parts) != 4:
        return False
    for part in parts:
        if not part.isdigit():
            return False
        if len(part) > 1 and part.startswith('0'):
            return False
        if int(part) > 255:
            return False
    return True
```

In `heuristic_detect`, after phone/date/SSN detection and before broad numeric identifiers, add:

```python
    for m in IP_ADDRESS_RE.finditer(text):
        if _is_valid_ipv4(m.group(0)):
            add_span(spans, 'IP_ADDRESS', m.start(), m.end())
```

In `scripts/privacy_filter/validate_privacy_filter_output.py`, add `IP_ADDRESS` to `ALLOWED_LABELS`.

- [x] **Step 4: Run Python tests to verify GREEN**

Run: `python -m pytest scripts/privacy_filter/test_run_privacy_filter.py -q`
Expected: PASS.

- [x] **Step 5: Verify validator accepts generated IP_ADDRESS output**

Run:

```bash
python - <<'PY' >/tmp/privacy-filter-ip-address.json
import json
from scripts.privacy_filter.run_privacy_filter import heuristic_detect
print(json.dumps(heuristic_detect('Patient Jane Example accessed from 192.168.10.42 MRN-12345')))
PY
python scripts/privacy_filter/validate_privacy_filter_output.py /tmp/privacy-filter-ip-address.json
```

Expected: validator exits 0 and prints no raw PHI.

- [x] **Step 6: Commit Python slice**

Run:

```bash
git add scripts/privacy_filter/run_privacy_filter.py scripts/privacy_filter/validate_privacy_filter_output.py scripts/privacy_filter/test_run_privacy_filter.py
git commit -m "feat(privacy-filter): detect IPv4 address text spans"
```

### Task 2: Rust CLI contract alignment and smoke test

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/cli_smoke.rs`

- [x] **Step 1: Write failing CLI smoke test**

Add a test near the other `privacy_filter_text` category smoke tests in `crates/mdid-cli/tests/cli_smoke.rs`:

```rust
#[test]
fn privacy_filter_text_detects_ip_address_from_stdin_without_raw_ip_leaks() {
    let temp_dir = tempdir().expect("tempdir");
    let report_path = temp_dir.path().join("ip-address-report.json");
    let input = "Patient Jane Example accessed from 192.168.10.42 with MRN-12345";

    let mut cmd = Command::new(mdid_cli_bin());
    cmd.args([
        "privacy-filter-text",
        "--stdin",
        "--runner-path",
        "scripts/privacy_filter/run_privacy_filter.py",
        "--report-path",
    ])
    .arg(&report_path)
    .arg("--python-command")
    .arg(default_python_command())
    .arg("--mock")
    .write_stdin(input);

    let output = cmd.output().expect("run privacy-filter-text");
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.is_empty(), "stderr should be empty on success: {stderr}");
    assert!(stdout.contains("\"report_path\":\"<redacted>\""));
    assert!(!stdout.contains("192.168.10.42"));
    assert!(!stdout.contains("Jane Example"));
    assert!(!stdout.contains("MRN-12345"));

    let report_text = fs::read_to_string(&report_path).expect("report");
    assert!(!report_text.contains("192.168.10.42"));
    assert!(!report_text.contains("Jane Example"));
    assert!(!report_text.contains("MRN-12345"));
    let report: Value = serde_json::from_str(&report_text).expect("json report");
    assert_eq!(report["summary"]["category_counts"]["IP_ADDRESS"], 1);
    assert!(report["masked_text"].as_str().unwrap().contains("[IP_ADDRESS]"));
    for span in report["spans"].as_array().unwrap() {
        assert_eq!(span["preview"], "<redacted>");
    }
}
```

- [x] **Step 2: Run CLI test to verify RED**

Run: `cargo test -p mdid-cli privacy_filter_text_detects_ip_address_from_stdin_without_raw_ip_leaks --test cli_smoke -- --nocapture`
Expected: FAIL because Rust `is_allowed_privacy_filter_label` rejects `IP_ADDRESS`.

- [x] **Step 3: Add Rust allowlist entry**

In `crates/mdid-cli/src/main.rs`, add `"IP_ADDRESS"` to `is_allowed_privacy_filter_label` next to the other Privacy Filter labels.

- [x] **Step 4: Run CLI test to verify GREEN**

Run: `cargo test -p mdid-cli privacy_filter_text_detects_ip_address_from_stdin_without_raw_ip_leaks --test cli_smoke -- --nocapture`
Expected: PASS.

- [x] **Step 5: Run broader Privacy Filter CLI regression gate**

Run: `cargo test -p mdid-cli privacy_filter_text --test cli_smoke -- --nocapture`
Expected: PASS.

- [x] **Step 6: Commit Rust CLI alignment**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs
git commit -m "test(cli): accept privacy filter IP address category"
```

### Task 3: README completion truth-sync and final verification

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/plans/2026-05-02-privacy-filter-ip-address-detection.md`

- [x] **Step 1: Update README completion snapshot**

Edit `README.md` current status to state this round adds and completes one CLI/runtime text-only PII detection requirement: `IP_ADDRESS`. Keep Browser/Web at 99%, Desktop app at 99%, Overall at 97%, and CLI at 96% unless fraction arithmetic visibly crosses the next integer threshold. Add explicit fraction accounting: prior CLI `131/136`, new requirement added and completed `132/137`, conservative floor remains `96%`. State Browser/Web +5%: FAIL/not claimed and Desktop +5%: FAIL/not claimed because this is CLI/runtime-only text detection.

Add a verification evidence paragraph:

```markdown
Verification evidence for the `mdid-cli privacy-filter-text` IP address detection slice landed on this branch: the bounded local text-only Privacy Filter runner now detects syntactically valid IPv4 addresses such as `192.168.10.42` as `IP_ADDRESS`, masks them as `[IP_ADDRESS]`, emits only `<redacted>` span previews, and keeps `metadata.network_api_called: false`. The detector rejects invalid or embedded IPv4-like tokens such as `999.168.10.42`, `host192.168.10.42`, and `192.168.10.42extra`. The Rust CLI summary/category validation now accepts `IP_ADDRESS`, and stdin smoke coverage proves stdout, stderr, and report contents do not leak the raw IP address, synthetic names, MRNs, report paths, or temp directories. Repository-visible verification: `python -m pytest scripts/privacy_filter/test_run_privacy_filter.py -q`, `python scripts/privacy_filter/validate_privacy_filter_output.py /tmp/privacy-filter-ip-address.json`, and `cargo test -p mdid-cli privacy_filter_text --test cli_smoke -- --nocapture`.
```

- [x] **Step 2: Truth-sync plan checkboxes**

Mark completed steps in this plan with `- [x]` after successful implementation and verification.

- [x] **Step 3: Run final verification**

Run:

```bash
python -m pytest scripts/privacy_filter/test_run_privacy_filter.py -q
python - <<'PY' >/tmp/privacy-filter-ip-address.json
import json
from scripts.privacy_filter.run_privacy_filter import heuristic_detect
print(json.dumps(heuristic_detect('Patient Jane Example accessed from 192.168.10.42 MRN-12345')))
PY
python scripts/privacy_filter/validate_privacy_filter_output.py /tmp/privacy-filter-ip-address.json
cargo test -p mdid-cli privacy_filter_text --test cli_smoke -- --nocapture
git diff --check
```

Expected: all commands PASS.

- [x] **Step 4: Commit docs truth-sync**

Run:

```bash
git add README.md docs/superpowers/plans/2026-05-02-privacy-filter-ip-address-detection.md
git commit -m "docs: truth sync privacy filter IP address detection"
```

- [x] **Step 5: Final branch status**

Run:

```bash
git status --short
git log --oneline -5
```

Expected: clean worktree and latest commits include the IP address detection implementation, CLI alignment, and docs truth-sync.

---

## Self-Review

- Spec coverage: Python runner, validator, Rust CLI allowlist, CLI smoke, README completion arithmetic, and final verification are all covered.
- Placeholder scan: no TBD/TODO/fill-in placeholders remain.
- Type consistency: category label is consistently `IP_ADDRESS`; summary count path is consistently `summary.category_counts.IP_ADDRESS`; span preview remains `<redacted>`.
