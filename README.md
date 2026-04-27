# med-de-id

Windows-first, local-first medical de-identification platform with a pure Rust core.

## What it is

`med-de-id` is a governed workflow system for reversible medical data de-identification. It is designed for local/offline use, keeps sensitive assets on-device by default, and supports controlled decode/re-identification with audit trails.

The product has three formal surfaces:

1. **CLI** — automation, batch, integration, headless execution
2. **Browser tool** — localhost pipeline/orchestration workbench for workflow composition and scheduling
3. **Desktop app** — sensitive workstation for review, vault operations, decode flows, and audit investigation

## Core workflow

```text
ingest -> extract -> detect -> review -> encode -> export -> decode -> audit
```

## Design principles

- **Pure Rust core**
- **Windows-first**
- **Local-first / offline-capable**
- **Reversible mapping stored in a protected vault**
- **High-risk decode is explicit and auditable**
- **Broad format-family coverage with honest depth grading**
- **Tri-surface product model with layered responsibility**

## Planned format support

| Format family | v1 depth | Notes |
|---|---|---|
| DICOM | L3 | tag-level handling, UID remap, private-tag policy, burned-in suspicion flagging |
| CSV / Excel | L3 | schema-aware reversible mapping and batch consistency |
| PDF / scanned records | L2 | text extraction, OCR, review, governed rewrite/export |
| FCS | L2/L3 metadata-first | TEXT/metadata identifier handling |
| Images | L1 | filename/path/metadata cleanup, OCR-assisted suspicion |
| Videos | L1 | filename/path/container metadata and sidecar handling |

## Architecture overview

```text
shared Rust workspace today
├─ mdid-domain
├─ mdid-vault
├─ mdid-adapters
├─ mdid-application
├─ mdid-runtime
├─ mdid-cli
├─ mdid-browser
└─ mdid-desktop
```

Planned follow-on core crates from the design, not yet implemented in this repository:

- `mdid-policy`
- `mdid-detection`

## Current repository status

This repository currently contains the Slice 1 workspace foundation, the Slice 2 vault MVP, and the first Slice 3 tabular workflow and adapter work.

Implemented so far:

- Shared domain models for pipeline, review, vault mapping, decode requests, audit events, and tabular workflow state
- An encrypted `mdid-vault` crate with local file-backed storage, explicit decode-by-record-id, audit recording, portable subset export, and repeated-value token reuse
- An implemented `mdid-adapters` crate with shared tabular extraction for CSV/XLSX inputs, schema inference, field-level PHI candidate policies, and blank-cell handling parity
- Tabular application orchestration that composes the adapters with vault-backed reversible encoding and honest batch summaries
- Initial `mdid-runtime`, `mdid-cli`, `mdid-browser`, and `mdid-desktop` scaffolding from the foundation slice

Planned next from the design:

- Additional policy and detection crates
- Deeper application orchestration and surface behavior beyond the current scaffolds

Available docs:

- Design spec: `docs/superpowers/specs/2026-04-25-med-de-id-design.md`
- Initial implementation plan: `docs/superpowers/plans/2026-04-25-med-de-id-foundation-implementation-plan.md`
- Slice 2 vault/decode MVP plan: `docs/superpowers/plans/2026-04-25-med-de-id-slice-2-vault-encode-decode-mvp.md`
- Slice 3 tabular deep-support plan: `docs/superpowers/plans/2026-04-25-med-de-id-slice-3-tabular-deep-support.md`

## Moat Loop Foundation

`med-de-id` now includes a local-first moat-loop foundation for deterministic bounded strategy rounds. The shipped slice models market snapshots, competitor profiles, lock-in analysis artifacts, moat strategies, deterministic moat scoring, a bounded control-plane snapshot for canonical task-state inspection, and bounded local round-history persistence/inspection through the CLI.

Run the default bounded round with:

```bash
cargo run -p mdid-cli -- moat round
```

Or run the same deterministic round pipeline from a local JSON file matching `MoatRoundInput`:

```bash
cargo run -p mdid-cli -- moat round --input-path moat-input.json
```

Input-file mode is local-only. The CLI reads the file synchronously, applies the same override flags, prints `input_path=PATH`, and persists history only when `--history-path PATH` is explicitly supplied; it does not crawl data, launch agents, open PRs, create cron jobs, or write artifacts.

