# Moat Schedule Next Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded CLI scheduler command that starts exactly one next deterministic moat round only when the persisted continuation gate allows it.

**Architecture:** Keep scheduling local-first and explicitly bounded: `mdid-cli moat schedule-next` reads an existing history file, evaluates the same continuation gate as `moat continue`, and appends one new deterministic round only when `can_continue=true`. It is intentionally one-shot and does not create background schedulers, cron jobs, PR automation, live crawling, or unrestricted autonomous loops.

**Tech Stack:** Rust workspace, `mdid-cli`, `mdid-runtime::moat_history::LocalMoatHistoryStore`, Cargo integration tests.

---

## Scope

This plan advances the Autonomous Multi-Agent System / moat-loop worktree by adding the smallest safe scheduler-control slice. It implements one operator-facing command:

```bash
cargo run -p mdid-cli -- moat schedule-next --history-path .mdid/moat-history.json [--improvement-threshold N]
```

The command must:

- require an existing history file; missing paths fail rather than creating a new file
- print the continuation-gate reason
- append exactly one new deterministic bounded round when `can_continue=true`
- not append when `can_continue=false`
- print a stable summary with `scheduled=true|false`, `reason=...`, and when scheduled, `scheduled_round_id=...`

## Files

- Modify: `crates/mdid-cli/src/main.rs` — parse and run `moat schedule-next`.
- Modify: `crates/mdid-cli/tests/moat_cli.rs` — integration tests for append and no-op scheduler behavior.
- Modify: `README.md` — document the command and one-shot constraints.
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md` — truth-sync shipped/planned status.

---

### Task 1: CLI one-shot scheduler command

**Files:**
- Modify: `crates/mdid-cli/tests/moat_cli.rs`
- Modify: `crates/mdid-cli/src/main.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`

- [ ] **Step 1: Write failing CLI integration tests**

Append tests to `crates/mdid-cli/tests/moat_cli.rs` that:

```rust
#[test]
fn moat_schedule_next_appends_one_round_when_gate_allows_continuation() {
    let dir = tempfile::tempdir().expect("tempdir");
    let history_path = dir.path().join("moat-history.json");
    let history_path_arg = history_path.to_string_lossy().to_string();

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round", "--history-path", &history_path_arg])
        .output()
        .expect("failed to seed moat history");
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "schedule-next", "--history-path", &history_path_arg])
        .output()
        .expect("failed to schedule next moat round");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("moat schedule next\n"));
    assert!(stdout.contains("scheduled=true\n"));
    assert!(stdout.contains("reason=latest round cleared continuation gate\n"));
    assert!(stdout.contains("scheduled_round_id="));

    let history = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "history", "--history-path", &history_path_arg])
        .output()
        .expect("failed to inspect moat history");
    let history_stdout = String::from_utf8_lossy(&history.stdout);
    assert!(history.status.success(), "{}", String::from_utf8_lossy(&history.stderr));
    assert!(history_stdout.contains("entries=2\n"));
}

#[test]
fn moat_schedule_next_does_not_append_when_gate_blocks_continuation() {
    let dir = tempfile::tempdir().expect("tempdir");
    let history_path = dir.path().join("moat-history.json");
    let history_path_arg = history_path.to_string_lossy().to_string();

    let seed = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args([
            "moat",
            "round",
            "--history-path",
            &history_path_arg,
            "--tests-passed",
            "false",
        ])
        .output()
        .expect("failed to seed stopped moat history");
    assert!(seed.status.success(), "{}", String::from_utf8_lossy(&seed.stderr));

    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "schedule-next", "--history-path", &history_path_arg])
        .output()
        .expect("failed to inspect schedule next gate");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("moat schedule next\n"));
    assert!(stdout.contains("scheduled=false\n"));
    assert!(stdout.contains("reason=latest round tests failed\n"));
    assert!(stdout.contains("scheduled_round_id=<none>\n"));

    let history = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "history", "--history-path", &history_path_arg])
        .output()
        .expect("failed to inspect moat history");
    let history_stdout = String::from_utf8_lossy(&history.stdout);
    assert!(history.status.success(), "{}", String::from_utf8_lossy(&history.stderr));
    assert!(history_stdout.contains("entries=1\n"));
}
```

Also update the local `USAGE` constant in the same test file so expected usage contains:

```text
moat schedule-next --history-path PATH [--improvement-threshold N]
```

- [ ] **Step 2: Run the failing CLI tests**

Run:

```bash
cargo test -p mdid-cli --test moat_cli moat_schedule_next -- --nocapture
```

Expected: FAIL because `moat schedule-next` is an unknown command.

- [ ] **Step 3: Implement minimal parser and runner**

Modify `crates/mdid-cli/src/main.rs`:

- add `MoatScheduleNext { history_path: String, improvement_threshold: i16 }` to `CliCommand`
- parse `moat schedule-next` with the same flags and defaults as `moat continue`
- run it by opening existing history, checking `continuation_gate`, appending `sample_round_report(&MoatRoundOverrides::default())` only when `can_continue=true`, and printing:

```text
moat schedule next
scheduled=true|false
reason=<gate reason>
scheduled_round_id=<uuid-or-none>
history_path=<path>
```

Reuse existing helper functions where possible; do not add background jobs or live crawling.

- [ ] **Step 4: Run targeted tests until green**

Run:

```bash
cargo test -p mdid-cli --test moat_cli moat_schedule_next -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Run broader CLI moat tests**

Run:

```bash
cargo test -p mdid-cli --test moat_cli
cargo test -p mdid-cli
```

Expected: PASS.

- [ ] **Step 6: Update docs**

In `README.md`, after the `moat continue` section and before `moat export-specs`, add:

```markdown
Schedule exactly one next bounded round when the continuation gate allows it with:

```bash
cargo run -p mdid-cli -- moat schedule-next --history-path .mdid/moat-history.json
```

`moat schedule-next` is a one-shot local scheduler control: it requires an existing history file, checks the same continuation gate as `moat continue`, appends one deterministic bounded round only when `can_continue=true`, and otherwise leaves history unchanged. It does not create a cron job, background daemon, live crawler, or unrestricted autonomous loop.
```

In `docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md`:

- add a shipped-foundation bullet for `mdid-cli moat schedule-next --history-path PATH [--improvement-threshold N]`
- update the shipped-slice paragraph to say scheduler control is now present only as a one-shot bounded local command, while no background scheduler/daemon exists
- remove `scheduler control` from the still-planned list only if the wording is replaced with `background scheduler/daemon control`

- [ ] **Step 7: Verify docs mention the command**

Run:

```bash
python - <<'PY'
from pathlib import Path
for path in [Path('README.md'), Path('docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md')]:
    text = path.read_text()
    assert 'moat schedule-next --history-path' in text, path
print('docs mention schedule-next')
PY
```

Expected: `docs mention schedule-next`.

- [ ] **Step 8: Commit**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/moat_cli.rs README.md docs/superpowers/specs/2026-04-25-med-de-id-moat-loop-design.md docs/superpowers/plans/2026-04-26-med-de-id-moat-schedule-next.md
git commit -m "feat: add bounded moat schedule-next command"
```
