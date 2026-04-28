# Conservative Media Adapter Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded adapter foundation for conservative image/video/FCS governance that inspects caller-provided metadata without claiming OCR, visual redaction, semantic FCS parsing, or rewrite/export support.

**Architecture:** Add a focused `mdid-adapters::ConservativeMediaAdapter` that converts explicit metadata key/value pairs into conservative `ConservativeMediaCandidate` review items and an honest `ConservativeMediaSummary`. The adapter is intentionally metadata-only and deterministic; unsupported/empty payloads are counted honestly and visual/OCR review is surfaced through status/summary rather than fabricated detections.

**Tech Stack:** Rust workspace, `mdid-domain` conservative media workflow models, `mdid-adapters`, Cargo integration tests, strict TDD.

---

## File Structure

- Create `crates/mdid-adapters/src/conservative_media.rs`
  - Owns the bounded conservative media adapter, input item structs, extraction output struct, and adapter error type.
  - Does not call OCR, image/video codecs, FCS parsers, filesystem APIs, daemons, agents, or controllers.
- Modify `crates/mdid-adapters/src/lib.rs`
  - Exports the new adapter types.
- Create `crates/mdid-adapters/tests/conservative_media_adapter.rs`
  - Covers metadata-only extraction, visual-review routing for image/video, unsupported payload counting, redacted debug output, and empty-label validation.
- Modify `README.md`
  - Truth-sync completion snapshot after landed adapter support. Completion should stay conservative unless controller-visible implementation and tests justify a percentage change.

## Scope Guard

This plan is in scope for med-de-id because it adds de-identification governance support for image/video/FCS metadata. It must not add agent workflow, controller loop, planner/coder/reviewer coordination, `agent_id`, `claim`, `complete_command`, moat/controller command, or orchestration-platform semantics.

### Task 1: Add bounded conservative media metadata adapter

**Files:**
- Create: `crates/mdid-adapters/src/conservative_media.rs`
- Modify: `crates/mdid-adapters/src/lib.rs`
- Test: `crates/mdid-adapters/tests/conservative_media_adapter.rs`

- [ ] **Step 1: Write the failing adapter tests**

Create `crates/mdid-adapters/tests/conservative_media_adapter.rs` with:

```rust
use mdid_adapters::{
    ConservativeMediaAdapter, ConservativeMediaAdapterError, ConservativeMediaInput,
    ConservativeMediaMetadataEntry,
};
use mdid_domain::{ConservativeMediaFormat, ConservativeMediaScanStatus};

fn metadata_entry(key: &str, value: &str) -> ConservativeMediaMetadataEntry {
    ConservativeMediaMetadataEntry {
        key: key.to_string(),
        value: value.to_string(),
    }
}

#[test]
fn image_metadata_extraction_routes_metadata_and_visual_review_honestly() {
    let input = ConservativeMediaInput {
        artifact_label: "patients/jane-face.png".to_string(),
        format: ConservativeMediaFormat::Image,
        metadata: vec![
            metadata_entry("EXIF Artist", "Jane Patient"),
            metadata_entry("CameraSerial", "SN-12345"),
        ],
        requires_visual_review: true,
        unsupported_payload: false,
    };

    let output = ConservativeMediaAdapter::extract_metadata(input).unwrap();

    assert_eq!(output.summary.total_items, 1);
    assert_eq!(output.summary.metadata_only_items, 0);
    assert_eq!(output.summary.visual_review_required_items, 1);
    assert_eq!(output.summary.unsupported_items, 0);
    assert_eq!(output.summary.review_required_candidates, 2);
    assert!(output.summary.requires_review());
    assert_eq!(output.candidates.len(), 2);
    assert_eq!(output.candidates[0].field_ref.field_path(), "media:patients_jane-face.png:EXIF Artist");
    assert_eq!(output.candidates[0].format, ConservativeMediaFormat::Image);
    assert_eq!(output.candidates[0].phi_type, "metadata_identifier");
    assert_eq!(output.candidates[0].source_value, "Jane Patient");
    assert_eq!(output.candidates[0].confidence, 0.35);
    assert_eq!(
        output.candidates[0].status,
        ConservativeMediaScanStatus::OcrOrVisualReviewRequired
    );
}

#[test]
fn fcs_metadata_extraction_stays_metadata_only_without_visual_claims() {
    let input = ConservativeMediaInput {
        artifact_label: "flow/panel.fcs".to_string(),
        format: ConservativeMediaFormat::Fcs,
        metadata: vec![metadata_entry("$FIL", "subject-42.fcs")],
        requires_visual_review: false,
        unsupported_payload: false,
    };

    let output = ConservativeMediaAdapter::extract_metadata(input).unwrap();

    assert_eq!(output.summary.total_items, 1);
    assert_eq!(output.summary.metadata_only_items, 1);
    assert_eq!(output.summary.visual_review_required_items, 0);
    assert_eq!(output.summary.unsupported_items, 0);
    assert_eq!(output.summary.review_required_candidates, 1);
    assert!(output.summary.requires_review());
    assert_eq!(output.candidates[0].format, ConservativeMediaFormat::Fcs);
    assert_eq!(output.candidates[0].status, ConservativeMediaScanStatus::MetadataOnly);
    assert_eq!(output.candidates[0].confidence, 0.35);
}

#[test]
fn unsupported_payload_counts_item_without_fabricating_candidates() {
    let input = ConservativeMediaInput {
        artifact_label: "video/unknown-container.bin".to_string(),
        format: ConservativeMediaFormat::Video,
        metadata: vec![metadata_entry("filename", "patient-walkthrough.mov")],
        requires_visual_review: true,
        unsupported_payload: true,
    };

    let output = ConservativeMediaAdapter::extract_metadata(input).unwrap();

    assert_eq!(output.summary.total_items, 1);
    assert_eq!(output.summary.metadata_only_items, 0);
    assert_eq!(output.summary.visual_review_required_items, 0);
    assert_eq!(output.summary.unsupported_items, 1);
    assert_eq!(output.summary.review_required_candidates, 0);
    assert!(!output.summary.requires_review());
    assert!(output.candidates.is_empty());
}

#[test]
fn extraction_output_debug_redacts_metadata_values() {
    let input = ConservativeMediaInput {
        artifact_label: "patient.png".to_string(),
        format: ConservativeMediaFormat::Image,
        metadata: vec![metadata_entry("Artist", "Jane Patient")],
        requires_visual_review: true,
        unsupported_payload: false,
    };

    let output = ConservativeMediaAdapter::extract_metadata(input).unwrap();
    let debug = format!("{output:?}");

    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("Jane Patient"));
}

#[test]
fn extraction_rejects_empty_artifact_label() {
    let input = ConservativeMediaInput {
        artifact_label: "   ".to_string(),
        format: ConservativeMediaFormat::Image,
        metadata: vec![metadata_entry("Artist", "Jane Patient")],
        requires_visual_review: true,
        unsupported_payload: false,
    };

    let err = ConservativeMediaAdapter::extract_metadata(input).unwrap_err();

    assert_eq!(err, ConservativeMediaAdapterError::EmptyArtifactLabel);
}
```

- [ ] **Step 2: Run tests to verify RED**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-adapters --test conservative_media_adapter -- --nocapture
```

Expected: FAIL to compile with unresolved imports for `ConservativeMediaAdapter`, `ConservativeMediaAdapterError`, `ConservativeMediaInput`, and `ConservativeMediaMetadataEntry`.

- [ ] **Step 3: Implement the minimal adapter**

Create `crates/mdid-adapters/src/conservative_media.rs` with:

```rust
use mdid_domain::{
    ConservativeMediaCandidate, ConservativeMediaFormat, ConservativeMediaRef,
    ConservativeMediaScanStatus, ConservativeMediaSummary,
};

const CONSERVATIVE_METADATA_CONFIDENCE: f32 = 0.35;
const METADATA_IDENTIFIER_PHI_TYPE: &str = "metadata_identifier";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConservativeMediaAdapterError {
    EmptyArtifactLabel,
}

