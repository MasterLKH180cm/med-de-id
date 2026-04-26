# med-de-id Moat Loop Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first bounded moat-analysis round for `med-de-id`, including market/competitor/lock-in/strategy domain models, deterministic moat scoring, a resource-bounded runtime round, and a CLI entry point that emits a round report.

**Architecture:** This first slice stays inside the existing Rust workspace and extends `mdid-domain`, `mdid-application`, `mdid-runtime`, and `mdid-cli` instead of creating a brand-new crate family. `mdid-domain` will define moat-loop artifacts, `mdid-application` will score and evaluate a round, `mdid-runtime` will orchestrate a bounded task graph round over deterministic inputs, and `mdid-cli` will expose a `moat` command for local execution and inspection.

**Tech Stack:** Rust workspace, Cargo, Serde, Chrono, UUID, thiserror, Tokio, Axum-free in-memory runtime for this slice, existing workspace testing setup.

---

## Scope note

This plan covers the first executable slice of the moat loop only. It does **not** implement live internet crawling, browser dashboards, desktop strategy workspaces, or autonomous continuous loops. It deliberately focuses on a single bounded round that can be tested end-to-end and later extended.

This slice implements:
- domain artifacts for market snapshots, competitor analysis, lock-in analysis, moat strategies, and round evaluation
- deterministic moat scoring and improvement evaluation
- bounded runtime orchestration over a small task graph
- CLI execution/reporting for one local round

## File structure

**Create:**
- `crates/mdid-domain/tests/moat_workflow_models.rs`
- `crates/mdid-application/tests/moat_rounds.rs`
- `crates/mdid-runtime/src/moat.rs`
- `crates/mdid-runtime/tests/moat_runtime.rs`
- `crates/mdid-cli/tests/moat_cli.rs`

**Modify:**
- `crates/mdid-domain/src/lib.rs`
- `crates/mdid-application/src/lib.rs`
- `crates/mdid-runtime/src/lib.rs`
- `crates/mdid-cli/src/main.rs`
- `README.md`

---

### Task 1: Add moat-loop domain vocabulary to `mdid-domain`

**Files:**
- Modify: `crates/mdid-domain/src/lib.rs`
- Create: `crates/mdid-domain/tests/moat_workflow_models.rs`

- [ ] **Step 1: Write the failing domain tests**

Create `crates/mdid-domain/tests/moat_workflow_models.rs`:

```rust
use mdid_domain::{
    ContinueDecision, CompetitorProfile, LockInReport, MarketMoatSnapshot, MoatRoundSummary,
    MoatStrategy, MoatType, ResourceBudget,
};

#[test]
fn moat_type_wire_values_are_stable() {
    assert_eq!(serde_json::to_string(&MoatType::ComplianceMoat).unwrap(), "\"compliance_moat\"");
    assert_eq!(serde_json::to_string(&MoatType::WorkflowLockIn).unwrap(), "\"workflow_lockin\"");
}

#[test]
fn resource_budget_reports_exhaustion() {
    let budget = ResourceBudget {
        max_round_minutes: 15,
        max_parallel_tasks: 4,
        max_strategy_candidates: 5,
        max_spec_generations: 2,
        max_implementation_tasks: 3,
        max_review_loops: 2,
    };

    assert!(budget.supports_parallelism());
    assert!(!budget.is_zero());
}

#[test]
fn moat_round_summary_calculates_improvement() {
    let summary = MoatRoundSummary {
        moat_score_before: 41,
        moat_score_after: 49,
        continue_decision: ContinueDecision::Continue,
        ..MoatRoundSummary::default()
    };

    assert_eq!(summary.improvement(), 8);
    assert!(summary.improved());
}

#[test]
fn market_and_lockin_models_store_core_scores() {
    let market = MarketMoatSnapshot {
        market_id: "healthcare-deid".into(),
        industry_segment: "medical de-identification".into(),
        moat_score: 62,
        moat_type: vec![MoatType::ComplianceMoat, MoatType::WorkflowLockIn],
        confidence: 0.75,
        evidence: vec!["HIPAA/GDPR burden".into()],
        assumptions: vec!["buyers value auditability".into()],
        ..MarketMoatSnapshot::default()
    };
    let lock_in = LockInReport {
        lockin_score: 58,
        switching_cost_strength: 61,
        data_gravity_strength: 44,
        workflow_dependency_strength: 72,
        evidence: vec!["review workflow embedded in operations".into()],
        ..LockInReport::default()
    };

    assert_eq!(market.moat_score, 62);
    assert_eq!(lock_in.workflow_dependency_strength, 72);
}

#[test]
fn competitor_and_strategy_models_capture_actionable_fields() {
    let competitor = CompetitorProfile {
        competitor_id: "comp-1".into(),
        name: "Acme DeID".into(),
        suspected_moat_types: vec![MoatType::DataMoat],
        threat_score: 55,
        ..CompetitorProfile::default()
    };
    let strategy = MoatStrategy {
        strategy_id: "strategy-1".into(),
        title: "Audit-driven workflow moat".into(),
        target_moat_type: MoatType::WorkflowLockIn,
        implementation_cost: 3,
        expected_moat_gain: 8,
        ..MoatStrategy::default()
    };

    assert_eq!(competitor.threat_score, 55);
    assert_eq!(strategy.expected_moat_gain, 8);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-domain --test moat_workflow_models
```

