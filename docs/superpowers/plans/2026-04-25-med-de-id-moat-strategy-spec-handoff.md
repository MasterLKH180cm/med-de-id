# Moat Strategy Spec Handoff Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded strategy-to-spec handoff slice so a moat round no longer leaves `implemented_specs` empty and the CLI/history surfaces show the deterministic normalized engineering handoff IDs produced from selected strategies.

**Architecture:** Keep this slice local-first and deterministic. `mdid-application` will own the pure helper that derives stable normalized spec handoff IDs from selected moat strategies and the configured spec-generation budget, skipping IDs that normalize to empty. `mdid-runtime` will use that helper to populate `MoatRoundSummary.implemented_specs` for both continue and stop paths, and `mdid-cli` plus docs will expose the new bounded handoff output without pretending the repo now auto-writes spec files or launches agents automatically.

**Tech Stack:** Rust workspace, Cargo, mdid-application, mdid-runtime, mdid-cli, mdid-domain, markdown docs.

---

## Scope note

This slice adds:
- deterministic `implemented_specs` population from selected strategies
- spec-generation budget enforcement over the handoff list
- CLI round/history output for the latest bounded handoff IDs
- README/spec truth-sync for the new handoff surface

This slice does **not** add:
- automatic markdown spec file creation
- automatic plan generation on disk
- agent scheduling or autonomous round chaining
- live market crawling
- GitHub PR automation

## File structure

**Modify:**
- `crates/mdid-application/src/lib.rs` — add the pure spec-handoff helper and use it from round evaluation
- `crates/mdid-application/tests/moat_rounds.rs` — add RED/GREEN coverage for the new helper and populated `implemented_specs`
- `crates/mdid-runtime/src/moat.rs` — pass the spec-generation budget through all round paths and keep stop-path summaries truthful
- `crates/mdid-runtime/tests/moat_runtime.rs` — verify successful and budget-stopped rounds expose the expected handoff IDs
- `crates/mdid-runtime/src/moat_history.rs` — surface the latest bounded handoff IDs in summary inspection
- `crates/mdid-runtime/tests/moat_history.rs` — verify history summary carries latest spec handoff IDs
- `crates/mdid-cli/src/main.rs` — print `implemented_specs=` in `moat round` and `latest_implemented_specs=` in `moat history`
- `crates/mdid-cli/tests/moat_cli.rs` — lock the new CLI contract strings
- `README.md` — document the bounded spec-handoff surface
- `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md` — truth-sync current implementation status for the new bounded handoff capability
- `docs/superpowers/plans/2026-04-25-med-de-id-moat-strategy-spec-handoff.md` — keep this plan honest if the contract changes during execution

---

### Task 1: Add deterministic spec-handoff generation in `mdid-application`

**Files:**
- Modify: `crates/mdid-application/src/lib.rs`
- Test: `crates/mdid-application/tests/moat_rounds.rs`

- [ ] **Step 1: Write the failing application tests**

Append these tests to `crates/mdid-application/tests/moat_rounds.rs`:

