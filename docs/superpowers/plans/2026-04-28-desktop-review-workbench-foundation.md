# Desktop Review Workbench Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the desktop crate from a static skeleton into a bounded sensitive-workstation review workbench foundation that truthfully presents current local runtime-backed flows and limits.

**Architecture:** Add a focused `mdid-desktop` library module that owns desktop workflow state, mode disclosures, validation, endpoint selection, and JSON request construction for currently supported local runtime review/de-identification entries. Keep the GUI thin by rendering this state; do not implement networking, uploads, vault persistence, OCR, PDF rewrite/export, or workflow orchestration in this slice.

**Tech Stack:** Rust 2021 workspace, `mdid-desktop`, `eframe`/`egui`, `serde_json`, cargo test/clippy.

---

## File Structure

- Create: `crates/mdid-desktop/src/lib.rs`
  - Owns `DesktopWorkflowMode`, `DesktopWorkbenchState`, `DesktopSubmitRequest`, validation errors, bounded runtime endpoint mapping, disclosure strings, and JSON request construction.
- Modify: `crates/mdid-desktop/src/main.rs`
  - Uses the library state to render a bounded workstation UI with mode selection, payload/source/policy inputs, disclosures, validation status, endpoint preview, and explicit not-yet-implemented runtime submission notice.
- Modify: `crates/mdid-desktop/Cargo.toml`
  - Adds `serde_json` workspace dependency for request-body construction tests and future runtime submit wiring.
- Modify: `README.md`
  - Truth-sync completion snapshot after the landed desktop foundation. Desktop app may increase only modestly because this is a local UI/request-preparation foundation, not a full workflow implementation.
- Test: `crates/mdid-desktop/src/lib.rs` unit tests.

### Task 1: Desktop workflow request model and validation

**Files:**
- Create: `crates/mdid-desktop/src/lib.rs`
- Modify: `crates/mdid-desktop/Cargo.toml`
- Test: `crates/mdid-desktop/src/lib.rs`

- [x] **Step 1: Write failing tests for mode disclosures, validation, and request bodies**

Add `serde_json` to `crates/mdid-desktop/Cargo.toml` dependencies first so the test file can compile:

```toml
[dependencies]
eframe.workspace = true
egui.workspace = true
serde_json.workspace = true
```

Create `crates/mdid-desktop/src/lib.rs` with only tests and empty public stubs needed for compilation:

