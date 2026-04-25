use std::io::Cursor;

use dicom_core::{DicomValue, Length, PrimitiveValue, Tag, VR};
use dicom_object::{
    file::ReadPreamble, meta::FileMetaTableBuilder, DefaultDicomObject, InMemDicomObject,
    OpenFileOptions,
};
use mdid_adapters::{
    sanitize_output_name, DicomAdapter, DicomAdapterError, DicomRewritePlan, DicomTagReplacement,
    DicomUidReplacement,
};
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
fn extract_skips_blank_or_whitespace_only_common_phi_tags() -> Result<(), DicomAdapterError> {
    let adapter = DicomAdapter::new(DicomPrivateTagPolicy::ReviewRequired);
    let bytes = build_dicom_fixture_with_common_phi_values("   ", "\t  ");

    let extracted = adapter.extract(&bytes, "fixture.dcm")?;

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
        ]
    );

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

#[test]
fn rewrite_replaces_encoded_phi_tags_and_remaps_uid_family() -> Result<(), DicomAdapterError> {
    let adapter = DicomAdapter::new(DicomPrivateTagPolicy::Keep);
    let plan = DicomRewritePlan {
        tag_replacements: vec![
            DicomTagReplacement::new(
                DicomTagRef::new(0x0010, 0x0010, "PatientName".into()),
                "PATIENT_001".into(),
            ),
            DicomTagReplacement::new(
                DicomTagRef::new(0x0010, 0x0020, "PatientID".into()),
                "ID_001".into(),
            ),
            DicomTagReplacement::new(
                DicomTagRef::new(0x0008, 0x1030, "StudyDescription".into()),
                "REDACTED_STUDY".into(),
            ),
        ],
        uid_replacements: vec![
            DicomUidReplacement::new(
                DicomTagRef::new(0x0020, 0x000D, "StudyInstanceUID".into()),
                "2.25.100000000000000000000000000000000001".into(),
            ),
            DicomUidReplacement::new(
                DicomTagRef::new(0x0020, 0x000E, "SeriesInstanceUID".into()),
                "2.25.100000000000000000000000000000000002".into(),
            ),
            DicomUidReplacement::new(
                DicomTagRef::new(0x0008, 0x0018, "SOPInstanceUID".into()),
                "2.25.100000000000000000000000000000000003".into(),
            ),
        ],
    };

    let rewritten = adapter.rewrite(&build_dicom_fixture("NO", false), &plan)?;
    let rewritten_obj = parse_dicom(&rewritten);

    assert_eq!(
        tag_value(&rewritten_obj, Tag(0x0010, 0x0010)),
        "PATIENT_001"
    );
    assert_eq!(tag_value(&rewritten_obj, Tag(0x0010, 0x0020)), "ID_001");
    assert_eq!(
        tag_value(&rewritten_obj, Tag(0x0008, 0x1030)),
        "REDACTED_STUDY"
    );
    assert_eq!(
        tag_value(&rewritten_obj, Tag(0x0020, 0x000D)),
        "2.25.100000000000000000000000000000000001"
    );
    assert_eq!(
        tag_value(&rewritten_obj, Tag(0x0020, 0x000E)),
        "2.25.100000000000000000000000000000000002"
    );
    assert_eq!(
        tag_value(&rewritten_obj, Tag(0x0008, 0x0018)),
        "2.25.100000000000000000000000000000000003"
    );
    assert_eq!(
        tag_value(&rewritten_obj, Tag(0x0008, 0x0050)),
        "ACC-4242",
        "unplanned tags should remain unchanged"
    );

    Ok(())
}

