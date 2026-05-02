use mdid_adapters::{
    ConservativeMediaAdapter, ConservativeMediaAdapterError, ConservativeMediaInput,
    ConservativeMediaMetadataEntry, FcsTextRewriteRequest,
};
use mdid_domain::{ConservativeMediaFormat, ConservativeMediaScanStatus};

fn metadata_entry(key: &str, value: &str) -> ConservativeMediaMetadataEntry {
    ConservativeMediaMetadataEntry {
        key: key.to_string(),
        value: value.to_string(),
    }
}

fn fcs_fixture(text: &str, data: &[u8]) -> Vec<u8> {
    let text_start = 58usize;
    let text_end = text_start + text.len() - 1;
    let data_start = text_end + 1;
    let data_end = data_start + data.len() - 1;
    let mut header = b"FCS3.1    000000000000000000000000000000000000000000000000".to_vec();
    header[10..18].copy_from_slice(format!("{text_start:>8}").as_bytes());
    header[18..26].copy_from_slice(format!("{text_end:>8}").as_bytes());
    header[26..34].copy_from_slice(format!("{data_start:>8}").as_bytes());
    header[34..42].copy_from_slice(format!("{data_end:>8}").as_bytes());
    [header, text.as_bytes().to_vec(), data.to_vec()].concat()
}

#[test]
fn fcs_text_rewrite_replaces_only_requested_text_values_and_preserves_data_bytes() {
    let data = [0, 1, 2, 3, 250, 251, 252, 253];
    let input = fcs_fixture(
        "|$BEGINANALYSIS|0|$SMNO|MRN-12345|$OP|Dr. Alice Example|$SRC|Bone Marrow|",
        &data,
    );

    let output = ConservativeMediaAdapter::rewrite_fcs_text_segment(
        &input,
        FcsTextRewriteRequest {
            replacements: [
                ("$SMNO".to_string(), "[FCS_SAMPLE]".to_string()),
                ("$OP".to_string(), "[FCS_OPERATOR]".to_string()),
            ]
            .into_iter()
            .collect(),
        },
    )
    .unwrap();

    assert!(output
        .bytes
        .windows(b"[FCS_SAMPLE]".len())
        .any(|w| w == b"[FCS_SAMPLE]"));
    assert!(output
        .bytes
        .windows(b"[FCS_OPERATOR]".len())
        .any(|w| w == b"[FCS_OPERATOR]"));
    assert!(!output
        .bytes
        .windows(b"MRN-12345".len())
        .any(|w| w == b"MRN-12345"));
    assert!(!output
        .bytes
        .windows(b"Dr. Alice Example".len())
        .any(|w| w == b"Dr. Alice Example"));
    assert!(output
        .bytes
        .windows(b"Bone Marrow".len())
        .any(|w| w == b"Bone Marrow"));
    assert_eq!(&output.bytes[output.bytes.len() - data.len()..], data);
    assert_eq!(output.summary.replacement_count, 2);
    assert_eq!(output.summary.rewritten_key_count, 2);
    let debug = format!("{output:?}");
    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("MRN-12345"));
    assert!(!debug.contains("Dr. Alice Example"));
}

fn header_offset(bytes: &[u8], range: std::ops::Range<usize>) -> usize {
    std::str::from_utf8(&bytes[range])
        .unwrap()
        .trim()
        .parse()
        .unwrap()
}

#[test]
fn fcs_text_rewrite_shifts_data_offsets_when_text_length_changes() {
    let data = [9, 8, 7, 6, 5];
    let input = fcs_fixture("|$SMNO|MRN|$OP|Dr. Alice Example|", &data);
    let old_text_end = header_offset(&input, 18..26);
    let old_data_start = header_offset(&input, 26..34);
    let old_data_end = header_offset(&input, 34..42);

    let output = ConservativeMediaAdapter::rewrite_fcs_text_segment(
        &input,
        FcsTextRewriteRequest {
            replacements: [("$SMNO".to_string(), "[MUCH_LONGER_FCS_SAMPLE]".to_string())]
                .into_iter()
                .collect(),
        },
    )
    .unwrap();

    let new_text_end = header_offset(&output.bytes, 18..26);
    let delta = new_text_end as isize - old_text_end as isize;
    assert_ne!(delta, 0);
    assert_eq!(
        header_offset(&output.bytes, 26..34),
        (old_data_start as isize + delta) as usize
    );
    assert_eq!(
        header_offset(&output.bytes, 34..42),
        (old_data_end as isize + delta) as usize
    );
    let new_data_start = header_offset(&output.bytes, 26..34);
    assert_eq!(
        &output.bytes[new_data_start..new_data_start + data.len()],
        data
    );
}

