# med-de-id Moat Loop Design Specification

**Date:** 2026-04-25  
**Status:** Draft approved from direct user requirements; written spec for implementation  
**Product area:** `med-de-id` strategic intelligence + autonomous moat optimization loop

## 1. Problem Statement

`med-de-id` currently has product and technical planning, but it does not yet have a built-in system that continuously:

- scans the market
- analyzes competitors
- evaluates lock-in structures
- generates moat strategies
- turns selected strategies into executable specs/plans
- drives implementation through SDD + TDD
- measures whether the product's moat is improving over time

The user wants this to become a first-class autonomous capability rather than an ad hoc research habit.

## 2. Goal

Build a **local-first strategic intelligence and moat-evolution loop** for `med-de-id` that can repeatedly:

1. analyze the market structure of the target industry
2. track competitors and infer moat patterns
3. evaluate switching costs / data lock-in / workflow dependence
4. generate actionable moat strategies
5. convert selected strategies into explicit specs and tests
6. drive implementation through SDD + TDD
7. score moat improvement after each round
8. continue iterating while gains justify further investment

## Implementation Status

### Shipped foundation slice in the repository

The repository currently contains a bounded local foundation for moat-loop execution:

- domain models in `mdid-domain` for market structure, competitor intelligence, lock-in analysis, moat strategies, round summaries, task-graph nodes, and decision-memory snapshots
- deterministic moat evaluation helpers in `mdid-application`
- bounded round orchestration in `mdid-runtime`
- a deterministic sample `mdid-cli moat round` path for inspecting one bounded round contract with canonical executed task IDs: planner-owned `market_scan`, `competitor_analysis`, `lockin_analysis`, `strategy_generation`, and `spec_planning`; coder-owned `implementation`; reviewer/evaluation `review` and `evaluation`
- a deterministic sample `mdid-cli moat control-plane` path for inspecting the bounded control-plane snapshot with canonical task states, ready-node visibility, and the latest bounded decision-memory summary

This shipped slice is intentionally narrower than the full autonomous moat-loop vision. It provides a deterministic single-round foundation for evaluating and inspecting moat work locally through both the round report and control-plane snapshot, but the CLI is still a canned sample round rather than a general operator-facing runner over user-supplied or persisted data.

### Still planned, not yet implemented

The broader Autonomous Multi-Agent System target described by this spec remains future work. This spec still includes, but the current repository does not yet implement:

- Planner / Coder / Reviewer role orchestration
- persistent memory store and decision log
- non-linear task graph persistence and scheduler control
- GitFlow PR / release automation
- live market / competitor / lock-in data collection
- continuous improvement loop stopping on resource or improvement thresholds

## 3. Scope and Product Positioning

This is **not** a generic business-automation engine for arbitrary companies.

This feature is specifically for `med-de-id` and adjacent product strategy workflows. It should help the project make better product decisions and prioritize defensible features.

### 3.1 v1 scope

v1 will support:

- local market snapshots
- competitor profile tracking
- lock-in analysis artifacts
- moat scoring
- strategy proposal generation
- bounded execution loops with explicit budgets
- spec/plan generation inputs for engineering work
- decision logging and historical comparison

### 3.2 v1 non-goals

v1 will not attempt:

- fully autonomous internet-scale crawling of the entire market
- live pricing intelligence across every competitor every minute
- unsupervised deployment to production
- unrestricted self-modifying code outside repo constraints
- cloud-only orchestration
- replacing human product judgment with opaque black-box decisions

## 4. High-Level Model

The feature is a **task-graph-based strategic loop**, not a single linear script.

### 4.1 Core loop stages

1. **Market Scan**
2. **Competitor Analysis**
3. **Lock-in Analysis**
4. **Moat Strategy Generation**
5. **Spec/Plan Generation**
6. **TDD/SDD Implementation Execution**
7. **Evaluation and Improvement Scoring**
8. **Loop Continue / Stop Decision**

### 4.2 Task graph principle

The system must model work as a graph of dependent tasks rather than a fixed pipeline. This allows:

- market scan and competitor scan to run in parallel
- strategy generation to depend on multiple upstream artifacts
- implementation tasks to branch into multiple independent sub-plans
- evaluation to feed back into future market scans or strategy generation