#[test]
fn rewrite_removes_private_tags_when_policy_is_remove() -> Result<(), DicomAdapterError> {
    let adapter = DicomAdapter::new(DicomPrivateTagPolicy::Remove);

    let rewritten = adapter.rewrite(
        &build_dicom_fixture_with_non_text_private_tag("NO"),
        &DicomRewritePlan::default(),
    )?;
    let rewritten_obj = parse_dicom(&rewritten);

    assert!(rewritten_obj.get(Tag(0x0011, 0x0010)).is_none());
    assert!(rewritten_obj.get(Tag(0x0011, 0x1010)).is_none());
    assert_eq!(
        tag_value(&rewritten_obj, Tag(0x0010, 0x0010)),
        "Alice^Smith",
        "non-private tags should remain intact"
    );

    Ok(())
}

#[test]
fn rewrite_removes_nested_private_tags_inside_sequence_items_when_policy_is_remove(
) -> Result<(), DicomAdapterError> {
    let adapter = DicomAdapter::new(DicomPrivateTagPolicy::Remove);

    let rewritten = adapter.rewrite(
        &build_dicom_fixture_with_nested_private_sequence("NO"),
        &DicomRewritePlan::default(),
    )?;
    let rewritten_obj = parse_dicom(&rewritten);
    let sequence_items = rewritten_obj
        .get(Tag(0x0008, 0x1115))
        .expect("expected referenced series sequence")
        .items()
        .expect("expected sequence items");
    let nested_item = sequence_items
        .first()
        .expect("expected one nested sequence item");

    assert!(nested_item.get(Tag(0x0011, 0x0010)).is_none());
    assert!(nested_item.get(Tag(0x0011, 0x1010)).is_none());
    assert_eq!(
        tag_value_in_item(nested_item, Tag(0x0020, 0x000E)),
        "2.25.123456789012345678901234567890123499",
        "non-private nested tags should remain intact"
    );

    Ok(())
}

#[test]
fn rewrite_drops_private_file_meta_information_when_policy_is_remove(
) -> Result<(), DicomAdapterError> {
    let adapter = DicomAdapter::new(DicomPrivateTagPolicy::Remove);

    let rewritten = adapter.rewrite(
        &build_dicom_fixture_with_private_file_meta("NO"),
        &DicomRewritePlan::default(),
    )?;
    let rewritten_obj = parse_dicom(&rewritten);

    assert!(rewritten_obj
        .meta()
        .private_information_creator_uid()
        .is_none());
    assert!(rewritten_obj.meta().private_information.is_none());

    Ok(())
}

#[test]
fn rewrite_drops_private_file_meta_information_when_policy_is_review_required(
) -> Result<(), DicomAdapterError> {
    let adapter = DicomAdapter::new(DicomPrivateTagPolicy::ReviewRequired);

    let rewritten = adapter.rewrite(
        &build_dicom_fixture_with_private_file_meta("NO"),
        &DicomRewritePlan::default(),
    )?;
    let rewritten_obj = parse_dicom(&rewritten);

    assert!(rewritten_obj
        .meta()
        .private_information_creator_uid()
        .is_none());
    assert!(rewritten_obj.meta().private_information.is_none());

    Ok(())
}

#[test]
fn sanitize_filename_replaces_phi_bearing_source_names_with_a_safe_neutral_output_name() {
    let sanitized = sanitize_output_name("Alice Smith\\MRN-001/scan (1).dcm");

    assert_eq!(sanitized, "dicom-output.dcm");
    assert!(!sanitized.contains("Alice"));
    assert!(!sanitized.contains("MRN"));
    assert!(!sanitized.contains("scan"));
}

#[test]
fn sanitize_filename_hardens_windows_reserved_and_dot_only_names() {
    assert_eq!(sanitize_output_name("."), "dicom-output");
    assert_eq!(sanitize_output_name(".."), "dicom-output");
    assert_eq!(sanitize_output_name("report."), "dicom-output");
    assert_eq!(sanitize_output_name("CON"), "dicom-output");
    assert_eq!(sanitize_output_name("con.txt"), "dicom-output");
    assert_eq!(sanitize_output_name("LPT1."), "dicom-output");
    assert_eq!(sanitize_output_name("scan.$$$"), "dicom-output");
    assert_eq!(sanitize_output_name("weird name..DCM"), "dicom-output.dcm");
}

