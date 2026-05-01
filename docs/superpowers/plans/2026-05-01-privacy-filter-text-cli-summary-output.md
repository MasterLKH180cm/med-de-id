# Privacy Filter Text CLI Summary Output Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an optional PHI-safe aggregate summary artifact to the existing `mdid-cli privacy-filter-text` command for single text-only Privacy Filter reports.

**Architecture:** The existing CLI wrapper remains the only production code path touched: it runs the checked-in text-only Privacy Filter runner, validates the full report, writes the full validator-compatible report, and optionally writes a second strict allowlisted summary JSON after validation succeeds. The summary must be derived from the validated report, never include raw input, `masked_text`, spans, previews, paths, or caller-controlled filenames, and must preserve the text-only/non-network scope.

**Tech Stack:** Rust CLI (`crates/mdid-cli`), serde_json, existing Python Privacy Filter runner/validator fixtures, cargo tests, README truth-sync.

---

## File Structure

- Modify `crates/mdid-cli/src/main.rs`: add `--summary-output <summary.json>` parsing for `privacy-filter-text`, remove stale summary artifacts on startup/failure, derive/write strict PHI-safe summary JSON after full report validation, and keep stdout path-redacted.
- Modify `crates/mdid-cli/tests/cli_smoke.rs`: add smoke coverage for summary success and stale summary cleanup on prerequisite failure using the checked-in runner/fixture.
- Modify `README.md`: truth-sync the bounded Privacy Filter text CLI summary artifact and completion arithmetic.
- Modify `scripts/privacy_filter/README.md`: document the optional single-text summary artifact and non-goals.

