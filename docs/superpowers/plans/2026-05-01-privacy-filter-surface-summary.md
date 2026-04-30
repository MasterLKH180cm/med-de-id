# Privacy Filter Surface Summary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expose existing bounded Privacy Filter text-only JSON reports as PHI-safe Browser/Web download and Desktop save summary artifacts without running Privacy Filter in those surfaces.

**Architecture:** Mirror the existing OCR handoff summary helpers on both Browser and Desktop. Parse an existing Privacy Filter JSON report, emit an allowlisted summary containing only safe aggregate metadata and category counts, and explicitly omit `masked_text`, spans, previews, raw input text, and any unallowlisted payloads.

**Tech Stack:** Rust workspace, `serde_json`, existing `mdid-browser` and `mdid-desktop` helper/test modules, Cargo tests.

---

## File Structure

- Modify: `crates/mdid-browser/src/app.rs`
  - Add `build_privacy_filter_summary_download()` and sanitizer helpers near the existing OCR handoff summary helper.
  - Add unit tests in the existing `#[cfg(test)] mod tests`.
- Modify: `crates/mdid-desktop/src/lib.rs`
  - Add `build_privacy_filter_summary_save()` and sanitizer helpers near the existing OCR handoff summary helper.
  - Add unit tests in the existing test module.
- Modify: `README.md`
  - Truth-sync current snapshot from Browser/Web 98%, Desktop 98%, Overall 96% to Browser/Web 99%, Desktop 99%, Overall 97% only after tests and SDD reviews pass.

### Task 1: Browser Privacy Filter summary download

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`
- Test: `crates/mdid-browser/src/app.rs` test module

- [ ] **Step 1: Write the failing tests**

Add tests that call `build_privacy_filter_summary_download()` with synthetic Privacy Filter output. The expected report must include only `mode`, `engine`, `network_api_called`, `preview_policy`, `input_char_count`, `detected_span_count`, and safe uppercase category counts, and must omit `masked_text`, `spans`, `preview`, raw synthetic PHI, object-shaped metadata, and string-shaped counts.

```rust
#[test]
fn privacy_filter_summary_download_preserves_safe_aggregate_contract() {
    let report_json = json!({
        "summary": {
            "input_char_count": 86,
            "detected_span_count": 4,
            "category_counts": {"NAME": 1, "MRN": 1, "EMAIL": 1, "PHONE": 1}
        },
        "masked_text": "Patient [NAME] MRN [MRN] email [EMAIL] phone [PHONE]",
        "spans": [
            {"label": "NAME", "start": 8, "end": 20, "preview": "[NAME]"}
        ],
        "metadata": {
            "engine": "fallback_synthetic_patterns",
            "network_api_called": false,
            "preview_policy": "redacted_bracket_labels_only"
        }
    });

    let payload = build_privacy_filter_summary_download(&report_json.to_string(), Some("privacy-output.json"))
        .expect("privacy filter summary download");

    assert_eq!(payload.file_name, "privacy-output-privacy-filter-summary.json");
    assert_eq!(payload.mime_type, "application/json");
    assert!(payload.is_text);
    let summary: serde_json::Value = serde_json::from_slice(&payload.bytes).unwrap();
    assert_eq!(summary["mode"], "privacy_filter_summary");
    assert_eq!(summary["engine"], "fallback_synthetic_patterns");
    assert_eq!(summary["network_api_called"], false);
    assert_eq!(summary["preview_policy"], "redacted_bracket_labels_only");
    assert_eq!(summary["input_char_count"], 86);
    assert_eq!(summary["detected_span_count"], 4);
    assert_eq!(summary["category_counts"]["NAME"], 1);
    assert_eq!(summary["category_counts"]["MRN"], 1);
    let serialized = serde_json::to_string(&summary).unwrap();
    for forbidden in ["masked_text", "spans", "preview", "Patient Jane Example", "MRN-12345", "jane@example.com", "555-123-4567"] {
        assert!(!serialized.contains(forbidden), "leaked {forbidden}: {serialized}");
    }
}

