use crate::moat::MoatRoundReport;
use chrono::{DateTime, Utc};
use mdid_domain::{ContinueDecision, MoatTaskArtifact, MoatTaskNodeState};
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsString,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    thread,
    time::Duration,
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
const CLAIM_LOCK_RETRY_ATTEMPTS: usize = 200;
const CLAIM_LOCK_RETRY_SLEEP: Duration = Duration::from_millis(5);

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompleteTaskArtifact {
    pub artifact_ref: String,
    pub artifact_summary: String,
    pub recorded_at: DateTime<Utc>,
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

        let _claim_lock = ClaimReadyTaskLock::acquire(&self.path)?;
        let mut next_entries = load_entries(&self.path)?;
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

    pub fn complete_in_progress_task(
        &mut self,
        round_id: Option<&str>,
        node_id: &str,
    ) -> Result<String, CompleteInProgressTaskError> {
        self.complete_in_progress_task_with_artifact(round_id, node_id, None)
    }

    pub fn complete_in_progress_task_with_artifact(
        &mut self,
        round_id: Option<&str>,
        node_id: &str,
        artifact: Option<CompleteTaskArtifact>,
    ) -> Result<String, CompleteInProgressTaskError> {
        self.transition_task_state_with_artifact(
            round_id,
            node_id,
            MoatTaskNodeState::InProgress,
            MoatTaskNodeState::Completed,
            artifact,
        )
    }

    pub fn block_in_progress_task(
        &mut self,
        round_id: Option<&str>,
        node_id: &str,
    ) -> Result<String, CompleteInProgressTaskError> {
        self.transition_task_state(
            round_id,
            node_id,
            MoatTaskNodeState::InProgress,
            MoatTaskNodeState::Blocked,
        )
    }

    pub fn release_in_progress_task(
        &mut self,
        round_id: Option<&str>,
        node_id: &str,
    ) -> Result<String, CompleteInProgressTaskError> {
        self.transition_task_state(
            round_id,
            node_id,
            MoatTaskNodeState::InProgress,
            MoatTaskNodeState::Ready,
        )
    }

    pub fn unblock_blocked_task(
        &mut self,
        round_id: Option<&str>,
        node_id: &str,
    ) -> Result<String, CompleteInProgressTaskError> {
        self.transition_task_state(
            round_id,
            node_id,
            MoatTaskNodeState::Blocked,
            MoatTaskNodeState::Ready,
        )
    }

    fn transition_task_state(
        &mut self,
        round_id: Option<&str>,
        node_id: &str,
        expected_state: MoatTaskNodeState,
        next_state: MoatTaskNodeState,
    ) -> Result<String, CompleteInProgressTaskError> {
        self.transition_task_state_with_artifact(
            round_id,
            node_id,
            expected_state,
            next_state,
            None,
        )
    }

    fn transition_task_state_with_artifact(
        &mut self,
        round_id: Option<&str>,
        node_id: &str,
        expected_state: MoatTaskNodeState,
        next_state: MoatTaskNodeState,
        artifact: Option<CompleteTaskArtifact>,
    ) -> Result<String, CompleteInProgressTaskError> {
        if let Some(artifact) = artifact.as_ref() {
            if artifact.artifact_ref.trim().is_empty() {
                return Err(CompleteInProgressTaskError::InvalidArtifact {
                    field: "artifact_ref",
                });
            }
            if artifact.artifact_summary.trim().is_empty() {
                return Err(CompleteInProgressTaskError::InvalidArtifact {
                    field: "artifact_summary",
                });
            }
        }

        if !self.path.exists() {
            return Err(CompleteInProgressTaskError::Store(
                LocalMoatHistoryStoreError::MissingFile(self.path.clone()),
            ));
        }

        let _claim_lock = ClaimReadyTaskLock::acquire(&self.path)?;
        let mut next_entries = load_entries(&self.path)?;
        let entry = match round_id {
            Some(round_id) => next_entries
                .iter_mut()
                .find(|entry| entry.report.summary.round_id.to_string() == round_id)
                .ok_or_else(|| CompleteInProgressTaskError::RoundNotFound(round_id.to_string()))?,
            None => next_entries
                .last_mut()
                .ok_or(CompleteInProgressTaskError::NoHistoryEntries)?,
        };

        let selected_round_id = entry.report.summary.round_id.to_string();
        let node = entry
            .report
            .control_plane
            .task_graph
            .nodes
            .iter_mut()
            .find(|node| node.node_id == node_id)
            .ok_or_else(|| CompleteInProgressTaskError::NodeNotFound {
                round_id: selected_round_id.clone(),
                node_id: node_id.to_string(),
            })?;

        if node.state != expected_state {
            if expected_state == MoatTaskNodeState::InProgress {
                return Err(CompleteInProgressTaskError::NodeNotInProgress {
                    round_id: selected_round_id.clone(),
                    node_id: node_id.to_string(),
                    state: node.state,
                });
            }
            return Err(CompleteInProgressTaskError::NodeNotInExpectedState {
                round_id: selected_round_id.clone(),
                node_id: node_id.to_string(),
                state: node.state,
                expected_state,
            });
        }

        node.state = next_state;
        if let Some(artifact) = artifact {
            node.artifacts.push(MoatTaskArtifact {
                artifact_ref: artifact.artifact_ref,
                summary: artifact.artifact_summary,
                recorded_at: artifact.recorded_at,
            });
        }
        self.persist(&next_entries)?;
        self.entries = next_entries;
        Ok(selected_round_id)
    }

    fn persist(&self, entries: &[MoatHistoryEntry]) -> Result<(), LocalMoatHistoryStoreError> {
        let contents = serde_json::to_vec_pretty(entries)?;
        atomic_write(&self.path, &contents)?;
        Ok(())
    }
}

