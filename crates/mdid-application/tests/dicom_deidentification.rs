use std::io::Cursor;

use dicom_core::{Tag, VR};
use dicom_object::{
    file::ReadPreamble, meta::FileMetaTableBuilder, DefaultDicomObject, InMemDicomObject,
    OpenFileOptions,
};
use mdid_application::{DicomDeidentificationOutput, DicomDeidentificationService};
use mdid_domain::{DicomPrivateTagPolicy, DicomTagRef, ReviewDecision, SurfaceKind};
use mdid_vault::LocalVaultStore;
use tempfile::tempdir;

#[test]
fn dicom_deidentification_reuses_vault_tokens_for_repeated_phi_values() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("vault.mdid");
    let mut vault = LocalVaultStore::create(&path, "correct horse battery staple").unwrap();
    let service = DicomDeidentificationService;

    let output = service
        .deidentify_bytes(
            &build_dicom_fixture_with_repeated_common_phi_value("SHARED-PHI-001"),
            "Alice Smith/MRN-001/report.real.DCM",
            DicomPrivateTagPolicy::Remove,
            &mut vault,
            SurfaceKind::Cli,
        )
        .unwrap();
    let rewritten = parse_dicom(&output.bytes);
    let replacement = tag_value(&rewritten, Tag(0x0010, 0x0010));

    assert_eq!(output.sanitized_file_name, "dicom-output.dcm");
    assert_eq!(replacement, tag_value(&rewritten, Tag(0x0010, 0x0020)));
    assert_eq!(replacement, tag_value(&rewritten, Tag(0x0008, 0x0050)));
    assert_eq!(replacement, tag_value(&rewritten, Tag(0x0008, 0x1030)));
    assert_ne!(replacement, "SHARED-PHI-001");
    assert_eq!(output.summary.total_tags, 4);
    assert_eq!(output.summary.encoded_tags, 4);
    assert_eq!(output.summary.review_required_tags, 0);
    assert_eq!(output.summary.removed_private_tags, 0);
    assert_eq!(output.summary.remapped_uids, 3);
    assert_eq!(output.summary.burned_in_suspicions, 0);
    assert!(!output.summary.pixel_redaction_performed);
    assert!(!output.summary.burned_in_review_required);
    assert!(output
        .summary
        .burned_in_annotation_notice
        .contains("Pixel redaction was not performed"));
    assert!(!output.summary.requires_review());
    assert!(output.review_queue.is_empty());
    assert_eq!(vault.audit_events().len(), 7);
    assert_ne!(
        tag_value(&rewritten, Tag(0x0020, 0x000D)),
        "2.25.123456789012345678901234567890123457"
    );
    assert_ne!(
        tag_value(&rewritten, Tag(0x0020, 0x000E)),
        "2.25.123456789012345678901234567890123458"
    );
    assert_ne!(
        tag_value(&rewritten, Tag(0x0008, 0x0018)),
        "2.25.123456789012345678901234567890123456"
    );
}

#[test]
fn dicom_deidentification_routes_private_tags_and_burned_in_suspicion_to_review() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("vault.mdid");
    let mut vault = LocalVaultStore::create(&path, "correct horse battery staple").unwrap();
    let service = DicomDeidentificationService;

    let output = service
        .deidentify_bytes(
            &build_dicom_fixture("YES", true),
            "Alice Smith/MRN-001/private-scan.dcm",
            DicomPrivateTagPolicy::ReviewRequired,
            &mut vault,
            SurfaceKind::Desktop,
        )
        .unwrap();

    assert_eq!(output.sanitized_file_name, "dicom-output.dcm");
    assert_eq!(output.summary.total_tags, 6);
    assert_eq!(output.summary.encoded_tags, 4);
    assert_eq!(output.summary.review_required_tags, 2);
    assert_eq!(output.summary.removed_private_tags, 0);
    assert_eq!(output.summary.remapped_uids, 3);
    assert_eq!(output.summary.burned_in_suspicions, 1);
    assert!(!output.summary.pixel_redaction_performed);
    assert!(output.summary.burned_in_review_required);
    assert!(output
        .summary
        .burned_in_annotation_notice
        .contains("burned-in annotation review is required"));
    assert!(output.summary.requires_review());
    assert_eq!(output.review_queue.len(), 2);
    assert_eq!(vault.audit_events().len(), 7);

    let mut review_tags = output
        .review_queue
        .iter()
        .map(|candidate| {
            (
                candidate.tag.clone(),
                candidate.decision,
                candidate.phi_type.clone(),
            )
        })
        .collect::<Vec<_>>();
    review_tags.sort_by_key(|(tag, _, _)| (tag.group, tag.element));

    assert_eq!(
        review_tags,
        vec![
            (
                DicomTagRef::new(0x0011, 0x0010, "PrivateTag".into()),
                ReviewDecision::NeedsReview,
                "private_tag".to_string(),
            ),
            (
                DicomTagRef::new(0x0011, 0x1010, "PrivateTag".into()),
                ReviewDecision::NeedsReview,
                "private_tag".to_string(),
            ),
        ]
    );
}

