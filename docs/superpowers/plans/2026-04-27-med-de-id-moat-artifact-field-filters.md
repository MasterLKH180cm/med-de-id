# Med De-ID Moat Artifact Field Filters Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add precise `mdid-cli moat artifacts` filters for artifact reference and artifact summary so autonomous agents can find handoff evidence without broad text matching.

**Architecture:** Keep the slice inside the existing CLI parser/runner. Extend `MoatArtifactsCommand` with two optional string filters, parse `--artifact-ref` and `--artifact-summary`, apply them as AND filters after the existing node and contains filters, and preserve read-only history behavior.

**Tech Stack:** Rust workspace, `mdid-cli`, integration tests in `crates/mdid-cli/tests/moat_cli.rs`, targeted Cargo verification with `CARGO_INCREMENTAL=0`.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Extend `MoatArtifactsCommand` with `artifact_ref` and `artifact_summary` fields.
  - Parse `--artifact-ref TEXT` and `--artifact-summary TEXT` for `moat artifacts`.
  - Update usage text for the new flags.
  - Apply exact field substring filters in `run_moat_artifacts`.
  - Update parser unit tests for the command struct.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Update `USAGE` to include the new flags.
  - Add integration coverage proving the new field filters select only matching artifact handoffs and do not mutate history.

### Task 1: Add artifact reference and summary filters to `mdid-cli moat artifacts`

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/moat_cli.rs`

- [ ] **Step 1: Write the failing integration test**

Add this test after `moat_artifacts_prints_completed_task_artifact_handoffs_read_only` in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn moat_artifacts_filters_by_artifact_ref_and_summary_read_only() {
    let history_path = unique_history_path("moat-artifacts-field-filters");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--review-loops",
            "0",
            "--history-path",
            history_path_arg,
        ])
        .output()
        .expect("failed to seed artifact filter history");
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));

    let claim = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "claim-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
        ])
        .output()
        .expect("failed to claim review task for artifact filter");
    assert!(claim.status.success(), "{}", String::from_utf8_lossy(&claim.stderr));

    let round_id = LocalMoatHistoryStore::open_existing(&history_path)
        .expect("history should reload")
        .entries()[0]
        .report
        .summary
        .round_id
        .to_string();

    let complete = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "complete-task",
            "--history-path",
            history_path_arg,
            "--node-id",
            "review",
            "--artifact-ref",
            "docs/review handoff.md",
            "--artifact-summary",
            "Reviewer approved release candidate",
        ])
        .output()
        .expect("failed to complete review with filter artifact");
    assert!(complete.status.success(), "{}", String::from_utf8_lossy(&complete.stderr));

    let before = fs::read_to_string(&history_path).expect("history should exist");
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "artifacts",
            "--history-path",
            history_path_arg,
            "--round-id",
            &round_id,
            "--artifact-ref",
            "review handoff",
            "--artifact-summary",
            "approved release",
        ])
        .output()
        .expect("failed to inspect artifact field filters");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!(
            "moat artifacts\nround_id={round_id}\nartifact_entries=1\nartifact={round_id}|review|docs/review handoff.md|Reviewer approved release candidate\n"
        )
    );
    assert_eq!(fs::read_to_string(&history_path).unwrap(), before);

    let no_match = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "artifacts",
            "--history-path",
            history_path_arg,
            "--round-id",
            &round_id,
            "--artifact-ref",
            "missing handoff",
            "--artifact-summary",
            "approved release",
        ])
        .output()
        .expect("failed to inspect non-matching artifact field filters");
    assert!(no_match.status.success(), "{}", String::from_utf8_lossy(&no_match.stderr));
    assert_eq!(
        String::from_utf8_lossy(&no_match.stdout),
        format!("moat artifacts\nround_id={round_id}\nartifact_entries=0\n")
    );

    cleanup_history_path(&history_path);
}
```

- [ ] **Step 2: Run the targeted test to verify RED**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_artifacts_filters_by_artifact_ref_and_summary_read_only -- --nocapture
```

Expected: FAIL with `unknown flag: --artifact-ref` from `mdid-cli moat artifacts`.

- [ ] **Step 3: Write the minimal implementation**

In `crates/mdid-cli/src/main.rs`:

1. Add fields to `MoatArtifactsCommand`:

```rust
    artifact_ref: Option<String>,
    artifact_summary: Option<String>,
```

2. In `parse_moat_artifacts_command`, initialize:

```rust
    let mut artifact_ref = None;
    let mut artifact_summary = None;
```

3. Add match arms before `--limit`:

```rust
            "--artifact-ref" => {
                let value = required_flag_value(args, index, "--artifact-ref", false)?;
                if artifact_ref.is_some() {
                    return Err(duplicate_flag_error("--artifact-ref"));
                }
                artifact_ref = Some(value.to_string());
            }
            "--artifact-summary" => {
                let value = required_flag_value(args, index, "--artifact-summary", false)?;
                if artifact_summary.is_some() {
                    return Err(duplicate_flag_error("--artifact-summary"));
                }
                artifact_summary = Some(value.to_string());
            }
```

4. Return the fields in `MoatArtifactsCommand`.

5. In `run_moat_artifacts`, after the existing `contains` filter, add:

```rust
        .filter(|(_node_id, artifact)| {
            command
                .artifact_ref
                .as_deref()
                .map(|needle| artifact.artifact_ref.contains(needle))
                .unwrap_or(true)
        })
        .filter(|(_node_id, artifact)| {
            command
                .artifact_summary
                .as_deref()
                .map(|needle| artifact.summary.contains(needle))
                .unwrap_or(true)
        })
```

6. Update the usage string in both `src/main.rs` and `tests/moat_cli.rs` so `moat artifacts` reads:

```text
moat artifacts --history-path PATH [--round-id ROUND_ID] [--node-id NODE_ID] [--contains TEXT] [--artifact-ref TEXT] [--artifact-summary TEXT] [--limit N]
```

7. Update parser unit test `parse_moat_artifacts_command_accepts_round_node_contains_and_limit_filters` to include the two new flags and expected fields:

```rust
"--artifact-ref", "review handoff", "--artifact-summary", "approved release",
```

and expected struct fields:

```rust
artifact_ref: Some("review handoff".to_string()),
artifact_summary: Some("approved release".to_string()),
```

- [ ] **Step 4: Run targeted tests to verify GREEN**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_artifacts_filters_by_artifact_ref_and_summary_read_only -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli parse_moat_artifacts_command_accepts_round_node_contains_and_limit_filters
```

Expected: both PASS.

- [ ] **Step 5: Run broader CLI verification**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli moat_artifacts
CARGO_INCREMENTAL=0 cargo test -p mdid-cli
```

Expected: PASS.

- [ ] **Step 6: Commit the slice**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs docs/superpowers/plans/2026-04-27-med-de-id-moat-artifact-field-filters.md
git commit -m "feat: add moat artifact field filters"
```
