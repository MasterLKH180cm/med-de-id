# Moat Round Input File Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Allow `mdid-cli moat round` and `mdid-cli moat control-plane` to run against a local JSON `MoatRoundInput` file instead of only the built-in deterministic sample.

**Architecture:** Keep the moat loop local-first and bounded: a user supplies a JSON input file matching `MoatRoundInput`, the CLI reads it synchronously, applies existing override flags, then runs the existing deterministic round/control-plane pipeline. No agents, crawlers, daemons, PR automation, or cron jobs are launched.

**Tech Stack:** Rust workspace, `serde`, `serde_json`, `mdid-cli` integration tests, `mdid-runtime::moat::MoatRoundInput`.

---

## File Structure

- Modify: `crates/mdid-runtime/src/moat.rs`
  - Add `Serialize` and `Deserialize` derives to `MoatRoundInput` so local JSON fixtures can use the same stable runtime input contract.
- Modify: `crates/mdid-cli/src/main.rs`
  - Add optional `input_path: Option<PathBuf>` to `MoatRoundCommand` and `MoatControlPlaneCommand`.
  - Parse `--input-path PATH` for `moat round` and `moat control-plane`.
  - Add `load_round_input` helper: load JSON input if present, otherwise use the current sample input, then apply existing overrides.
  - Print `input_path=PATH` in `moat round` and `source=input` in `moat control-plane` when input file is used.
  - Update usage text.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add CLI integration tests proving input-file round/control-plane behavior and parser error behavior.
- Modify: `README.md`, `AGENTS.md`, `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Truth-sync the shipped moat-loop surface and constraints.

### Task 1: Add JSON input-path support to moat round/control-plane

**Files:**
- Modify: `crates/mdid-runtime/src/moat.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `README.md`
- Modify: `AGENTS.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`

- [ ] **Step 1: Write failing tests**

Add these tests near existing moat round/control-plane tests in `crates/mdid-cli/tests/moat_cli.rs`:

```rust
#[test]
fn moat_round_uses_local_json_input_file() {
    let input_path = unique_history_path("round-input-file");
    let history_path = unique_history_path("round-input-history");
    let input_path_arg = input_path.to_string_lossy().to_string();
    let history_path_arg = history_path.to_string_lossy().to_string();

    fs::write(
        &input_path,
        serde_json::json!({
            "market": {"market_id": "clinic-deid", "moat_score": 20},
            "competitor": {"competitor_id": "manual-competitor", "threat_score": 80},
            "lock_in": {"lockin_score": 90, "workflow_dependency_strength": 95},
            "strategies": [{
                "strategy_id": "clinic-workflow-lock",
                "title": "Clinic workflow lock",
                "target_moat_type": "WorkflowLockIn",
                "implementation_cost": 1,
                "expected_moat_gain": 12
            }],
            "budget": {
                "max_round_minutes": 30,
                "max_parallel_tasks": 3,
                "max_strategy_candidates": 2,
                "max_spec_generations": 1,
                "max_implementation_tasks": 1,
                "max_review_loops": 1
            },
            "improvement_threshold": 3,
            "tests_passed": true
        }).to_string(),
    )
    .expect("failed to write moat input fixture");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--input-path",
            &input_path_arg,
            "--history-path",
            &history_path_arg,
        ])
        .output()
        .expect("failed to run moat round with input path");

    assert!(output.status.success(), "round failed: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(&format!("input_path={input_path_arg}\n")));
    assert!(stdout.contains("implemented_specs=moat-spec/clinic-workflow-lock\n"));
    assert!(stdout.contains(&format!("history_saved_to={history_path_arg}\n")));
}

#[test]
fn moat_control_plane_uses_local_json_input_file_without_saving_history() {
    let input_path = unique_history_path("control-plane-input-file");
    let input_path_arg = input_path.to_string_lossy().to_string();

    fs::write(
        &input_path,
        serde_json::json!({
            "market": {"market_id": "clinic-deid", "moat_score": 20},
            "competitor": {"competitor_id": "manual-competitor", "threat_score": 80},
            "lock_in": {"lockin_score": 90, "workflow_dependency_strength": 95},
            "strategies": [{
                "strategy_id": "clinic-workflow-lock",
                "title": "Clinic workflow lock",
                "target_moat_type": "WorkflowLockIn",
                "implementation_cost": 1,
                "expected_moat_gain": 12
            }],
            "budget": {
                "max_round_minutes": 30,
                "max_parallel_tasks": 3,
                "max_strategy_candidates": 2,
                "max_spec_generations": 1,
                "max_implementation_tasks": 1,
                "max_review_loops": 1
            },
            "improvement_threshold": 3,
            "tests_passed": true
        }).to_string(),
    )
    .expect("failed to write moat input fixture");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "control-plane", "--input-path", &input_path_arg])
        .output()
        .expect("failed to run moat control-plane with input path");

    assert!(output.status.success(), "control-plane failed: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("source=input\n"));
    assert!(stdout.contains(&format!("input_path={input_path_arg}\n")));
    assert!(stdout.contains("latest_implemented_specs=moat-spec/clinic-workflow-lock\n"));
}

#[test]
fn moat_round_rejects_missing_input_path_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--input-path"])
        .output()
        .expect("failed to run moat round with missing input path");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing value for moat round --input-path"));
}

#[test]
fn moat_round_rejects_invalid_input_json() {
    let input_path = unique_history_path("round-invalid-input-json");
    let input_path_arg = input_path.to_string_lossy().to_string();
    fs::write(&input_path, "{not-json").expect("failed to write invalid input fixture");

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--input-path", &input_path_arg])
        .output()
        .expect("failed to run moat round with invalid input json");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("failed to parse moat round input"));
}
```

