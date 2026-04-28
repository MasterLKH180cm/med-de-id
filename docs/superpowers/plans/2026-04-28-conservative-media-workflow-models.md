# Conservative Media Workflow Models Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a small domain-model foundation for conservative image/video/FCS ingestion status without implementing OCR, pixel redaction, FCS semantic parsing, or workflow orchestration.

**Architecture:** Keep this slice inside `mdid-domain` as pure serializable workflow value types that downstream adapters/application surfaces can use later. The models must communicate honest conservative scope: metadata-only review signals for media/FCS and no claim of final rewrite/export support.

**Tech Stack:** Rust workspace, `mdid-domain`, Serde, Cargo integration tests.

---

## File Structure

- Modify: `crates/mdid-domain/src/lib.rs` — add public conservative media/FCS workflow enums and structs near the existing PDF workflow models.
- Create: `crates/mdid-domain/tests/conservative_media_workflow_models.rs` — behavior tests for wire values, honest review semantics, sanitized field paths, and redacted debug output.
- Modify: `README.md` — truth-sync completion snapshot and implemented-status bullets if this lands.

### Task 1: Add conservative media/FCS workflow domain models

**Files:**
- Modify: `crates/mdid-domain/src/lib.rs`
- Create: `crates/mdid-domain/tests/conservative_media_workflow_models.rs`

- [ ] **Step 1: Write the failing tests**

Create `crates/mdid-domain/tests/conservative_media_workflow_models.rs` with:

```rust
use mdid_domain::{ConservativeMediaCandidate, ConservativeMediaFormat, ConservativeMediaRef, ConservativeMediaScanStatus, ConservativeMediaSummary};

#[test]
fn conservative_media_format_uses_stable_snake_case_wire_values() {
    assert_eq!(serde_json::to_string(&ConservativeMediaFormat::Image).unwrap(), "\"image\"");
    assert_eq!(serde_json::to_string(&ConservativeMediaFormat::Video).unwrap(), "\"video\"");
    assert_eq!(serde_json::to_string(&ConservativeMediaFormat::Fcs).unwrap(), "\"fcs\"");
}

#[test]
fn conservative_media_status_uses_stable_snake_case_wire_values() {
    assert_eq!(serde_json::to_string(&ConservativeMediaScanStatus::MetadataOnly).unwrap(), "\"metadata_only\"");
    assert_eq!(serde_json::to_string(&ConservativeMediaScanStatus::OcrOrVisualReviewRequired).unwrap(), "\"ocr_or_visual_review_required\"");
    assert_eq!(serde_json::to_string(&ConservativeMediaScanStatus::UnsupportedPayload).unwrap(), "\"unsupported_payload\"");
}

#[test]
fn conservative_media_ref_sanitizes_slashes_in_field_path_labels() {
    let field_ref = ConservativeMediaRef { artifact_label: "dicom/screenshots/patient.png".to_string(), metadata_key: "Patient/Name".to_string() };
    assert_eq!(field_ref.field_path(), "media:dicom_screenshots_patient.png:Patient_Name");
}

#[test]
fn conservative_media_candidate_debug_redacts_source_value() {
    let candidate = ConservativeMediaCandidate { field_ref: ConservativeMediaRef { artifact_label: "patient.png".to_string(), metadata_key: "EXIF Artist".to_string() }, format: ConservativeMediaFormat::Image, phi_type: "person_name".to_string(), source_value: "Jane Patient".to_string(), confidence: 0.55, status: ConservativeMediaScanStatus::MetadataOnly };
    let debug = format!("{candidate:?}");
    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("Jane Patient"));
}

#[test]
fn conservative_media_summary_requires_review_for_visual_review_or_review_candidates() {
    let clean = ConservativeMediaSummary { total_items: 1, metadata_only_items: 1, visual_review_required_items: 0, unsupported_items: 0, review_required_candidates: 0 };
    assert!(!clean.requires_review());

    let visual_review = ConservativeMediaSummary { visual_review_required_items: 1, ..clean.clone() };
    assert!(visual_review.requires_review());

    let candidate_review = ConservativeMediaSummary { review_required_candidates: 1, ..clean };
    assert!(candidate_review.requires_review());
}
```

