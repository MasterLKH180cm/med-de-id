# CLI Verification Report Implementation Plan

> **For implementation workers:** REQUIRED PROCESS: implement task-by-task with strict TDD, then run spec review and quality review before merge. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded PHI-safe `mdid-cli verify-artifacts` command that verifies the existence and size bounds of local de-identification output artifacts without reading or printing their contents.

**Architecture:** Keep the CLI thin and local-only: parse explicit artifact paths and an optional maximum size, inspect file metadata only, and emit a PHI-safe JSON aggregate report. This raises CLI completion by adding verification/audit polish without adding workflow orchestration, vault browsing, decoded-value display, or controller/agent semantics.

**Tech Stack:** Rust, existing `mdid-cli` single-binary parser, serde_json, std::fs metadata APIs, cargo tests.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `VerifyArtifacts(VerifyArtifactsArgs)` to `CliCommand`.
  - Add parser support for `verify-artifacts --artifact-paths-json <json-array> [--max-bytes <positive-int>]`.
  - Add metadata-only report generation that does not read file contents.
  - Add command execution branch and usage text.
  - Add unit tests near existing CLI tests.
- Modify: `README.md`
  - Truth-sync completion snapshot after landed verification, bump CLI only if tests pass and command lands.

## Task 1: CLI metadata-only artifact verification command

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/src/main.rs` unit tests

- [x] **Step 1: Write failing parser/report tests**

Add tests that call the existing parser and new report helper. Test names:

```rust
#[test]
fn parses_verify_artifacts_command_without_requiring_debug() {
    let command = parse_cli_args(&[
        "verify-artifacts".to_string(),
        "--artifact-paths-json".to_string(),
        "[\"/tmp/a.csv\",\"/tmp/b.json\"]".to_string(),
        "--max-bytes".to_string(),
        "1024".to_string(),
    ]).expect("verify artifacts command should parse");

    match command {
        CliCommand::VerifyArtifacts(args) => {
            assert_eq!(args.artifact_paths_json, "[\"/tmp/a.csv\",\"/tmp/b.json\"]");
            assert_eq!(args.max_bytes, Some(1024));
        }
        other => panic!("expected VerifyArtifacts command, got {other:?}"),
    }
}

#[test]
fn verify_artifacts_report_checks_metadata_without_printing_phi_paths_or_contents() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let phi_path = temp_dir.path().join("Jane-Doe-MRN-123-output.csv");
    std::fs::write(&phi_path, "name\nJane Doe\n").expect("write fixture");

    let report = build_verify_artifacts_report(
        &[phi_path.to_string_lossy().to_string()],
        Some(1024),
    ).expect("report");
    let json = serde_json::to_string(&report).expect("json");

    assert_eq!(report.artifact_count, 1);
    assert_eq!(report.existing_count, 1);
    assert_eq!(report.oversized_count, 0);
    assert_eq!(report.missing_count, 0);
    assert!(!json.contains("Jane"));
    assert!(!json.contains("MRN"));
    assert!(!json.contains("name"));
}

#[test]
fn verify_artifacts_rejects_empty_path_list_and_non_positive_max_bytes() {
    assert!(parse_artifact_paths_json("[]").is_err());
    assert!(parse_positive_max_bytes("0").is_err());
    assert!(parse_positive_max_bytes("not-a-number").is_err());
}
```

- [x] **Step 2: Run targeted tests to verify RED**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli verify_artifacts -- --nocapture
```

Expected: FAIL because `VerifyArtifacts`, `build_verify_artifacts_report`, `parse_artifact_paths_json`, or `parse_positive_max_bytes` do not exist.

- [x] **Step 3: Implement minimal command**

Implement:
- `CliCommand::VerifyArtifacts(VerifyArtifactsArgs)`
- `VerifyArtifactsArgs { artifact_paths_json: String, max_bytes: Option<u64> }`
- parser branch for `verify-artifacts`
- `parse_artifact_paths_json(&str) -> Result<Vec<String>, String>` requiring a non-empty JSON string array of non-blank paths, matching the existing CLI parser error convention
- `parse_positive_max_bytes(&str) -> Result<u64, String>` requiring `> 0`, matching the existing CLI parser error convention
- `build_verify_artifacts_report(paths: &[String], max_bytes: Option<u64>) -> Result<VerifyArtifactsReport, String>` using `std::fs::metadata` only; report entries must not include raw paths/filenames
- command execution that prints only safe JSON counts and entry indices/status/byte sizes

Report shape:

```rust
#[derive(Serialize)]
struct VerifyArtifactsReport {
    artifact_count: usize,
    existing_count: usize,
    missing_count: usize,
    oversized_count: usize,
    max_bytes: Option<u64>,
    artifacts: Vec<VerifyArtifactEntryReport>,
}

#[derive(Serialize)]
struct VerifyArtifactEntryReport {
    index: usize,
    exists: bool,
    byte_len: Option<u64>,
    within_max_bytes: Option<bool>,
}
```

- [x] **Step 4: Run targeted tests to verify GREEN**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli verify_artifacts -- --nocapture
```

Expected: PASS.

- [x] **Step 5: Run broader CLI verification**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --all-targets
cargo clippy -p mdid-cli --all-targets -- -D warnings
git diff --check
```

Expected: all PASS.

- [x] **Step 6: Commit**

```bash
git add crates/mdid-cli/src/main.rs docs/superpowers/plans/2026-04-29-cli-verification-report.md
git commit -m "feat(cli): add PHI-safe artifact verification report"
```

## Task 2: README truth-sync

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update README completion snapshot**

Change the snapshot sentence to mention the bounded CLI artifact verification command. If Task 1 verification passed, set CLI to 95%, Browser/web unchanged at 63%, Desktop app unchanged at 58%, and Overall to 88%.

- [ ] **Step 2: Verify README claims**

Run:

```bash
grep -n "Completion snapshot\|CLI | 95%\|Browser/web | 63%\|Desktop app | 58%\|Overall | 88%\|verify-artifacts" README.md
git diff --check
```

Expected: grep finds the updated rows and command mention; diff check passes.

- [ ] **Step 3: Commit README truth-sync**

```bash
git add README.md docs/superpowers/plans/2026-04-29-cli-verification-report.md
git commit -m "docs: truth-sync CLI verification completion"
```

## Self-Review

- Spec coverage: command is local-only, metadata-only, PHI-safe, verification/audit polish for CLI; helper functions use the existing CLI `Result<_, String>` parser convention; no browser/desktop changes; README truth-sync included.
- Placeholder scan: no TODO/TBD placeholders.
- Type consistency: parser, args, report helper, and report structs all use `verify-artifacts` and `VerifyArtifacts` consistently.
