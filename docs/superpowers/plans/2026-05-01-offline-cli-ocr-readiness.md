# Offline CLI OCR Readiness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded CLI readiness report tying together text-only Privacy Filter and PP-OCRv5 mobile printed-text OCR spike prerequisites without executing OCR, calling a network API, or claiming Browser/Desktop/visual-redaction/PDF rewrite completion.

**Architecture:** Extend `mdid-cli` with an aggregate-only `offline-readiness` command that validates required local runner/fixture files and prints a PHI-safe JSON capability report. Keep the report limited to readiness metadata and non-goals so it supports the Privacy Filter/OCR mainline without adding agent/controller/orchestration semantics or new Browser/Desktop capability claims.

**Tech Stack:** Rust CLI (`crates/mdid-cli`), serde_json, assert_cmd integration tests, Cargo test runner.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `CliCommand::OfflineReadiness`, argument parsing, usage text, and report builder.
  - Report must omit local paths, fixture names, raw OCR text, normalized text, spans, previews, bbox/image data, and raw synthetic PHI.
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
  - Add integration coverage that runs the command against checked-in Privacy Filter/OCR fixture files and verifies PHI/path-safe output.
- Create: `docs/superpowers/plans/2026-05-01-offline-cli-ocr-readiness.md`
  - Record the actual bounded task under SDD review.

### Task 1: CLI Offline Readiness Aggregate Report

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
- Create: `docs/superpowers/plans/2026-05-01-offline-cli-ocr-readiness.md`

- [ ] **Step 1: Write the failing parser/unit tests**

Add these tests to `crates/mdid-cli/src/main.rs` under `mod tests`:

```rust
#[test]
fn parses_offline_readiness_command_for_cli_opf_and_ocr_evidence() {
    let command = parse_command(&[
        "offline-readiness".to_string(),
        "--privacy-runner-path".to_string(),
        "scripts/privacy_filter/run_privacy_filter.py".to_string(),
        "--ocr-runner-path".to_string(),
        "scripts/ocr_eval/run_small_ocr.py".to_string(),
        "--ocr-fixture-path".to_string(),
        "scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png".to_string(),
        "--python-command".to_string(),
        default_python_command(),
    ])
    .expect("offline readiness command should parse");

    match command {
        CliCommand::OfflineReadiness(args) => {
            assert_eq!(args.privacy_runner_path, PathBuf::from("scripts/privacy_filter/run_privacy_filter.py"));
            assert_eq!(args.ocr_runner_path, PathBuf::from("scripts/ocr_eval/run_small_ocr.py"));
            assert_eq!(args.ocr_fixture_path, PathBuf::from("scripts/ocr_eval/fixtures/synthetic_printed_phi_line.png"));
        }
        _ => panic!("expected OfflineReadiness command"),
    }
}
```

- [ ] **Step 2: Run parser/unit test to verify RED**

Run: `cargo test -p mdid-cli parses_offline_readiness_command_for_cli_opf_and_ocr_evidence -- --nocapture`

Expected: FAIL because `OfflineReadiness` command/args do not exist.

- [ ] **Step 3: Write the failing PHI/path-safety report tests**

Add a unit test in `crates/mdid-cli/src/main.rs` and an integration test in `crates/mdid-cli/tests/cli_smoke.rs` verifying:

```rust
assert_eq!(report["artifact"], "offline_cli_ocr_readiness");
assert_eq!(report["schema_version"], 1);
assert_eq!(report["network_required"], false);
assert_eq!(report["privacy_filter"]["opf_requires_explicit_flag"], true);
assert_eq!(report["privacy_filter"]["network_api_called"], false);
assert_eq!(report["ocr"]["candidate"], "PP-OCRv5_mobile_rec");
assert_eq!(report["ocr"]["fallback_fixture_available"], true);
assert!(!stdout.contains("synthetic_printed_phi_line.png"));
assert!(!stdout.contains("run_small_ocr.py"));
assert!(!stdout.contains("run_privacy_filter.py"));
assert!(!stdout.contains("Jane Example"));
assert!(!stdout.contains("MRN-12345"));
```

- [ ] **Step 4: Run report tests to verify RED**

Run: `cargo test -p mdid-cli offline_readiness -- --nocapture`

Expected: FAIL because `build_offline_readiness_report` and the CLI command are not implemented.

- [ ] **Step 5: Implement minimal command/report logic**

In `crates/mdid-cli/src/main.rs`:

```rust
struct OfflineReadinessArgs {
    privacy_runner_path: PathBuf,
    ocr_runner_path: PathBuf,
    ocr_fixture_path: PathBuf,
    python_command: String,
}

fn build_offline_readiness_report(args: &OfflineReadinessArgs) -> Result<Value, String> {
    require_regular_file(&args.privacy_runner_path, "missing Privacy Filter runner file")?;
    require_regular_file(&args.ocr_runner_path, "missing OCR runner file")?;
    require_regular_file(&args.ocr_fixture_path, "missing OCR fixture file")?;
    Ok(json!({
        "artifact": "offline_cli_ocr_readiness",
        "schema_version": 1,
        "cli_surface": "ready",
        "local_first": true,
        "network_required": false,
        "python_command_configured": !args.python_command.trim().is_empty(),
        "privacy_filter": {
            "runner_available": true,
            "default_mode": "deterministic_offline_fallback",
            "opf_requires_explicit_flag": true,
            "network_api_called": false,
            "scope": "text_only_pii_detection"
        },
        "ocr": {
            "runner_available": true,
            "fallback_fixture_available": true,
            "candidate": "PP-OCRv5_mobile_rec",
            "engine": "PP-OCRv5-mobile-bounded-spike",
            "scope": "printed_text_line_extraction_only",
            "privacy_filter_contract": "text_only_normalized_input"
        },
        "non_goals": [
            "network_api_use",
            "browser_ocr_execution",
            "desktop_ocr_execution",
            "visual_redaction",
            "image_pixel_redaction",
            "handwriting_recognition",
            "final_pdf_rewrite_export",
            "model_quality_benchmark"
        ]
    }))
}
```

Wire parsing, dispatch, and usage exactly for:

```text
mdid-cli offline-readiness --privacy-runner-path <path> --ocr-runner-path <path> --ocr-fixture-path <path> [--python-command <cmd>]
```

- [ ] **Step 6: Run targeted tests to verify GREEN**

Run: `cargo test -p mdid-cli offline_readiness -- --nocapture`

Expected: PASS with 4 offline-readiness tests passing.

- [ ] **Step 7: Run formatting and diff checks**

Run:

```bash
cargo fmt --check
git diff --check
```

Expected: both PASS.

- [ ] **Step 8: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs docs/superpowers/plans/2026-05-01-offline-cli-ocr-readiness.md README.md
git commit -m "feat(cli): add offline Privacy Filter OCR readiness report"
```
