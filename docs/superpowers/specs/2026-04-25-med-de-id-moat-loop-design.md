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
- a reusable local JSON-backed history store in `mdid-runtime` exposed as `mdid_runtime::moat_history::LocalMoatHistoryStore`, with `open(path)` for persistence paths that may need to create a new file, `open_existing(path)` for honest read-only inspection of an already-persisted history file, `append(recorded_at, report)` for persistence, and `summary()` for bounded inspection over persisted `MoatHistoryEntry` records
- a bounded operator-facing `mdid-cli moat round` runner over deterministic sample inputs, including override flags for strategy/spec/implementation/review budgets plus `tests_passed`, canonical executed task IDs, surfaced `implemented_specs` handoff IDs, honest `stop_reason` reporting, and optional `--history-path PATH` persistence
- a bounded operator-facing `mdid-cli moat control-plane` runner over deterministic sample inputs, plus read-only `--history-path PATH` inspection of the latest persisted control-plane snapshot, including task states, ready-node visibility, bounded decision-memory summary, and inspection-only `agent_assignments` projection for ready nodes
- a bounded operator-facing `mdid-cli moat history --history-path PATH [--round-id ROUND_ID] [--decision Continue|Stop|Pivot] [--contains TEXT] [--stop-reason-contains TEXT] [--min-score N] [--tests-passed true|false] [--limit N]` summary surface for inspecting persisted local history, including the latest surfaced `implemented_specs` handoff IDs as `latest_implemented_specs`; filters combine conjunctively across `--round-id`, `--decision`, `--contains`, `--stop-reason-contains`, `--min-score`, and `--tests-passed`; `--round-id ROUND_ID` is read-only, exact-matches the persisted `entry.report.summary.round_id`, and applies before `--limit`; `--min-score N` is read-only, accepts a non-negative integer, filters persisted entries where `entry.report.summary.moat_score_after >= N`, applies before `--limit`; `--tests-passed true|false` is read-only, accepts exact boolean values, filters persisted entries where `entry.report.summary.tests_passed` equals the requested value, applies before `--limit`, and never runs rounds, appends history, schedules work, launches agents, opens PRs, or creates cron jobs
- a read-only `mdid-cli moat decision-log --history-path PATH [--round-id ROUND_ID] [--role planner|coder|reviewer] [--contains TEXT] [--summary-contains TEXT] [--rationale-contains TEXT]` inspection surface that requires an already-persisted history file, inspects the latest persisted round when `--round-id` is absent, exact-matches persisted `entry.report.summary.round_id` when `--round-id ROUND_ID` is present, applies round selection before role/text/summary/rationale/limit filters, prints exactly `decision_log_entries=0` when no persisted round matches, and never runs or appends a new round; it optionally filters decisions by Planner/Coder/Reviewer role, a case-sensitive substring over persisted unescaped decision summary/rationale (`--contains`), a case-sensitive substring over persisted unescaped decision summary only (`--summary-contains`), and/or a case-sensitive substring over persisted unescaped decision rationale only (`--rationale-contains TEXT`); `--round-id`, `--role`, `--contains`, `--summary-contains`, and `--rationale-contains` combine conjunctively, and prints each persisted decision as `decision=<role>|<summary>|<rationale>` with summary/rationale escaped for pipe-delimited output (`\\`, `|`, newline, and carriage return become `\\\\`, `\\|`, `\\n`, and `\\r`)
- `mdid-cli moat assignments --history-path PATH [--round-id ROUND_ID] [--role planner|coder|reviewer] [--state pending|ready|in_progress|completed|blocked] [--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation] [--node-id NODE_ID] [--depends-on NODE_ID] [--title-contains TEXT] [--spec-ref SPEC_REF] [--contains TEXT] [--limit N]` is a read-only latest-round inspection surface for persisted `agent_assignments`. It prints bounded `assignment=<role>|<node_id>|<title>|<kind>|<spec_ref>` rows for operators and future SDD handoff tooling; persisted string fields (`node_id`, `title`, `spec_ref`) are escaped for pipe-delimited output (`\\`, `|`, newline, and carriage return become `\\\\`, `\\|`, `\\n`, and `\\r`). It inspects the latest persisted `agent_assignments` rows only from an existing history file unless `--round-id ROUND_ID` is provided; `--round-id` is read-only, exact-matches the persisted `entry.report.summary.round_id`, selects that persisted round before assignment projection, and absence preserves latest-round behavior. `--kind` accepts only exact persisted task-node kind wire values, `--node-id` uses exact persisted node ID matching, `--depends-on NODE_ID` finds the persisted selected-round task-graph node whose `node_id` matches each assignment and keeps only assignments whose persisted dependency list contains the requested upstream node ID exactly, `--title-contains` performs a case-sensitive substring match over persisted assignment titles, `--spec-ref` performs an exact match against persisted `assignment.spec_ref`, and `--contains TEXT` performs a case-sensitive substring match over raw persisted assignment `node_id`, `title`, or `spec_ref` before escaping. Filters combine conjunctively with `--role`, `--state`, `--kind`, `--node-id`, `--depends-on`, `--title-contains`, `--spec-ref`, and `--contains`. `--limit N` accepts positive integers only, applies after all other filters including `--depends-on`, and keeps the first `N` rows in deterministic persisted assignment order. No matches return `assignment_entries=0` without error. It does not mutate history, append rounds, schedule work, launch agents, crawl data, open PRs, or create cron jobs.
- `mdid-cli moat task-graph --history-path PATH [--round-id ROUND_ID] [--role planner|coder|reviewer] [--state pending|ready|in_progress|completed|blocked] [--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation] [--node-id NODE_ID] [--title-contains TEXT] [--spec-ref SPEC_REF]` is a read-only inspection surface for the persisted control-plane task graph, defaulting to the latest persisted round when `--round-id` is absent and exact-matching persisted `entry.report.summary.round_id` when provided. It opens only existing history and prints `node=<role>|<node_id>|<title>|<kind>|<state>|<dependencies>|<spec_ref>` rows with `<none>` for empty dependency/spec fields, comma-joined dependency node IDs, optional role/state/kind filtering, optional exact persisted `--node-id` matching with no normalization, optional case-sensitive `--title-contains` substring matching against persisted node titles without normalization or mutation, and optional `--spec-ref SPEC_REF` exact matching against raw persisted `node.spec_ref.as_deref()` without comparing escaped output, matching `<none>`, substring matching, or normalization. Escaping applies only to pipe-delimited output fields. Filters are conjunctive. `--kind` accepts only exact persisted task-node kind wire values without normalization. When no persisted node matches the combined filters, it succeeds with only the `moat task graph` header and never appends, schedules, runs agents, or launches background work. It does not mutate history, append rounds, schedule work, launch agents, crawl data, open PRs, or create cron jobs.
- `mdid-cli moat ready-tasks --history-path PATH [--round-id ROUND_ID] [--role planner|coder|reviewer] [--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation] [--node-id NODE_ID] [--limit N]` is a bounded latest-round routing surface for autonomous controllers. It opens only an existing history file, selects the latest persisted round unless `--round-id` exact-matches a specific persisted round, derives immediately claimable nodes from `task_graph.ready_node_ids()`, applies optional round/role/kind and read-only exact persisted `--node-id NODE_ID` filters conjunctively before `--limit`, and prints `ready_task=<role>|<kind>|<node_id>|<title>|<spec_ref>` rows without launching agents or mutating history.
- `mdid-cli moat claim-task --history-path PATH --node-id NODE_ID [--round-id ROUND_ID]` is a bounded local coordination mutation for external Planner/Coder/Reviewer controllers. It opens only an existing history file, selects the latest persisted round unless `--round-id` exact-matches a specific persisted round, reloads the latest on-disk history before mutation to avoid stale-handle double claims, requires the selected node to be `ready`, persists only that node's state transition to `in_progress`, and prints stable claim metadata (`round_id`, `node_id`, `previous_state=ready`, `new_state=in_progress`, and `history_path`). It does not run agents, append rounds, schedule work, crawl data, open PRs, or create cron jobs.
- `mdid-cli moat complete-task --history-path PATH --node-id NODE_ID [--round-id ROUND_ID] [--artifact-ref TEXT --artifact-summary TEXT]` is a bounded local coordination mutation for external Planner/Coder/Reviewer controllers. It opens only an existing history file, selects the latest persisted round unless `--round-id` exact-matches a specific persisted round, reloads latest on-disk history before mutation, requires the selected node to be `in_progress`, persists only that task transition to `completed`, and optionally records a paired artifact handoff on the completed task when both `--artifact-ref` and `--artifact-summary` are supplied. Its deterministic output includes `artifact_recorded=true|false`, `artifact_ref=<none>|...`, and `artifact_summary=<none>|...` before downstream routing rows, then reloads the selected persisted round and prints `next_ready_task_entries=N` plus `next_ready_task=<role>|<node_id>|<title>|<kind>|<spec_ref>` for any newly/currently ready nodes exposed by dependency resolution. It leaves task execution/artifact generation to the external worker and does not launch agents, schedule work, append rounds, open PRs, create cron jobs, crawl data, or write artifact files.
- `mdid-cli moat artifacts --history-path PATH [--round-id ROUND_ID] [--node-id NODE_ID] [--contains TEXT] [--limit N]` is a read-only inspection surface for completed task artifact handoffs persisted in the selected round's task graph. It opens only an existing history file, selects the latest persisted round unless `--round-id` exact-matches a specific persisted round, filters exact node IDs and case-sensitive raw node/ref/summary text, applies `--limit` after filtering, escapes pipe-delimited output fields, and never mutates history, appends rounds, schedules work, launches agents, opens PRs, creates cron jobs, crawls data, or writes artifact files.
- a bounded operator-facing `mdid-cli moat continue --history-path PATH [--improvement-threshold N]` gate that truthfully reports whether the latest persisted round completed evaluation and cleared the configured continuation threshold, while requiring an already-persisted history file and failing for missing paths instead of creating a new one during inspection
- a bounded operator-facing `mdid-cli moat schedule-next --history-path PATH [--improvement-threshold N]` one-shot scheduler control that requires an already-persisted history file, checks the same continuation gate as `moat continue`, appends exactly one deterministic bounded round only when `can_continue=true`, and otherwise leaves history unchanged
- a bounded operator-facing `mdid-cli moat export-specs --history-path PATH [--round-id ROUND_ID] --output-dir DIR` export surface that reads the latest persisted round by default or selects an exact persisted round with `--round-id ROUND_ID`, requires a pre-existing history file, fails for empty history or missing selected `implemented_specs` handoffs, creates the output directory as needed, and writes one markdown file per selected handoff via `mdid_application::render_moat_spec_markdown`
- a bounded operator-facing `mdid-cli moat export-plans --history-path PATH [--round-id ROUND_ID] --output-dir DIR` export surface that reads the latest persisted round by default or selects an exact persisted round with `--round-id ROUND_ID`, requires a pre-existing history file, fails for empty history or missing selected `implemented_specs` handoffs, creates the output directory as needed, and writes one deterministic implementation-plan markdown file per selected handoff via `mdid_application::render_moat_plan_markdown`

