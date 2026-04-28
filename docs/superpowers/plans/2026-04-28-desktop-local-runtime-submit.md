# Desktop Local Runtime Submit Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a narrow desktop workstation action that submits the already-built request envelope to a local `mdid-runtime` HTTP server and renders the runtime response.

**Architecture:** Keep the desktop surface bounded and local-first: a small HTTP helper in `mdid-desktop` posts JSON to `127.0.0.1:<port>` for the existing selected route, then reuses `DesktopWorkflowResponseState` parsing/rendering. This is not a controller/orchestration feature and does not add agent workflow semantics.

**Tech Stack:** Rust workspace, `eframe`/`egui`, `serde_json`, standard-library `TcpStream` HTTP/1.1, Cargo tests.

---

## File Structure

- Modify: `crates/mdid-desktop/src/lib.rs`
  - Add a focused `DesktopRuntimeClient` and `DesktopRuntimeSubmitError` using `std::net::TcpStream`.
  - Add unit tests for URL/request construction, successful JSON body extraction, non-2xx errors, and invalid host/port guardrails.
- Modify: `crates/mdid-desktop/src/main.rs`
  - Add runtime host/port state and a "Submit to local runtime" button that calls the helper and applies success/error rendering.
- Modify: `README.md`
  - Truth-sync desktop/browser/overall completion snapshot after landed behavior and verification.

### Task 1: Desktop runtime HTTP submit helper

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`
- Test: `crates/mdid-desktop/src/lib.rs`

- [ ] **Step 1: Write failing tests**

Add tests inside the existing `#[cfg(test)] mod tests` in `crates/mdid-desktop/src/lib.rs`:

```rust
#[test]
fn desktop_runtime_client_builds_local_post_request() {
    let state = DesktopWorkflowRequestState {
        mode: DesktopWorkflowMode::CsvText,
        payload: "patient_name\nJane".to_string(),
        field_policy_json: r#"[{"header":"patient_name","phi_type":"Name","action":"encode"}]"#.to_string(),
        source_name: "unused.pdf".to_string(),
    };
    let request = state.try_build_request().expect("valid request");

    let client = DesktopRuntimeClient::new("127.0.0.1", 8787).expect("valid local client");
    let http = client.build_http_request(&request).expect("request bytes");

    assert!(http.starts_with("POST /tabular/deidentify HTTP/1.1\r\n"));
    assert!(http.contains("Host: 127.0.0.1:8787\r\n"));
    assert!(http.contains("Content-Type: application/json\r\n"));
    assert!(http.contains("Connection: close\r\n"));
    assert!(http.ends_with("Jane"));
}

#[test]
fn desktop_runtime_client_rejects_non_local_hosts() {
    let error = DesktopRuntimeClient::new("example.com", 8787).expect_err("remote host rejected");
    assert_eq!(
        error,
        DesktopRuntimeSubmitError::InvalidEndpoint("desktop runtime client only supports localhost/127.0.0.1".to_string())
    );
}

#[test]
fn desktop_runtime_client_extracts_success_json_body() {
    let response = "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 15\r\n\r\n{\"csv\":\"ok\"}";

    let body = DesktopRuntimeClient::extract_json_body(response).expect("success body");

    assert_eq!(body, serde_json::json!({"csv":"ok"}));
}

#[test]
fn desktop_runtime_client_reports_runtime_error_body() {
    let response = "HTTP/1.1 422 Unprocessable Entity\r\ncontent-type: application/json\r\n\r\n{\"error\":\"bad csv\"}";

    let error = DesktopRuntimeClient::extract_json_body(response).expect_err("runtime error");

    assert_eq!(
        error,
        DesktopRuntimeSubmitError::RuntimeHttpStatus {
            status: 422,
            body: "{\"error\":\"bad csv\"}".to_string(),
        }
    );
}
```

- [ ] **Step 2: Run tests to verify RED**

Run: `cargo test -p mdid-desktop desktop_runtime_client_ -- --nocapture`

Expected: FAIL because `DesktopRuntimeClient` and `DesktopRuntimeSubmitError` are not defined.

- [ ] **Step 3: Implement minimal helper**

In `crates/mdid-desktop/src/lib.rs`, add public helper types after `DesktopWorkflowValidationError`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopRuntimeSubmitError {
    InvalidEndpoint(String),
    Io(String),
    InvalidHttpResponse(String),
    RuntimeHttpStatus { status: u16, body: String },
    InvalidJson(String),
}

pub struct DesktopRuntimeClient {
    host: String,
    port: u16,
}

