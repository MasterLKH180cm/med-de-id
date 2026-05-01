# Runtime OCR Handoff Summary Endpoint Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded runtime endpoint that converts an existing PP-OCRv5 mobile OCR handoff JSON report into a PHI-safe aggregate summary suitable for downstream text-only Privacy Filter evaluation evidence.

**Architecture:** Reuse the existing `crates/mdid-runtime/src/http.rs` Axum router and the current OCR handoff contract from `scripts/ocr_eval/fixtures/ocr_handoff_expected_shape.json`. The endpoint only summarizes an existing OCR handoff report; it does not run OCR, call Privacy Filter, redact pixels, rewrite/export PDFs, or add workflow orchestration semantics.

**Tech Stack:** Rust, Axum, serde/serde_json, existing mdid-runtime test module, Cargo workspace.

---

## File Structure

- Modify: `crates/mdid-runtime/src/http.rs`
  - Add route `POST /ocr-handoff/summary`.
  - Add request/response structs for an existing handoff report wrapper.
  - Add sanitizer helpers that allowlist safe OCR handoff fields only.
  - Add tests in the existing `#[cfg(test)]` module.
- Modify: `README.md`
  - Truth-sync current repository status with the landed runtime OCR handoff summary endpoint.
  - Keep Browser/Web and Desktop app completion unchanged because this is runtime-only.
  - Keep the slice explicitly bounded to existing PP-OCRv5 mobile printed-text extraction handoff JSON summaries.

### Task 1: Runtime OCR Handoff Summary Endpoint

**Files:**
- Modify: `crates/mdid-runtime/src/http.rs`
- Test: `crates/mdid-runtime/src/http.rs` existing test module

- [ ] **Step 1: Write failing test for fixture-backed success**

Add this test to the existing `#[cfg(test)]` module in `crates/mdid-runtime/src/http.rs`:

```rust
#[tokio::test]
async fn ocr_handoff_summary_accepts_existing_fixture_contract() {
    let app = router();
    let handoff: serde_json::Value = serde_json::from_str(include_str!(
        "../../../scripts/ocr_eval/fixtures/ocr_handoff_expected_shape.json"
    ))
    .unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/ocr-handoff/summary")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({ "handoff": handoff }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let summary: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(summary["artifact"], "ocr_handoff_summary");
    assert_eq!(summary["candidate"], "PP-OCRv5_mobile_rec");
    assert_eq!(summary["engine"], "PP-OCRv5-mobile-bounded-spike");
    assert_eq!(summary["scope"], "printed_text_line_extraction_only");
    assert_eq!(summary["privacy_filter_contract"], "text_only_normalized_input");
    assert_eq!(summary["ready_for_text_pii_eval"], true);
    assert_eq!(summary["network_api_called"], false);
    assert!(summary["non_goals"].as_array().unwrap().contains(&serde_json::json!("visual_redaction")));
    let serialized = serde_json::to_string(&summary).unwrap();
    for forbidden in ["Jane Example", "MRN-12345", "jane@example.com", "555-123-4567", "extracted_text", "normalized_text", "bbox"] {
        assert!(!serialized.contains(forbidden), "summary leaked {forbidden}");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `/home/azureuser/.cargo/bin/cargo test -p mdid-runtime ocr_handoff_summary_accepts_existing_fixture_contract -- --nocapture`

Expected: FAIL because `/ocr-handoff/summary` is not routed or returns 404.

- [ ] **Step 3: Implement minimal route, request, response, and sanitizer**

In `crates/mdid-runtime/src/http.rs`:

1. Add route near existing Privacy Filter routes:

```rust
.route("/ocr-handoff/summary", post(ocr_handoff_summary))
```

2. Add structs near the Privacy Filter request/response structs:

```rust
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OcrHandoffSummaryRequest {
    handoff: Value,
}

#[derive(Debug, Serialize)]
struct OcrHandoffSummaryResponse {
    artifact: &'static str,
    candidate: String,
    engine: String,
    engine_status: String,
    scope: String,
    ready_for_text_pii_eval: bool,
    privacy_filter_contract: String,
    line_count: Option<u64>,
    char_count: Option<u64>,
    network_api_called: bool,
    non_goals: Vec<String>,
}
```

3. Add handler and helpers near `privacy_filter_summary`:

```rust
async fn ocr_handoff_summary(
    payload: Result<Json<OcrHandoffSummaryRequest>, JsonRejection>,
) -> Response {
    let Json(payload) = match payload {
        Ok(payload) => payload,
        Err(_) => return invalid_ocr_handoff_summary_request_response().into_response(),
    };

    match build_ocr_handoff_summary(&payload.handoff) {
        Some(summary) => (StatusCode::OK, Json(summary)).into_response(),
        None => invalid_ocr_handoff_summary_request_response().into_response(),
    }
}

