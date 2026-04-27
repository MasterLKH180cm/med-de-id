# med-de-id Development Rules

## Core constraints
- Follow the approved spec in `docs/superpowers/specs/2026-04-25-med-de-id-design.md`.
- Follow implementation plans under `docs/superpowers/plans/`.
- Prefer small, verifiable slices.
- Use TDD for feature and bugfix work.
- Keep the product local-first, Windows-first, and pure-Rust-core.

## TDD rule
- No production behavior code without a failing test first.
- For each behavior change: RED -> GREEN -> REFACTOR.
- Run targeted tests first, then relevant broader tests.

## Narrow bootstrap exception
A narrow bootstrap exception is allowed only for initial greenfield scaffold files that must exist before meaningful tests can run.

Allowed under this exception:
- Cargo workspace manifests
- toolchain configuration
- empty or minimal crate entry points required to make tests runnable
- CI skeleton wiring

Not allowed under this exception:
- feature logic
- vault logic
- adapter behavior
- detection behavior
- decode behavior
- review behavior beyond minimal shells required to compile

As soon as the scaffold is runnable, return to strict TDD immediately.

## Product-surface rules
- Browser tool is the pipeline/orchestration surface.
- Desktop app is the sensitive workstation surface.
- CLI is the automation surface.
- Shared domain/application/runtime semantics must stay consistent across surfaces.

Moat task leases are local deterministic history-file coordination for external controllers only; heartbeat/reap commands must not be treated as daemon, crawler, or PR automation.

Moat input-file mode is local-only: `mdid-cli moat round --input-path PATH` and `mdid-cli moat control-plane --input-path PATH` read a JSON `MoatRoundInput`, apply explicit override flags, and run the same bounded deterministic pipeline. Input-file mode must not crawl data, launch agents, open PRs, create cron jobs, schedule background work, or write artifacts; `moat round` persists only when `--history-path PATH` is also supplied, while `moat control-plane --input-path` remains an inspection run.

## Moat task events

Task lifecycle commands append deterministic task graph events for claim, heartbeat, reap, complete, release, block, and unblock. `claim-task`, `heartbeat-task`, and `reap-stale-tasks` are local history-file coordination commands for external controllers only; each supports `--format text|json`, with text default unchanged and JSON emitted as deterministic pretty envelopes. Inspect lifecycle events with `mdid-cli moat task-events --history-path PATH`; this command is read-only, defaults to the latest round, supports exact `--round-id`, conjunctive filters, and prints `task_event_entries=0` when no round/events match. Text output is the default (`--format text`). `--format json` emits a pretty deterministic envelope with `type: "moat_task_events"`, `round_id`, `history_path`, `task_event_entries`, and `events`; each event includes `recorded_at`, `node_id`, `action`, `previous_state`, `new_state`, `agent_id`, `lease_expires_at`, `artifact_ref`, `artifact_summary`, and `reason`, using `null` for unavailable optional fields.

## Moat dispatch next

`mdid-cli moat dispatch-next --history-path PATH [--agent-id AGENT_ID] [--format text|json]` is a bounded one-task dispatch surface for external controllers. It selects one ready task, claims it unless `--dry-run` is supplied, and never launches agents, schedules work, crawls data, opens PRs, creates cron jobs, or writes artifact files. Text is the default. `--format json` emits a deterministic `moat_dispatch_next` envelope with task fields (`round_id`, `node_id`, `role`, `kind`, `title`, `dependencies`, `spec_ref`), request/persisted ownership (`agent_id`, `assigned_agent_id`), `dry_run`, `claimed`, and `complete_command`; claimed envelopes also include `previous_state`, `new_state`, and numeric `lease_seconds`.

## Moat work packets

`mdid-cli moat work-packet --history-path PATH --node-id NODE_ID [--round-id ROUND_ID] [--format text|json]` exports a deterministic read-only work packet for an external Planner/Coder/Reviewer controller. It includes task metadata, dependency IDs, completed upstream artifact handoffs, acceptance criteria, and the recommended `complete-task` command. It never launches agents, mutates history, schedules work, crawls data, opens PRs, creates cron jobs, or writes artifact files.

## Moat artifacts

`mdid-cli moat artifacts --history-path PATH [--round-id ROUND_ID] [--node-id NODE_ID] [--contains TEXT] [--artifact-ref TEXT] [--artifact-summary TEXT] [--limit N] [--format text|json]` is a read-only completed-artifact handoff inspection surface. Text output remains the default. `--format json` emits a deterministic `moat_artifacts` envelope with `round_id`, `history_path`, `artifact_entries`, and artifact rows containing node metadata plus `artifact_ref` and `artifact_summary`. It never launches agents, mutates history, schedules work, crawls data, opens PRs, creates cron jobs, or writes artifact files.
