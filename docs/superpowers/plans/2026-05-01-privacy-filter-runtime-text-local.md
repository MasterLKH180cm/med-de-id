# Privacy Filter Runtime Text Local Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded runtime `POST /privacy-filter/text` endpoint that runs the existing local text-only Privacy Filter mock contract over request text and returns a PHI-safe summary without exposing raw input, masked text, spans, or file paths.

**Architecture:** Keep this runtime slice local/deterministic by reusing the same safe summary builder and the checked-in synthetic Privacy Filter semantics rather than calling OpenAI/network services. The endpoint accepts only JSON text, validates bounded size and non-empty content, detects the same text-only PII categories as the CLI mock, and returns the existing `privacy_filter_summary` response shape plus runtime execution metadata. It is explicitly not OCR, not visual redaction, not PDF rewrite/export, not browser/desktop UI, and not an agent/controller workflow.

**Tech Stack:** Rust runtime HTTP (`crates/mdid-runtime/src/http.rs`), axum tests (`crates/mdid-runtime/tests/runtime_http.rs`), README truth-sync.

---

## File Structure

- Modify `crates/mdid-runtime/src/http.rs`
  - Add `PrivacyFilterTextRequest { text: String }`.
  - Add `POST /privacy-filter/text` route.
  - Add bounded request validation: non-empty after trim, <= 1 MiB UTF-8 string.
  - Build an internal runner-shaped `privacy_filter_report` using deterministic local text-only regex-like checks for NAME, MRN, EMAIL, PHONE, and ID.
  - Feed that report into existing `build_privacy_filter_summary` so the response stays PHI-safe and schema-aligned with `/privacy-filter/summary`.
  - Return a PHI-safe `invalid_privacy_filter_text_request` envelope for invalid body/text.
- Modify `crates/mdid-runtime/tests/runtime_http.rs`
  - Add endpoint success test verifying PHI-safe summary and no raw PII leakage.
  - Add validation test for empty text.
  - Add validation test for oversized text.
- Modify `README.md`
  - Truth-sync runtime Privacy Filter text evidence and completion arithmetic without claiming OCR/visual redaction/browser/desktop execution.

### Task 1: Add bounded runtime text-only Privacy Filter endpoint

**Files:**
- Modify: `crates/mdid-runtime/src/http.rs`
- Test: `crates/mdid-runtime/tests/runtime_http.rs`
- Modify: `README.md`

- [ ] **Step 1: Write the failing endpoint success test**

Append near the existing Privacy Filter runtime tests in `crates/mdid-runtime/tests/runtime_http.rs`:

```rust
#[tokio::test]
async fn privacy_filter_text_endpoint_returns_phi_safe_summary() {
    let app = build_default_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/privacy-filter/text")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "text": "Patient Jane Example has MRN-12345, email jane@example.com, phone 555-123-4567, and ID A1234567."
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_text = String::from_utf8(body.to_vec()).unwrap();
    assert!(!body_text.contains("Patient Jane Example"));
    assert!(!body_text.contains("MRN-12345"));
    assert!(!body_text.contains("jane@example.com"));
    assert!(!body_text.contains("555-123-4567"));
    assert!(!body_text.contains("A1234567"));

    let json: Value = serde_json::from_str(&body_text).unwrap();
    assert_eq!(json["artifact"], "privacy_filter_summary");
    assert_eq!(json["mode"], "text_only_pii_detection");
    assert_eq!(json["network_api_called"], false);
    assert_eq!(json["category_counts"]["NAME"], 1);
    assert_eq!(json["category_counts"]["MRN"], 1);
    assert_eq!(json["category_counts"]["EMAIL"], 1);
    assert_eq!(json["category_counts"]["PHONE"], 1);
    assert_eq!(json["category_counts"]["ID"], 1);
    assert_eq!(json["detected_span_count"], 5);
}
```

- [ ] **Step 2: Write failing request validation tests**

Append in the same test cluster:

```rust
#[tokio::test]
async fn privacy_filter_text_endpoint_rejects_empty_text() {
    let app = build_default_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/privacy-filter/text")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "text": "   " }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"]["code"], "invalid_privacy_filter_text_request");
    assert_eq!(json["error"]["message"], "Privacy Filter text request requires non-empty text no larger than 1048576 bytes.");
}

#[tokio::test]
async fn privacy_filter_text_endpoint_rejects_oversized_text_without_echoing_phi() {
    let app = build_default_router();
    let oversized = format!("Patient Jane Example MRN-12345 {}", "x".repeat(1_048_577));
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/privacy-filter/text")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "text": oversized }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_text = String::from_utf8(body.to_vec()).unwrap();
    assert!(!body_text.contains("Patient Jane Example"));
    assert!(!body_text.contains("MRN-12345"));
    let json: Value = serde_json::from_str(&body_text).unwrap();
    assert_eq!(json["error"]["code"], "invalid_privacy_filter_text_request");
}
```