```rust
use serde_json::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DesktopWorkflowMode {
    CsvText,
    XlsxBase64,
    PdfBase64,
}

impl Default for DesktopWorkflowMode {
    fn default() -> Self {
        Self::CsvText
    }
}

pub struct DesktopWorkbenchState {
    pub mode: DesktopWorkflowMode,
    pub payload: String,
    pub field_policy_json: String,
    pub source_name: String,
}

pub struct DesktopSubmitRequest {
    pub endpoint: &'static str,
    pub body: Value,
}

#[derive(Debug, Eq, PartialEq)]
pub enum DesktopSubmitError {
    EmptyPayload,
    EmptySourceName,
    EmptyFieldPolicyJson,
    InvalidFieldPolicyJson(String),
}

impl DesktopWorkflowMode {
    pub fn label(self) -> &'static str { "" }
    pub fn endpoint(self) -> &'static str { "" }
    pub fn disclosure(self) -> &'static str { "" }
    pub fn requires_field_policies(self) -> bool { false }
}

impl Default for DesktopWorkbenchState {
    fn default() -> Self {
        Self {
            mode: DesktopWorkflowMode::CsvText,
            payload: String::new(),
            field_policy_json: String::new(),
            source_name: String::new(),
        }
    }
}

impl DesktopWorkbenchState {
    pub fn build_submit_request(&self) -> Result<DesktopSubmitRequest, DesktopSubmitError> {
        Err(DesktopSubmitError::EmptyPayload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_is_bounded_csv_workstation_flow() {
        let state = DesktopWorkbenchState::default();

        assert_eq!(state.mode, DesktopWorkflowMode::CsvText);
        assert_eq!(state.source_name, "local-workstation-review.pdf");
        assert!(state.field_policy_json.contains("patient_name"));
        assert!(DesktopWorkflowMode::CsvText.disclosure().contains("local runtime CSV"));
        assert!(DesktopWorkflowMode::CsvText.disclosure().contains("not a generalized workflow orchestrator"));
    }

    #[test]
    fn csv_request_targets_tabular_endpoint_with_field_policies() {
        let state = DesktopWorkbenchState {
            mode: DesktopWorkflowMode::CsvText,
            payload: "patient_id,patient_name\nMRN-001,Alice Smith".to_string(),
            field_policy_json: r#"{"patient_name":"Approve","patient_id":"ReviewRequired"}"#.to_string(),
            source_name: "ignored.pdf".to_string(),
        };

        let request = state.build_submit_request().expect("csv request should build");

        assert_eq!(request.endpoint, "/tabular/deidentify/csv");
        assert_eq!(request.body["csv"], "patient_id,patient_name\nMRN-001,Alice Smith");
        assert_eq!(request.body["field_policies"]["patient_name"], "Approve");
        assert!(request.body.get("source_name").is_none());
    }

    #[test]
    fn xlsx_request_targets_xlsx_endpoint_with_base64_payload() {
        let state = DesktopWorkbenchState {
            mode: DesktopWorkflowMode::XlsxBase64,
            payload: "UEsDBBQAAAA=".to_string(),
            field_policy_json: r#"{"patient_name":"Approve"}"#.to_string(),
            source_name: "ignored.pdf".to_string(),
        };

        let request = state.build_submit_request().expect("xlsx request should build");

        assert_eq!(request.endpoint, "/tabular/deidentify/xlsx");
        assert_eq!(request.body["xlsx_bytes_base64"], "UEsDBBQAAAA=");
        assert_eq!(request.body["field_policies"]["patient_name"], "Approve");
        assert!(request.body.get("source_name").is_none());
    }

    #[test]
    fn pdf_request_targets_review_endpoint_without_field_policies() {
        let state = DesktopWorkbenchState {
            mode: DesktopWorkflowMode::PdfBase64,
            payload: "JVBERi0xLjQK".to_string(),
            field_policy_json: r#"{"patient_name":"Approve"}"#.to_string(),
            source_name: "Radiology Report.pdf".to_string(),
        };

        let request = state.build_submit_request().expect("pdf request should build");

        assert_eq!(request.endpoint, "/pdf/deidentify");
        assert_eq!(request.body["pdf_bytes_base64"], "JVBERi0xLjQK");
        assert_eq!(request.body["source_name"], "Radiology Report.pdf");
        assert!(request.body.get("field_policies").is_none());
    }

    #[test]
    fn validation_rejects_blank_payload_source_and_bad_policy_json() {
        let mut state = DesktopWorkbenchState::default();
        state.payload = "   ".to_string();
        assert_eq!(state.build_submit_request(), Err(DesktopSubmitError::EmptyPayload));

        state.mode = DesktopWorkflowMode::PdfBase64;
        state.payload = "JVBERi0xLjQK".to_string();
        state.source_name = "   ".to_string();
        assert_eq!(state.build_submit_request(), Err(DesktopSubmitError::EmptySourceName));

        state.mode = DesktopWorkflowMode::CsvText;
        state.payload = "patient_name\nAlice".to_string();
        state.field_policy_json = "not json".to_string();
        assert!(matches!(state.build_submit_request(), Err(DesktopSubmitError::InvalidFieldPolicyJson(_))));
    }
}
```

- [x] **Step 2: Run test to verify it fails for missing behavior**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-desktop default_state_is_bounded_csv_workstation_flow csv_request_targets_tabular_endpoint_with_field_policies xlsx_request_targets_xlsx_endpoint_with_base64_payload pdf_request_targets_review_endpoint_without_field_policies validation_rejects_blank_payload_source_and_bad_policy_json -- --nocapture
```

Expected: FAIL because the stub methods return empty strings or `EmptyPayload`, proving the tests exercise missing behavior.

- [x] **Step 3: Implement minimal library behavior**

Replace `crates/mdid-desktop/src/lib.rs` with:

```rust
use serde_json::{json, Value};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DesktopWorkflowMode {
    CsvText,
    XlsxBase64,
    PdfBase64,
}

