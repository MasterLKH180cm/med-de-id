use lopdf::Document;
use mdid_domain::{
    PdfExtractionSummary, PdfPageRef, PdfPhiCandidate, PdfScanStatus, ReviewDecision,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PdfAdapterError {
    #[error("failed to parse PDF input: {0}")]
    Parse(#[from] lopdf::Error),
}

#[derive(Debug, Clone, Default)]
pub struct PdfAdapter;

impl PdfAdapter {
    pub fn new() -> Self {
        Self
    }

    pub fn extract(
        &self,
        bytes: &[u8],
        source_name: &str,
    ) -> Result<ExtractedPdfData, PdfAdapterError> {
        let document = Document::load_mem(bytes)?;
        let page_numbers = document.get_pages().into_keys().collect::<Vec<_>>();

        let mut pages = Vec::with_capacity(page_numbers.len());
        let mut candidates = Vec::new();
        let mut summary = PdfExtractionSummary {
            total_pages: page_numbers.len(),
            ..PdfExtractionSummary::default()
        };

        for page_number in page_numbers {
            let extracted_text = document.extract_text(&[page_number])?;
            let normalized_fragments = extracted_text
                .lines()
                .map(str::trim)
                .filter(|fragment| !fragment.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>();
            let page = PdfPageRef::new(page_number as usize, format!("page-{page_number}"));

            let status = if normalized_fragments.is_empty() {
                summary.ocr_required_pages += 1;
                PdfScanStatus::OcrRequired
            } else {
                summary.text_layer_pages += 1;
                for fragment in normalized_fragments {
                    candidates.push(PdfPhiCandidate {
                        page: page.clone(),
                        phi_type: "extracted_text".into(),
                        source_text: fragment,
                        confidence: 100,
                        decision: ReviewDecision::NeedsReview,
                    });
                }
                PdfScanStatus::TextLayerPresent
            };

            pages.push(PdfPageExtraction { page, status });
        }

        summary.extracted_candidates = candidates.len();
        summary.review_required_candidates = candidates
            .iter()
            .filter(|candidate| candidate.decision.requires_human_review())
            .count();

        Ok(ExtractedPdfData {
            source_name: source_name.into(),
            pages,
            candidates,
            summary,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfPageExtraction {
    pub page: PdfPageRef,
    pub status: PdfScanStatus,
}

#[derive(Clone)]
pub struct ExtractedPdfData {
    pub source_name: String,
    pub pages: Vec<PdfPageExtraction>,
    pub candidates: Vec<PdfPhiCandidate>,
    pub summary: PdfExtractionSummary,
}

impl std::fmt::Debug for ExtractedPdfData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExtractedPdfData")
            .field("source_name", &"<redacted>")
            .field("pages", &self.pages)
            .field("candidates", &self.candidates)
            .field("summary", &self.summary)
            .finish()
    }
}
