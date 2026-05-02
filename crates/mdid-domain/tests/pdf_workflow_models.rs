use mdid_domain::{
    PdfExtractionSummary, PdfPageRef, PdfPhiCandidate, PdfRewriteStatus, PdfScanStatus,
    ReviewDecision,
};

#[test]
fn pdf_page_ref_builds_a_stable_field_path() {
    let page = PdfPageRef::new(3, "page-3".into());
    assert_eq!(page.field_path(), "pdf/pages/3/page-3");
}

#[test]
fn pdf_page_ref_sanitizes_slashes_in_field_path_labels() {
    let page = PdfPageRef::new(3, "page/3".into());
    assert_eq!(page.field_path(), "pdf/pages/3/page_3");
}

#[test]
fn pdf_scan_status_wire_values_are_stable() {
    assert_eq!(
        serde_json::to_string(&PdfScanStatus::TextLayerPresent).unwrap(),
        "\"text_layer_present\""
    );
    assert_eq!(
        serde_json::to_string(&PdfScanStatus::OcrRequired).unwrap(),
        "\"ocr_required\""
    );
    assert_eq!(
        serde_json::to_string(&PdfScanStatus::HandwritingReviewRequired).unwrap(),
        "\"handwriting_review_required\""
    );
}

#[test]
fn pdf_rewrite_status_wire_value_is_explicitly_review_only() {
    assert_eq!(
        serde_json::to_string(&PdfRewriteStatus::ReviewOnlyNoRewrittenPdf).unwrap(),
        "\"review_only_no_rewritten_pdf\""
    );
}

#[test]
fn pdf_summary_defaults_to_review_only_no_rewritten_pdf() {
    let summary = PdfExtractionSummary::default();

    assert_eq!(
        summary.rewrite_status,
        PdfRewriteStatus::ReviewOnlyNoRewrittenPdf
    );
    assert!(summary.no_rewritten_pdf);
    assert!(summary.review_only);
}

#[test]
fn pdf_phi_candidate_debug_redacts_source_text() {
    let candidate = PdfPhiCandidate {
        page: PdfPageRef::new(1, "page-1".into()),
        phi_type: "patient_name".into(),
        source_text: "Alice Smith".into(),
        confidence: 91,
        decision: ReviewDecision::NeedsReview,
    };

    let debug = format!("{candidate:?}");
    assert!(debug.contains("PdfPhiCandidate"));
    assert!(!debug.contains("Alice Smith"));
}

#[test]
fn pdf_summary_requires_review_when_ocr_or_candidates_need_it() {
    let needs_ocr = PdfExtractionSummary {
        ocr_required_pages: 2,
        ..PdfExtractionSummary::default()
    };
    let needs_review = PdfExtractionSummary {
        review_required_candidates: 1,
        ..PdfExtractionSummary::default()
    };

    assert!(needs_ocr.requires_review());
    assert!(needs_review.requires_review());
    let needs_handwriting_review = PdfExtractionSummary {
        handwriting_review_required_pages: 1,
        ..PdfExtractionSummary::default()
    };
    assert!(needs_handwriting_review.requires_review());
    assert!(!PdfExtractionSummary::default().requires_review());
}
