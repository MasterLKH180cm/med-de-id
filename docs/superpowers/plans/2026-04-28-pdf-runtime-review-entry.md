# PDF Runtime Review Entry Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded local HTTP PDF de-identification review entry that delegates to the existing application PDF service and truthfully reports text-layer/ocr-needed review status without claiming rewrite/export.

**Architecture:** `mdid-runtime` gets one thin `POST /pdf/deidentify` route. The route accepts JSON with base64 PDF bytes plus a source name, decodes the transport payload, delegates to `mdid_application::PdfDeidentificationService`, and serializes the existing summary/page statuses/review queue with `rewritten_pdf_bytes_base64: null`.

**Tech Stack:** Rust workspace, Axum runtime crate, `base64`, `serde`, `mdid-application`, `mdid-domain`, Rust integration tests under `crates/mdid-runtime/tests/runtime_http.rs`.

---

## File Structure

- Modify `crates/mdid-runtime/src/http.rs`
  - Import `PdfDeidentificationService`, `PdfExtractionSummary`, `PdfPageExtraction`, and `PdfPhiCandidate` if not already imported.
  - Add private request DTO `PdfDeidentifyRequest` with `pdf_bytes_base64: String` and `source_name: String`; do not derive `Debug` because the source name may carry PHI.
  - Add response DTO `PdfDeidentifyResponse` with `summary`, `page_statuses`, `review_queue`, and `rewritten_pdf_bytes_base64: Option<String>`.
  - Register `POST /pdf/deidentify` in `app()`.
  - Implement `pdf_deidentify(...)`, `pdf_success_response(...)`, and `invalid_pdf_response()`.
  - Map malformed JSON, malformed base64, and `ApplicationError::PdfAdapter(PdfAdapterError::Parse(_))` to `422 invalid_pdf`; keep unexpected errors as `500 internal_error`.
- Modify `crates/mdid-runtime/tests/runtime_http.rs`
  - Add tests for text-layer PDF review routing, scan-only/OCR-required honest status, invalid PDF bytes, and malformed base64.
- Modify `README.md`
  - Reassess CLI, browser/web, desktop, and overall completion.
  - Mention the bounded PDF runtime review entry narrowly.
  - Keep missing items honest: no OCR, no visual redaction, no PDF rewrite/export, no browser/desktop PDF flow.

---

### Task 1: Add bounded PDF runtime review route

**Files:**
- Modify: `crates/mdid-runtime/src/http.rs`
- Test: `crates/mdid-runtime/tests/runtime_http.rs`

- [ ] **Step 1: Write failing runtime integration tests**

Add this code to `crates/mdid-runtime/tests/runtime_http.rs` near the other endpoint tests:

```rust
const TEXT_LAYER_PDF: &[u8] =
    include_bytes!("../../mdid-adapters/tests/fixtures/pdf/text-layer-minimal.pdf");
const NO_TEXT_PDF: &[u8] =
    include_bytes!("../../mdid-adapters/tests/fixtures/pdf/no-text-minimal.pdf");

#[tokio::test]
async fn pdf_deidentify_endpoint_routes_text_layer_candidates_to_review_without_rewrite() {
    let app = app();
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/pdf/deidentify")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::json!({
                    "pdf_bytes_base64": STANDARD.encode(TEXT_LAYER_PDF),
                    "source_name": "alice-smith-record.pdf"
                }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_body_json(response).await;
    assert_eq!(body["summary"]["total_pages"], 1);
    assert_eq!(body["summary"]["text_layer_pages"], 1);
    assert_eq!(body["summary"]["ocr_required_pages"], 0);
    assert_eq!(body["summary"]["extracted_candidates"], 1);
    assert_eq!(body["summary"]["review_required_candidates"], 1);
    assert_eq!(body["page_statuses"][0]["status"], "text_layer_present");
    assert_eq!(body["review_queue"].as_array().unwrap().len(), 1);
    assert_eq!(body["rewritten_pdf_bytes_base64"], serde_json::Value::Null);
}

#[tokio::test]
async fn pdf_deidentify_endpoint_reports_ocr_required_without_fabricating_candidates() {
    let app = app();
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/pdf/deidentify")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::json!({
                    "pdf_bytes_base64": STANDARD.encode(NO_TEXT_PDF),
                    "source_name": "scan-only.pdf"
                }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_body_json(response).await;
    assert_eq!(body["summary"]["total_pages"], 1);
    assert_eq!(body["summary"]["text_layer_pages"], 0);
    assert_eq!(body["summary"]["ocr_required_pages"], 1);
    assert_eq!(body["summary"]["extracted_candidates"], 0);
    assert_eq!(body["summary"]["review_required_candidates"], 0);
    assert_eq!(body["page_statuses"][0]["status"], "ocr_required");
    assert_eq!(body["review_queue"].as_array().unwrap().len(), 0);
    assert_eq!(body["rewritten_pdf_bytes_base64"], serde_json::Value::Null);
}

#[tokio::test]
async fn pdf_deidentify_endpoint_rejects_invalid_pdf_bytes() {
    let app = app();
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/pdf/deidentify")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::json!({
                    "pdf_bytes_base64": STANDARD.encode(b"not a pdf"),
                    "source_name": "broken.pdf"
                }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = response_body_json(response).await;
    assert_eq!(body["error"]["code"], "invalid_pdf");
}

#[tokio::test]
async fn pdf_deidentify_endpoint_rejects_malformed_base64_payload() {
    let app = app();
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/pdf/deidentify")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::json!({
                    "pdf_bytes_base64": "not-base64%%%",
                    "source_name": "broken.pdf"
                }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = response_body_json(response).await;
    assert_eq!(body["error"]["code"], "invalid_pdf");
}
```

