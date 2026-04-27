# Moat Export Round Selection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let `mdid-cli moat export-specs` and `mdid-cli moat export-plans` export handoffs from an explicitly selected persisted round via `--round-id`.

**Architecture:** Extend the existing one-shot export commands with the same read-only round selection semantics used by `moat artifacts`, `moat assignments`, and `moat task-graph`. The command still opens only existing local history, writes deterministic markdown for persisted `implemented_specs`, and never launches agents, creates cron jobs, crawls data, or mutates history.

**Tech Stack:** Rust workspace, `mdid-cli` integration tests, `mdid-runtime` local JSON moat history, `mdid-application` markdown renderers.

---

## File Structure

- Modify: `crates/mdid-cli/tests/moat_cli.rs` — add integration coverage for `--round-id` on both export commands and a no-match error path.
- Modify: `crates/mdid-cli/src/main.rs` — parse optional `--round-id` for `moat export-specs` and `moat export-plans`, select the requested persisted round before rendering, and update usage text.
- Modify: `README.md` — document the optional `--round-id` flag for operator handoff replay.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md` — truth-sync shipped status for round-scoped exports.

---

### Task 1: Round-scoped moat exports

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`

- [ ] **Step 1: Write failing CLI integration tests**

Append these tests to `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn moat_export_specs_can_select_persisted_round_by_exact_round_id() {
    let history_path = unique_history_path("export-specs-round-id");
    let output_dir = unique_history_path("export-specs-round-id-output");
    if output_dir.exists() {
        std::fs::remove_file(&output_dir).expect("remove placeholder path");
    }
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let output_dir_arg = output_dir.to_str().expect("output dir should be utf-8");

    let first = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed first round");
    assert!(first.status.success(), "{}", String::from_utf8_lossy(&first.stderr));

    let first_store = LocalMoatHistoryStore::open(&history_path).expect("history store should open");
    let first_round_id = first_store
        .summary()
        .latest_round_id
        .expect("first persisted round id should exist");

    let second = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--review-loops", "0", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed second round");
    assert!(second.status.success(), "{}", String::from_utf8_lossy(&second.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "export-specs",
            "--history-path",
            history_path_arg,
            "--round-id",
            &first_round_id,
            "--output-dir",
            output_dir_arg,
        ])
        .output()
        .expect("failed to export moat specs for selected round");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("moat spec export\n"));
    assert!(stdout.contains(&format!("round_id={first_round_id}\n")));
    assert!(stdout.contains("exported_specs=moat-spec/workflow-audit\n"));
    assert!(output_dir.join("workflow-audit.md").exists());

    cleanup_history_path(&history_path);
    cleanup_history_path(&output_dir);
}

#[test]
fn moat_export_plans_can_select_persisted_round_by_exact_round_id() {
    let history_path = unique_history_path("export-plans-round-id");
    let output_dir = unique_history_path("export-plans-round-id-output");
    if output_dir.exists() {
        std::fs::remove_file(&output_dir).expect("remove placeholder path");
    }
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let output_dir_arg = output_dir.to_str().expect("output dir should be utf-8");

    let first = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed first round");
    assert!(first.status.success(), "{}", String::from_utf8_lossy(&first.stderr));

    let first_store = LocalMoatHistoryStore::open(&history_path).expect("history store should open");
    let first_round_id = first_store
        .summary()
        .latest_round_id
        .expect("first persisted round id should exist");

    let second = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--review-loops", "0", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed second round");
    assert!(second.status.success(), "{}", String::from_utf8_lossy(&second.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "export-plans",
            "--history-path",
            history_path_arg,
            "--round-id",
            &first_round_id,
            "--output-dir",
            output_dir_arg,
        ])
        .output()
        .expect("failed to export moat plans for selected round");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("moat plan export\n"));
    assert!(stdout.contains(&format!("round_id={first_round_id}\n")));
    assert!(stdout.contains("exported_plans=moat-spec/workflow-audit\n"));
    assert!(output_dir.join("workflow-audit-implementation-plan.md").exists());

    cleanup_history_path(&history_path);
    cleanup_history_path(&output_dir);
}

#[test]
fn moat_export_specs_reports_error_when_round_id_does_not_match_history() {
    let history_path = unique_history_path("export-specs-missing-round-id");
    let output_dir = unique_history_path("export-specs-missing-round-id-output");
    if output_dir.exists() {
        std::fs::remove_file(&output_dir).expect("remove placeholder path");
    }
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let output_dir_arg = output_dir.to_str().expect("output dir should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed moat history");
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "export-specs",
            "--history-path",
            history_path_arg,
            "--round-id",
            "00000000-0000-0000-0000-000000000999",
            "--output-dir",
            output_dir_arg,
        ])
        .output()
        .expect("failed to run moat spec export with missing round id");

    assert!(!output.status.success(), "export should fail for missing round id");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "error: no moat history entry matched round_id 00000000-0000-0000-0000-000000000999\n"
    );
    assert!(!output_dir.exists(), "failed export must not create output directory");

    cleanup_history_path(&history_path);
    cleanup_history_path(&output_dir);
}
```

- [ ] **Step 2: Run tests to verify RED**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli export_specs_can_select_persisted_round_by_exact_round_id export_plans_can_select_persisted_round_by_exact_round_id export_specs_reports_error_when_round_id_does_not_match_history -- --nocapture
```

Expected: FAIL because `export-specs` and `export-plans` do not accept `--round-id` yet.

- [ ] **Step 3: Implement minimal CLI support**

In `crates/mdid-cli/src/main.rs`:

1. Replace the `CliCommand::MoatExportSpecs { history_path, output_dir }` and `CliCommand::MoatExportPlans { history_path, output_dir }` variants with structs that include `round_id: Option<String>`.
2. Parse `--round-id ROUND_ID` for both commands, reject duplicates with the existing duplicate flag pattern, and keep `--history-path` / `--output-dir` required.
3. Select the export source entry by exact persisted `entry.report.summary.round_id.to_string()` when `round_id` is present; otherwise keep latest-round behavior.
4. Return exactly `no moat history entry matched round_id {round_id}` before creating the output directory when no persisted round matches.
5. Print `round_id=<selected round id>` in both successful export summaries.
6. Update the `USAGE` string in `crates/mdid-cli/tests/moat_cli.rs` and `crates/mdid-cli/src/main.rs` to include `[--round-id ROUND_ID]` for both export commands.

- [ ] **Step 4: Run tests to verify GREEN**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli export_specs_can_select_persisted_round_by_exact_round_id export_plans_can_select_persisted_round_by_exact_round_id export_specs_reports_error_when_round_id_does_not_match_history -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Update docs/spec**

In `README.md`, update the `moat export-specs` and `moat export-plans` examples to show optional `--round-id ROUND_ID` for replaying a prior handoff round.

In `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`, update the shipped foundation bullet for export commands so both mention optional `--round-id ROUND_ID` exact persisted round selection.

- [ ] **Step 6: Run broader verification**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-application --test moat_rounds -- --nocapture
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/mdid-cli/tests/moat_cli.rs crates/mdid-cli/src/main.rs README.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-27-med-de-id-moat-export-round-id-selection.md
git commit -m "feat: select moat export rounds"
```
