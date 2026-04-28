# Desktop Runtime Response Workbench Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded desktop-side runtime response workbench so the sensitive workstation can render local runtime results/errors for CSV, XLSX, and PDF review request modes without claiming full networking, file picker, vault, decode, audit, OCR, or PDF rewrite support.

**Architecture:** Extend `mdid-desktop`’s focused request-preparation model with a small response-state model that accepts runtime-shaped JSON envelopes and renders honest summary/review/output/error text. Keep the slice local and deterministic: no async networking, no generalized workflow orchestration, and no controller/agent concepts.

**Tech Stack:** Rust workspace, `mdid-desktop`, `serde_json`, eframe/egui, Cargo tests.

---

## File Structure

- Modify `crates/mdid-desktop/src/lib.rs`: add `DesktopWorkflowResponseState`, response parsing/apply helpers, tests for CSV/XLSX/PDF response envelopes and error handling, and update status copy to say response rendering is available while networking is still not wired.
- Modify `crates/mdid-desktop/src/main.rs`: show response summary/review/output/error panes using the new response state; keep the UI honest that runtime networking and upload/download UX are not implemented in this slice.
- Modify `README.md`: update desktop completion/status and overall completion snapshot after landed tests prove desktop response rendering exists.

### Task 1: Desktop Runtime Response State

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Add these tests inside the existing `#[cfg(test)] mod tests` in `crates/mdid-desktop/src/lib.rs`:

```rust
    #[test]
    fn response_state_renders_csv_runtime_success_envelope() {
        let mut response = DesktopWorkflowResponseState::default();

        response.apply_success_json(
            DesktopWorkflowMode::CsvText,
            json!({
                "rewritten_csv": "patient_name\n<NAME-1>",
                "summary": {"encoded_fields": 1, "review_required": 0},
                "review_queue": []
            }),
        );

        assert_eq!(response.banner, "CSV text runtime response rendered locally.");
        assert!(response.output.contains("<NAME-1>"));
        assert!(response.summary.contains("encoded_fields"));
        assert_eq!(response.review_queue, "[]");
        assert!(response.error.is_none());
    }

    #[test]
    fn response_state_renders_xlsx_runtime_success_envelope() {
        let mut response = DesktopWorkflowResponseState::default();

        response.apply_success_json(
            DesktopWorkflowMode::XlsxBase64,
            json!({
                "rewritten_workbook_base64": "UEsDBAo=",
                "summary": {"encoded_fields": 2},
                "review_queue": [{"header":"patient_id"}]
            }),
        );

        assert_eq!(response.banner, "XLSX base64 runtime response rendered locally.");
        assert_eq!(response.output, "UEsDBAo=");
        assert!(response.summary.contains("encoded_fields"));
        assert!(response.review_queue.contains("patient_id"));
        assert!(response.error.is_none());
    }

    #[test]
    fn response_state_renders_pdf_review_runtime_success_envelope_without_rewrite_claim() {
        let mut response = DesktopWorkflowResponseState::default();

        response.apply_success_json(
            DesktopWorkflowMode::PdfBase64Review,
            json!({
                "rewritten_pdf_bytes_base64": null,
                "summary": {"pages": 1, "ocr_required_pages": 1},
                "pages": [{"page_number": 1, "status": "ocr_required"}],
                "review_queue": [{"page_number": 1, "reason":"ocr_required"}]
            }),
        );

        assert_eq!(response.banner, "PDF base64 review runtime response rendered locally; no PDF rewrite/export is available.");
        assert_eq!(response.output, "No rewritten PDF bytes returned by the bounded review route.");
        assert!(response.summary.contains("ocr_required_pages"));
        assert!(response.review_queue.contains("ocr_required"));
        assert!(response.error.is_none());
    }

    #[test]
    fn response_state_records_runtime_error_without_stale_output() {
        let mut response = DesktopWorkflowResponseState::default();
        response.apply_success_json(
            DesktopWorkflowMode::CsvText,
            json!({"rewritten_csv":"patient_name\n<NAME-1>","summary":{},"review_queue":[]}),
        );

        response.apply_error("runtime rejected invalid payload");

        assert_eq!(response.banner, "Runtime response error.");
        assert_eq!(response.output, "");
        assert_eq!(response.summary, "No successful runtime summary rendered yet.");
        assert_eq!(response.review_queue, "No review queue rendered yet.");
        assert_eq!(response.error.as_deref(), Some("runtime rejected invalid payload"));
    }
```

