# Conservative Media Application Service Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded application-layer service that routes conservative image/video/FCS metadata extraction into review-only workflow output without claiming OCR, visual redaction, FCS semantic parsing, rewrite/export, or runtime/browser/desktop support.

**Architecture:** Keep the adapter as the only extraction component and add a thin `mdid-application` orchestration wrapper mirroring the existing PDF service pattern. The service returns adapter summaries and review queues only, never vault-encodes or rewrites media payloads, because current conservative media support is explicitly metadata/review foundation.

**Tech Stack:** Rust workspace, `mdid-domain`, `mdid-adapters`, `mdid-application`, Cargo integration tests, strict TDD.

---

## File Structure

- Modify: `crates/mdid-application/src/lib.rs`
  - Import conservative media adapter/domain types.
  - Add `ApplicationError::ConservativeMediaAdapter`.
  - Add `ConservativeMediaDeidentificationOutput` with redacted `Debug`.
  - Add `ConservativeMediaDeidentificationService::deidentify_metadata(...)` that delegates to `ConservativeMediaAdapter::extract_metadata(...)` and returns summary + review queue + `rewritten_media_bytes: None`.
- Create: `crates/mdid-application/tests/conservative_media_deidentification.rs`
  - Integration tests for review-only metadata routing, unsupported payload honesty, adapter error propagation, and output debug redaction.
- Modify: `README.md`
  - Truth-sync completion snapshot after the service lands; completion percentages may remain unchanged if the bounded service does not materially change surface readiness.

### Task 1: Conservative Media Application Service

**Files:**
- Modify: `crates/mdid-application/src/lib.rs`
- Create: `crates/mdid-application/tests/conservative_media_deidentification.rs`
- Modify: `README.md`

- [ ] **Step 1: Write failing tests**

Create `crates/mdid-application/tests/conservative_media_deidentification.rs` with this complete content:

```rust
use mdid_adapters::{ConservativeMediaInput, ConservativeMediaMetadataEntry};
use mdid_application::{ApplicationError, ConservativeMediaDeidentificationService};
use mdid_domain::{ConservativeMediaFormat, ConservativeMediaScanStatus};

fn sample_input() -> ConservativeMediaInput {
    ConservativeMediaInput {
        artifact_label: "patient-jane-face.jpg".to_string(),
        format: ConservativeMediaFormat::Image,
        metadata: vec![ConservativeMediaMetadataEntry {
            key: "CameraOwner".to_string(),
            value: "Jane Patient".to_string(),
        }],
        requires_visual_review: true,
        unsupported_payload: false,
    }
}

#[test]
fn conservative_media_deidentification_routes_metadata_candidates_to_review_without_rewrite() {
    let output = ConservativeMediaDeidentificationService::default()
        .deidentify_metadata(sample_input())
        .expect("metadata extraction should succeed");

    assert_eq!(output.summary.total_items, 1);
    assert_eq!(output.summary.visual_review_required_items, 1);
    assert_eq!(output.summary.metadata_only_items, 0);
    assert_eq!(output.summary.unsupported_items, 0);
    assert_eq!(output.summary.review_required_candidates, 1);
    assert!(output.summary.requires_review());
    assert_eq!(output.review_queue.len(), 1);
    assert_eq!(output.review_queue[0].status, ConservativeMediaScanStatus::OcrOrVisualReviewRequired);
    assert_eq!(output.review_queue[0].phi_type, "metadata_identifier");
    assert_eq!(output.review_queue[0].source_value, "Jane Patient");
    assert!(output.rewritten_media_bytes.is_none());
}

#[test]
fn conservative_media_deidentification_reports_unsupported_payload_without_fabricating_candidates() {
    let mut input = sample_input();
    input.unsupported_payload = true;

    let output = ConservativeMediaDeidentificationService::default()
        .deidentify_metadata(input)
        .expect("unsupported payload should still produce honest summary");

    assert_eq!(output.summary.total_items, 1);
    assert_eq!(output.summary.unsupported_items, 1);
    assert_eq!(output.summary.review_required_candidates, 0);
    assert!(!output.summary.requires_review());
    assert!(output.review_queue.is_empty());
    assert!(output.rewritten_media_bytes.is_none());
}

#[test]
fn conservative_media_deidentification_surfaces_adapter_errors() {
    let mut input = sample_input();
    input.artifact_label = "   ".to_string();

    let error = ConservativeMediaDeidentificationService::default()
        .deidentify_metadata(input)
        .expect_err("blank artifact labels must be rejected by the adapter");

    assert!(matches!(error, ApplicationError::ConservativeMediaAdapter(_)));
}

#[test]
fn conservative_media_deidentification_output_debug_redacts_phi() {
    let output = ConservativeMediaDeidentificationService::default()
        .deidentify_metadata(sample_input())
        .expect("metadata extraction should succeed");

    let debug = format!("{output:?}");

    assert!(debug.contains("ConservativeMediaDeidentificationOutput"));
    assert!(debug.contains("review_queue_len"));
    assert!(debug.contains("rewritten_media_bytes"));
    assert!(!debug.contains("patient-jane-face.jpg"));
    assert!(!debug.contains("CameraOwner"));
    assert!(!debug.contains("Jane Patient"));
}
```

