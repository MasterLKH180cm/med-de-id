use crate::moat::MoatRoundReport;
use chrono::{DateTime, Utc};
use mdid_domain::{ContinueDecision, MoatTaskNodeState};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use thiserror::Error;
use uuid::Uuid;

#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;

#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::{
    MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
};

const EVALUATION_TASK_ID: &str = "evaluation";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoatHistoryEntry {
    pub recorded_at: DateTime<Utc>,
    pub report: MoatRoundReport,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoatHistorySummary {
    pub entry_count: usize,
    pub latest_round_id: Option<String>,
    pub latest_continue_decision: Option<ContinueDecision>,
    pub latest_stop_reason: Option<String>,
    pub latest_decision_summary: Option<String>,
    pub latest_implemented_specs: Vec<String>,
    pub latest_moat_score_after: Option<i16>,
    pub best_moat_score_after: Option<i16>,
    pub improvement_deltas: Vec<i16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoatContinuationGate {
    pub latest_round_id: Option<String>,
    pub latest_continue_decision: Option<ContinueDecision>,
    pub latest_tests_passed: Option<bool>,
    pub latest_improvement_delta: Option<i16>,
    pub latest_stop_reason: Option<String>,
    pub evaluation_completed: bool,
    pub can_continue: bool,
    pub reason: String,
    pub required_improvement_threshold: i16,
}

#[derive(Debug)]
pub struct LocalMoatHistoryStore {
    path: PathBuf,
    entries: Vec<MoatHistoryEntry>,
}

impl LocalMoatHistoryStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, LocalMoatHistoryStoreError> {
        Self::open_with_mode(path, MissingHistoryBehavior::CreateEmptyFile)
    }

    pub fn open_existing(path: impl AsRef<Path>) -> Result<Self, LocalMoatHistoryStoreError> {
        Self::open_with_mode(path, MissingHistoryBehavior::Fail)
    }

    fn open_with_mode(
        path: impl AsRef<Path>,
        missing_history_behavior: MissingHistoryBehavior,
    ) -> Result<Self, LocalMoatHistoryStoreError> {
        let path = path.as_ref().to_path_buf();
        match missing_history_behavior {
            MissingHistoryBehavior::CreateEmptyFile => {
                if let Some(parent) = path.parent() {
                    if !parent.as_os_str().is_empty() {
                        fs::create_dir_all(parent)?;
                    }
                }
                if !path.exists() {
                    atomic_write(&path, b"[]")?;
                }
            }
            MissingHistoryBehavior::Fail => {
                if !path.exists() {
                    return Err(LocalMoatHistoryStoreError::MissingFile(path));
                }
            }
        }

        Ok(Self {
            entries: load_entries(&path)?,
            path,
        })
    }

    pub fn entries(&self) -> &[MoatHistoryEntry] {
        &self.entries
    }

    pub fn append(
        &mut self,
        recorded_at: DateTime<Utc>,
        report: MoatRoundReport,
    ) -> Result<(), LocalMoatHistoryStoreError> {
        let mut next_entries = self.entries.clone();
        next_entries.push(MoatHistoryEntry {
            recorded_at,
            report,
        });
        sort_entries(&mut next_entries);
        self.persist(&next_entries)?;
        self.entries = next_entries;
        Ok(())
    }

    pub fn summary(&self) -> MoatHistorySummary {
        let Some(latest) = self.entries.last() else {
            return MoatHistorySummary::default();
        };

        MoatHistorySummary {
            entry_count: self.entries.len(),
            latest_round_id: Some(latest.report.summary.round_id.to_string()),
            latest_continue_decision: Some(latest.report.summary.continue_decision),
            latest_stop_reason: latest.report.summary.stop_reason.clone(),
            latest_decision_summary: latest.report.control_plane.memory.latest_decision_summary(),
            latest_implemented_specs: latest.report.summary.implemented_specs.clone(),
            latest_moat_score_after: Some(latest.report.summary.moat_score_after),
            best_moat_score_after: self
                .entries
                .iter()
                .map(|entry| entry.report.summary.moat_score_after)
                .max(),
            improvement_deltas: self
                .entries
                .iter()
                .map(|entry| entry.report.summary.improvement())
                .collect(),
        }
    }

    pub fn continuation_gate(&self, required_improvement_threshold: i16) -> MoatContinuationGate {
        let Some(latest) = self.entries.last() else {
            return MoatContinuationGate {
                latest_round_id: None,
                latest_continue_decision: None,
                latest_tests_passed: None,
                latest_improvement_delta: None,
                latest_stop_reason: None,
                evaluation_completed: false,
                can_continue: false,
                reason: "no persisted moat rounds to evaluate".to_string(),
                required_improvement_threshold,
            };
        };

        let improvement_delta = latest.report.summary.improvement();
        let evaluation_completed = latest
            .report
            .executed_tasks
            .iter()
            .any(|task| task == EVALUATION_TASK_ID);

        let (can_continue, reason) = if !evaluation_completed {
            (false, "latest round did not complete evaluation")
        } else if !latest.report.summary.tests_passed {
            (false, "latest round tests failed")
        } else if improvement_delta < required_improvement_threshold {
            (false, "latest round improvement below threshold")
        } else if latest.report.summary.continue_decision == ContinueDecision::Continue {
            (true, "latest round cleared continuation gate")
        } else {
            (false, "latest round requested stop")
        };

        MoatContinuationGate {
            latest_round_id: Some(latest.report.summary.round_id.to_string()),
            latest_continue_decision: Some(latest.report.summary.continue_decision),
            latest_tests_passed: Some(latest.report.summary.tests_passed),
            latest_improvement_delta: Some(improvement_delta),
            latest_stop_reason: latest.report.summary.stop_reason.clone(),
            evaluation_completed,
            can_continue,
            reason: reason.to_string(),
            required_improvement_threshold,
        }
    }

    pub fn claim_ready_task(
        &mut self,
        round_id: Option<&str>,
        node_id: &str,
    ) -> Result<(), ClaimReadyTaskError> {
        if !self.path.exists() {
            return Err(ClaimReadyTaskError::Store(
                LocalMoatHistoryStoreError::MissingFile(self.path.clone()),
            ));
        }

        let mut next_entries = self.entries.clone();
        let entry = match round_id {
            Some(round_id) => next_entries
                .iter_mut()
                .find(|entry| entry.report.summary.round_id.to_string() == round_id)
                .ok_or_else(|| ClaimReadyTaskError::RoundNotFound(round_id.to_string()))?,
            None => next_entries
                .last_mut()
                .ok_or(ClaimReadyTaskError::NoHistoryEntries)?,
        };

        let node = entry
            .report
            .control_plane
            .task_graph
            .nodes
            .iter_mut()
            .find(|node| node.node_id == node_id)
            .ok_or_else(|| ClaimReadyTaskError::NodeNotFound {
                round_id: entry.report.summary.round_id.to_string(),
                node_id: node_id.to_string(),
            })?;

        if node.state != MoatTaskNodeState::Ready {
            return Err(ClaimReadyTaskError::NodeNotReady {
                round_id: entry.report.summary.round_id.to_string(),
                node_id: node_id.to_string(),
                state: node.state,
            });
        }

        node.state = MoatTaskNodeState::InProgress;
        self.persist(&next_entries)?;
        self.entries = next_entries;
        Ok(())
    }

    fn persist(&self, entries: &[MoatHistoryEntry]) -> Result<(), LocalMoatHistoryStoreError> {
        let contents = serde_json::to_vec_pretty(entries)?;
        atomic_write(&self.path, &contents)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MissingHistoryBehavior {
    CreateEmptyFile,
    Fail,
}

fn atomic_write(path: &Path, contents: &[u8]) -> Result<(), LocalMoatHistoryStoreError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("moat-history.json");
    let temp_path = path.with_file_name(format!(".{file_name}.{}.tmp", Uuid::new_v4().simple()));

    let write_result = (|| -> Result<(), std::io::Error> {
        let mut temp_file = fs::File::create(&temp_path)?;
        temp_file.write_all(contents)?;
        temp_file.sync_all()?;
        drop(temp_file);
        Ok(())
    })();

    if let Err(error) = write_result {
        let _ = fs::remove_file(&temp_path);
        return Err(error.into());
    }

    if let Err(error) = replace_atomic(&temp_path, path) {
        let _ = fs::remove_file(&temp_path);
        return Err(error);
    }

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            if let Ok(directory) = fs::File::open(parent) {
                let _ = directory.sync_all();
            }
        }
    }

    Ok(())
}

