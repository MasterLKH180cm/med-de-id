use std::io::Cursor;

use dicom_core::{header::Header, value::ConvertValueError, Tag};
use dicom_object::{
    file::ReadPreamble,
    meta::{FileMetaTable, FileMetaTableBuilder},
    DefaultDicomObject, InMemDicomObject, OpenFileOptions, ReadError, WithMetaError, WriteError,
};
use mdid_domain::{
    BurnedInAnnotationStatus, DicomPhiCandidate, DicomPrivateTagPolicy, DicomTagRef, ReviewDecision,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DicomAdapterError {
    #[error("failed to parse DICOM input: {0}")]
    Parse(#[from] ReadError),
    #[error("failed to convert DICOM value: {0}")]
    Value(#[from] ConvertValueError),
    #[error("failed to rebuild DICOM file metadata: {0}")]
    Meta(#[from] WithMetaError),
    #[error("failed to serialize DICOM output: {0}")]
    Write(#[from] WriteError),
}

#[derive(Debug, Clone)]
pub struct DicomAdapter {
    private_tag_policy: DicomPrivateTagPolicy,
}

impl DicomAdapter {
    pub fn new(private_tag_policy: DicomPrivateTagPolicy) -> Self {
        Self { private_tag_policy }
    }

    pub fn extract(
        &self,
        bytes: &[u8],
        source_name: &str,
    ) -> Result<ExtractedDicomData, DicomAdapterError> {
        let obj = OpenFileOptions::new()
            .read_preamble(ReadPreamble::Always)
            .from_reader(Cursor::new(bytes))?;
        let mut candidates = common_phi_candidates(&obj)?;

        let private_tags = obj
            .iter()
            .filter_map(|element| {
                let tag = element.tag();
                (tag.0 % 2 == 1).then(|| dicom_tag_ref(tag, "PrivateTag"))
            })
            .collect::<Vec<_>>();

        if self.private_tag_policy == DicomPrivateTagPolicy::ReviewRequired {
            for element in obj.iter().filter(|element| element.tag().0 % 2 == 1) {
                candidates.push(DicomPhiCandidate {
                    tag: dicom_tag_ref(element.tag(), "PrivateTag"),
                    phi_type: "private_tag".into(),
                    value: match element.to_str() {
                        Ok(value) => value.into_owned(),
                        Err(_) => "<non-text>".into(),
                    },
                    decision: ReviewDecision::NeedsReview,
                });
            }
        }

        Ok(ExtractedDicomData {
            source_name: source_name.into(),
            candidates,
            private_tags,
            burned_in_annotation: burned_in_annotation_status(&obj)?,
        })
    }

    pub fn rewrite(
        &self,
        bytes: &[u8],
        plan: &DicomRewritePlan,
    ) -> Result<Vec<u8>, DicomAdapterError> {
        let obj = OpenFileOptions::new()
            .read_preamble(ReadPreamble::Always)
            .from_reader(Cursor::new(bytes))?;
        let meta = obj.meta().clone();
        let mut dataset = obj.into_inner();

        apply_tag_replacements(&mut dataset, &plan.tag_replacements);
        apply_uid_replacements(&mut dataset, &plan.uid_replacements);

        if self.private_tag_policy == DicomPrivateTagPolicy::Remove {
            dataset.retain(|element| element.tag().0 % 2 == 0);
        }

        let file_obj = dataset.with_meta(file_meta_builder(&meta))?;
        let mut rewritten = Vec::new();
        file_obj.write_all(&mut rewritten)?;
        Ok(rewritten)
    }
}

#[derive(Clone, PartialEq, Eq, Default)]
pub struct DicomRewritePlan {
    pub tag_replacements: Vec<DicomTagReplacement>,
    pub uid_replacements: Vec<DicomUidReplacement>,
}

impl std::fmt::Debug for DicomRewritePlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DicomRewritePlan")
            .field("tag_replacements", &self.tag_replacements)
            .field("uid_replacements", &self.uid_replacements)
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct DicomTagReplacement {
    pub tag: DicomTagRef,
    value: String,
}

impl DicomTagReplacement {
    pub fn new(tag: DicomTagRef, value: String) -> Self {
        Self { tag, value }
    }
}

impl std::fmt::Debug for DicomTagReplacement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DicomTagReplacement")
            .field("tag", &self.tag)
            .field("value", &"<redacted>")
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct DicomUidReplacement {
    pub tag: DicomTagRef,
    value: String,
}

impl DicomUidReplacement {
    pub fn new(tag: DicomTagRef, value: String) -> Self {
        Self { tag, value }
    }
}

impl std::fmt::Debug for DicomUidReplacement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DicomUidReplacement")
            .field("tag", &self.tag)
            .field("value", &"<redacted>")
            .finish()
    }
}

#[derive(Clone)]
pub struct ExtractedDicomData {
    pub source_name: String,
    pub candidates: Vec<DicomPhiCandidate>,
    pub private_tags: Vec<DicomTagRef>,
    pub burned_in_annotation: BurnedInAnnotationStatus,
}

impl std::fmt::Debug for ExtractedDicomData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExtractedDicomData")
            .field("source_name", &"<redacted>")
            .field("candidates", &self.candidates)
            .field("private_tags", &self.private_tags)
            .field("burned_in_annotation", &self.burned_in_annotation)
            .finish()
    }
}

pub fn sanitize_output_name(source_name: &str) -> String {
    let sanitized = source_name
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '_' | '-' => ch,
            _ => '_',
        })
        .collect::<String>();

    if sanitized.is_empty() {
        "_".into()
    } else {
        sanitized
    }
}

