use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use mdid_runtime::http::build_router;
use tower::ServiceExt;

#[tokio::test]
async fn health_endpoint_returns_ok() {
    let app = build_router();
    let response = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn pipelines_endpoint_registers_pipeline() {
    let app = build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/pipelines")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"foundation"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}