The round command prints a deterministic report containing:

- `continue_decision=Continue|Stop|Pivot`
- `executed_tasks=market_scan,competitor_analysis,lockin_analysis,strategy_generation,spec_planning,implementation,review,evaluation`
- `implemented_specs=<none>|moat-spec/<normalized-strategy-id>[,...]`
- `moat_score_before`
- `moat_score_after`
- `stop_reason=<none>|...`

`implemented_specs` is a bounded handoff surface: it exposes normalized stable IDs derived from selected strategy IDs (for example `moat-spec/workflow-audit`). The CLI can now export markdown spec files for the latest persisted round, but it still does **not** automatically dispatch coding/review agents from the CLI output.

Persist the produced round report locally only when you explicitly provide a history path:

```bash
cargo run -p mdid-cli -- moat round --history-path .mdid/moat-history.json
```

When `--history-path PATH` is used, the round output stays the same and adds one extra line:

- `history_saved_to=PATH`

Run bounded stop-path scenarios by overriding the deterministic sample budgets, for example:

```bash
cargo run -p mdid-cli -- moat round --review-loops 0
cargo run -p mdid-cli -- moat control-plane --strategy-candidates 0
```

Inspect the bounded control-plane snapshot with:

```bash
cargo run -p mdid-cli -- moat control-plane
```

Inspect a dry-run control-plane snapshot from a local `MoatRoundInput` JSON file without saving history:

```bash
cargo run -p mdid-cli -- moat control-plane --input-path moat-input.json
```

This input source prints `source=input` and `input_path=PATH` and remains local-only: it does not save history, crawl data, launch agents, open PRs, create cron jobs, or write artifacts.

The control-plane command prints a deterministic snapshot containing:

- `source=sample|input|history`
- `latest_round_id` when inspecting persisted history
- `history_path` when inspecting persisted history
- `input_path` when inspecting a local input file
- `ready_nodes`
- `latest_decision_summary`
- `improvement_delta`
- `agent_assignments=<none>|planner:<node_id>|coder:<node_id>|reviewer:<node_id>[,...]`
- `task_states=market_scan:...,competitor_analysis:...,lockin_analysis:...,strategy_generation:...,spec_planning:...,implementation:...,review:...,evaluation:...`

Inspect the latest persisted moat control-plane snapshot with:

```bash
cargo run -p mdid-cli -- moat control-plane --history-path .mdid/moat-history.json
```

This read-only local operator surface reports the latest persisted task states, ready-node visibility, decision-memory summary, improvement delta, and inspection-only agent assignment projection for ready nodes. `agent_assignments` is a projection only: it does not launch agents, start a daemon, dispatch Planner/Coder/Reviewer work, write code, schedule work, append rounds, crawl the web, or automate code changes.

External controllers can inspect ready tasks in parseable JSON without mutating local history:

```bash
cargo run -p mdid-cli -- moat ready-tasks --history-path .mdid/moat-history.json --role reviewer --format json
```

`moat ready-tasks --format json` is read-only and applies the same filters and limit as text output before emitting a pretty deterministic envelope with `type: "moat_ready_tasks"`, `round_id`, `history_path`, `ready_task_entries`, and `tasks`. Supported routing filters include exact `--round-id`, `--role`, `--kind`, exact `--node-id`, `--depends-on`, `--no-dependencies`, `--requires-artifacts`, case-sensitive `--title-contains`, exact `--spec-ref`, and positive `--limit`; filters are conjunctive and applied before rendering. Each task object includes `role`, `kind`, `node_id`, `title`, and nullable `spec_ref`. Text output remains the default (`--format text`) for backward compatibility.

Export a deterministic work packet and later record the external controller handoff with:

```bash
cargo run -p mdid-cli -- moat work-packet --history-path .mdid/moat-history.json --node-id review --format json
cargo run -p mdid-cli -- moat complete-task --history-path .mdid/moat-history.json --node-id review --artifact-ref plan://review-output --artifact-summary 'Reviewed implementation handoff'
```