```rust
use mdid_application::{
    build_moat_spec_handoff_ids, evaluate_moat_round, select_top_strategies, MoatImprovementThreshold,
};

#[test]
fn build_moat_spec_handoff_ids_uses_selected_order_and_spec_budget() {
    let selected = vec![
        MoatStrategy {
            strategy_id: "workflow-audit".into(),
            title: "Workflow audit moat".into(),
            target_moat_type: MoatType::WorkflowLockIn,
            implementation_cost: 2,
            expected_moat_gain: 8,
            ..MoatStrategy::default()
        },
        MoatStrategy {
            strategy_id: "compliance-playbook".into(),
            title: "Compliance playbook moat".into(),
            target_moat_type: MoatType::ComplianceMoat,
            implementation_cost: 3,
            expected_moat_gain: 6,
            ..MoatStrategy::default()
        },
    ];

    assert_eq!(
        build_moat_spec_handoff_ids(&selected, 1),
        vec!["moat-spec/workflow-audit".to_string()]
    );
    assert_eq!(
        build_moat_spec_handoff_ids(&selected, 2),
        vec![
            "moat-spec/workflow-audit".to_string(),
            "moat-spec/compliance-playbook".to_string(),
        ]
    );
}

#[test]
fn build_moat_spec_handoff_ids_normalizes_strategy_ids_and_skips_empty_values() {
    let selected = vec![
        MoatStrategy {
            strategy_id: " Workflow Audit / 2026 ".into(),
            ..MoatStrategy::default()
        },
        MoatStrategy {
            strategy_id: "***".into(),
            ..MoatStrategy::default()
        },
        MoatStrategy {
            strategy_id: "Compliance Ledger".into(),
            ..MoatStrategy::default()
        },
    ];

    assert_eq!(
        build_moat_spec_handoff_ids(&selected, 2),
        vec![
            "moat-spec/workflow-audit-2026".to_string(),
            "moat-spec/compliance-ledger".to_string(),
        ]
    );
}

#[test]
fn evaluate_moat_round_populates_implemented_specs_from_selected_strategies() {
    let summary = evaluate_moat_round(
        Uuid::nil(),
        &MarketMoatSnapshot {
            moat_score: 40,
            ..MarketMoatSnapshot::default()
        },
        &CompetitorProfile {
            threat_score: 35,
            ..CompetitorProfile::default()
        },
        &LockInReport {
            lockin_score: 60,
            workflow_dependency_strength: 70,
            ..LockInReport::default()
        },
        &[
            MoatStrategy {
                strategy_id: "workflow-audit".into(),
                title: "Workflow audit moat".into(),
                expected_moat_gain: 7,
                implementation_cost: 2,
                target_moat_type: MoatType::WorkflowLockIn,
                ..MoatStrategy::default()
            },
            MoatStrategy {
                strategy_id: "compliance-playbook".into(),
                title: "Compliance playbook moat".into(),
                expected_moat_gain: 5,
                implementation_cost: 3,
                target_moat_type: MoatType::ComplianceMoat,
                ..MoatStrategy::default()
            },
        ],
        1,
        true,
        MoatImprovementThreshold(3),
    );

    assert_eq!(
        summary.implemented_specs,
        vec!["moat-spec/workflow-audit".to_string()]
    );
}
```

