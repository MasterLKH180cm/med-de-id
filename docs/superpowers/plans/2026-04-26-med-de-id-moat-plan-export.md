# Moat Plan Export Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded local `moat export-plans` command that converts latest persisted moat spec handoffs into implementation-plan markdown files.

**Architecture:** Extend the existing spec-handoff export path without adding background automation: application code renders deterministic implementation plans from `MoatRoundSummary`, and CLI code writes one plan file per latest handoff from an existing history file. The command is local-first, one-shot, and intentionally does not create cron jobs, PRs, or unrestricted autonomous agents.

**Tech Stack:** Rust workspace, `mdid-application` markdown renderer, `mdid-cli` integration tests, local JSON moat history.

---

## Scope

This plan advances the Autonomous Multi-Agent System by closing the safe handoff gap between generated moat specs and SDD/TDD execution plans. It implements:

```bash
cargo run -p mdid-cli -- moat export-plans --history-path .mdid/moat-history.json --output-dir docs/superpowers/plans/generated
```

The command must:

- require an existing history file; missing paths fail instead of creating a file
- fail for empty history
- fail when the latest round has no `implemented_specs` handoff IDs
- create the output directory when needed
- write one deterministic `*.md` implementation plan per latest handoff
- print a stable summary containing `exported_plans=...`, `written_files=...`, and `output_dir=...`

## Files

- Modify: `crates/mdid-application/src/lib.rs` — add deterministic `render_moat_plan_markdown` next to `render_moat_spec_markdown`.
- Modify: `crates/mdid-application/tests/moat_rounds.rs` — unit tests for plan markdown rendering.
- Modify: `crates/mdid-cli/src/main.rs` — parse and run `moat export-plans`.
- Modify: `crates/mdid-cli/tests/moat_cli.rs` — integration tests for plan export and usage text.
- Modify: `README.md` — document the one-shot plan export command.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md` — truth-sync shipped/planned status.

---

### Task 1: Application plan markdown renderer

**Files:**
- Modify: `crates/mdid-application/tests/moat_rounds.rs`
- Modify: `crates/mdid-application/src/lib.rs`

- [ ] **Step 1: Write failing renderer tests**

Append tests to `crates/mdid-application/tests/moat_rounds.rs`:

```rust
#[test]
fn render_moat_plan_markdown_creates_sdd_tdd_plan_for_handoff() {
    let round_id = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000123").unwrap();
    let mut summary = sample_summary(round_id);
    summary.implemented_specs = vec!["moat-spec/workflow-audit".to_string()];
    summary.selected_strategies = vec!["workflow-audit".to_string()];

    let markdown = mdid_application::render_moat_plan_markdown(
        "moat-spec/workflow-audit",
        &summary,
        &summary.selected_strategies,
    )
    .expect("plan markdown should render");

    assert!(markdown.starts_with("# Workflow Audit Implementation Plan\n"));
    assert!(markdown.contains("REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development"));
    assert!(markdown.contains("**Goal:** Ship the workflow-audit moat slice"));
    assert!(markdown.contains("### Task 1: Persist workflow-audit artifact"));
    assert!(markdown.contains("cargo test -p mdid-application moat_rounds::"));
    assert!(markdown.contains("git commit -m \"feat: add workflow-audit moat plan\""));
}

#[test]
fn render_moat_plan_markdown_rejects_unknown_handoff() {
    let round_id = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000124").unwrap();
    let summary = sample_summary(round_id);

    let error = mdid_application::render_moat_plan_markdown(
        "moat-spec/missing",
        &summary,
        &summary.selected_strategies,
    )
    .expect_err("unknown handoff should fail");

    assert!(error.contains("handoff id moat-spec/missing not present"));
}
```

- [ ] **Step 2: Run renderer tests to verify RED**

Run:

```bash
cargo test -p mdid-application --test moat_rounds render_moat_plan_markdown -- --nocapture
```

Expected: FAIL because `render_moat_plan_markdown` is not defined.

- [ ] **Step 3: Implement minimal renderer**

Add `pub fn render_moat_plan_markdown(handoff_id: &str, summary: &MoatRoundSummary, selected_strategies: &[String]) -> Result<String, String>` to `crates/mdid-application/src/lib.rs`. Reuse the same handoff validation and selected-strategy validation semantics as `render_moat_spec_markdown`. Render a complete plan with the required writing-plans header, SDD/TDD architecture note, a single Task 1 with explicit RED/GREEN/verify/commit steps, and no placeholders.

- [ ] **Step 4: Run renderer tests to verify GREEN**

Run:

```bash
cargo test -p mdid-application --test moat_rounds render_moat_plan_markdown -- --nocapture
```

Expected: PASS.

### Task 2: CLI one-shot plan export command

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`