Expected: FAIL because the moat-loop domain types do not exist yet.

- [ ] **Step 3: Write the minimal domain implementation**

Append to `crates/mdid-domain/src/lib.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoatType {
    ComplianceMoat,
    DataMoat,
    WorkflowLockIn,
    EcosystemMoat,
    DistributionMoat,
    NetworkEffectAdjacent,
    BrandTrustMoat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContinueDecision {
    Continue,
    Stop,
    Pivot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ResourceBudget {
    pub max_round_minutes: u32,
    pub max_parallel_tasks: u8,
    pub max_strategy_candidates: u8,
    pub max_spec_generations: u8,
    pub max_implementation_tasks: u8,
    pub max_review_loops: u8,
}

impl ResourceBudget {
    pub fn supports_parallelism(&self) -> bool {
        self.max_parallel_tasks > 1
    }

    pub fn is_zero(&self) -> bool {
        self.max_round_minutes == 0
            && self.max_parallel_tasks == 0
            && self.max_strategy_candidates == 0
            && self.max_spec_generations == 0
            && self.max_implementation_tasks == 0
            && self.max_review_loops == 0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MarketMoatSnapshot {
    pub market_id: String,
    pub industry_segment: String,
    pub market_snapshot_at: Option<DateTime<Utc>>,
    pub moat_score: u8,
    pub moat_type: Vec<MoatType>,
    pub confidence: f32,
    pub evidence: Vec<String>,
    pub assumptions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CompetitorProfile {
    pub competitor_id: String,
    pub name: String,
    pub category: String,
    pub pricing_summary: String,
    pub feature_summary: String,
    pub talent_signal_summary: String,
    pub suspected_moat_types: Vec<MoatType>,
    pub threat_score: u8,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct LockInReport {
    pub lockin_score: u8,
    pub lockin_vectors: Vec<String>,
    pub switching_cost_strength: u8,
    pub data_gravity_strength: u8,
    pub workflow_dependency_strength: u8,
    pub portability_risk: u8,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MoatStrategy {
    pub strategy_id: String,
    pub title: String,
    pub rationale: String,
    pub target_moat_type: MoatType,
    pub implementation_cost: u8,
    pub expected_moat_gain: i16,
    pub risk_level: u8,
    pub dependencies: Vec<String>,
    pub testable_hypotheses: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoatRoundSummary {
    pub round_id: Uuid,
    pub selected_strategies: Vec<String>,
    pub implemented_specs: Vec<String>,
    pub tests_passed: bool,
    pub moat_score_before: i16,
    pub moat_score_after: i16,
    pub continue_decision: ContinueDecision,
    pub stop_reason: Option<String>,
    pub pivot_reason: Option<String>,
}

impl Default for MoatRoundSummary {
    fn default() -> Self {
        Self {
            round_id: Uuid::new_v4(),
            selected_strategies: Vec::new(),
            implemented_specs: Vec::new(),
            tests_passed: false,
            moat_score_before: 0,
            moat_score_after: 0,
            continue_decision: ContinueDecision::Stop,
            stop_reason: None,
            pivot_reason: None,
        }
    }
}

impl MoatRoundSummary {
    pub fn improvement(&self) -> i16 {
        self.moat_score_after - self.moat_score_before
    }

    pub fn improved(&self) -> bool {
        self.improvement() > 0
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-domain --test moat_workflow_models
cargo test -p mdid-domain
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-domain/src/lib.rs crates/mdid-domain/tests/moat_workflow_models.rs
git commit -m "feat: add moat loop domain models"
```