fn load_entries(path: &Path) -> Result<Vec<MoatHistoryEntry>, LocalMoatHistoryStoreError> {
    let contents = fs::read_to_string(path)?;
    if contents.trim().is_empty() {
        return Ok(Vec::new());
    }

    let mut entries = serde_json::from_str::<Vec<MoatHistoryEntry>>(&contents)?;
    sort_entries(&mut entries);
    Ok(entries)
}

fn sort_entries(entries: &mut [MoatHistoryEntry]) {
    entries.sort_by(|left, right| left.recorded_at.cmp(&right.recorded_at));
}

#[cfg(not(windows))]
fn replace_atomic(temp_path: &Path, path: &Path) -> Result<(), LocalMoatHistoryStoreError> {
    fs::rename(temp_path, path)?;
    Ok(())
}

#[cfg(windows)]
fn replace_atomic(temp_path: &Path, path: &Path) -> Result<(), LocalMoatHistoryStoreError> {
    let temp_path = encode_wide_path(temp_path);
    let path = encode_wide_path(path);
    let moved = unsafe {
        MoveFileExW(
            temp_path.as_ptr(),
            path.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };

    if moved == 0 {
        return Err(std::io::Error::last_os_error().into());
    }

    Ok(())
}

#[cfg(windows)]
fn encode_wide_path(path: &Path) -> Vec<u16> {
    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[derive(Debug, Error)]
pub enum ClaimReadyTaskError {
    #[error(transparent)]
    Store(#[from] LocalMoatHistoryStoreError),
    #[error("no moat history entries exist")]
    NoHistoryEntries,
    #[error("moat round not found: {0}")]
    RoundNotFound(String),
    #[error("moat task node not found in round {round_id}: {node_id}")]
    NodeNotFound { round_id: String, node_id: String },
    #[error("moat task node is not ready in round {round_id}: {node_id} is {state:?}")]
    NodeNotReady {
        round_id: String,
        node_id: String,
        state: MoatTaskNodeState,
    },
}

#[derive(Debug, Error)]
pub enum LocalMoatHistoryStoreError {
    #[error("failed to access moat history file")]
    Io(#[from] std::io::Error),
    #[error("failed to parse moat history file")]
    Json(#[from] serde_json::Error),
    #[error("moat history file does not exist: {0}")]
    MissingFile(PathBuf),
}