## 5. Key Concepts and Outputs

### 5.1 Market structure analysis

The system must analyze and store at least:

- market concentration
- entry barriers
- regulatory burden
- network effects
- distribution advantage
- compliance asymmetry
- switching friction
- incumbent data advantage

Output fields:

- `market_id`
- `industry_segment`
- `market_snapshot_at`
- `moat_score`
- `moat_type`
- `confidence`
- `evidence[]`
- `assumptions[]`

### 5.2 Competitor analysis

The system must track competitor-level intelligence including:

- product surface
- pricing model
- packaging/tiering
- feature movement
- integration footprint
- hiring signals
- messaging shifts
- implied moat pattern

Output fields:

- `competitor_id`
- `name`
- `category`
- `pricing_summary`
- `feature_summary`
- `talent_signal_summary`
- `suspected_moat_types[]`
- `threat_score`
- `evidence[]`

### 5.3 Lock-in analysis

The system must evaluate:

- switching costs
- data lock-in
- workflow dependency
- integration coupling
- audit/compliance dependency
- training/process lock-in

Output fields:

- `lockin_score`
- `lockin_vectors[]`
- `switching_cost_strength`
- `data_gravity_strength`
- `workflow_dependency_strength`
- `portability_risk`
- `evidence[]`

### 5.4 Moat strategy generation

The system must produce executable moat options, for example:

- data advantage strategies
- compliance moat strategies
- workflow lock-in strategies
- ecosystem/integration strategies
- network-effect-adjacent strategies
- review/audit governance moat strategies

Each strategy must include:

- `strategy_id`
- `title`
- `rationale`
- `target_moat_type`
- `implementation_cost`
- `expected_moat_gain`
- `risk_level`
- `dependencies[]`
- `testable_hypotheses[]`

### 5.5 Evaluation outputs

Every round must produce:

- `round_id`
- `selected_strategies[]`
- `implemented_specs[]`
- `tests_passed`
- `moat_score_before`
- `moat_score_after`
- `moat_score_improvement`
- `continue_decision`
- `stop_reason` or `pivot_reason`

## 6. Operating Rules

### 6.1 Bounded autonomy

The loop must not run without limits. Every execution round must respect:

- max iteration count
- time budget
- token/analysis budget
- implementation budget
- stop threshold for low improvement

### 6.2 Safe continuation logic

A new loop round may start only if:

- the previous round completed evaluation
- required tests passed
- no critical safety rollback condition triggered
- `moat_score_improvement >= threshold`, or a pivot rule explicitly says to continue with a new direction

### 6.3 Rollback and safety

The system must support:

- reverting a bad strategy choice
- marking a round as invalid
- keeping decision logs immutable enough for auditability
- never promoting speculative code to stable branches without verification

## 7. Relationship to SDD + TDD

This feature does not replace SDD/TDD. It **feeds** them.

### 7.1 SDD responsibilities

For any selected moat strategy, the loop must generate or update:

- spec inputs
- constraints
- success criteria
- output contracts
- dependency graph

### 7.2 TDD responsibilities

Before implementation of any selected strategy slice:

- tests must be generated or defined first
- the first implementation step must observe a failing test
- no round may claim successful feature delivery unless the relevant tests pass

### 7.3 Review gate

A strategy is not considered implemented merely because code changed. It must satisfy:

- spec compliance
- code quality review
- test pass criteria
- round evaluation update

## 8. Architecture

## 8.1 New subsystem

Introduce a new product subsystem tentatively named **Moat Loop Engine**.

Recommended logical components:

- `moat-domain`
- `moat-analysis`
- `moat-strategy`
- `moat-runtime`
- `moat-memory`
- `moat-reporting`

These may become crates or modules depending on the implementation slice.

## 8.2 Component responsibilities

### `moat-domain`
Defines shared models:

- market snapshot
- competitor profile
- lock-in report
- moat strategy
- round result
- resource budget
- continue/stop decision

### `moat-analysis`
Produces upstream intelligence artifacts:

- market scanner
- competitor scanner
- lock-in evaluator
- evidence normalization

### `moat-strategy`
Produces and ranks moat strategies.