### Task 1: `privacy-filter-text --summary-output` CLI artifact

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/cli_smoke.rs`

- [ ] **Step 1: Write the failing smoke test for PHI-safe summary success**

Add this test to `crates/mdid-cli/tests/cli_smoke.rs` near the existing `privacy-filter-text` smoke tests:

```rust
#[test]
fn cli_privacy_filter_text_summary_output_is_phi_safe() {
    let temp_dir = std::env::temp_dir().join(format!(
        "Jane-Example-MRN-12345-privacy-filter-summary-{}",
        unique_suffix()
    ));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let report_path = temp_dir.join("full-report.json");
    let summary_path = temp_dir.join("summary-output.json");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .arg("privacy-filter-text")
        .arg("--input-path")
        .arg("scripts/privacy_filter/fixtures/sample_text_input.txt")
        .arg("--runner-path")
        .arg("scripts/privacy_filter/run_privacy_filter.py")
        .arg("--report-path")
        .arg(&report_path)
        .arg("--summary-output")
        .arg(&summary_path)
        .arg("--mock")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout}\nstderr={stderr}");
    assert!(report_path.exists());
    assert!(summary_path.exists());

    let summary_text = std::fs::read_to_string(&summary_path).unwrap();
    let summary: serde_json::Value = serde_json::from_str(&summary_text).unwrap();
    assert_eq!(summary["artifact"], "privacy_filter_text_summary");
    assert_eq!(summary["engine"], "fallback_synthetic_patterns");
    assert_eq!(summary["network_api_called"], false);
    assert_eq!(summary["preview_policy"], "redacted_bracket_labels_only");
    assert!(summary["input_char_count"].as_u64().unwrap() > 0);
    assert!(summary["detected_span_count"].as_u64().unwrap() > 0);
    assert!(summary["category_counts"].is_object());
    assert_eq!(summary["non_goals"].as_array().unwrap().len(), 6);

    for forbidden in [
        "Jane Example",
        "jane@example.com",
        "+1-555-123-4567",
        "555-123-4567",
        "MRN-12345",
        "masked_text",
        "spans",
        "preview",
        temp_dir.to_string_lossy().as_ref(),
    ] {
        assert!(!summary_text.contains(forbidden), "summary leaked {forbidden}");
        assert!(!stdout.contains(forbidden), "stdout leaked {forbidden}");
        assert!(!stderr.contains(forbidden), "stderr leaked {forbidden}");
    }
}
```

- [ ] **Step 2: Run RED for the summary success test**

Run:

```bash
cargo test -p mdid-cli cli_privacy_filter_text_summary_output_is_phi_safe -- --nocapture
```

Expected: FAIL because `privacy-filter-text` does not yet accept `--summary-output` or write the summary artifact.

- [ ] **Step 3: Implement minimal parser/data model support**

In `PrivacyFilterTextArgs`, add:

```rust
summary_output: Option<PathBuf>,
```

In `parse_privacy_filter_text_args`, parse:

```rust
"--summary-output" => {
    summary_output = Some(PathBuf::from(require_value(&mut iter, "missing summary output")?));
}
```

Initialize `let mut summary_output: Option<PathBuf> = None;` and include `summary_output` when constructing `PrivacyFilterTextArgs`.

- [ ] **Step 4: Implement minimal summary writer and cleanup**

In `run_privacy_filter_text`, remove stale summary output before prerequisite checks and remove it on failure:

```rust
if let Some(summary_output) = &args.summary_output {
    let _ = fs::remove_file(summary_output);
}
```

Also remove it inside the existing `if result.is_err()` cleanup block.

After writing the full report in `run_privacy_filter_text_inner`, add:

```rust
if let Some(summary_output) = &args.summary_output {
    let summary_report = build_privacy_filter_text_summary(&value)?;
    let summary_text = serde_json::to_string_pretty(&summary_report)
        .map_err(|err| format!("failed to render privacy filter summary: {err}"))?;
    fs::write(summary_output, summary_text)
        .map_err(|err| format!("failed to write privacy filter summary: {err}"))?;
}
```

Add helper:

```rust
fn build_privacy_filter_text_summary(value: &Value) -> Result<Value, String> {
    validate_privacy_filter_output(value)?;
    let summary = value
        .get("summary")
        .and_then(Value::as_object)
        .ok_or_else(|| "privacy filter report missing summary".to_string())?;
    let metadata = value
        .get("metadata")
        .and_then(Value::as_object)
        .ok_or_else(|| "privacy filter report missing metadata".to_string())?;
    let category_counts = summary
        .get("category_counts")
        .filter(|counts| validate_privacy_filter_text_category_counts(counts))
        .ok_or_else(|| "privacy filter report has invalid category counts".to_string())?;
    Ok(json!({
        "artifact": "privacy_filter_text_summary",
        "scope": "text_only_single_report_summary",
        "engine": metadata.get("engine").cloned().unwrap_or(Value::Null),
        "network_api_called": metadata.get("network_api_called").cloned().unwrap_or(Value::Null),
        "preview_policy": metadata.get("preview_policy").cloned().unwrap_or(Value::Null),
        "input_char_count": summary.get("input_char_count").cloned().unwrap_or(Value::Null),
        "detected_span_count": summary.get("detected_span_count").cloned().unwrap_or(Value::Null),
        "category_counts": category_counts.clone(),
        "non_goals": [
            "ocr",
            "visual_redaction",
            "image_pixel_redaction",
            "final_pdf_rewrite_export",
            "browser_ui",
            "desktop_ui"
        ]
    }))
}
```

Use the existing known safe category validator or add `ID` if the single-text validator already permits it:

```rust
fn validate_privacy_filter_text_category_counts(value: &Value) -> bool {
    let Some(counts) = value.as_object() else { return false; };
    let allowed_labels = ["NAME", "MRN", "EMAIL", "PHONE", "ID"];
    counts.iter().all(|(label, count)| allowed_labels.contains(&label.as_str()) && count.as_u64().is_some())
}
```

- [ ] **Step 5: Run GREEN for the summary success test**

Run:

```bash
cargo test -p mdid-cli cli_privacy_filter_text_summary_output_is_phi_safe -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Write the failing stale-summary cleanup test**

Add this test to `crates/mdid-cli/tests/cli_smoke.rs`:

```rust
#[test]
fn cli_privacy_filter_text_summary_output_removes_stale_file_on_prerequisite_failure() {
    let temp_dir = std::env::temp_dir().join(format!(
        "Jane-Example-MRN-12345-stale-privacy-filter-summary-{}",
        unique_suffix()
    ));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let report_path = temp_dir.join("full-report.json");
    let summary_path = temp_dir.join("summary-output.json");
    std::fs::write(&summary_path, "Jane Example stale summary").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .arg("privacy-filter-text")
        .arg("--input-path")
        .arg(temp_dir.join("missing-input.txt"))
        .arg("--runner-path")
        .arg("scripts/privacy_filter/run_privacy_filter.py")
        .arg("--report-path")
        .arg(&report_path)
        .arg("--summary-output")
        .arg(&summary_path)
        .arg("--mock")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(!summary_path.exists(), "stale summary was not removed");
    assert!(!report_path.exists(), "report should not be written on prerequisite failure");
    for forbidden in ["Jane Example", "MRN-12345", temp_dir.to_string_lossy().as_ref()] {
        assert!(!stdout.contains(forbidden), "stdout leaked {forbidden}");
        assert!(!stderr.contains(forbidden), "stderr leaked {forbidden}");
    }
}
```