- [ ] **Step 2: Run tests to verify RED**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_round_uses_local_json_input_file moat_control_plane_uses_local_json_input_file_without_saving_history moat_round_rejects_missing_input_path_value moat_round_rejects_invalid_input_json -- --nocapture
```

Expected: FAIL because `--input-path` is not parsed and/or `MoatRoundInput` is not deserializable.

- [ ] **Step 3: Implement minimal support**

Implement exactly this behavior:

1. In `crates/mdid-runtime/src/moat.rs`, change `MoatRoundInput` derive to:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoatRoundInput {
```

2. In `crates/mdid-cli/src/main.rs`, add `input_path: Option<PathBuf>` to `MoatRoundCommand` and `MoatControlPlaneCommand`.

3. Parse `--input-path PATH` in both `parse_moat_round_command` and `parse_moat_control_plane_command` with duplicate and missing-value errors:

```rust
"--input-path" => {
    let value = iter
        .next()
        .ok_or_else(|| "missing value for moat round --input-path".to_string())?;
    if input_path.replace(PathBuf::from(value)).is_some() {
        return Err("duplicate moat round --input-path".to_string());
    }
}
```

Use `moat control-plane` in the error strings for the control-plane parser.

4. Add helpers:

```rust
fn round_input_for_command(command: &MoatRoundCommand) -> Result<MoatRoundInput, String> {
    let mut input = if let Some(path) = &command.input_path {
        load_round_input(path)?
    } else {
        sample_round_input(&MoatRoundOverrides::default())
    };
    apply_round_overrides(&mut input, &command.overrides);
    Ok(input)
}

fn control_plane_input_for_command(command: &MoatControlPlaneCommand) -> Result<MoatRoundInput, String> {
    let mut input = if let Some(path) = &command.input_path {
        load_round_input(path)?
    } else {
        sample_round_input(&MoatRoundOverrides::default())
    };
    apply_round_overrides(&mut input, &command.overrides);
    Ok(input)
}

fn load_round_input(path: &Path) -> Result<MoatRoundInput, String> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("failed to read moat round input {}: {error}", path.display()))?;
    serde_json::from_str(&content)
        .map_err(|error| format!("failed to parse moat round input {}: {error}", path.display()))
}

fn apply_round_overrides(input: &mut MoatRoundInput, overrides: &MoatRoundOverrides) {
    if let Some(value) = overrides.strategy_candidates {
        input.budget.max_strategy_candidates = value;
    }
    if let Some(value) = overrides.spec_generations {
        input.budget.max_spec_generations = value;
    }
    if let Some(value) = overrides.implementation_tasks {
        input.budget.max_implementation_tasks = value;
    }
    if let Some(value) = overrides.review_loops {
        input.budget.max_review_loops = value;
    }
    if let Some(value) = overrides.tests_passed {
        input.tests_passed = value;
    }
}
```

Update `sample_round_input` to call `apply_round_overrides` instead of duplicating override mutation.

5. Update `run_moat_round` to call `round_input_for_command`, print `input_path=...` when present, then append history if requested.

6. Update `run_moat_control_plane` to call `control_plane_input_for_command` for sample/input mode. Print `source=input` and `input_path=...` when present; preserve `source=sample` when absent and `source=history` for persisted history inspection.

7. Update `usage()` for `moat round` and `moat control-plane` to include `[--input-path PATH]`.

- [ ] **Step 4: Run targeted tests to verify GREEN**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_round_uses_local_json_input_file -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_control_plane_uses_local_json_input_file_without_saving_history -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_round_rejects_missing_input_path_value -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_round_rejects_invalid_input_json -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Truth-sync docs**

Update `README.md`, `AGENTS.md`, and `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md` to state:

- `moat round` accepts optional `--input-path PATH` for local JSON `MoatRoundInput`.
- `moat control-plane` accepts optional `--input-path PATH` for dry-run planning/control-plane inspection from local inputs.
- Input-file mode is local-only and still does not crawl data, launch agents, append history unless `--history-path` is explicitly supplied, open PRs, create cron jobs, or write artifacts.

- [ ] **Step 6: Run broader relevant validation**

Run:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_round -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_control_plane -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p mdid-runtime moat -- --nocapture
```

Expected: PASS.

- [ ] **Step 7: Commit**

Run:

```bash
git add crates/mdid-runtime/src/moat.rs crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs README.md AGENTS.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-28-med-de-id-moat-round-input-file.md
git commit -m "feat(cli): run moat rounds from local input files"
```

## Self-Review

- Spec coverage: This plan advances the Autonomous Multi-Agent System by replacing sample-only rounds with local user-supplied strategic inputs, while preserving bounded local-first behavior.
- Placeholder scan: no TBD/TODO/fill-in placeholders remain.
- Type consistency: `MoatRoundInput`, `--input-path`, and `input_path` names are consistent across tests, implementation, and docs.