This shipped slice is intentionally narrower than the full autonomous moat-loop vision. It provides a deterministic local round runner, bounded control-plane inspection, bounded local history persistence/inspection, read-only latest-round decision-log inspection, an inspection-only continuation gate over deterministic sample data, one-shot bounded local scheduler control, bounded ready-task routing, persisted ready-task claiming, claimed-task completion with downstream ready-task summary output, read-only artifact handoff inspection, and bounded markdown export for the latest or exact prior persisted `implemented_specs` handoff IDs such as `moat-spec/workflow-audit`. Deterministic implementation-plan markdown export now exists for those handoffs, and control-plane `agent_assignments` are read-only projections only; `ready-tasks`, `claim-task`, and `complete-task` provide external-controller coordination but do not launch agents, start a daemon, dispatch Planner/Coder/Reviewer work, generate artifacts, or write code. Full autonomous Planner/Coder/Reviewer orchestration remains future work; there is no background scheduler/daemon, no live market crawling, and no full autonomous multi-agent runtime over user-supplied or external inputs.

### Still planned, not yet implemented

The broader Autonomous Multi-Agent System target described by this spec remains future work. This spec still includes, but the current repository does not yet implement:

- Planner / Coder / Reviewer role orchestration
- full persistent memory store and decision-log workflow beyond bounded local round-history snapshots
- non-linear task graph persistence and background scheduler/daemon control
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
- `implemented_specs[]` (bounded normalized handoff IDs such as `moat-spec/workflow-audit`)
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
