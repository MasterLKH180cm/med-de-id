# Conservative Media Runtime Entry Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded local HTTP runtime entry that routes conservative image/video/FCS metadata payloads to the existing application-layer review service without claiming OCR, visual redaction, rewrite/export, or workflow orchestration.

**Architecture:** Keep `mdid-runtime` as a thin HTTP surface: deserialize JSON, call `ConservativeMediaDeidentificationService::deidentify_metadata`, and serialize the existing summary/review queue plus `rewritten_media_bytes_base64: null`. Errors from blank artifact labels surface as typed `422 invalid_conservative_media_request`; unsupported payloads remain successful review summaries. README completion status is truth-synced only after controller-visible tests pass.

**Tech Stack:** Rust workspace, Axum runtime HTTP handlers, `mdid-application`, `mdid-adapters`, `mdid-domain`, serde, cargo test/clippy.

---

## File Structure

- Modify: `crates/mdid-runtime/src/http.rs`
  - Add request/response DTOs for conservative media metadata de-identification.
  - Add `POST /media/conservative/deidentify` route.
  - Handler delegates to `ConservativeMediaDeidentificationService::default().deidentify_metadata(...)`.
  - Add narrow error mapper for `ApplicationError::ConservativeMediaAdapter(_)` to `422 invalid_conservative_media_request`; all other unexpected errors remain `500 internal_error`.
- Modify: `crates/mdid-runtime/tests/runtime_http.rs`
  - Add endpoint tests for metadata review routing, unsupported payload honesty, and blank-label typed rejection.
- Modify: `README.md`
  - Update completion snapshot and implemented-so-far/runtime wording based only on landed behavior and passing tests.

## Scope Guard

This plan is intentionally **not** an agent/controller/moat feature. It must not add controller loop, planner/coder/reviewer coordination, `agent_id`, `claim`, `complete_command`, or `controller-step` behavior. It also must not add OCR, visual redaction, FCS semantic parsing, media rewrite/export, multipart upload, auth/session handling, or browser/desktop UI claims.

### Task 1: Runtime conservative media endpoint

**Files:**
- Modify: `crates/mdid-runtime/src/http.rs`
- Test: `crates/mdid-runtime/tests/runtime_http.rs`

- [ ] **Step 1: Write failing endpoint tests**

Append these tests to `crates/mdid-runtime/tests/runtime_http.rs` and add imports only if missing (`axum::body::Body`, `http::{Request, StatusCode}`, `tower::ServiceExt`, and `serde_json::json` already exist in this test file in current runtime tests; reuse existing helpers if present):

```rust
#[tokio::test]
async fn conservative_media_deidentify_endpoint_routes_metadata_to_review_without_rewrite() {
    let app = build_router(RuntimeState::default());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/media/conservative/deidentify")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "artifact_label": "patient-jane-fundus.jpg",
                        "format": "image",
                        "metadata": [
                            {"key": "PatientName", "value": "Jane Doe"},
                            {"key": "Device", "value": "Camera 1"}
                        ],
                        "ocr_or_visual_review_required": true
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["rewritten_media_bytes_base64"], serde_json::Value::Null);
    assert_eq!(json["summary"]["total_items"], 1);
    assert_eq!(json["summary"]["review_required"], 1);
    assert_eq!(json["summary"]["unsupported_items"], 0);
    assert_eq!(json["review_queue"].as_array().unwrap().len(), 2);
    assert_eq!(json["review_queue"][0]["format"], "image");
    assert_eq!(json["review_queue"][0]["status"], "ocr_or_visual_review_required");
}

#[tokio::test]
async fn conservative_media_deidentify_endpoint_reports_unsupported_payload_without_candidates() {
    let app = build_router(RuntimeState::default());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/media/conservative/deidentify")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "artifact_label": "unknown-media.bin",
                        "format": "unsupported",
                        "metadata": [],
                        "ocr_or_visual_review_required": false
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["rewritten_media_bytes_base64"], serde_json::Value::Null);
    assert_eq!(json["summary"]["total_items"], 1);
    assert_eq!(json["summary"]["review_required"], 0);
    assert_eq!(json["summary"]["unsupported_items"], 1);
    assert_eq!(json["review_queue"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn conservative_media_deidentify_endpoint_rejects_blank_artifact_label_as_invalid_request() {
    let app = build_router(RuntimeState::default());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/media/conservative/deidentify")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "artifact_label": "   ",
                        "format": "fcs",
                        "metadata": [{"key": "PATIENT ID", "value": "ABC-123"}],
                        "ocr_or_visual_review_required": false
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["error"]["code"], "invalid_conservative_media_request");
}
```

