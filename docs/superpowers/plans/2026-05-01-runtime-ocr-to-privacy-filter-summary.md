# Runtime OCR-to-Privacy-Filter Summary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded runtime endpoint that proves existing PP-OCRv5 mobile OCR handoff JSON can feed the local text-only Privacy Filter summary path without returning raw OCR text or PHI-bearing fields.

**Architecture:** Extend `mdid-runtime` with `POST /ocr-to-privacy-filter/summary`, accepting `{ "handoff": <ocr-handoff-json> }`, validating the existing OCR handoff contract, extracting only `normalized_text` internally, building the existing deterministic runtime Privacy Filter text report, and returning the existing PHI-safe `privacy_filter_summary` response. The endpoint is runtime-only OCR-to-text-PII handoff evidence; it does not execute OCR, call a network API, perform visual redaction, rewrite PDFs, or add Browser/Desktop execution.

**Tech Stack:** Rust, Axum, serde/serde_json, tower `oneshot` HTTP tests, existing `mdid-runtime/src/http.rs` helper patterns.

---

## File Structure

- Modify: `crates/mdid-runtime/src/http.rs`
  - Add `OcrToPrivacyFilterSummaryRequest` request struct with `#[serde(deny_unknown_fields)]`.
  - Add `POST /ocr-to-privacy-filter/summary` route.
  - Add handler that validates the OCR handoff with existing handoff summary logic, extracts `normalized_text` only after validation, reuses `build_runtime_privacy_filter_text_report()` and `build_privacy_filter_summary()`.
  - Add PHI-safe tests for success, unknown fields, invalid handoff, empty normalized text, OCR/visual/PDF marker rejection, and raw PHI omission from response.
- Modify: `README.md`
  - Truth-sync runtime evidence and completion fraction accounting after the endpoint lands.

### Task 1: Runtime OCR-to-Privacy-Filter Summary Endpoint

**Files:**
- Modify: `crates/mdid-runtime/src/http.rs`
- Test: `crates/mdid-runtime/src/http.rs` test module

- [x] **Step 1: Write the failing route success test**

Add this test inside the existing `#[cfg(test)] mod tests` in `crates/mdid-runtime/src/http.rs`:

```rust
    #[tokio::test]
    async fn ocr_to_privacy_filter_summary_accepts_fixture_handoff_without_raw_phi() {
        let handoff: Value = serde_json::from_str(include_str!(
            "../../../scripts/ocr_eval/fixtures/ocr_handoff_expected_shape.json"
        ))
        .expect("fixture should be valid JSON");
        let response = build_default_router()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/ocr-to-privacy-filter/summary")
                    .header("content-type", "application/json")
                    .body(Body::from(json!({ "handoff": handoff }).to_string()))
                    .unwrap(),
            )
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let body_text = String::from_utf8(body.to_vec()).expect("response should be utf8");
        assert!(!body_text.contains("Jane Example"));
        assert!(!body_text.contains("MRN-12345"));
        assert!(!body_text.contains("jane@example.com"));
        assert!(!body_text.contains("555-123-4567"));
        let value: Value = serde_json::from_str(&body_text).expect("response should be json");
        assert_eq!(value["artifact"], "privacy_filter_summary");
        assert_eq!(value["network_api_called"], false);
        assert_eq!(value["category_counts"]["NAME"], 1);
        assert_eq!(value["category_counts"]["MRN"], 1);
        assert_eq!(value["category_counts"]["EMAIL"], 1);
        assert_eq!(value["category_counts"]["PHONE"], 1);
    }
```

- [x] **Step 2: Run the success test to verify RED**

Run:

```bash
/home/azureuser/.cargo/bin/cargo test -p mdid-runtime ocr_to_privacy_filter_summary_accepts_fixture_handoff_without_raw_phi -- --nocapture
```

Expected: FAIL with 404 Not Found or missing route because `/ocr-to-privacy-filter/summary` is not implemented yet.

- [x] **Step 3: Implement the minimal route and handler**

In `crates/mdid-runtime/src/http.rs`, add the request struct near the existing request structs:

```rust
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OcrToPrivacyFilterSummaryRequest {
    handoff: Value,
}
```

