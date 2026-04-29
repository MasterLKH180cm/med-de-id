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