impl Default for DesktopWorkflowMode {
    fn default() -> Self {
        Self::CsvText
    }
}

impl DesktopWorkflowMode {
    pub const ALL: [Self; 3] = [Self::CsvText, Self::XlsxBase64, Self::PdfBase64];

    pub fn label(self) -> &'static str {
        match self {
            Self::CsvText => "CSV text",
            Self::XlsxBase64 => "XLSX base64",
            Self::PdfBase64 => "PDF base64 review",
        }
    }

    pub fn endpoint(self) -> &'static str {
        match self {
            Self::CsvText => "/tabular/deidentify/csv",
            Self::XlsxBase64 => "/tabular/deidentify/xlsx",
            Self::PdfBase64 => "/pdf/deidentify",
        }
    }

    pub fn disclosure(self) -> &'static str {
        match self {
            Self::CsvText => "Desktop CSV mode prepares a local runtime CSV request for bounded tabular de-identification. It is not a generalized workflow orchestrator, upload manager, or background controller.",
            Self::XlsxBase64 => "Desktop XLSX mode prepares a local runtime request for the existing bounded first-non-empty-worksheet XLSX flow. It does not provide workbook-wide sheet selection, generalized import/export, or workflow orchestration.",
            Self::PdfBase64 => "Desktop PDF mode prepares a local runtime PDF review request only. It reports text-layer/OCR-needed review status and does not perform OCR, handwriting handling, visual redaction, or PDF rewrite/export.",
        }
    }

    pub fn requires_field_policies(self) -> bool {
        matches!(self, Self::CsvText | Self::XlsxBase64)
    }

    pub fn payload_hint(self) -> &'static str {
        match self {
            Self::CsvText => "Paste CSV text to send to the local runtime.",
            Self::XlsxBase64 => "Paste base64-encoded XLSX workbook bytes for the bounded first-sheet runtime flow.",
            Self::PdfBase64 => "Paste base64-encoded PDF bytes for review-only runtime analysis.",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesktopWorkbenchState {
    pub mode: DesktopWorkflowMode,
    pub payload: String,
    pub field_policy_json: String,
    pub source_name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesktopSubmitRequest {
    pub endpoint: &'static str,
    pub body: Value,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DesktopSubmitError {
    EmptyPayload,
    EmptySourceName,
    EmptyFieldPolicyJson,
    InvalidFieldPolicyJson(String),
}

impl Default for DesktopWorkbenchState {
    fn default() -> Self {
        Self {
            mode: DesktopWorkflowMode::CsvText,
            payload: String::new(),
            field_policy_json: r#"{"patient_name":"Approve","patient_id":"ReviewRequired"}"#.to_string(),
            source_name: "local-workstation-review.pdf".to_string(),
        }
    }
}

impl DesktopWorkbenchState {
    pub fn build_submit_request(&self) -> Result<DesktopSubmitRequest, DesktopSubmitError> {
        let payload = self.payload.trim();
        if payload.is_empty() {
            return Err(DesktopSubmitError::EmptyPayload);
        }

        let body = match self.mode {
            DesktopWorkflowMode::CsvText => {
                let field_policies = self.parse_field_policies()?;
                json!({
                    "csv": payload,
                    "field_policies": field_policies,
                })
            }
            DesktopWorkflowMode::XlsxBase64 => {
                let field_policies = self.parse_field_policies()?;
                json!({
                    "xlsx_bytes_base64": payload,
                    "field_policies": field_policies,
                })
            }
            DesktopWorkflowMode::PdfBase64 => {
                let source_name = self.source_name.trim();
                if source_name.is_empty() {
                    return Err(DesktopSubmitError::EmptySourceName);
                }
                json!({
                    "pdf_bytes_base64": payload,
                    "source_name": source_name,
                })
            }
        };

        Ok(DesktopSubmitRequest {
            endpoint: self.mode.endpoint(),
            body,
        })
    }

    fn parse_field_policies(&self) -> Result<Value, DesktopSubmitError> {
        let policy = self.field_policy_json.trim();
        if policy.is_empty() {
            return Err(DesktopSubmitError::EmptyFieldPolicyJson);
        }
        serde_json::from_str(policy)
            .map_err(|error| DesktopSubmitError::InvalidFieldPolicyJson(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_is_bounded_csv_workstation_flow() {
        let state = DesktopWorkbenchState::default();

        assert_eq!(state.mode, DesktopWorkflowMode::CsvText);
        assert_eq!(state.source_name, "local-workstation-review.pdf");
        assert!(state.field_policy_json.contains("patient_name"));
        assert!(DesktopWorkflowMode::CsvText.disclosure().contains("local runtime CSV"));
        assert!(DesktopWorkflowMode::CsvText.disclosure().contains("not a generalized workflow orchestrator"));
    }

    #[test]
    fn csv_request_targets_tabular_endpoint_with_field_policies() {
        let state = DesktopWorkbenchState {
            mode: DesktopWorkflowMode::CsvText,
            payload: "patient_id,patient_name\nMRN-001,Alice Smith".to_string(),
            field_policy_json: r#"{"patient_name":"Approve","patient_id":"ReviewRequired"}"#.to_string(),
            source_name: "ignored.pdf".to_string(),
        };

        let request = state.build_submit_request().expect("csv request should build");

        assert_eq!(request.endpoint, "/tabular/deidentify/csv");
        assert_eq!(request.body["csv"], "patient_id,patient_name\nMRN-001,Alice Smith");
        assert_eq!(request.body["field_policies"]["patient_name"], "Approve");
        assert!(request.body.get("source_name").is_none());
    }

    #[test]
    fn xlsx_request_targets_xlsx_endpoint_with_base64_payload() {
        let state = DesktopWorkbenchState {
            mode: DesktopWorkflowMode::XlsxBase64,
            payload: "UEsDBBQAAAA=".to_string(),
            field_policy_json: r#"{"patient_name":"Approve"}"#.to_string(),
            source_name: "ignored.pdf".to_string(),
        };

        let request = state.build_submit_request().expect("xlsx request should build");

        assert_eq!(request.endpoint, "/tabular/deidentify/xlsx");
        assert_eq!(request.body["xlsx_bytes_base64"], "UEsDBBQAAAA=");
        assert_eq!(request.body["field_policies"]["patient_name"], "Approve");
        assert!(request.body.get("source_name").is_none());
    }

    #[test]
    fn pdf_request_targets_review_endpoint_without_field_policies() {
        let state = DesktopWorkbenchState {
            mode: DesktopWorkflowMode::PdfBase64,
            payload: "JVBERi0xLjQK".to_string(),
            field_policy_json: r#"{"patient_name":"Approve"}"#.to_string(),
            source_name: "Radiology Report.pdf".to_string(),
        };

        let request = state.build_submit_request().expect("pdf request should build");

        assert_eq!(request.endpoint, "/pdf/deidentify");
        assert_eq!(request.body["pdf_bytes_base64"], "JVBERi0xLjQK");
        assert_eq!(request.body["source_name"], "Radiology Report.pdf");
        assert!(request.body.get("field_policies").is_none());
    }

    #[test]
    fn validation_rejects_blank_payload_source_and_bad_policy_json() {
        let mut state = DesktopWorkbenchState::default();
        state.payload = "   ".to_string();
        assert_eq!(state.build_submit_request(), Err(DesktopSubmitError::EmptyPayload));

        state.mode = DesktopWorkflowMode::PdfBase64;
        state.payload = "JVBERi0xLjQK".to_string();
        state.source_name = "   ".to_string();
        assert_eq!(state.build_submit_request(), Err(DesktopSubmitError::EmptySourceName));

        state.mode = DesktopWorkflowMode::CsvText;
        state.payload = "patient_name\nAlice".to_string();
        state.field_policy_json = "not json".to_string();
        assert!(matches!(state.build_submit_request(), Err(DesktopSubmitError::InvalidFieldPolicyJson(_))));
    }
}
```

- [x] **Step 4: Run tests to verify pass**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-desktop --lib -- --nocapture
```

Expected: PASS, five tests pass.

- [x] **Step 5: Run crate check**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-desktop
cargo clippy -p mdid-desktop --all-targets -- -D warnings
```

Expected: PASS.

- [x] **Step 6: Commit**

```bash
git add crates/mdid-desktop/Cargo.toml crates/mdid-desktop/src/lib.rs Cargo.lock
git commit -m "feat(desktop): add bounded workstation request model"
```

### Task 2: Render bounded desktop workbench UI and truth-sync README

**Files:**
- Modify: `crates/mdid-desktop/src/main.rs`
- Modify: `README.md`
- Test: `crates/mdid-desktop/src/lib.rs`

- [ ] **Step 1: Write failing test for UI status copy helper**

Append this test to `crates/mdid-desktop/src/lib.rs` tests:

```rust
#[test]
fn status_message_explains_preview_only_runtime_submit_boundary() {
    let state = DesktopWorkbenchState {
        mode: DesktopWorkflowMode::PdfBase64,
        payload: "JVBERi0xLjQK".to_string(),
        field_policy_json: String::new(),
        source_name: "scan.pdf".to_string(),
    };

    let message = state.status_message();

    assert!(message.contains("Ready to submit to /pdf/deidentify"));
    assert!(message.contains("submission is not wired in this desktop slice"));
    assert!(message.contains("no OCR, visual redaction, PDF rewrite/export, or controller workflow"));
}
```

Also add a stub method so the test compiles:

```rust
impl DesktopWorkbenchState {
    pub fn status_message(&self) -> String {
        String::new()
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-desktop status_message_explains_preview_only_runtime_submit_boundary -- --nocapture
```

Expected: FAIL because `status_message()` returns an empty string.

- [ ] **Step 3: Implement `status_message()` and update GUI**

Implement `status_message()` in `crates/mdid-desktop/src/lib.rs`:

```rust
impl DesktopWorkbenchState {
    pub fn status_message(&self) -> String {
        match self.build_submit_request() {
            Ok(request) => format!(
                "Ready to submit to {}; submission is not wired in this desktop slice. This workstation preview performs no OCR, visual redaction, PDF rewrite/export, or controller workflow.",
                request.endpoint
            ),
            Err(error) => format!("Not ready: {error:?}"),
        }
    }
}
```

Replace `crates/mdid-desktop/src/main.rs` with:

```rust
use mdid_desktop::{DesktopWorkbenchState, DesktopWorkflowMode};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "med-de-id desktop",
        options,
        Box::new(|_cc| Box::<DesktopApp>::default()),
    )
}

#[derive(Default)]
struct DesktopApp {
    state: DesktopWorkbenchState,
}

impl eframe::App for DesktopApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("med-de-id desktop workstation");
            ui.label("Bounded sensitive-workstation foundation for local runtime-backed review preparation.");

            egui::ComboBox::from_label("Workflow mode")
                .selected_text(self.state.mode.label())
                .show_ui(ui, |ui| {
                    for mode in DesktopWorkflowMode::ALL {
                        ui.selectable_value(&mut self.state.mode, mode, mode.label());
                    }
                });

            ui.separator();
            ui.label(self.state.mode.disclosure());
            ui.label(format!("Runtime endpoint preview: {}", self.state.mode.endpoint()));
            ui.label(self.state.mode.payload_hint());
            ui.text_edit_multiline(&mut self.state.payload);

            if self.state.mode.requires_field_policies() {
                ui.label("Field policy JSON");
                ui.text_edit_multiline(&mut self.state.field_policy_json);
            } else {
                ui.label("PDF source name");
                ui.text_edit_singleline(&mut self.state.source_name);
            }

            ui.separator();
            ui.label(self.state.status_message());
            ui.label("Runtime networking, file picker upload/download UX, vault browsing, decode, and audit investigation are not implemented in this desktop slice.");
        });
    }
}
```

- [ ] **Step 4: Update README completion snapshot truthfully**

Patch README status table and bullets:

```markdown
| Desktop app | 16% | Bounded sensitive-workstation foundation now renders local runtime request-preparation modes for CSV, XLSX, and PDF review with honest disclosures; actual runtime submission, file picker upload/download UX, vault browsing, decode, audit investigation, and full review workflows remain unimplemented. |
| Overall | 40% | Core workspace, vault MVP, tabular path, bounded DICOM/PDF/runtime slices, conservative media/FCS domain models, adapter/application review foundation, bounded runtime metadata review and PDF review entries, browser tabular/PDF review surface, desktop request-preparation foundation, and local CLI foundations are present; major workflow depth and surface parity remain missing; scope-drift controller/moat CLI wording is not counted as core product progress. |
```

Also add an implemented-so-far bullet:

```markdown
- `mdid-desktop` now renders a bounded sensitive-workstation foundation for preparing local runtime CSV, XLSX, and PDF review requests with endpoint previews, validation status, mode-specific disclosures, and explicit notice that runtime submission, file picker upload/download UX, vault browsing, decode, audit investigation, OCR, visual redaction, PDF rewrite/export, and controller workflows are not implemented in this desktop slice
```

Update Missing items to keep `desktop app behavior` honest:

```markdown
Missing items include deeper policy/detection crates, full review/governance workflows, richer browser UX including browser upload UX, actual desktop runtime submission, desktop file picker upload/download UX, desktop vault/decode/audit workflows, desktop PDF flow beyond request preparation, broader import/export and upload flows, OCR, visual redaction, handwriting handling, full PDF rewrite/export, FCS semantic parsing, media rewrite/export, generalized spreadsheet handling, auth/session handling where needed, generalized workflow orchestration, removal or isolation of scope-drift controller/moat CLI surfaces from product-facing documentation and roadmap claims, and production packaging/hardening.
```

- [ ] **Step 5: Verify tests and docs**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-desktop -- --nocapture
cargo clippy -p mdid-desktop --all-targets -- -D warnings
git diff --check
grep -nE 'Desktop app|Overall|mdid-desktop|controller|orchestration|agent|moat' README.md
```

Expected: tests/clippy/diff check pass. Grep hits for controller/orchestration/agent/moat must be negative scope-drift or explicit not-implemented wording only.

- [ ] **Step 6: Commit**

```bash
git add crates/mdid-desktop/src/lib.rs crates/mdid-desktop/src/main.rs README.md
git commit -m "feat(desktop): render bounded workstation foundation"
```

### Task 3: Merge verified desktop foundation to develop

**Files:**
- No code changes expected beyond merge.

- [ ] **Step 1: Final branch verification**

Run:

```bash
source "$HOME/.cargo/env"
git branch --show-current
git status --short
git log --oneline -8 --decorate
cargo test -p mdid-desktop
cargo clippy -p mdid-desktop --all-targets -- -D warnings
git diff develop...HEAD --stat
```

Expected: on `feature/desktop-review-workbench-foundation`, clean worktree, desktop tests/clippy pass, diff limited to desktop crate, README, plan, and lockfile.

- [ ] **Step 2: Merge to develop**

Run:

```bash
git checkout develop
git merge --no-ff feature/desktop-review-workbench-foundation -m "merge: add desktop review workbench foundation"
```

Expected: merge succeeds.

- [ ] **Step 3: Develop verification**

Run:

```bash
source "$HOME/.cargo/env"
git branch --show-current
git status --short
cargo test -p mdid-desktop
cargo clippy -p mdid-desktop --all-targets -- -D warnings
git log --oneline -8 --decorate
```

Expected: on `develop`, clean worktree, desktop tests/clippy pass, merge commit visible.

## Self-Review

1. Spec coverage: This plan addresses desktop app behavior, README completion maintenance, local-first runtime-backed surface alignment, and scope-drift avoidance. It does not implement actual runtime networking/upload/download/vault/decode/audit flows and explicitly documents those as remaining gaps.
2. Placeholder scan: No TBD/TODO/implement-later placeholders are present; each code-changing step includes concrete code or exact README text.
3. Type consistency: `DesktopWorkflowMode`, `DesktopWorkbenchState`, `DesktopSubmitRequest`, `DesktopSubmitError`, and `status_message()` names are consistent across tasks.