Add the route in `build_router()`:

```rust
        .route("/ocr-to-privacy-filter/summary", post(ocr_to_privacy_filter_summary))
```

Add this handler near the existing OCR/Privacy Filter handlers:

```rust
async fn ocr_to_privacy_filter_summary(
    payload: Result<Json<OcrToPrivacyFilterSummaryRequest>, JsonRejection>,
) -> Response {
    let Json(payload) = match payload {
        Ok(payload) => payload,
        Err(_) => return invalid_ocr_to_privacy_filter_summary_request_response().into_response(),
    };

    if build_ocr_handoff_summary(&payload.handoff).is_none() {
        return invalid_ocr_to_privacy_filter_summary_request_response().into_response();
    }

    let normalized_text = match payload.handoff.get("normalized_text").and_then(Value::as_str) {
        Some(text) if !text.trim().is_empty() && text.len() <= PRIVACY_FILTER_TEXT_MAX_BYTES => text,
        _ => return invalid_ocr_to_privacy_filter_summary_request_response().into_response(),
    };

    let report = build_runtime_privacy_filter_text_report(normalized_text);
    match build_privacy_filter_summary(&report) {
        Some(summary) => (StatusCode::OK, Json(summary)).into_response(),
        None => internal_error_response().into_response(),
    }
}
```

Add the fixed PHI-safe error response near other error helpers:

```rust
fn invalid_ocr_to_privacy_filter_summary_request_response() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorEnvelope {
            error: ErrorBody {
                code: "invalid_ocr_to_privacy_filter_summary_request",
                message: "request body did not contain a valid OCR handoff report for text-only Privacy Filter summary",
            },
        }),
    )
}
```

- [x] **Step 4: Run the success test to verify GREEN**

Run:

```bash
/home/azureuser/.cargo/bin/cargo test -p mdid-runtime ocr_to_privacy_filter_summary_accepts_fixture_handoff_without_raw_phi -- --nocapture
```

Expected: PASS.

- [x] **Step 5: Write failing rejection tests**

Add these tests in the same test module:

```rust
    #[tokio::test]
    async fn ocr_to_privacy_filter_summary_rejects_unknown_request_fields_without_phi_echo() {
        let response = build_default_router()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/ocr-to-privacy-filter/summary")
                    .header("content-type", "application/json")
                    .body(Body::from(json!({
                        "handoff": {},
                        "Patient Jane Example": "MRN-12345"
                    }).to_string()))
                    .unwrap(),
            )
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_text = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_text.contains("invalid_ocr_to_privacy_filter_summary_request"));
        assert!(!body_text.contains("Patient Jane Example"));
        assert!(!body_text.contains("MRN-12345"));
    }

    #[tokio::test]
    async fn ocr_to_privacy_filter_summary_rejects_visual_redaction_marker_without_phi_echo() {
        let mut handoff: Value = serde_json::from_str(include_str!(
            "../../../scripts/ocr_eval/fixtures/ocr_handoff_expected_shape.json"
        ))
        .expect("fixture should be valid JSON");
        handoff["visual_redaction"] = json!({ "preview": "Patient Jane Example MRN-12345" });

        let body_text = post_ocr_to_privacy_filter_summary(handoff, StatusCode::BAD_REQUEST).await;
        assert!(body_text.contains("invalid_ocr_to_privacy_filter_summary_request"));
        assert!(!body_text.contains("Patient Jane Example"));
        assert!(!body_text.contains("MRN-12345"));
    }

    #[tokio::test]
    async fn ocr_to_privacy_filter_summary_rejects_empty_normalized_text() {
        let mut handoff: Value = serde_json::from_str(include_str!(
            "../../../scripts/ocr_eval/fixtures/ocr_handoff_expected_shape.json"
        ))
        .expect("fixture should be valid JSON");
        handoff["normalized_text"] = json!("   ");

        let body_text = post_ocr_to_privacy_filter_summary(handoff, StatusCode::BAD_REQUEST).await;
        assert!(body_text.contains("invalid_ocr_to_privacy_filter_summary_request"));
        assert!(!body_text.contains("Jane Example"));
    }
```

