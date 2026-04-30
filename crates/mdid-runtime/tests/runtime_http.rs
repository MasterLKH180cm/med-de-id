use std::io::{Cursor, Read, Write};

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use calamine::{open_workbook_from_rs, Data, Reader, Xlsx};
use dicom_core::{Tag, VR};
use dicom_object::{
    file::ReadPreamble, meta::FileMetaTableBuilder, DefaultDicomObject, InMemDicomObject,
    OpenFileOptions,
};
use mdid_adapters::XlsxTabularAdapter;
use mdid_domain::{MappingScope, SurfaceKind};
use mdid_runtime::http::{build_router, RuntimeState};
use mdid_vault::{LocalVaultStore, NewMappingRecord, PortableVaultArtifact};
use rust_xlsxwriter::Workbook;
use serde_json::{json, Value};
use tempfile::tempdir;
use tower::ServiceExt;
use uuid::Uuid;
use xmltree::{Element, XMLNode};
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};

const SAMPLE_XLSX_WORKBOOK_BASE64: &str = "UEsDBBQAAAAAAHmpm1y2+9qcrgIAAK4CAAATAAAAW0NvbnRlbnRfVHlwZXNdLnhtbDw/eG1sIHZlcnNpb249IjEuMCIgZW5jb2Rpbmc9IlVURi04IiBzdGFuZGFsb25lPSJ5ZXMiPz4KPFR5cGVzIHhtbG5zPSJodHRwOi8vc2NoZW1hcy5vcGVueG1sZm9ybWF0cy5vcmcvcGFja2FnZS8yMDA2L2NvbnRlbnQtdHlwZXMiPgo8RGVmYXVsdCBFeHRlbnNpb249InJlbHMiIENvbnRlbnRUeXBlPSJhcHBsaWNhdGlvbi92bmQub3BlbnhtbGZvcm1hdHMtcGFja2FnZS5yZWxhdGlvbnNoaXBzK3htbCIvPgo8RGVmYXVsdCBFeHRlbnNpb249InhtbCIgQ29udGVudFR5cGU9ImFwcGxpY2F0aW9uL3htbCIvPgo8T3ZlcnJpZGUgUGFydE5hbWU9Ii94bC93b3JrYm9vay54bWwiIENvbnRlbnRUeXBlPSJhcHBsaWNhdGlvbi92bmQub3BlbnhtbGZvcm1hdHMtb2ZmaWNlZG9jdW1lbnQuc3ByZWFkc2hlZXRtbC5zaGVldC5tYWluK3htbCIvPgo8T3ZlcnJpZGUgUGFydE5hbWU9Ii94bC93b3Jrc2hlZXRzL3NoZWV0MS54bWwiIENvbnRlbnRUeXBlPSJhcHBsaWNhdGlvbi92bmQub3BlbnhtbGZvcm1hdHMtb2ZmaWNlZG9jdW1lbnQuc3ByZWFkc2hlZXRtbC53b3Jrc2hlZXQreG1sIi8+CjxPdmVycmlkZSBQYXJ0TmFtZT0iL3hsL3N0eWxlcy54bWwiIENvbnRlbnRUeXBlPSJhcHBsaWNhdGlvbi92bmQub3BlbnhtbGZvcm1hdHMtb2ZmaWNlZG9jdW1lbnQuc3ByZWFkc2hlZXRtbC5zdHlsZXMreG1sIi8+CjwvVHlwZXM+UEsDBBQAAAAAAHmpm1x+b8CFKgEAACoBAAALAAAAX3JlbHMvLnJlbHM8P3htbCB2ZXJzaW9uPSIxLjAiIGVuY29kaW5nPSJVVEYtOCIgc3RhbmRhbG9uZT0ieWVzIj8+CjxSZWxhdGlvbnNoaXBzIHhtbG5zPSJodHRwOi8vc2NoZW1hcy5vcGVueG1sZm9ybWF0cy5vcmcvcGFja2FnZS8yMDA2L3JlbGF0aW9uc2hpcHMiPgo8UmVsYXRpb25zaGlwIElkPSJySWQxIiBUeXBlPSJodHRwOi8vc2NoZW1hcy5vcGVueG1sZm9ybWF0cy5vcmcvb2ZmaWNlRG9jdW1lbnQvMjAwNi9yZWxhdGlvbnNoaXBzL29mZmljZURvY3VtZW50IiBUYXJnZXQ9InhsL3dvcmtib29rLnhtbCIvPgo8L1JlbGF0aW9uc2hpcHM+UEsDBBQAAAAAAHmpm1x3QP7EHAEAABwBAAAPAAAAeGwvd29ya2Jvb2sueG1sPD94bWwgdmVyc2lvbj0iMS4wIiBlbmNvZGluZz0iVVRGLTgiIHN0YW5kYWxvbmU9InllcyI/Pgo8d29ya2Jvb2sgeG1sbnM9Imh0dHA6Ly9zY2hlbWFzLm9wZW54bWxmb3JtYXRzLm9yZy9zcHJlYWRzaGVldG1sLzIwMDYvbWFpbiIgeG1sbnM6cj0iaHR0cDovL3NjaGVtYXMub3BlbnhtbGZvcm1hdHMub3JnL29mZmljZURvY3VtZW50LzIwMDYvcmVsYXRpb25zaGlwcyI+PHNoZWV0cz48c2hlZXQgbmFtZT0iU2hlZXQxIiBzaGVldElkPSIxIiByOmlkPSJySWQxIi8+PC9zaGVldHM+PC93b3JrYm9vaz5QSwMEFAAAAAAAeambXB+qsIOrAQAAqwEAABoAAAB4bC9fcmVscy93b3JrYm9vay54bWwucmVsczw/eG1sIHZlcnNpb249IjEuMCIgZW5jb2Rpbmc9IlVURi04IiBzdGFuZGFsb25lPSJ5ZXMiPz4KPFJlbGF0aW9uc2hpcHMgeG1sbnM9Imh0dHA6Ly9zY2hlbWFzLm9wZW54bWxmb3JtYXRzLm9yZy9wYWNrYWdlLzIwMDYvcmVsYXRpb25zaGlwcyI+CjxSZWxhdGlvbnNoaXAgSWQ9InJJZDEiIFR5cGU9Imh0dHA6Ly9zY2hlbWFzLm9wZW54bWxmb3JtYXRzLm9yZy9vZmZpY2VEb2N1bWVudC8yMDA2L3JlbGF0aW9uc2hpcHMvd29ya3NoZWV0IiBUYXJnZXQ9IndvcmtzaGVldHMvc2hlZXQxLnhtbCIvPgo8UmVsYXRpb25zaGlwIElkPSJySWQyIiBUeXBlPSJodHRwOi8vc2NoZW1hcy5vcGVueG1sZm9ybWF0cy5vcmcvb2ZmaWNlRG9jdW1lbnQvMjAwNi9yZWxhdGlvbnNoaXBzL3N0eWxlcyIgVGFyZ2V0PSJzdHlsZXMueG1sIi8+CjwvUmVsYXRpb25zaGlwcz5QSwMEFAAAAAAAeambXL2k1bb0AQAA9AEAAA0AAAB4bC9zdHlsZXMueG1sPD94bWwgdmVyc2lvbj0iMS4wIiBlbmNvZGluZz0iVVRGLTgiIHN0YW5kYWxvbmU9InllcyI/Pgo8c3R5bGVTaGVldCB4bWxucz0iaHR0cDovL3NjaGVtYXMub3BlbnhtbGZvcm1hdHMub3JnL3NwcmVhZHNoZWV0bWwvMjAwNi9tYWluIj48Zm9udHMgY291bnQ9IjEiPjxmb250PjxzeiB2YWw9IjExIi8+PG5hbWUgdmFsPSJDYWxpYnJpIi8+PC9mb250PjwvZm9udHM+PGZpbGxzIGNvdW50PSIxIj48ZmlsbD48cGF0dGVybkZpbGwgcGF0dGVyblR5cGU9Im5vbmUiLz48L2ZpbGw+PC9maWxscz48Ym9yZGVycyBjb3VudD0iMSI+PGJvcmRlci8+PC9ib3JkZXJzPjxjZWxsU3R5bGVYZnMgY291bnQ9IjEiPjx4Zi8+PC9jZWxsU3R5bGVYZnM+PGNlbGxYZnMgY291bnQ9IjEiPjx4ZiB4ZklkPSIwIi8+PC9jZWxsWGZzPjxjZWxsU3R5bGVzIGNvdW50PSIxIj48Y2VsbFN0eWxlIG5hbWU9Ik5vcm1hbCIgeGZJZD0iMCIgYnVpbHRpbklkPSIwIi8+PC9jZWxsU3R5bGVzPjwvc3R5bGVTaGVldD5QSwMEFAAAAAAAeambXJyibJUdAgAAHQIAABgAAAB4bC93b3Jrc2hlZXRzL3NoZWV0MS54bWw8P3htbCB2ZXJzaW9uPSIxLjAiIGVuY29kaW5nPSJVVEYtOCIgc3RhbmRhbG9uZT0ieWVzIj8+Cjx3b3Jrc2hlZXQgeG1sbnM9Imh0dHA6Ly9zY2hlbWFzLm9wZW54bWxmb3JtYXRzLm9yZy9zcHJlYWRzaGVldG1sLzIwMDYvbWFpbiI+PHNoZWV0RGF0YT48cm93IHI9IjEiPjxjIHI9IkExIiB0PSJpbmxpbmVTdHIiPjxpcz48dD5wYXRpZW50X2lkPC90PjwvaXM+PC9jPjxjIHI9IkIxIiB0PSJpbmxpbmVTdHIiPjxpcz48dD5wYXRpZW50X25hbWU8L3Q+PC9pcz48L2M+PC9yb3c+PHJvdyByPSIyIj48YyByPSJBMiIgdD0iaW5saW5lU3RyIj48aXM+PHQ+TVJOLTAwMTwvdD48L2lzPjwvYz48YyByPSJCMiIgdD0iaW5saW5lU3RyIj48aXM+PHQ+QWxpY2UgU21pdGg8L3Q+PC9pcz48L2M+PC9yb3c+PHJvdyByPSIzIj48YyByPSJBMyIgdD0iaW5saW5lU3RyIj48aXM+PHQ+TVJOLTAwMTwvdD48L2lzPjwvYz48YyByPSJCMyIgdD0iaW5saW5lU3RyIj48aXM+PHQ+QWxpY2UgU21pdGg8L3Q+PC9pcz48L2M+PC9yb3c+PC9zaGVldERhdGE+PC93b3Jrc2hlZXQ+UEsBAhQDFAAAAAAAeambXLb72pyuAgAArgIAABMAAAAAAAAAAAAAAIABAAAAAFtDb250ZW50X1R5cGVzXS54bWxQSwECFAMUAAAAAAB5qZtcfm/AhSoBAAAqAQAACwAAAAAAAAAAAAAAgAHfAgAAX3JlbHMvLnJlbHNQSwECFAMUAAAAAAB5qZtcd0D+xBwBAAAcAQAADwAAAAAAAAAAAAAAgAEyBAAAeGwvd29ya2Jvb2sueG1sUEsBAhQDFAAAAAAAeambXB+qsIOrAQAAqwEAABoAAAAAAAAAAAAAAIABewUAAHhsL19yZWxzL3dvcmtib29rLnhtbC5yZWxzUEsBAhQDFAAAAAAAeambXL2k1bb0AQAA9AEAAA0AAAAAAAAAAAAAAIABXgcAAHhsL3N0eWxlcy54bWxQSwECFAMUAAAAAAB5qZtcnKJslR0CAAAdAgAAGAAAAAAAAAAAAAAAgAF9CQAAeGwvd29ya3NoZWV0cy9zaGVldDEueG1sUEsFBgAAAAAGAAYAgAEAANALAAAAAA==";

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
async fn tabular_deidentify_endpoint_returns_rewritten_csv_and_summary() {
    let app = build_router(RuntimeState::default());
    let request = json!({
        "csv": "patient_id,patient_name\nMRN-001,Alice Smith\nMRN-001,Alice Smith\n",
        "policies": [
            {
                "header": "patient_id",
                "phi_type": "patient_id",
                "action": "encode"
            },
            {
                "header": "patient_name",
                "phi_type": "patient_name",
                "action": "review"
            }
        ]
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/tabular/deidentify")
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert!(json["csv"].as_str().is_some());
    assert!(json["summary"].is_object());
    assert!(json["review_queue"].is_array());

    let rewritten_csv = json["csv"].as_str().unwrap();
    let lines = rewritten_csv
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .collect::<Vec<_>>();
    assert_eq!(lines[0], "patient_id,patient_name");
    assert_eq!(lines[1], lines[2]);
    assert!(lines[1].starts_with("tok-"));
    assert!(lines[1].contains(",Alice Smith"));
    assert!(!lines[1].contains("MRN-001"));

    let review_queue = json["review_queue"].as_array().unwrap();
    assert_eq!(review_queue.len(), 2);
    assert!(review_queue
        .iter()
        .all(|candidate| candidate["value"] == "Alice Smith"));

    assert_eq!(json["summary"]["total_rows"], 2);
    assert_eq!(json["summary"]["encoded_cells"], 2);
    assert_eq!(json["summary"]["review_required_cells"], 2);
    assert_eq!(json["summary"]["failed_rows"], 0);
}

#[tokio::test]
async fn tabular_deidentify_endpoint_rejects_invalid_policy_payload() {
    let app = build_router(RuntimeState::default());
    let request = json!({
        "csv": "patient_id\nMRN-001\n",
        "policies": [
            {
                "header": "patient_id",
                "phi_type": "patient_id",
                "action": "totally_invalid"
            }
        ]
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/tabular/deidentify")
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_invalid_tabular_request_response(response).await;
}

#[tokio::test]
async fn tabular_xlsx_deidentify_endpoint_returns_rewritten_workbook_and_summary() {
    let app = build_router(RuntimeState::default());
    let workbook = workbook_with_named_sheets(
        "Cover",
        vec![],
        "Patients",
        vec![
            vec!["patient_id", "patient_name"],
            vec!["MRN-001", "Alice Smith"],
            vec!["MRN-001", "Alice Smith"],
        ],
        Some(("Notes", vec![vec!["status"], vec!["keep me"]])),
    );
    let request = json!({
        "workbook_base64": STANDARD.encode(&workbook),
        "field_policies": [
            {
                "header": "patient_id",
                "phi_type": "patient_id",
                "action": "encode"
            },
            {
                "header": "patient_name",
                "phi_type": "patient_name",
                "action": "review"
            }
        ]
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/tabular/deidentify/xlsx")
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["rewritten_workbook_base64"].as_str().is_some());
    assert!(json["summary"].is_object());
    assert!(json["review_queue"].is_array());
    assert!(json.get("csv").is_none());

    let rewritten_workbook = STANDARD
        .decode(json["rewritten_workbook_base64"].as_str().unwrap())
        .unwrap();
    let mut workbook =
        open_workbook_from_rs::<Xlsx<_>, _>(Cursor::new(&rewritten_workbook)).unwrap();
    assert_eq!(
        workbook.sheet_names(),
        vec![
            "Cover".to_string(),
            "Patients".to_string(),
            "Notes".to_string()
        ]
    );

    let notes_rows = worksheet_rows(workbook.worksheet_range("Notes").unwrap());
    assert_eq!(
        notes_rows,
        vec![vec!["status".to_string()], vec!["keep me".to_string()]]
    );

    let patient_rows = worksheet_rows(workbook.worksheet_range("Patients").unwrap());
    assert_eq!(
        patient_rows[0],
        vec!["patient_id".to_string(), "patient_name".to_string()]
    );
    assert_eq!(patient_rows.len(), 3);
    assert_eq!(patient_rows[1], patient_rows[2]);
    assert!(patient_rows[1][0].starts_with("tok-"));
    assert_eq!(patient_rows[1][1], "Alice Smith");
    assert_ne!(patient_rows[1][0], "MRN-001");

    let extracted = XlsxTabularAdapter::new(Vec::new())
        .extract(&rewritten_workbook)
        .expect("rewritten workbook should remain parseable");

    assert_eq!(
        extracted
            .columns
            .iter()
            .map(|column| column.name.as_str())
            .collect::<Vec<_>>(),
        vec!["patient_id", "patient_name"]
    );
    assert_eq!(extracted.rows.len(), 2);
    assert_eq!(extracted.rows[0], extracted.rows[1]);
    assert!(extracted.rows[0][0].starts_with("tok-"));
    assert_eq!(extracted.rows[0][1], "Alice Smith");
    assert_ne!(extracted.rows[0][0], "MRN-001");

    assert_eq!(json["summary"]["total_rows"], 2);
    assert_eq!(json["summary"]["encoded_cells"], 2);
    assert_eq!(json["summary"]["review_required_cells"], 2);
    assert_eq!(json["summary"]["failed_rows"], 0);
    assert_eq!(json["review_queue"].as_array().unwrap().len(), 2);
    assert_eq!(json["xlsx_disclosure"]["selected_sheet_name"], "Patients");
    assert_eq!(json["xlsx_disclosure"]["selected_sheet_index"], 1);
    assert_eq!(json["xlsx_disclosure"]["total_sheet_count"], 3);
    assert_eq!(
        json["xlsx_disclosure"]["disclosure"],
        "XLSX processing used the first non-empty worksheet; other worksheets were not processed."
    );
}

#[tokio::test]
async fn tabular_xlsx_deidentify_endpoint_preserves_selected_sheet_fidelity() {
    let app = build_router(RuntimeState::default());
    let workbook = workbook_with_selected_sheet_extras();
    let original_notes_xml = read_workbook_entry(&workbook, "xl/worksheets/sheet3.xml");
    let original_patients_xml = read_workbook_entry(&workbook, "xl/worksheets/sheet2.xml");
    assert!(
        original_patients_xml.contains("<v>42</v>"),
        "{original_patients_xml}"
    );

    let request = json!({
        "workbook_base64": STANDARD.encode(&workbook),
        "field_policies": [
            {
                "header": "patient_id",
                "phi_type": "patient_id",
                "action": "encode"
            }
        ]
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/tabular/deidentify/xlsx")
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let rewritten_workbook = STANDARD
        .decode(json["rewritten_workbook_base64"].as_str().unwrap())
        .unwrap();

    let patients_xml = read_workbook_entry(&rewritten_workbook, "xl/worksheets/sheet2.xml");
    assert!(patients_xml.contains("r=\"C2\""), "{patients_xml}");
    assert!(patients_xml.contains("<v>42</v>"), "{patients_xml}");
    assert!(patients_xml.contains("s=\"0\""), "{patients_xml}");
    assert!(patients_xml.contains("r=\"D5\""), "{patients_xml}");
    assert!(patients_xml.contains("<f>SUM(C2,8)</f>"), "{patients_xml}");
    assert!(patients_xml.contains("<v>50</v>"), "{patients_xml}");
    assert!(patients_xml.contains("status note"), "{patients_xml}");

    let notes_xml = read_workbook_entry(&rewritten_workbook, "xl/worksheets/sheet3.xml");
    assert_eq!(notes_xml, original_notes_xml);

    let mut workbook =
        open_workbook_from_rs::<Xlsx<_>, _>(Cursor::new(&rewritten_workbook)).unwrap();
    let patient_rows = worksheet_rows(workbook.worksheet_range("Patients").unwrap());
    assert_eq!(patient_rows[1][2], "42");
    assert_eq!(patient_rows[4][3], "50");
}

#[tokio::test]
async fn tabular_xlsx_deidentify_endpoint_rejects_invalid_payloads() {
    let app = build_router(RuntimeState::default());

    let malformed_json_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/tabular/deidentify/xlsx")
                .header("content-type", "application/json")
                .body(Body::from("{"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_invalid_tabular_xlsx_request_response(malformed_json_response).await;

    let missing_fields_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/tabular/deidentify/xlsx")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"workbook_base64": SAMPLE_XLSX_WORKBOOK_BASE64}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_invalid_tabular_xlsx_request_response(missing_fields_response).await;

    let malformed_base64_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/tabular/deidentify/xlsx")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "workbook_base64": "%%%not-base64%%%",
                        "field_policies": [{
                            "header": "patient_id",
                            "phi_type": "patient_id",
                            "action": "encode"
                        }]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_invalid_tabular_xlsx_request_response(malformed_base64_response).await;

    let invalid_workbook_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/tabular/deidentify/xlsx")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "workbook_base64": STANDARD.encode(b"not-an-xlsx"),
                        "field_policies": [{
                            "header": "patient_id",
                            "phi_type": "patient_id",
                            "action": "encode"
                        }]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_invalid_tabular_xlsx_request_response(invalid_workbook_response).await;
}

#[tokio::test]
async fn conservative_media_deidentify_endpoint_routes_image_metadata_to_review() {
    let app = build_router(RuntimeState::default());
    let request = json!({
        "artifact_label": "patient-jane-face.jpg",
        "format": "image",
        "metadata": [
            {"key": "CameraOwner", "value": "Jane Patient"},
            {"key": "Description", "value": "MRN-001 face image"}
        ],
        "ocr_or_visual_review_required": true
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/media/conservative/deidentify")
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["summary"]["total_items"], 1);
    assert_eq!(json["summary"]["metadata_only_items"], 0);
    assert_eq!(json["summary"]["visual_review_required_items"], 1);
    assert_eq!(json["summary"]["unsupported_items"], 0);
    assert_eq!(json["summary"]["review_required_candidates"], 2);
    assert_eq!(json["review_queue"].as_array().unwrap().len(), 2);
    assert_eq!(json["review_queue"][0]["format"], "image");
    assert_eq!(json["review_queue"][0]["phi_type"], "metadata_identifier");
    assert_eq!(
        json["review_queue"][0]["status"],
        "ocr_or_visual_review_required"
    );
    assert_eq!(json["rewritten_media_bytes_base64"], Value::Null);
}

#[tokio::test]
async fn conservative_media_deidentify_endpoint_reports_unsupported_payload_without_candidates() {
    let app = build_router(RuntimeState::default());
    let request = json!({
        "artifact_label": "patient-jane-video.mov",
        "format": "video",
        "metadata": [
            {"key": "CameraOwner", "value": "Jane Patient"}
        ],
        "ocr_or_visual_review_required": false,
        "unsupported_payload": true
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/media/conservative/deidentify")
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["summary"]["total_items"], 1);
    assert_eq!(json["summary"]["unsupported_items"], 1);
    assert_eq!(json["summary"]["review_required_candidates"], 0);
    assert!(json["review_queue"].as_array().unwrap().is_empty());
    assert_eq!(json["rewritten_media_bytes_base64"], Value::Null);
}

#[tokio::test]
async fn conservative_media_deidentify_endpoint_rejects_blank_artifact_label() {
    let app = build_router(RuntimeState::default());
    let request = json!({
        "artifact_label": "   ",
        "format": "image",
        "metadata": [{"key": "CameraOwner", "value": "Jane Patient"}]
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/media/conservative/deidentify")
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"]["code"], "invalid_conservative_media_request");
    assert!(json.get("summary").is_none());
    assert!(json.get("review_queue").is_none());
    assert!(json.get("rewritten_media_bytes_base64").is_none());
}

#[tokio::test]
async fn pdf_deidentify_endpoint_routes_text_layer_candidates_to_review_without_rewrite() {
    let app = build_router(RuntimeState::default());
    let request = json!({
        "pdf_bytes_base64": STANDARD.encode(include_bytes!("../../mdid-adapters/tests/fixtures/pdf/text-layer-minimal.pdf")),
        "source_name": "Alice Smith MRN-001 intake.pdf"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/pdf/deidentify")
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["summary"]["total_pages"], 1);
    assert_eq!(json["summary"]["text_layer_pages"], 1);
    assert_eq!(json["summary"]["ocr_required_pages"], 0);
    assert_eq!(json["summary"]["extracted_candidates"], 1);
    assert_eq!(json["summary"]["review_required_candidates"], 1);
    assert_eq!(json["page_statuses"].as_array().unwrap().len(), 1);
    assert_eq!(json["page_statuses"][0]["status"], "text_layer_present");
    assert_eq!(json["review_queue"].as_array().unwrap().len(), 1);
    assert_eq!(json["review_queue"][0]["phi_type"], "extracted_text");
    assert_eq!(json["review_queue"][0]["decision"], "needs_review");
    assert_eq!(json["rewritten_pdf_bytes_base64"], Value::Null);
}

#[tokio::test]
async fn pdf_deidentify_endpoint_reports_ocr_required_without_fabricating_candidates() {
    let app = build_router(RuntimeState::default());
    let request = json!({
        "pdf_bytes_base64": STANDARD.encode(include_bytes!("../../mdid-adapters/tests/fixtures/pdf/no-text-minimal.pdf")),
        "source_name": "scan needing OCR.pdf"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/pdf/deidentify")
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["summary"]["total_pages"], 1);
    assert_eq!(json["summary"]["text_layer_pages"], 0);
    assert_eq!(json["summary"]["ocr_required_pages"], 1);
    assert_eq!(json["summary"]["extracted_candidates"], 0);
    assert_eq!(json["summary"]["review_required_candidates"], 0);
    assert_eq!(json["page_statuses"].as_array().unwrap().len(), 1);
    assert_eq!(json["page_statuses"][0]["status"], "ocr_required");
    assert!(json["review_queue"].as_array().unwrap().is_empty());
    assert_eq!(json["rewritten_pdf_bytes_base64"], Value::Null);
}

#[tokio::test]
async fn pdf_deidentify_endpoint_rejects_invalid_pdf_bytes() {
    let app = build_router(RuntimeState::default());
    let request = json!({
        "pdf_bytes_base64": STANDARD.encode(b"not-a-pdf-payload"),
        "source_name": "broken.pdf"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/pdf/deidentify")
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_invalid_pdf_response(response).await;
}

#[tokio::test]
async fn pdf_deidentify_endpoint_rejects_malformed_base64_payload() {
    let app = build_router(RuntimeState::default());
    let request = json!({
        "pdf_bytes_base64": "%%%not-base64%%%",
        "source_name": "broken.pdf"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/pdf/deidentify")
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_invalid_pdf_response(response).await;
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
async fn vault_decode_endpoint_rejects_duplicate_record_ids_with_phi_safe_bad_request() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("runtime-vault.mdid");
    let _vault = LocalVaultStore::create(&vault_path, "correct horse battery staple").unwrap();
    let duplicate = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();

    let app = build_router(RuntimeState::default());
    let request = json!({
        "vault_path": vault_path,
        "vault_passphrase": "correct horse battery staple",
        "record_ids": [duplicate, duplicate],
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

    assert_duplicate_record_id_bad_request_response(response, &["values", "audit_event"]).await;
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

#[tokio::test]
async fn audit_events_endpoint_returns_filtered_events_in_reverse_chronological_order() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("runtime-vault.mdid");
    let mut vault = LocalVaultStore::create(&vault_path, "correct horse battery staple").unwrap();
    let first = vault
        .store_mapping(
            NewMappingRecord {
                scope: sample_scope("patient.name"),
                phi_type: "patient_name".into(),
                original_value: "Alice Smith".into(),
            },
            SurfaceKind::Browser,
        )
        .unwrap();
    let second = vault
        .store_mapping(
            NewMappingRecord {
                scope: sample_scope("patient.id"),
                phi_type: "patient_id".into(),
                original_value: "MRN-001".into(),
            },
            SurfaceKind::Cli,
        )
        .unwrap();
    vault
        .decode(
            mdid_domain::DecodeRequest::new(
                vec![first.id],
                "investigator export".into(),
                "incident review".into(),
                SurfaceKind::Desktop,
            )
            .unwrap(),
        )
        .unwrap();
    vault
        .export_portable(
            &[second.id],
            "partner-passphrase",
            SurfaceKind::Desktop,
            "partner-site transfer package",
        )
        .unwrap();

    let app = build_router(RuntimeState::default());
    let request = json!({
        "vault_path": vault_path,
        "vault_passphrase": "correct horse battery staple",
        "actor": "desktop",
        "limit": 2
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/vault/audit/events")
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let events = json["events"].as_array().unwrap();
    assert_eq!(events.len(), 2);

    assert_eq!(events[0]["kind"], "export");
    assert_eq!(events[0]["actor"], "desktop");
    assert!(events[0]["detail"]
        .as_str()
        .unwrap()
        .contains("partner-site transfer package"));
    assert!(events[0]["recorded_at"].as_str().is_some());

    assert_eq!(events[1]["kind"], "decode");
    assert_eq!(events[1]["actor"], "desktop");
    assert!(events[1]["detail"]
        .as_str()
        .unwrap()
        .contains("incident review"));
    assert!(events[1]["recorded_at"].as_str().is_some());

    let first_timestamp = events[0]["recorded_at"].as_str().unwrap();
    let second_timestamp = events[1]["recorded_at"].as_str().unwrap();
    assert!(first_timestamp >= second_timestamp);

    let kind_filtered_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/vault/audit/events")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "vault_path": vault_path,
                        "vault_passphrase": "correct horse battery staple",
                        "kind": "encode",
                        "actor": "cli",
                        "limit": 10
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(kind_filtered_response.status(), StatusCode::OK);
    let body = to_bytes(kind_filtered_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let events = json["events"].as_array().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["kind"], "encode");
    assert_eq!(events[0]["actor"], "cli");
    assert!(events[0]["detail"].as_str().unwrap().contains("patient.id"));
}

#[tokio::test]
async fn audit_events_endpoint_rejects_wrong_passphrase() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("runtime-vault.mdid");
    let mut vault = LocalVaultStore::create(&vault_path, "correct horse battery staple").unwrap();
    vault
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
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/vault/audit/events")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "vault_path": vault_path,
                        "vault_passphrase": "wrong passphrase"
                    })
                    .to_string(),
                ))
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
    assert!(json.get("events").is_none());
}

#[tokio::test]
async fn audit_events_endpoint_rejects_invalid_filter_payload() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("runtime-vault.mdid");
    let mut vault = LocalVaultStore::create(&vault_path, "correct horse battery staple").unwrap();
    vault
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
    let bad_limit_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/vault/audit/events")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "vault_path": vault_path,
                        "vault_passphrase": "correct horse battery staple",
                        "limit": 0
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_invalid_audit_events_request_response(bad_limit_response).await;

    let blank_passphrase_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/vault/audit/events")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "vault_path": vault_path,
                        "vault_passphrase": ""
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_invalid_audit_events_request_response(blank_passphrase_response).await;

    let bad_enum_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/vault/audit/events")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "vault_path": vault_path,
                        "vault_passphrase": "correct horse battery staple",
                        "kind": "totally_invalid"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_invalid_audit_events_request_response(bad_enum_response).await;
}

#[tokio::test]
async fn vault_export_endpoint_returns_portable_artifact_and_records_audit_event() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("runtime-vault.mdid");
    let mut vault = LocalVaultStore::create(&vault_path, "correct horse battery staple").unwrap();
    let kept = vault
        .store_mapping(
            NewMappingRecord {
                scope: sample_scope("patient.name"),
                phi_type: "patient_name".into(),
                original_value: "Alice Smith".into(),
            },
            SurfaceKind::Browser,
        )
        .unwrap();
    let omitted = vault
        .store_mapping(
            NewMappingRecord {
                scope: sample_scope("patient.id"),
                phi_type: "patient_id".into(),
                original_value: "MRN-001".into(),
            },
            SurfaceKind::Browser,
        )
        .unwrap();

    let app = build_router(RuntimeState::default());
    let request = json!({
        "vault_path": vault_path,
        "vault_passphrase": "correct horse battery staple",
        "record_ids": [kept.id],
        "export_passphrase": "portable-passphrase",
        "context": "partner-site transfer package",
        "requested_by": "desktop"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/vault/export")
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let artifact_json = json
        .get("artifact")
        .cloned()
        .expect("artifact should be present");
    assert!(json.get("audit_event").is_none());

    let artifact: PortableVaultArtifact = serde_json::from_value(artifact_json).unwrap();
    let snapshot = artifact.unlock("portable-passphrase").unwrap();
    assert_eq!(snapshot.records.len(), 1);
    assert_eq!(snapshot.records[0].id, kept.id);
    assert_eq!(snapshot.records[0].original_value, "Alice Smith");
    assert_eq!(snapshot.records[0].token, kept.token);
    assert!(snapshot
        .records
        .iter()
        .all(|record| record.id != omitted.id));

    let reopened = LocalVaultStore::unlock(&vault_path, "correct horse battery staple").unwrap();
    let audit_events = reopened.audit_events();
    let export_event = audit_events
        .last()
        .expect("export event should be recorded");
    assert_eq!(export_event.kind.as_str(), "export");
    assert_eq!(export_event.actor, SurfaceKind::Desktop);
    assert!(export_event
        .detail
        .contains("partner-site transfer package"));
    assert!(export_event.detail.contains("1 record"));
    assert!(export_event.detail.contains(&kept.id.to_string()));
    assert!(!export_event.detail.contains(&omitted.id.to_string()));
}

#[tokio::test]
async fn vault_export_endpoint_rejects_unknown_record_scope() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("runtime-vault.mdid");
    let _vault = LocalVaultStore::create(&vault_path, "correct horse battery staple").unwrap();

    let app = build_router(RuntimeState::default());
    let request = json!({
        "vault_path": vault_path,
        "vault_passphrase": "correct horse battery staple",
        "record_ids": [Uuid::new_v4()],
        "export_passphrase": "portable-passphrase",
        "context": "partner-site transfer package",
        "requested_by": "desktop"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/vault/export")
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
                "message": "export scope referenced a record that does not exist"
            }
        })
    );
    assert!(json.get("artifact").is_none());
}

#[tokio::test]
async fn vault_export_endpoint_rejects_unusable_vault_target() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("not-a-vault.mdid");
    std::fs::write(&vault_path, "not valid vault json").unwrap();

    let app = build_router(RuntimeState::default());
    let request = json!({
        "vault_path": vault_path,
        "vault_passphrase": "correct horse battery staple",
        "record_ids": [Uuid::new_v4()],
        "export_passphrase": "portable-passphrase",
        "context": "partner-site transfer package",
        "requested_by": "desktop"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/vault/export")
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_invalid_vault_target_response(response).await;
}

#[tokio::test]
async fn vault_export_endpoint_rejects_corrupted_encrypted_vault_artifact() {
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
        "export_passphrase": "portable-passphrase",
        "context": "partner-site transfer package",
        "requested_by": "desktop"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/vault/export")
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_invalid_vault_target_response(response).await;
}

#[tokio::test]
async fn vault_export_endpoint_rejects_invalid_export_payload() {
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
    for request in [
        json!({
            "vault_path": vault_path,
            "vault_passphrase": "correct horse battery staple",
            "record_ids": [],
            "export_passphrase": "portable-passphrase",
            "context": "partner-site transfer package",
            "requested_by": "desktop"
        }),
        json!({
            "vault_path": vault_path,
            "vault_passphrase": "correct horse battery staple",
            "record_ids": [stored.id],
            "export_passphrase": "portable-passphrase",
            "context": "   ",
            "requested_by": "desktop"
        }),
        json!({
            "vault_path": vault_path,
            "vault_passphrase": "correct horse battery staple",
            "record_ids": [stored.id],
            "export_passphrase": " ",
            "context": "partner-site transfer package",
            "requested_by": "desktop"
        }),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/vault/export")
                    .header("content-type", "application/json")
                    .body(Body::from(request.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_invalid_export_request_response(response).await;
    }
}

#[tokio::test]
async fn vault_export_endpoint_rejects_duplicate_record_ids_with_phi_safe_bad_request() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("runtime-vault.mdid");
    let _vault = LocalVaultStore::create(&vault_path, "correct horse battery staple").unwrap();
    let duplicate = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();

    let app = build_router(RuntimeState::default());
    let request = json!({
        "vault_path": vault_path,
        "vault_passphrase": "correct horse battery staple",
        "record_ids": [duplicate, duplicate],
        "export_passphrase": "portable-passphrase",
        "context": "partner-site transfer package",
        "requested_by": "desktop"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/vault/export")
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_duplicate_record_id_bad_request_response(response, &["artifact"]).await;
}

#[tokio::test]
async fn vault_export_endpoint_rejects_wrong_passphrase() {
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
        "vault_passphrase": "wrong passphrase",
        "record_ids": [stored.id],
        "export_passphrase": "portable-passphrase",
        "context": "partner-site transfer package",
        "requested_by": "desktop"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/vault/export")
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
    assert!(json.get("artifact").is_none());
}

#[tokio::test]
async fn portable_artifact_inspect_endpoint_returns_bounded_snapshot_summary() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("runtime-vault.mdid");
    let mut vault = LocalVaultStore::create(&vault_path, "correct horse battery staple").unwrap();
    let first = vault
        .store_mapping(
            NewMappingRecord {
                scope: sample_scope("patient.name"),
                phi_type: "patient_name".into(),
                original_value: "Alice Smith".into(),
            },
            SurfaceKind::Browser,
        )
        .unwrap();
    let second = vault
        .store_mapping(
            NewMappingRecord {
                scope: sample_scope("patient.id"),
                phi_type: "patient_id".into(),
                original_value: "MRN-001".into(),
            },
            SurfaceKind::Browser,
        )
        .unwrap();
    let artifact = vault
        .export_portable(
            &[first.id, second.id],
            "portable-passphrase",
            SurfaceKind::Desktop,
            "partner-site transfer package",
        )
        .unwrap();

    let app = build_router(RuntimeState::default());
    let request = json!({
        "artifact": artifact,
        "portable_passphrase": "portable-passphrase"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/portable-artifacts/inspect")
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["record_count"], 2);
    let records = json["records"].as_array().unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0]["id"], first.id.to_string());
    assert_eq!(records[0]["scope"]["field_path"], first.scope.field_path);
    assert_eq!(records[0]["phi_type"], first.phi_type);
    assert_eq!(records[0]["token"], first.token);
    assert_eq!(records[0]["original_value"], first.original_value);
    assert_eq!(
        records[0]["created_at"]
            .as_str()
            .unwrap()
            .parse::<chrono::DateTime<chrono::Utc>>()
            .unwrap(),
        first.created_at
    );
    assert_eq!(records[1]["id"], second.id.to_string());
    assert_eq!(records[1]["scope"]["field_path"], second.scope.field_path);
    assert_eq!(records[1]["phi_type"], second.phi_type);
    assert_eq!(records[1]["token"], second.token);
    assert_eq!(records[1]["original_value"], second.original_value);
    assert_eq!(
        records[1]["created_at"]
            .as_str()
            .unwrap()
            .parse::<chrono::DateTime<chrono::Utc>>()
            .unwrap(),
        second.created_at
    );
    assert!(records[0].get("audit_events").is_none());
    assert!(json.get("artifact").is_none());
    assert!(json.get("audit_event").is_none());
}

#[tokio::test]
async fn portable_artifact_inspect_endpoint_rejects_invalid_request_payload() {
    let app = build_router(RuntimeState::default());

    for request in [
        json!({"portable_passphrase": "portable-passphrase"}),
        json!({"artifact": {}, "portable_passphrase": "   "}),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/portable-artifacts/inspect")
                    .header("content-type", "application/json")
                    .body(Body::from(request.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_invalid_portable_artifact_inspection_request_response(response).await;
    }
}

#[tokio::test]
async fn portable_artifact_inspect_endpoint_rejects_wrong_portable_passphrase() {
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
    let artifact = vault
        .export_portable(
            &[stored.id],
            "portable-passphrase",
            SurfaceKind::Desktop,
            "partner-site transfer package",
        )
        .unwrap();

    let app = build_router(RuntimeState::default());
    let request = json!({
        "artifact": artifact,
        "portable_passphrase": "wrong passphrase"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/portable-artifacts/inspect")
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
                "code": "portable_artifact_unlock_failed",
                "message": "portable artifact could not be unlocked with the supplied passphrase"
            }
        })
    );
    assert!(json.get("records").is_none());
}

#[tokio::test]
async fn portable_artifact_inspect_endpoint_rejects_corrupted_artifact() {
    let app = build_router(RuntimeState::default());
    let request = json!({
        "artifact": {
            "salt_b64": "%%%not-base64%%%",
            "nonce_b64": "still-not-base64",
            "ciphertext_b64": "broken"
        },
        "portable_passphrase": "portable-passphrase"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/portable-artifacts/inspect")
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_invalid_portable_artifact_response(response).await;
}

#[tokio::test]
async fn portable_artifact_import_endpoint_returns_bounded_import_summary_and_audit_event() {
    let dir = tempdir().unwrap();
    let export_vault_path = dir.path().join("export-vault.mdid");
    let mut export_vault =
        LocalVaultStore::create(&export_vault_path, "correct horse battery staple").unwrap();
    let first = export_vault
        .store_mapping(
            NewMappingRecord {
                scope: sample_scope("patient.name"),
                phi_type: "patient_name".into(),
                original_value: "Alice Smith".into(),
            },
            SurfaceKind::Browser,
        )
        .unwrap();
    let second = export_vault
        .store_mapping(
            NewMappingRecord {
                scope: sample_scope("patient.id"),
                phi_type: "patient_id".into(),
                original_value: "MRN-001".into(),
            },
            SurfaceKind::Browser,
        )
        .unwrap();
    let artifact = export_vault
        .export_portable(
            &[first.id, second.id],
            "portable-passphrase",
            SurfaceKind::Desktop,
            "partner-site transfer package",
        )
        .unwrap();

    let import_vault_path = dir.path().join("import-vault.mdid");
    let mut import_vault =
        LocalVaultStore::create(&import_vault_path, "correct horse battery staple").unwrap();
    let duplicate_seed = import_vault
        .store_mapping(
            NewMappingRecord {
                scope: second.scope.clone(),
                phi_type: second.phi_type.clone(),
                original_value: second.original_value.clone(),
            },
            SurfaceKind::Desktop,
        )
        .unwrap();

    let app = build_router(RuntimeState::default());
    let request = json!({
        "vault_path": import_vault_path,
        "vault_passphrase": "correct horse battery staple",
        "artifact": artifact,
        "portable_passphrase": "portable-passphrase",
        "context": "runtime import into local vault",
        "requested_by": "desktop"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/portable-artifacts/import")
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["imported_record_count"], 1);
    assert_eq!(json["duplicate_record_count"], 1);
    assert_eq!(json["audit_event"]["kind"], "import");
    assert_eq!(json["audit_event"]["actor"], "desktop");
    let detail = json["audit_event"]["detail"].as_str().unwrap();
    assert!(detail.contains("runtime import into local vault"));
    assert!(detail.contains("imported 1 record"));
    assert!(detail.contains("with 1 duplicate"));
    assert!(detail.contains(&first.id.to_string()));
    assert!(detail.contains(&second.id.to_string()));
    assert!(json.get("artifact").is_none());
    assert!(json.get("records").is_none());

    let mut unlocked =
        LocalVaultStore::unlock(&import_vault_path, "correct horse battery staple").unwrap();
    let audit_events = unlocked.audit_events();
    assert!(audit_events
        .iter()
        .any(|event| event.kind == mdid_domain::AuditEventKind::Import));
    assert_eq!(
        unlocked
            .decode(
                mdid_domain::DecodeRequest::new(
                    vec![first.id, duplicate_seed.id],
                    "verification target".into(),
                    "verify import route".into(),
                    SurfaceKind::Desktop,
                )
                .unwrap(),
            )
            .unwrap()
            .values
            .len(),
        2
    );
}

#[tokio::test]
async fn portable_artifact_import_endpoint_rejects_invalid_request_payload() {
    let app = build_router(RuntimeState::default());

    for request in [
        json!({
            "vault_passphrase": "correct horse battery staple",
            "portable_passphrase": "portable-passphrase",
            "context": "runtime import into local vault",
            "requested_by": "desktop"
        }),
        json!({
            "vault_path": "/tmp/runtime-vault.mdid",
            "vault_passphrase": "   ",
            "artifact": {},
            "portable_passphrase": "portable-passphrase",
            "context": "runtime import into local vault",
            "requested_by": "desktop"
        }),
        json!({
            "vault_path": "/tmp/runtime-vault.mdid",
            "vault_passphrase": "correct horse battery staple",
            "artifact": {},
            "portable_passphrase": "   ",
            "context": "runtime import into local vault",
            "requested_by": "desktop"
        }),
        json!({
            "vault_path": "/tmp/runtime-vault.mdid",
            "vault_passphrase": "correct horse battery staple",
            "artifact": {},
            "portable_passphrase": "portable-passphrase",
            "context": "   ",
            "requested_by": "desktop"
        }),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/portable-artifacts/import")
                    .header("content-type", "application/json")
                    .body(Body::from(request.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_invalid_portable_artifact_import_request_response(response).await;
    }
}

#[tokio::test]
async fn portable_artifact_import_endpoint_rejects_wrong_vault_passphrase() {
    let dir = tempdir().unwrap();
    let export_vault_path = dir.path().join("export-vault.mdid");
    let mut export_vault =
        LocalVaultStore::create(&export_vault_path, "correct horse battery staple").unwrap();
    let stored = export_vault
        .store_mapping(
            NewMappingRecord {
                scope: sample_scope("patient.name"),
                phi_type: "patient_name".into(),
                original_value: "Alice Smith".into(),
            },
            SurfaceKind::Browser,
        )
        .unwrap();
    let artifact = export_vault
        .export_portable(
            &[stored.id],
            "portable-passphrase",
            SurfaceKind::Desktop,
            "partner-site transfer package",
        )
        .unwrap();

    let import_vault_path = dir.path().join("import-vault.mdid");
    let _import_vault =
        LocalVaultStore::create(&import_vault_path, "correct horse battery staple").unwrap();

    let app = build_router(RuntimeState::default());
    let request = json!({
        "vault_path": import_vault_path,
        "vault_passphrase": "wrong passphrase",
        "artifact": artifact,
        "portable_passphrase": "portable-passphrase",
        "context": "runtime import into local vault",
        "requested_by": "desktop"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/portable-artifacts/import")
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
    assert!(json.get("imported_record_count").is_none());
    assert!(json.get("duplicate_record_count").is_none());
    assert!(json.get("audit_event").is_none());
}

#[tokio::test]
async fn portable_artifact_import_endpoint_rejects_wrong_portable_passphrase() {
    let dir = tempdir().unwrap();
    let export_vault_path = dir.path().join("export-vault.mdid");
    let mut export_vault =
        LocalVaultStore::create(&export_vault_path, "correct horse battery staple").unwrap();
    let stored = export_vault
        .store_mapping(
            NewMappingRecord {
                scope: sample_scope("patient.name"),
                phi_type: "patient_name".into(),
                original_value: "Alice Smith".into(),
            },
            SurfaceKind::Browser,
        )
        .unwrap();
    let artifact = export_vault
        .export_portable(
            &[stored.id],
            "portable-passphrase",
            SurfaceKind::Desktop,
            "partner-site transfer package",
        )
        .unwrap();

    let import_vault_path = dir.path().join("import-vault.mdid");
    let _import_vault =
        LocalVaultStore::create(&import_vault_path, "correct horse battery staple").unwrap();

    let app = build_router(RuntimeState::default());
    let request = json!({
        "vault_path": import_vault_path,
        "vault_passphrase": "correct horse battery staple",
        "artifact": artifact,
        "portable_passphrase": "wrong passphrase",
        "context": "runtime import into local vault",
        "requested_by": "desktop"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/portable-artifacts/import")
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
                "code": "portable_artifact_unlock_failed",
                "message": "portable artifact could not be unlocked with the supplied passphrase"
            }
        })
    );
    assert!(json.get("imported_record_count").is_none());
    assert!(json.get("duplicate_record_count").is_none());
    assert!(json.get("audit_event").is_none());
}

#[tokio::test]
async fn portable_artifact_import_endpoint_rejects_corrupted_artifact() {
    let dir = tempdir().unwrap();
    let import_vault_path = dir.path().join("import-vault.mdid");
    let _import_vault =
        LocalVaultStore::create(&import_vault_path, "correct horse battery staple").unwrap();

    let app = build_router(RuntimeState::default());
    let request = json!({
        "vault_path": import_vault_path,
        "vault_passphrase": "correct horse battery staple",
        "artifact": {
            "salt_b64": "%%%not-base64%%%",
            "nonce_b64": "still-not-base64",
            "ciphertext_b64": "broken"
        },
        "portable_passphrase": "portable-passphrase",
        "context": "runtime import into local vault",
        "requested_by": "desktop"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/portable-artifacts/import")
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_invalid_portable_artifact_response(response).await;
}

#[tokio::test]
async fn portable_artifact_import_endpoint_rejects_invalid_vault_target() {
    let dir = tempdir().unwrap();
    let export_vault_path = dir.path().join("export-vault.mdid");
    let mut export_vault =
        LocalVaultStore::create(&export_vault_path, "correct horse battery staple").unwrap();
    let stored = export_vault
        .store_mapping(
            NewMappingRecord {
                scope: sample_scope("patient.name"),
                phi_type: "patient_name".into(),
                original_value: "Alice Smith".into(),
            },
            SurfaceKind::Browser,
        )
        .unwrap();
    let artifact = export_vault
        .export_portable(
            &[stored.id],
            "portable-passphrase",
            SurfaceKind::Desktop,
            "partner-site transfer package",
        )
        .unwrap();

    let import_vault_path = dir.path().join("not-a-vault.mdid");
    std::fs::write(&import_vault_path, "not valid vault json").unwrap();

    let app = build_router(RuntimeState::default());
    let request = json!({
        "vault_path": import_vault_path,
        "vault_passphrase": "correct horse battery staple",
        "artifact": artifact,
        "portable_passphrase": "portable-passphrase",
        "context": "runtime import into local vault",
        "requested_by": "desktop"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/portable-artifacts/import")
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_invalid_vault_target_response(response).await;
}

fn workbook_with_selected_sheet_extras() -> Vec<u8> {
    let workbook = workbook_with_named_sheets(
        "Cover",
        vec![],
        "Patients",
        vec![
            vec!["patient_id", "patient_name"],
            vec!["MRN-001", "Alice Smith"],
            vec!["MRN-002", "Bob Jones"],
        ],
        Some(("Notes", vec![vec!["status"], vec!["keep me"]])),
    );

    rewrite_workbook_entry(&workbook, "xl/worksheets/sheet2.xml", |sheet_xml| {
        let mut worksheet = Element::parse(sheet_xml.as_bytes()).unwrap();
        let sheet_data = worksheet.get_mut_child("sheetData").unwrap();

        let row_two = find_row_mut(sheet_data, 2).unwrap();
        row_two
            .children
            .push(XMLNode::Element(number_cell("C2", "42", Some("0"))));
        row_two.children.push(XMLNode::Element(inline_string_cell(
            "D2",
            "status note",
            None,
        )));

        let mut row_five = Element::new("row");
        row_five.attributes.insert("r".into(), "5".into());
        row_five
            .children
            .push(XMLNode::Element(formula_cell("D5", "SUM(C2,8)", "50")));
        sheet_data.children.push(XMLNode::Element(row_five));

        let mut rewritten = Vec::new();
        worksheet.write(&mut rewritten).unwrap();
        String::from_utf8(rewritten).unwrap()
    })
}

fn workbook_with_named_sheets(
    cover_sheet_name: &str,
    cover_rows: Vec<Vec<&str>>,
    data_sheet_name: &str,
    data_rows: Vec<Vec<&str>>,
    trailing_sheet: Option<(&str, Vec<Vec<&str>>)>,
) -> Vec<u8> {
    let mut workbook = Workbook::new();

    let cover = workbook.add_worksheet();
    cover.set_name(cover_sheet_name).unwrap();
    write_worksheet_rows(cover, &cover_rows);

    let data = workbook.add_worksheet();
    data.set_name(data_sheet_name).unwrap();
    write_worksheet_rows(data, &data_rows);

    if let Some((sheet_name, rows)) = trailing_sheet {
        let sheet = workbook.add_worksheet();
        sheet.set_name(sheet_name).unwrap();
        write_worksheet_rows(sheet, &rows);
    }

    workbook.save_to_buffer().unwrap()
}

fn write_worksheet_rows(worksheet: &mut rust_xlsxwriter::Worksheet, rows: &[Vec<&str>]) {
    for (row_index, row) in rows.iter().enumerate() {
        for (column_index, value) in row.iter().enumerate() {
            worksheet
                .write_string(row_index as u32, column_index as u16, *value)
                .unwrap();
        }
    }
}

fn read_workbook_entry(workbook: &[u8], path: &str) -> String {
    let mut archive = ZipArchive::new(Cursor::new(workbook)).unwrap();
    let mut file = archive.by_name(path).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    contents
}

fn rewrite_workbook_entry(
    workbook: &[u8],
    path: &str,
    rewrite: impl FnOnce(String) -> String,
) -> Vec<u8> {
    let mut archive = ZipArchive::new(Cursor::new(workbook)).unwrap();
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let mut rewrite = Some(rewrite);

    for index in 0..archive.len() {
        let mut file = archive.by_index(index).unwrap();
        let mut contents = Vec::new();
        file.read_to_end(&mut contents).unwrap();
        let options = SimpleFileOptions::default().compression_method(file.compression());
        writer.start_file(file.name(), options).unwrap();
        if file.name() == path {
            let rewritten = rewrite.take().unwrap()(String::from_utf8(contents).unwrap());
            writer.write_all(rewritten.as_bytes()).unwrap();
        } else {
            writer.write_all(&contents).unwrap();
        }
    }

    writer.finish().unwrap().into_inner()
}

fn find_row_mut(sheet_data: &mut Element, row_number: u32) -> Option<&mut Element> {
    sheet_data.children.iter_mut().find_map(|node| match node {
        XMLNode::Element(row)
            if row.name == "row"
                && row
                    .attributes
                    .get("r")
                    .and_then(|value| value.parse::<u32>().ok())
                    == Some(row_number) =>
        {
            Some(row)
        }
        _ => None,
    })
}

fn inline_string_cell(reference: &str, value: &str, style: Option<&str>) -> Element {
    let mut cell = Element::new("c");
    cell.attributes.insert("r".into(), reference.into());
    cell.attributes.insert("t".into(), "inlineStr".into());
    if let Some(style) = style {
        cell.attributes.insert("s".into(), style.into());
    }

    let mut text = Element::new("t");
    text.children.push(XMLNode::Text(value.into()));
    let mut inline_string = Element::new("is");
    inline_string.children.push(XMLNode::Element(text));
    cell.children.push(XMLNode::Element(inline_string));
    cell
}

fn number_cell(reference: &str, value: &str, style: Option<&str>) -> Element {
    let mut cell = Element::new("c");
    cell.attributes.insert("r".into(), reference.into());
    if let Some(style) = style {
        cell.attributes.insert("s".into(), style.into());
    }
    let mut cell_value = Element::new("v");
    cell_value.children.push(XMLNode::Text(value.into()));
    cell.children.push(XMLNode::Element(cell_value));
    cell
}

fn formula_cell(reference: &str, formula: &str, value: &str) -> Element {
    let mut cell = Element::new("c");
    cell.attributes.insert("r".into(), reference.into());

    let mut formula_element = Element::new("f");
    formula_element.children.push(XMLNode::Text(formula.into()));
    cell.children.push(XMLNode::Element(formula_element));

    let mut value_element = Element::new("v");
    value_element.children.push(XMLNode::Text(value.into()));
    cell.children.push(XMLNode::Element(value_element));
    cell
}

fn worksheet_rows(range: calamine::Range<Data>) -> Vec<Vec<String>> {
    range
        .rows()
        .map(|row| row.iter().map(cell_to_string).collect::<Vec<_>>())
        .collect()
}

fn cell_to_string(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        other => other.to_string(),
    }
}

async fn assert_invalid_audit_events_request_response(response: axum::response::Response) {
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        json,
        json!({
            "error": {
                "code": "invalid_audit_events_request",
                "message": "request body did not contain a valid vault audit events request"
            }
        })
    );
    assert!(json.get("events").is_none());
}

async fn assert_invalid_tabular_request_response(response: axum::response::Response) {
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        json,
        json!({
            "error": {
                "code": "invalid_tabular_request",
                "message": "request body did not contain a valid tabular deidentification request"
            }
        })
    );
    assert!(json.get("csv").is_none());
    assert!(json.get("summary").is_none());
    assert!(json.get("review_queue").is_none());
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

async fn assert_invalid_pdf_response(response: axum::response::Response) {
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["error"]["code"], "invalid_pdf");
    assert!(json.get("rewritten_pdf_bytes_base64").is_none());
    assert!(json.get("summary").is_none());
    assert!(json.get("page_statuses").is_none());
    assert!(json.get("review_queue").is_none());
}

async fn assert_invalid_tabular_xlsx_request_response(response: axum::response::Response) {
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        json,
        json!({
            "error": {
                "code": "invalid_tabular_xlsx_request",
                "message": "request body did not contain a valid XLSX tabular deidentification request"
            }
        })
    );
    assert!(json.get("rewritten_workbook_base64").is_none());
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

async fn assert_duplicate_record_id_bad_request_response(
    response: axum::response::Response,
    absent_fields: &[&str],
) {
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_text = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_text.contains("duplicate record id"));
    assert!(!body_text.contains("550e8400"));
    let json: Value = serde_json::from_str(&body_text).unwrap();
    assert_eq!(json["error"]["code"], "duplicate_record_id");
    assert_eq!(
        json["error"]["message"],
        "duplicate record id is not allowed"
    );
    for field in absent_fields {
        assert!(json.get(*field).is_none());
    }
}

async fn assert_invalid_export_request_response(response: axum::response::Response) {
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        json,
        json!({
            "error": {
                "code": "invalid_export_request",
                "message": "request body did not contain a valid vault export request"
            }
        })
    );
    assert!(json.get("artifact").is_none());
}

async fn assert_invalid_portable_artifact_inspection_request_response(
    response: axum::response::Response,
) {
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        json,
        json!({
            "error": {
                "code": "invalid_portable_artifact_inspection_request",
                "message": "request body did not contain a valid portable artifact inspection request"
            }
        })
    );
    assert!(json.get("records").is_none());
}

async fn assert_invalid_portable_artifact_response(response: axum::response::Response) {
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        json,
        json!({
            "error": {
                "code": "invalid_portable_artifact",
                "message": "portable artifact could not be read as a usable portable vault artifact"
            }
        })
    );
    assert!(json.get("records").is_none());
}

async fn assert_invalid_portable_artifact_import_request_response(
    response: axum::response::Response,
) {
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        json,
        json!({
            "error": {
                "code": "invalid_portable_artifact_import_request",
                "message": "request body did not contain a valid portable artifact import request"
            }
        })
    );
    assert!(json.get("imported_record_count").is_none());
    assert!(json.get("duplicate_record_count").is_none());
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
