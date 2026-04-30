# CLI Verify Artifacts Failure Exit Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `mdid-cli verify-artifacts` return a non-zero exit status when requested artifacts are missing or exceed the optional byte limit, while keeping the PHI-safe JSON verification report on stdout.

**Architecture:** Keep the existing CLI verification report shape and path-free artifact entries. Add a narrow post-report failure decision in the CLI command path so automation can fail builds on missing or oversized artifacts without exposing local paths.

**Tech Stack:** Rust workspace, `mdid-cli`, Cargo integration tests, `assert_cmd`, `serde_json`, `tempfile`.

---

## File Structure

- Modify `crates/mdid-cli/src/main.rs`: keep `VerifyArtifactsReport` as the report model; after rendering the JSON report, return an error if `missing_count > 0` or `oversized_count > 0`.
- Modify `crates/mdid-cli/tests/cli_smoke.rs`: add integration tests that run the compiled CLI and verify non-zero exit for missing and oversized artifacts while asserting stdout remains a parseable path-free report.
- Modify `README.md`: truth-sync the CLI/browser/desktop/overall completion snapshot after the landed slice and record that overall percentage remains unchanged unless this actually removes a larger completion blocker.

### Task 1: CLI verify-artifacts exits non-zero for failed verification

**Files:**
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
- Modify: `crates/mdid-cli/src/main.rs`

- [ ] **Step 1: Write the failing missing-artifact integration test**

Append this test to `crates/mdid-cli/tests/cli_smoke.rs`:

```rust
#[test]
fn verify_artifacts_exits_nonzero_when_artifact_is_missing() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let missing_path = temp_dir.path().join("missing-output.json");
    let paths_json = serde_json::to_string(&vec![missing_path.to_string_lossy().to_string()])
        .expect("paths json");

    let assert = Command::cargo_bin("mdid-cli")
        .expect("binary")
        .args([
            "verify-artifacts",
            "--artifact-paths-json",
            &paths_json,
        ])
        .assert()
        .failure();

    let output = assert.get_output();
    let stdout = String::from_utf8(output.stdout.clone()).expect("stdout utf8");
    let report: serde_json::Value = serde_json::from_str(stdout.trim()).expect("json report");

    assert_eq!(report["artifact_count"], 1);
    assert_eq!(report["existing_count"], 0);
    assert_eq!(report["missing_count"], 1);
    assert_eq!(report["oversized_count"], 0);
    assert_eq!(report["artifacts"][0]["index"], 0);
    assert_eq!(report["artifacts"][0]["exists"], false);
    assert!(!stdout.contains("missing-output.json"));
}
```

- [ ] **Step 2: Run the missing-artifact test and verify RED**

Run:

```bash
cargo test -p mdid-cli --test cli_smoke verify_artifacts_exits_nonzero_when_artifact_is_missing -- --nocapture
```

Expected: FAIL because the command currently exits successfully even though the report has `missing_count: 1`.

- [ ] **Step 3: Implement the minimal missing-artifact failure behavior**

Change only `run_verify_artifacts` in `crates/mdid-cli/src/main.rs` to:

```rust
fn run_verify_artifacts(args: VerifyArtifactsArgs) -> Result<(), String> {
    let paths = parse_artifact_paths_json(&args.artifact_paths_json)?;
    let report = build_verify_artifacts_report(&paths, args.max_bytes)?;
    let missing_count = report.missing_count;
    let oversized_count = report.oversized_count;
    println!(
        "{}",
        serde_json::to_string(&report)
            .map_err(|err| format!("failed to render artifact verification report: {err}"))?
    );
    if missing_count > 0 || oversized_count > 0 {
        return Err(format!(
            "artifact verification failed: {missing_count} missing, {oversized_count} oversized"
        ));
    }
    Ok(())
}
```

- [ ] **Step 4: Run the missing-artifact test and verify GREEN**

Run:

```bash
cargo test -p mdid-cli --test cli_smoke verify_artifacts_exits_nonzero_when_artifact_is_missing -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Write the failing oversized-artifact integration test**

Append this test to `crates/mdid-cli/tests/cli_smoke.rs`:

```rust
#[test]
fn verify_artifacts_exits_nonzero_when_artifact_exceeds_max_bytes() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let artifact_path = temp_dir.path().join("large-output.json");
    std::fs::write(&artifact_path, b"abcdef").expect("write artifact");
    let paths_json = serde_json::to_string(&vec![artifact_path.to_string_lossy().to_string()])
        .expect("paths json");

    let assert = Command::cargo_bin("mdid-cli")
        .expect("binary")
        .args([
            "verify-artifacts",
            "--artifact-paths-json",
            &paths_json,
            "--max-bytes",
            "3",
        ])
        .assert()
        .failure();

    let output = assert.get_output();
    let stdout = String::from_utf8(output.stdout.clone()).expect("stdout utf8");
    let report: serde_json::Value = serde_json::from_str(stdout.trim()).expect("json report");

    assert_eq!(report["artifact_count"], 1);
    assert_eq!(report["existing_count"], 1);
    assert_eq!(report["missing_count"], 0);
    assert_eq!(report["oversized_count"], 1);
    assert_eq!(report["max_bytes"], 3);
    assert_eq!(report["artifacts"][0]["byte_len"], 6);
    assert_eq!(report["artifacts"][0]["within_max_bytes"], false);
    assert!(!stdout.contains("large-output.json"));
}
```

- [ ] **Step 6: Run the oversized-artifact test and verify it passes with the same minimal implementation**

Run:

```bash
cargo test -p mdid-cli --test cli_smoke verify_artifacts_exits_nonzero_when_artifact_exceeds_max_bytes -- --nocapture
```

Expected: PASS because Step 3 already fails when `oversized_count > 0`.

- [ ] **Step 7: Run the relevant CLI smoke tests**

Run:

```bash
cargo test -p mdid-cli --test cli_smoke verify_artifacts -- --nocapture
```

Expected: PASS for the existing successful verification tests and the two new failure tests.

- [ ] **Step 8: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs
git commit -m "fix(cli): fail artifact verification on missing outputs"
```

### Task 2: README completion truth-sync for CLI artifact verification

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update the completion snapshot text**

Update the `Current repository status` snapshot so it says the CLI artifact verification command now exits non-zero when artifacts are missing or oversized, the CLI percentage remains 95%, browser remains 75%, desktop remains 69%, and overall remains 93% because this is automation hardening rather than a larger OCR/rewrite/workflow completion blocker.

- [ ] **Step 2: Run formatting/checks for docs-only change**

Run:

```bash
git diff --check
```

Expected: no whitespace errors.

- [ ] **Step 3: Commit**

```bash
git add README.md docs/superpowers/plans/2026-04-30-cli-verify-artifacts-failure-exit.md
git commit -m "docs: truth-sync CLI artifact verification status"
```

---

## Self-Review

- Spec coverage: Task 1 covers non-zero exit for missing and oversized artifacts while preserving path-free JSON reports; Task 2 covers the required README completion truth-sync.
- Placeholder scan: No TBD/TODO/implement-later placeholders remain.
- Type consistency: The plan uses the existing `VerifyArtifactsReport` fields `missing_count` and `oversized_count`, and existing CLI test dependencies already used in `cli_smoke.rs`.