#[derive(Debug)]
struct ClaimReadyTaskLock {
    path: PathBuf,
}

impl ClaimReadyTaskLock {
    fn acquire(history_path: &Path) -> Result<Self, LocalMoatHistoryStoreError> {
        let path = claim_lock_path(history_path);
        for attempt in 0..CLAIM_LOCK_RETRY_ATTEMPTS {
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => {
                    drop(file);
                    return Ok(Self { path });
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    if attempt + 1 == CLAIM_LOCK_RETRY_ATTEMPTS {
                        return Err(io::Error::new(
                            io::ErrorKind::WouldBlock,
                            format!(
                                "timed out acquiring moat history claim lock: {}",
                                path.display()
                            ),
                        )
                        .into());
                    }
                    thread::sleep(CLAIM_LOCK_RETRY_SLEEP);
                }
                Err(error) => return Err(error.into()),
            }
        }

        Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            format!(
                "timed out acquiring moat history claim lock: {}",
                path.display()
            ),
        )
        .into())
    }
}

impl Drop for ClaimReadyTaskLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn claim_lock_path(history_path: &Path) -> PathBuf {
    let mut lock_file_name = history_path
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_else(|| OsString::from("moat-history.json"));
    lock_file_name.push(".lock");
    history_path.with_file_name(lock_file_name)
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
pub enum CompleteInProgressTaskError {
    #[error(transparent)]
    Store(#[from] LocalMoatHistoryStoreError),
    #[error("no moat history entries exist")]
    NoHistoryEntries,
    #[error("moat round not found: {0}")]
    RoundNotFound(String),
    #[error("moat task node not found in round {round_id}: {node_id}")]
    NodeNotFound { round_id: String, node_id: String },
    #[error("moat task node is not in progress in round {round_id}: {node_id} is {state:?}")]
    NodeNotInProgress {
        round_id: String,
        node_id: String,
        state: MoatTaskNodeState,
    },
    #[error("moat task node is not in expected state in round {round_id}: {node_id} is {state:?}, expected {expected_state:?}")]
    NodeNotInExpectedState {
        round_id: String,
        node_id: String,
        state: MoatTaskNodeState,
        expected_state: MoatTaskNodeState,
    },
    #[error("invalid complete task artifact: {field} must not be blank")]
    InvalidArtifact { field: &'static str },
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moat::{run_bounded_round, MoatRoundInput};
    use chrono::Utc;
    use mdid_domain::{
        CompetitorProfile, LockInReport, MarketMoatSnapshot, MoatStrategy, MoatType, ResourceBudget,
    };
    use tempfile::TempDir;

    #[test]
    fn complete_in_progress_task_with_artifact_persists_completed_state_and_artifact() {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let history_path = temp_dir.path().join("moat-history.json");
        let mut report = run_bounded_round(sample_round_input());
        report
            .control_plane
            .task_graph
            .nodes
            .iter_mut()
            .find(|node| node.node_id == "strategy_generation")
            .expect("strategy_generation node should exist")
            .state = MoatTaskNodeState::InProgress;
        fs::write(
            &history_path,
            serde_json::to_vec_pretty(&vec![MoatHistoryEntry {
                recorded_at: Utc::now(),
                report,
            }])
            .expect("failed to serialize history"),
        )
        .expect("failed to write history");

        let mut store =
            LocalMoatHistoryStore::open(history_path.clone()).expect("failed to open history");
        let artifact_recorded_at = Utc::now();
        store
            .complete_in_progress_task_with_artifact(
                None,
                "strategy_generation",
                Some(CompleteTaskArtifact {
                    artifact_ref: "docs/superpowers/plans/artifact.md".to_string(),
                    artifact_summary: "handoff summary".to_string(),
                    recorded_at: artifact_recorded_at,
                }),
            )
            .expect("failed to complete task with artifact");

        let entries = load_entries(&history_path).expect("failed to reload entries");
        let strategy_node = entries[0]
            .report
            .control_plane
            .task_graph
            .nodes
            .iter()
            .find(|node| node.node_id == "strategy_generation")
            .expect("strategy_generation node should exist");
        assert_eq!(strategy_node.state, MoatTaskNodeState::Completed);
        assert_eq!(strategy_node.artifacts.len(), 1);
        assert_eq!(
            strategy_node.artifacts[0].artifact_ref,
            "docs/superpowers/plans/artifact.md"
        );
        assert_eq!(strategy_node.artifacts[0].summary, "handoff summary");
        assert_eq!(strategy_node.artifacts[0].recorded_at, artifact_recorded_at);
    }

    #[test]
    fn complete_in_progress_task_with_no_artifact_persists_completed_state_without_artifact() {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let history_path = temp_dir.path().join("moat-history.json");
        let mut report = run_bounded_round(sample_round_input());
        report
            .control_plane
            .task_graph
            .nodes
            .iter_mut()
            .find(|node| node.node_id == "strategy_generation")
            .expect("strategy_generation node should exist")
            .state = MoatTaskNodeState::InProgress;
        fs::write(
            &history_path,
            serde_json::to_vec_pretty(&vec![MoatHistoryEntry {
                recorded_at: Utc::now(),
                report,
            }])
            .expect("failed to serialize history"),
        )
        .expect("failed to write history");

        let mut store =
            LocalMoatHistoryStore::open(history_path.clone()).expect("failed to open history");
        store
            .complete_in_progress_task_with_artifact(None, "strategy_generation", None)
            .expect("failed to complete task without artifact");

        let entries = load_entries(&history_path).expect("failed to reload entries");
        let strategy_node = entries[0]
            .report
            .control_plane
            .task_graph
            .nodes
            .iter()
            .find(|node| node.node_id == "strategy_generation")
            .expect("strategy_generation node should exist");
        assert_eq!(strategy_node.state, MoatTaskNodeState::Completed);
        assert!(strategy_node.artifacts.is_empty());
    }

    #[test]
    fn complete_in_progress_task_with_blank_artifact_ref_is_rejected() {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let history_path = temp_dir.path().join("moat-history.json");
        let mut report = run_bounded_round(sample_round_input());
        report
            .control_plane
            .task_graph
            .nodes
            .iter_mut()
            .find(|node| node.node_id == "strategy_generation")
            .expect("strategy_generation node should exist")
            .state = MoatTaskNodeState::InProgress;
        fs::write(
            &history_path,
            serde_json::to_vec_pretty(&vec![MoatHistoryEntry {
                recorded_at: Utc::now(),
                report,
            }])
            .expect("failed to serialize history"),
        )
        .expect("failed to write history");

        let mut store =
            LocalMoatHistoryStore::open(history_path.clone()).expect("failed to open history");
        let error = store
            .complete_in_progress_task_with_artifact(
                None,
                "strategy_generation",
                Some(CompleteTaskArtifact {
                    artifact_ref: " \t\n".to_string(),
                    artifact_summary: "handoff summary".to_string(),
                    recorded_at: Utc::now(),
                }),
            )
            .expect_err("blank artifact_ref should be rejected");

        assert!(matches!(
            error,
            CompleteInProgressTaskError::InvalidArtifact {
                field: "artifact_ref"
            }
        ));
        let entries = load_entries(&history_path).expect("failed to reload entries");
        let strategy_node = entries[0]
            .report
            .control_plane
            .task_graph
            .nodes
            .iter()
            .find(|node| node.node_id == "strategy_generation")
            .expect("strategy_generation node should exist");
        assert_eq!(strategy_node.state, MoatTaskNodeState::InProgress);
        assert!(strategy_node.artifacts.is_empty());
    }

    #[test]
    fn complete_in_progress_task_with_blank_artifact_summary_is_rejected() {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let history_path = temp_dir.path().join("moat-history.json");
        let mut report = run_bounded_round(sample_round_input());
        report
            .control_plane
            .task_graph
            .nodes
            .iter_mut()
            .find(|node| node.node_id == "strategy_generation")
            .expect("strategy_generation node should exist")
            .state = MoatTaskNodeState::InProgress;
        fs::write(
            &history_path,
            serde_json::to_vec_pretty(&vec![MoatHistoryEntry {
                recorded_at: Utc::now(),
                report,
            }])
            .expect("failed to serialize history"),
        )
        .expect("failed to write history");

        let mut store =
            LocalMoatHistoryStore::open(history_path.clone()).expect("failed to open history");
        let error = store
            .complete_in_progress_task_with_artifact(
                None,
                "strategy_generation",
                Some(CompleteTaskArtifact {
                    artifact_ref: "docs/superpowers/plans/artifact.md".to_string(),
                    artifact_summary: " \t\n".to_string(),
                    recorded_at: Utc::now(),
                }),
            )
            .expect_err("blank artifact summary should be rejected");

        assert!(matches!(
            error,
            CompleteInProgressTaskError::InvalidArtifact {
                field: "artifact_summary"
            }
        ));
        let entries = load_entries(&history_path).expect("failed to reload entries");
        let strategy_node = entries[0]
            .report
            .control_plane
            .task_graph
            .nodes
            .iter()
            .find(|node| node.node_id == "strategy_generation")
            .expect("strategy_generation node should exist");
        assert_eq!(strategy_node.state, MoatTaskNodeState::InProgress);
        assert!(strategy_node.artifacts.is_empty());
    }

    #[test]
    fn release_in_progress_task_returns_node_to_ready() {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let history_path = temp_dir.path().join("moat-history.json");
        let mut report = run_bounded_round(sample_round_input());
        let round_id = report.summary.round_id;
        report
            .control_plane
            .task_graph
            .nodes
            .iter_mut()
            .find(|node| node.node_id == "strategy_generation")
            .expect("strategy_generation node should exist")
            .state = MoatTaskNodeState::InProgress;
        fs::write(
            &history_path,
            serde_json::to_vec_pretty(&vec![MoatHistoryEntry {
                recorded_at: Utc::now(),
                report,
            }])
            .expect("failed to serialize history"),
        )
        .expect("failed to write history");

        let mut store =
            LocalMoatHistoryStore::open(history_path.clone()).expect("failed to open history");
        let selected_round_id = store
            .release_in_progress_task(None, "strategy_generation")
            .expect("failed to release task");

        assert_eq!(selected_round_id, round_id.to_string());
        let entries = load_entries(&history_path).expect("failed to reload entries");
        let strategy_node = entries[0]
            .report
            .control_plane
            .task_graph
            .nodes
            .iter()
            .find(|node| node.node_id == "strategy_generation")
            .expect("strategy_generation node should exist");
        assert_eq!(strategy_node.state, MoatTaskNodeState::Ready);
    }

    fn sample_round_input() -> MoatRoundInput {
        MoatRoundInput {
            market: MarketMoatSnapshot {
                market_id: "healthcare-deid".into(),
                moat_score: 45,
                ..MarketMoatSnapshot::default()
            },
            competitor: CompetitorProfile {
                competitor_id: "comp-1".into(),
                threat_score: 30,
                ..CompetitorProfile::default()
            },
            lock_in: LockInReport {
                lockin_score: 60,
                workflow_dependency_strength: 72,
                ..LockInReport::default()
            },
            strategies: vec![MoatStrategy {
                strategy_id: "workflow-audit".into(),
                title: "Workflow audit moat".into(),
                target_moat_type: MoatType::WorkflowLockIn,
                implementation_cost: 2,
                expected_moat_gain: 8,
                ..MoatStrategy::default()
            }],
            budget: ResourceBudget {
                max_round_minutes: 30,
                max_parallel_tasks: 3,
                max_strategy_candidates: 2,
                max_spec_generations: 1,
                max_implementation_tasks: 1,
                max_review_loops: 1,
            },
            improvement_threshold: 3,
            tests_passed: true,
        }
    }
}