- [ ] **Step 2: Run tests to verify RED**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-domain --test conservative_media_workflow_models`

Expected: FAIL with unresolved imports for the new conservative media/FCS types.

- [ ] **Step 3: Implement minimal domain models**

Add to `crates/mdid-domain/src/lib.rs` near the PDF workflow structs:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConservativeMediaFormat {
    Image,
    Video,
    Fcs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConservativeMediaScanStatus {
    MetadataOnly,
    OcrOrVisualReviewRequired,
    UnsupportedPayload,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConservativeMediaRef {
    pub artifact_label: String,
    pub metadata_key: String,
}

impl ConservativeMediaRef {
    pub fn field_path(&self) -> String {
        let artifact = sanitize_field_path_segment(&self.artifact_label);
        let metadata = sanitize_field_path_segment(&self.metadata_key);
        format!("media:{artifact}:{metadata}")
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ConservativeMediaCandidate {
    pub field_ref: ConservativeMediaRef,
    pub format: ConservativeMediaFormat,
    pub phi_type: String,
    pub source_value: String,
    pub confidence: f32,
    pub status: ConservativeMediaScanStatus,
}

impl std::fmt::Debug for ConservativeMediaCandidate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConservativeMediaCandidate")
            .field("field_ref", &self.field_ref)
            .field("format", &self.format)
            .field("phi_type", &self.phi_type)
            .field("source_value", &"<redacted>")
            .field("confidence", &self.confidence)
            .field("status", &self.status)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConservativeMediaSummary {
    pub total_items: usize,
    pub metadata_only_items: usize,
    pub visual_review_required_items: usize,
    pub unsupported_items: usize,
    pub review_required_candidates: usize,
}

impl ConservativeMediaSummary {
    pub fn requires_review(&self) -> bool {
        self.visual_review_required_items > 0 || self.review_required_candidates > 0
    }
}
```

If `sanitize_field_path_segment` is not already available as a private helper, add:

```rust
fn sanitize_field_path_segment(value: &str) -> String {
    value.replace(['/', '\\'], "_")
}
```

- [ ] **Step 4: Run targeted tests to verify GREEN**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-domain --test conservative_media_workflow_models`

Expected: PASS.

- [ ] **Step 5: Run broader domain tests**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-domain`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/mdid-domain/src/lib.rs crates/mdid-domain/tests/conservative_media_workflow_models.rs
git commit -m "feat(domain): add conservative media workflow models"
```

### Task 2: Truth-sync README completion snapshot

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Verify landed behavior before editing docs**

Run: `source "$HOME/.cargo/env" && cargo test -p mdid-domain --test conservative_media_workflow_models && cargo test -p mdid-domain`

Expected: PASS.

- [ ] **Step 2: Update README truthfully**

Change the completion snapshot only if the landed conservative media/FCS foundation changes the visible status. The intended truthful target after Task 1 is:

```markdown
| CLI | 45% | Early automation surface with bounded local workflow entry points; not a complete automation product |
| Browser/web | 25% | Bounded localhost tabular de-identification page backed by local runtime routes; not a broader browser governance workspace |
| Desktop app | 10% | Early scaffold only; sensitive-workstation review, vault, decode, and audit flows remain mostly unimplemented |
| Overall | 36% | Core workspace, vault MVP, tabular path, bounded DICOM/PDF/runtime slices, conservative media/FCS domain models, browser tabular surface, and controller-visible CLI slices are present, but major workflow depth and surface parity remain missing |
```

Also add an implemented-status bullet:

```markdown
- Conservative media/FCS domain workflow models now distinguish image/video/FCS metadata-only status, OCR-or-visual-review-required status, unsupported payloads, review-required summaries, and redacted candidate debug output; this is only a domain-model foundation and does not implement OCR, visual redaction, FCS semantic parsing, rewrite/export, or runtime/browser/desktop flows
```

- [ ] **Step 3: Commit README update**

```bash
git add README.md
git commit -m "docs: update conservative media completion snapshot"
```

## Self-Review

- Spec coverage: This plan covers the approved conservative image/video/FCS v1 direction at domain-model foundation depth only. It does not implement adapters/runtime/UI and explicitly forbids overclaiming OCR, visual redaction, FCS semantics, or export.
- Placeholder scan: No TBD/TODO/implement-later placeholders remain.
- Type consistency: Test names and type names match the implementation names in Task 1.