`mdid-cli moat work-packet --history-path PATH --node-id NODE_ID [--round-id ROUND_ID] [--format text|json]` exports a deterministic read-only work packet for an external Planner/Coder/Reviewer controller. It includes task metadata, dependency IDs, completed upstream artifact handoffs, acceptance criteria, and the recommended `complete-task` command. It never launches agents, mutates history, schedules work, crawls data, opens PRs, creates cron jobs, or writes artifact files.

Inspect completed artifact handoffs as parseable JSON without mutating local history:

```bash
cargo run -p mdid-cli -- moat artifacts --history-path .mdid/moat-history.json --format json
cargo run -p mdid-cli -- moat artifacts --history-path .mdid/moat-history.json --node-id review --format json
```

`mdid-cli moat artifacts --history-path PATH [--round-id ROUND_ID] [--node-id NODE_ID] [--contains TEXT] [--artifact-ref TEXT] [--artifact-summary TEXT] [--limit N] [--format text|json]` is read-only. Text remains the default. `--format json` emits a deterministic `moat_artifacts` envelope with `round_id`, `history_path`, `artifact_entries`, and one object per completed artifact handoff, including node metadata plus `artifact_ref` and `artifact_summary`. It never launches agents, mutates history, schedules work, crawls data, opens PRs, creates cron jobs, or writes artifact files.

Inspect persisted local history with:

```bash
cargo run -p mdid-cli -- moat history --history-path .mdid/moat-history.json
```

`moat history` is a read-only inspection path: the history file must already exist, and a missing or typoed path fails instead of creating a brand-new empty file.

Use `mdid-cli moat history --history-path PATH [--decision Continue|Stop|Pivot] [--contains TEXT] [--min-score N] [--limit N]` to inspect only persisted history. `--decision Continue|Stop|Pivot` filters detailed history rows by persisted continuation decision. The optional `--min-score N` filter accepts a non-negative integer and keeps entries whose persisted `entry.report.summary.moat_score_after >= N`; it is conjunctive with `--decision` and `--contains`, applies before `--limit`, and never runs rounds, appends history, schedules work, launches agents, opens PRs, or creates cron jobs.

The history command prints a bounded summary containing:

- `entries`
- `latest_round_id`
- `latest_continue_decision`
- `latest_stop_reason`
- `latest_decision_summary`
- `latest_implemented_specs=<none>|moat-spec/<normalized-strategy-id>[,...]`
- `latest_moat_score_after`
- `best_moat_score_after`
- `improvement_deltas`

Inspect the latest persisted round's decision log without running or appending a new round with:

```bash
cargo run -p mdid-cli -- moat decision-log --history-path .mdid/moat-history.json
```

Filter that read-only inspection to one bounded role and/or a decision text substring with:

```bash
cargo run -p mdid-cli -- moat decision-log --history-path .mdid/moat-history.json --role reviewer
cargo run -p mdid-cli -- moat decision-log --history-path .mdid/moat-history.json --contains "approved bounded"
cargo run -p mdid-cli -- moat decision-log --history-path .mdid/moat-history.json --summary-contains "review approved"
```

`moat decision-log` is read-only: the history file must already exist, and it prints `decision_log_entries=N` followed by each persisted decision as `decision=<role>|<summary>|<rationale>`. The `<summary>` and `<rationale>` output fields are escaped for pipe-delimited output (`\\` as `\\\\`, `|` as `\\|`, newline as `\\n`, carriage return as `\\r`). Use `mdid-cli moat decision-log --history-path PATH [--round-id ROUND_ID] [--role planner|coder|reviewer] [--contains TEXT] [--summary-contains TEXT] [--rationale-contains TEXT]` to inspect a persisted round. When `--round-id ROUND_ID` is present, it exact-matches the persisted `entry.report.summary.round_id`, applies before the role/text/summary/rationale/limit filters, and prints exactly `decision_log_entries=0` when no persisted round matches; when absent, decision-log inspects the latest persisted round. The optional `--contains TEXT` filter performs a case-sensitive substring match over each persisted, unescaped decision summary or rationale before rendering; the optional `--summary-contains TEXT` and `--rationale-contains TEXT` filters match persisted, unescaped summary or rationale text only. When combined with `--round-id` and `--role`, filters are conjunctive and must all match. Inspection never runs or appends a new round.

