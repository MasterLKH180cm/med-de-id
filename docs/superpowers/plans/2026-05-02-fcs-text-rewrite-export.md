# FCS TEXT Rewrite Export Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded real FCS TEXT-segment byte rewrite/export path for field-level PHI metadata keys.

**Architecture:** Implement a conservative FCS 3.x byte parser/re-writer in `mdid-adapters` that validates the HEADER/TEXT offsets, parses delimited TEXT key/value pairs, rewrites only explicitly requested metadata keys, and emits rewritten FCS bytes plus PHI-safe aggregate verification. Expose that adapter through `mdid-cli redact-fcs-text` with JSON key/value replacements and a PHI-safe summary file.

**Tech Stack:** Rust workspace, `mdid-adapters`, `mdid-cli`, serde_json, Cargo integration tests.

---

## File Structure

- Modify `crates/mdid-adapters/src/conservative_media.rs`: add FCS TEXT rewrite data types, parser, bounded byte rewrite, validation, and PHI-safe `Debug` behavior.
- Modify `crates/mdid-adapters/tests/conservative_media_adapter.rs`: add adapter RED/GREEN tests for rewriting, no-op preservation, and fail-closed malformed offsets.
- Modify `crates/mdid-cli/src/main.rs`: add `redact-fcs-text` command parsing, adapter invocation, output writing, and summary writing.
- Modify `crates/mdid-cli/tests/cli_smoke.rs`: add CLI smoke tests proving output bytes are rewritten and reports omit raw PHI.
- Modify `README.md`: truth-sync only the landed bounded FCS byte rewrite/export behavior; do not raise completion beyond 99% unless repository-visible behavior justifies it.

### Task 1: Adapter FCS TEXT parser and byte rewrite

**Files:**
- Modify: `crates/mdid-adapters/src/conservative_media.rs`
- Test: `crates/mdid-adapters/tests/conservative_media_adapter.rs`

- [ ] **Step 1: Write failing tests**

Add tests named:

```rust
#[test]
fn fcs_text_rewrite_replaces_only_requested_text_values_and_preserves_data_bytes() { /* construct HEADER+TEXT+DATA bytes; rewrite $SMNO and $OP; assert output contains tokens, omits originals, and data bytes unchanged */ }

#[test]
fn fcs_text_rewrite_fails_closed_on_invalid_text_offsets() { /* HEADER references text offsets outside bytes; assert InvalidTextSegment */ }
```

- [ ] **Step 2: Run RED**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-adapters --test conservative_media_adapter fcs_text_rewrite -- --nocapture`
Expected: FAIL because `FcsTextRewriteRequest` / `rewrite_fcs_text_segment` do not exist.

- [ ] **Step 3: Implement minimal adapter**

Add public structs `FcsTextRewriteRequest`, `FcsTextRewriteSummary`, `FcsTextRewriteOutput` and error enum variants for unsupported/non-FCS/invalid header/text segment. Implement `ConservativeMediaAdapter::rewrite_fcs_text_segment(bytes, request)` for FCS 3.x HEADER with ASCII byte offsets at fixed ranges 10..18 and 18..26, single-byte TEXT delimiter, key/value pairs, and same-length segment expansion with adjusted end offset when replacements change length. Summary must include only counts, byte lengths, and rewritten keys, never raw values.

- [ ] **Step 4: Run GREEN**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-adapters --test conservative_media_adapter fcs_text_rewrite -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit**

Run: `git add crates/mdid-adapters/src/conservative_media.rs crates/mdid-adapters/tests/conservative_media_adapter.rs && git commit -m "feat(fcs): rewrite bounded TEXT metadata bytes"`

### Task 2: CLI FCS TEXT rewrite/export command

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/cli_smoke.rs`

- [ ] **Step 1: Write failing CLI smoke test**

Add a smoke test that runs:

```bash
mdid-cli redact-fcs-text --fcs-path input.fcs --replacements-json '{"$SMNO":"[FCS_SAMPLE]","$OP":"[FCS_OPERATOR]"}' --output-path redacted.fcs --summary-output summary.json
```

Assert the output FCS bytes contain `[FCS_SAMPLE]` and `[FCS_OPERATOR]`, omit raw `MRN-12345` and `Dr. Alice Example`, preserve non-TEXT DATA bytes, and the summary JSON omits raw PHI.

- [ ] **Step 2: Run RED**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-cli redact_fcs_text --test cli_smoke -- --nocapture`
Expected: FAIL with unknown command.

- [ ] **Step 3: Implement command**

Add command enum/usage parsing for `redact-fcs-text`, parse required paths and JSON object replacements, call `ConservativeMediaAdapter::rewrite_fcs_text_segment`, write output bytes and PHI-safe summary JSON.

- [ ] **Step 4: Run GREEN**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-cli redact_fcs_text --test cli_smoke -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit**

Run: `git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs && git commit -m "feat(cli): export redacted FCS TEXT bytes"`

### Task 3: README truth-sync

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update landed capability rows and evidence paragraph**

Update the FCS support row from `no FCS byte rewrite/export` to bounded TEXT-segment byte rewrite/export for explicit metadata replacement JSON, while preserving that vendor DATA/ANALYSIS semantics are not rewritten.

- [ ] **Step 2: Run README grep check**

Run: `grep -n "FCS" README.md | head -8`
Expected: shows bounded FCS TEXT byte rewrite/export and no overclaim of full FCS vendor-semantic rewriting.

- [ ] **Step 3: Commit**

Run: `git add README.md docs/superpowers/plans/2026-05-02-fcs-text-rewrite-export.md && git commit -m "docs(readme): truth-sync fcs text rewrite export"`

### Task 4: Controller verification

- [ ] Run `source "$HOME/.cargo/env" && cargo test -p mdid-adapters --test conservative_media_adapter -- --nocapture`.
- [ ] Run `source "$HOME/.cargo/env" && cargo test -p mdid-cli redact_fcs_text --test cli_smoke -- --nocapture`.
- [ ] Run `source "$HOME/.cargo/env" && cargo test -p mdid-cli review_media_export_report_omits_payload_bytes --test cli_smoke -- --nocapture`.

### Task 5: GitFlow release

- [ ] Push develop.
- [ ] Merge verified develop to main using fast-forward if possible; if blocked, use `/tmp/med-de-id-release-main` no-ff release merge.
- [ ] Re-run targeted FCS/CLI gates on release tree before pushing main.