- [ ] **Step 2: Run targeted tests to verify RED**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-application --test conservative_media_deidentification -- --nocapture
```

Expected: FAIL to compile because `ConservativeMediaDeidentificationService`, `ConservativeMediaDeidentificationOutput`, and `ApplicationError::ConservativeMediaAdapter` do not exist yet.

- [ ] **Step 3: Write minimal implementation**

Modify `crates/mdid-application/src/lib.rs` as follows:

1. Extend the `use mdid_adapters::{ ... }` import list to include:

```rust
ConservativeMediaAdapter, ConservativeMediaAdapterError, ConservativeMediaInput,
```

2. Extend the `use mdid_domain::{ ... }` import list to include:

```rust
ConservativeMediaCandidate, ConservativeMediaSummary,
```

3. Add this variant to `ApplicationError` after the PDF adapter variant:

```rust
    #[error(transparent)]
    ConservativeMediaAdapter(#[from] ConservativeMediaAdapterError),
```

4. Add this output type near the other de-identification output structs:

```rust
#[derive(Clone)]
pub struct ConservativeMediaDeidentificationOutput {
    pub summary: ConservativeMediaSummary,
    pub review_queue: Vec<ConservativeMediaCandidate>,
    pub rewritten_media_bytes: Option<Vec<u8>>,
}

impl fmt::Debug for ConservativeMediaDeidentificationOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConservativeMediaDeidentificationOutput")
            .field("summary", &self.summary)
            .field("review_queue", &"[REDACTED]")
            .field("review_queue_len", &self.review_queue.len())
            .field(
                "rewritten_media_bytes",
                &self.rewritten_media_bytes.as_ref().map(|_| "[REDACTED]"),
            )
            .finish()
    }
}
```

5. Add this service declaration near the existing service declarations:

```rust
#[derive(Clone, Default)]
pub struct ConservativeMediaDeidentificationService;
```

6. Add this implementation before `impl PdfDeidentificationService`:

```rust
impl ConservativeMediaDeidentificationService {
    pub fn deidentify_metadata(
        &self,
        input: ConservativeMediaInput,
    ) -> Result<ConservativeMediaDeidentificationOutput, ApplicationError> {
        let extracted = ConservativeMediaAdapter::extract_metadata(input)?;

        Ok(ConservativeMediaDeidentificationOutput {
            summary: extracted.summary,
            review_queue: extracted.candidates,
            rewritten_media_bytes: None,
        })
    }
}
```

- [ ] **Step 4: Run targeted tests to verify GREEN**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-application --test conservative_media_deidentification -- --nocapture
```

Expected: PASS, all 4 conservative media application tests pass.

- [ ] **Step 5: Run relevant broader verification**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-application
cargo clippy -p mdid-application --tests -- -D warnings
```

Expected: PASS; no clippy warnings.

- [ ] **Step 6: README truth-sync**

Update `README.md` completion snapshot and implemented-so-far wording. If percentages remain unchanged, keep the numbers but add/update wording so the landed application service is represented honestly. Do not claim runtime, browser, desktop, OCR, visual redaction, semantic FCS parsing, or rewrite/export support.

Use this exact replacement for the conservative media bullet if still applicable:

```markdown
- Conservative media/FCS domain workflow models, bounded adapter foundation, and application-layer review routing now distinguish image/video/FCS metadata-only status, OCR-or-visual-review-required status, unsupported payloads, review-required metadata candidates, honest summary counts, and redacted candidate/reference/output debug; this does not implement OCR, visual redaction, FCS semantic parsing, rewrite/export, or runtime/browser/desktop flows
```

- [ ] **Step 7: Commit**

Run:

```bash
git add crates/mdid-application/src/lib.rs crates/mdid-application/tests/conservative_media_deidentification.rs README.md docs/superpowers/plans/2026-04-28-conservative-media-application-service.md
git commit -m "feat(application): add conservative media review service"
```

Expected: commit created on `feature/conservative-media-application-service`.

## Self-Review

- Spec coverage: This plan covers the bounded application-layer service only; it explicitly avoids runtime/browser/desktop/media rewrite/OCR/FCS semantic parsing.
- Placeholder scan: No TBD/TODO/fill-in placeholders are present.
- Type consistency: Service, output, error variant, and test names match across the plan.