### Task 2: Add deterministic moat scoring and round evaluation in `mdid-application`

**Files:**
- Modify: `crates/mdid-application/src/lib.rs`
- Create: `crates/mdid-application/tests/moat_rounds.rs`

- [ ] **Step 1: Write the failing application tests**

Create `crates/mdid-application/tests/moat_rounds.rs`:

```rust
use mdid_application::{evaluate_moat_round, select_top_strategies, MoatImprovementThreshold};
use mdid_domain::{
    CompetitorProfile, ContinueDecision, LockInReport, MarketMoatSnapshot, MoatStrategy, MoatType,
};
use uuid::Uuid;

#[test]
fn selects_highest_expected_gain_within_budget() {
    let selected = select_top_strategies(
        vec![
            MoatStrategy {
                strategy_id: "a".into(),
                title: "A".into(),
                target_moat_type: MoatType::ComplianceMoat,
                implementation_cost: 2,
                expected_moat_gain: 5,
                ..MoatStrategy::default()
            },
            MoatStrategy {
                strategy_id: "b".into(),
                title: "B".into(),
                target_moat_type: MoatType::WorkflowLockIn,
                implementation_cost: 1,
                expected_moat_gain: 8,
                ..MoatStrategy::default()
            },
        ],
        1,
    );

    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].strategy_id, "b");
}

#[test]
fn evaluate_round_stops_when_improvement_is_below_threshold() {
    let summary = evaluate_moat_round(
        Uuid::nil(),
        &MarketMoatSnapshot {
            moat_score: 50,
            ..MarketMoatSnapshot::default()
        },
        &CompetitorProfile {
            threat_score: 60,
            ..CompetitorProfile::default()
        },
        &LockInReport {
            lockin_score: 52,
            workflow_dependency_strength: 55,
            ..LockInReport::default()
        },
        &[],
        true,
        MoatImprovementThreshold(3),
    );

    assert_eq!(summary.continue_decision, ContinueDecision::Stop);
    assert_eq!(summary.moat_score_before, 54);
    assert_eq!(summary.moat_score_after, 54);
}

#[test]
fn evaluate_round_continues_when_tests_pass_and_score_improves() {
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
        &[MoatStrategy {
            strategy_id: "workflow-audit".into(),
            title: "Workflow audit moat".into(),
            expected_moat_gain: 7,
            implementation_cost: 2,
            target_moat_type: MoatType::WorkflowLockIn,
            ..MoatStrategy::default()
        }],
        true,
        MoatImprovementThreshold(3),
    );

    assert_eq!(summary.continue_decision, ContinueDecision::Continue);
    assert!(summary.moat_score_after > summary.moat_score_before);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-application --test moat_rounds
```

Expected: FAIL because moat evaluation helpers do not exist yet.

