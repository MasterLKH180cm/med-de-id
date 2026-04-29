use mdid_adapters::{ExtractedPdfData, PdfAdapter, PdfAdapterError};
use mdid_domain::{PdfScanStatus, ReviewDecision};

const TEXT_LAYER_PDF: &[u8] = include_bytes!("fixtures/pdf/text-layer-minimal.pdf");
const NO_TEXT_PDF: &[u8] = include_bytes!("fixtures/pdf/no-text-minimal.pdf");
const MIXED_MULTIPAGE_PDF: &[u8] = include_bytes!("fixtures/pdf/mixed-multipage.pdf");

#[test]
fn extract_pdf_text_layer_returns_review_only_candidates_with_placeholder_confidence() {
    let adapter = PdfAdapter::new();

    let extracted = adapter
        .extract(TEXT_LAYER_PDF, "patient-record.pdf")
        .expect("text layer pdf should parse");

    assert_eq!(extracted.pages.len(), 1);
    assert_eq!(extracted.pages[0].page.page_number, 1);
    assert_eq!(extracted.pages[0].status, PdfScanStatus::TextLayerPresent);

    assert_eq!(extracted.candidates.len(), 1);
    let candidate = &extracted.candidates[0];
    assert_eq!(candidate.page.page_number, 1);
    assert_eq!(candidate.phi_type, "extracted_text");
    assert_eq!(candidate.source_text, "Alice Smith");
    assert_eq!(candidate.confidence, 1);
    assert_eq!(candidate.decision, ReviewDecision::NeedsReview);

    assert_eq!(extracted.summary.total_pages, 1);
    assert_eq!(extracted.summary.text_layer_pages, 1);
    assert_eq!(extracted.summary.ocr_required_pages, 0);
    assert_eq!(extracted.summary.extracted_candidates, 1);
    assert_eq!(extracted.summary.review_required_candidates, 1);
    assert!(extracted.summary.requires_review());
}

#[test]
fn extract_pdf_with_no_extractable_text_marks_ocr_required_without_claiming_detected_phi() {
    let adapter = PdfAdapter::new();

    let extracted = adapter
        .extract(NO_TEXT_PDF, "scan-only.pdf")
        .expect("no-text pdf should parse");

    assert_eq!(extracted.pages.len(), 1);
    assert_eq!(extracted.pages[0].status, PdfScanStatus::OcrRequired);
    assert!(
        extracted.candidates.is_empty(),
        "a page with no extractable text should only be flagged for OCR review"
    );
    assert_eq!(extracted.summary.total_pages, 1);
    assert_eq!(extracted.summary.text_layer_pages, 0);
    assert_eq!(extracted.summary.ocr_required_pages, 1);
    assert_eq!(extracted.summary.extracted_candidates, 0);
    assert_eq!(extracted.summary.review_required_candidates, 0);
    assert!(extracted.summary.requires_review());
}

#[test]
fn extract_pdf_mixed_multipage_reports_page_order_and_mixed_statuses() {
    let adapter = PdfAdapter::new();

    let extracted = adapter
        .extract(MIXED_MULTIPAGE_PDF, "mixed-multipage.pdf")
        .expect("mixed multipage pdf should parse");

    assert_eq!(extracted.pages.len(), 2);
    assert_eq!(
        extracted
            .pages
            .iter()
            .map(|page| page.page.page_number)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert_eq!(extracted.pages[0].status, PdfScanStatus::TextLayerPresent);
    assert_eq!(extracted.pages[1].status, PdfScanStatus::OcrRequired);

    assert_eq!(extracted.summary.total_pages, 2);
    assert_eq!(extracted.summary.text_layer_pages, 1);
    assert_eq!(extracted.summary.ocr_required_pages, 1);
    assert_eq!(extracted.summary.extracted_candidates, 1);
    assert_eq!(extracted.summary.review_required_candidates, 1);

    assert_eq!(extracted.candidates.len(), 1);
    let candidate = &extracted.candidates[0];
    assert_eq!(candidate.page.page_number, 1);
    assert_eq!(candidate.phi_type, "extracted_text");
    assert_eq!(candidate.source_text, "Alice Smith");
    assert_eq!(candidate.decision, ReviewDecision::NeedsReview);
}

#[test]
fn extract_pdf_invalid_bytes_returns_parse_error() {
    let adapter = PdfAdapter::new();

    let error = adapter
        .extract(b"this is definitely not a pdf", "invalid.pdf")
        .expect_err("invalid bytes should return a parse error");

    assert!(matches!(error, PdfAdapterError::Parse(_)));
}

#[test]
fn extracted_pdf_data_debug_redacts_source_name() {
    let adapter = PdfAdapter::new();
    let extracted: ExtractedPdfData = adapter
        .extract(TEXT_LAYER_PDF, "alice-smith-record.pdf")
        .expect("text layer pdf should parse");

    let debug = format!("{extracted:?}");

    assert!(debug.contains("ExtractedPdfData"));
    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("alice-smith-record.pdf"));
}
