# Browser Portable Artifact Flows Implementation Plan

> **For implementation workers:** REQUIRED SUB-SKILL: Use subagent-driven-development (recommended) or executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add bounded browser vault export plus portable artifact inspect/import request modes using existing localhost runtime routes, with PHI-safe response rendering and README truth-sync.

**Architecture:** Extend `mdid-browser` input modes and helper functions only; the browser remains a thin localhost client and does not add vault browsing, decoded-value display, transfer orchestration, auth/session, or media/PDF rewrite behavior. Reuse the runtime contracts `/vault/export`, `/portable-artifacts/inspect`, and `/portable-artifacts/import`; render only safe counts/disclosures for portable artifacts.

**Tech Stack:** Rust workspace, Leptos browser crate, serde_json helpers, existing `mdid-runtime` JSON contracts, cargo tests/clippy.

---

## File Structure

- Modify `crates/mdid-browser/src/app.rs`: add `InputMode::{VaultExport, PortableArtifactInspect, PortableArtifactImport}`, payload builders, safe runtime success rendering, UI form controls, and regression tests.
- Modify `README.md`: truth-sync completion snapshot for CLI, Browser/web, Desktop app, Overall, and missing items based on landed features and tests.
- Create/modify this plan file only for implementation evidence.

### Task 1: Browser portable request modes and PHI-safe rendering

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`
- Test: inline `#[cfg(test)] mod tests` in `crates/mdid-browser/src/app.rs`

- [x] **Step 1: Write failing tests for mode routing, payload builders, validation, and safe rendering**

Add tests like these to the existing browser tests module, adapting only helper names already present in the file:

```rust
#[test]
fn vault_export_mode_uses_existing_runtime_endpoint() {
    assert_eq!(InputMode::from_select_value("vault-export"), InputMode::VaultExport);
    assert_eq!(InputMode::VaultExport.select_value(), "vault-export");
    assert_eq!(InputMode::VaultExport.endpoint(), "/vault/export");
    assert!(!InputMode::VaultExport.requires_field_policy());
    assert!(!InputMode::VaultExport.requires_source_name());
}

#[test]
fn vault_export_payload_maps_form_to_runtime_contract() {
    let payload = build_vault_export_request_payload(
        " /tmp/vault.json ",
        " passphrase ",
        r#"["11111111-1111-1111-1111-111111111111"]"#,
        " portable secret ",
        " export for local review ",
    )
    .expect("payload");

    assert_eq!(payload["vault_path"], "/tmp/vault.json");
    assert_eq!(payload["vault_passphrase"], "passphrase");
    assert_eq!(payload["record_ids"][0], "11111111-1111-1111-1111-111111111111");
    assert_eq!(payload["export_passphrase"], "portable secret");
    assert_eq!(payload["context"], "export for local review");
    assert_eq!(payload["requested_by"], "browser");
}

#[test]
fn portable_artifact_payloads_reject_blank_required_fields_and_bad_uuid() {
    assert!(build_vault_export_request_payload("", "pw", "[]", "portable", "context").is_err());
    assert!(build_vault_export_request_payload("vault", "pw", r#"["not-a-uuid"]"#, "portable", "context").is_err());
    assert!(build_portable_artifact_inspect_request_payload("{}", "").is_err());
    assert!(build_portable_artifact_import_request_payload("vault", "pw", "{}", "portable", "").is_err());
}

#[test]
fn portable_artifact_runtime_success_hides_artifact_values_and_raw_audit_detail() {
    let export = serde_json::json!({
        "artifact": {"version": 1, "ciphertext": "patient Jane token", "salt": "secret salt", "nonce": "secret nonce"}
    });
    let rendered_export = render_runtime_success(InputMode::VaultExport, &export).expect("export render");
    assert!(rendered_export.output.contains("Portable artifact created"));
    assert!(!rendered_export.output.contains("patient Jane"));
    assert!(!rendered_export.output.contains("ciphertext"));

    let inspect = serde_json::json!({
        "record_count": 1,
        "records": [{"original_value": "Jane Patient", "token": "TOKEN-1"}]
    });
    let rendered_inspect = render_runtime_success(InputMode::PortableArtifactInspect, &inspect).expect("inspect render");
    assert!(rendered_inspect.summary.contains("1 portable record"));
    assert!(!rendered_inspect.output.contains("Jane Patient"));
    assert!(!rendered_inspect.output.contains("TOKEN-1"));

    let import = serde_json::json!({
        "imported_record_count": 1,
        "duplicate_record_count": 2,
        "audit_event": {"kind": "import", "detail": "imported MRN 123", "actor": "browser"}
    });
    let rendered_import = render_runtime_success(InputMode::PortableArtifactImport, &import).expect("import render");
    assert!(rendered_import.summary.contains("1 imported"));
    assert!(rendered_import.review_queue.contains("2 duplicate"));
    assert!(!rendered_import.output.contains("MRN 123"));
}
```

