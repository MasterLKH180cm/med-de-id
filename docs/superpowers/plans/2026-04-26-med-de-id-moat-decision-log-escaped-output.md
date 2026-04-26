# Moat Decision Log Escaped Output Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Escape pipe-delimited `mdid-cli moat decision-log` output fields so persisted decision summaries and rationales containing `\`, `|`, newline, or carriage return remain parseable, while preserving the command's read-only/latest-round behavior and existing role/text/summary/rationale filters.

**Architecture:** Reuse the existing CLI output escaping semantics already used by moat assignments and task-graph rows. This is a conservative CLI rendering-only slice: parsing, filtering, history loading, latest-round selection, and no-append guarantees stay unchanged. Output remains `decision=<role>|<summary>|<rationale>`, with summary and rationale escaped before printing.

**Tech Stack:** Rust workspace, `mdid-cli` binary integration tests, local JSON moat history store, Cargo test runner, README/spec markdown documentation.

---

## File Structure

- Modify: `crates/mdid-cli/tests/moat_cli.rs`
  - Add RED tests first for decision-log escaped summary/rationale output.
  - Add/keep verification that decision-log inspection does not append to history when escaping is exercised.
  - Add/keep verification that filters still match persisted unescaped values before rendering.
- Modify: `crates/mdid-cli/src/main.rs`
  - Reuse or generalize `escape_assignment_output_field` for decision-log summary/rationale rendering.
  - Apply escaping only at the final `println!("decision=...")` boundary.
  - Do not change `LocalMoatHistoryStore::open_existing`, latest entry selection, or filter predicates.
- Modify: `README.md`
  - Document that decision-log summary/rationale fields are pipe-delimited and escaped using `\\`, `\|`, `\n`, and `\r`.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
  - Document escaped decision-log output fields on the read-only latest-round inspection surface.
- Modify CLI usage only if needed:
  - No usage string change is expected because flags and invocation shape do not change.
- Create: `docs/superpowers/plans/2026-04-26-med-de-id-moat-decision-log-escaped-output.md`
  - This plan.

## Task 1: Prove decision-log output escapes summary/rationale fields before changing production code

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`

- [x] **Step 1: Add a failing integration test for escaped decision-log fields**

Add a test near the existing decision-log tests in `crates/mdid-cli/tests/moat_cli.rs`, for example `cli_moat_decision_log_escapes_pipe_delimited_summary_and_rationale_fields`.

The test must:

1. Create a unique history path with `unique_history_path("decision-log-escaped-fields")`.
2. Seed one persisted round by invoking the test binary with `--history-path history_path_arg`:
   ```rust
   Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
       .args(["moat", "round", "--review-loops", "0", "--history-path", history_path_arg])
       .output()
   ```
3. Read the JSON history file and patch the persisted reviewer decision so the summary and rationale contain all characters requiring escaping:
   - summary includes a literal pipe and newline, such as `review|approved\nsummary`
   - rationale includes a literal pipe, carriage return, and backslash, such as `evaluation|completed\rpath\\tail`
4. Write the patched JSON back to the same history path.
5. Run:
   ```rust
   mdid-cli moat decision-log --history-path <path>
   ```
6. Assert success and exact stdout for the latest round:
   ```rust
   concat!(
       "decision_log_entries=1\n",
       "decision=reviewer|review\\|approved\\nsummary|evaluation\\|completed\\rpath\\\\tail\n",
   )
   ```
7. Clean up the history path.

This test must fail before production code changes because current decision-log rendering prints `decision.summary` and `decision.rationale` directly.

- [x] **Step 2: Add a failing regression test that escaping does not append history**

Add a test such as `decision_log_escaped_output_does_not_append_history` or extend the escaped-field test with explicit history-count assertions.

The test must:

1. Seed exactly one round in a unique history file.
2. Patch persisted decision text to include escapable characters.
3. Run `mdid-cli moat decision-log --history-path <path>` one or more times.
4. Run `mdid-cli moat history --history-path <path>` afterward.
5. Assert history output still contains `entries=1\n`.

This protects the existing read-only/latest-round guarantee while changing rendering.

- [x] **Step 3: Add a failing regression test that filters use unescaped persisted content**

Add a test such as `decision_log_filters_before_escaping_output_fields`.

The test must:

1. Seed and patch the latest decision summary/rationale to include raw pipe/newline/carriage-return/backslash characters.
2. Run `mdid-cli moat decision-log` with existing filters that match the raw persisted text, for example:
   - `--summary-contains "review|approved"`
   - `--rationale-contains "path\\tail"`
   - optionally `--contains "completed\rpath"`
3. Assert each filtered command returns `decision_log_entries=1\n` and the escaped `decision=...` row.
4. Assert a non-matching existing filter still returns exactly `decision_log_entries=0\n`.

Do not change parser tests or usage tests unless the implementation unexpectedly requires usage text changes.

- [x] **Step 4: Run the targeted tests and confirm RED**

Run:

```bash
cargo test -p mdid-cli --test moat_cli decision_log -- --nocapture
```

Expected result before production changes: the new escaped-output assertions fail because decision-log output is not escaped yet. Existing decision-log tests should continue to show the current read-only/filter behavior.

## Task 2: Escape decision-log rendering without changing command semantics

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`

