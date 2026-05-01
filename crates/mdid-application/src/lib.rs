use chrono::Utc;
use mdid_adapters::{
    sanitize_output_name, ConservativeMediaAdapter, ConservativeMediaAdapterError,
    ConservativeMediaInput, CsvTabularAdapter, DicomAdapter, DicomAdapterError, DicomRewritePlan,
    DicomTagReplacement, DicomUidReplacement, ExtractedTabularData, FieldPolicy, PdfAdapter,
    PdfAdapterError, PdfPageExtraction, TabularAdapterError, XlsxSheetDisclosure,
};
use mdid_domain::{
    BatchSummary, BurnedInAnnotationStatus, ConservativeMediaCandidate, ConservativeMediaSummary,
    DicomDeidentificationSummary, DicomPhiCandidate, DicomPrivateTagPolicy, MappingScope,
    PdfExtractionSummary, PdfPhiCandidate, PdfRewriteStatus, PhiCandidate, PipelineDefinition,
    PipelineRun, PipelineRunState, SurfaceKind, TabularColumn,
    DICOM_BURNED_IN_PIXEL_REDACTION_NOTICE,
};
use mdid_vault::{LocalVaultStore, NewMappingRecord, VaultError};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error("pipeline not found: {0}")]
    PipelineNotFound(Uuid),
    #[error(transparent)]
    DicomAdapter(#[from] DicomAdapterError),
    #[error(transparent)]
    PdfAdapter(#[from] PdfAdapterError),
    #[error(transparent)]
    ConservativeMediaAdapter(#[from] ConservativeMediaAdapterError),
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

#[derive(Clone)]
pub struct TabularDeidentificationOutput {
    pub csv: String,
    pub summary: BatchSummary,
    pub review_queue: Vec<PhiCandidate>,
    pub worksheet_disclosure: Option<XlsxSheetDisclosure>,
}

impl fmt::Debug for TabularDeidentificationOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TabularDeidentificationOutput")
            .field("csv", &"[REDACTED]")
            .field("summary", &self.summary)
            .field("review_queue_len", &self.review_queue.len())
            .finish()
    }
}

#[derive(Clone)]
pub struct DicomDeidentificationOutput {
    pub bytes: Vec<u8>,
    pub summary: DicomDeidentificationSummary,
    pub review_queue: Vec<DicomPhiCandidate>,
    pub sanitized_file_name: String,
}

impl fmt::Debug for DicomDeidentificationOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DicomDeidentificationOutput")
            .field("bytes", &"[REDACTED]")
            .field("summary", &self.summary)
            .field("review_queue_len", &self.review_queue.len())
            .field("sanitized_file_name", &self.sanitized_file_name)
            .finish()
    }
}

#[derive(Clone)]
pub struct PdfDeidentificationOutput {
    pub summary: PdfExtractionSummary,
    pub page_statuses: Vec<PdfPageExtraction>,
    pub review_queue: Vec<PdfPhiCandidate>,
    pub rewrite_status: PdfRewriteStatus,
    pub no_rewritten_pdf: bool,
    pub review_only: bool,
    pub rewritten_pdf_bytes: Option<Vec<u8>>,
}

impl fmt::Debug for PdfDeidentificationOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PdfDeidentificationOutput")
            .field("summary", &self.summary)
            .field("page_statuses", &self.page_statuses)
            .field("review_queue", &"[REDACTED]")
            .field("review_queue_len", &self.review_queue.len())
            .field("rewrite_status", &self.rewrite_status)
            .field("no_rewritten_pdf", &self.no_rewritten_pdf)
            .field("review_only", &self.review_only)
            .field(
                "rewritten_pdf_bytes",
                &self.rewritten_pdf_bytes.as_ref().map(|_| "[REDACTED]"),
            )
            .finish()
    }
}

#[derive(Clone)]
pub struct ConservativeMediaDeidentificationOutput {
    pub summary: ConservativeMediaSummary,
    pub review_queue: Vec<ConservativeMediaCandidate>,
    pub rewritten_media_bytes: Option<Vec<u8>>,
}

impl fmt::Debug for ConservativeMediaDeidentificationOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConservativeMediaDeidentificationOutput")
            .field("summary", &self.summary)
            .field("review_queue", &"[REDACTED]")
            .field("review_queue_len", &self.review_queue.len())
            .field(
                "rewritten_media_bytes",
                &self.rewritten_media_bytes.as_ref().map(|_| "[REDACTED]"),
            )
            .finish()
    }
}

#[derive(Clone, Default)]
pub struct TabularDeidentificationService;

#[derive(Clone, Default)]
pub struct DicomDeidentificationService;

#[derive(Clone, Default)]
pub struct PdfDeidentificationService;

