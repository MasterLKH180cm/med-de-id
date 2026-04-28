# Moat-Loop Convergence Implementation Plan

> **Execution note:** Implement this plan task-by-task with small verified commits. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Converge selected safe moat-loop CLI capabilities back into the main `med-de-id` product line without reintroducing the deleted moat runtime/domain surfaces or forcing a high-conflict branch merge.

**Architecture:** Do **not** merge `feature/moat-loop-autonomy` directly into `develop`. Instead, use `feature/moat-loop-convergence` as a dedicated GitFlow convergence branch cut from current `develop`, then selectively port or cherry-pick bounded CLI-only commands whose contracts are local-only, read-only, and independently testable. Each convergence batch must stay within the CLI surface unless and until shared/runtime dependencies are re-approved.

**Tech Stack:** Rust workspace, mdid-cli, existing moat history-file coordination commands, cargo test, git cherry-pick/porting.

---

### Task 1: Converge `moat controller-plan` CLI-only command into mainline

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/tests/cli_smoke.rs` (expected only when CLI usage/help text changes)
- Modify only if required by clean porting: `crates/mdid-cli/Cargo.toml`, `Cargo.lock`

**Task spec:**
- Port the bounded local-only `moat controller-plan` command from `feature/moat-loop-autonomy` into `feature/moat-loop-convergence`.
- Keep the scope CLI-only:
  - local history-file read-only packet export
  - text/json envelope support
  - no agent launch
  - no daemon/background work
  - no PR/cron creation
  - no artifact/code writes
- Do not reintroduce deleted moat runtime/domain files into mainline.
- Run targeted tests first:
  - controller-plan specific tests
  - broader moat CLI tests as needed
- Commit target: `feat(cli): add moat controller-plan command`

### Task 2: Review and close out controller-plan convergence batch

**Files:**
- Review Task 1 landed files only

**Task spec:**
- Run normal SDD review loop:
  - spec compliance
  - quality review
- Verify no deleted moat runtime/domain surfaces were accidentally pulled back in.
- Keep the landed unit narrow and releaseable.

### Task 3: Decide next safe convergence batch based on controller-visible diff radius

**Files:**
- No code change required unless the next batch is started

**Task spec:**
- After Task 1 is landed and reviewed, reassess `feature/moat-loop-autonomy` vs `feature/moat-loop-convergence`.
- Select the next smallest safe CLI-only or documentation-only convergence batch, likely one of:
  - `controller-step` handoff improvements
  - JSON envelope-only additions
  - artifact inspection filters
- Explicitly reject any batch that would force reintroduction of deleted moat runtime/domain surfaces.
