# Duplicate Record ID Validation Implementation Plan

> **Implementation workflow:** Execute this plan task-by-task with strict TDD, independent review after each task, and checkbox (`- [ ]`) tracking.

**Goal:** Reject duplicate vault/portable record IDs consistently before decode/export/import requests reach sensitive runtime paths.

**Architecture:** Add fail-closed duplicate ID validation at each user-facing request builder and runtime/domain boundary that accepts a record ID list. Keep error text PHI-safe by reporting duplicate presence/count only, never echoing caller-provided paths, payloads, or record IDs. This is a core de-identification safety hardening slice, not a workflow coordination feature.

**Tech Stack:** Rust workspace, cargo tests, serde_json, existing mdid CLI/browser/desktop/runtime/vault crates.

---

## File Structure

- Modify `crates/mdid-cli/src/main.rs`: CLI JSON parsing for vault decode/export should reject duplicate UUIDs with PHI-safe errors.
- Modify `crates/mdid-browser/src/app.rs`: browser vault/portable form payload builders should reject duplicate record IDs before constructing runtime requests.
- Modify `crates/mdid-desktop/src/lib.rs`: desktop vault/portable request builders should reject duplicate record IDs before runtime submission.
- Modify `crates/mdid-runtime/tests/runtime_http.rs` and runtime handler code if needed: HTTP endpoints should reject duplicate record IDs even if a surface missed validation.
- Modify `crates/mdid-vault` tests/source if needed: domain vault export/decode request constructors should reject duplicate record IDs at the lowest shared boundary.

### Task 1: CLI duplicate record ID validation

**Files:**
- Modify: `crates/mdid-cli/src/main.rs`

- [x] **Step 1: Write failing CLI parser/unit tests**

Add tests near existing vault decode/export parser tests:

```rust
#[test]
fn rejects_duplicate_record_ids_json_for_vault_decode() {
    let err = parse_record_ids_json(
        r#"["550e8400-e29b-41d4-a716-446655440000","550e8400-e29b-41d4-a716-446655440000"]"#,
    )
    .expect_err("duplicate record ids must be rejected before decode");
    let message = err.to_string();
    assert!(message.contains("duplicate record id"));
    assert!(!message.contains("550e8400"));
}

#[test]
fn rejects_duplicate_record_ids_json_for_vault_export() {
    let err = parse_record_ids_json(
        r#"["550e8400-e29b-41d4-a716-446655440000","550e8400-e29b-41d4-a716-446655440000"]"#,
    )
    .expect_err("duplicate record ids must be rejected before export");
    let message = err.to_string();
    assert!(message.contains("duplicate record id"));
    assert!(!message.contains("550e8400"));
}
```

- [x] **Step 2: Run RED**

Run: `cargo test -p mdid-cli rejects_duplicate_record_ids_json_for_vault_decode`
Expected: FAIL because duplicate IDs are currently accepted or helper is missing duplicate checks.

Run: `cargo test -p mdid-cli rejects_duplicate_record_ids_json_for_vault_export`
Expected: FAIL because duplicate IDs are currently accepted or helper is missing duplicate checks.

- [x] **Step 3: Implement minimal CLI validation**

Inside the existing record ID parser, track a `HashSet<Uuid>` or `HashSet<String>` of normalized IDs. If insert fails, return the same CLI error type with exactly `duplicate record id is not allowed` and do not include the ID value.

- [x] **Step 4: Run GREEN and regression tests**

Run: `cargo test -p mdid-cli rejects_duplicate_record_ids_json_for_vault_decode`
Expected: PASS.

Run: `cargo test -p mdid-cli rejects_duplicate_record_ids_json_for_vault_export`
Expected: PASS.

Run: `cargo test -p mdid-cli`
Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add crates/mdid-cli/src/main.rs
git commit -m "fix(cli): reject duplicate vault record ids"
```

### Task 2: Browser duplicate record ID validation

**Files:**
- Modify: `crates/mdid-browser/src/app.rs`

- [ ] **Step 1: Write failing browser tests**

Add tests near existing portable/vault payload tests:

```rust
#[test]
fn browser_vault_decode_payload_rejects_duplicate_record_ids() {
    let err = build_vault_decode_payload(
        "scope-a",
        r#"["550e8400-e29b-41d4-a716-446655440000","550e8400-e29b-41d4-a716-446655440000"]"#,
        "case review",
    )
    .expect_err("browser must reject duplicate decode record ids");
    assert!(err.contains("duplicate record id"));
    assert!(!err.contains("550e8400"));
}

