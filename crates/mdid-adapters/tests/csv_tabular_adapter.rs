use mdid_adapters::{CsvTabularAdapter, FieldPolicy, FieldPolicyAction};
use mdid_domain::ReviewDecision;

#[test]
fn csv_adapter_infers_schema_and_marks_review_columns() {
    let csv_input = "patient_id,patient_name,age\nMRN-001,Alice Smith,42\n";
    let adapter = CsvTabularAdapter::new(vec![
        FieldPolicy::encode("patient_id", "patient_id"),
        FieldPolicy::review("patient_name", "patient_name"),
    ]);

    let extracted = adapter.extract(csv_input.as_bytes()).unwrap();

    assert_eq!(extracted.columns.len(), 3);
    assert_eq!(extracted.columns[0].name, "patient_id");
    assert_eq!(extracted.columns[2].inferred_kind, "integer");
    assert_eq!(extracted.candidates.len(), 2);
    assert_eq!(extracted.candidates[0].decision, ReviewDecision::Approved);
    assert_eq!(
        extracted.candidates[1].decision,
        ReviewDecision::NeedsReview
    );
}

#[test]
fn utf8_bom_prefixed_first_header_matches_field_policy() {
    let csv_input = "\u{feff}patient_id,patient_name\nMRN-001,Alice Smith\n";
    let adapter = CsvTabularAdapter::new(vec![FieldPolicy::encode("patient_id", "patient_id")]);

    let extracted = adapter.extract(csv_input.as_bytes()).unwrap();

    assert_eq!(extracted.columns[0].name, "patient_id");
    assert_eq!(extracted.candidates.len(), 1);
    assert_eq!(extracted.candidates[0].cell.header, "patient_id");
    assert_eq!(extracted.candidates[0].value, "MRN-001");
}

#[test]
fn field_policy_helpers_assign_expected_actions() {
    let encode = FieldPolicy::encode("patient_id", "patient_id");
    let review = FieldPolicy::review("patient_name", "patient_name");

    assert_eq!(encode.action, FieldPolicyAction::Encode);
    assert_eq!(review.action, FieldPolicyAction::Review);
}

#[test]
fn extracted_tabular_data_debug_redacts_raw_rows() {
    let csv_input = "patient_id,patient_name\nMRN-001,Alice Smith\n";
    let adapter = CsvTabularAdapter::new(vec![
        FieldPolicy::encode("patient_id", "patient_id"),
        FieldPolicy::review("patient_name", "patient_name"),
    ]);

    let extracted = adapter.extract(csv_input.as_bytes()).unwrap();
    let debug = format!("{extracted:?}");

    assert!(debug.contains("ExtractedTabularData"));
    assert!(debug.contains("rows_len"));
    assert!(!debug.contains("MRN-001"));
    assert!(!debug.contains("Alice Smith"));
}

#[test]
fn whitespace_only_cells_do_not_create_phi_candidates() {
    let csv_input = "patient_id,patient_name,notes\n   ,\t  ,kept\n";
    let adapter = CsvTabularAdapter::new(vec![
        FieldPolicy::encode("patient_id", "patient_id"),
        FieldPolicy::review("patient_name", "patient_name"),
    ]);

    let extracted = adapter.extract(csv_input.as_bytes()).unwrap();

    assert!(extracted.candidates.is_empty());
}
