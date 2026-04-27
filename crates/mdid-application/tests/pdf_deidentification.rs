use mdid_application::{PdfDeidentificationOutput, PdfDeidentificationService};
use mdid_domain::{PdfScanStatus, ReviewDecision};

const TEXT_LAYER_PDF: &[u8] =
    include_bytes!("../../mdid-adapters/tests/fixtures/pdf/text-layer-minimal.pdf");
const NO_TEXT_PDF: &[u8] =
    include_bytes!("../../mdid-adapters/tests/fixtures/pdf/no-text-minimal.pdf");

#[test]
fn pdf_deidentification_reports_ocr_required_pages_honestly() {
    let service = PdfDeidentificationService;

    let output = service
        .deidentify_bytes(NO_TEXT_PDF, "scan-only.pdf")
        .expect("scan-only pdf should parse");

    assert_eq!(output.summary.total_pages, 1);
    assert_eq!(output.summary.text_layer_pages, 0);
    assert_eq!(output.summary.ocr_required_pages, 1);
    assert_eq!(output.summary.extracted_candidates, 0);
    assert_eq!(output.summary.review_required_candidates, 0);
    assert!(output.summary.requires_review());
    assert_eq!(output.page_statuses.len(), 1);
    assert_eq!(output.page_statuses[0].status, PdfScanStatus::OcrRequired);
    assert!(output.review_queue.is_empty());
    assert_eq!(output.rewritten_pdf_bytes, None);
}

#[test]
fn pdf_deidentification_routes_text_layer_candidates_to_review() {
    let service = PdfDeidentificationService;

    let output = service
        .deidentify_bytes(TEXT_LAYER_PDF, "patient-record.pdf")
        .expect("text layer pdf should parse");

    assert_eq!(output.summary.total_pages, 1);
    assert_eq!(output.summary.text_layer_pages, 1);
    assert_eq!(output.summary.ocr_required_pages, 0);
    assert_eq!(output.summary.extracted_candidates, 1);
    assert_eq!(output.summary.review_required_candidates, 1);
    assert!(output.summary.requires_review());
    assert_eq!(output.page_statuses.len(), 1);
    assert_eq!(
        output.page_statuses[0].status,
        PdfScanStatus::TextLayerPresent
    );
    assert_eq!(output.rewritten_pdf_bytes, None);
    assert_eq!(output.review_queue.len(), 1);
    assert!(output
        .review_queue
        .iter()
        .all(|candidate| candidate.decision == ReviewDecision::NeedsReview));
    assert!(output
        .review_queue
        .iter()
        .any(|candidate| candidate.source_text.contains("Alice Smith")));
}

#[test]
fn pdf_deidentification_output_debug_redacts_phi() {
    let service = PdfDeidentificationService;

    let output = service
        .deidentify_bytes(TEXT_LAYER_PDF, "alice-smith-record.pdf")
        .expect("text layer pdf should parse");

    assert_debug_redacted(&output);
}

fn assert_debug_redacted(output: &PdfDeidentificationOutput) {
    let debug = format!("{output:?}");

    assert!(debug.contains("PdfDeidentificationOutput"));
    assert!(debug.contains("[REDACTED]"));
    assert!(!debug.contains("alice-smith-record.pdf"));
    assert!(!debug.contains("Alice Smith"));
    assert!(!debug.contains("patient-record.pdf"));
}
