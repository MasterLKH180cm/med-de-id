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
use mdid_domain::{MappingScope, SurfaceKind};
use mdid_runtime::http::{build_router, RuntimeState};
use mdid_vault::{LocalVaultStore, NewMappingRecord};
use serde_json::{json, Value};
use tempfile::tempdir;
use tower::ServiceExt;
use uuid::Uuid;

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

    assert_ne!(
        tag_value(&rewritten_obj, Tag(0x0010, 0x0010)),
        "Alice^Smith"
    );
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

#[tokio::test]
async fn vault_decode_endpoint_returns_decoded_values_and_audit_event() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("runtime-vault.mdid");
    let mut vault = LocalVaultStore::create(&vault_path, "correct horse battery staple").unwrap();
    let stored = vault
        .store_mapping(
            NewMappingRecord {
                scope: sample_scope("patient.name"),
                phi_type: "patient_name".into(),
                original_value: "Alice Smith".into(),
            },
            SurfaceKind::Browser,
        )
        .unwrap();

    let app = build_router(RuntimeState::default());
    let request = json!({
        "vault_path": vault_path,
        "vault_passphrase": "correct horse battery staple",
        "record_ids": [stored.id],
        "output_target": "investigator export",
        "justification": "incident review",
        "requested_by": "desktop"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/vault/decode")
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    let values = json["values"].as_array().unwrap();
    assert_eq!(values.len(), 1);
    assert_eq!(values[0]["original_value"], "Alice Smith");
    assert_eq!(values[0]["token"], stored.token);
    assert_eq!(values[0]["record_id"], stored.id.to_string());
    assert_eq!(
        values[0]["scope"]["job_id"],
        stored.scope.job_id.to_string()
    );
    assert_eq!(
        values[0]["scope"]["artifact_id"],
        stored.scope.artifact_id.to_string()
    );
    assert_eq!(values[0]["scope"]["field_path"], stored.scope.field_path);

    assert_eq!(json["audit_event"]["kind"], "decode");
    let detail = json["audit_event"]["detail"].as_str().unwrap();
    assert!(detail.contains("investigator export"));
    assert!(detail.contains("incident review"));
    assert!(detail.contains("1 record"));
}

#[tokio::test]
async fn vault_decode_endpoint_rejects_unknown_record_scope() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("runtime-vault.mdid");
    let _vault = LocalVaultStore::create(&vault_path, "correct horse battery staple").unwrap();

    let app = build_router(RuntimeState::default());
    let request = json!({
        "vault_path": vault_path,
        "vault_passphrase": "correct horse battery staple",
        "record_ids": [Uuid::new_v4()],
        "output_target": "investigator export",
        "justification": "incident review",
        "requested_by": "desktop"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/vault/decode")
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        json,
        json!({
            "error": {
                "code": "unknown_record",
                "message": "decode scope referenced a record that does not exist"
            }
        })
    );
    assert!(json.get("values").is_none());
    assert!(json.get("audit_event").is_none());
}

#[tokio::test]
async fn vault_decode_endpoint_rejects_wrong_passphrase() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("runtime-vault.mdid");
    let mut vault = LocalVaultStore::create(&vault_path, "correct horse battery staple").unwrap();
    let stored = vault
        .store_mapping(
            NewMappingRecord {
                scope: sample_scope("patient.name"),
                phi_type: "patient_name".into(),
                original_value: "Alice Smith".into(),
            },
            SurfaceKind::Browser,
        )
        .unwrap();

    let app = build_router(RuntimeState::default());
    let request = json!({
        "vault_path": vault_path,
        "vault_passphrase": "totally wrong passphrase",
        "record_ids": [stored.id],
        "output_target": "investigator export",
        "justification": "incident review",
        "requested_by": "desktop"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/vault/decode")
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        json,
        json!({
            "error": {
                "code": "vault_unlock_failed",
                "message": "vault could not be unlocked with the supplied passphrase"
            }
        })
    );
    assert!(json.get("values").is_none());
    assert!(json.get("audit_event").is_none());
}

#[tokio::test]
async fn vault_decode_endpoint_rejects_invalid_decode_request_payload() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("runtime-vault.mdid");
    let _vault = LocalVaultStore::create(&vault_path, "correct horse battery staple").unwrap();

    let app = build_router(RuntimeState::default());
    let request = json!({
        "vault_path": vault_path,
        "vault_passphrase": "correct horse battery staple",
        "record_ids": [],
        "output_target": "investigator export",
        "justification": "incident review",
        "requested_by": "desktop"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/vault/decode")
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_invalid_decode_request_response(response).await;
}

#[tokio::test]
async fn vault_decode_endpoint_rejects_unusable_vault_target() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("not-a-vault.mdid");
    std::fs::write(&vault_path, "not valid vault json").unwrap();

    let app = build_router(RuntimeState::default());
    let request = json!({
        "vault_path": vault_path,
        "vault_passphrase": "correct horse battery staple",
        "record_ids": [Uuid::new_v4()],
        "output_target": "investigator export",
        "justification": "incident review",
        "requested_by": "desktop"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/vault/decode")
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_invalid_vault_target_response(response).await;
}

#[tokio::test]
async fn vault_decode_endpoint_rejects_corrupted_encrypted_vault_artifact() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("runtime-vault.mdid");
    let mut vault = LocalVaultStore::create(&vault_path, "correct horse battery staple").unwrap();
    let stored = vault
        .store_mapping(
            NewMappingRecord {
                scope: sample_scope("patient.name"),
                phi_type: "patient_name".into(),
                original_value: "Alice Smith".into(),
            },
            SurfaceKind::Browser,
        )
        .unwrap();
    let mut envelope: Value =
        serde_json::from_str(&std::fs::read_to_string(&vault_path).unwrap()).unwrap();
    envelope["ciphertext_b64"] = json!("%%%not-base64%%%");
    std::fs::write(&vault_path, envelope.to_string()).unwrap();

    let app = build_router(RuntimeState::default());
    let request = json!({
        "vault_path": vault_path,
        "vault_passphrase": "correct horse battery staple",
        "record_ids": [stored.id],
        "output_target": "investigator export",
        "justification": "incident review",
        "requested_by": "desktop"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/vault/decode")
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_invalid_vault_target_response(response).await;
}

async fn assert_invalid_dicom_response(response: axum::response::Response) {
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        json,
        json!({
            "error": {
                "code": "invalid_dicom",
                "message": "request body did not contain a valid DICOM payload"
            }
        })
    );
    assert!(json.get("rewritten_dicom_bytes_base64").is_none());
    assert!(json.get("summary").is_none());
    assert!(json.get("review_queue").is_none());
}

async fn assert_invalid_decode_request_response(response: axum::response::Response) {
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        json,
        json!({
            "error": {
                "code": "invalid_decode_request",
                "message": "request body did not contain a valid vault decode request"
            }
        })
    );
    assert!(json.get("values").is_none());
    assert!(json.get("audit_event").is_none());
}

async fn assert_invalid_vault_target_response(response: axum::response::Response) {
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        json,
        json!({
            "error": {
                "code": "invalid_vault_target",
                "message": "vault target could not be read as a usable vault artifact"
            }
        })
    );
    assert!(json.get("values").is_none());
    assert!(json.get("audit_event").is_none());
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

fn sample_scope(field_path: &str) -> MappingScope {
    MappingScope::new(Uuid::new_v4(), Uuid::new_v4(), field_path.to_string())
}
