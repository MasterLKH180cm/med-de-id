use dicom_core::{PrimitiveValue, Tag, VR};
use dicom_object::{meta::FileMetaTableBuilder, InMemDicomObject};
use mdid_adapters::{DicomAdapter, DicomAdapterError};
use mdid_domain::{BurnedInAnnotationStatus, DicomPrivateTagPolicy, DicomTagRef, ReviewDecision};

#[test]
fn extract_identifies_common_phi_tags_and_redacts_debug_output() -> Result<(), DicomAdapterError> {
    let adapter = DicomAdapter::new(DicomPrivateTagPolicy::ReviewRequired);
    let bytes = build_dicom_fixture("NO", false);

    let extracted = adapter.extract(&bytes, "fixture.dcm")?;

    assert_eq!(extracted.source_name, "fixture.dcm");
    assert_eq!(
        extracted.burned_in_annotation,
        BurnedInAnnotationStatus::Clean
    );
    assert!(extracted.private_tags.is_empty());
    assert_eq!(
        candidate_summary(&extracted),
        vec![
            (
                DicomTagRef::new(0x0008, 0x0050, "AccessionNumber".into()),
                "accession_number".to_string(),
                ReviewDecision::Approved,
                "ACC-4242".to_string(),
            ),
            (
                DicomTagRef::new(0x0008, 0x1030, "StudyDescription".into()),
                "study_description".to_string(),
                ReviewDecision::Approved,
                "Cardiac MRI".to_string(),
            ),
            (
                DicomTagRef::new(0x0010, 0x0010, "PatientName".into()),
                "patient_name".to_string(),
                ReviewDecision::Approved,
                "Alice^Smith".to_string(),
            ),
            (
                DicomTagRef::new(0x0010, 0x0020, "PatientID".into()),
                "patient_id".to_string(),
                ReviewDecision::Approved,
                "MRN-001".to_string(),
            ),
        ]
    );

    let debug = format!("{extracted:?}");
    assert!(debug.contains("ExtractedDicomData"));
    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("Alice^Smith"));
    assert!(!debug.contains("MRN-001"));
    assert!(!debug.contains("ACC-4242"));
    assert!(!debug.contains("Cardiac MRI"));

    Ok(())
}