- `mdid-cli moat assignments --history-path PATH [--role planner|coder|reviewer] [--state pending|ready|in_progress|completed|blocked] [--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation] [--node-id NODE_ID] [--assigned-agent-id AGENT_ID] [--title-contains TEXT] [--spec-ref SPEC_REF] [--contains TEXT] [--limit N]` inspects the latest persisted read-only Planner/Coder/Reviewer assignment projection and prints deterministic `assignment=<role>|<node_id>|<title>|<kind>|<spec_ref>` rows. The optional filters are conjunctive; `--state` accepts only the exact persisted task-node state wire values shown in usage and does not normalize input, `--kind` accepts only the exact persisted task-node kind wire values listed in the usage, and `--spec-ref SPEC_REF` performs an exact match against persisted `assignment.spec_ref`. The optional `--contains TEXT` filter performs a case-sensitive substring match over raw persisted assignment `node_id`, `title`, or `spec_ref` before escaping; it is conjunctive with `--role`, `--state`, `--kind`, `--node-id`, `--title-contains`, and `--spec-ref`. The optional `--limit N` filter accepts positive integers only, is applied after all other assignment filters, and keeps the first `N` rows in deterministic persisted assignment order. Inspection opens an existing history file only and never appends or runs a new round. Persisted `node_id`, `title`, and `spec_ref` fields are escaped for pipe-delimited output (`\\` as `\\\\`, `|` as `\\|`, newline as `\\n`, carriage return as `\\r`); bounded enum fields are not escaped. `moat assignments` is read-only and latest-round scoped. `--role` filters by bounded agent role, `--node-id` performs an exact match against the persisted assignment node ID, and `--title-contains TEXT` performs a case-sensitive substring match over persisted assignment titles. `--assigned-agent-id AGENT_ID` exact-matches the persisted task-node `assigned_agent_id` owner, enabling read-only inspection of tasks owned by a persisted local worker ID without spawning, scheduling, or supervising agents. Filters are conjunctive; `--title-contains`, `--spec-ref`, and `--contains` combine with `--role`, `--state`, `--kind`, and `--node-id`, and if no assignment matches, the command prints `assignment_entries=0` and does not error or mutate history. It uses existing moat history only, never creates missing history files, never appends rounds, never schedules work, never launches agents, and never creates cron jobs.

- `mdid-cli moat task-graph --history-path PATH [--round-id ROUND_ID] [--role planner|coder|reviewer] [--state pending|ready|in_progress|completed|blocked] [--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation] [--node-id NODE_ID] [--assigned-agent-id AGENT_ID] [--title-contains TEXT] [--spec-ref SPEC_REF]` inspects the selected persisted task graph read-only, defaulting to the latest persisted round when `--round-id` is absent and exact-matching persisted `entry.report.summary.round_id` when provided, and prints `moat task graph` followed by deterministic `node=<role>|<node_id>|<title>|<kind>|<state>|<dependencies>|<spec_ref>` rows plus adjacent `assigned_agent_id=<node_id>|<owner-or-<none>>` metadata lines. Missing dependency/spec/owner fields print `<none>`, dependency lists are comma-joined, and pipe-delimited string fields are escaped. Filters are conjunctive; `--kind` accepts only the exact persisted task-node kind wire values shown in usage and does not normalize input. `--node-id` matches exact persisted task graph node IDs with no normalization, `--assigned-agent-id AGENT_ID` exact-matches the persisted task-node `assigned_agent_id` owner for read-only inspection of tasks owned by a persisted local worker ID without spawning, scheduling, or supervising agents, `--title-contains` performs case-sensitive substring matching against persisted node titles without normalization or mutation, and `--spec-ref SPEC_REF` exact-matches the persisted optional task graph node `spec_ref` field without matching the escaped output, `<none>`, substrings, or normalized forms. It prints only the header when no persisted node matches. It opens only existing history, so missing paths fail without creating files; inspection is read-only and never appends history, schedules work, launches agents, opens PRs, creates cron jobs, runs agents, or launches background work.

Inspect whether the latest persisted round is eligible to start another bounded round with:

```bash
cargo run -p mdid-cli -- moat continue --history-path .mdid/moat-history.json
```

`moat continue` requires an already-existing history file created by `moat round --history-path ...` and fails for missing paths instead of creating a new history file during inspection.