- [x] **Step 2: Run targeted tests to verify RED**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-browser portable_artifact -- --nocapture
cargo test -p mdid-browser vault_export -- --nocapture
```

Expected: FAIL because the new modes and helper functions do not exist yet.

- [x] **Step 3: Implement minimal browser mode, payload, and rendering support**

In `crates/mdid-browser/src/app.rs`:

- Add input modes with select values `vault-export`, `portable-artifact-inspect`, and `portable-artifact-import`.
- Map endpoints exactly to `/vault/export`, `/portable-artifacts/inspect`, and `/portable-artifacts/import`.
- Add builders:
  - `build_vault_export_request_payload(vault_path, vault_passphrase, record_ids_json, export_passphrase, context)` -> JSON with trimmed `vault_path`, `vault_passphrase`, parsed non-empty UUID `record_ids`, trimmed `export_passphrase`, trimmed `context`, and `requested_by: "browser"`.
  - `build_portable_artifact_inspect_request_payload(artifact_json, portable_passphrase)` -> JSON with parsed `artifact` object and trimmed `portable_passphrase`.
  - `build_portable_artifact_import_request_payload(vault_path, vault_passphrase, artifact_json, portable_passphrase, context)` -> JSON with trimmed vault/passphrases/context, parsed `artifact`, and `requested_by: "browser"`.
- Render success safely:
  - vault export: generic artifact-created notice, no ciphertext/salt/nonce/artifact JSON.
  - portable inspect: record count only, no record previews, tokens, original values, scopes, ids, or artifact JSON.
  - portable import: imported and duplicate counts plus generic audit notice, no audit detail/context/artifact/vault path/passphrases.
- UI disclosure copy must explicitly say these are bounded localhost portable artifact request surfaces, not vault browsing, decoded-value display, generalized transfer workflow, or auth/session.

- [x] **Step 4: Run targeted and crate verification**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-browser portable_artifact -- --nocapture
cargo test -p mdid-browser vault_export -- --nocapture
cargo test -p mdid-browser --lib
cargo clippy -p mdid-browser --all-targets -- -D warnings
git diff --check
```

Expected: all pass.

- [x] **Step 5: Commit Task 1**

```bash
git add crates/mdid-browser/src/app.rs docs/superpowers/plans/2026-04-29-browser-portable-artifact-flows.md
git commit -m "feat(browser): add portable artifact request modes"
```

### Task 2: README completion truth-sync

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/plans/2026-04-29-browser-portable-artifact-flows.md`

- [x] **Step 1: Update README completion snapshot based on landed Task 1**

Change the snapshot narrative to credit browser vault export, portable artifact inspect, and portable artifact import only as bounded localhost request/response surfaces with PHI-safe rendering. Use these grounded percentages after Task 1 verification: CLI 84% unchanged, Browser/web 58%, Desktop app 49% unchanged, Overall 80%. Keep missing items honest and do not claim full vault browsing, raw decoded values, generalized portable transfer workflow, auth/session, packaging, OCR, visual redaction, or media/PDF rewrite/export.

- [x] **Step 2: Mark this plan evidence as completed**

Add a completion evidence section listing the branch, commits, and commands run. Mark checkboxes completed for implemented steps.

- [x] **Step 3: Verify docs and scope drift wording**

Run:

```bash
git diff --check
grep -n "Completion snapshot\|CLI | 84%\|Browser/web | 58%\|Desktop app | 49%\|Overall | 80%" README.md
grep -niE 'agent|controller|orchestration|planner|coder|reviewer|moat|claim|complete_command' README.md docs/superpowers/plans/2026-04-29-browser-portable-artifact-flows.md || true
```

Expected: diff check passes; README contains the updated snapshot; any grep hits are limited to non-product plan process text or explicit negative scope wording, not product roadmap claims.

- [x] **Step 4: Commit Task 2**

```bash
git add README.md docs/superpowers/plans/2026-04-29-browser-portable-artifact-flows.md
git commit -m "docs: truth-sync browser portable artifact completion"
```


---

## Completion Evidence

- Branch: `feature/browser-portable-artifact-flows`
- Task 1 commits:
  - `ecc1e35 feat(browser): add portable artifact request modes`
  - `12874a4 fix(browser): redact portable artifact runtime errors`
- Task 1 verification commands run:
  - `source "$HOME/.cargo/env" && cargo test -p mdid-browser portable_artifact -- --nocapture`
  - `source "$HOME/.cargo/env" && cargo test -p mdid-browser vault_export -- --nocapture`
  - `source "$HOME/.cargo/env" && cargo test -p mdid-browser --lib`
  - `source "$HOME/.cargo/env" && cargo clippy -p mdid-browser --all-targets -- -D warnings`
  - `git diff --check`
- Task 2 verification commands run:
  - `git diff --check`
  - `grep -n "Completion snapshot\|CLI | 84%\|Browser/web | 58%\|Desktop app | 49%\|Overall | 80%" README.md`
  - `grep -niE 'agent|controller|orchestration|planner|coder|reviewer|moat|claim|complete_command' README.md docs/superpowers/plans/2026-04-29-browser-portable-artifact-flows.md || true`
- README truth-sync: updated completion snapshot to CLI 84%, Browser/web 58%, Desktop app 49%, Overall 80%; credited browser vault export, portable artifact inspect, and portable artifact import only as bounded localhost request/response surfaces with PHI-safe rendering; preserved explicit gaps for full vault browsing, raw decoded values, generalized portable transfer workflow, auth/session, packaging, OCR, visual redaction, and media/PDF rewrite/export.