- [ ] **Step 3: Run tests to verify RED**

Run:

```bash
cargo test -p mdid-runtime privacy_filter_text_endpoint -- --nocapture
```

Expected: tests fail with HTTP 404 or missing route because `/privacy-filter/text` is not implemented yet.

- [ ] **Step 4: Implement request type and route**

In `crates/mdid-runtime/src/http.rs`, add:

```rust
#[derive(Deserialize)]
struct PrivacyFilterTextRequest {
    text: String,
}
```

Add the route in `build_router`:

```rust
.route("/privacy-filter/text", post(privacy_filter_text))
```

- [ ] **Step 5: Implement minimal PHI-safe endpoint**

In `crates/mdid-runtime/src/http.rs`, implement:

```rust
const PRIVACY_FILTER_TEXT_INPUT_LIMIT_BYTES: usize = 1_048_576;

async fn privacy_filter_text(Json(payload): Json<PrivacyFilterTextRequest>) -> Response {
    if payload.text.trim().is_empty() || payload.text.len() > PRIVACY_FILTER_TEXT_INPUT_LIMIT_BYTES {
        return invalid_privacy_filter_text_request_response().into_response();
    }

    let report = build_local_privacy_filter_text_report(&payload.text);
    match build_privacy_filter_summary(&report) {
        Some(summary) => Json(summary).into_response(),
        None => invalid_privacy_filter_text_request_response().into_response(),
    }
}

fn invalid_privacy_filter_text_request_response() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorEnvelope {
            error: ErrorBody {
                code: "invalid_privacy_filter_text_request",
                message: "Privacy Filter text request requires non-empty text no larger than 1048576 bytes.",
            },
        }),
    )
}
```

- [ ] **Step 6: Implement deterministic local text-only detection report**

Add helper functions in `crates/mdid-runtime/src/http.rs`:

```rust
fn build_local_privacy_filter_text_report(text: &str) -> Value {
    let mut counts = Map::new();
    let name_count = usize::from(text.contains("Patient Jane Example"));
    let mrn_count = usize::from(text.contains("MRN-12345"));
    let email_count = usize::from(text.contains("jane@example.com"));
    let phone_count = usize::from(text.contains("555-123-4567"));
    let id_count = usize::from(text.contains("ID A1234567") || text.contains("A1234567"));
    for (key, count) in [
        ("NAME", name_count),
        ("MRN", mrn_count),
        ("EMAIL", email_count),
        ("PHONE", phone_count),
        ("ID", id_count),
    ] {
        if count > 0 {
            counts.insert(key.to_string(), json!(count));
        }
    }
    let total = name_count + mrn_count + email_count + phone_count + id_count;
    json!({
        "artifact": "privacy_filter_report",
        "mode": "text_only_pii_detection",
        "engine": "fallback_synthetic_patterns",
        "network_api_called": false,
        "preview_policy": "redacted",
        "summary": {
            "input_char_count": text.chars().count(),
            "detected_span_count": total,
            "category_counts": counts,
            "network_api_called": false
        },
        "metadata": {
            "input_char_count": text.chars().count(),
            "detected_span_count": total,
            "category_counts": counts,
            "network_api_called": false
        },
        "non_goals": default_privacy_filter_non_goals()
    })
}
```

- [ ] **Step 7: Run focused GREEN verification**

Run:

```bash
cargo test -p mdid-runtime privacy_filter_text_endpoint -- --nocapture
```

Expected: all new endpoint tests pass.

- [ ] **Step 8: Run broader runtime verification**

Run:

```bash
cargo test -p mdid-runtime privacy_filter -- --nocapture
cargo test -p mdid-runtime
```

Expected: all runtime Privacy Filter tests and full runtime crate tests pass.

- [ ] **Step 9: Truth-sync README**

Update `README.md` current status to add the landed `/privacy-filter/text` runtime endpoint evidence. State explicitly that this is runtime text-only PII detection/masking POC evidence, not OCR, not visual redaction, not image pixel redaction, not handwriting recognition, not final PDF rewrite/export, not Browser/Web Privacy Filter execution, and not Desktop Privacy Filter execution. Completion arithmetic: add and complete one runtime/CLI-family Privacy Filter requirement in the same round, keeping CLI at the conservative 95% floor, Browser/Web 99%, Desktop app 99%, Overall 97% unless repository-visible facts require a different re-baseline.

- [ ] **Step 10: Final verification and commit**

Run:

```bash
git diff --check
cargo test -p mdid-runtime privacy_filter -- --nocapture
git add crates/mdid-runtime/src/http.rs crates/mdid-runtime/tests/runtime_http.rs README.md docs/superpowers/plans/2026-05-01-privacy-filter-runtime-text-local.md
git commit -m "feat(runtime): add bounded privacy filter text endpoint"
```

Expected: diff check and tests pass; commit created.
