use mdid_domain::{
    ConservativeMediaCandidate, ConservativeMediaFormat, ConservativeMediaRef,
    ConservativeMediaScanStatus, ConservativeMediaSummary,
};

#[test]
fn conservative_media_format_uses_stable_snake_case_wire_values() {
    assert_eq!(
        serde_json::to_string(&ConservativeMediaFormat::Image).unwrap(),
        "\"image\""
    );
    assert_eq!(
        serde_json::to_string(&ConservativeMediaFormat::Video).unwrap(),
        "\"video\""
    );
    assert_eq!(
        serde_json::to_string(&ConservativeMediaFormat::Fcs).unwrap(),
        "\"fcs\""
    );
}

#[test]
fn conservative_media_status_uses_stable_snake_case_wire_values() {
    assert_eq!(
        serde_json::to_string(&ConservativeMediaScanStatus::MetadataOnly).unwrap(),
        "\"metadata_only\""
    );
    assert_eq!(
        serde_json::to_string(&ConservativeMediaScanStatus::OcrOrVisualReviewRequired).unwrap(),
        "\"ocr_or_visual_review_required\""
    );
    assert_eq!(
        serde_json::to_string(&ConservativeMediaScanStatus::UnsupportedPayload).unwrap(),
        "\"unsupported_payload\""
    );
}

#[test]
fn conservative_media_ref_sanitizes_slashes_and_backslashes_in_field_path_labels() {
    let field_ref = ConservativeMediaRef {
        artifact_label: "dicom\\screenshots/patient.png".to_string(),
        metadata_key: "Patient\\Name/Family".to_string(),
    };
    assert_eq!(
        field_ref.field_path(),
        "media:dicom_screenshots_patient.png:Patient_Name_Family"
    );
}

#[test]
fn conservative_media_ref_debug_redacts_phi_bearing_labels() {
    let field_ref = ConservativeMediaRef {
        artifact_label: "patients/Jane-Doe-face.png".to_string(),
        metadata_key: "Artist".to_string(),
    };

    let debug = format!("{field_ref:?}");

    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("Jane-Doe"));
    assert!(!debug.contains("patients/"));
    assert!(!debug.contains("Artist"));
}

#[test]
fn conservative_media_candidate_debug_redacts_source_value() {
    let candidate = ConservativeMediaCandidate {
        field_ref: ConservativeMediaRef {
            artifact_label: "patient.png".to_string(),
            metadata_key: "EXIF Artist".to_string(),
        },
        format: ConservativeMediaFormat::Image,
        phi_type: "person_name".to_string(),
        source_value: "Jane Patient".to_string(),
        confidence: 0.55,
        status: ConservativeMediaScanStatus::MetadataOnly,
    };
    let debug = format!("{candidate:?}");
    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("Jane Patient"));
}

#[test]
fn conservative_media_candidate_debug_redacts_phi_bearing_reference_and_source_value() {
    let candidate = ConservativeMediaCandidate {
        field_ref: ConservativeMediaRef {
            artifact_label: "patients/Jane-Doe-face.png".to_string(),
            metadata_key: "Artist".to_string(),
        },
        format: ConservativeMediaFormat::Image,
        phi_type: "person_name".to_string(),
        source_value: "Jane Patient".to_string(),
        confidence: 0.55,
        status: ConservativeMediaScanStatus::MetadataOnly,
    };

    let debug = format!("{candidate:?}");

    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("Jane-Doe"));
    assert!(!debug.contains("patients/"));
    assert!(!debug.contains("Jane Patient"));
}

#[test]
fn conservative_media_summary_requires_review_for_visual_review_or_review_candidates() {
    let clean = ConservativeMediaSummary {
        total_items: 1,
        metadata_only_items: 1,
        visual_review_required_items: 0,
        unsupported_items: 0,
        review_required_candidates: 0,
    };
    assert!(!clean.requires_review());

    let visual_review = ConservativeMediaSummary {
        visual_review_required_items: 1,
        ..clean.clone()
    };
    assert!(visual_review.requires_review());

    let candidate_review = ConservativeMediaSummary {
        review_required_candidates: 1,
        ..clean
    };
    assert!(candidate_review.requires_review());
}