impl std::fmt::Display for ConservativeMediaAdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyArtifactLabel => f.write_str("artifact label must not be empty"),
        }
    }
}

impl std::error::Error for ConservativeMediaAdapterError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConservativeMediaMetadataEntry {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConservativeMediaInput {
    pub artifact_label: String,
    pub format: ConservativeMediaFormat,
    pub metadata: Vec<ConservativeMediaMetadataEntry>,
    pub requires_visual_review: bool,
    pub unsupported_payload: bool,
}

#[derive(Clone, PartialEq)]
pub struct ExtractedConservativeMediaData {
    pub candidates: Vec<ConservativeMediaCandidate>,
    pub summary: ConservativeMediaSummary,
}

impl std::fmt::Debug for ExtractedConservativeMediaData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExtractedConservativeMediaData")
            .field("candidates", &self.candidates)
            .field("summary", &self.summary)
            .finish()
    }
}

pub struct ConservativeMediaAdapter;

impl ConservativeMediaAdapter {
    pub fn extract_metadata(
        input: ConservativeMediaInput,
    ) -> Result<ExtractedConservativeMediaData, ConservativeMediaAdapterError> {
        if input.artifact_label.trim().is_empty() {
            return Err(ConservativeMediaAdapterError::EmptyArtifactLabel);
        }

        let mut summary = ConservativeMediaSummary {
            total_items: 1,
            metadata_only_items: 0,
            visual_review_required_items: 0,
            unsupported_items: 0,
            review_required_candidates: 0,
        };

        if input.unsupported_payload {
            summary.unsupported_items = 1;
            return Ok(ExtractedConservativeMediaData {
                candidates: Vec::new(),
                summary,
            });
        }

        let status = if input.requires_visual_review {
            summary.visual_review_required_items = 1;
            ConservativeMediaScanStatus::OcrOrVisualReviewRequired
        } else {
            summary.metadata_only_items = 1;
            ConservativeMediaScanStatus::MetadataOnly
        };

        let candidates = input
            .metadata
            .into_iter()
            .filter(|entry| !entry.key.trim().is_empty() && !entry.value.trim().is_empty())
            .map(|entry| ConservativeMediaCandidate {
                field_ref: ConservativeMediaRef {
                    artifact_label: input.artifact_label.clone(),
                    metadata_key: entry.key,
                },
                format: input.format,
                phi_type: METADATA_IDENTIFIER_PHI_TYPE.to_string(),
                source_value: entry.value,
                confidence: CONSERVATIVE_METADATA_CONFIDENCE,
                status,
            })
            .collect::<Vec<_>>();

        summary.review_required_candidates = candidates.len();

        Ok(ExtractedConservativeMediaData { candidates, summary })
    }
}
```

Modify `crates/mdid-adapters/src/lib.rs` to include:

```rust
mod conservative_media;
pub mod dicom;
mod pdf;
mod tabular;

pub use conservative_media::{
    ConservativeMediaAdapter, ConservativeMediaAdapterError, ConservativeMediaInput,
    ConservativeMediaMetadataEntry, ExtractedConservativeMediaData,
};
pub use dicom::{
    sanitize_output_name, DicomAdapter, DicomAdapterError, DicomRewritePlan, DicomTagReplacement,
    DicomUidReplacement, DicomUidValue, ExtractedDicomData,
};
pub use pdf::{ExtractedPdfData, PdfAdapter, PdfAdapterError, PdfPageExtraction};
pub use tabular::{
    CsvTabularAdapter, ExtractedTabularData, FieldPolicy, FieldPolicyAction, TabularAdapterError,
    XlsxTabularAdapter,
};
```

- [ ] **Step 4: Run tests to verify GREEN**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-adapters --test conservative_media_adapter -- --nocapture
```

Expected: PASS, 5 tests pass.