fn invalid_ocr_handoff_summary_request_response() -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "invalid_ocr_handoff_summary_request",
            message: "OCR handoff summary request was invalid",
        }),
    )
}

fn build_ocr_handoff_summary(handoff: &Value) -> Option<OcrHandoffSummaryResponse> {
    let handoff = handoff.as_object()?;
    if contains_incompatible_ocr_handoff_marker(handoff) {
        return None;
    }
    let candidate = safe_identifier(handoff.get("candidate")?.as_str()?)?.to_owned();
    let engine = safe_identifier(handoff.get("engine")?.as_str()?)?.to_owned();
    let engine_status = safe_identifier(handoff.get("engine_status")?.as_str()?)?.to_owned();
    let scope = safe_ocr_handoff_scope(handoff.get("scope")?.as_str()?)?.to_owned();
    let ready_for_text_pii_eval = handoff.get("ready_for_text_pii_eval")?.as_bool()?;
    if !ready_for_text_pii_eval {
        return None;
    }
    let privacy_filter_contract = handoff.get("privacy_filter_contract")?.as_str()?;
    if privacy_filter_contract != "text_only_normalized_input" {
        return None;
    }
    let non_goals = sanitized_ocr_handoff_non_goals(handoff.get("non_goals"))?;
    Some(OcrHandoffSummaryResponse {
        artifact: "ocr_handoff_summary",
        candidate,
        engine,
        engine_status,
        scope,
        ready_for_text_pii_eval,
        privacy_filter_contract: privacy_filter_contract.to_owned(),
        line_count: optional_nonnegative_u64(handoff.get("line_count"))?,
        char_count: optional_nonnegative_u64(handoff.get("char_count"))?,
        network_api_called: false,
        non_goals,
    })
}

fn safe_ocr_handoff_scope(scope: &str) -> Option<&str> {
    match scope {
        "printed_text_line_extraction_only" => Some(scope),
        _ => None,
    }
}

fn optional_nonnegative_u64(value: Option<&Value>) -> Option<Option<u64>> {
    match value {
        Some(Value::Number(number)) => number.as_u64().map(Some),
        Some(_) => None,
        None => Some(None),
    }
}

fn sanitized_ocr_handoff_non_goals(value: Option<&Value>) -> Option<Vec<String>> {
    const ALLOWED: &[&str] = &[
        "ocr_quality_claim",
        "visual_redaction",
        "image_pixel_redaction",
        "final_pdf_rewrite_export",
        "browser_ui",
        "desktop_ui",
        "handwriting_recognition",
    ];
    let values = value?.as_array()?;
    let mut out = Vec::new();
    for item in values {
        let item = item.as_str()?;
        if ALLOWED.contains(&item) {
            out.push(item.to_owned());
        } else {
            return None;
        }
    }
    if !out.iter().any(|item| item == "visual_redaction")
        || !out.iter().any(|item| item == "image_pixel_redaction")
        || !out.iter().any(|item| item == "final_pdf_rewrite_export")
    {
        return None;
    }
    Some(out)
}

fn contains_incompatible_ocr_handoff_marker(report: &Map<String, Value>) -> bool {
    const INCOMPATIBLE_MARKERS: &[&str] = &[
        "image_bytes",
        "image_bytes_base64",
        "masked_text",
        "spans",
        "preview",
        "pdf_rewrite",
        "pdf_export",
        "visual_redaction_result",
        "pixel_redaction",
        "agent_id",
        "controller_step",
        "complete_command",
        "claim",
    ];
    report.iter().any(|(key, value)| {
        INCOMPATIBLE_MARKERS.contains(&key.as_str())
            || match value {
                Value::Object(object) => contains_incompatible_ocr_handoff_marker(object),
                Value::Array(values) => values.iter().any(|value| match value {
                    Value::Object(object) => contains_incompatible_ocr_handoff_marker(object),
                    _ => false,
                }),
                _ => false,
            }
    })
}
```

- [ ] **Step 4: Run success test to verify it passes**

Run: `/home/azureuser/.cargo/bin/cargo test -p mdid-runtime ocr_handoff_summary_accepts_existing_fixture_contract -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Write failing tests for unsafe inputs**

Add tests:

```rust
#[tokio::test]
async fn ocr_handoff_summary_rejects_raw_text_and_incompatible_markers_without_echoing_phi() {
    let app = router();
    let mut handoff: serde_json::Value = serde_json::from_str(include_str!(
        "../../../scripts/ocr_eval/fixtures/ocr_handoff_expected_shape.json"
    ))
    .unwrap();
    handoff["spans"] = serde_json::json!([{ "preview": "Jane Example" }]);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/ocr-handoff/summary")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({ "handoff": handoff }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let serialized = String::from_utf8(body.to_vec()).unwrap();
    assert!(serialized.contains("invalid_ocr_handoff_summary_request"));
    for forbidden in ["Jane Example", "MRN-12345", "jane@example.com", "555-123-4567"] {
        assert!(!serialized.contains(forbidden), "error leaked {forbidden}");
    }
}

#[tokio::test]
async fn ocr_handoff_summary_rejects_not_ready_or_wrong_contract() {
    for mutation in [
        ("ready_for_text_pii_eval", serde_json::json!(false)),
        ("privacy_filter_contract", serde_json::json!("visual_redaction")),
        ("scope", serde_json::json!("full_pdf_ocr")),
        ("line_count", serde_json::json!("Patient Jane Example")),
    ] {
        let app = router();
        let mut handoff: serde_json::Value = serde_json::from_str(include_str!(
            "../../../scripts/ocr_eval/fixtures/ocr_handoff_expected_shape.json"
        ))
        .unwrap();
        handoff[mutation.0] = mutation.1;
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/ocr-handoff/summary")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::json!({ "handoff": handoff }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "mutation {}", mutation.0);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let serialized = String::from_utf8(body.to_vec()).unwrap();
        assert!(!serialized.contains("Jane Example"));
    }
}
```

- [ ] **Step 6: Run unsafe-input tests to verify they fail if helper is permissive, then pass after implementation**

Run: `/home/azureuser/.cargo/bin/cargo test -p mdid-runtime ocr_handoff_summary -- --nocapture`

Expected after implementation: PASS.

- [ ] **Step 7: Run broader runtime tests and formatting**

Run:

```bash
/home/azureuser/.cargo/bin/cargo fmt --check
/home/azureuser/.cargo/bin/cargo test -p mdid-runtime privacy_filter -- --nocapture
/home/azureuser/.cargo/bin/cargo test -p mdid-runtime ocr_handoff_summary -- --nocapture
```

Expected: all PASS.

- [ ] **Step 8: Commit**

```bash
git add crates/mdid-runtime/src/http.rs
git commit -m "feat(runtime): add OCR handoff summary endpoint"
```

### Task 2: README Completion Truth-Sync

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update current status evidence**

Modify `README.md` current repository status to mention `POST /ocr-handoff/summary` as runtime-only bounded PP-OCRv5 mobile existing-report summary evidence. Keep completion numbers unchanged unless the controller-visible rubric supports a change: CLI 95%, Browser/Web 99%, Desktop app 99%, Overall 97%.

- [ ] **Step 2: Add verification evidence paragraph**

Add a paragraph near the existing OCR evidence:

```markdown
Verification evidence for the bounded runtime OCR handoff summary endpoint landed on this branch: `POST /ocr-handoff/summary` accepts an existing PP-OCRv5 mobile OCR handoff JSON report wrapped as `{ "handoff": <handoff-output> }`, supports the current fixture-backed contract with `candidate: PP-OCRv5_mobile_rec`, `scope: printed_text_line_extraction_only`, `privacy_filter_contract: text_only_normalized_input`, and `ready_for_text_pii_eval: true`, and returns only an aggregate PHI-safe `ocr_handoff_summary`. It does not run OCR, does not run Privacy Filter, does not call a network API, does not expose raw OCR text / normalized text / bbox / image bytes / spans / previews, and does not claim visual redaction, image pixel redaction, final PDF rewrite/export, Browser/Web execution, Desktop execution, or unrelated workflow orchestration semantics. Repository-visible verification: `cargo test -p mdid-runtime ocr_handoff_summary -- --nocapture`, `cargo test -p mdid-runtime privacy_filter -- --nocapture`, and `cargo fmt --check`.
```

- [ ] **Step 3: Update completion arithmetic text without changing unsupported surface numbers**

In the current snapshot prose, state that this round adds and completes one runtime OCR handoff summary requirement in the same round. Use explicit fraction accounting in the README paragraph if a fraction is already present nearby; otherwise keep integer snapshot unchanged and explain that Browser/Web/Desktop did not change because no new user-facing surface capability landed.

- [ ] **Step 4: Run README grep sanity checks**

Run:

```bash
grep -n "ocr-handoff/summary\|CLI | 95%\|Browser/Web | 99%\|Desktop app | 99%\|Overall | 97%" README.md
```

Expected: new endpoint is documented; completion rows remain truthful.

- [ ] **Step 5: Commit**

```bash
git add README.md
git commit -m "docs: truth-sync runtime OCR handoff summary evidence"
```

## Self-Review

- Spec coverage: The plan covers runtime route, safe request/response contract, fixture-backed success, unsafe rejection, broader runtime verification, and README completion truth-sync.
- Placeholder scan: No TBD/TODO/fill-in placeholders are present.
- Type consistency: Endpoint name is consistently `/ocr-handoff/summary`; request field is consistently `handoff`; response artifact is consistently `ocr_handoff_summary`.