#[test]
fn dicom_deidentification_counts_nested_private_tag_review_and_removal_honestly() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("vault.mdid");
    let mut vault = LocalVaultStore::create(&path, "correct horse battery staple").unwrap();
    let service = DicomDeidentificationService;

    let review_output = service
        .deidentify_bytes(
            &build_dicom_fixture_with_nested_private_sequence("NO"),
            "nested-private-review.dcm",
            DicomPrivateTagPolicy::ReviewRequired,
            &mut vault,
            SurfaceKind::Desktop,
        )
        .unwrap();

    assert_eq!(review_output.summary.total_tags, 6);
    assert_eq!(review_output.summary.encoded_tags, 4);
    assert_eq!(review_output.summary.review_required_tags, 2);
    assert_eq!(review_output.summary.removed_private_tags, 0);
    assert_eq!(review_output.summary.remapped_uids, 4);
    assert_eq!(review_output.summary.burned_in_suspicions, 0);
    assert!(review_output.summary.requires_review());
    assert_eq!(review_output.review_queue.len(), 2);
    assert_eq!(
        review_output
            .review_queue
            .iter()
            .map(|candidate| candidate.tag.clone())
            .collect::<Vec<_>>(),
        vec![
            DicomTagRef::new(0x0011, 0x0010, "PrivateTag".into()),
            DicomTagRef::new(0x0011, 0x1010, "PrivateTag".into()),
        ]
    );

    let remove_output = service
        .deidentify_bytes(
            &build_dicom_fixture_with_nested_private_sequence("NO"),
            "nested-private-remove.dcm",
            DicomPrivateTagPolicy::Remove,
            &mut vault,
            SurfaceKind::Cli,
        )
        .unwrap();
    let rewritten = parse_dicom(&remove_output.bytes);
    let sequence_items = rewritten
        .get(Tag(0x0008, 0x1115))
        .expect("expected referenced series sequence")
        .items()
        .expect("expected sequence items");
    let nested_item = sequence_items
        .first()
        .expect("expected one nested sequence item");

    assert_eq!(remove_output.summary.total_tags, 4);
    assert_eq!(remove_output.summary.encoded_tags, 4);
    assert_eq!(remove_output.summary.review_required_tags, 0);
    assert_eq!(remove_output.summary.removed_private_tags, 2);
    assert_eq!(remove_output.summary.remapped_uids, 4);
    assert_eq!(remove_output.summary.burned_in_suspicions, 0);
    assert!(!remove_output.summary.requires_review());
    assert!(remove_output.review_queue.is_empty());
    assert!(nested_item.get(Tag(0x0011, 0x0010)).is_none());
    assert!(nested_item.get(Tag(0x0011, 0x1010)).is_none());
}

