use mdid_application::ApplicationService;
use mdid_domain::{PipelineRunState, SurfaceKind};

#[test]
fn application_service_creates_pipeline_and_run() {
    let service = ApplicationService::default();
    let pipeline = service.register_pipeline("foundation".into());
    let run = service.start_run(pipeline.id, SurfaceKind::Cli).unwrap();

    assert_eq!(pipeline.name, "foundation");
    assert_eq!(run.pipeline_id, pipeline.id);
    assert_eq!(run.state, PipelineRunState::Pending);
}

#[test]
fn application_service_rejects_unknown_pipeline() {
    let service = ApplicationService::default();
    let err = service
        .start_run(uuid::Uuid::new_v4(), SurfaceKind::Browser)
        .unwrap_err();
    assert!(err.to_string().contains("pipeline not found"));
}