#[test]
fn extract_marks_private_tags_for_review_or_removal_per_policy() -> Result<(), DicomAdapterError> {
    let bytes = build_dicom_fixture("NO", true);

    let review_required =
        DicomAdapter::new(DicomPrivateTagPolicy::ReviewRequired).extract(&bytes, "fixture.dcm")?;
    assert_eq!(
        review_required.private_tags,
        vec![
            DicomTagRef::new(0x0011, 0x0010, "PrivateTag".into()),
            DicomTagRef::new(0x0011, 0x1010, "PrivateTag".into()),
        ]
    );
    let private_review_candidates = review_required
        .candidates
        .iter()
        .filter(|candidate| candidate.tag.is_private())
        .map(|candidate| {
            (
                candidate.tag.clone(),
                candidate.phi_type.clone(),
                candidate.decision,
                candidate.value.clone(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        private_review_candidates,
        vec![
            (
                DicomTagRef::new(0x0011, 0x0010, "PrivateTag".into()),
                "private_tag".to_string(),
                ReviewDecision::NeedsReview,
                "ACME_CREATOR".to_string(),
            ),
            (
                DicomTagRef::new(0x0011, 0x1010, "PrivateTag".into()),
                "private_tag".to_string(),
                ReviewDecision::NeedsReview,
                "secret-annotation".to_string(),
            ),
        ]
    );

    let remove = DicomAdapter::new(DicomPrivateTagPolicy::Remove).extract(&bytes, "fixture.dcm")?;
    assert_eq!(remove.private_tags, review_required.private_tags);
    assert!(remove
        .candidates
        .iter()
        .all(|candidate| !candidate.tag.is_private()));

    Ok(())
}

#[test]
fn extract_review_required_handles_non_text_private_tags_without_failing(
) -> Result<(), DicomAdapterError> {
    let bytes = build_dicom_fixture_with_non_text_private_tag("NO");

    let extracted =
        DicomAdapter::new(DicomPrivateTagPolicy::ReviewRequired).extract(&bytes, "fixture.dcm")?;

    assert_eq!(
        extracted.private_tags,
        vec![
            DicomTagRef::new(0x0011, 0x0010, "PrivateTag".into()),
            DicomTagRef::new(0x0011, 0x1010, "PrivateTag".into()),
        ]
    );
    let private_review_candidates = extracted
        .candidates
        .iter()
        .filter(|candidate| candidate.tag.is_private())
        .map(|candidate| {
            (
                candidate.tag.clone(),
                candidate.phi_type.clone(),
                candidate.decision,
                candidate.value.clone(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        private_review_candidates,
        vec![
            (
                DicomTagRef::new(0x0011, 0x0010, "PrivateTag".into()),
                "private_tag".to_string(),
                ReviewDecision::NeedsReview,
                "ACME_CREATOR".to_string(),
            ),
            (
                DicomTagRef::new(0x0011, 0x1010, "PrivateTag".into()),
                "private_tag".to_string(),
                ReviewDecision::NeedsReview,
                "<non-text>".to_string(),
            ),
        ]
    );

    Ok(())
}

#[test]
fn extracted_dicom_data_debug_redacts_source_name() -> Result<(), DicomAdapterError> {
    let extracted = DicomAdapter::new(DicomPrivateTagPolicy::Remove).extract(
        &build_dicom_fixture("NO", false),
        "Alice_Smith_MRN-001/scan.dcm",
    )?;

    let debug = format!("{extracted:?}");

    assert!(debug.contains("ExtractedDicomData"));
    assert!(debug.contains("source_name: \"<redacted>\""));
    assert!(!debug.contains("Alice_Smith_MRN-001/scan.dcm"));

    Ok(())
}

#[test]
fn extract_flags_burned_in_annotation_as_suspicious() -> Result<(), DicomAdapterError> {
    let adapter = DicomAdapter::new(DicomPrivateTagPolicy::Remove);

    let suspicious = adapter.extract(&build_dicom_fixture("YES", false), "yes.dcm")?;
    let clean = adapter.extract(&build_dicom_fixture("NO", false), "no.dcm")?;

    assert_eq!(
        suspicious.burned_in_annotation,
        BurnedInAnnotationStatus::Suspicious
    );
    assert_eq!(clean.burned_in_annotation, BurnedInAnnotationStatus::Clean);

    Ok(())
}

fn build_dicom_fixture(burned_in_annotation: &str, include_private: bool) -> Vec<u8> {
    let mut obj = InMemDicomObject::new_empty();
    obj.put_str(Tag(0x0008, 0x0016), VR::UI, "1.2.840.10008.5.1.4.1.1.7");
    obj.put_str(
        Tag(0x0008, 0x0018),
        VR::UI,
        "2.25.123456789012345678901234567890123456",
    );
    obj.put_str(Tag(0x0008, 0x0050), VR::SH, "ACC-4242");
    obj.put_str(Tag(0x0008, 0x1030), VR::LO, "Cardiac MRI");
    obj.put_str(Tag(0x0010, 0x0010), VR::PN, "Alice^Smith");
    obj.put_str(Tag(0x0010, 0x0020), VR::LO, "MRN-001");
    obj.put_str(Tag(0x0028, 0x0301), VR::CS, burned_in_annotation);

    if include_private {
        obj.put_str(Tag(0x0011, 0x0010), VR::LO, "ACME_CREATOR");
        obj.put(dicom_core::DataElement::new(
            Tag(0x0011, 0x1010),
            VR::LO,
            PrimitiveValue::from("secret-annotation"),
        ));
    }

    let file_obj = obj
        .with_meta(FileMetaTableBuilder::new().transfer_syntax("1.2.840.10008.1.2.1"))
        .expect("fixture should create file object");
    let mut bytes = Vec::new();
    file_obj
        .write_all(&mut bytes)
        .expect("fixture should serialize to bytes");
    bytes
}

fn build_dicom_fixture_with_non_text_private_tag(burned_in_annotation: &str) -> Vec<u8> {
    let mut obj = InMemDicomObject::new_empty();
    obj.put_str(Tag(0x0008, 0x0016), VR::UI, "1.2.840.10008.5.1.4.1.1.7");
    obj.put_str(
        Tag(0x0008, 0x0018),
        VR::UI,
        "2.25.123456789012345678901234567890123456",
    );
    obj.put_str(Tag(0x0008, 0x0050), VR::SH, "ACC-4242");
    obj.put_str(Tag(0x0008, 0x1030), VR::LO, "Cardiac MRI");
    obj.put_str(Tag(0x0010, 0x0010), VR::PN, "Alice^Smith");
    obj.put_str(Tag(0x0010, 0x0020), VR::LO, "MRN-001");
    obj.put_str(Tag(0x0028, 0x0301), VR::CS, burned_in_annotation);
    obj.put_str(Tag(0x0011, 0x0010), VR::LO, "ACME_CREATOR");
    obj.put(dicom_core::DataElement::empty(Tag(0x0011, 0x1010), VR::SQ));

    let file_obj = obj
        .with_meta(FileMetaTableBuilder::new().transfer_syntax("1.2.840.10008.1.2.1"))
        .expect("fixture should create file object");
    let mut bytes = Vec::new();
    file_obj
        .write_all(&mut bytes)
        .expect("fixture should serialize to bytes");
    bytes
}

fn candidate_summary(
    extracted: &mdid_adapters::ExtractedDicomData,
) -> Vec<(DicomTagRef, String, ReviewDecision, String)> {
    let mut summary = extracted
        .candidates
        .iter()
        .map(|candidate| {
            (
                candidate.tag.clone(),
                candidate.phi_type.clone(),
                candidate.decision,
                candidate.value.clone(),
            )
        })
        .collect::<Vec<_>>();
    summary.sort_by_key(|(tag, _, _, _)| (tag.group, tag.element));
    summary
}
