# CLI Verify Artifacts Duplicate Paths Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `mdid-cli verify-artifacts` reject duplicate artifact paths before building a verification report, so audit evidence cannot accidentally count the same output more than once.

**Architecture:** Keep the behavior inside the existing CLI verification helper. Normalize only by exact trimmed path string for this bounded guard; do not resolve filesystem paths or print submitted path values, preserving PHI-safe reporting.

**Tech Stack:** Rust, Cargo test, existing `mdid-cli` unit tests in `crates/mdid-cli/src/main.rs`.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add a duplicate exact trimmed path guard inside `build_verify_artifacts_report` before metadata inspection.
  - Add unit tests near existing `verify_artifacts_*` tests.
- Modify: `README.md`
  - Truth-sync CLI/browser/desktop/overall completion snapshot and verification evidence after the landed slice.

### Task 1: Reject duplicate artifact paths in CLI verification

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/src/main.rs`

- [x] **Step 1: Write the failing test**

Add this test after `verify_artifacts_rejects_empty_path_list_and_non_positive_max_bytes`:

```rust
#[test]
fn verify_artifacts_report_rejects_duplicate_paths_without_echoing_path() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let phi_path = temp_dir.path().join("Jane-Doe-MRN-123-output.csv");
    std::fs::write(&phi_path, "name\nJane Doe\n").expect("write fixture");
    let path = phi_path.to_string_lossy().to_string();

    let error = build_verify_artifacts_report(&[path.clone(), path], Some(1024))
        .expect_err("duplicate artifact path should be rejected");

    assert_eq!(error, "artifact path list must not contain duplicate paths");
    assert!(!error.contains("Jane"));
    assert!(!error.contains("MRN"));
    assert!(!error.contains("output.csv"));
}
```

- [x] **Step 2: Run test to verify it fails**

Run: `cargo test -p mdid-cli verify_artifacts_report_rejects_duplicate_paths_without_echoing_path -- --nocapture`

Expected: FAIL because duplicate paths are currently inspected and counted instead of rejected.

- [x] **Step 3: Write minimal implementation**

In `build_verify_artifacts_report`, after the existing empty/blank path guard and before `let mut existing_count = 0;`, add:

```rust
let mut seen_paths = std::collections::HashSet::with_capacity(paths.len());
for path in paths {
    if !seen_paths.insert(path.trim()) {
        return Err("artifact path list must not contain duplicate paths".to_string());
    }
}
```

- [x] **Step 4: Run targeted and broader tests**

Run: `cargo test -p mdid-cli verify_artifacts -- --nocapture`

Expected: PASS.

Run: `cargo test -p mdid-cli --test cli_smoke cli_verify_artifacts_reports_missing_and_exits_nonzero -- --nocapture`

Expected: PASS.

Run: `cargo fmt --check && git diff --check`

Expected: PASS.

- [x] **Step 5: Commit implementation**

```bash
git add crates/mdid-cli/src/main.rs docs/superpowers/plans/2026-04-30-cli-verify-artifacts-duplicate-paths.md
git commit -m "fix(cli): reject duplicate artifact verification paths"
```

### Task 2: README completion truth-sync

**Files:**
- Modify: `README.md`

- [x] **Step 1: Update completion snapshot**

Update the `Current repository status` snapshot to mention this bounded CLI verification hardening. CLI remains 95%, browser/web remains 76%, desktop app remains 70%, and overall remains 93% because the change hardens verification semantics but does not add a major missing capability.

- [x] **Step 2: Run docs verification**

Run: `cargo fmt --check && git diff --check`

Expected: PASS.

- [x] **Step 3: Commit docs truth-sync**

```bash
git add README.md docs/superpowers/plans/2026-04-30-cli-verify-artifacts-duplicate-paths.md
git commit -m "docs: truth-sync duplicate artifact verification guard"
```
