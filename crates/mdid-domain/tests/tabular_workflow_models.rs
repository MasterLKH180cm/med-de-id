use mdid_domain::{
    BatchSummary, PhiCandidate, ReviewDecision, TabularCellRef, TabularColumn, TabularFormat,
};

#[test]
fn tabular_cell_ref_builds_a_stable_field_path() {
    let cell = TabularCellRef::new(3, 1, "patient/name".into());

    assert_eq!(cell.field_path(), "rows/3/columns/1/patient_name");
}

#[test]
fn tabular_format_uses_explicit_serde_wire_values() {
    assert_eq!(
        serde_json::to_string(&TabularFormat::Csv).unwrap(),
        "\"csv\""
    );
    assert_eq!(
        serde_json::from_str::<TabularFormat>("\"xlsx\"").unwrap(),
        TabularFormat::Xlsx
    );
}

#[test]
fn review_decision_uses_explicit_serde_wire_values() {
    assert_eq!(
        serde_json::to_string(&ReviewDecision::NeedsReview).unwrap(),
        "\"needs_review\""
    );
    assert_eq!(
        serde_json::from_str::<ReviewDecision>("\"approved\"").unwrap(),
        ReviewDecision::Approved
    );
}

#[test]
fn review_decision_reports_when_manual_review_is_required() {
    assert!(ReviewDecision::NeedsReview.requires_human_review());
    assert!(!ReviewDecision::Approved.requires_human_review());
    assert!(ReviewDecision::Approved.allows_encode());
    assert!(!ReviewDecision::Rejected.allows_encode());
}

#[test]
fn batch_summary_flags_partial_failure_when_any_rows_fail() {
    let summary = BatchSummary {
        total_rows: 12,
        encoded_cells: 9,
        review_required_cells: 2,
        failed_rows: 1,
    };

    assert!(summary.is_partial_failure());
}

#[test]
fn batch_summary_is_not_a_partial_failure_when_no_rows_fail() {
    let summary = BatchSummary {
        total_rows: 12,
        encoded_cells: 9,
        review_required_cells: 2,
        failed_rows: 0,
    };

    assert!(!summary.is_partial_failure());
}

#[test]
fn phi_candidate_debug_redacts_source_value() {
    let candidate = PhiCandidate {
        format: TabularFormat::Csv,
        column: TabularColumn::new(1, "patient_name".into(), "string".into()),
        cell: TabularCellRef::new(1, 1, "patient_name".into()),
        phi_type: "patient_name".into(),
        value: "Alice Smith".into(),
        confidence: 98,
        decision: ReviewDecision::NeedsReview,
    };

    let debug = format!("{candidate:?}");

    assert!(debug.contains("PhiCandidate"));
    assert!(!debug.contains("Alice Smith"));
}