- [ ] **Step 3: Write the minimal application implementation**

Append to `crates/mdid-application/src/lib.rs`:

```rust
use mdid_domain::{
    CompetitorProfile, ContinueDecision, LockInReport, MarketMoatSnapshot, MoatRoundSummary,
    MoatStrategy,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoatImprovementThreshold(pub i16);

pub fn select_top_strategies(
    mut strategies: Vec<MoatStrategy>,
    max_strategy_candidates: usize,
) -> Vec<MoatStrategy> {
    strategies.sort_by(|left, right| {
        right
            .expected_moat_gain
            .cmp(&left.expected_moat_gain)
            .then_with(|| left.implementation_cost.cmp(&right.implementation_cost))
    });
    strategies.truncate(max_strategy_candidates);
    strategies
}

pub fn evaluate_moat_round(
    round_id: Uuid,
    market: &MarketMoatSnapshot,
    competitor: &CompetitorProfile,
    lock_in: &LockInReport,
    selected_strategies: &[MoatStrategy],
    tests_passed: bool,
    threshold: MoatImprovementThreshold,
) -> MoatRoundSummary {
    let moat_score_before = ((market.moat_score as i16 + lock_in.lockin_score as i16)
        - (competitor.threat_score as i16 / 2))
        .max(0);
    let strategy_gain: i16 = selected_strategies.iter().map(|item| item.expected_moat_gain).sum();
    let moat_score_after = if tests_passed {
        moat_score_before + strategy_gain
    } else {
        moat_score_before
    };
    let continue_decision = if tests_passed && moat_score_after - moat_score_before >= threshold.0 {
        ContinueDecision::Continue
    } else {
        ContinueDecision::Stop
    };

    MoatRoundSummary {
        round_id,
        selected_strategies: selected_strategies
            .iter()
            .map(|item| item.strategy_id.clone())
            .collect(),
        implemented_specs: Vec::new(),
        tests_passed,
        moat_score_before,
        moat_score_after,
        continue_decision,
        stop_reason: (continue_decision == ContinueDecision::Stop)
            .then(|| "moat improvement below threshold".into()),
        pivot_reason: None,
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

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
git commit -m "feat: add moat round evaluation helpers"
```

### Task 3: Add a bounded moat runtime round with deterministic task-graph execution

**Files:**
- Create: `crates/mdid-runtime/src/moat.rs`
- Modify: `crates/mdid-runtime/src/lib.rs`
- Create: `crates/mdid-runtime/tests/moat_runtime.rs`

- [ ] **Step 1: Write the failing runtime tests**

Create `crates/mdid-runtime/tests/moat_runtime.rs`:

```rust
use mdid_runtime::moat::{run_bounded_round, MoatRoundInput};
use mdid_domain::{ContinueDecision, LockInReport, MarketMoatSnapshot, MoatStrategy, MoatType, ResourceBudget};

#[test]
fn bounded_round_returns_continue_when_gain_exceeds_threshold() {
    let report = run_bounded_round(MoatRoundInput {
        market: MarketMoatSnapshot {
            market_id: "healthcare-deid".into(),
            moat_score: 45,
            ..MarketMoatSnapshot::default()
        },
        competitor: mdid_domain::CompetitorProfile {
            competitor_id: "comp-1".into(),
            threat_score: 30,
            ..mdid_domain::CompetitorProfile::default()
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
            max_review_loops: 1,
        },
        improvement_threshold: 3,
        tests_passed: true,
    });

    assert_eq!(report.summary.continue_decision, ContinueDecision::Continue);
    assert_eq!(report.executed_tasks, vec!["market_scan", "competitor_analysis", "lockin_analysis", "strategy_selection", "evaluation"]);
}

#[test]
fn bounded_round_stops_when_budget_disallows_strategy_work() {
    let report = run_bounded_round(MoatRoundInput {
        market: MarketMoatSnapshot::default(),
        competitor: mdid_domain::CompetitorProfile::default(),
        lock_in: LockInReport::default(),
        strategies: vec![MoatStrategy {
            strategy_id: "data-room".into(),
            title: "Data moat".into(),
            target_moat_type: MoatType::DataMoat,
            implementation_cost: 1,
            expected_moat_gain: 4,
            ..MoatStrategy::default()
        }],
        budget: ResourceBudget {
            max_round_minutes: 10,
            max_parallel_tasks: 1,
            max_strategy_candidates: 0,
            max_spec_generations: 0,
            max_implementation_tasks: 0,
            max_review_loops: 0,
        },
        improvement_threshold: 2,
        tests_passed: true,
    });

    assert_eq!(report.summary.continue_decision, ContinueDecision::Stop);
    assert!(report.stop_reason.unwrap().contains("strategy budget exhausted"));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-runtime --test moat_runtime
```

