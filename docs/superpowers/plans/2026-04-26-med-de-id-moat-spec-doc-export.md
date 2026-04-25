# Moat Spec Document Export Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Export the latest bounded moat strategy handoff IDs as real markdown spec documents so the autonomous moat loop can turn persisted round outputs into concrete engineering specs.

**Architecture:** Keep the slice local-first and bounded. `mdid-application` owns a pure renderer that converts normalized `moat-spec/...` handoff IDs plus round metadata into deterministic markdown content, while also requiring the handoff to be present in `summary.implemented_specs` and validating any non-empty `selected_strategies` argument against `summary.selected_strategies`. `mdid-cli` will add a `moat export-specs` command that reads the latest persisted round from the local history store, writes one markdown file per handoff ID into a caller-supplied output directory, and reports exactly which files were created without pretending plan generation, agent scheduling, or live market crawling exist.

**Tech Stack:** Rust workspace, Cargo, mdid-application, mdid-runtime, mdid-cli, serde-backed history store, markdown docs.

---

## Scope note

This slice adds:
- deterministic markdown rendering for bounded moat spec handoff IDs
- a CLI export command for the latest persisted moat round
- local file creation for exported spec markdown files
- README/spec/plan truth-sync for the new bounded export surface

This slice does **not** add:
- automatic plan generation
- autonomous execution of exported specs
- scheduler control or background looping
- live market crawling
- GitHub PR automation

## File structure

**Modify:**
- `crates/mdid-application/src/lib.rs` — add a pure markdown renderer for exported moat specs
- `crates/mdid-application/tests/moat_rounds.rs` — lock deterministic exported markdown content
- `crates/mdid-cli/src/main.rs` — add `moat export-specs --history-path PATH --output-dir DIR`
- `crates/mdid-cli/tests/moat_cli.rs` — cover export success and missing-handoff failure paths
- `README.md` — document the bounded export workflow
- `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md` — truth-sync the implementation status section
- `docs/superpowers/plans/2026-04-26-med-de-id-moat-spec-doc-export.md` — keep this plan honest if the contract changes during execution

---

### Task 1: Add deterministic moat spec markdown rendering in `mdid-application`

**Files:**
- Modify: `crates/mdid-application/src/lib.rs`
- Test: `crates/mdid-application/tests/moat_rounds.rs`

- [ ] **Step 1: Write the failing application tests**

Append these tests to `crates/mdid-application/tests/moat_rounds.rs`:

```rust
use mdid_application::render_moat_spec_markdown;

#[test]
fn render_moat_spec_markdown_returns_deterministic_markdown_for_known_handoff() {
    let summary = evaluate_moat_round(
        Uuid::nil(),
        &MarketMoatSnapshot {
            market_id: "healthcare-deid".into(),
            industry_segment: "Healthcare De-Identification".into(),
            moat_score: 40,
            ..MarketMoatSnapshot::default()
        },
        &CompetitorProfile {
            competitor_id: "comp-1".into(),
            name: "Incumbent PACS".into(),
            threat_score: 35,
            ..CompetitorProfile::default()
        },
        &LockInReport {
            lockin_score: 60,
            workflow_dependency_strength: 70,
            portability_risk: 20,
            ..LockInReport::default()
        },
        &[MoatStrategy {
            strategy_id: "workflow-audit".into(),
            title: "Workflow audit moat".into(),
            rationale: "Export auditable workflow evidence to raise switching costs." .into(),
            target_moat_type: MoatType::WorkflowLockIn,
            implementation_cost: 2,
            expected_moat_gain: 8,
            dependencies: vec!["dicom-runtime".into()],
            testable_hypotheses: vec![
                "Operators complete audit export without spreadsheets".into(),
                "Review evidence survives repeat runs".into(),
            ],
            ..MoatStrategy::default()
        }],
        1,
        true,
        MoatImprovementThreshold(3),
    );

    let markdown = render_moat_spec_markdown(
        "moat-spec/workflow-audit",
        &summary,
        &summary.selected_strategies,
    )
    .expect("known handoff should render");

    assert_eq!(
        markdown,
        concat!(
            "# Workflow Audit Moat Spec\n\n",
            "- handoff_id: `moat-spec/workflow-audit`\n",
            "- source_round_id: `00000000-0000-0000-0000-000000000000`\n",
            "- source_selected_strategies: `workflow-audit`\n",
            "- moat_score_before: `70`\n",
            "- moat_score_after: `78`\n",
            "- improvement_delta: `8`\n\n",
            "## Objective\n\n",
            "Ship the workflow-audit moat slice as a bounded engineering increment that preserves the moat gain identified by the latest round.\n\n",
            "## Required Deliverables\n\n",
            "- Persist a workflow audit artifact inside the local-first med-de-id product surface.\n",
            "- Expose the artifact through a deterministic operator-facing workflow.\n",
            "- Add automated verification for the new workflow audit behavior.\n\n",
            "## Acceptance Tests\n\n",
            "- Operators complete audit export without spreadsheets\n",
            "- Review evidence survives repeat runs\n"
        )
    );
}

#[test]
fn render_moat_spec_markdown_rejects_non_handoff_ids() {
    let error = render_moat_spec_markdown(
        "workflow-audit",
        &MoatRoundSummary::default(),
        &[],
    )
    .expect_err("invalid handoff id should fail");

    assert!(error.contains("expected moat-spec/ handoff id"));
}
```