### `moat-runtime`
Executes the task graph and enforces budgets / loop policy.

### `moat-memory`
Stores prior rounds, evidence, strategy history, and decision rationale.

### `moat-reporting`
Generates summaries for CLI / browser / desktop surfaces.

## 9. Data Flow

### 9.1 Round execution flow

```text
market scan + competitor scan + lock-in scan
            -> evidence normalization
            -> moat strategy generation
            -> strategy selection
            -> spec/plan generation
            -> TDD implementation execution
            -> verification
            -> moat improvement evaluation
            -> continue / stop / pivot
```

### 9.2 Feedback loop

Evaluation outputs are stored and fed back into later rounds so the system can:

- avoid repeating bad ideas
- compare moat gains across strategies
- discover which moat types compound best
- decide whether to deepen the same direction or pivot

## 10. Integration with med-de-id Product Surfaces

### 10.1 CLI

CLI must support:

- run a bounded moat round
- inspect round history
- export strategy reports
- score current moat assumptions

### 10.2 Browser tool

Browser tool must support:

- task graph visualization
- strategy graph / dependency view
- round monitoring
- evidence/score dashboards

### 10.3 Desktop app

Desktop app should eventually support:

- strategic review workspace
- evidence inspection
- strategy comparison
- implementation handoff traceability

v1 may keep the desktop part lighter than CLI/browser if needed, but the underlying capability model must remain shared.

## 11. Persistence and Memory

The system must persist:

- raw evidence references
- normalized analytical outputs
- strategy proposals
- selected strategy sets
- implementation outcomes
- moat score history
- round decisions and rationale

The memory layer must support longitudinal analysis such as:

- “which moat type improved fastest over the last N rounds?”
- “which competitor signals most often triggered useful strategies?”
- “which strategy classes repeatedly fail verification?”

## 12. Scoring Model

### 12.1 `moat_score`

`moat_score` is a composite score, not a single subjective label.

Suggested v1 subcomponents:

- market defensibility
- compliance defensibility
- data advantage strength
- workflow lock-in strength
- ecosystem leverage
- differentiation durability

### 12.2 `moat_type`

`moat_type` may be one or more of:

- `compliance_moat`
- `data_moat`
- `workflow_lockin`
- `ecosystem_moat`
- `distribution_moat`
- `network_effect_adjacent`
- `brand_trust_moat`

### 12.3 Improvement threshold

The loop must compare `moat_score_after - moat_score_before` against a configurable threshold.

If improvement is below threshold, the runtime must either:

- stop the loop, or
- pivot exploration direction

## 13. Resource Budget Model

Each round must declare a budget such as:

- `max_round_minutes`
- `max_parallel_tasks`
- `max_strategy_candidates`
- `max_spec_generations`
- `max_implementation_tasks`
- `max_review_loops`

This prevents unbounded autonomous churn.

## 14. v1 Execution Strategy

v1 should not attempt to build the full self-improving engine in one batch.

Instead, implement in slices:

1. domain + persistence for market/competitor/lock-in/moat outputs
2. deterministic moat scoring and round evaluation
3. task-graph runtime with bounded rounds
4. strategy-to-spec/plan handoff
5. CLI/browser reporting and control

## 15. Acceptance Criteria

The feature is acceptable only if:

- it can run at least one bounded strategic round end-to-end
- it outputs market, competitor, lock-in, and moat strategy artifacts
- it records round decisions and evidence
- it computes `moat_score_before`, `moat_score_after`, and `moat_score_improvement`
- it can stop or pivot when improvement is below threshold
- it can hand selected strategies into explicit spec/plan generation
- all implemented round behaviors are test-covered and pass

## 16. Non-Goals for Initial Implementation Plan

The first implementation plan should not attempt:

- full web crawling breadth
- advanced ML-based market inference
- complete autonomous code generation for every strategy type
- production-grade desktop strategy UI
- unrestricted perpetual loops

## 17. Final Design Statement

`med-de-id` will gain a **local-first moat loop engine** that continuously analyzes market structure, competitor behavior, and user lock-in patterns, converts the strongest opportunities into executable specs and tests, and iterates only while verified moat strength improves under explicit resource limits.