- [ ] **Step 2: Run tests to verify RED**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-runtime --test runtime_http pdf_deidentify_endpoint -- --nocapture
```

Expected: FAIL because `/pdf/deidentify` is not registered and returns 404 or because the PDF response fields do not exist yet.

- [ ] **Step 3: Implement the minimal runtime route**

In `crates/mdid-runtime/src/http.rs`:

1. Add `PdfDeidentificationService` and PDF domain types to existing imports.
2. Add private DTOs:

```rust
#[derive(Deserialize)]
struct PdfDeidentifyRequest {
    pdf_bytes_base64: String,
    source_name: String,
}

#[derive(Serialize)]
struct PdfDeidentifyResponse {
    summary: PdfExtractionSummary,
    page_statuses: Vec<PdfPageExtraction>,
    review_queue: Vec<PdfPhiCandidate>,
    rewritten_pdf_bytes_base64: Option<String>,
}
```

3. Register the route in `app()`:

```rust
.route("/pdf/deidentify", post(pdf_deidentify))
```

4. Add the handler and response helpers:

```rust
async fn pdf_deidentify(payload: Result<Json<PdfDeidentifyRequest>, JsonRejection>) -> Response {
    let Json(payload) = match payload {
        Ok(payload) => payload,
        Err(_) => return invalid_pdf_response().into_response(),
    };

    let pdf_bytes = match STANDARD.decode(&payload.pdf_bytes_base64) {
        Ok(bytes) => bytes,
        Err(_) => return invalid_pdf_response().into_response(),
    };

    let output = match PdfDeidentificationService
        .deidentify_bytes(&pdf_bytes, &payload.source_name)
    {
        Ok(output) => output,
        Err(ApplicationError::PdfAdapter(PdfAdapterError::Parse(_))) => {
            return invalid_pdf_response().into_response();
        }
        Err(_) => return internal_error_response().into_response(),
    };

    pdf_success_response(output).into_response()
}

fn pdf_success_response(output: PdfDeidentificationOutput) -> Json<PdfDeidentifyResponse> {
    Json(PdfDeidentifyResponse {
        summary: output.summary,
        page_statuses: output.page_statuses,
        review_queue: output.review_queue,
        rewritten_pdf_bytes_base64: None,
    })
}

fn invalid_pdf_response() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(ErrorEnvelope {
            error: ErrorDetail {
                code: "invalid_pdf",
                message: "request body did not contain a valid PDF deidentification request",
            },
        }),
    )
}
```

Adjust placement/names only as needed to match existing runtime helper style. Do not derive `Debug` for `PdfDeidentifyRequest`.

- [ ] **Step 4: Run tests to verify GREEN**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-runtime --test runtime_http pdf_deidentify_endpoint -- --nocapture
cargo test -p mdid-runtime --test runtime_http
cargo test -p mdid-runtime
```

Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-runtime/src/http.rs crates/mdid-runtime/tests/runtime_http.rs
git commit -m "feat(runtime): add pdf review endpoint"
```

---

### Task 2: Truth-sync README completion and PDF runtime wording

**Files:**
- Modify: `README.md`

- [x] **Step 1: Update README after landed runtime behavior**

Modify `README.md` completion/status wording so it states:

- CLI remains `42%` unless this task also changes CLI behavior.
- Browser/web remains `25%` unless this task also changes browser behavior.
- Desktop app remains `10%` unless this task also changes desktop behavior.
- Overall may increase from `37%` to `38%` because a new runtime PDF review entry landed, but must still state that full OCR, visual redaction, and PDF rewrite/export remain missing.
- Add the bounded `/pdf/deidentify` entry to the runtime bullets as a local HTTP PDF review entry accepting base64 PDF bytes, reporting text-layer/OCR-required summary/page statuses/review queue, and returning `rewritten_pdf_bytes_base64: null`.
- Add missing-item wording that browser/desktop PDF flows are still absent.

- [ ] **Step 2: Verify documentation truthfulness**

Run:

```bash
git diff -- README.md
git diff --check
```

Expected: README diff mentions PDF runtime narrowly and `git diff --check` exits 0.

- [ ] **Step 3: Commit**

```bash
git add README.md docs/superpowers/plans/2026-04-28-pdf-runtime-review-entry.md
git commit -m "docs: update pdf runtime completion snapshot"
```

---

## Self-Review

- Spec coverage: plan covers runtime route, typed invalid-payload handling, no rewrite/export claims, and README completion maintenance.
- Placeholder scan: no TBD/TODO/implement-later placeholders are present.
- Type consistency: route, DTO, and response names consistently use `PdfDeidentify*`; response field is consistently `rewritten_pdf_bytes_base64`.