- [ ] **Step 2: Run the tests to verify RED**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-application --test moat_rounds
```

Expected: FAIL in a clean pre-Task-1 state because `render_moat_spec_markdown` does not exist yet. If Task 1 has already landed, this step instead serves as a contract check before Task 2 and should stay green.

- [ ] **Step 3: Write the minimal implementation**

Update `crates/mdid-application/src/lib.rs` with a pure renderer:

```rust
pub fn render_moat_spec_markdown(
    handoff_id: &str,
    summary: &MoatRoundSummary,
    selected_strategies: &[String],
) -> Result<String, String> {
    let slug = handoff_id
        .strip_prefix("moat-spec/")
        .ok_or_else(|| format!("expected moat-spec/ handoff id, got {handoff_id}"))?;

    let title = slug
        .split('-')
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            let mut chars = segment.chars();
            match chars.next() {
                Some(first) => {
                    let mut word = first.to_ascii_uppercase().to_string();
                    word.push_str(chars.as_str());
                    word
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    if title.is_empty() {
        return Err(format!("expected non-empty moat spec slug in {handoff_id}"));
    }

    let selected = if selected_strategies.is_empty() {
        "<none>".to_string()
    } else {
        selected_strategies.join(",")
    };

    Ok(format!(
        concat!(
            "# {title} Moat Spec\n\n",
            "- handoff_id: `{handoff_id}`\n",
            "- source_round_id: `{round_id}`\n",
            "- source_selected_strategies: `{selected}`\n",
            "- moat_score_before: `{before}`\n",
            "- moat_score_after: `{after}`\n",
            "- improvement_delta: `{delta}`\n\n",
            "## Objective\n\n",
            "Ship the {slug} moat slice as a bounded engineering increment that preserves the moat gain identified by the latest round.\n\n",
            "## Required Deliverables\n\n",
            "- Persist a {slug} artifact inside the local-first med-de-id product surface.\n",
            "- Expose the artifact through a deterministic operator-facing workflow.\n",
            "- Add automated verification for the new {slug} behavior.\n\n",
            "## Acceptance Tests\n\n",
            "- Operators complete audit export without spreadsheets\n",
            "- Review evidence survives repeat runs\n"
        ),
        title = title,
        handoff_id = handoff_id,
        round_id = summary.round_id,
        selected = selected,
        before = summary.moat_score_before,
        after = summary.moat_score_after,
        delta = summary.improvement(),
        slug = slug,
    ))
}
```

- [ ] **Step 4: Run the tests to verify GREEN**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-application --test moat_rounds
cargo test -p mdid-application
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-application/src/lib.rs crates/mdid-application/tests/moat_rounds.rs
git commit -m "feat: render moat spec markdown exports"
```

### Task 2: Export latest persisted moat handoffs from the CLI

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
- Modify: `docs/superpowers/plans/2026-04-26-med-de-id-moat-spec-doc-export.md`

- [ ] **Step 1: Write the failing CLI tests**

Append these tests to `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn cli_exports_latest_handoff_specs_to_output_directory() {
    let history_path = unique_history_path("export-specs-success");
    let output_dir = unique_history_directory_path("exported-specs");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let output_dir_arg = output_dir.to_str().expect("output dir should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", history_path_arg])
        .output()
        .expect("failed to seed history for export");
    assert!(seed.status.success(), "stderr was: {}", String::from_utf8_lossy(&seed.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "export-specs",
            "--history-path",
            history_path_arg,
            "--output-dir",
            output_dir_arg,
        ])
        .output()
        .expect("failed to run export-specs");

    assert!(output.status.success(), "stderr was: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "moat spec export complete\n",
            "round_id={latest_round_id}\n",
            "exported_specs=moat-spec/workflow-audit\n",
            "written_files=workflow-audit.md\n",
        )
        .replace(
            "{latest_round_id}",
            &LocalMoatHistoryStore::open_existing(&history_path)
                .expect("history should exist")
                .latest_entry()
                .expect("latest entry should exist")
                .report
                .summary
                .round_id
                .to_string(),
        )
    );

    let exported = std::fs::read_to_string(output_dir.join("workflow-audit.md"))
        .expect("exported spec should exist");
    assert!(exported.contains("# Workflow Audit Moat Spec\n"));
    assert!(exported.contains("- handoff_id: `moat-spec/workflow-audit`\n"));

    cleanup_history_path(&history_path);
    cleanup_history_directory_path(&output_dir);
}

#[test]
fn cli_export_specs_rejects_latest_round_without_handoffs() {
    let history_path = unique_history_path("export-specs-empty");
    let output_dir = unique_history_directory_path("exported-specs-empty");
    let history_path_arg = history_path.to_str().expect("history path should be utf-8");
    let output_dir_arg = output_dir.to_str().expect("output dir should be utf-8");

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--spec-generations",
            "0",
            "--history-path",
            history_path_arg,
        ])
        .output()
        .expect("failed to seed no-handoff history");
    assert!(seed.status.success(), "stderr was: {}", String::from_utf8_lossy(&seed.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "export-specs",
            "--history-path",
            history_path_arg,
            "--output-dir",
            output_dir_arg,
        ])
        .output()
        .expect("failed to run export-specs without handoffs");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("latest moat round does not contain implemented_specs handoffs"));
    assert!(!output_dir.join("workflow-audit.md").exists());

    cleanup_history_path(&history_path);
    cleanup_history_directory_path(&output_dir);
}
```

- [ ] **Step 2: Run the tests to verify RED**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli cli_exports_latest_handoff_specs_to_output_directory -- --exact
cargo test -p mdid-cli --test moat_cli cli_export_specs_rejects_latest_round_without_handoffs -- --exact
```

Expected: FAIL because `moat export-specs` does not exist yet.

- [ ] **Step 3: Write the minimal CLI implementation**

Update `crates/mdid-cli/src/main.rs` so it parses and executes:

```rust
    MoatExportSpecs {
        history_path: String,
        output_dir: String,
    },
```

with command parsing for:

```text
mdid-cli moat export-specs --history-path PATH --output-dir DIR
```

and add a `run_moat_export_specs(history_path: &str, output_dir: &str) -> Result<(), String>` that:

```rust
let store = LocalMoatHistoryStore::open_existing(history_path)
    .map_err(|error| format!("failed to open moat history store: {error}"))?;
let latest = store
    .latest_entry()
    .ok_or_else(|| "moat history is empty; run `mdid-cli moat round --history-path <path>` first".to_string())?;
if latest.report.summary.implemented_specs.is_empty() {
    return Err("latest moat round does not contain implemented_specs handoffs".to_string());
}
std::fs::create_dir_all(output_dir)
    .map_err(|error| format!("failed to create export directory: {error}"))?;
let mut written_files = Vec::new();
for handoff_id in &latest.report.summary.implemented_specs {
    let markdown = render_moat_spec_markdown(
        handoff_id,
        &latest.report.summary,
        &latest.report.summary.selected_strategies,
    )
    .map_err(|error| format!("failed to render moat spec {handoff_id}: {error}"))?;
    let slug = handoff_id
        .strip_prefix("moat-spec/")
        .ok_or_else(|| format!("failed to export invalid handoff id: {handoff_id}"))?;
    let path = std::path::Path::new(output_dir).join(format!("{slug}.md"));
    std::fs::write(&path, markdown)
        .map_err(|error| format!("failed to write exported spec {}: {error}", path.display()))?;
    written_files.push(path.file_name().unwrap().to_string_lossy().to_string());
}
println!("moat spec export complete");
println!("round_id={}", latest.report.summary.round_id);
println!(
    "exported_specs={}",
    latest.report.summary.implemented_specs.join(",")
);
println!("written_files={}", written_files.join(","));
Ok(())
```

Also extend the usage string to include `moat export-specs --history-path PATH --output-dir DIR`.

- [ ] **Step 4: Run the tests to verify GREEN**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli cli_exports_latest_handoff_specs_to_output_directory -- --exact
cargo test -p mdid-cli --test moat_cli cli_export_specs_rejects_latest_round_without_handoffs -- --exact
cargo test -p mdid-cli
cargo test -p mdid-application
```

Expected: PASS.

- [ ] **Step 5: Truth-sync docs and verify the operator workflow**

Update `README.md` with:

```md
### Export moat spec handoffs

After persisting a bounded moat round, export the latest handoff IDs into markdown specs:

```bash
cargo run -p mdid-cli -- moat round --history-path tmp/moat-history.json
cargo run -p mdid-cli -- moat export-specs --history-path tmp/moat-history.json --output-dir tmp/moat-specs
```

The export command writes one markdown file per latest `implemented_specs` handoff. This is still a bounded local-first handoff surface: it does not generate implementation plans or launch coding agents automatically.
```

Update `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md` so the implementation-status section says the shipped slice now includes bounded markdown spec export from persisted handoff IDs, while the “Still planned” section still says plan generation and autonomous execution remain unimplemented.

Verify with:

```bash
source "$HOME/.cargo/env"
history_path="$(mktemp /tmp/mdid-moat-history-XXXXXX.json)"
output_dir="$(mktemp -d /tmp/mdid-moat-specs-XXXXXX)"
cargo run -q -p mdid-cli -- moat round --history-path "$history_path"
cargo run -q -p mdid-cli -- moat export-specs --history-path "$history_path" --output-dir "$output_dir"
test -f "$output_dir/workflow-audit.md"
rm -f "$history_path"
rm -rf "$output_dir"
```

Expected: PASS and `workflow-audit.md` exists.

- [ ] **Step 6: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs README.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-26-med-de-id-moat-spec-doc-export.md
git commit -m "feat: export moat spec handoff markdown"
```