fn apply_tag_replacements(obj: &mut InMemDicomObject, replacements: &[DicomTagReplacement]) {
    for replacement in replacements {
        let tag = tag_from_ref(&replacement.tag);
        if !is_common_phi_tag(tag) {
            continue;
        }

        let Some(vr) = obj.get(tag).map(|element| element.vr()) else {
            continue;
        };

        obj.put_str(tag, vr, replacement.value.clone());
    }
}

fn apply_uid_replacements(obj: &mut InMemDicomObject, replacements: &[DicomUidReplacement]) {
    for replacement in replacements {
        let tag = tag_from_ref(&replacement.tag);
        if !is_uid_family_tag(tag) {
            continue;
        }

        let Some(vr) = obj.get(tag).map(|element| element.vr()) else {
            continue;
        };

        obj.put_str(tag, vr, replacement.value.clone());
    }
}

fn file_meta_builder(meta: &FileMetaTable) -> FileMetaTableBuilder {
    let mut builder = FileMetaTableBuilder::new()
        .information_version(meta.information_version)
        .transfer_syntax(meta.transfer_syntax())
        .implementation_class_uid(meta.implementation_class_uid());

    if let Some(value) = trimmed_optional(meta.implementation_version_name.as_deref()) {
        builder = builder.implementation_version_name(value);
    }
    if let Some(value) = trimmed_optional(meta.source_application_entity_title.as_deref()) {
        builder = builder.source_application_entity_title(value);
    }
    if let Some(value) = trimmed_optional(meta.sending_application_entity_title.as_deref()) {
        builder = builder.sending_application_entity_title(value);
    }
    if let Some(value) = trimmed_optional(meta.receiving_application_entity_title.as_deref()) {
        builder = builder.receiving_application_entity_title(value);
    }
    if let Some(value) = meta.private_information_creator_uid() {
        builder = builder.private_information_creator_uid(value);
    }
    if let Some(value) = meta.private_information.clone() {
        builder = builder.private_information(value);
    }

    builder
}

fn trimmed_optional(value: Option<&str>) -> Option<&str> {
    value.map(trimmed_value).filter(|value| !value.is_empty())
}

fn trimmed_value(value: &str) -> &str {
    value.trim_end_matches(|ch: char| ch.is_whitespace() || ch == '\0')
}

fn common_phi_candidates(
    obj: &DefaultDicomObject,
) -> Result<Vec<DicomPhiCandidate>, DicomAdapterError> {
    let mut candidates = Vec::new();

    for spec in COMMON_PHI_TAGS {
        if let Some(element) = obj.get(spec.tag) {
            let value = element.to_str()?;
            if value.trim().is_empty() {
                continue;
            }

            candidates.push(DicomPhiCandidate {
                tag: dicom_tag_ref(spec.tag, spec.keyword),
                phi_type: spec.phi_type.into(),
                value: value.into_owned(),
                decision: ReviewDecision::Approved,
            });
        }
    }

    Ok(candidates)
}

fn burned_in_annotation_status(
    obj: &DefaultDicomObject,
) -> Result<BurnedInAnnotationStatus, DicomAdapterError> {
    let Some(element) = obj.get(Tag(0x0028, 0x0301)) else {
        return Ok(BurnedInAnnotationStatus::Clean);
    };

    let value = element.to_str()?;
    if value.as_ref() == "YES" {
        Ok(BurnedInAnnotationStatus::Suspicious)
    } else {
        Ok(BurnedInAnnotationStatus::Clean)
    }
}

fn dicom_tag_ref(tag: Tag, keyword: &str) -> DicomTagRef {
    DicomTagRef::new(tag.0, tag.1, keyword.into())
}

fn tag_from_ref(tag: &DicomTagRef) -> Tag {
    Tag(tag.group, tag.element)
}

fn is_common_phi_tag(tag: Tag) -> bool {
    COMMON_PHI_TAGS.iter().any(|spec| spec.tag == tag)
}

fn is_uid_family_tag(tag: Tag) -> bool {
    UID_FAMILY_TAGS.iter().any(|uid_tag| *uid_tag == tag)
}

struct CommonPhiTagSpec {
    tag: Tag,
    keyword: &'static str,
    phi_type: &'static str,
}

const COMMON_PHI_TAGS: [CommonPhiTagSpec; 4] = [
    CommonPhiTagSpec {
        tag: Tag(0x0010, 0x0010),
        keyword: "PatientName",
        phi_type: "patient_name",
    },
    CommonPhiTagSpec {
        tag: Tag(0x0010, 0x0020),
        keyword: "PatientID",
        phi_type: "patient_id",
    },
    CommonPhiTagSpec {
        tag: Tag(0x0008, 0x0050),
        keyword: "AccessionNumber",
        phi_type: "accession_number",
    },
    CommonPhiTagSpec {
        tag: Tag(0x0008, 0x1030),
        keyword: "StudyDescription",
        phi_type: "study_description",
    },
];

const UID_FAMILY_TAGS: [Tag; 3] = [
    Tag(0x0020, 0x000D),
    Tag(0x0020, 0x000E),
    Tag(0x0008, 0x0018),
];