Expected: FAIL because the moat runtime module does not exist yet.

- [ ] **Step 3: Write the minimal runtime implementation**

Create `crates/mdid-runtime/src/moat.rs`:

```rust
use mdid_application::{evaluate_moat_round, select_top_strategies, MoatImprovementThreshold};
use mdid_domain::{
    CompetitorProfile, LockInReport, MarketMoatSnapshot, MoatRoundSummary, MoatStrategy, ResourceBudget,
};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct MoatRoundInput {
    pub market: MarketMoatSnapshot,
    pub competitor: CompetitorProfile,
    pub lock_in: LockInReport,
    pub strategies: Vec<MoatStrategy>,
    pub budget: ResourceBudget,
    pub improvement_threshold: i16,
    pub tests_passed: bool,
}

#[derive(Debug, Clone)]
pub struct MoatRoundReport {
    pub summary: MoatRoundSummary,
    pub executed_tasks: Vec<String>,
    pub stop_reason: Option<String>,
}

pub fn run_bounded_round(input: MoatRoundInput) -> MoatRoundReport {
    let mut executed_tasks = vec![
        "market_scan".into(),
        "competitor_analysis".into(),
        "lockin_analysis".into(),
    ];

    if input.budget.max_strategy_candidates == 0 {
        let summary = evaluate_moat_round(
            Uuid::new_v4(),
            &input.market,
            &input.competitor,
            &input.lock_in,
            &[],
            input.tests_passed,
            MoatImprovementThreshold(input.improvement_threshold),
        );

        return MoatRoundReport {
            summary,
            executed_tasks,
            stop_reason: Some("strategy budget exhausted".into()),
        };
    }

    let selected = select_top_strategies(
        input.strategies,
        input.budget.max_strategy_candidates as usize,
    );
    executed_tasks.push("strategy_selection".into());

    let summary = evaluate_moat_round(
        Uuid::new_v4(),
        &input.market,
        &input.competitor,
        &input.lock_in,
        &selected,
        input.tests_passed,
        MoatImprovementThreshold(input.improvement_threshold),
    );
    executed_tasks.push("evaluation".into());

    let stop_reason = if matches!(summary.continue_decision, mdid_domain::ContinueDecision::Stop) {
        Some("strategy budget exhausted or moat improvement below threshold".into())
    } else {
        None
    };

    MoatRoundReport {
        summary,
        executed_tasks,
        stop_reason,
    }
}
```

Modify `crates/mdid-runtime/src/lib.rs`:

```rust
pub mod http;
pub mod moat;
```

- [ ] **Step 4: Run the tests to verify they pass**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-runtime --test moat_runtime
cargo test -p mdid-runtime
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-runtime/src/lib.rs crates/mdid-runtime/src/moat.rs crates/mdid-runtime/tests/moat_runtime.rs
git commit -m "feat: add bounded moat round runtime"
```

### Task 4: Expose moat rounds through the CLI

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Create: `crates/mdid-cli/tests/moat_cli.rs`

- [ ] **Step 1: Write the failing CLI tests**

Create `crates/mdid-cli/tests/moat_cli.rs`:

```rust
use std::process::Command;