- [ ] **Step 2: Run the tests to verify RED**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-application --test moat_rounds
```

Expected: FAIL because `build_moat_spec_handoff_ids` does not exist yet and `evaluate_moat_round` does not accept a spec-generation budget.

- [ ] **Step 3: Write the minimal implementation**

Update `crates/mdid-application/src/lib.rs` so it exports the helper and uses it in round evaluation:

```rust
fn normalize_moat_spec_handoff_id(strategy_id: &str) -> Option<String> {
    let mut normalized = String::new();
    let mut last_was_separator = false;

    for character in strategy_id.chars() {
        if character.is_ascii_alphanumeric() {
            normalized.push(character.to_ascii_lowercase());
            last_was_separator = false;
        } else if !normalized.is_empty() && !last_was_separator {
            normalized.push('-');
            last_was_separator = true;
        }
    }

    while normalized.ends_with('-') {
        normalized.pop();
    }

    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

pub fn build_moat_spec_handoff_ids(
    selected_strategies: &[MoatStrategy],
    max_spec_generations: usize,
) -> Vec<String> {
    selected_strategies
        .iter()
        .filter_map(|strategy| {
            normalize_moat_spec_handoff_id(&strategy.strategy_id)
                .map(|normalized| format!("moat-spec/{normalized}"))
        })
        .take(max_spec_generations)
        .collect()
}

pub fn evaluate_moat_round(
    round_id: Uuid,
    market: &MarketMoatSnapshot,
    competitor: &CompetitorProfile,
    lock_in: &LockInReport,
    selected_strategies: &[MoatStrategy],
    max_spec_generations: usize,
    tests_passed: bool,
    threshold: MoatImprovementThreshold,
) -> MoatRoundSummary {
    let moat_score_before = ((market.moat_score as i16 + lock_in.lockin_score as i16)
        - (competitor.threat_score as i16 / 2))
        .max(0);
    let strategy_gain: i16 = selected_strategies
        .iter()
        .map(|strategy| strategy.expected_moat_gain)
        .sum();
    let moat_score_after = if tests_passed {
        moat_score_before + strategy_gain
    } else {
        moat_score_before
    };
    let continue_decision = if tests_passed && (moat_score_after - moat_score_before) >= threshold.0 {
        ContinueDecision::Continue
    } else {
        ContinueDecision::Stop
    };

    MoatRoundSummary {
        round_id,
        selected_strategies: selected_strategies
            .iter()
            .map(|strategy| strategy.strategy_id.clone())
            .collect(),
        implemented_specs: build_moat_spec_handoff_ids(selected_strategies, max_spec_generations),
        tests_passed,
        moat_score_before,
        moat_score_after,
        continue_decision,
        stop_reason: if continue_decision == ContinueDecision::Stop {
            Some(
                if tests_passed {
                    "moat improvement below threshold"
                } else {
                    "tests failed"
                }
                .into(),
            )
        } else {
            None
        },
        pivot_reason: None,
    }
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
git commit -m "feat: derive moat strategy spec handoff ids"
```

### Task 2: Carry the bounded spec handoff through runtime and history

**Files:**
- Modify: `crates/mdid-runtime/src/moat.rs`
- Modify: `crates/mdid-runtime/tests/moat_runtime.rs`
- Modify: `crates/mdid-runtime/src/moat_history.rs`
- Modify: `crates/mdid-runtime/tests/moat_history.rs`

- [ ] **Step 1: Write the failing runtime/history tests**

Append these assertions to `crates/mdid-runtime/tests/moat_runtime.rs`:

```rust
assert_eq!(
    report.summary.implemented_specs,
    vec!["moat-spec/workflow-audit".to_string()]
);
```

Add this new test to `crates/mdid-runtime/tests/moat_runtime.rs`:

```rust
#[test]
fn bounded_round_limits_handoff_ids_to_the_spec_generation_budget() {
    let report = run_bounded_round(MoatRoundInput {
        market: MarketMoatSnapshot::default(),
        competitor: CompetitorProfile::default(),
        lock_in: LockInReport::default(),
        strategies: vec![
            MoatStrategy {
                strategy_id: "workflow-audit".into(),
                title: "Workflow audit moat".into(),
                target_moat_type: MoatType::WorkflowLockIn,
                implementation_cost: 2,
                expected_moat_gain: 8,
                ..MoatStrategy::default()
            },
            MoatStrategy {
                strategy_id: "compliance-playbook".into(),
                title: "Compliance playbook moat".into(),
                target_moat_type: MoatType::ComplianceMoat,
                implementation_cost: 3,
                expected_moat_gain: 6,
                ..MoatStrategy::default()
            },
        ],
        budget: ResourceBudget {
            max_round_minutes: 30,
            max_parallel_tasks: 3,
            max_strategy_candidates: 2,
            max_spec_generations: 1,
            max_implementation_tasks: 1,
            max_review_loops: 1,
        },
        improvement_threshold: 3,
        tests_passed: true,
    });

    assert_eq!(
        report.summary.implemented_specs,
        vec!["moat-spec/workflow-audit".to_string()]
    );
}
```

Add this new stop-path regression test to `crates/mdid-runtime/tests/moat_runtime.rs`:

```rust
#[test]
fn bounded_round_keeps_handoff_ids_when_review_budget_stops_after_implementation() {
    let report = run_bounded_round(MoatRoundInput {
        market: MarketMoatSnapshot {
            moat_score: 45,
            ..MarketMoatSnapshot::default()
        },
        competitor: CompetitorProfile {
            threat_score: 30,
            ..CompetitorProfile::default()
        },
        lock_in: LockInReport {
            lockin_score: 60,
            workflow_dependency_strength: 72,
            ..LockInReport::default()
        },
        strategies: vec![MoatStrategy {
            strategy_id: "workflow-audit".into(),
            title: "Workflow audit moat".into(),
            target_moat_type: MoatType::WorkflowLockIn,
            implementation_cost: 2,
            expected_moat_gain: 8,
            ..MoatStrategy::default()
        }],
        budget: ResourceBudget {
            max_round_minutes: 30,
            max_parallel_tasks: 3,
            max_strategy_candidates: 2,
            max_spec_generations: 1,
            max_implementation_tasks: 1,
            max_review_loops: 0,
        },
        improvement_threshold: 3,
        tests_passed: true,
    });

    assert_eq!(report.summary.moat_score_before, 90);
    assert_eq!(report.summary.moat_score_after, 90);
    assert_eq!(
        report.summary.implemented_specs,
        vec!["moat-spec/workflow-audit".to_string()]
    );
}
```

Strengthen the existing `spec or implementation budget exhausted` runtime test so it asserts:

```rust
assert_eq!(report.summary.moat_score_before, 90);
assert_eq!(report.summary.moat_score_after, 90);
assert!(report.summary.implemented_specs.is_empty());
```

Add this new test to `crates/mdid-runtime/tests/moat_history.rs`:

```rust
#[test]
fn history_summary_exposes_latest_implemented_specs() {
    let dir = tempdir().expect("temp dir should exist");
    let history_path = dir.path().join("moat-history.json");
    let mut store = LocalMoatHistoryStore::open(&history_path).expect("history store should open");
    let round_id = Uuid::new_v4();

    store
        .append(
            recorded_at("2026-04-25T22:00:00Z"),
            sample_report(
                round_id,
                ContinueDecision::Continue,
                None,
                "review approved bounded moat round",
                90,
                98,
                true,
                &[
                    "market_scan",
                    "competitor_analysis",
                    "lockin_analysis",
                    "strategy_generation",
                    "spec_planning",
                    "implementation",
                    "review",
                    "evaluation",
                ],
            ),
        )
        .expect("report should persist");

    assert_eq!(
        store.summary().latest_implemented_specs,
        vec!["moat-spec/workflow-audit".to_string()]
    );
}
```

- [ ] **Step 2: Run the tests to verify RED**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-runtime --test moat_runtime
cargo test -p mdid-runtime --test moat_history
```

Expected: FAIL because runtime still leaves `implemented_specs` empty on stop-path summaries and history summary does not expose `latest_implemented_specs`.

- [ ] **Step 3: Write the minimal implementation**

Update `crates/mdid-runtime/src/moat.rs` imports and round evaluation calls:

```rust
use mdid_application::{
    build_default_moat_task_graph, evaluate_moat_round, project_task_graph_progress,
    select_top_strategies, summarize_round_memory, MoatImprovementThreshold,
};
```

Change both `evaluate_moat_round(...)` call sites in `run_bounded_round` and `stop_report` to pass the budgeted spec count:

```rust
let summary = evaluate_moat_round(
    round_id,
    &input.market,
    &input.competitor,
    &input.lock_in,
    &selected_strategies,
    usize::from(input.budget.max_spec_generations),
    input.tests_passed,
    MoatImprovementThreshold(input.improvement_threshold),
);
```

Keep stop-path reports truthful by computing the stop summary without granting moat gains for unexecuted later stages, then reattaching only the fields that were honestly produced before the stop. In practice:

```rust
let mut summary = evaluate_moat_round(
    round_id,
    &input.market,
    &input.competitor,
    &input.lock_in,
    &[],
    0,
    input.tests_passed,
    MoatImprovementThreshold(input.improvement_threshold),
);
summary.selected_strategies = selected_strategies
    .iter()
    .map(|strategy| strategy.strategy_id.clone())
    .collect();
summary.implemented_specs = if executed_tasks.iter().any(|task| task == SPEC_PLANNING) {
    build_moat_spec_handoff_ids(
        &selected_strategies,
        usize::from(input.budget.max_spec_generations),
    )
} else {
    Vec::new()
};
summary.continue_decision = ContinueDecision::Stop;
summary.stop_reason = stop_reason.clone();
```

This preserves truthful moat scores for pre-evaluation stop paths while still exposing handoff IDs only when the round actually reached `spec_planning`.
Update `crates/mdid-runtime/src/moat_history.rs` to carry the latest handoff IDs in the summary:

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoatHistorySummary {
    pub entry_count: usize,
    pub latest_round_id: Option<String>,
    pub latest_continue_decision: Option<ContinueDecision>,
    pub latest_stop_reason: Option<String>,
    pub latest_decision_summary: Option<String>,
    pub latest_moat_score_after: Option<i16>,
    pub best_moat_score_after: Option<i16>,
    pub improvement_deltas: Vec<i16>,
    pub latest_implemented_specs: Vec<String>,
}
```

and inside `summary()`:

```rust
latest_implemented_specs: latest.report.summary.implemented_specs.clone(),
```

Update the shared `sample_report(...)` helper in `crates/mdid-runtime/tests/moat_history.rs` so the returned `MoatRoundSummary` contains:

```rust
implemented_specs: vec!["moat-spec/workflow-audit".to_string()],
```

- [ ] **Step 4: Run the tests to verify GREEN**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-runtime --test moat_runtime
cargo test -p mdid-runtime --test moat_history
CARGO_BUILD_JOBS=1 cargo test -p mdid-runtime
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-runtime/src/moat.rs crates/mdid-runtime/tests/moat_runtime.rs crates/mdid-runtime/src/moat_history.rs crates/mdid-runtime/tests/moat_history.rs
git commit -m "feat: carry moat spec handoff ids through runtime history"
```

### Task 3: Surface the bounded handoff IDs in CLI and docs

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
- Modify: `docs/superpowers/plans/2026-04-25-med-de-id-moat-strategy-spec-handoff.md`

- [ ] **Step 1: Write the failing CLI contract tests**

Update the expected `moat round` output in `crates/mdid-cli/tests/moat_cli.rs` to include a new line after `moat_score_after=...`:

```rust
"implemented_specs=moat-spec/workflow-audit\n",
```

Update the expected `moat history` output in `crates/mdid-cli/tests/moat_cli.rs` to include:

```rust
"latest_implemented_specs=moat-spec/workflow-audit\n",
```

Add this focused parsing/formatting test near the CLI unit tests:

```rust
#[test]
fn format_string_list_uses_none_for_empty_values_and_commas_for_non_empty_values() {
    assert_eq!(format_string_list(&[]), "<none>");
    assert_eq!(
        format_string_list(&[
            "moat-spec/workflow-audit".to_string(),
            "moat-spec/compliance-playbook".to_string(),
        ]),
        "moat-spec/workflow-audit,moat-spec/compliance-playbook"
    );
}
```

- [ ] **Step 2: Run the tests to verify RED**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli
```

Expected: FAIL because CLI output does not print the new handoff lines and `format_string_list` does not exist yet.

- [ ] **Step 3: Write the minimal CLI/docs implementation**

Update `crates/mdid-cli/src/main.rs`:

```rust
println!("implemented_specs={}", format_string_list(&report.summary.implemented_specs));
```

Place that line in `run_moat_round` immediately after `moat_score_after=...`.

Add this line to `print_history_summary` after `latest_decision_summary=...`:

```rust
println!(
    "latest_implemented_specs={}",
    format_string_list(&summary.latest_implemented_specs)
);
```

Add the helper near `format_improvement_deltas`:

```rust
fn format_string_list(values: &[String]) -> String {
    if values.is_empty() {
        "<none>".to_string()
    } else {
        values.join(",")
    }
}
```

Truth-sync `README.md` by adding to the Moat Loop Foundation section:

```md
A successful bounded round now also prints `implemented_specs=...`, which is a deterministic handoff list of bounded engineering spec IDs derived from the selected strategies and the configured spec-generation budget. These IDs are inspection outputs only; the current slice still does not auto-write markdown spec files or launch implementation agents by itself.
```

Truth-sync `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md` in the shipped-foundation status section so it states that the current repository now includes:

```md
- bounded strategy-to-spec handoff IDs in moat round outputs and persisted history summaries
```

and keep the not-yet-implemented section honest by retaining:

```md
- no automatic markdown spec file generation on disk
- no automatic agent dispatch from the continuation gate
```

If the final contract string differs from the plan while implementing, patch this plan file so the snippets stay truthful.

- [ ] **Step 4: Run the tests to verify GREEN**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli
cargo test -p mdid-cli
cargo run -q -p mdid-cli -- moat round
cargo run -q -p mdid-cli -- moat round --history-path .mdid/moat-history-plan-check.json
cargo run -q -p mdid-cli -- moat history --history-path .mdid/moat-history-plan-check.json
rm -f .mdid/moat-history-plan-check.json
```

Expected:
- tests PASS
- `moat round` prints `implemented_specs=moat-spec/workflow-audit`
- `moat history` prints `latest_implemented_specs=moat-spec/workflow-audit`

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs README.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-25-med-de-id-moat-strategy-spec-handoff.md
git commit -m "feat: expose bounded moat spec handoff ids"
```

## Self-review

- **Spec coverage:** This plan covers the remaining bounded `strategy-to-spec/plan handoff` gap by turning selected strategies into deterministic `implemented_specs`, carrying them through runtime/history, and surfacing them in CLI/docs.
- **Placeholder scan:** No `TODO`, `TBD`, or “similar to above” placeholders remain.
- **Type consistency:** The plan consistently uses `build_moat_spec_handoff_ids`, `implemented_specs`, `latest_implemented_specs`, and the `moat-spec/<strategy-id>` contract.

## Execution handoff

Plan complete and saved to `docs/superpowers/plans/2026-04-25-med-de-id-moat-strategy-spec-handoff.md`.

Autonomous controller choice for this cron run: **Subagent-Driven (recommended)** — execute task-by-task with fresh subagents, spec review first, code-quality review second.