#[test]
fn browser_portable_export_payload_rejects_duplicate_record_ids() {
    let err = build_portable_export_payload(
        "scope-a",
        r#"["550e8400-e29b-41d4-a716-446655440000","550e8400-e29b-41d4-a716-446655440000"]"#,
        "portable passphrase",
        "case handoff",
    )
    .expect_err("browser must reject duplicate export record ids");
    assert!(err.contains("duplicate record id"));
    assert!(!err.contains("550e8400"));
}
```

- [ ] **Step 2: Run RED**

Run: `cargo test -p mdid-browser browser_vault_decode_payload_rejects_duplicate_record_ids browser_portable_export_payload_rejects_duplicate_record_ids -- --exact`
Expected: FAIL because duplicate IDs are accepted or helper names require adjustment to existing function names.

- [ ] **Step 3: Implement minimal browser validation**

Update the shared UUID array parser used by vault decode/export/portable export to reject duplicate normalized UUID strings with `duplicate record id is not allowed`.

- [ ] **Step 4: Run GREEN and broader browser tests**

Run: `cargo test -p mdid-browser browser_vault_decode_payload_rejects_duplicate_record_ids browser_portable_export_payload_rejects_duplicate_record_ids -- --exact`
Expected: PASS.

Run: `cargo test -p mdid-browser`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-browser/src/app.rs
git commit -m "fix(browser): reject duplicate vault record ids"
```

### Task 3: Desktop duplicate record ID validation

**Files:**
- Modify: `crates/mdid-desktop/src/lib.rs`

- [ ] **Step 1: Write failing desktop request-builder tests**

Add tests near existing portable/vault request tests:

```rust
#[test]
fn desktop_vault_decode_request_rejects_duplicate_record_ids() {
    let mut request = VaultRequestState::default();
    request.scope = "scope-a".to_string();
    request.record_ids_json = r#"["550e8400-e29b-41d4-a716-446655440000","550e8400-e29b-41d4-a716-446655440000"]"#.to_string();
    request.justification = "case review".to_string();

    let err = request
        .build_decode_request()
        .expect_err("desktop must reject duplicate decode record ids");
    assert!(err.contains("duplicate record id"));
    assert!(!err.contains("550e8400"));
}

#[test]
fn desktop_portable_export_request_rejects_duplicate_record_ids() {
    let mut request = PortableWorkflowRequest::default();
    request.record_ids_json = r#"["550e8400-e29b-41d4-a716-446655440000","550e8400-e29b-41d4-a716-446655440000"]"#.to_string();
    request.scope = "scope-a".to_string();
    request.passphrase = "portable passphrase".to_string();
    request.context = "case handoff".to_string();

    let err = request
        .build_export_request()
        .expect_err("desktop must reject duplicate export record ids");
    assert!(err.contains("duplicate record id"));
    assert!(!err.contains("550e8400"));
}
```

- [ ] **Step 2: Run RED**

Run: `cargo test -p mdid-desktop desktop_vault_decode_request_rejects_duplicate_record_ids desktop_portable_export_request_rejects_duplicate_record_ids -- --exact`
Expected: FAIL because duplicates are accepted or exact builder names need alignment.

- [ ] **Step 3: Implement minimal desktop validation**

Update the shared desktop record ID JSON parser to reject duplicate normalized UUID strings with a PHI-safe message.

- [ ] **Step 4: Run GREEN and broader desktop tests**

Run: `cargo test -p mdid-desktop desktop_vault_decode_request_rejects_duplicate_record_ids desktop_portable_export_request_rejects_duplicate_record_ids -- --exact`
Expected: PASS.

Run: `cargo test -p mdid-desktop`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-desktop/src/lib.rs
git commit -m "fix(desktop): reject duplicate vault record ids"
```

### Task 4: Runtime duplicate record ID validation

**Files:**
- Modify: `crates/mdid-runtime/tests/runtime_http.rs`
- Modify: runtime handler code under `crates/mdid-runtime/src/`

- [ ] **Step 1: Write failing runtime HTTP tests**

Add tests next to vault decode/export endpoint validation tests:

```rust
#[test]
fn vault_decode_endpoint_rejects_duplicate_record_ids() {
    let response = post_json(
        "/vault/decode",
        serde_json::json!({
            "vault_path": "local.vault",
            "vault_passphrase": "correct horse battery staple",
            "scope": "scope-a",
            "record_ids": [
                "550e8400-e29b-41d4-a716-446655440000",
                "550e8400-e29b-41d4-a716-446655440000"
            ],
            "justification": "case review"
        }),
    );
    assert_eq!(response.status(), 400);
    let body = response_body_text(response);
    assert!(body.contains("duplicate record id"));
    assert!(!body.contains("550e8400"));
}

