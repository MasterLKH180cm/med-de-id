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