- [ ] **Step 2: Run tests to verify RED**

Run: `cargo test -p mdid-desktop response_state_ -- --nocapture`

Expected: FAIL because `DesktopWorkflowResponseState` is not defined.

- [ ] **Step 3: Implement minimal response state**

Add this production code in `crates/mdid-desktop/src/lib.rs` after `DesktopWorkflowValidationError`:

```rust
#[derive(Clone, PartialEq, Eq)]
pub struct DesktopWorkflowResponseState {
    pub banner: String,
    pub output: String,
    pub summary: String,
    pub review_queue: String,
    pub error: Option<String>,
}

impl Default for DesktopWorkflowResponseState {
    fn default() -> Self {
        Self {
            banner: "No runtime response rendered yet.".to_string(),
            output: String::new(),
            summary: "No successful runtime summary rendered yet.".to_string(),
            review_queue: "No review queue rendered yet.".to_string(),
            error: None,
        }
    }
}

impl DesktopWorkflowResponseState {
    pub fn apply_success_json(&mut self, mode: DesktopWorkflowMode, envelope: serde_json::Value) {
        self.banner = match mode {
            DesktopWorkflowMode::CsvText => "CSV text runtime response rendered locally.".to_string(),
            DesktopWorkflowMode::XlsxBase64 => {
                "XLSX base64 runtime response rendered locally.".to_string()
            }
            DesktopWorkflowMode::PdfBase64Review => "PDF base64 review runtime response rendered locally; no PDF rewrite/export is available.".to_string(),
        };

        self.output = match mode {
            DesktopWorkflowMode::CsvText => envelope
                .get("rewritten_csv")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string(),
            DesktopWorkflowMode::XlsxBase64 => envelope
                .get("rewritten_workbook_base64")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string(),
            DesktopWorkflowMode::PdfBase64Review => envelope
                .get("rewritten_pdf_bytes_base64")
                .and_then(serde_json::Value::as_str)
                .map(ToString::to_string)
                .unwrap_or_else(|| {
                    "No rewritten PDF bytes returned by the bounded review route.".to_string()
                }),
        };

        self.summary = pretty_json_field(&envelope, "summary");
        self.review_queue = pretty_json_field(&envelope, "review_queue");
        self.error = None;
    }

    pub fn apply_error(&mut self, message: impl Into<String>) {
        self.banner = "Runtime response error.".to_string();
        self.output.clear();
        self.summary = "No successful runtime summary rendered yet.".to_string();
        self.review_queue = "No review queue rendered yet.".to_string();
        self.error = Some(message.into());
    }
}

fn pretty_json_field(envelope: &serde_json::Value, field: &str) -> String {
    envelope
        .get(field)
        .and_then(|value| serde_json::to_string_pretty(value).ok())
        .unwrap_or_else(|| "null".to_string())
}
```

Update `DesktopWorkflowRequestState::status_message` success copy to:

```rust
                "Ready to submit to {}; this slice can render runtime-shaped responses locally, but desktop networking is not wired. This workstation preview performs no OCR, visual redaction, PDF rewrite/export, file picker upload/download UX, vault/decode/audit workflow, or controller workflow.",
```

- [ ] **Step 4: Run tests to verify GREEN**

Run: `cargo test -p mdid-desktop response_state_ -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Run desktop crate tests**

Run: `cargo test -p mdid-desktop`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/mdid-desktop/src/lib.rs
git commit -m "feat(desktop): render runtime response state"
```

### Task 2: Desktop UI Response Panels and README Truth Sync

**Files:**
- Modify: `crates/mdid-desktop/src/main.rs`
- Modify: `README.md`

- [ ] **Step 1: Write the failing UI-facing test**

Add this test inside the existing `#[cfg(test)] mod tests` in `crates/mdid-desktop/src/lib.rs`:

