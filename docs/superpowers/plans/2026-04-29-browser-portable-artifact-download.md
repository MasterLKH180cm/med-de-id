# Browser Portable Artifact Download Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let the browser vault export flow turn a successful localhost `/vault/export` response into a downloadable encrypted portable artifact JSON file without showing decoded PHI or turning med-de-id into an agent/controller platform.

**Architecture:** Keep the existing browser flow and runtime contract intact. Add a small browser-side formatter that accepts only object-shaped encrypted portable artifacts, pretty-prints the artifact JSON for the existing download button, and keeps summary/review copy PHI-safe and explicit about bounded localhost export semantics.

**Tech Stack:** Rust workspace, `mdid-browser`, Leptos browser UI helpers, serde_json, cargo tests, clippy.

---

## File Structure

- Modify: `crates/mdid-browser/src/app.rs`
  - Extend `parse_runtime_success(InputMode::VaultExport, ...)` behavior so a valid object-valued `artifact` is rendered as pretty JSON in `RuntimeRenderResult.rewritten_output` for download/copy.
  - Keep malformed export responses fail-closed.
  - Keep decoded/original value disclosure out of summary/review copy.
  - Add tests in the existing `#[cfg(test)] mod tests`.
- Modify: `README.md`
  - Truth-sync the browser/web and overall completion snapshot after the landed browser export-download improvement.
  - Preserve explicit missing items and no agent/controller/moat roadmap semantics.

### Task 1: Browser portable artifact download rendering

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`
- Test: `crates/mdid-browser/src/app.rs` existing unit tests

- [ ] **Step 1: Write the failing test**

Add this test inside the existing `mod tests` in `crates/mdid-browser/src/app.rs`, near the other portable artifact runtime tests:

```rust
#[test]
fn vault_export_runtime_success_renders_downloadable_encrypted_artifact_json() {
    let export = json!({
        "artifact": {
            "version": 1,
            "records": [
                {"record_id": "11111111-1111-1111-1111-111111111111", "token": "MDID-1"}
            ],
            "ciphertext": "encrypted-local-artifact"
        }
    });

    let rendered = parse_runtime_success(InputMode::VaultExport, &export.to_string())
        .expect("valid vault export success renders artifact JSON");

    assert!(rendered.summary.contains("Portable artifact created"));
    assert!(rendered.review_queue.contains("encrypted portable artifact"));
    assert!(rendered.rewritten_output.contains("\"version\": 1"));
    assert!(rendered.rewritten_output.contains("encrypted-local-artifact"));
    assert!(rendered.rewritten_output.contains("11111111-1111-1111-1111-111111111111"));
    assert!(!rendered.summary.contains("encrypted-local-artifact"));
    assert!(!rendered.review_queue.contains("MDID-1"));
}
```

- [ ] **Step 2: Run the focused test to verify RED**

Run:

```bash
cargo test -p mdid-browser vault_export_runtime_success_renders_downloadable_encrypted_artifact_json -- --nocapture
```

Expected: FAIL because `parse_runtime_success` currently returns a notice instead of artifact JSON in `rewritten_output`.

- [ ] **Step 3: Implement minimal rendering behavior**

In `parse_runtime_success` for `InputMode::VaultExport`, replace the current hard-coded hidden notice with object validation plus pretty JSON rendering:

```rust
let artifact = value
    .get("artifact")
    .filter(|artifact| artifact.is_object())
    .ok_or("Vault export response missing artifact object.".to_string())?;
let artifact_json = serde_json::to_string_pretty(artifact)
    .map_err(|error| format!("Failed to render portable artifact JSON: {error}"))?;
Ok(RuntimeRenderResult {
    rewritten_output: artifact_json,
    summary: "Portable artifact created. Use the download button to save the encrypted portable artifact JSON locally.".to_string(),
    review_queue: "Encrypted portable artifact is available for local download/copy. Decoded PHI is not rendered by this browser export view.".to_string(),
})
```

Keep the existing malformed-contract rejection path and do not add any agent/controller/planner/moat terminology.

- [ ] **Step 4: Run the focused browser test**

Run:

```bash
cargo test -p mdid-browser vault_export_runtime_success_renders_downloadable_encrypted_artifact_json -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Run related browser tests**

Run:

```bash
cargo test -p mdid-browser portable_artifact -- --nocapture
cargo test -p mdid-browser vault_export -- --nocapture
```

Expected: PASS. If the older test `portable_artifact_runtime_success_hides_artifact_values_and_raw_audit_detail` conflicts with the new bounded encrypted-artifact-download behavior, update only its vault-export assertions so it verifies decoded/original PHI stays out of summary/review copy while encrypted artifact JSON is intentionally downloadable.

- [ ] **Step 6: Commit**

Run:

```bash
git add crates/mdid-browser/src/app.rs
git commit -m "feat(browser): render portable artifact export downloads"
```

### Task 2: README completion truth-sync

**Files:**
- Modify: `README.md`

- [x] **Step 1: Update completion snapshot text**

In `README.md`, update the current repository status snapshot to mention the new browser encrypted portable artifact JSON download/copy behavior. Use these completion numbers unless tests or review reveal a reason to be more conservative:

```markdown
| CLI | 84% | ... |
| Browser/web | 61% | ... now renders successful vault export responses as downloadable encrypted portable artifact JSON while still hiding decoded PHI from summaries/review copy ... |
| Desktop app | 50% | ... |
| Overall | 82% | ... browser ... bounded vault export/download ... |
```

Keep missing items honest: still missing generalized portable transfer workflow UX, full desktop vault/decode/audit execution UX, OCR/visual redaction, PDF rewrite/export, packaging/hardening, and deeper policy/detection.

- [x] **Step 2: Run verification for docs plus touched crate**

Run:

```bash
cargo test -p mdid-browser vault_export -- --nocapture
cargo test -p mdid-browser portable_artifact -- --nocapture
cargo clippy -p mdid-browser --all-targets -- -D warnings
```

Expected: all PASS.

- [x] **Step 3: Commit README truth-sync**

Run:

```bash
git add README.md docs/superpowers/plans/2026-04-29-browser-portable-artifact-download.md
git commit -m "docs: truth-sync browser portable artifact download status"
```

## Self-Review

- Spec coverage: Task 1 implements downloadable encrypted portable artifact JSON for browser vault export. Task 2 updates README completion and missing-items truthfully.
- Placeholder scan: No TBD/TODO/fill-in placeholders are present.
- Type consistency: `InputMode::VaultExport`, `parse_runtime_success`, and `RuntimeRenderResult.rewritten_output/summary/review_queue` match existing code names.