- [x] **Step 1: Reuse the existing escape helper**

Use the existing `escape_assignment_output_field(value: &str) -> String` behavior for decision-log field output. Either:

- call `escape_assignment_output_field` directly for `decision.summary` and `decision.rationale`, or
- rename/generalize the helper to a neutral name such as `escape_pipe_delimited_output_field` and update assignments/task-graph call sites mechanically.

Keep the exact escaping order/semantics:

```rust
value
    .replace('\\', "\\\\")
    .replace('|', "\\|")
    .replace('\n', "\\n")
    .replace('\r', "\\r")
```

- [x] **Step 2: Change only the final decision-log print boundary**

In `run_moat_decision_log`, update the row rendering from direct fields to escaped fields:

```rust
println!(
    "decision={}|{}|{}",
    format_agent_role(decision.author_role),
    escape_pipe_delimited_output_field(&decision.summary),
    escape_pipe_delimited_output_field(&decision.rationale)
);
```

If the helper is not renamed, use `escape_assignment_output_field` for the two decision fields.

Do not alter:

- `LocalMoatHistoryStore::open_existing(&command.history_path)`
- `store.entries().last()` latest-round selection
- role matching
- `--contains` summary-or-rationale matching
- `--summary-contains` matching
- `--rationale-contains` matching
- argument parsing
- history writing/appending behavior

## Task 3: Document escaped decision-log output

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`
- Modify CLI usage only if needed in `crates/mdid-cli/src/main.rs` and corresponding test usage constants

- [x] **Step 1: Update README decision-log documentation**

In the existing `moat decision-log` section, keep the command examples and read-only/latest-round wording. Update the output description to say:

- rows are still printed as `decision=<role>|<summary>|<rationale>`
- `<summary>` and `<rationale>` are escaped for pipe-delimited output
- escaping uses `\\` for backslash, `\|` for pipe, `\n` for newline, and `\r` for carriage return
- filters still match the persisted unescaped summary/rationale text before rendering

- [x] **Step 2: Update the moat-loop design spec**

In `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`, extend the read-only `decision-log` inspection bullet to document escaped output fields. Preserve the existing scope language that this is latest-round, read-only, bounded local inspection.

- [x] **Step 3: Leave CLI usage unchanged unless flags change**

Because this slice changes output encoding, not invocation shape, do not change the `usage()` string unless implementation reveals an existing inaccurate usage line. If usage changes are needed, update any test-local `USAGE` constants in the same TDD cycle.

## Task 4: Verify the conservative slice

**Files:**
- Verify only; modify tests/docs/code only as described above.

- [x] **Step 1: Run targeted decision-log tests**

```bash
cargo test -p mdid-cli --test moat_cli decision_log -- --nocapture
```

Confirm the new escaped-output tests and existing role/contains/summary/rationale/read-only tests pass.

- [x] **Step 2: Run related escaped-output tests**

```bash
cargo test -p mdid-cli --test moat_cli escapes_pipe_delimited_fields -- --nocapture
```

Confirm assignments/task-graph escaping behavior remains intact if the helper was renamed or generalized.

- [x] **Step 3: Run the full CLI integration test target if time permits**

```bash
cargo test -p mdid-cli --test moat_cli
```

If this is too slow in the current environment, record the targeted commands and results in the implementation summary.

- [x] **Step 4: Review the final diff**

```bash
git diff -- crates/mdid-cli/tests/moat_cli.rs crates/mdid-cli/src/main.rs README.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md
```

Confirm:

- tests were added before production changes
- decision-log uses the latest persisted round only
- no new history append path was introduced
- filters still operate before escaping
- only summary/rationale rendering changed
- documentation matches behavior

## Acceptance Criteria

- `mdid-cli moat decision-log --history-path PATH` still reads only the latest persisted round and does not create or append history.
- Existing filters `--role`, `--contains`, `--summary-contains`, and `--rationale-contains` keep their current case-sensitive, conjunctive semantics.
- Decision-log rows remain pipe-delimited as `decision=<role>|<summary>|<rationale>`.
- Decision-log `<summary>` and `<rationale>` output fields escape backslash, pipe, newline, and carriage return exactly like assignments/task-graph fields.
- Tests in `crates/mdid-cli/tests/moat_cli.rs` prove escaped output, read-only/no-append behavior, and filter-before-rendering behavior.
- README and the moat-loop design spec document the escaped output contract.
- No production code is changed until the RED tests are in place.