```rust
    #[test]
    fn response_state_default_copy_keeps_networking_and_workflow_limits_honest() {
        let response = DesktopWorkflowResponseState::default();

        assert_eq!(response.banner, "No runtime response rendered yet.");
        assert_eq!(response.summary, "No successful runtime summary rendered yet.");
        assert_eq!(response.review_queue, "No review queue rendered yet.");
        assert!(response.error.is_none());
    }
```

- [ ] **Step 2: Run test to verify RED if Task 1 was not already complete, otherwise confirm coverage**

Run: `cargo test -p mdid-desktop response_state_default_copy_keeps_networking_and_workflow_limits_honest -- --nocapture`

Expected: PASS if Task 1 already added the default response model; if it fails, complete Task 1 first.

- [ ] **Step 3: Wire response state into the egui application shell**

In `crates/mdid-desktop/src/main.rs`, change imports and app state to:

```rust
use mdid_desktop::{
    DesktopWorkflowMode, DesktopWorkflowRequestState, DesktopWorkflowResponseState,
};

#[derive(Default)]
struct DesktopApp {
    request_state: DesktopWorkflowRequestState,
    response_state: DesktopWorkflowResponseState,
}
```

After the status-message label, add response panes:

```rust
            ui.separator();
            ui.heading("Runtime response workbench");
            ui.label(self.response_state.banner.as_str());
            if let Some(error) = &self.response_state.error {
                ui.colored_label(egui::Color32::RED, error);
            }
            ui.label("Summary");
            ui.add(
                egui::TextEdit::multiline(&mut self.response_state.summary)
                    .desired_rows(4)
                    .interactive(false),
            );
            ui.label("Review queue");
            ui.add(
                egui::TextEdit::multiline(&mut self.response_state.review_queue)
                    .desired_rows(4)
                    .interactive(false),
            );
            ui.label("Rewritten output / review notice");
            ui.add(
                egui::TextEdit::multiline(&mut self.response_state.output)
                    .desired_rows(6)
                    .interactive(false),
            );
```

Update the limitation label to:

```rust
                "Not implemented in this desktop slice: runtime networking, file picker upload/download UX, vault browsing, decode, audit investigation, OCR, visual redaction, PDF rewrite/export, and controller workflows.",
```

- [ ] **Step 4: Update README completion snapshot**

In `README.md`, update the completion table and implemented bullets exactly as follows:

- Desktop app completion: `18%`
- Overall completion: `41%`
- Desktop status text: `Bounded sensitive-workstation foundation now renders local runtime request-preparation modes and runtime-shaped response panes for CSV, XLSX, and PDF review with honest disclosures; actual runtime submission, file picker upload/download UX, vault browsing, decode, audit investigation, and full review workflows remain unimplemented.`
- Add/update implemented bullet: `mdid-desktop now renders a bounded sensitive-workstation foundation for preparing local runtime CSV, XLSX, and PDF review requests and for displaying runtime-shaped summary/review/output/error panes locally, with endpoint previews, validation status, mode-specific disclosures, and explicit notice that runtime submission, file picker upload/download UX, vault browsing, decode, audit investigation, OCR, visual redaction, PDF rewrite/export, and controller workflows are not implemented in this desktop slice`

- [ ] **Step 5: Run verification**

Run: `cargo test -p mdid-desktop`

Expected: PASS.

Run: `cargo test -p mdid-browser --lib`

Expected: PASS, proving browser work was not regressed by desktop changes.

- [ ] **Step 6: Commit**

```bash
git add crates/mdid-desktop/src/lib.rs crates/mdid-desktop/src/main.rs README.md docs/superpowers/plans/2026-04-28-desktop-runtime-response-workbench.md
git commit -m "docs: update desktop response workbench status"
```

## Self-Review

- Spec coverage: The plan adds only bounded desktop response rendering and README truth-sync. It does not add networking, upload/download, vault/decode/audit, OCR, rewrite/export, or controller/agent workflow features.
- Placeholder scan: No TBD/TODO/implement-later placeholders remain.
- Type consistency: `DesktopWorkflowResponseState`, `apply_success_json`, `apply_error`, and `pretty_json_field` are used consistently across tasks.