The continuation command prints a bounded gate summary containing:

- `latest_round_id`
- `latest_continue_decision`
- `latest_tests_passed`
- `latest_improvement_delta`
- `latest_stop_reason`
- `evaluation_completed=true|false`
- `can_continue=true|false`
- `reason`
- `required_improvement_threshold`

This is an inspection surface only. It does not auto-schedule or launch the next round.

Schedule exactly one next bounded round when the continuation gate allows it with:

```bash
cargo run -p mdid-cli -- moat schedule-next --history-path .mdid/moat-history.json
```

`moat schedule-next` is a one-shot local scheduler control: it requires an existing history file, checks the same continuation gate as `moat continue`, appends one deterministic bounded round only when `can_continue=true`, and otherwise leaves history unchanged. It does not create a cron job, background daemon, live crawler, or unrestricted autonomous loop.

Export the latest persisted implemented-spec handoffs as markdown with:

```bash
cargo run -p mdid-cli -- moat export-specs --history-path .mdid/moat-history.json [--round-id ROUND_ID] --output-dir .mdid/moat-specs
```

`moat export-specs` requires an already-existing history file, fails when the history is empty, fails when the selected round has no `implemented_specs` handoffs, creates the output directory when needed, and writes one markdown file per handoff such as `workflow-audit.md` for `moat-spec/workflow-audit`. By default it exports the latest persisted round; pass `--round-id ROUND_ID` to replay a prior handoff round by exact persisted round id.

The export command prints a deterministic summary containing:

- `round_id`
- `exported_specs=<comma-list>`
- `written_files=<comma-list>`

Export deterministic implementation-plan markdown for the latest persisted handoffs with:

```bash
cargo run -p mdid-cli -- moat export-plans --history-path .mdid/moat-history.json [--round-id ROUND_ID] --output-dir docs/superpowers/plans/generated
```

`moat export-plans --history-path PATH [--round-id ROUND_ID] --output-dir DIR` is also one-shot and local: it requires an already-existing history file, fails when the history is empty or the selected round has no `implemented_specs` handoffs, creates the output directory when needed, and writes one `*-implementation-plan.md` file per handoff. By default it exports the latest persisted round; pass `--round-id ROUND_ID` to replay a prior handoff round by exact persisted round id. It does not start background agents, create cron jobs, open PRs, or run an unrestricted autonomous loop.

This foundation is still intentionally narrow. It now supports bounded local JSON-backed history persistence and inspection, inspection-only continuation-gate reporting, one-shot bounded local scheduler control via `moat schedule-next`, and markdown export of latest or exact prior persisted moat-spec handoffs plus implementation plans, but it still does not perform live market crawling, background scheduler/daemon control, PR automation, or a full autonomous multi-agent runtime over external data.

External workers can claim one ready task with a bounded dispatch envelope:

```bash
cargo run -p mdid-cli -- moat dispatch-next --history-path .mdid/moat-history.json --role coder --agent-id coder-7
cargo run -p mdid-cli -- moat dispatch-next --history-path .mdid/moat-history.json --role coder --agent-id coder-7 --format json
```

`--agent-id AGENT_ID` is optional local persisted ownership metadata only. Text output preserves `agent_id=<value|<none>>` request attribution and reports persisted ownership separately as `assigned_agent_id=<value|<none>>`; dry-runs never persist ownership, so `assigned_agent_id=<none>` even when `--agent-id` is supplied. JSON output includes nullable `agent_id` and `assigned_agent_id` fields, with `assigned_agent_id: null` on dry-run. `moat dispatch-next` selects and, unless `--dry-run` is used, claims exactly one already-ready persisted task; it does not launch agents, open PRs, create schedulers or cron jobs, start background work, crawl the web, or run an autonomous loop.

External workers can claim tasks with optional local ownership metadata via `mdid-cli moat claim-task --history-path PATH --node-id NODE_ID [--round-id ROUND_ID] [--agent-id AGENT_ID] [--lease-seconds N] [--format text|json]`; text output remains the default and includes `assigned_agent_id=<value|<none>>`, while `--format json` emits a deterministic pretty `moat_claim_task` envelope. The CLI only records metadata/state in local history. External workers can complete claimed tasks and optionally hand off produced artifacts with paired flags:

```bash
cargo run -p mdid-cli -- moat complete-task --history-path .mdid/moat-history.json --node-id review --artifact-ref docs/review.md --artifact-summary "review approved"
```

External controllers that need parseable completion, artifact handoff, and downstream routing metadata from one local mutation can request the JSON envelope:

```bash
cargo run -p mdid-cli -- moat complete-task \
  --history-path .mdid/moat-history.json \
  --node-id spec-workflow-audit \
  --artifact-ref docs/superpowers/specs/workflow-audit.md \
  --artifact-summary "Workflow audit spec drafted" \
  --format json
```

`--artifact-ref` and `--artifact-summary` must be supplied together. When present, the completed task records the handoff in persisted history and the deterministic completion output includes `artifact_recorded=true`, `artifact_ref=...`, and `artifact_summary=...` before `next_ready_task_entries`. Without those flags, completion preserves the prior transition behavior and prints `<none>` artifact fields. Text output remains the default (`--format text`); `--format json` emits `type`, `round_id`, `history_path`, `node_id`, `previous_state`, `new_state`, `artifact_recorded`, nullable `artifact`, `next_ready_task_entries`, and `next_ready_tasks[]`.

## Roadmap shape

- **v1**: governed workflow core, vault/decode controls, audit trail, tri-surface skeleton, deep CSV/Excel + DICOM tag-level support, medium PDF/OCR support, conservative image/video/FCS support
- **v1.5**: detection quality/provenance upgrades, PDF/DICOM policy depth, parity and workflow polish
- **v2**: AI/NLP detectors, stronger media handling, richer custom node/plugin model, enterprise controls

## Repo conventions

- Planning and design docs live under `docs/superpowers/`
- Implementation is expected to follow TDD and small verified slices
- The browser tool is local-first and served on `127.0.0.1`, not a SaaS deployment

## License

Workspace metadata is currently marked `UNLICENSED`.

Moat task leases: local external controllers coordinate claims via history-file lease metadata (`claimed_at`, `lease_expires_at`, `last_heartbeat_at`). Use `mdid-cli moat dispatch-next --history-path PATH [--agent-id AGENT_ID] [--format text|json]` to select one ready task and optionally claim it; text remains default. `--format json` emits a deterministic pretty `moat_dispatch_next` envelope with `type`, `dry_run`, `claimed`, nullable request `agent_id`, nullable persisted `assigned_agent_id`, `round_id`, `node_id`, `role`, `kind`, `title`, `dependencies`, nullable `spec_ref`, and `complete_command`; non-dry-run claims also include `previous_state`, `new_state`, and numeric `lease_seconds`. Use `mdid-cli moat heartbeat-task --history-path PATH --node-id NODE_ID [--round-id ROUND_ID] [--agent-id AGENT_ID] [--lease-seconds N] [--format text|json]` to renew and `mdid-cli moat reap-stale-tasks --history-path PATH [--round-id ROUND_ID] [--now RFC3339] [--format text|json]` to requeue expired in-progress local claims; text remains default, and JSON emits deterministic pretty `moat_heartbeat_task` / `moat_reap_stale_tasks` envelopes. This is not a daemon/crawler/PR automation loop and does not launch agents, schedule background work, create cron jobs, crawl data, open PRs, or write artifacts.

### Moat task event inspection

`mdid-cli moat task-events --history-path PATH` opens an existing moat history file without mutating it and prints the latest round task lifecycle events. Use `--round-id` for exact round selection, and combine `--node-id`, `--action claim|heartbeat|reap|complete|release|block|unblock`, `--agent-id`, `--contains`, and positive `--limit` filters. Text output remains the default (`--format text`) and emits rows as `task_event=<action>|<node_id>|<agent-or-<none>>|<summary>`.

Use `--format json` for machine-readable output after the same selection, filters, and limit are applied. The pretty JSON envelope has `type: "moat_task_events"`, `round_id`, `history_path`, `task_event_entries`, and an `events` array. Each event object includes `recorded_at`, `node_id`, `action`, `previous_state`, `new_state`, `agent_id`, `lease_expires_at`, `artifact_ref`, `artifact_summary`, and `reason`, with unavailable optional fields emitted as `null`.