#[test]
fn fcs_text_rewrite_handles_doubled_delimiter_escaping_in_values_and_replacements() {
    let input = fcs_fixture("|$SMNO|MRN||123|$OP|Dr. Alice|", &[1, 2, 3]);

    let output = ConservativeMediaAdapter::rewrite_fcs_text_segment(
        &input,
        FcsTextRewriteRequest {
            replacements: [("$OP".to_string(), "Tech|One".to_string())]
                .into_iter()
                .collect(),
        },
    )
    .unwrap();

    let text_start = header_offset(&output.bytes, 10..18);
    let text_end = header_offset(&output.bytes, 18..26);
    let rewritten_text = std::str::from_utf8(&output.bytes[text_start..=text_end]).unwrap();
    assert!(rewritten_text.contains("|$SMNO|MRN||123|"));
    assert!(rewritten_text.contains("|$OP|Tech||One|"));
}

#[test]
fn fcs_text_rewrite_rejects_text_start_inside_header() {
    let mut input = fcs_fixture("|$SMNO|MRN-12345|", &[1, 2, 3]);
    input[10..18].copy_from_slice(b"      57");

    let err = ConservativeMediaAdapter::rewrite_fcs_text_segment(
        &input,
        FcsTextRewriteRequest {
            replacements: Default::default(),
        },
    )
    .unwrap_err();

    assert_eq!(err, ConservativeMediaAdapterError::InvalidFcsTextSegment);
}

#[test]
fn fcs_text_rewrite_fails_closed_on_invalid_text_offsets() {
    let mut input = fcs_fixture("|$SMNO|MRN-12345|", &[1, 2, 3]);
    input[10..18].copy_from_slice(b"     999");
    let err = ConservativeMediaAdapter::rewrite_fcs_text_segment(
        &input,
        FcsTextRewriteRequest {
            replacements: [("$SMNO".to_string(), "[FCS_SAMPLE]".to_string())]
                .into_iter()
                .collect(),
        },
    )
    .unwrap_err();

    assert_eq!(err, ConservativeMediaAdapterError::InvalidFcsTextSegment);
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
fn fcs_metadata_uses_semantic_phi_types_for_known_text_keys() {
    let input = ConservativeMediaInput {
        artifact_label: "flow/panel.fcs".to_string(),
        format: ConservativeMediaFormat::Fcs,
        metadata: vec![
            metadata_entry("$FIL", "Jane-Doe-panel.fcs"),
            metadata_entry("$SMNO", "MRN-12345"),
            metadata_entry("$SRC", "Bone Marrow aspirate"),
            metadata_entry("$OP", "Dr. Alice Example"),
            metadata_entry("$DATE", "2026-04-23"),
            metadata_entry("CUSTOM_NOTE", "Research subject Jane Example"),
        ],
        requires_visual_review: false,
        unsupported_payload: false,
    };

    let output = ConservativeMediaAdapter::extract_metadata(input).unwrap();

    let phi_types = output
        .candidates
        .iter()
        .map(|candidate| {
            (
                candidate.field_ref.metadata_key.as_str(),
                candidate.phi_type.as_str(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        phi_types,
        vec![
            ("$FIL", "fcs_filename_identifier"),
            ("$SMNO", "fcs_sample_identifier"),
            ("$SRC", "fcs_source_identifier"),
            ("$OP", "fcs_operator_identifier"),
            ("$DATE", "fcs_collection_date"),
            ("CUSTOM_NOTE", "metadata_identifier"),
        ]
    );
    assert!(output
        .candidates
        .iter()
        .all(|candidate| candidate.status == ConservativeMediaScanStatus::MetadataOnly));
    assert_eq!(output.summary.review_required_candidates, 6);
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
