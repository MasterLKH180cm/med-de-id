# Moat-Loop Convergence Batch 2 Implementation Plan

> **Execution note:** Implement this plan task-by-task with small verified commits. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Converge the bounded CLI-only `moat controller-step` handoff surface into the main `med-de-id` line without reintroducing deleted moat runtime/domain files.

**Architecture:** Stay on `feature/moat-loop-convergence`. Do not merge `feature/moat-loop-autonomy` directly. Port only the CLI-only local read/claim handoff command pieces needed for `controller-step`, keeping the implementation bounded to `mdid-cli` plus tests/docs. If a shared runtime/domain dependency is required, stop and re-scope rather than pulling those deleted files back in.

**Tech Stack:** Rust workspace, mdid-cli, existing local history-file JSON coordination model, cargo test, gitflow convergence branch.

---

### Task 1: Port bounded `moat controller-step` CLI handoff command

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify only if strictly needed: `crates/mdid-cli/Cargo.toml`, `Cargo.lock`

**Task spec:**
- Port `moat controller-step` into `feature/moat-loop-convergence`.
- Keep scope bounded:
  - local history-file coordination only
  - text/json output
  - may claim one ready task in local history if that is already part of the CLI-only contract
  - no agent launch
  - no daemon/background work
  - no PR/cron creation
  - no code/artifact generation
- Do not reintroduce deleted moat runtime/domain source files.
- Follow TDD:
  - add/port focused controller-step tests first
  - verify RED
  - implement minimal command support
  - rerun targeted tests to GREEN
  - rerun broader mdid-cli tests
- Commit target: `feat(cli): add moat controller-step handoff`

### Task 2: Review and harden controller-step convergence batch

**Files:**
- Review Task 1 landed files only

**Task spec:**
- Run standard SDD review loop:
  - spec compliance
  - code quality
- Fix any misleading help/output/error semantics.
- Verify the command remains bounded and local-only.

### Task 3: Merge safe controller-step batch into develop if approved

**Files:**
- No product file change required unless review finds issues

**Task spec:**
- If Task 1+2 pass and controller truth-sync confirms no unrelated dirty files, merge `feature/moat-loop-convergence` back into `develop` and push.
- If review finds a blocker, fix it first; do not merge a partial or misleading CLI surface.
