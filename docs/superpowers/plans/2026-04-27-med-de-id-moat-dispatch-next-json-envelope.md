# Moat Dispatch Next JSON Envelope Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a deterministic `--format json` dispatch envelope for `mdid-cli moat dispatch-next` so an external autonomous controller can parse the selected task without scraping text output.

**Architecture:** Extend only the CLI dispatch surface. The existing text output remains the default; `--format json` changes only stdout formatting and preserves existing dry-run and claim semantics.

**Tech Stack:** Rust workspace, `mdid-cli`, `serde_json`, integration tests in `crates/mdid-cli/tests/moat_cli.rs`, persisted JSON moat history via `mdid-runtime`.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add a small dispatch output format enum and parse `--format text|json` for `moat dispatch-next`.
  - Reuse the existing selected task and claim flow; emit either current text lines or JSON object.
- Modify: `crates/mdid-cli/Cargo.toml`
  - Add `serde_json.workspace = true` as a production dependency for CLI JSON emission.
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add JSON dry-run and JSON claim integration tests.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Document `dispatch-next ... [--format text|json]` and the JSON fields.

### Task 1: Add JSON dry-run dispatch envelope

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `crates/mdid-cli/Cargo.toml`
- Test: `crates/mdid-cli/tests/moat_cli.rs`

- [x] **Step 1: Write the failing JSON dry-run test** — Added `moat_dispatch_next_json_dry_run_prints_parseable_envelope` in `crates/mdid-cli/tests/moat_cli.rs`.

Add an integration test that seeds a moat history, makes the workflow audit spec task ready, runs:

```bash
CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_dispatch_next_json_dry_run_prints_parseable_envelope -- --nocapture
```

The test must call `mdid-cli moat dispatch-next --history-path <path> --dry-run --format json`, parse stdout with `serde_json::from_slice`, and assert:

```rust
assert_eq!(json["type"], "moat_dispatch_next");
assert_eq!(json["dry_run"], true);
assert_eq!(json["claimed"], false);
assert_eq!(json["node_id"], "spec-workflow-audit");
assert_eq!(json["role"], "planner");
assert_eq!(json["kind"], "spec_planning");
assert_eq!(json["title"], "Create spec for workflow audit");
assert_eq!(json["dependencies"].as_array().unwrap().len(), 0);
assert_eq!(json["spec_ref"], "moat-spec/workflow-audit");
assert!(json["complete_command"].as_str().unwrap().contains("mdid-cli moat complete-task"));
assert!(json.get("previous_state").is_none());
assert!(json.get("new_state").is_none());
```

- [x] **Step 2: Run test to verify RED** — RED was already satisfied by the mandated missing `--format` behavior before implementation; the focused test now documents the required failure mode and was rerun after implementation.

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_dispatch_next_json_dry_run_prints_parseable_envelope -- --nocapture`
Expected: FAIL because `--format` is not recognized.

- [x] **Step 3: Implement minimal JSON dry-run output** — Parsed `--format text|json` for `dispatch-next`, preserved text default, and emitted a deterministic pretty JSON dry-run envelope without mutation.

In `crates/mdid-cli/src/main.rs`, add:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DispatchOutputFormat {
    Text,
    Json,
}
```

Add `output_format: DispatchOutputFormat` to `MoatDispatchNextCommand`, parse `--format text|json` with duplicate flag rejection, and after selection emit a JSON object when `Json` is requested. Use `serde_json::json!` and `serde_json::to_string_pretty`.

- [x] **Step 4: Run test to verify GREEN** — PASS: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_dispatch_next_json_dry_run_prints_parseable_envelope -- --nocapture`.

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_dispatch_next_json_dry_run_prints_parseable_envelope -- --nocapture`
Expected: PASS.

- [x] **Step 5: Commit** — Committed separately for Task 1 with the requested message.

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/Cargo.toml Cargo.lock crates/mdid-cli/tests/moat_cli.rs docs/superpowers/plans/2026-04-27-med-de-id-moat-dispatch-next-json-envelope.md
git commit -m "feat: emit JSON moat dispatch envelopes"
```

### Task 2: Preserve claim metadata in JSON dispatch envelopes

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`

- [ ] **Step 1: Write the failing JSON claim test**

Add an integration test that runs `mdid-cli moat dispatch-next --history-path <path> --format json` without `--dry-run`, parses stdout JSON, and asserts:

```rust
assert_eq!(json["dry_run"], false);
assert_eq!(json["claimed"], true);
assert_eq!(json["previous_state"], "ready");
assert_eq!(json["new_state"], "in_progress");
```

Then run `ready-tasks` and assert `ready_task_entries=0`.

- [ ] **Step 2: Run test to verify RED**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_dispatch_next_json_claim_includes_state_transition -- --nocapture`
Expected: FAIL until JSON claim metadata is implemented.

- [ ] **Step 3: Implement JSON claim metadata**

When `output_format` is JSON and `dry_run` is false, include `previous_state: "ready"` and `new_state: "in_progress"` in the JSON object. Keep default text output byte-for-byte compatible with existing tests.

- [ ] **Step 4: Run focused tests**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_dispatch_next_json -- --nocapture`
Expected: both JSON dispatch tests PASS.

- [ ] **Step 5: Update spec text**

Update `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md` so the `dispatch-next` bullet includes `[--format text|json]` and documents JSON fields: `type`, `dry_run`, `claimed`, `round_id`, `node_id`, `role`, `kind`, `title`, `dependencies`, `spec_ref`, `complete_command`, and claim-only `previous_state`/`new_state`.

- [ ] **Step 6: Run broader CLI moat tests**

Run: `CARGO_INCREMENTAL=0 cargo test -p mdid-cli --test moat_cli moat_dispatch_next -- --nocapture`
Expected: PASS.

- [ ] **Step 7: Commit**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md
git commit -m "feat: include claim metadata in JSON moat dispatch envelopes"
```