- [ ] **Step 5: Run broader adapter verification**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-adapters
cargo clippy -p mdid-adapters --tests -- -D warnings
```

Expected: both commands PASS.

- [ ] **Step 6: Commit**

Run:

```bash
git add crates/mdid-adapters/src/conservative_media.rs crates/mdid-adapters/src/lib.rs crates/mdid-adapters/tests/conservative_media_adapter.rs
git commit -m "feat(adapters): add conservative media metadata adapter"
```

Expected: commit succeeds on `feature/conservative-media-adapter-foundation`.

### Task 2: Truth-sync README completion snapshot

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update README status text**

Modify `README.md` so the completion snapshot remains truthful and mentions the newly landed bounded adapter support:

- Keep CLI at `45%` unless separate CLI behavior landed.
- Keep Browser/web at `25%` unless separate browser behavior landed.
- Keep Desktop app at `10%` unless separate desktop behavior landed.
- Set Overall to `37%` only if Task 1 landed and adapter tests passed; otherwise leave Overall at `36%`.
- In `Implemented so far`, update the conservative media/FCS bullet to say: `Conservative media/FCS domain workflow models and a bounded adapter foundation now distinguish image/video/FCS metadata-only status, OCR-or-visual-review-required status, unsupported payloads, review-required metadata candidates, honest summary counts, and redacted candidate debug output; this does not implement OCR, visual redaction, FCS semantic parsing, rewrite/export, or runtime/browser/desktop flows.`
- In `Planned format support`, leave Image/Video/FCS as conservative L1/L2/L3 metadata-first support and do not claim full OCR/redaction/export.

- [ ] **Step 2: Verify README wording does not overclaim**

Run:

```bash
grep -n "Conservative media/FCS\|Overall\|OCR\|visual redaction\|FCS semantic" README.md
```

Expected: README says bounded adapter foundation only and explicitly says OCR, visual redaction, FCS semantic parsing, rewrite/export, and runtime/browser/desktop flows are not implemented.

- [ ] **Step 3: Run relevant tests again**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-adapters --test conservative_media_adapter
cargo test -p mdid-adapters
```

Expected: both commands PASS.

- [ ] **Step 4: Commit**

Run:

```bash
git add README.md
git commit -m "docs: update conservative media adapter completion snapshot"
```

Expected: commit succeeds on `feature/conservative-media-adapter-foundation`.

### Task 3: Merge verified slice back to develop

**Files:**
- No source changes; Git integration only.

- [ ] **Step 1: Verify feature branch is clean and tests pass**

Run:

```bash
git status --short
source "$HOME/.cargo/env"
cargo test -p mdid-adapters --test conservative_media_adapter
cargo test -p mdid-adapters
cargo clippy -p mdid-adapters --tests -- -D warnings
```

Expected: clean status and all commands PASS.

- [ ] **Step 2: Merge into develop**

Run:

```bash
git checkout develop
git pull --ff-only origin develop
git merge --no-ff feature/conservative-media-adapter-foundation -m "merge: add conservative media adapter foundation"
```

Expected: merge commit succeeds on `develop`.

- [ ] **Step 3: Final controller verification**

Run:

```bash
git branch --show-current
git status --short
git log --oneline -8 --decorate
source "$HOME/.cargo/env"
cargo test -p mdid-adapters --test conservative_media_adapter
cargo test -p mdid-adapters
```

Expected: on `develop`, clean status, recent merge commit visible, and tests PASS.

## Self-Review

1. Spec coverage: This plan covers a bounded Slice 6 adapter foundation for metadata-first image/video/FCS governance and explicitly avoids OCR, visual redaction, FCS semantic parsing, rewrite/export, and surface/runtime claims.
2. Placeholder scan: No TBD/TODO/implement-later placeholders remain. Every code-changing step includes exact code or exact README wording requirements.
3. Type consistency: The types `ConservativeMediaAdapter`, `ConservativeMediaInput`, `ConservativeMediaMetadataEntry`, `ConservativeMediaAdapterError`, and `ExtractedConservativeMediaData` are defined in Task 1 and exported before use by tests.