#[derive(Clone, Default)]
pub struct ConservativeMediaDeidentificationService {
    _private: (),
}

impl ConservativeMediaDeidentificationService {
    pub fn deidentify_metadata(
        &self,
        input: ConservativeMediaInput,
    ) -> Result<ConservativeMediaDeidentificationOutput, ApplicationError> {
        let extracted = ConservativeMediaAdapter::extract_metadata(input)?;

        Ok(ConservativeMediaDeidentificationOutput {
            summary: extracted.summary,
            review_queue: extracted.candidates,
            rewritten_media_bytes: None,
        })
    }
}

impl PdfDeidentificationService {
    pub fn deidentify_bytes(
        &self,
        bytes: &[u8],
        source_name: &str,
    ) -> Result<PdfDeidentificationOutput, ApplicationError> {
        let extracted = PdfAdapter::new().extract(bytes, source_name)?;

        Ok(PdfDeidentificationOutput {
            summary: extracted.summary,
            page_statuses: extracted.pages,
            review_queue: extracted.candidates,
            rewrite_status: PdfRewriteStatus::ReviewOnlyNoRewrittenPdf,
            no_rewritten_pdf: true,
            review_only: true,
            rewritten_pdf_bytes: None,
        })
    }
}

impl DicomDeidentificationService {
    pub fn deidentify_bytes(
        &self,
        bytes: &[u8],
        source_name: &str,
        private_tag_policy: DicomPrivateTagPolicy,
        vault: &mut LocalVaultStore,
        actor: SurfaceKind,
    ) -> Result<DicomDeidentificationOutput, ApplicationError> {
        let adapter = DicomAdapter::new(private_tag_policy);
        let extracted = adapter.extract(bytes, source_name)?;
        let job_id = Uuid::new_v4();
        let artifact_id = Uuid::new_v4();
        let burned_in_review_required =
            extracted.burned_in_annotation == BurnedInAnnotationStatus::Suspicious;
        let mut summary = DicomDeidentificationSummary {
            total_tags: extracted.candidates.len(),
            removed_private_tags: if private_tag_policy == DicomPrivateTagPolicy::Remove {
                extracted.private_tags.len()
            } else {
                0
            },
            burned_in_suspicions: usize::from(burned_in_review_required),
            burned_in_review_required,
            burned_in_annotation_notice: burned_in_annotation_notice(burned_in_review_required)
                .into(),
            burned_in_disclosure: DICOM_BURNED_IN_PIXEL_REDACTION_NOTICE.into(),
            ..DicomDeidentificationSummary::default()
        };
        let mut review_queue = Vec::new();
        let mut tag_replacements = Vec::new();

        for candidate in extracted.candidates {
            if candidate.decision.requires_human_review() {
                summary.review_required_tags += 1;
                review_queue.push(candidate);
                continue;
            }

            if !candidate.decision.allows_encode() {
                continue;
            }

            let mapping = vault.ensure_mapping(
                NewMappingRecord {
                    scope: MappingScope::new(job_id, artifact_id, candidate.tag.field_path()),
                    phi_type: DICOM_COMMON_PHI_MAPPING_TYPE.into(),
                    original_value: candidate.value.clone(),
                },
                actor,
            )?;

            tag_replacements.push(DicomTagReplacement::new(candidate.tag, mapping.token));
            summary.encoded_tags += 1;
        }

        let mut uid_replacements = Vec::new();
        for uid in adapter.extract_uid_family(bytes)? {
            let mapping = vault.ensure_mapping(
                NewMappingRecord {
                    scope: MappingScope::new(job_id, artifact_id, uid.field_path().to_string()),
                    phi_type: DICOM_UID_MAPPING_TYPE.into(),
                    original_value: uid.value.clone(),
                },
                actor,
            )?;

            uid_replacements.push(DicomUidReplacement::new(uid.tag, uid.value, mapping.token));
        }
        summary.remapped_uids = uid_replacements.len();

        Ok(DicomDeidentificationOutput {
            bytes: adapter.rewrite(
                bytes,
                &DicomRewritePlan {
                    tag_replacements,
                    uid_replacements,
                },
            )?,
            summary,
            review_queue,
            sanitized_file_name: sanitize_output_name(source_name),
        })
    }
}

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
            if candidate.cell.row_index >= summary.total_rows {
                continue;
            }

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
            worksheet_disclosure: extracted.xlsx_disclosure,
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

fn burned_in_annotation_notice(_review_required: bool) -> &'static str {
    DICOM_BURNED_IN_PIXEL_REDACTION_NOTICE
}

const DICOM_COMMON_PHI_MAPPING_TYPE: &str = "dicom_common_phi";
const DICOM_UID_MAPPING_TYPE: &str = "dicom_uid";
