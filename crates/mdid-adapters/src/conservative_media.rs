use mdid_domain::{
    ConservativeMediaCandidate, ConservativeMediaFormat, ConservativeMediaRef,
    ConservativeMediaScanStatus, ConservativeMediaSummary,
};

const CONSERVATIVE_METADATA_CONFIDENCE: f32 = 0.35;
const METADATA_IDENTIFIER_PHI_TYPE: &str = "metadata_identifier";

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConservativeMediaMetadataEntry {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConservativeMediaInput {
    pub artifact_label: String,
    pub format: ConservativeMediaFormat,
    pub metadata: Vec<ConservativeMediaMetadataEntry>,
    pub requires_visual_review: bool,
    pub unsupported_payload: bool,
}

#[derive(Clone)]
pub struct ExtractedConservativeMediaData {
    pub candidates: Vec<ConservativeMediaCandidate>,
    pub summary: ConservativeMediaSummary,
}

impl std::fmt::Debug for ExtractedConservativeMediaData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExtractedConservativeMediaData")
            .field("candidates", &self.candidates)
            .field("summary", &self.summary)
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
            .map(|entry| ConservativeMediaCandidate {
                field_ref: ConservativeMediaRef {
                    artifact_label: input.artifact_label.clone(),
                    metadata_key: entry.key,
                },
                format: input.format,
                phi_type: METADATA_IDENTIFIER_PHI_TYPE.to_string(),
                source_value: entry.value,
                confidence: CONSERVATIVE_METADATA_CONFIDENCE,
                status,
            })
            .collect::<Vec<_>>();

        summary.review_required_candidates = candidates.len();

        Ok(ExtractedConservativeMediaData {
            candidates,
            summary,
        })
    }
}
