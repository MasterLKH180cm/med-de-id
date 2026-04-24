use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use mdid_application::ApplicationService;
use serde::{Deserialize, Serialize};

#[derive(Clone, Default)]
pub struct RuntimeState {
    pub application: ApplicationService,
}

#[derive(Debug, Deserialize)]
struct CreatePipelineRequest {
    name: String,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
}

pub fn build_router() -> Router {
    let state = RuntimeState::default();

    Router::new()
        .route("/health", get(health))
        .route("/pipelines", post(create_pipeline))
        .with_state(state)
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, Json(HealthResponse { status: "ok" }))
}

async fn create_pipeline(
    State(state): State<RuntimeState>,
    Json(payload): Json<CreatePipelineRequest>,
) -> impl IntoResponse {
    let pipeline = state.application.register_pipeline(payload.name);
    (StatusCode::CREATED, Json(pipeline))
}
