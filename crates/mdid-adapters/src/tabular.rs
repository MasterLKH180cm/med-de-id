use std::io::Cursor;

use calamine::{open_workbook_from_rs, Data, Reader, Xlsx, XlsxError};
use mdid_domain::{PhiCandidate, ReviewDecision, TabularCellRef, TabularColumn, TabularFormat};
use rust_xlsxwriter::Workbook;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TabularAdapterError {
    #[error("failed to parse CSV input: {0}")]
    Csv(#[from] csv::Error),
    #[error("failed to parse XLSX input: {0}")]
    Xlsx(#[from] XlsxError),
    #[error("xlsx workbook did not contain any worksheets")]
    MissingWorksheet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldPolicyAction {
    Encode,
    Review,
    Ignore,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldPolicy {
    pub header: String,
    pub phi_type: String,
    pub action: FieldPolicyAction,
}

impl FieldPolicy {
    pub fn encode(header: &str, phi_type: &str) -> Self {
        Self {
            header: header.into(),
            phi_type: phi_type.into(),
            action: FieldPolicyAction::Encode,
        }
    }

    pub fn review(header: &str, phi_type: &str) -> Self {
        Self {
            header: header.into(),
            phi_type: phi_type.into(),
            action: FieldPolicyAction::Review,
        }
    }
}

#[derive(Clone)]
pub struct ExtractedTabularData {
    pub format: TabularFormat,
    pub columns: Vec<TabularColumn>,
    pub rows: Vec<Vec<String>>,
    pub candidates: Vec<PhiCandidate>,
}

impl std::fmt::Debug for ExtractedTabularData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExtractedTabularData")
            .field("format", &self.format)
            .field("columns", &self.columns)
            .field("rows_len", &self.rows.len())
            .field("candidates", &self.candidates)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct CsvTabularAdapter {
    policies: Vec<FieldPolicy>,
}

impl CsvTabularAdapter {
    pub fn new(policies: Vec<FieldPolicy>) -> Self {
        Self { policies }
    }

    pub fn extract(&self, bytes: &[u8]) -> Result<ExtractedTabularData, TabularAdapterError> {
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(bytes);
        let headers = reader
            .headers()?
            .iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();

        let mut rows = Vec::new();
        for record in reader.records() {
            let record = record?;
            rows.push(record.iter().map(str::to_owned).collect::<Vec<_>>());
        }

        Ok(build_extracted_data(
            TabularFormat::Csv,
            headers,
            rows,
            &self.policies,
        ))
    }
}

#[derive(Debug, Clone)]
pub struct XlsxTabularAdapter {
    policies: Vec<FieldPolicy>,
}

impl XlsxTabularAdapter {
    pub fn new(policies: Vec<FieldPolicy>) -> Self {
        Self { policies }
    }

    pub fn extract(&self, bytes: &[u8]) -> Result<ExtractedTabularData, TabularAdapterError> {
        let mut workbook = open_workbook_from_rs::<Xlsx<_>, _>(Cursor::new(bytes))?;
        let sheet_names = workbook.sheet_names().to_owned();
        let mut selected_rows = None;

        for (sheet_index, sheet_name) in sheet_names.iter().enumerate() {
            let rows = worksheet_rows(workbook.worksheet_range(sheet_name)?);
            let has_non_blank_cells = worksheet_has_non_blank_cells(&rows);

            if sheet_index == 0 {
                selected_rows = Some(rows);
                if has_non_blank_cells {
                    break;
                }
                continue;
            }

            if has_non_blank_cells {
                selected_rows = Some(rows);
                break;
            }
        }

        let mut rows = selected_rows.ok_or(TabularAdapterError::MissingWorksheet)?;
        let headers = rows.first().cloned().unwrap_or_default();
        let data_rows = if rows.is_empty() {
            Vec::new()
        } else {
            rows.remove(0);
            rows
        };

        Ok(build_extracted_data(
            TabularFormat::Xlsx,
            headers,
            data_rows,
            &self.policies,
        ))
    }

    pub fn fixture_bytes(rows: Vec<Vec<&str>>) -> Vec<u8> {
        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();

        for (row_index, row) in rows.iter().enumerate() {
            for (column_index, value) in row.iter().enumerate() {
                worksheet
                    .write_string(row_index as u32, column_index as u16, *value)
                    .expect("fixture workbook cell write should succeed");
            }
        }

        workbook
            .save_to_buffer()
            .expect("fixture workbook serialization should succeed")
    }
}

fn worksheet_rows(range: calamine::Range<Data>) -> Vec<Vec<String>> {
    range
        .rows()
        .map(|row| row.iter().map(cell_to_string).collect::<Vec<_>>())
        .collect()
}

fn worksheet_has_non_blank_cells(rows: &[Vec<String>]) -> bool {
    rows.iter().flatten().any(|value| !is_blank(value))
}

fn build_extracted_data(
    format: TabularFormat,
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    policies: &[FieldPolicy],
) -> ExtractedTabularData {
    let columns = headers
        .iter()
        .enumerate()
        .map(|(index, header)| {
            TabularColumn::new(index, header.clone(), infer_kind(&rows, index).into())
        })
        .collect::<Vec<_>>();

    let mut candidates = Vec::new();
    for (row_index, row) in rows.iter().enumerate() {
        for (column_index, value) in row.iter().enumerate() {
            if is_blank(value) {
                continue;
            }

            let Some(header) = headers.get(column_index) else {
                continue;
            };

            let Some(policy) = policies.iter().find(|policy| policy.header == *header) else {
                continue;
            };

            let decision = match policy.action {
                FieldPolicyAction::Encode => ReviewDecision::Approved,
                FieldPolicyAction::Review => ReviewDecision::NeedsReview,
                FieldPolicyAction::Ignore => continue,
            };

            candidates.push(PhiCandidate {
                format,
                column: columns[column_index].clone(),
                cell: TabularCellRef::new(row_index, column_index, header.clone()),
                phi_type: policy.phi_type.clone(),
                value: value.clone(),
                confidence: 100,
                decision,
            });
        }
    }

    ExtractedTabularData {
        format,
        columns,
        rows,
        candidates,
    }
}

fn cell_to_string(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        _ => cell.to_string(),
    }
}

fn infer_kind(rows: &[Vec<String>], column_index: usize) -> &'static str {
    let mut saw_value = false;

    for row in rows {
        let Some(value) = row.get(column_index) else {
            continue;
        };

        if is_blank(value) {
            continue;
        }

        saw_value = true;
        if value.trim().parse::<i64>().is_err() {
            return "string";
        }
    }

    if saw_value {
        "integer"
    } else {
        "string"
    }
}

fn is_blank(value: &str) -> bool {
    value.trim().is_empty()
}
