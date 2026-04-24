# med-de-id Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Establish a testable Rust workspace skeleton for `med-de-id` with shared domain/application/runtime foundations plus the first CLI, browser, and desktop entry points.

**Architecture:** This plan intentionally covers only the first implementation slice from the approved spec because the full product spans multiple subsystems. The slice creates the shared workspace, core crates, runtime API shell, and tri-surface entry points so later slices can add vaults, adapters, and format-specific workflows without restructuring the repo.

**Tech Stack:** Rust workspace, Cargo, Axum, Tokio, Serde, thiserror, uuid, chrono, Leptos (browser shell), egui/eframe (desktop shell), GitHub Actions.

---

## Scope note

The approved spec is too broad for one safe implementation plan. This plan covers **Slice 1 — platform skeleton** only. Remaining slices should be implemented as follow-on plans/issues:

- Slice 2 — vault + encode/decode MVP
- Slice 3 — CSV/Excel deep support
- Slice 4 — DICOM tag-level support
- Slice 5 — PDF/OCR support
- Slice 6 — image/video/FCS conservative support

## File structure

**Create:**
- `Cargo.toml`
- `rust-toolchain.toml`
- `crates/mdid-domain/Cargo.toml`
- `crates/mdid-domain/src/lib.rs`
- `crates/mdid-domain/tests/workflow_models.rs`
- `crates/mdid-application/Cargo.toml`
- `crates/mdid-application/src/lib.rs`
- `crates/mdid-application/tests/application_services.rs`
- `crates/mdid-runtime/Cargo.toml`
- `crates/mdid-runtime/src/lib.rs`
- `crates/mdid-runtime/src/http.rs`
- `crates/mdid-runtime/tests/runtime_http.rs`
- `crates/mdid-cli/Cargo.toml`
- `crates/mdid-cli/src/main.rs`
- `crates/mdid-cli/tests/cli_smoke.rs`
- `crates/mdid-browser/Cargo.toml`
- `crates/mdid-browser/src/lib.rs`
- `crates/mdid-browser/src/app.rs`
- `crates/mdid-desktop/Cargo.toml`
- `crates/mdid-desktop/src/main.rs`
- `.github/workflows/ci.yml`

**Modify:**
- `README.md`
- `.gitignore`

---

### Task 1: Bootstrap the Cargo workspace and repo-wide tooling

**Files:**
- Create: `Cargo.toml`
- Create: `rust-toolchain.toml`
- Modify: `README.md`
- Modify: `.gitignore`

- [ ] **Step 1: Write the failing bootstrap check**

Create a minimal workspace manifest with all planned members but without any member files yet:

```toml
[workspace]
members = [
  "crates/mdid-domain",
  "crates/mdid-application",
  "crates/mdid-runtime",
  "crates/mdid-cli",
  "crates/mdid-browser",
  "crates/mdid-desktop",
]
resolver = "2"
```

- [ ] **Step 2: Run the workspace metadata command to verify it fails**

Run:

```bash
cargo metadata --format-version 1
```

Expected: FAIL because the member crates do not exist yet.

- [ ] **Step 3: Add the real workspace files**

Write `Cargo.toml`:

```toml
[workspace]
members = [
  "crates/mdid-domain",
  "crates/mdid-application",
  "crates/mdid-runtime",
  "crates/mdid-cli",
  "crates/mdid-browser",
  "crates/mdid-desktop",
]
resolver = "2"

[workspace.package]
edition = "2021"
license = "UNLICENSED"
version = "0.1.0"

[workspace.dependencies]
anyhow = "1"
axum = { version = "0.7", features = ["macros"] }
chrono = { version = "0.4", features = ["serde"] }
eframe = "0.27"
egui = "0.27"
http = "1"
leptos = { version = "0.6", features = ["csr"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "sync"] }
tower = "0.5"
tower-http = { version = "0.5", features = ["cors", "trace"] }
uuid = { version = "1", features = ["serde", "v4"] }
```

Write `rust-toolchain.toml`:

```toml
[toolchain]
channel = "stable"
components = ["clippy", "rustfmt"]
targets = ["wasm32-unknown-unknown"]
```

- [ ] **Step 4: Run the workspace metadata command again**

Run:

```bash
cargo metadata --format-version 1
```

Expected: still FAIL until the crate manifests are added in later tasks, which is acceptable at this stage.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml rust-toolchain.toml README.md .gitignore
git commit -m "chore: bootstrap med-de-id workspace metadata"
```

### Task 2: Create the shared domain crate and lock the workflow vocabulary

**Files:**
- Create: `crates/mdid-domain/Cargo.toml`
- Create: `crates/mdid-domain/src/lib.rs`
- Create: `crates/mdid-domain/tests/workflow_models.rs`

- [ ] **Step 1: Write the failing domain tests**

Create `crates/mdid-domain/tests/workflow_models.rs`:

```rust
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
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```bash
cargo test -p mdid-domain --test workflow_models
```

