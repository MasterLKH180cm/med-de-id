# CLI Conservative Media Review Command Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded CLI command that produces PHI-safe review JSON for the already-landed conservative media metadata service.

**Architecture:** Extend only `mdid-cli` to parse explicit local metadata JSON and call `ConservativeMediaDeidentificationService::deidentify_metadata`. The command writes a PHI-safe report with summary counts and review-queue metadata only; it does not rewrite media bytes, perform OCR/visual redaction, add browser/desktop flows, or introduce agent/controller semantics.

**Tech Stack:** Rust workspace, `mdid-cli`, `mdid-application`, `mdid-adapters`, `mdid-domain`, integration tests with `cargo test -p mdid-cli`.

---

## File Structure

- Modify: `crates/mdid-cli/src/main.rs`
  - Add `review-media` command args, bounded format parsing, metadata JSON parsing, service invocation, and PHI-safe report writer.
- Modify: `crates/mdid-cli/tests/cli_smoke.rs`
  - Add CLI smoke tests for review output and validation.
- Modify: `README.md`
  - Truth-sync CLI/overall completion and missing items after tests pass.

### Task 1: Bounded CLI conservative media review command

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`
- Test: `crates/mdid-cli/tests/cli_smoke.rs`

- [ ] **Step 1: Write failing tests**

Add tests that run `mdid-cli review-media --artifact-label scan-1.png --format image --metadata-json '[{"key":"DeviceSerialNumber","value":"ABC123"}]' --requires-visual-review false --unsupported-payload false --report-path <temp>/media-review.json`, assert success, assert report contains `metadata_only_items: 1`, `review_queue_len: 1`, `rewritten_media_bytes: null`, and assert it does not contain `ABC123`. Add a validation test for blank artifact label expecting non-zero exit.

- [ ] **Step 2: Run tests to verify RED**

Run: `cargo test -p mdid-cli review_media -- --nocapture`
Expected: FAIL because the `review-media` command is not implemented.

- [ ] **Step 3: Write minimal implementation**

Implement:
- `CliCommand::ReviewMedia(ReviewMediaArgs)`
- `ReviewMediaArgs { artifact_label, format, metadata_json, requires_visual_review, unsupported_payload, report_path }`
- parser for flags: `--artifact-label`, `--format` (`image`, `video`, `fcs`), `--metadata-json`, `--requires-visual-review`, `--unsupported-payload`, `--report-path`
- metadata parser into `ConservativeMediaMetadataEntry`
- `run_review_media` calling `ConservativeMediaDeidentificationService::default().deidentify_metadata(ConservativeMediaInput { ... })`
- report JSON containing `summary`, `review_queue_len`, `rewritten_media_bytes: null`, and review queue entries with non-identifying `candidate_index`, `format`, `phi_type`, `confidence`, `status` only; omit artifact labels, metadata keys, metadata values, and artifact-label-derived field paths.

- [ ] **Step 4: Run targeted tests to verify GREEN**

Run: `cargo test -p mdid-cli review_media -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Run broader CLI tests**

Run: `cargo test -p mdid-cli`
Expected: PASS.

- [ ] **Step 6: Commit**

Run:

```bash
git add crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs
git commit -m "feat(cli): add conservative media review command"
```

### Task 2: README truth-sync

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update completion table and missing items**

Update CLI status to include bounded `review-media`; update Overall status to include the landed CLI conservative media review command; reduce remaining CLI gaps by removing conservative-media commands from the missing list while still stating OCR, visual redaction, media rewrite/export, and packaging remain missing.

- [ ] **Step 2: Run verification**

Run: `cargo test -p mdid-cli`
Expected: PASS.

- [ ] **Step 3: Commit**

Run:

```bash
git add README.md docs/superpowers/plans/2026-04-29-cli-conservative-media-review-command.md
git commit -m "docs: truth-sync cli conservative media review completion"
```

## Self-Review

- Spec coverage: The plan implements one bounded CLI conservative-media review/report command backed by existing service behavior.
- Placeholder scan: No TBD/TODO/fill-later placeholders are present.
- Type consistency: `ReviewMediaArgs`, `review-media`, and report fields are consistent across tasks.