impl DesktopRuntimeClient {
    pub fn new(host: impl Into<String>, port: u16) -> Result<Self, DesktopRuntimeSubmitError> {
        let host = host.into();
        if !matches!(host.as_str(), "127.0.0.1" | "localhost") {
            return Err(DesktopRuntimeSubmitError::InvalidEndpoint(
                "desktop runtime client only supports localhost/127.0.0.1".to_string(),
            ));
        }
        if port == 0 {
            return Err(DesktopRuntimeSubmitError::InvalidEndpoint(
                "desktop runtime port must be greater than zero".to_string(),
            ));
        }
        Ok(Self { host, port })
    }

    pub fn submit(
        &self,
        request: &DesktopWorkflowRequest,
    ) -> Result<serde_json::Value, DesktopRuntimeSubmitError> {
        use std::io::{Read, Write};
        use std::net::TcpStream;
        use std::time::Duration;

        let mut stream = TcpStream::connect((self.host.as_str(), self.port))
            .map_err(|error| DesktopRuntimeSubmitError::Io(error.to_string()))?;
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .map_err(|error| DesktopRuntimeSubmitError::Io(error.to_string()))?;
        stream
            .set_write_timeout(Some(Duration::from_secs(10)))
            .map_err(|error| DesktopRuntimeSubmitError::Io(error.to_string()))?;
        let http = self.build_http_request(request)?;
        stream
            .write_all(http.as_bytes())
            .map_err(|error| DesktopRuntimeSubmitError::Io(error.to_string()))?;
        let mut response = String::new();
        stream
            .read_to_string(&mut response)
            .map_err(|error| DesktopRuntimeSubmitError::Io(error.to_string()))?;
        Self::extract_json_body(&response)
    }

    pub fn build_http_request(
        &self,
        request: &DesktopWorkflowRequest,
    ) -> Result<String, DesktopRuntimeSubmitError> {
        let body = serde_json::to_string(&request.body)
            .map_err(|error| DesktopRuntimeSubmitError::InvalidJson(error.to_string()))?;
        Ok(format!(
            "POST {route} HTTP/1.1\r\nHost: {host}:{port}\r\nContent-Type: application/json\r\nAccept: application/json\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n{body}",
            route = request.route,
            host = self.host,
            port = self.port,
            length = body.as_bytes().len(),
        ))
    }

    pub fn extract_json_body(response: &str) -> Result<serde_json::Value, DesktopRuntimeSubmitError> {
        let (head, body) = response.split_once("\r\n\r\n").ok_or_else(|| {
            DesktopRuntimeSubmitError::InvalidHttpResponse(
                "runtime response did not contain an HTTP header/body separator".to_string(),
            )
        })?;
        let status_line = head.lines().next().ok_or_else(|| {
            DesktopRuntimeSubmitError::InvalidHttpResponse(
                "runtime response was missing a status line".to_string(),
            )
        })?;
        let status = status_line
            .split_whitespace()
            .nth(1)
            .and_then(|value| value.parse::<u16>().ok())
            .ok_or_else(|| {
                DesktopRuntimeSubmitError::InvalidHttpResponse(format!(
                    "runtime response had an invalid status line: {status_line}"
                ))
            })?;
        if !(200..300).contains(&status) {
            return Err(DesktopRuntimeSubmitError::RuntimeHttpStatus {
                status,
                body: body.to_string(),
            });
        }
        serde_json::from_str(body)
            .map_err(|error| DesktopRuntimeSubmitError::InvalidJson(error.to_string()))
    }
}
```

- [ ] **Step 4: Run targeted and package tests**

Run: `cargo test -p mdid-desktop desktop_runtime_client_ -- --nocapture`

Expected: PASS.

Run: `cargo test -p mdid-desktop`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-desktop/src/lib.rs
git commit -m "feat(desktop): add local runtime submit client"
```

### Task 2: Desktop UI submit action and README truth-sync

**Files:**
- Modify: `crates/mdid-desktop/src/main.rs`
- Modify: `crates/mdid-desktop/src/lib.rs`
- Modify: `README.md`

- [ ] **Step 1: Write failing UI-facing tests**

Add tests in `crates/mdid-desktop/src/lib.rs`:

```rust
#[test]
fn desktop_runtime_settings_default_to_localhost() {
    let settings = DesktopRuntimeSettings::default();

    assert_eq!(settings.host, "127.0.0.1");
    assert_eq!(settings.port_text, "8787");
    assert_eq!(settings.parse_port(), Ok(8787));
}

#[test]
fn desktop_runtime_settings_reject_blank_or_invalid_ports() {
    let mut settings = DesktopRuntimeSettings::default();

    settings.port_text = "".to_string();
    assert_eq!(
        settings.parse_port(),
        Err(DesktopRuntimeSubmitError::InvalidEndpoint(
            "desktop runtime port must be a number between 1 and 65535".to_string()
        ))
    );

    settings.port_text = "99999".to_string();
    assert_eq!(
        settings.parse_port(),
        Err(DesktopRuntimeSubmitError::InvalidEndpoint(
            "desktop runtime port must be a number between 1 and 65535".to_string()
        ))
    );
}
```

