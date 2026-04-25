# Moat Foundation Compile-Green Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restore compile-green verification for the current moat-loop foundation branch by fixing the `ContinueDecision` move bug exposed by `cargo test -p mdid-application --test moat_rounds`.

**Architecture:** The current controller-visible branch already contains moat domain/application/runtime/CLI foundation work plus newer moat-memory docs/domain commits. The narrow next slice is a bugfix only: make `ContinueDecision` cheap-copy semantics match the way other small workflow enums are modeled so `evaluate_moat_round` and downstream runtime/CLI builds compile cleanly again without changing wire values.

**Tech Stack:** Rust workspace, Cargo, mdid-domain, mdid-application, mdid-runtime, mdid-cli.

---

## Scope note

This is a blocker-fix slice discovered during controller-side integration verification on `feature/moat-loop-autonomy`. Do not widen scope into moat-memory/task-graph implementation; only restore compile-green behavior for the already-landed foundation work.

## File structure

**Modify:**
- `crates/mdid-domain/src/lib.rs` — make `ContinueDecision` copyable
- optionally `crates/mdid-domain/tests/moat_workflow_models.rs` — only if a focused assertion is needed to lock the enum contract

---

### Task 1: Fix `ContinueDecision` move semantics and re-verify the moat foundation slice

**Files:**
- Modify: `crates/mdid-domain/src/lib.rs`
- Test: `crates/mdid-application/tests/moat_rounds.rs`

- [ ] **Step 1: Reproduce the failing compile/test state**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-application --test moat_rounds
```

Expected: FAIL with `E0382 borrow of moved value: continue_decision` in `crates/mdid-application/src/lib.rs`.

- [ ] **Step 2: Apply the minimal fix in the owning domain type**

Update the enum derive in `crates/mdid-domain/src/lib.rs` from:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
```

to:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
```

Keep the enum variants unchanged:

```rust
pub enum ContinueDecision {
    Continue,
    Stop,
    Pivot,
}
```

- [ ] **Step 3: Re-run focused verification**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-application --test moat_rounds
cargo test -p mdid-runtime --test moat_runtime
cargo test -p mdid-cli --test moat_cli
```

Expected: PASS.

- [ ] **Step 4: Re-run broader workspace verification**

Run:

```bash
source "$HOME/.cargo/env"
cargo test --workspace
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-domain/src/lib.rs Cargo.lock
git commit -m "fix: restore moat foundation compile-green verification"
```

## Self-review checklist

- fix stays in the owning enum instead of cloning values ad hoc downstream
- serde wire values for `ContinueDecision` remain unchanged
- application/runtime/cli verification is green again
- no moat-memory/task-graph scope creep slips into this blocker fix
