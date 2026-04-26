use mdid_domain::{
    BurnedInAnnotationStatus, DicomDeidentificationSummary, DicomPhiCandidate,
    DicomPrivateTagPolicy, DicomTagRef, ReviewDecision,
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