#[test]
fn cli_runs_moat_round_and_prints_summary() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round"])
        .output()
        .expect("failed to run mdid-cli moat round");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("moat round complete"));
    assert!(stdout.contains("continue_decision"));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli
```

Expected: FAIL because the CLI does not expose a `moat round` command yet.

- [ ] **Step 3: Write the minimal CLI implementation**

Modify `crates/mdid-cli/src/main.rs` to:

```rust
use mdid_domain::{CompetitorProfile, LockInReport, MarketMoatSnapshot, MoatStrategy, MoatType, ResourceBudget};
use mdid_runtime::moat::{run_bounded_round, MoatRoundInput};

fn main() {
    let mut args = std::env::args().skip(1);
    match (args.next().as_deref(), args.next().as_deref()) {
        (Some("status"), _) | (None, _) => println!("med-de-id CLI ready"),
        (Some("moat"), Some("round")) => {
            let report = run_bounded_round(MoatRoundInput {
                market: MarketMoatSnapshot {
                    market_id: "healthcare-deid".into(),
                    industry_segment: "medical de-identification".into(),
                    moat_score: 45,
                    ..MarketMoatSnapshot::default()
                },
                competitor: CompetitorProfile {
                    competitor_id: "comp-1".into(),
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
                    max_review_loops: 1,
                },
                improvement_threshold: 3,
                tests_passed: true,
            });
            println!("moat round complete");
            println!("continue_decision={:?}", report.summary.continue_decision);
            println!("moat_score_before={}", report.summary.moat_score_before);
            println!("moat_score_after={}", report.summary.moat_score_after);
        }
        (Some(other), maybe_second) => {
            let second = maybe_second.unwrap_or("");
            eprintln!("unknown command: {other} {second}".trim());
            std::process::exit(1);
        }
    }
}
```

Also update `crates/mdid-cli/Cargo.toml` if needed to depend on:

```toml
mdid-domain = { path = "../mdid-domain" }
mdid-runtime = { path = "../mdid-runtime" }
```

- [ ] **Step 4: Run the tests to verify they pass**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-cli --test moat_cli
cargo test -p mdid-cli
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs crates/mdid-cli/Cargo.toml
git commit -m "feat: add moat round cli command"
```

### Task 5: Truth-sync docs and verify the workspace slice

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Add README coverage for the moat loop foundation**

Append to `README.md` a section named `## Moat Loop Foundation` describing:

- bounded strategic rounds
- market/competitor/lock-in artifacts
- deterministic moat scoring
- `mdid-cli moat round`

- [ ] **Step 2: Run full slice verification**

Run:

```bash
source "$HOME/.cargo/env"
cargo fmt --all
cargo test -p mdid-domain --test moat_workflow_models
cargo test -p mdid-application --test moat_rounds
cargo test -p mdid-runtime --test moat_runtime
cargo test -p mdid-cli --test moat_cli
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: add moat loop foundation overview"
```

## Self-review checklist

Before executing this plan, verify:

- the plan implements one bounded moat-analysis round, not an unbounded autonomous system
- every new moat type and score field used later is defined in Task 1
- runtime uses explicit budget checks instead of implicit endless loops
- CLI scope stays minimal and local-first
- no task assumes external web crawling or desktop/browser UI exists yet

## Spec coverage for this plan

This plan covers these approved-spec areas only:

- moat-loop domain models
- deterministic moat scoring and continue/stop evaluation
- bounded task-graph runtime for one round
- CLI round execution
- basic doc truth-sync for the first slice

It intentionally defers:

- live market crawling
- browser/desktop moat dashboards
- persistent historical memory storage
- autonomous multi-round continuation
- strategy-to-code generation beyond the first bounded round foundation