#[test]
fn vault_export_endpoint_rejects_duplicate_record_ids() {
    let response = post_json(
        "/vault/export",
        serde_json::json!({
            "vault_path": "local.vault",
            "vault_passphrase": "correct horse battery staple",
            "scope": "scope-a",
            "record_ids": [
                "550e8400-e29b-41d4-a716-446655440000",
                "550e8400-e29b-41d4-a716-446655440000"
            ],
            "portable_passphrase": "portable passphrase",
            "context": "case handoff"
        }),
    );
    assert_eq!(response.status(), 400);
    let body = response_body_text(response);
    assert!(body.contains("duplicate record id"));
    assert!(!body.contains("550e8400"));
}
```

- [ ] **Step 2: Run RED**

Run: `cargo test -p mdid-runtime vault_decode_endpoint_rejects_duplicate_record_ids vault_export_endpoint_rejects_duplicate_record_ids -- --exact`
Expected: FAIL until runtime validation is added.

- [ ] **Step 3: Implement minimal runtime validation**

Add duplicate checks immediately after JSON request deserialization or use lower-layer constructors that validate duplicates. Return HTTP 400 with `duplicate record id is not allowed`.

- [ ] **Step 4: Run GREEN and broader runtime tests**

Run: `cargo test -p mdid-runtime vault_decode_endpoint_rejects_duplicate_record_ids vault_export_endpoint_rejects_duplicate_record_ids -- --exact`
Expected: PASS.

Run: `cargo test -p mdid-runtime`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-runtime/src crates/mdid-runtime/tests/runtime_http.rs
git commit -m "fix(runtime): reject duplicate vault record ids"
```

### Task 5: Domain/vault lowest-boundary duplicate validation

**Files:**
- Modify: `crates/mdid-domain` or `crates/mdid-vault` source/tests depending on where decode/export request value objects live.

- [ ] **Step 1: Write failing lowest-boundary tests**

Add tests beside existing vault workflow model or local vault store tests:

```rust
#[test]
fn decode_request_rejects_duplicate_record_ids() {
    let duplicate = uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
    let err = DecodeRequest::new(
        MappingScope::new("scope-a").unwrap(),
        vec![duplicate, duplicate],
        "case review".to_string(),
    )
    .expect_err("domain request must reject duplicate decode ids");
    assert!(err.to_string().contains("duplicate record id"));
    assert!(!err.to_string().contains("550e8400"));
}

#[test]
fn export_request_rejects_duplicate_record_ids() {
    let duplicate = uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
    let err = PortableExportRequest::new(
        MappingScope::new("scope-a").unwrap(),
        vec![duplicate, duplicate],
        "case handoff".to_string(),
    )
    .expect_err("domain request must reject duplicate export ids");
    assert!(err.to_string().contains("duplicate record id"));
    assert!(!err.to_string().contains("550e8400"));
}
```

- [ ] **Step 2: Run RED**

Run the exact crate/test command for the file that contains the request constructors, for example `cargo test -p mdid-domain decode_request_rejects_duplicate_record_ids export_request_rejects_duplicate_record_ids -- --exact`.
Expected: FAIL until lowest-boundary constructors reject duplicates.

- [ ] **Step 3: Implement minimal lowest-boundary validation**

Add duplicate checks to request constructors or equivalent shared validation function. Keep messages PHI-safe.

- [ ] **Step 4: Run GREEN and full workspace**

Run exact tests from Step 2.
Expected: PASS.

Run: `cargo test --workspace --all-targets`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-domain crates/mdid-vault
git commit -m "fix(vault): reject duplicate record ids at request boundary"
```

## Self-Review

Spec coverage: The plan covers CLI, browser, desktop, runtime, and lowest shared vault/domain boundaries for duplicate record ID rejection. Placeholder scan: no TBD/TODO/fill-in-later placeholders remain. Type consistency: all tasks use the same external behavior, `duplicate record id is not allowed`, and require PHI-safe error messages that do not echo UUIDs.