- [ ] **Step 2: Run tests to verify RED**

Run: `cargo test -p mdid-desktop desktop_runtime_settings_ -- --nocapture`

Expected: FAIL because `DesktopRuntimeSettings` is not defined.

- [ ] **Step 3: Implement settings and wire UI**

In `crates/mdid-desktop/src/lib.rs`, add:

```rust
#[derive(Clone, PartialEq, Eq)]
pub struct DesktopRuntimeSettings {
    pub host: String,
    pub port_text: String,
}

impl Default for DesktopRuntimeSettings {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port_text: "8787".to_string(),
        }
    }
}

impl DesktopRuntimeSettings {
    pub fn parse_port(&self) -> Result<u16, DesktopRuntimeSubmitError> {
        self.port_text.trim().parse::<u16>().map_err(|_| {
            DesktopRuntimeSubmitError::InvalidEndpoint(
                "desktop runtime port must be a number between 1 and 65535".to_string(),
            )
        })
    }

    pub fn client(&self) -> Result<DesktopRuntimeClient, DesktopRuntimeSubmitError> {
        DesktopRuntimeClient::new(self.host.trim(), self.parse_port()?)
    }
}
```

In `crates/mdid-desktop/src/main.rs`:

```rust
use mdid_desktop::{
    DesktopRuntimeSettings, DesktopWorkflowMode, DesktopWorkflowRequestState,
    DesktopWorkflowResponseState,
};

#[derive(Default)]
struct DesktopApp {
    request_state: DesktopWorkflowRequestState,
    response_state: DesktopWorkflowResponseState,
    runtime_settings: DesktopRuntimeSettings,
}
```

Add controls and submit handling after the status label:

```rust
ui.horizontal(|ui| {
    ui.label("Runtime host");
    ui.text_edit_singleline(&mut self.runtime_settings.host);
    ui.label("port");
    ui.text_edit_singleline(&mut self.runtime_settings.port_text);
});

if ui.button("Submit to local runtime").clicked() {
    match self
        .request_state
        .try_build_request()
        .and_then(|request| {
            let client = self.runtime_settings.client()?;
            client.submit(&request).map(|envelope| (request, envelope))
        }) {
        Ok((request, envelope)) => {
            self.response_state
                .apply_success_json(self.request_state.mode, envelope);
            self.response_state.banner = format!(
                "Runtime response rendered from local route {}.",
                request.route
            );
        }
        Err(error) => self.response_state.apply_error(format!("{error:?}")),
    }
}
```

If the chained `and_then` has a concrete error-type mismatch, replace it with nested `match` statements that preserve the same behavior and messages.

- [ ] **Step 4: Update README completion snapshot truthfully**

Change the completion table to reflect only the landed desktop local runtime submission improvement:

```markdown
| Desktop app | 23% | Bounded sensitive-workstation foundation now prepares local runtime CSV, XLSX, and PDF review requests, can submit those prepared envelopes to a localhost runtime, and renders runtime-shaped response panes with honest disclosures; file picker upload/download UX, vault browsing, decode, audit investigation, and full review workflows remain unimplemented. |
| Overall | 43% | Core workspace, vault MVP, tabular path, bounded DICOM/PDF/runtime slices, conservative media/FCS domain models, adapter/application review foundation, bounded runtime metadata review and PDF review entries, browser tabular/PDF review surface, desktop request-preparation/localhost-submit/response workbench foundation, and local CLI foundations are present; major workflow depth and surface parity remain missing; scope-drift controller/moat CLI wording is not counted as core product progress. |
```

Also update the desktop implemented bullet to mention localhost runtime submission and keep missing limitations explicit.

- [ ] **Step 5: Run targeted and broader tests**

Run: `cargo test -p mdid-desktop desktop_runtime_settings_ -- --nocapture`

Expected: PASS.

Run: `cargo test -p mdid-desktop`

Expected: PASS.

Run: `cargo test -p mdid-browser -p mdid-application`

Expected: PASS (or report any pre-existing failure with exact output).

- [ ] **Step 6: Commit**

```bash
git add crates/mdid-desktop/src/lib.rs crates/mdid-desktop/src/main.rs README.md docs/superpowers/plans/2026-04-28-desktop-local-runtime-submit.md
git commit -m "feat(desktop): wire local runtime submit action"
```

## Self-Review

- Spec coverage: the plan adds only a bounded desktop localhost submit action and README truth-sync, matching the highest-leverage desktop runtime-submission gap. It does not add controller/orchestration/agent semantics.
- Placeholder scan: no TBD/TODO/fill-in placeholders are present. The only contingency gives exact behavior-preserving nested-match fallback for a Rust type mismatch.
- Type consistency: `DesktopRuntimeClient`, `DesktopRuntimeSubmitError`, and `DesktopRuntimeSettings` names are consistent across tests, implementation, and UI wiring.