Also add this test helper near existing HTTP test helpers:

```rust
    async fn post_ocr_to_privacy_filter_summary(handoff: Value, expected_status: StatusCode) -> String {
        let response = build_default_router()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/ocr-to-privacy-filter/summary")
                    .header("content-type", "application/json")
                    .body(Body::from(json!({ "handoff": handoff }).to_string()))
                    .unwrap(),
            )
            .await
            .expect("router should respond");
        assert_eq!(response.status(), expected_status);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        String::from_utf8(body.to_vec()).unwrap()
    }
```

- [x] **Step 6: Run rejection tests to verify RED/GREEN as appropriate**

Run:

```bash
/home/azureuser/.cargo/bin/cargo test -p mdid-runtime ocr_to_privacy_filter_summary_ -- --nocapture
```

Expected: PASS after the route implementation from Step 3, because the handler uses `deny_unknown_fields`, existing OCR handoff validation, and empty-text rejection.

- [x] **Step 7: Run focused runtime tests**

Run:

```bash
/home/azureuser/.cargo/bin/cargo test -p mdid-runtime privacy_filter_text -- --nocapture
/home/azureuser/.cargo/bin/cargo test -p mdid-runtime ocr_handoff_summary -- --nocapture
/home/azureuser/.cargo/bin/cargo test -p mdid-runtime ocr_to_privacy_filter_summary -- --nocapture
```

Expected: PASS.

- [x] **Step 8: Commit runtime endpoint**

Run:

```bash
git add crates/mdid-runtime/src/http.rs docs/superpowers/plans/2026-05-01-runtime-ocr-to-privacy-filter-summary.md
git commit -m "feat(runtime): add OCR to Privacy Filter summary endpoint"
```

### Task 2: README Truth-Sync for Runtime OCR-to-Privacy-Filter Summary

**Files:**
- Modify: `README.md`
- Test: repository-visible verification commands from Task 1

- [ ] **Step 1: Update README completion evidence**

In `README.md`, add a verification paragraph near the other Privacy Filter/OCR runtime evidence:

```markdown
Verification evidence for the bounded runtime OCR-to-Privacy-Filter summary endpoint landed on this branch: `POST /ocr-to-privacy-filter/summary` accepts an existing PP-OCRv5 mobile OCR handoff JSON report wrapped as `{ "handoff": <handoff-output> }`, validates the existing printed-text extraction handoff contract, extracts `normalized_text` only internally, runs the deterministic local text-only Privacy Filter summary path, and returns only the PHI-safe `privacy_filter_summary` response. It does not execute OCR, call a network API, perform visual redaction, image pixel redaction, handwriting recognition, final PDF rewrite/export, Browser/Web execution, Desktop execution, or unrelated workflow orchestration semantics.
```

Update the completion snapshot text to mention the new endpoint. Use fraction accounting: this round adds one new CLI/runtime requirement and completes it in the same round, so CLI stays `95%` by conservative floor unless the repo already contains enough landed evidence to justify a higher numerator; Browser/Web and Desktop remain `99%`; Overall remains `97%` unless exact rubric fractions justify a conservative increase.

- [ ] **Step 2: Run docs/format verification**

Run:

```bash
git diff --check
/home/azureuser/.cargo/bin/cargo test -p mdid-runtime ocr_to_privacy_filter_summary -- --nocapture
```

Expected: PASS.

- [ ] **Step 3: Commit README truth-sync**

Run:

```bash
git add README.md
git commit -m "docs: truth-sync OCR to Privacy Filter runtime summary"
```

## Self-Review

- Spec coverage: Task 1 adds the runtime endpoint, validates existing OCR handoff input, reuses text-only Privacy Filter summary semantics, rejects unsafe markers, and omits raw OCR/PHI output. Task 2 updates repository-visible completion evidence.
- Placeholder scan: no TBD/TODO placeholders remain; all commands and code snippets are concrete.
- Type consistency: request type is `OcrToPrivacyFilterSummaryRequest`; route is `/ocr-to-privacy-filter/summary`; helper is `post_ocr_to_privacy_filter_summary`; error code is `invalid_ocr_to_privacy_filter_summary_request`.
