use mdid_adapters::{CsvTabularAdapter, FieldPolicy, XlsxTabularAdapter};
use mdid_domain::ReviewDecision;
use rust_xlsxwriter::Workbook;

#[test]
fn xlsx_adapter_uses_first_non_empty_worksheet_and_matches_csv_semantics() {
    let workbook = workbook_with_blank_cover_sheet(vec![
        vec!["patient_id", "patient_name", "age"],
        vec!["MRN-001", "Alice Smith", "42"],
        vec!["MRN-002", "Bob Jones", "37"],
    ]);
    let csv_input = "patient_id,patient_name,age\nMRN-001,Alice Smith,42\nMRN-002,Bob Jones,37\n";
    let xlsx_adapter = XlsxTabularAdapter::new(vec![
        FieldPolicy::encode("patient_id", "patient_id"),
        FieldPolicy::review("patient_name", "patient_name"),
    ]);
    let csv_adapter = CsvTabularAdapter::new(vec![
        FieldPolicy::encode("patient_id", "patient_id"),
        FieldPolicy::review("patient_name", "patient_name"),
    ]);

    let extracted = xlsx_adapter.extract(&workbook).unwrap();
    let csv_extracted = csv_adapter.extract(csv_input.as_bytes()).unwrap();

    assert_eq!(extracted.columns, csv_extracted.columns);
    assert_eq!(extracted.rows, csv_extracted.rows);
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
    assert_eq!(
        candidate_summary(&extracted),
        candidate_summary(&csv_extracted)
    );
}

#[test]
fn xlsx_adapter_skips_whitespace_only_cells_when_building_candidates() {
    let workbook = workbook_fixture(vec![
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

fn workbook_with_blank_cover_sheet(rows: Vec<Vec<&str>>) -> Vec<u8> {
    let mut workbook = Workbook::new();
    let _ = workbook.add_worksheet();
    write_rows(workbook.add_worksheet(), &rows);

    workbook
        .save_to_buffer()
        .expect("fixture workbook serialization should succeed")
}

fn workbook_fixture(rows: Vec<Vec<&str>>) -> Vec<u8> {
    let mut workbook = Workbook::new();
    write_rows(workbook.add_worksheet(), &rows);

    workbook
        .save_to_buffer()
        .expect("fixture workbook serialization should succeed")
}

fn write_rows(
    worksheet: &mut rust_xlsxwriter::Worksheet,
    rows: &[Vec<&str>],
) {
    for (row_index, row) in rows.iter().enumerate() {
        for (column_index, value) in row.iter().enumerate() {
            worksheet
                .write_string(row_index as u32, column_index as u16, *value)
                .expect("fixture workbook cell write should succeed");
        }
    }
}

fn candidate_summary(
    extracted: &mdid_adapters::ExtractedTabularData,
) -> Vec<(usize, usize, String, String, ReviewDecision)> {
    extracted
        .candidates
        .iter()
        .map(|candidate| {
            (
                candidate.cell.row_index,
                candidate.cell.column_index,
                candidate.phi_type.clone(),
                candidate.value.clone(),
                candidate.decision,
            )
        })
        .collect()
}