Expected: FAIL because the crate and exported types do not exist.

- [ ] **Step 3: Write the minimal domain implementation**

Create `crates/mdid-domain/Cargo.toml`:

```toml
[package]
name = "mdid-domain"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
chrono.workspace = true
serde.workspace = true
uuid.workspace = true
```

Create `crates/mdid-domain/src/lib.rs`:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurfaceKind {
    Cli,
    Browser,
    Desktop,
}

impl SurfaceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            SurfaceKind::Cli => "cli",
            SurfaceKind::Browser => "browser",
            SurfaceKind::Desktop => "desktop",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PipelineRunState {
    Pending,
    Scheduled,
    Running,
    WaitingForReview,
    WaitingForApproval,
    Retrying,
    Completed,
    PartiallyFailed,
    Failed,
    Cancelled,
}

impl PipelineRunState {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            PipelineRunState::Completed
                | PipelineRunState::PartiallyFailed
                | PipelineRunState::Failed
                | PipelineRunState::Cancelled
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewTaskState {
    Open,
    Claimed,
    Resolved,
    Rejected,
    Expired,
}

impl ReviewTaskState {
    pub fn is_open(&self) -> bool {
        matches!(self, ReviewTaskState::Open | ReviewTaskState::Claimed)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineDefinition {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineRun {
    pub id: Uuid,
    pub pipeline_id: Uuid,
    pub state: PipelineRunState,
    pub started_by: SurfaceKind,
    pub created_at: DateTime<Utc>,
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run:

```bash
cargo test -p mdid-domain --test workflow_models
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-domain/Cargo.toml crates/mdid-domain/src/lib.rs crates/mdid-domain/tests/workflow_models.rs
git commit -m "feat: add shared domain workflow models"
```

### Task 3: Add application services for pipeline registration and run creation

**Files:**
- Create: `crates/mdid-application/Cargo.toml`
- Create: `crates/mdid-application/src/lib.rs`
- Create: `crates/mdid-application/tests/application_services.rs`

- [ ] **Step 1: Write the failing application tests**

Create `crates/mdid-application/tests/application_services.rs`:

```rust
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
    let err = service.start_run(uuid::Uuid::new_v4(), SurfaceKind::Browser).unwrap_err();
    assert!(err.to_string().contains("pipeline not found"));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```bash
cargo test -p mdid-application --test application_services
```

Expected: FAIL because the crate and service do not exist.

- [ ] **Step 3: Write the minimal application service**

Create `crates/mdid-application/Cargo.toml`:

```toml
[package]
name = "mdid-application"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
chrono.workspace = true
mdid-domain = { path = "../mdid-domain" }
thiserror.workspace = true
uuid.workspace = true
```

Create `crates/mdid-application/src/lib.rs`:

```rust
use chrono::Utc;
use mdid_domain::{PipelineDefinition, PipelineRun, PipelineRunState, SurfaceKind};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error("pipeline not found: {0}")]
    PipelineNotFound(Uuid),
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
```

- [ ] **Step 4: Run the tests to verify they pass**

Run:

```bash
cargo test -p mdid-application --test application_services
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-application/Cargo.toml crates/mdid-application/src/lib.rs crates/mdid-application/tests/application_services.rs
git commit -m "feat: add application service for pipeline registration"
```

### Task 4: Add the runtime crate with a localhost health and pipeline registration API

**Files:**
- Create: `crates/mdid-runtime/Cargo.toml`
- Create: `crates/mdid-runtime/src/lib.rs`
- Create: `crates/mdid-runtime/src/http.rs`
- Create: `crates/mdid-runtime/tests/runtime_http.rs`

- [ ] **Step 1: Write the failing runtime HTTP tests**

Create `crates/mdid-runtime/tests/runtime_http.rs`:

```rust
use axum::{body::Body, http::{Request, StatusCode}};
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
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```bash
cargo test -p mdid-runtime --test runtime_http
```

Expected: FAIL because the runtime crate does not exist.

- [ ] **Step 3: Write the minimal runtime implementation**

Create `crates/mdid-runtime/Cargo.toml`:

```toml
[package]
name = "mdid-runtime"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
axum.workspace = true
mdid-application = { path = "../mdid-application" }
mdid-domain = { path = "../mdid-domain" }
serde.workspace = true
serde_json.workspace = true
tokio.workspace = true
tower-http.workspace = true
```

Create `crates/mdid-runtime/src/lib.rs`:

```rust
pub mod http;
```

Create `crates/mdid-runtime/src/http.rs`:

```rust
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
```

- [ ] **Step 4: Run the tests to verify they pass**

Run:

```bash
cargo test -p mdid-runtime --test runtime_http
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-runtime/Cargo.toml crates/mdid-runtime/src/lib.rs crates/mdid-runtime/src/http.rs crates/mdid-runtime/tests/runtime_http.rs
git commit -m "feat: add localhost runtime HTTP skeleton"
```

### Task 5: Add the first CLI, browser, and desktop surface entry points

**Files:**
- Create: `crates/mdid-cli/Cargo.toml`
- Create: `crates/mdid-cli/src/main.rs`
- Create: `crates/mdid-cli/tests/cli_smoke.rs`
- Create: `crates/mdid-browser/Cargo.toml`
- Create: `crates/mdid-browser/src/lib.rs`
- Create: `crates/mdid-browser/src/app.rs`
- Create: `crates/mdid-desktop/Cargo.toml`
- Create: `crates/mdid-desktop/src/main.rs`
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Write the failing CLI smoke test**

Create `crates/mdid-cli/tests/cli_smoke.rs`:

```rust
use std::process::Command;

#[test]
fn cli_prints_status_banner() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .arg("status")
        .output()
        .expect("failed to run mdid-cli");

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("med-de-id CLI ready"));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```bash
cargo test -p mdid-cli --test cli_smoke
```

Expected: FAIL because the binary crate does not exist.

- [ ] **Step 3: Write the minimal surface implementations and CI**

Create `crates/mdid-cli/Cargo.toml`:

```toml
[package]
name = "mdid-cli"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
anyhow.workspace = true
mdid-application = { path = "../mdid-application" }
mdid-domain = { path = "../mdid-domain" }

[[bin]]
name = "mdid-cli"
path = "src/main.rs"
```

Create `crates/mdid-cli/src/main.rs`:

```rust
fn main() {
    let command = std::env::args().nth(1).unwrap_or_else(|| "status".into());
    match command.as_str() {
        "status" => println!("med-de-id CLI ready"),
        other => {
            eprintln!("unknown command: {other}");
            std::process::exit(1);
        }
    }
}
```

Create `crates/mdid-browser/Cargo.toml`:

```toml
[package]
name = "mdid-browser"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
leptos.workspace = true
mdid-domain = { path = "../mdid-domain" }
```

Create `crates/mdid-browser/src/lib.rs`:

```rust
pub mod app;
```

Create `crates/mdid-browser/src/app.rs`:

```rust
use leptos::*;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <main>
            <h1>"med-de-id browser tool"</h1>
            <p>"Pipeline/orchestration surface skeleton"</p>
        </main>
    }
}
```

Create `crates/mdid-desktop/Cargo.toml`:

```toml
[package]
name = "mdid-desktop"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
eframe.workspace = true
egui.workspace = true
```

Create `crates/mdid-desktop/src/main.rs`:

```rust
fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "med-de-id desktop",
        options,
        Box::new(|_cc| Box::<DesktopApp>::default()),
    )
}

