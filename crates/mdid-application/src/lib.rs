use chrono::Utc;
use mdid_adapters::{CsvTabularAdapter, ExtractedTabularData, FieldPolicy, TabularAdapterError};
use mdid_domain::{
    BatchSummary, MappingScope, PhiCandidate, PipelineDefinition, PipelineRun, PipelineRunState,
    SurfaceKind, TabularColumn,
};
use mdid_vault::{LocalVaultStore, NewMappingRecord, VaultError};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{Arc, Mutex};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error("pipeline not found: {0}")]
    PipelineNotFound(Uuid),
    #[error(transparent)]
    TabularAdapter(#[from] TabularAdapterError),
    #[error(transparent)]
    Vault(#[from] VaultError),
    #[error("csv rewrite failure: {0}")]
    Csv(#[from] csv::Error),
    #[error("io failure: {0}")]
    Io(#[from] std::io::Error),
    #[error("utf8 conversion failure: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}

#[derive(Clone, Default)]
pub struct ApplicationService {
    pipelines: Arc<Mutex<HashMap<Uuid, PipelineDefinition>>>,
}

impl ApplicationService {
    pub fn register_pipeline(&self, name: String) -> PipelineDefinition {
        let pipeline = PipelineDefinition {
            id: Uuid::new_v4(),
            name,
            created_at: Utc::now(),
        };
        self.pipelines
            .lock()
            .expect("pipelines lock poisoned")
            .insert(pipeline.id, pipeline.clone());
        pipeline
    }

    pub fn start_run(
        &self,
        pipeline_id: Uuid,
        started_by: SurfaceKind,
    ) -> Result<PipelineRun, ApplicationError> {
        let has_pipeline = self
            .pipelines
            .lock()
            .expect("pipelines lock poisoned")
            .contains_key(&pipeline_id);

        if !has_pipeline {
            return Err(ApplicationError::PipelineNotFound(pipeline_id));
        }

        Ok(PipelineRun {
            id: Uuid::new_v4(),
            pipeline_id,
            state: PipelineRunState::Pending,
            started_by,
            created_at: Utc::now(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct TabularDeidentificationOutput {
    pub csv: String,
    pub summary: BatchSummary,
    pub review_queue: Vec<PhiCandidate>,
}

#[derive(Clone, Default)]
pub struct TabularDeidentificationService;

impl TabularDeidentificationService {
    pub fn deidentify_csv(
        &self,
        csv: &str,
        policies: &[FieldPolicy],
        vault: &mut LocalVaultStore,
        actor: SurfaceKind,
    ) -> Result<TabularDeidentificationOutput, ApplicationError> {
        let adapter = CsvTabularAdapter::new(policies.to_vec());
        let extracted = adapter.extract(csv.as_bytes())?;
        self.deidentify_extracted(extracted, vault, actor)
    }

    pub fn deidentify_extracted(
        &self,
        extracted: ExtractedTabularData,
        vault: &mut LocalVaultStore,
        actor: SurfaceKind,
    ) -> Result<TabularDeidentificationOutput, ApplicationError> {
        let job_id = Uuid::new_v4();
        let artifact_id = Uuid::new_v4();
        let mut summary = BatchSummary {
            total_rows: extracted.rows.len(),
            ..BatchSummary::default()
        };
        let mut review_queue = Vec::new();
        let mut rewritten_rows = extracted.rows.clone();
        let mut candidates_by_row = BTreeMap::<usize, Vec<PhiCandidate>>::new();
        let mut failed_rows = BTreeSet::new();

        for candidate in extracted.candidates {
            if candidate.decision.requires_human_review() {
                summary.review_required_cells += 1;
                review_queue.push(candidate);
                continue;
            }

            if !candidate.decision.allows_encode() {
                continue;
            }

            candidates_by_row
                .entry(candidate.cell.row_index)
                .or_default()
                .push(candidate);
        }

        for (row_index, row_candidates) in candidates_by_row {
            let Some(row) = rewritten_rows.get_mut(row_index) else {
                failed_rows.insert(row_index);
                continue;
            };

            if row_candidates
                .iter()
                .any(|candidate| row.get(candidate.cell.column_index).is_none())
            {
                failed_rows.insert(row_index);
                continue;
            }

            for candidate in row_candidates {
                let mapping = vault.ensure_mapping(
                    NewMappingRecord {
                        scope: MappingScope::new(job_id, artifact_id, candidate.cell.field_path()),
                        phi_type: candidate.phi_type.clone(),
                        original_value: candidate.value.clone(),
                    },
                    actor,
                )?;

                row[candidate.cell.column_index] = mapping.token;
                summary.encoded_cells += 1;
            }
        }

        summary.failed_rows = failed_rows.len();

        Ok(TabularDeidentificationOutput {
            csv: write_csv(&extracted.columns, &rewritten_rows)?,
            summary,
            review_queue,
        })
    }
}

fn write_csv(columns: &[TabularColumn], rows: &[Vec<String>]) -> Result<String, ApplicationError> {
    let mut ordered_columns = columns.iter().collect::<Vec<_>>();
    ordered_columns.sort_by_key(|column| column.index);

    let mut writer = csv::WriterBuilder::new()
        .terminator(csv::Terminator::Any(b'\n'))
        .from_writer(Vec::new());
    writer.write_record(ordered_columns.iter().map(|column| column.name.as_str()))?;

    for row in rows {
        writer.write_record(row)?;
    }

    let bytes = writer.into_inner().map_err(|err| err.into_error())?;
    Ok(String::from_utf8(bytes)?)
}
