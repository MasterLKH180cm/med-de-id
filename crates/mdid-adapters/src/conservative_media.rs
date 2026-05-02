use mdid_domain::{
    ConservativeMediaCandidate, ConservativeMediaFormat, ConservativeMediaRef,
    ConservativeMediaScanStatus, ConservativeMediaSummary,
};

const CONSERVATIVE_METADATA_CONFIDENCE: f32 = 0.35;
const METADATA_IDENTIFIER_PHI_TYPE: &str = "metadata_identifier";

fn classify_conservative_media_phi_type(
    format: ConservativeMediaFormat,
    metadata_key: &str,
) -> &'static str {
    if format != ConservativeMediaFormat::Fcs {
        return METADATA_IDENTIFIER_PHI_TYPE;
    }

    match metadata_key.trim().to_ascii_uppercase().as_str() {
        "$FIL" | "FILENAME" | "FILE" => "fcs_filename_identifier",
        "$SMNO" | "SMNO" | "SAMPLE_ID" | "SAMPLEID" | "SPECIMEN_ID" => "fcs_sample_identifier",
        "$SRC" | "SRC" | "SOURCE" | "SPECIMEN_SOURCE" => "fcs_source_identifier",
        "$OP" | "OP" | "OPERATOR" | "CREATOR" => "fcs_operator_identifier",
        "$DATE" | "DATE" | "COLLECTION_DATE" | "ACQUISITION_DATE" => "fcs_collection_date",
        _ => METADATA_IDENTIFIER_PHI_TYPE,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConservativeMediaAdapterError {
    EmptyArtifactLabel,
}

impl std::fmt::Display for ConservativeMediaAdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyArtifactLabel => f.write_str("artifact label must not be empty"),
        }
    }
}

impl std::error::Error for ConservativeMediaAdapterError {}

#[derive(Clone, PartialEq, Eq)]
pub struct ConservativeMediaMetadataEntry {
    pub key: String,
    pub value: String,
}

impl std::fmt::Debug for ConservativeMediaMetadataEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConservativeMediaMetadataEntry")
            .field("key", &self.key)
            .field("value", &"<redacted>")
            .finish()
    }
}

#[derive(Clone, PartialEq)]
pub struct ConservativeMediaInput {
    pub artifact_label: String,
    pub format: ConservativeMediaFormat,
    pub metadata: Vec<ConservativeMediaMetadataEntry>,
    pub requires_visual_review: bool,
    pub unsupported_payload: bool,
}

impl std::fmt::Debug for ConservativeMediaInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConservativeMediaInput")
            .field("artifact_label", &"<redacted>")
            .field("format", &self.format)
            .field("metadata", &self.metadata)
            .field("requires_visual_review", &self.requires_visual_review)
            .field("unsupported_payload", &self.unsupported_payload)
            .finish()
    }
}

#[derive(Clone)]
pub struct ExtractedConservativeMediaData {
    pub candidates: Vec<ConservativeMediaCandidate>,
    pub summary: ConservativeMediaSummary,
}

impl std::fmt::Debug for ExtractedConservativeMediaData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExtractedConservativeMediaData")
            .field(
                "candidates",
                &RedactedConservativeMediaCandidates(&self.candidates),
            )
            .field("summary", &self.summary)
            .finish()
    }
}

struct RedactedConservativeMediaCandidates<'a>(&'a [ConservativeMediaCandidate]);

impl std::fmt::Debug for RedactedConservativeMediaCandidates<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut list = f.debug_list();
        for candidate in self.0 {
            list.entry(&RedactedConservativeMediaCandidate(candidate));
        }
        list.finish()
    }
}

struct RedactedConservativeMediaCandidate<'a>(&'a ConservativeMediaCandidate);

impl std::fmt::Debug for RedactedConservativeMediaCandidate<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let candidate = self.0;
        f.debug_struct("ConservativeMediaCandidate")
            .field("field_ref", &"<redacted>")
            .field("format", &candidate.format)
            .field("phi_type", &candidate.phi_type)
            .field("source_value", &"<redacted>")
            .field("confidence", &candidate.confidence)
            .field("status", &candidate.status)
            .finish()
    }
}

pub struct ConservativeMediaAdapter;

impl ConservativeMediaAdapter {
    pub fn extract_metadata(
        input: ConservativeMediaInput,
    ) -> Result<ExtractedConservativeMediaData, ConservativeMediaAdapterError> {
        if input.artifact_label.trim().is_empty() {
            return Err(ConservativeMediaAdapterError::EmptyArtifactLabel);
        }

        let mut summary = ConservativeMediaSummary {
            total_items: 1,
            metadata_only_items: 0,
            visual_review_required_items: 0,
            unsupported_items: 0,
            review_required_candidates: 0,
        };

        if input.unsupported_payload {
            summary.unsupported_items = 1;
            return Ok(ExtractedConservativeMediaData {
                candidates: Vec::new(),
                summary,
            });
        }

        let status = if input.requires_visual_review {
            summary.visual_review_required_items = 1;
            ConservativeMediaScanStatus::OcrOrVisualReviewRequired
        } else {
            summary.metadata_only_items = 1;
            ConservativeMediaScanStatus::MetadataOnly
        };

        let candidates = input
            .metadata
            .into_iter()
            .filter(|entry| !entry.key.trim().is_empty() && !entry.value.trim().is_empty())
            .map(|entry| {
                let phi_type =
                    classify_conservative_media_phi_type(input.format, entry.key.as_str());

                ConservativeMediaCandidate {
                    field_ref: ConservativeMediaRef {
                        artifact_label: input.artifact_label.clone(),
                        metadata_key: entry.key,
                    },
                    format: input.format,
                    phi_type: phi_type.to_string(),
                    source_value: entry.value,
                    confidence: CONSERVATIVE_METADATA_CONFIDENCE,
                    status,
                }
            })
            .collect::<Vec<_>>();

        summary.review_required_candidates = candidates.len();

        Ok(ExtractedConservativeMediaData {
            candidates,
            summary,
        })
    }
}