#[test]
fn privacy_filter_summary_download_rejects_or_omits_unsafe_shapes() {
    let report_json = json!({
        "summary": {
            "input_char_count": "Patient Jane Example",
            "detected_span_count": "MRN-12345",
            "category_counts": {"NAME": 1, "Patient Jane Example": 1, "EMAIL": "jane@example.com"}
        },
        "metadata": {
            "engine": {"raw": "Patient Jane Example"},
            "network_api_called": true,
            "preview_policy": ["jane@example.com"]
        },
        "masked_text": "Patient Jane Example MRN-12345 jane@example.com 555-123-4567",
        "spans": [{"preview": "Patient Jane Example"}]
    });

    let payload = build_privacy_filter_summary_download(&report_json.to_string(), Some("unsafe.json"))
        .expect("privacy filter summary download");
    let summary: serde_json::Value = serde_json::from_slice(&payload.bytes).unwrap();
    assert!(summary.get("input_char_count").is_none());
    assert!(summary.get("detected_span_count").is_none());
    assert!(summary.get("engine").is_none());
    assert!(summary.get("preview_policy").is_none());
    assert_eq!(summary["network_api_called"], true);
    assert_eq!(summary["category_counts"], json!({"NAME": 1}));
    let serialized = serde_json::to_string(&summary).unwrap();
    for forbidden in ["Patient Jane Example", "MRN-12345", "jane@example.com", "555-123-4567"] {
        assert!(!serialized.contains(forbidden), "leaked {forbidden}: {serialized}");
    }
}
```

- [ ] **Step 2: Run test to verify RED**

Run: `cargo test -p mdid-browser privacy_filter_summary -- --nocapture`

Expected: FAIL because `build_privacy_filter_summary_download` is not defined/imported.

- [ ] **Step 3: Implement minimal Browser helper**

Add a helper that parses a top-level JSON object, creates `mode: "privacy_filter_summary"`, preserves numeric counts only, preserves metadata primitives only, preserves `category_counts` only for safe uppercase labels with integer counts, serializes pretty JSON, and uses `pdf_review_report_source_stem(imported_file_name)` for safe filename stems.

- [ ] **Step 4: Run Browser verification**

Run: `cargo test -p mdid-browser privacy_filter_summary -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Commit Browser slice**

```bash
git add crates/mdid-browser/src/app.rs
git commit -m "feat(browser): add privacy filter summary downloads"
```

### Task 2: Desktop Privacy Filter summary save

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`
- Test: `crates/mdid-desktop/src/lib.rs` test module

- [ ] **Step 1: Write the failing tests**

Add Desktop tests equivalent to Task 1, calling `build_privacy_filter_summary_save()` and expecting a save payload/default path such as `privacy-output-privacy-filter-summary.json`.

- [ ] **Step 2: Run test to verify RED**

Run: `cargo test -p mdid-desktop privacy_filter_summary -- --nocapture`

Expected: FAIL because the Desktop Privacy Filter summary helper is missing.

- [ ] **Step 3: Implement minimal Desktop helper**

Mirror the Browser sanitizer contract in `crates/mdid-desktop/src/lib.rs`, using existing Desktop save payload conventions and source-stem helper patterns.

- [ ] **Step 4: Run Desktop verification**

Run: `cargo test -p mdid-desktop privacy_filter_summary -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Commit Desktop slice**

```bash
git add crates/mdid-desktop/src/lib.rs
git commit -m "feat(desktop): add privacy filter summary saves"
```

### Task 3: README completion truth-sync

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Verify landed Browser/Desktop tests**

Run:

```bash
cargo test -p mdid-browser privacy_filter_summary -- --nocapture
cargo test -p mdid-desktop privacy_filter_summary -- --nocapture
git diff --check
```

Expected: both targeted test commands PASS and `git diff --check` prints no whitespace errors.

- [ ] **Step 2: Update README completion snapshot**

Update the top completion snapshot to state Browser/Web 99%, Desktop app 99%, Overall 97%, CLI unchanged at 95%. Explain that this is +1/+1 from 98 to the 99% threshold because existing Privacy Filter text-only report JSON can now be surfaced as PHI-safe user-facing Browser/ Desktop summary artifacts, not because Browser/Desktop execute Privacy Filter or perform OCR/visual redaction/PDF rewrite.

- [ ] **Step 3: Run README sanity checks**

Run:

```bash
git diff --check
git status --short
```

Expected: only README is dirty for this task before commit.

- [ ] **Step 4: Commit README truth-sync**

```bash
git add README.md
git commit -m "docs: truth-sync privacy filter surface summaries"
```