- [ ] **Step 1: Write failing CLI integration tests**

Append tests to `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn moat_export_plans_writes_latest_handoff_plan_markdown() {
    let history_path = unique_history_path("export-plans");
    let output_dir = unique_history_path("export-plans-output");
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
        .args(["moat", "export-plans", "--history-path", history_path_arg, "--output-dir", output_dir_arg])
        .output()
        .expect("failed to export moat plans");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("moat plan export\n"));
    assert!(stdout.contains("exported_plans=moat-spec/workflow-audit\n"));
    assert!(stdout.contains("written_files=workflow-audit-implementation-plan.md\n"));

    let markdown = std::fs::read_to_string(output_dir.join("workflow-audit-implementation-plan.md"))
        .expect("plan markdown should be written");
    assert!(markdown.contains("# Workflow Audit Implementation Plan"));
    assert!(markdown.contains("REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development"));
}

#[test]
fn moat_export_plans_requires_existing_history_file() {
    let history_path = unique_history_path("missing-export-plans");
    let output_dir = unique_history_path("missing-export-plans-output");
    if output_dir.exists() {
        std::fs::remove_file(&output_dir).expect("remove placeholder path");
    }
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "export-plans",
            "--history-path",
            history_path.to_str().expect("history path should be utf-8"),
            "--output-dir",
            output_dir.to_str().expect("output dir should be utf-8"),
        ])
        .output()
        .expect("failed to run export-plans");

    assert!(!output.status.success(), "export-plans should fail for missing history");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed to open moat history"));
}
```

Also update `USAGE` in the test file so it includes:

```text
moat export-plans --history-path PATH --output-dir DIR
```

- [ ] **Step 2: Run CLI tests to verify RED**

Run:

```bash
cargo test -p mdid-cli --test moat_cli moat_export_plans -- --nocapture
```

Expected: FAIL because `moat export-plans` is an unknown command.

- [ ] **Step 3: Implement parser and runner**

Modify `crates/mdid-cli/src/main.rs` to:

- import `render_moat_plan_markdown`
- add a `MoatExportPlans { history_path: String, output_dir: String }` command variant
- parse `moat export-plans --history-path PATH --output-dir DIR`
- open existing history with `LocalMoatHistoryStore::open_existing`
- fail for empty history and missing latest handoffs
- write each handoff to `<slug>-implementation-plan.md`
- print `moat plan export`, `exported_plans=...`, `written_files=...`, and `output_dir=...`
- update usage text

- [ ] **Step 4: Run targeted CLI tests to verify GREEN**

Run:

```bash
cargo test -p mdid-cli --test moat_cli moat_export_plans -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Run broader verification**

Run:

```bash
cargo test -p mdid-application --test moat_rounds
cargo test -p mdid-cli --test moat_cli
cargo test -p mdid-cli
```

Expected: PASS.

- [ ] **Step 6: Update docs and spec status**

In `README.md`, document `moat export-plans` immediately after `moat export-specs`, emphasizing that it is one-shot, local, and does not start background agents.

In `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`, add a shipped-foundation bullet for `moat export-plans`, and update the shipped-slice paragraph to say deterministic implementation-plan markdown export exists while full autonomous Planner/Coder/Reviewer orchestration remains future work.

- [ ] **Step 7: Verify docs mention the command**

Run:

```bash
python - <<'PY'
from pathlib import Path
for path in [Path('README.md'), Path('docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md')]:
    text = path.read_text()
    assert 'moat export-plans --history-path' in text, path
print('docs mention export-plans')
PY
```

Expected: `docs mention export-plans`.

- [ ] **Step 8: Commit**

Run:

```bash
git add crates/mdid-application/src/lib.rs crates/mdid-application/tests/moat_rounds.rs crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs README.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-26-med-de-id-moat-plan-export.md
git commit -m "feat: export moat implementation plans"
```

Expected: commit succeeds on the current feature branch.
