use mdid_domain::{PipelineRunState, ReviewTaskState, SurfaceKind};

#[test]
fn pipeline_run_state_reports_terminal_variants() {
    assert!(PipelineRunState::Completed.is_terminal());
    assert!(PipelineRunState::Failed.is_terminal());
    assert!(PipelineRunState::Cancelled.is_terminal());
    assert!(!PipelineRunState::Running.is_terminal());
}

#[test]
fn review_task_state_reports_open_and_terminal_variants() {
    assert!(ReviewTaskState::Open.is_open());
    assert!(!ReviewTaskState::Resolved.is_open());
}

#[test]
fn surface_kind_display_names_are_stable() {
    assert_eq!(SurfaceKind::Cli.as_str(), "cli");
    assert_eq!(SurfaceKind::Browser.as_str(), "browser");
    assert_eq!(SurfaceKind::Desktop.as_str(), "desktop");
}
