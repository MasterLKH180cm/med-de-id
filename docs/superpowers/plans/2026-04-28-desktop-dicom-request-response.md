# Desktop DICOM Request/Response Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add bounded desktop DICOM request preparation, import, response rendering, and export helper support aligned with the existing local runtime `/dicom/deidentify` route.

**Architecture:** Extend the existing `mdid-desktop` pure helper layer with a fourth `DesktopWorkflowMode` for DICOM, keeping all sensitive payloads redacted in `Debug` and all network behavior bounded to the existing localhost submit mechanism. Reuse the existing response workbench shape and add DICOM-specific request body, success output, banner, and suggested export filename without adding vault browsing, audit, decode, file pickers, OCR, controller, or agent workflow semantics.

**Tech Stack:** Rust workspace, `mdid-desktop`, `serde_json`, existing egui desktop app, Cargo tests/clippy.

---

## File Structure

- Modify: `crates/mdid-desktop/src/lib.rs`
  - Add `DesktopWorkflowMode::DicomBase64`.
  - Map `.dcm`/`.dicom` imports to DICOM base64 payloads and source names.
  - Build runtime-compatible DICOM JSON: `{ "dicom_bytes_base64", "source_name", "private_tag_policy" }`.
  - Render DICOM runtime success envelopes using `rewritten_dicom_bytes_base64`, `summary`, and `review_queue`.
  - Add a safe DICOM export helper filename.
- Modify: `crates/mdid-desktop/src/main.rs`
  - Ensure mode picker, source-name/private-policy controls, status text, runtime submit, and response rendering remain truthful for the new DICOM mode. If UI is already generic through `DesktopWorkflowMode::ALL`, only update copy/control conditions needed for DICOM.
- Modify: `README.md`
  - Truth-sync completion: Desktop app and Overall get modest credit only for bounded DICOM desktop request/response helper support.
  - Keep missing-item list honest: no vault browsing, decode, audit investigation, full DICOM workflow management, auth/session, or controller/agent semantics.

### Task 1: Desktop DICOM runtime helper mode

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`
- Modify: `crates/mdid-desktop/src/main.rs`
- Test: inline `#[cfg(test)]` tests in `crates/mdid-desktop/src/lib.rs`

- [x] **Step 1: Write failing tests for DICOM import, request, response, export, and disclosure behavior**

Add these tests inside `crates/mdid-desktop/src/lib.rs` `mod tests`:

```rust
#[test]
fn desktop_file_import_dicom_bytes_map_to_dicom_base64_payload_with_source_name() {
    let imported = DesktopFileImportPayload::from_bytes("scan.dcm", b"DICM\x00\x01").unwrap();

    assert_eq!(imported.mode, DesktopWorkflowMode::DicomBase64);
    assert_eq!(imported.payload, "RElDTRA=AAE=");
    assert_eq!(imported.source_name.as_deref(), Some("scan.dcm"));

    let imported = DesktopFileImportPayload::from_bytes("scan.dicom", b"DICM").unwrap();
    assert_eq!(imported.mode, DesktopWorkflowMode::DicomBase64);
    assert_eq!(imported.payload, "RElDTQ==");
    assert_eq!(imported.source_name.as_deref(), Some("scan.dicom"));
}

#[test]
fn dicom_base64_builds_runtime_compatible_dicom_request_body() {
    let state = DesktopWorkflowRequestState {
        mode: DesktopWorkflowMode::DicomBase64,
        payload: " RElDTQ== ".to_string(),
        field_policy_json: DEFAULT_POLICY_JSON.to_string(),
        source_name: " scan.dcm ".to_string(),
    };

    let request = state.try_build_request().unwrap();

    assert_eq!(request.route, "/dicom/deidentify");
    assert_eq!(
        request.body,
        json!({"dicom_bytes_base64":"RElDTQ==","source_name":"scan.dcm","private_tag_policy":"review_required"})
    );

    let disclosure = state.mode.disclosure();
    assert!(disclosure.contains("bounded local runtime"));
    assert!(disclosure.contains("tag-level DICOM de-identification"));
    assert!(disclosure.contains("no generalized workflow orchestrator"));
}

#[test]
fn dicom_submit_requires_source_name_before_runtime_request() {
    let state = DesktopWorkflowRequestState {
        mode: DesktopWorkflowMode::DicomBase64,
        payload: "RElDTQ==".to_string(),
        field_policy_json: DEFAULT_POLICY_JSON.to_string(),
        source_name: "  ".to_string(),
    };

    assert!(matches!(
        state.try_build_request(),
        Err(DesktopWorkflowValidationError::BlankSourceName)
    ));
}

#[test]
fn response_state_renders_dicom_runtime_success_envelope() {
    let mut response = DesktopWorkflowResponseState::default();
    response.apply_success_json(
        DesktopWorkflowMode::DicomBase64,
        json!({
            "sanitized_file_name":"deidentified.dcm",
            "rewritten_dicom_bytes_base64":"RElDTQ==",
            "summary":{"rewritten_tags":2,"review_required":1},
            "review_queue":[{"tag":"0010,0010","decision":"review"}]
        }),
    );

    assert_eq!(response.banner, "DICOM base64 runtime response rendered locally.");
    assert_eq!(response.output, "RElDTQ==");
    assert!(response.summary.contains("rewritten_tags"));
    assert!(response.review_queue.contains("0010,0010"));
    assert_eq!(response.exportable_output(), Some("RElDTQ=="));
    assert_eq!(
        response.suggested_export_file_name(DesktopWorkflowMode::DicomBase64),
        Some("desktop-deidentified.dcm.base64.txt")
    );
}
```

