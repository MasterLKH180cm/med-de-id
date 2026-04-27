use std::io::Cursor;

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use dicom_core::{Tag, VR};
use dicom_object::{
    file::ReadPreamble, meta::FileMetaTableBuilder, DefaultDicomObject, InMemDicomObject,
    OpenFileOptions,
};
use mdid_runtime::http::{build_router, RuntimeState};
use serde_json::{json, Value};
use tower::ServiceExt;

#[tokio::test]
async fn health_endpoint_returns_ok() {
    let app = build_router(RuntimeState::default());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");
}

#[tokio::test]
async fn pipelines_endpoint_registers_pipeline() {
    let app = build_router(RuntimeState::default());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/pipelines")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"foundation"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["name"], "foundation");
    assert!(json["id"].as_str().is_some());
}

#[tokio::test]
async fn dicom_deidentify_endpoint_returns_rewritten_bytes_and_summary() {
    let app = build_router(RuntimeState::default());
    let request = json!({
        "dicom_bytes_base64": STANDARD.encode(build_dicom_fixture("YES", true)),
        "source_name": "Alice Smith/MRN-001/private-scan.dcm",
        "private_tag_policy": "review_required"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/dicom/deidentify")
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["sanitized_file_name"], "dicom-output.dcm");
    assert!(json["rewritten_dicom_bytes_base64"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    assert_eq!(json["summary"]["total_tags"], 6);
    assert_eq!(json["summary"]["encoded_tags"], 4);
    assert_eq!(json["summary"]["review_required_tags"], 2);
    assert_eq!(json["summary"]["removed_private_tags"], 0);
    assert_eq!(json["summary"]["remapped_uids"], 3);
    assert_eq!(json["summary"]["burned_in_suspicions"], 1);
    assert!(json["review_queue"].is_array());
    assert_eq!(json["review_queue"].as_array().unwrap().len(), 2);

    let rewritten = STANDARD
        .decode(
            json["rewritten_dicom_bytes_base64"]
                .as_str()
                .expect("expected rewritten bytes payload"),
        )
        .unwrap();
    let rewritten_obj = parse_dicom(&rewritten);

    assert_ne!(tag_value(&rewritten_obj, Tag(0x0010, 0x0010)), "Alice^Smith");
    assert_eq!(tag_value(&rewritten_obj, Tag(0x0028, 0x0301)), "YES");
}

#[tokio::test]
async fn dicom_deidentify_endpoint_rejects_malformed_base64_payload() {
    let app = build_router(RuntimeState::default());
    let request = json!({
        "dicom_bytes_base64": "%%%not-base64%%%",
        "source_name": "broken.dcm",
        "private_tag_policy": "remove"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/dicom/deidentify")
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_invalid_dicom_response(response).await;
}

#[tokio::test]
async fn dicom_deidentify_endpoint_rejects_invalid_dicom_bytes() {
    let app = build_router(RuntimeState::default());
    let request = json!({
        "dicom_bytes_base64": STANDARD.encode(b"not-a-dicom-payload"),
        "source_name": "broken.dcm",
        "private_tag_policy": "remove"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/dicom/deidentify")
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_invalid_dicom_response(response).await;
}

async fn assert_invalid_dicom_response(response: axum::response::Response) {
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json, json!({
        "error": {
            "code": "invalid_dicom",
            "message": "request body did not contain a valid DICOM payload"
        }
    }));
    assert!(json.get("rewritten_dicom_bytes_base64").is_none());
    assert!(json.get("summary").is_none());
    assert!(json.get("review_queue").is_none());
}

fn build_dicom_fixture(burned_in_annotation: &str, include_private: bool) -> Vec<u8> {
    let mut obj = base_dicom_fixture();
    obj.put_str(Tag(0x0008, 0x0050), VR::SH, "ACC-4242");
    obj.put_str(Tag(0x0008, 0x1030), VR::LO, "Cardiac MRI");
    obj.put_str(Tag(0x0010, 0x0010), VR::PN, "Alice^Smith");
    obj.put_str(Tag(0x0010, 0x0020), VR::LO, "MRN-001");
    obj.put_str(Tag(0x0028, 0x0301), VR::CS, burned_in_annotation);

    if include_private {
        obj.put_str(Tag(0x0011, 0x0010), VR::LO, "ACME_CREATOR");
        obj.put_str(Tag(0x0011, 0x1010), VR::LO, "secret-annotation");
    }

    serialize_dicom(obj)
}

fn base_dicom_fixture() -> InMemDicomObject {
    let mut obj = InMemDicomObject::new_empty();
    obj.put_str(Tag(0x0008, 0x0016), VR::UI, "1.2.840.10008.5.1.4.1.1.7");
    obj.put_str(
        Tag(0x0008, 0x0018),
        VR::UI,
        "2.25.123456789012345678901234567890123456",
    );
    obj.put_str(
        Tag(0x0020, 0x000D),
        VR::UI,
        "2.25.123456789012345678901234567890123457",
    );
    obj.put_str(
        Tag(0x0020, 0x000E),
        VR::UI,
        "2.25.123456789012345678901234567890123458",
    );
    obj
}

fn serialize_dicom(obj: InMemDicomObject) -> Vec<u8> {
    let file_obj = obj
        .with_meta(FileMetaTableBuilder::new().transfer_syntax("1.2.840.10008.1.2.1"))
        .expect("fixture should create file object");
    let mut bytes = Vec::new();
    file_obj
        .write_all(&mut bytes)
        .expect("fixture should serialize to bytes");
    bytes
}

fn parse_dicom(bytes: &[u8]) -> DefaultDicomObject {
    OpenFileOptions::new()
        .read_preamble(ReadPreamble::Always)
        .from_reader(Cursor::new(bytes))
        .expect("rewritten fixture should parse as DICOM")
}

fn tag_value(obj: &DefaultDicomObject, tag: Tag) -> String {
    obj.get(tag)
        .expect("expected DICOM tag to be present")
        .to_str()
        .expect("expected DICOM tag to be textual")
        .into_owned()
}