- [ ] **Step 2: Run tests to verify RED**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-runtime --test runtime_http conservative_media_deidentify_endpoint -- --nocapture
```

Expected: FAIL because `/media/conservative/deidentify` does not exist yet and responses are not `200`/`422` as asserted.

- [ ] **Step 3: Implement minimal runtime route**

In `crates/mdid-runtime/src/http.rs`, make these focused changes:

```rust
use mdid_adapters::{
    ConservativeMediaInput, ConservativeMediaMetadataEntry, CsvTabularAdapter, DicomAdapterError,
    FieldPolicy, FieldPolicyAction, XlsxTabularAdapter,
};
use mdid_application::{
    ApplicationError, ApplicationService, ConservativeMediaDeidentificationOutput,
    ConservativeMediaDeidentificationService, DicomDeidentificationOutput,
    DicomDeidentificationService, TabularDeidentificationOutput, TabularDeidentificationService,
};
use mdid_domain::{
    AuditEvent, AuditEventKind, BatchSummary, ConservativeMediaCandidate, ConservativeMediaFormat,
    ConservativeMediaSummary, DecodeRequest, DicomDeidentificationSummary, DicomPhiCandidate,
    DicomPrivateTagPolicy, MappingRecord, MappingScope, PhiCandidate, SurfaceKind,
};
```

Add request and response structs near existing request/response DTOs:

```rust
#[derive(Debug, Deserialize)]
struct ConservativeMediaDeidentifyRequest {
    artifact_label: String,
    format: ConservativeMediaFormat,
    metadata: Vec<ConservativeMediaMetadataEntryRequest>,
    #[serde(default)]
    ocr_or_visual_review_required: bool,
}

#[derive(Debug, Deserialize)]
struct ConservativeMediaMetadataEntryRequest {
    key: String,
    value: String,
}

#[derive(Debug, Serialize)]
struct ConservativeMediaDeidentifyResponse {
    summary: ConservativeMediaSummary,
    review_queue: Vec<ConservativeMediaCandidate>,
    rewritten_media_bytes_base64: Option<String>,
}
```

Add the route:

```rust
.route(
    "/media/conservative/deidentify",
    post(conservative_media_deidentify),
)
```

Add the handler:

```rust
async fn conservative_media_deidentify(
    payload: Result<Json<ConservativeMediaDeidentifyRequest>, JsonRejection>,
) -> Response {
    let Json(payload) = match payload {
        Ok(payload) => payload,
        Err(_) => return invalid_conservative_media_request_response().into_response(),
    };

    let input = ConservativeMediaInput {
        artifact_label: payload.artifact_label,
        format: payload.format,
        metadata: payload
            .metadata
            .into_iter()
            .map(|entry| ConservativeMediaMetadataEntry {
                key: entry.key,
                value: entry.value,
            })
            .collect(),
        ocr_or_visual_review_required: payload.ocr_or_visual_review_required,
    };

    let output = match ConservativeMediaDeidentificationService::default().deidentify_metadata(input) {
        Ok(output) => output,
        Err(error) => return map_conservative_media_application_error(&error).into_response(),
    };

    conservative_media_success_response(output).into_response()
}
```

Add response helpers near existing helpers:

```rust
fn conservative_media_success_response(
    output: ConservativeMediaDeidentificationOutput,
) -> (StatusCode, Json<ConservativeMediaDeidentifyResponse>) {
    (
        StatusCode::OK,
        Json(ConservativeMediaDeidentifyResponse {
            summary: output.summary,
            review_queue: output.review_queue,
            rewritten_media_bytes_base64: None,
        }),
    )
}

fn map_conservative_media_application_error(
    error: &ApplicationError,
) -> (StatusCode, Json<ErrorEnvelope>) {
    match error {
        ApplicationError::ConservativeMediaAdapter(_) => {
            invalid_conservative_media_request_response()
        }
        _ => internal_error_response(),
    }
}

fn invalid_conservative_media_request_response() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(ErrorEnvelope {
            error: ErrorBody {
                code: "invalid_conservative_media_request",
                message: "request body did not contain a valid conservative media deidentification request",
            },
        }),
    )
}
```

- [ ] **Step 4: Run targeted tests to verify GREEN**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-runtime --test runtime_http conservative_media_deidentify_endpoint -- --nocapture
```

Expected: PASS for the three conservative media endpoint tests.

