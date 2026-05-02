use mdid_adapters::PdfAdapterError;
use mdid_application::{ApplicationError, PdfDeidentificationOutput, PdfDeidentificationService};
use mdid_domain::{PdfRewriteStatus, PdfScanStatus, ReviewDecision};

const TEXT_LAYER_PDF: &[u8] =
    include_bytes!("../../mdid-adapters/tests/fixtures/pdf/text-layer-minimal.pdf");
const NO_TEXT_PDF: &[u8] =
    include_bytes!("../../mdid-adapters/tests/fixtures/pdf/no-text-minimal.pdf");
const MIXED_MULTIPAGE_PDF: &[u8] =
    include_bytes!("../../mdid-adapters/tests/fixtures/pdf/mixed-multipage.pdf");
const INVALID_PDF_BYTES: &[u8] = b"not a pdf";

fn clinic_note_text_layer_pdf() -> Vec<u8> {
    let mut pdf = TEXT_LAYER_PDF.to_vec();
    let needle = b"Alice Smith";
    let offset = pdf
        .windows(needle.len())
        .position(|window| window == needle)
        .expect("fixture should contain Alice Smith text fragment");
    pdf[offset..offset + needle.len()].copy_from_slice(b"ClinicNote ");
    pdf
}

#[test]
fn pdf_deidentification_routes_handwriting_suspicion_to_manual_review_without_rewrite() {
    let output = PdfDeidentificationService
        .deidentify_bytes(NO_TEXT_PDF, "handwritten-intake.pdf")
        .expect("handwriting-suspected pdf should parse");

    assert_eq!(output.summary.handwriting_review_required_pages, 1);
    assert!(output.summary.requires_review());
    assert_eq!(
        output.page_statuses[0].status,
        PdfScanStatus::HandwritingReviewRequired
    );
    assert!(output.no_rewritten_pdf);
    assert!(output.review_only);
    assert_eq!(output.rewritten_pdf_bytes, None);
}

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
    assert_eq!(
        output.rewrite_status,
        PdfRewriteStatus::ReviewOnlyNoRewrittenPdf
    );
    assert!(output.no_rewritten_pdf);
    assert!(output.review_only);
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
fn pdf_deidentification_exports_clean_one_page_text_layer_without_review_queue() {
    let service = PdfDeidentificationService;
    let pdf = clinic_note_text_layer_pdf();

    let output = service
        .deidentify_bytes(&pdf, "clinic-note.pdf")
        .expect("clean one-page text layer pdf should parse");

    assert_eq!(output.summary.total_pages, 1);
    assert!(output.summary.total_pages > 0);
    assert_eq!(output.summary.text_layer_pages, 1);
    assert_eq!(output.summary.ocr_required_pages, 0);
    assert_eq!(output.summary.extracted_candidates, 0);
    assert_eq!(output.summary.review_required_candidates, 0);
    assert!(!output.summary.requires_review());
    assert_eq!(output.page_statuses.len(), 1);
    assert_eq!(
        output.page_statuses[0].status,
        PdfScanStatus::TextLayerPresent
    );
    assert_eq!(output.review_queue.len(), 0);
    assert_eq!(
        output.rewrite_status,
        PdfRewriteStatus::CleanTextLayerPdfBytesAvailable
    );
    assert!(!output.no_rewritten_pdf);
    assert!(!output.review_only);
    assert_eq!(output.rewritten_pdf_bytes.as_deref(), Some(pdf.as_slice()));
}

#[test]
fn pdf_deidentification_reports_mixed_page_statuses_honestly() {
    let service = PdfDeidentificationService;

    let output = service
        .deidentify_bytes(MIXED_MULTIPAGE_PDF, "mixed-multipage.pdf")
        .expect("mixed pdf should parse");

    assert_eq!(output.page_statuses.len(), 2);
    assert_eq!(output.page_statuses[0].page.page_number, 1);
    assert_eq!(
        output.page_statuses[0].status,
        PdfScanStatus::TextLayerPresent
    );
    assert_eq!(output.page_statuses[1].page.page_number, 2);
    assert_eq!(output.page_statuses[1].status, PdfScanStatus::OcrRequired);
    assert_eq!(output.summary.total_pages, 2);
    assert_eq!(output.summary.text_layer_pages, 1);
    assert_eq!(output.summary.ocr_required_pages, 1);
    assert_eq!(output.summary.extracted_candidates, 1);
    assert_eq!(output.summary.review_required_candidates, 1);
    assert!(output.summary.requires_review());
    assert_eq!(output.review_queue.len(), 1);
    let candidate = &output.review_queue[0];
    assert_eq!(candidate.page.page_number, 1);
    assert_eq!(candidate.decision, ReviewDecision::NeedsReview);
    assert_eq!(candidate.source_text, "Alice Smith");
    assert_eq!(output.rewritten_pdf_bytes, None);
}

#[test]
fn pdf_deidentification_returns_parse_failure_for_invalid_pdf_bytes() {
    let service = PdfDeidentificationService;

    let error = service
        .deidentify_bytes(INVALID_PDF_BYTES, "invalid.pdf")
        .expect_err("invalid bytes should not return a fake partial success object");

    assert!(matches!(
        error,
        ApplicationError::PdfAdapter(PdfAdapterError::Parse(_))
    ));
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