#[test]
fn dicom_deidentification_counts_nested_uid_remaps_honestly() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("vault.mdid");
    let mut vault = LocalVaultStore::create(&path, "correct horse battery staple").unwrap();
    let service = DicomDeidentificationService;

    let output = service
        .deidentify_bytes(
            &build_dicom_fixture_with_nested_uid_reference("NO"),
            "nested-uids.dcm",
            DicomPrivateTagPolicy::Remove,
            &mut vault,
            SurfaceKind::Cli,
        )
        .unwrap();
    let rewritten = parse_dicom(&output.bytes);
    let sequence_items = rewritten
        .get(Tag(0x0008, 0x1115))
        .expect("expected referenced series sequence")
        .items()
        .expect("expected sequence items");
    let nested_item = sequence_items
        .first()
        .expect("expected one nested sequence item");
    let top_level_series_uid = tag_value(&rewritten, Tag(0x0020, 0x000E));
    let nested_series_uid = tag_value_in_item(nested_item, Tag(0x0020, 0x000E));

    assert_eq!(output.summary.total_tags, 4);
    assert_eq!(output.summary.encoded_tags, 4);
    assert_eq!(output.summary.review_required_tags, 0);
    assert_eq!(output.summary.removed_private_tags, 0);
    assert_eq!(output.summary.remapped_uids, 4);
    assert_eq!(output.summary.burned_in_suspicions, 0);
    assert_eq!(vault.audit_events().len(), 8);
    assert_ne!(
        top_level_series_uid,
        "2.25.123456789012345678901234567890123458"
    );
    assert_ne!(
        nested_series_uid,
        "2.25.123456789012345678901234567890123499"
    );
    assert_ne!(
        top_level_series_uid, nested_series_uid,
        "distinct original nested UID values should not collapse onto one replacement"
    );
}

#[test]
fn dicom_deidentification_output_debug_redacts_phi() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("vault.mdid");
    let mut vault = LocalVaultStore::create(&path, "correct horse battery staple").unwrap();
    let service = DicomDeidentificationService;

    let output = service
        .deidentify_bytes(
            &build_dicom_fixture("NO", false),
            "Alice Smith/MRN-001/report.real.DCM",
            DicomPrivateTagPolicy::Remove,
            &mut vault,
            SurfaceKind::Cli,
        )
        .unwrap();

    assert_debug_redacted(&output);
}

fn assert_debug_redacted(output: &DicomDeidentificationOutput) {
    let debug = format!("{output:?}");

    assert!(debug.contains("DicomDeidentificationOutput"));
    assert!(debug.contains("[REDACTED]"));
    assert!(debug.contains("sanitized_file_name: \"dicom-output.dcm\""));
    assert!(!debug.contains("Alice Smith/MRN-001/report.real.DCM"));
    assert!(!debug.contains("Alice^Smith"));
    assert!(!debug.contains("MRN-001"));
    assert!(!debug.contains("ACC-4242"));
    assert!(!debug.contains("Cardiac MRI"));
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

fn build_dicom_fixture_with_repeated_common_phi_value(value: &str) -> Vec<u8> {
    let mut obj = base_dicom_fixture();
    obj.put_str(Tag(0x0008, 0x0050), VR::SH, value);
    obj.put_str(Tag(0x0008, 0x1030), VR::LO, value);
    obj.put_str(Tag(0x0010, 0x0010), VR::PN, value);
    obj.put_str(Tag(0x0010, 0x0020), VR::LO, value);
    obj.put_str(Tag(0x0028, 0x0301), VR::CS, "NO");
    serialize_dicom(obj)
}

fn build_dicom_fixture_with_nested_private_sequence(burned_in_annotation: &str) -> Vec<u8> {
    let mut obj = base_dicom_fixture();
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
        dicom_core::DicomValue::new_sequence(vec![nested_item], dicom_core::Length::UNDEFINED),
    ));

    serialize_dicom(obj)
}

fn build_dicom_fixture_with_nested_uid_reference(burned_in_annotation: &str) -> Vec<u8> {
    let mut obj = base_dicom_fixture();
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
    obj.put(dicom_core::DataElement::new(
        Tag(0x0008, 0x1115),
        VR::SQ,
        dicom_core::DicomValue::new_sequence(vec![nested_item], dicom_core::Length::UNDEFINED),
    ));

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

fn tag_value_in_item(obj: &InMemDicomObject, tag: Tag) -> String {
    obj.get(tag)
        .expect("expected DICOM tag to be present")
        .to_str()
        .expect("expected DICOM tag to be textual")
        .into_owned()
}