- [ ] **Step 5: Run broader runtime verification**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-runtime
cargo clippy -p mdid-runtime --tests -- -D warnings
```

Expected: PASS with no clippy warnings.

- [ ] **Step 6: Commit runtime endpoint**

Run:

```bash
git add crates/mdid-runtime/src/http.rs crates/mdid-runtime/tests/runtime_http.rs
git commit -m "feat(runtime): add conservative media review endpoint"
```

### Task 2: README completion truth-sync

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Re-read controller-visible landed behavior**

Run:

```bash
git branch --show-current
git status --short
git log --oneline -8 --decorate
source "$HOME/.cargo/env"
cargo test -p mdid-runtime --test runtime_http conservative_media_deidentify_endpoint -- --nocapture
```

Expected: on `feature/conservative-media-runtime-entry`, runtime endpoint tests pass.

- [ ] **Step 2: Update README completion snapshot**

Edit `README.md` so the completion table remains honest. Use these exact values unless additional landed functionality exists in the controller-visible branch:

```markdown
| CLI | 45% | Early automation surface with bounded local history-file inspection/handoff commands plus existing local workflow entry points; not a complete automation product |
| Browser/web | 25% | Bounded localhost tabular de-identification page backed by local runtime routes; not a broader browser governance workspace |
| Desktop app | 10% | Early scaffold only; sensitive-workstation review, vault, decode, and audit flows remain mostly unimplemented |
| Overall | 38% | Core workspace, vault MVP, tabular path, bounded DICOM/PDF/runtime slices, conservative media/FCS domain models, adapter/application foundations, a bounded runtime media review entry, browser tabular surface, and bounded CLI slices are present, but major workflow depth and surface parity remain missing |
```

Add or update the implemented-so-far bullets to include this narrow runtime wording:

```markdown
- `mdid-runtime` also exposes a bounded local HTTP conservative media review entry that accepts image/video/FCS metadata JSON, routes it through the existing application review service, returns honest summary/review queue data, and always reports `rewritten_media_bytes_base64: null`; it does not implement OCR, visual redaction, FCS semantic parsing, media rewrite/export, multipart upload, browser/desktop flows, auth/session handling, or generalized media workflow orchestration
```

Update the runtime HTTP slice paragraph so it includes conservative media metadata review in the list and repeats the same limitations.

- [ ] **Step 3: Verify README wording does not overclaim**

Run:

```bash
grep -n "conservative media\|Overall\|rewritten_media_bytes_base64\|OCR\|visual redaction" README.md
```

Expected: README says bounded review entry only, `rewritten_media_bytes_base64: null`, and explicitly does not claim OCR/visual redaction/rewrite/export.

- [ ] **Step 4: Re-run relevant tests after docs change**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-runtime --test runtime_http conservative_media_deidentify_endpoint -- --nocapture
cargo test -p mdid-runtime
```

Expected: PASS.

- [ ] **Step 5: Commit README truth-sync**

Run:

```bash
git add README.md
git commit -m "docs: update conservative media runtime status"
```

### Task 3: Merge verified feature to develop

**Files:**
- No code changes expected; GitFlow integration only.

- [ ] **Step 1: Final feature branch verification**

Run:

```bash
git branch --show-current
git status --short
git log --oneline -8 --decorate
source "$HOME/.cargo/env"
cargo test -p mdid-runtime --test runtime_http conservative_media_deidentify_endpoint -- --nocapture
cargo test -p mdid-runtime
cargo clippy -p mdid-runtime --tests -- -D warnings
```

Expected: clean worktree and all commands PASS.

- [ ] **Step 2: Merge to develop**

Run:

```bash
git checkout develop
git pull --ff-only origin develop
git merge --no-ff feature/conservative-media-runtime-entry -m "merge: add conservative media runtime entry"
```

Expected: merge commit created on `develop`.

- [ ] **Step 3: Verify develop after merge**

Run:

```bash
git branch --show-current
git status --short
git log --oneline -8 --decorate
source "$HOME/.cargo/env"
cargo test -p mdid-runtime --test runtime_http conservative_media_deidentify_endpoint -- --nocapture
cargo test -p mdid-runtime
```

Expected: on `develop`, clean worktree and tests PASS.

- [ ] **Step 4: Push develop if authenticated**

Run:

```bash
git push origin develop
```

Expected: `origin/develop` advances to the verified merge commit. If push is unavailable, report the local merge and push failure honestly.

---

## Self-Review

- Spec coverage: The plan adds only a bounded runtime entry for already-landed conservative media application behavior, includes tests for review routing, unsupported payload honesty, blank-label typed errors, README truth-sync, and GitFlow merge.
- Placeholder scan: No TBD/TODO/implement-later placeholders are present.
- Type consistency: Request/response names and fields match the existing application/domain/adapters types and the test JSON wire values.