- [ ] **Step 7: Run RED/GREEN for stale cleanup**

Run:

```bash
cargo test -p mdid-cli cli_privacy_filter_text_summary_output_removes_stale_file_on_prerequisite_failure -- --nocapture
```

Expected after Step 4 cleanup: PASS. If it fails because cleanup happens after prerequisite checks, move cleanup before `require_regular_file` and re-run.

- [ ] **Step 8: Run targeted and broader CLI verification**

Run:

```bash
cargo test -p mdid-cli privacy_filter_text -- --nocapture
cargo test -p mdid-cli cli_privacy_filter_text_summary_output -- --nocapture
cargo fmt --check
git diff --check
```

Expected: all pass.

- [ ] **Step 9: Commit Task 1**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs
git commit -m "feat(cli): add privacy filter text summary output"
```

### Task 2: Documentation and completion truth-sync

**Files:**
- Modify: `README.md`
- Modify: `scripts/privacy_filter/README.md`

- [ ] **Step 1: Update local Privacy Filter CLI docs**

In `scripts/privacy_filter/README.md`, add this paragraph near the CLI wrapper section:

```markdown
`mdid-cli privacy-filter-text` also supports an optional `--summary-output <summary.json>` artifact for single text-only reports. The summary is written only after the full report validates, is derived from the validated report, and contains only allowlisted aggregate fields: `artifact`, `scope`, `engine`, `network_api_called`, `preview_policy`, `input_char_count`, `detected_span_count`, safe `category_counts`, and explicit non-goals. It omits raw input text, `masked_text`, spans, previews, local paths, OCR output, visual redaction, image pixel redaction, browser UI, desktop UI, and final PDF rewrite/export semantics.
```

- [ ] **Step 2: Update top-level README completion evidence**

In `README.md`, update the completion snapshot sentence and add a new evidence paragraph after the current Privacy Filter text stdout hardening paragraph:

```markdown
A follow-up CLI single-report summary artifact slice now adds `mdid-cli privacy-filter-text --summary-output <summary.json>`. The optional summary is derived only after the full text-only Privacy Filter report validates, removes stale summary artifacts on prerequisite/failure paths, and writes an aggregate PHI-safe JSON artifact with `artifact: privacy_filter_text_summary`, bounded engine/network/count metadata, safe category counts, and explicit non-goals. It omits raw input, `masked_text`, spans, previews, local paths, OCR output, visual redaction, image pixel redaction, browser UI, desktop UI, and final PDF rewrite/export semantics. This adds and completes one CLI/runtime artifact requirement in the same round; fraction accounting is CLI `96/101 -> 97/102 = 95%` floor, Browser/Web `99%`, Desktop app `99%`, Overall `97%`.
```

Do not increase Browser/Web or Desktop app completion.

- [ ] **Step 3: Run docs verification**

Run:

```bash
git diff --check
```

Expected: PASS.

- [ ] **Step 4: Commit Task 2**

Run:

```bash
git add README.md scripts/privacy_filter/README.md
git commit -m "docs: truth-sync privacy filter text summary output"
```

### Final Integration Verification

- [ ] **Step 1: Run final verification**

Run:

```bash
cargo test -p mdid-cli cli_privacy_filter_text_summary_output -- --nocapture
cargo test -p mdid-cli privacy_filter_text -- --nocapture
cargo fmt --check
git diff --check
```

Expected: all pass and working tree clean except intentional commits.

- [ ] **Step 2: Merge back to develop**

Run:

```bash
git checkout develop
git merge --no-ff feat/privacy-filter-text-summary-output-cron-0813 -m "merge: privacy filter text summary output"
```

- [ ] **Step 3: Verify develop**

Run:

```bash
git status --short
git log --oneline -8
```

Expected: clean develop with the merge commit visible.
