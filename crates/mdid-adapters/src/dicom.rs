use std::io::Cursor;

use dicom_core::{header::Header, value::ConvertValueError, Tag};
use dicom_object::{file::ReadPreamble, DefaultDicomObject, OpenFileOptions, ReadError};
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

fn common_phi_candidates(
    obj: &DefaultDicomObject,
) -> Result<Vec<DicomPhiCandidate>, DicomAdapterError> {
    let mut candidates = Vec::new();

    for spec in COMMON_PHI_TAGS {
        if let Some(element) = obj.get(spec.tag) {
            candidates.push(DicomPhiCandidate {
                tag: dicom_tag_ref(spec.tag, spec.keyword),
                phi_type: spec.phi_type.into(),
                value: element.to_str()?.into_owned(),
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
