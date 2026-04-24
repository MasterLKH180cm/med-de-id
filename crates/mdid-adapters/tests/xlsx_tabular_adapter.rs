use mdid_adapters::{FieldPolicy, XlsxTabularAdapter};
use mdid_domain::ReviewDecision;

#[test]
fn xlsx_adapter_reads_headers_preserves_rows_and_matches_csv_semantics() {
    let workbook = XlsxTabularAdapter::fixture_bytes(vec![
        vec!["patient_id", "patient_name", "age"],
        vec!["MRN-001", "Alice Smith", "42"],
        vec!["MRN-002", "Bob Jones", "37"],
    ]);
    let adapter = XlsxTabularAdapter::new(vec![
        FieldPolicy::encode("patient_id", "patient_id"),
        FieldPolicy::review("patient_name", "patient_name"),
    ]);

    let extracted = adapter.extract(&workbook).unwrap();

    assert_eq!(extracted.columns.len(), 3);
    assert_eq!(extracted.columns[0].name, "patient_id");
    assert_eq!(extracted.columns[2].inferred_kind, "integer");
    assert_eq!(extracted.rows.len(), 2);
    assert_eq!(extracted.rows[0], vec!["MRN-001", "Alice Smith", "42"]);
    assert_eq!(extracted.candidates.len(), 4);
    assert_eq!(extracted.candidates[0].decision, ReviewDecision::Approved);
    assert_eq!(
        extracted.candidates[1].decision,
        ReviewDecision::NeedsReview
    );
}

#[test]
fn xlsx_adapter_skips_whitespace_only_cells_when_building_candidates() {
    let workbook = XlsxTabularAdapter::fixture_bytes(vec![
        vec!["patient_id", "patient_name", "notes"],
        vec!["   ", "\t  ", "kept"],
    ]);
    let adapter = XlsxTabularAdapter::new(vec![
        FieldPolicy::encode("patient_id", "patient_id"),
        FieldPolicy::review("patient_name", "patient_name"),
    ]);

    let extracted = adapter.extract(&workbook).unwrap();

    assert!(extracted.candidates.is_empty());
}
