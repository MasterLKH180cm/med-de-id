use mdid_adapters::{
    ConservativeMediaAdapter, ConservativeMediaAdapterError, ConservativeMediaInput,
    ConservativeMediaMetadataEntry,
};
use mdid_domain::{ConservativeMediaFormat, ConservativeMediaScanStatus};

fn metadata_entry(key: &str, value: &str) -> ConservativeMediaMetadataEntry {
    ConservativeMediaMetadataEntry {
        key: key.to_string(),
        value: value.to_string(),
    }
}

#[test]
fn image_metadata_extraction_routes_metadata_and_visual_review_honestly() {
    let input = ConservativeMediaInput {
        artifact_label: "patients/jane-face.png".to_string(),
        format: ConservativeMediaFormat::Image,
        metadata: vec![
            metadata_entry("EXIF Artist", "Jane Patient"),
            metadata_entry("CameraSerial", "SN-12345"),
        ],
        requires_visual_review: true,
        unsupported_payload: false,
    };

    let output = ConservativeMediaAdapter::extract_metadata(input).unwrap();

    assert_eq!(output.summary.total_items, 1);
    assert_eq!(output.summary.metadata_only_items, 0);
    assert_eq!(output.summary.visual_review_required_items, 1);
    assert_eq!(output.summary.unsupported_items, 0);
    assert_eq!(output.summary.review_required_candidates, 2);
    assert!(output.summary.requires_review());
    assert_eq!(output.candidates.len(), 2);
    assert_eq!(
        output.candidates[0].field_ref.field_path(),
        "media:patients_jane-face.png:EXIF Artist"
    );
    assert_eq!(output.candidates[0].format, ConservativeMediaFormat::Image);
    assert_eq!(output.candidates[0].phi_type, "metadata_identifier");
    assert_eq!(output.candidates[0].source_value, "Jane Patient");
    assert_eq!(output.candidates[0].confidence, 0.35);
    assert_eq!(
        output.candidates[0].status,
        ConservativeMediaScanStatus::OcrOrVisualReviewRequired
    );
}

#[test]
fn fcs_metadata_extraction_stays_metadata_only_without_visual_claims() {
    let input = ConservativeMediaInput {
        artifact_label: "flow/panel.fcs".to_string(),
        format: ConservativeMediaFormat::Fcs,
        metadata: vec![metadata_entry("$FIL", "subject-42.fcs")],
        requires_visual_review: false,
        unsupported_payload: false,
    };

    let output = ConservativeMediaAdapter::extract_metadata(input).unwrap();

    assert_eq!(output.summary.total_items, 1);
    assert_eq!(output.summary.metadata_only_items, 1);
    assert_eq!(output.summary.visual_review_required_items, 0);
    assert_eq!(output.summary.unsupported_items, 0);
    assert_eq!(output.summary.review_required_candidates, 1);
    assert!(output.summary.requires_review());
    assert_eq!(output.candidates[0].format, ConservativeMediaFormat::Fcs);
    assert_eq!(
        output.candidates[0].status,
        ConservativeMediaScanStatus::MetadataOnly
    );
    assert_eq!(output.candidates[0].confidence, 0.35);
}

#[test]
fn unsupported_payload_counts_item_without_fabricating_candidates() {
    let input = ConservativeMediaInput {
        artifact_label: "video/unknown-container.bin".to_string(),
        format: ConservativeMediaFormat::Video,
        metadata: vec![metadata_entry("filename", "patient-walkthrough.mov")],
        requires_visual_review: true,
        unsupported_payload: true,
    };

    let output = ConservativeMediaAdapter::extract_metadata(input).unwrap();

    assert_eq!(output.summary.total_items, 1);
    assert_eq!(output.summary.metadata_only_items, 0);
    assert_eq!(output.summary.visual_review_required_items, 0);
    assert_eq!(output.summary.unsupported_items, 1);
    assert_eq!(output.summary.review_required_candidates, 0);
    assert!(!output.summary.requires_review());
    assert!(output.candidates.is_empty());
}

#[test]
fn extraction_output_debug_redacts_metadata_values() {
    let input = ConservativeMediaInput {
        artifact_label: "patient.png".to_string(),
        format: ConservativeMediaFormat::Image,
        metadata: vec![metadata_entry("Artist", "Jane Patient")],
        requires_visual_review: true,
        unsupported_payload: false,
    };

    let output = ConservativeMediaAdapter::extract_metadata(input).unwrap();
    let debug = format!("{output:?}");

    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("Jane Patient"));
}

#[test]
fn extraction_output_debug_redacts_phi_bearing_artifact_labels() {
    let input = ConservativeMediaInput {
        artifact_label: "patients/Jane-Doe-face.png".to_string(),
        format: ConservativeMediaFormat::Image,
        metadata: vec![metadata_entry("Artist", "Jane Patient")],
        requires_visual_review: true,
        unsupported_payload: false,
    };

    let output = ConservativeMediaAdapter::extract_metadata(input).unwrap();
    let output_debug = format!("{output:?}");
    let candidates_debug = format!("{:?}", output.candidates);
    let candidate_debug = format!("{:?}", output.candidates[0]);

    for debug in [output_debug, candidates_debug, candidate_debug] {
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("Jane-Doe"));
        assert!(!debug.contains("patients/"));
        assert!(!debug.contains("Jane Patient"));
    }
}

#[test]
fn input_debug_redacts_metadata_values_and_artifact_labels() {
    let input = ConservativeMediaInput {
        artifact_label: "patients/Jane-Doe-face.png".to_string(),
        format: ConservativeMediaFormat::Image,
        metadata: vec![metadata_entry("Artist", "Jane Patient")],
        requires_visual_review: true,
        unsupported_payload: false,
    };

    let debug = format!("{input:?}");

    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("Jane Patient"));
    assert!(!debug.contains("Jane-Doe"));
    assert!(!debug.contains("patients/"));
}

#[test]
fn extraction_rejects_empty_artifact_label() {
    let input = ConservativeMediaInput {
        artifact_label: "   ".to_string(),
        format: ConservativeMediaFormat::Image,
        metadata: vec![metadata_entry("Artist", "Jane Patient")],
        requires_visual_review: true,
        unsupported_payload: false,
    };

    let err = ConservativeMediaAdapter::extract_metadata(input).unwrap_err();

    assert_eq!(err, ConservativeMediaAdapterError::EmptyArtifactLabel);
}
