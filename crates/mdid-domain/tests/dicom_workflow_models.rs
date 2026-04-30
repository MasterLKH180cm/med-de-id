use mdid_domain::{
    BurnedInAnnotationStatus, DicomDeidentificationSummary, DicomPhiCandidate,
    DicomPrivateTagPolicy, DicomTagRef, ReviewDecision, DICOM_BURNED_IN_PIXEL_REDACTION_NOTICE,
};

#[test]
fn dicom_tag_ref_builds_a_stable_field_path_and_detects_private_groups() {
    let patient_name = DicomTagRef::new(0x0010, 0x0010, "PatientName".into());
    let private_creator = DicomTagRef::new(0x0011, 0x0010, "PrivateCreator".into());

    assert_eq!(patient_name.field_path(), "dicom/0010,0010/PatientName");
    assert!(!patient_name.is_private());
    assert!(private_creator.is_private());
}

#[test]
fn dicom_policy_wire_values_are_stable() {
    assert_eq!(
        serde_json::to_string(&DicomPrivateTagPolicy::ReviewRequired).unwrap(),
        "\"review_required\""
    );
    assert_eq!(
        serde_json::to_string(&BurnedInAnnotationStatus::Suspicious).unwrap(),
        "\"suspicious\""
    );
}

#[test]
fn dicom_phi_candidate_debug_redacts_phi() {
    let candidate = DicomPhiCandidate {
        tag: DicomTagRef::new(0x0010, 0x0010, "PatientName".into()),
        phi_type: "patient_name".into(),
        value: "Alice Smith".into(),
        decision: ReviewDecision::Approved,
    };

    let debug = format!("{candidate:?}");
    assert!(debug.contains("DicomPhiCandidate"));
    assert!(!debug.contains("Alice Smith"));
}

#[test]
fn dicom_summary_requires_review_for_review_items_or_burned_in_suspicion() {
    let review_summary = DicomDeidentificationSummary {
        review_required_tags: 1,
        ..DicomDeidentificationSummary::default()
    };
    let suspicious_summary = DicomDeidentificationSummary {
        burned_in_suspicions: 1,
        ..DicomDeidentificationSummary::default()
    };

    assert!(review_summary.requires_review());
    assert!(suspicious_summary.requires_review());
    assert!(!DicomDeidentificationSummary::default().requires_review());
}

#[test]
fn dicom_summary_discloses_pixel_redaction_is_not_performed() {
    let summary = DicomDeidentificationSummary::default();

    assert!(!summary.pixel_redaction_performed);
    assert!(summary
        .burned_in_annotation_notice
        .contains("Pixel redaction was not performed"));
}

#[test]
fn dicom_summary_deserializes_older_json_without_pixel_disclosure_fields() {
    let summary: DicomDeidentificationSummary = serde_json::from_str(
        r#"{
            "total_tags": 5,
            "encoded_tags": 2,
            "review_required_tags": 1,
            "removed_private_tags": 3,
            "remapped_uids": 4,
            "burned_in_suspicions": 0
        }"#,
    )
    .unwrap();

    assert_eq!(summary.total_tags, 5);
    assert!(!summary.pixel_redaction_performed);
    assert!(!summary.burned_in_review_required);
    assert_eq!(
        summary.burned_in_annotation_notice,
        DICOM_BURNED_IN_PIXEL_REDACTION_NOTICE
    );
}

#[test]
fn dicom_summary_flags_burned_in_review_when_suspicious() {
    let summary = DicomDeidentificationSummary {
        burned_in_suspicions: 1,
        burned_in_review_required: true,
        ..DicomDeidentificationSummary::default()
    };

    assert!(summary.burned_in_review_required);
}
