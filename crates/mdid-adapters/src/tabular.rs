use mdid_domain::{PhiCandidate, ReviewDecision, TabularCellRef, TabularColumn, TabularFormat};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TabularAdapterError {
    #[error("failed to parse CSV input: {0}")]
    Csv(#[from] csv::Error),
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

                let Some(policy) = self
                    .policies
                    .iter()
                    .find(|policy| policy.header == headers[column_index])
                else {
                    continue;
                };

                let decision = match policy.action {
                    FieldPolicyAction::Encode => ReviewDecision::Approved,
                    FieldPolicyAction::Review => ReviewDecision::NeedsReview,
                    FieldPolicyAction::Ignore => continue,
                };

                candidates.push(PhiCandidate {
                    format: TabularFormat::Csv,
                    column: columns[column_index].clone(),
                    cell: TabularCellRef::new(
                        row_index,
                        column_index,
                        headers[column_index].clone(),
                    ),
                    phi_type: policy.phi_type.clone(),
                    value: value.clone(),
                    confidence: 100,
                    decision,
                });
            }
        }

        Ok(ExtractedTabularData {
            format: TabularFormat::Csv,
            columns,
            rows,
            candidates,
        })
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