#[test]
fn sanitize_filename_whitelists_only_dicom_safe_extensions() {
    assert_eq!(sanitize_output_name("study.MRN12345"), "dicom-output");
    assert_eq!(sanitize_output_name("scan.123-45-6789"), "dicom-output");
    assert_eq!(sanitize_output_name("scan.dcm.exe"), "dicom-output");
    assert_eq!(
        sanitize_output_name("scan.real.dicom"),
        "dicom-output.dicom"
    );
}

fn build_dicom_fixture(burned_in_annotation: &str, include_private: bool) -> Vec<u8> {
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

fn build_dicom_fixture_with_common_phi_values(patient_name: &str, patient_id: &str) -> Vec<u8> {
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
    obj.put_str(Tag(0x0008, 0x0050), VR::SH, "ACC-4242");
    obj.put_str(Tag(0x0008, 0x1030), VR::LO, "Cardiac MRI");
    obj.put_str(Tag(0x0010, 0x0010), VR::PN, patient_name);
    obj.put_str(Tag(0x0010, 0x0020), VR::LO, patient_id);
    obj.put_str(Tag(0x0028, 0x0301), VR::CS, "NO");

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

fn build_dicom_fixture_with_nested_private_sequence(burned_in_annotation: &str) -> Vec<u8> {
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
    obj.put_str(Tag(0x0008, 0x0050), VR::SH, "ACC-4242");
    obj.put_str(Tag(0x0008, 0x1030), VR::LO, "Cardiac MRI");
    obj.put_str(Tag(0x0010, 0x0010), VR::PN, "Alice^Smith");
    obj.put_str(Tag(0x0010, 0x0020), VR::LO, "MRN-001");
    obj.put_str(Tag(0x0028, 0x0301), VR::CS, burned_in_annotation);

    let mut nested_item = InMemDicomObject::new_empty();
    nested_item.put_str(
        Tag(0x0020, 0x000E),
        VR::UI,
        "2.25.123456789012345678901234567890123499",
    );
    nested_item.put_str(Tag(0x0011, 0x0010), VR::LO, "ACME_CREATOR");
    nested_item.put_str(Tag(0x0011, 0x1010), VR::LO, "nested-secret");
    obj.put(dicom_core::DataElement::new(
        Tag(0x0008, 0x1115),
        VR::SQ,
        DicomValue::new_sequence(vec![nested_item], Length::UNDEFINED),
    ));

    let file_obj = obj
        .with_meta(FileMetaTableBuilder::new().transfer_syntax("1.2.840.10008.1.2.1"))
        .expect("fixture should create file object");
    let mut bytes = Vec::new();
    file_obj
        .write_all(&mut bytes)
        .expect("fixture should serialize to bytes");
    bytes
}

fn build_dicom_fixture_with_private_file_meta(burned_in_annotation: &str) -> Vec<u8> {
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
    obj.put_str(Tag(0x0008, 0x0050), VR::SH, "ACC-4242");
    obj.put_str(Tag(0x0008, 0x1030), VR::LO, "Cardiac MRI");
    obj.put_str(Tag(0x0010, 0x0010), VR::PN, "Alice^Smith");
    obj.put_str(Tag(0x0010, 0x0020), VR::LO, "MRN-001");
    obj.put_str(Tag(0x0028, 0x0301), VR::CS, burned_in_annotation);

    let file_obj = obj
        .with_meta(
            FileMetaTableBuilder::new()
                .transfer_syntax("1.2.840.10008.1.2.1")
                .private_information_creator_uid("1.2.826.0.1.3680043.10.54321")
                .private_information(vec![0x13, 0x37, 0x42]),
        )
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

fn tag_value_in_item(obj: &InMemDicomObject, tag: Tag) -> String {
    obj.get(tag)
        .expect("expected DICOM tag to be present")
        .to_str()
        .expect("expected DICOM tag to be textual")
        .into_owned()
}