- [x] **Step 2: Run tests to verify RED**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-desktop dicom -- --nocapture
```

Expected: FAIL because `DesktopWorkflowMode::DicomBase64` and DICOM handling do not exist yet.

- [x] **Step 3: Implement minimal DICOM mode support**

In `crates/mdid-desktop/src/lib.rs`:

- Add `DicomBase64` to `DesktopWorkflowMode` and `DesktopWorkflowMode::ALL`.
- Add `.dcm` and `.dicom` import mapping to base64 payload plus `source_name`.
- Add labels/hints/disclosure/route for DICOM.
- In `try_build_request`, handle `DicomBase64` like PDF for source-name validation but emit runtime-compatible DICOM request JSON with `private_tag_policy: "review_required"`.
- In `DesktopWorkflowResponseState::apply_success_json`, read `rewritten_dicom_bytes_base64` for DICOM output.
- In `suggested_export_file_name`, return `Some("desktop-deidentified.dcm.base64.txt")` for DICOM when output is exportable.

Expected code shape:

```rust
DesktopWorkflowMode::DicomBase64 => "/dicom/deidentify"
serde_json::json!({
    "dicom_bytes_base64": self.payload.trim(),
    "source_name": self.source_name.trim(),
    "private_tag_policy": "review_required",
})
```

In `crates/mdid-desktop/src/main.rs`, update any match statements over `DesktopWorkflowMode` so DICOM compiles and DICOM uses source-name controls rather than tabular field-policy controls.

- [x] **Step 4: Run targeted tests to verify GREEN**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-desktop dicom -- --nocapture
```

Expected: PASS.

- [x] **Step 5: Run broader desktop verification**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-desktop
cargo clippy -p mdid-desktop --all-targets -- -D warnings
git diff --check
```

Expected: all commands PASS.

- [x] **Step 6: Commit Task 1**

```bash
git add crates/mdid-desktop/src/lib.rs crates/mdid-desktop/src/main.rs
git commit -m "feat(desktop): add bounded DICOM runtime helpers"
```

### Task 2: README completion truth-sync

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/plans/2026-04-28-desktop-dicom-request-response.md`

- [x] **Step 1: Update README completion snapshot**

Change completion rows to:

```markdown
| Desktop app | 30% | Bounded sensitive-workstation foundation prepares CSV, XLSX, PDF review, and DICOM requests, can apply bounded CSV/XLSX/PDF/DICOM file import/export helpers, submit prepared envelopes to a localhost runtime, and render response panes with honest disclosures; vault browsing, decode, audit investigation, PDF rewrite/export, full DICOM review workflow, and full review workflows remain unimplemented. |
| Overall | 47% | Core workspace, vault MVP, tabular path, bounded DICOM/PDF/runtime slices, conservative media/FCS domain models, adapter/application review foundation, bounded runtime metadata review/PDF review/DICOM entries, browser tabular/PDF review surface with bounded CSV/XLSX/PDF import/export helpers, desktop request-preparation/localhost-submit/response workbench foundation with bounded CSV/XLSX/PDF/DICOM file import/export helpers, and local CLI foundations are present; major workflow depth and surface parity remain missing; unrelated scope-drift legacy CLI commands are not counted as core product progress. |
```

Update missing items to include:

```markdown
Missing items include deeper policy/detection crates, full review/governance workflows, richer browser UX including deeper upload/download UX beyond bounded CSV/XLSX/PDF import/export helpers, deeper desktop vault/decode/audit workflows, desktop PDF flow beyond request preparation and bounded review/export helper naming, desktop DICOM flow beyond bounded request/response/import/export helper support, broader import/export and upload flows, OCR, visual redaction, handwriting handling, full PDF rewrite/export, FCS semantic parsing, media rewrite/export, generalized spreadsheet handling, auth/session handling where needed, removal or isolation of scope-drift legacy CLI surfaces from product-facing documentation and roadmap claims, and production packaging/hardening.
```

- [x] **Step 2: Mark plan checkboxes complete after implementation evidence exists**

Update this plan file's checkboxes from `- [ ]` to `- [x]` only after the referenced implementation, tests, docs, and commit are done.

- [x] **Step 3: Verify docs and scope wording**

Run:

```bash
git diff --check
grep -nE 'CLI|Browser/web|Desktop app|Overall|Missing items|controller|orchestration|agent|moat' README.md
```

Expected: diff check passes; scope-drift terms appear only in negative/disclaimer wording and not as product roadmap expansion.

- [x] **Step 4: Commit Task 2**

```bash
git add README.md docs/superpowers/plans/2026-04-28-desktop-dicom-request-response.md
git commit -m "docs: truth-sync desktop DICOM completion"
```

## Self-Review

1. **Spec coverage:** This plan covers bounded desktop DICOM request preparation, file import, response rendering, export naming, README completion, and negative scope disclosures. It does not add vault browsing, decode, audit, OCR, PDF rewrite, generalized orchestration, controller, or agent workflow behavior.
2. **Placeholder scan:** No TBD/TODO/implement-later placeholders are present. Every code-bearing step includes concrete tests or concrete implementation shape.
3. **Type consistency:** The new mode name is consistently `DesktopWorkflowMode::DicomBase64`; runtime fields are consistently `dicom_bytes_base64`, `source_name`, `private_tag_policy`, and `rewritten_dicom_bytes_base64`.