#[derive(Default)]
struct DesktopApp;

impl eframe::App for DesktopApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("med-de-id desktop");
            ui.label("Sensitive workstation surface skeleton");
        });
    }
}
```

Create `.github/workflows/ci.yml`:

```yaml
name: ci

on:
  push:
    branches: [main]
  pull_request:

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown
      - uses: Swatinem/rust-cache@v2
      - name: Format check
        run: cargo fmt --all -- --check
      - name: Clippy
        run: cargo clippy --workspace --all-targets -- -D warnings
      - name: Tests
        run: cargo test --workspace
```

- [ ] **Step 4: Run the full workspace verification**

Run:

```bash
cargo fmt --all
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-cli/Cargo.toml crates/mdid-cli/src/main.rs crates/mdid-cli/tests/cli_smoke.rs \
  crates/mdid-browser/Cargo.toml crates/mdid-browser/src/lib.rs crates/mdid-browser/src/app.rs \
  crates/mdid-desktop/Cargo.toml crates/mdid-desktop/src/main.rs .github/workflows/ci.yml
git commit -m "feat: add tri-surface skeleton and CI"
```

## Self-review checklist

Before executing this plan, verify:

- every file path above exists in exactly one task
- the crate names stay consistent (`mdid-domain`, `mdid-application`, `mdid-runtime`, `mdid-cli`, `mdid-browser`, `mdid-desktop`)
- CLI uses the `mdid-cli` binary name consistently
- the runtime exposes only localhost-safe foundation endpoints in this slice
- no task assumes vault/adapters/format logic already exists

## Spec coverage for this plan

This plan intentionally covers these approved-spec areas only:

- workspace and crate boundaries
- shared domain vocabulary
- application-service seam
- localhost runtime API skeleton
- first CLI/browser/desktop entry points
- CI enforcement for the initial foundation slice

The remaining approved spec areas are tracked as separate GitHub issues/milestones and should receive dedicated follow-on implementation plans.
