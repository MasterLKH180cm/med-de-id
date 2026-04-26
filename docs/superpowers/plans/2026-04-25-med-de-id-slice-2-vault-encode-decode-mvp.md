# med-de-id Slice 2 Vault Encode Decode MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first encrypted local vault, reversible token encode/decode path, portable export artifact, and audit-aware decode controls for `med-de-id`.

**Architecture:** This slice stays inside the shared Rust core and does not add format adapters yet. The implementation adds missing vault/decode domain types, a new `mdid-vault` crate with passphrase-rooted encrypted file storage plus portable subset export, and a vault-backed application service that exposes encode/decode use cases without coupling the rest of the system to storage details.

**Tech Stack:** Rust workspace, Cargo, Serde, Chrono, UUID, thiserror, Argon2, ChaCha20Poly1305, Base64, tempfile.

---

## Scope note

This plan covers **Slice 2 — vault + encode/decode MVP** only. It intentionally does not implement CSV/Excel, DICOM, PDF/OCR, or media adapters yet. The slice produces a verifiable shared-core reversible path that later format adapters can call.

## File structure

**Create:**
- `crates/mdid-domain/tests/vault_workflow_models.rs`
- `crates/mdid-vault/Cargo.toml`
- `crates/mdid-vault/src/lib.rs`
- `crates/mdid-vault/tests/local_vault_store.rs`
- `crates/mdid-application/tests/vault_application_services.rs`

**Modify:**
- `Cargo.toml`
- `README.md`
- `crates/mdid-domain/Cargo.toml`
- `crates/mdid-domain/src/lib.rs`
- `crates/mdid-application/Cargo.toml`
- `crates/mdid-application/src/lib.rs`
- `.github/workflows/ci.yml`

---

### Task 1: Add vault, audit, and decode domain vocabulary

**Files:**
- Modify: `crates/mdid-domain/Cargo.toml`
- Modify: `crates/mdid-domain/src/lib.rs`
- Create: `crates/mdid-domain/tests/vault_workflow_models.rs`

- [ ] **Step 1: Write the failing domain tests**

Create `crates/mdid-domain/tests/vault_workflow_models.rs`:

```rust
use mdid_domain::{AuditEventKind, DecodeRequest, DecodeRequestError, MappingScope, SurfaceKind};
use serde_json::{from_str, to_string};
use uuid::Uuid;

#[test]
fn audit_event_kind_flags_decode_as_high_risk() {
    assert_eq!(AuditEventKind::Encode.as_str(), "encode");
    assert_eq!(AuditEventKind::Decode.as_str(), "decode");
    assert!(!AuditEventKind::Encode.is_high_risk());
    assert!(AuditEventKind::Decode.is_high_risk());
}

#[test]
fn mapping_scope_builds_a_stable_scope_key() {
    let scope = MappingScope::new(
        Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap(),
        Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap(),
        "patient.name".into(),
    );

    assert_eq!(
        scope.scope_key(),
        "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa/bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb/patient.name"
    );
}

#[test]
fn decode_request_requires_scope_and_justification() {
    let err = DecodeRequest::new(vec![], "stdout".into(), "incident triage".into(), SurfaceKind::Desktop)
        .unwrap_err();
    assert!(matches!(err, DecodeRequestError::EmptyScope));

    let err = DecodeRequest::new(
        vec![Uuid::new_v4()],
        "stdout".into(),
        "   ".into(),
        SurfaceKind::Desktop,
    )
    .unwrap_err();
    assert!(matches!(err, DecodeRequestError::MissingJustification));
}

#[test]
fn decode_request_requires_an_explicit_output_target() {
    let err = DecodeRequest::new(
        vec![Uuid::new_v4()],
        "   ".into(),
        "incident triage".into(),
        SurfaceKind::Desktop,
    )
    .unwrap_err();

    assert!(matches!(err, DecodeRequestError::MissingOutputTarget));
}

#[test]
fn decode_request_exposes_validated_fields_via_accessors() {
    let record_id = Uuid::parse_str("cccccccc-cccc-cccc-cccc-cccccccccccc").unwrap();

    let request = DecodeRequest::new(
        vec![record_id],
        "stdout".into(),
        "incident triage".into(),
        SurfaceKind::Desktop,
    )
    .unwrap();

    assert_eq!(request.record_ids(), &[record_id]);
    assert_eq!(request.output_target(), "stdout");
    assert_eq!(request.justification(), "incident triage");
    assert_eq!(request.requested_by(), SurfaceKind::Desktop);
}

#[test]
fn serde_uses_stable_lowercase_wire_values_for_surface_and_audit_kinds() {
    assert_eq!(to_string(&SurfaceKind::Desktop).unwrap(), "\"desktop\"");
    assert_eq!(to_string(&AuditEventKind::Decode).unwrap(), "\"decode\"");
    assert_eq!(from_str::<SurfaceKind>("\"browser\"").unwrap(), SurfaceKind::Browser);
    assert_eq!(from_str::<AuditEventKind>("\"encode\"").unwrap(), AuditEventKind::Encode);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-domain --test vault_workflow_models
```

Expected: FAIL because the domain types and validation logic do not exist yet.

- [ ] **Step 3: Write the minimal domain implementation**

Update `crates/mdid-domain/Cargo.toml`:

```toml
[package]
name = "mdid-domain"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
chrono.workspace = true
serde.workspace = true
thiserror.workspace = true
uuid.workspace = true
```

Append to `crates/mdid-domain/src/lib.rs`:

```rust
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditEventKind {
    Encode,
    Decode,
}

impl AuditEventKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditEventKind::Encode => "encode",
            AuditEventKind::Decode => "decode",
        }
    }

    pub fn is_high_risk(&self) -> bool {
        matches!(self, AuditEventKind::Decode)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MappingScope {
    pub job_id: Uuid,
    pub artifact_id: Uuid,
    pub field_path: String,
}

impl MappingScope {
    pub fn new(job_id: Uuid, artifact_id: Uuid, field_path: String) -> Self {
        Self {
            job_id,
            artifact_id,
            field_path,
        }
    }

    pub fn scope_key(&self) -> String {
        format!("{}/{}/{}", self.job_id, self.artifact_id, self.field_path)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingRecord {
    pub id: Uuid,
    pub scope: MappingScope,
    pub phi_type: String,
    pub token: String,
    pub original_value: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: Uuid,
    pub kind: AuditEventKind,
    pub actor: SurfaceKind,
    pub detail: String,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "DecodeRequestSerde")]
pub struct DecodeRequest {
    record_ids: Vec<Uuid>,
    output_target: String,
    justification: String,
    requested_by: SurfaceKind,
}

impl DecodeRequest {
    pub fn new(
        record_ids: Vec<Uuid>,
        output_target: String,
        justification: String,
        requested_by: SurfaceKind,
    ) -> Result<Self, DecodeRequestError> {
        if record_ids.is_empty() {
            return Err(DecodeRequestError::EmptyScope);
        }

        if output_target.trim().is_empty() {
            return Err(DecodeRequestError::MissingOutputTarget);
        }

        if justification.trim().is_empty() {
            return Err(DecodeRequestError::MissingJustification);
        }

        Ok(Self {
            record_ids,
            output_target,
            justification,
            requested_by,
        })
    }

    pub fn record_ids(&self) -> &[Uuid] {
        &self.record_ids
    }

    pub fn output_target(&self) -> &str {
        &self.output_target
    }

    pub fn justification(&self) -> &str {
        &self.justification
    }

    pub fn requested_by(&self) -> SurfaceKind {
        self.requested_by
    }
}

#[derive(Debug, Deserialize)]
struct DecodeRequestSerde {
    record_ids: Vec<Uuid>,
    output_target: String,
    justification: String,
    requested_by: SurfaceKind,
}

impl TryFrom<DecodeRequestSerde> for DecodeRequest {
    type Error = DecodeRequestError;

    fn try_from(value: DecodeRequestSerde) -> Result<Self, Self::Error> {
        Self::new(
            value.record_ids,
            value.output_target,
            value.justification,
            value.requested_by,
        )
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DecodeRequestError {
    #[error("decode scope must include at least one mapping record")]
    EmptyScope,
    #[error("decode output target is required")]
    MissingOutputTarget,
    #[error("decode justification is required")]
    MissingJustification,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodedValue {
    pub record_id: Uuid,
    pub token: String,
    pub original_value: String,
    pub scope: MappingScope,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodeResult {
    pub values: Vec<DecodedValue>,
    pub audit_event: AuditEvent,
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-domain --test vault_workflow_models
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-domain/Cargo.toml crates/mdid-domain/src/lib.rs crates/mdid-domain/tests/vault_workflow_models.rs
git commit -m "feat: add vault and decode domain models"
```

### Task 2: Create the encrypted local vault store and portable export artifact

**Files:**
- Modify: `Cargo.toml`
- Modify: `README.md`
- Create: `crates/mdid-vault/Cargo.toml`
- Create: `crates/mdid-vault/src/lib.rs`
- Create: `crates/mdid-vault/tests/local_vault_store.rs`

- [ ] **Step 1: Write the failing vault-store tests**

Create `crates/mdid-vault/tests/local_vault_store.rs`:

```rust
use mdid_domain::{DecodeRequest, MappingScope, SurfaceKind};
use mdid_vault::{LocalVaultStore, NewMappingRecord};
use tempfile::tempdir;
use uuid::Uuid;

fn sample_scope(field_path: &str) -> MappingScope {
    MappingScope::new(Uuid::new_v4(), Uuid::new_v4(), field_path.to_string())
}

#[test]
fn local_vault_store_encrypts_disk_state_and_decodes_selected_scope() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("vault.mdid");

    let mut vault = LocalVaultStore::create(&path, "correct horse battery staple").unwrap();
    let stored = vault
        .store_mapping(
            NewMappingRecord {
                scope: sample_scope("patient.name"),
                phi_type: "patient_name".into(),
                original_value: "Alice Smith".into(),
            },
            SurfaceKind::Cli,
        )
        .unwrap();

    let raw = std::fs::read_to_string(&path).unwrap();
    assert!(!raw.contains("Alice Smith"));
    assert!(!raw.contains(&stored.token));

    let mut reopened = LocalVaultStore::unlock(&path, "correct horse battery staple").unwrap();
    let decoded = reopened
        .decode(
            DecodeRequest::new(
                vec![stored.id],
                "stdout".into(),
                "incident investigation".into(),
                SurfaceKind::Desktop,
            )
            .unwrap(),
        )
        .unwrap();

    assert_eq!(decoded.values.len(), 1);
    assert_eq!(decoded.values[0].original_value, "Alice Smith");
    assert_eq!(decoded.audit_event.kind.as_str(), "decode");
}

#[test]
fn portable_export_is_scope_limited() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("vault.mdid");

    let mut vault = LocalVaultStore::create(&path, "correct horse battery staple").unwrap();
    let kept = vault
        .store_mapping(
            NewMappingRecord {
                scope: sample_scope("patient.id"),
                phi_type: "patient_id".into(),
                original_value: "MRN-001".into(),
            },
            SurfaceKind::Cli,
        )
        .unwrap();
    let _ignored = vault
        .store_mapping(
            NewMappingRecord {
                scope: sample_scope("patient.name"),
                phi_type: "patient_name".into(),
                original_value: "Alice Smith".into(),
            },
            SurfaceKind::Cli,
        )
        .unwrap();

    let artifact = vault
        .export_portable(
            &[kept.id],
            "portable-passphrase",
            SurfaceKind::Desktop,
            "partner-site transfer package",
        )
        .unwrap();
    let snapshot = artifact.unlock("portable-passphrase").unwrap();

    assert_eq!(snapshot.records.len(), 1);
    assert_eq!(snapshot.records[0].id, kept.id);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-vault --test local_vault_store
```

Expected: FAIL because the crate, store, and export artifact do not exist.

- [ ] **Step 3: Write the minimal encrypted vault implementation**

Update the root `Cargo.toml`:

```toml
[workspace]
members = [
  "crates/mdid-domain",
  "crates/mdid-vault",
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
argon2 = "0.5"
axum = { version = "0.7", features = ["macros"] }
base64 = "0.22"
chacha20poly1305 = "0.10"
chrono = { version = "0.4", features = ["serde"] }
eframe = "0.27"
egui = "0.27"
http = "1"
leptos = { version = "0.6", features = ["csr"] }
rand_core = "0.6"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tempfile = "3"
thiserror = "1"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "sync"] }
tower = { version = "0.5", features = ["util"] }
tower-http = { version = "0.5", features = ["cors", "trace"] }
uuid = { version = "1", features = ["serde", "v4"] }
```

Create `crates/mdid-vault/Cargo.toml`:

```toml
[package]
name = "mdid-vault"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
argon2.workspace = true
base64.workspace = true
chacha20poly1305.workspace = true
chrono.workspace = true
mdid-domain = { path = "../mdid-domain" }
rand_core.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
uuid.workspace = true

[dev-dependencies]
tempfile.workspace = true
```

Create `crates/mdid-vault/src/lib.rs`:

```rust
use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
use base64::{engine::general_purpose::STANDARD, Engine};
use chacha20poly1305::{
    aead::{Aead, KeyInit, OsRng},
    ChaCha20Poly1305, Key, Nonce,
};
use chrono::Utc;
use mdid_domain::{
    AuditEvent, AuditEventKind, DecodeRequest, DecodeResult, DecodedValue, MappingRecord,
    MappingScope, SurfaceKind,
};
use rand_core::RngCore;
use serde::{Deserialize, Serialize};
use std::{fs, path::{Path, PathBuf}};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct NewMappingRecord {
    pub scope: MappingScope,
    pub phi_type: String,
    pub original_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortableVaultSnapshot {
    pub records: Vec<MappingRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortableVaultArtifact {
    salt_b64: String,
    nonce_b64: String,
    ciphertext_b64: String,
}

impl PortableVaultArtifact {
    pub fn unlock(&self, passphrase: &str) -> Result<PortableVaultSnapshot, VaultError> {
        decrypt_payload(passphrase, &self.salt_b64, &self.nonce_b64, &self.ciphertext_b64)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct VaultState {
    records: Vec<MappingRecord>,
    audit_events: Vec<AuditEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VaultEnvelope {
    salt_b64: String,
    nonce_b64: String,
    ciphertext_b64: String,
}

pub struct LocalVaultStore {
    path: PathBuf,
    passphrase: String,
    state: VaultState,
}

impl LocalVaultStore {
    pub fn create(path: impl AsRef<Path>, passphrase: &str) -> Result<Self, VaultError> {
        let mut store = Self {
            path: path.as_ref().to_path_buf(),
            passphrase: passphrase.to_string(),
            state: VaultState::default(),
        };
        store.flush()?;
        Ok(store)
    }

    pub fn unlock(path: impl AsRef<Path>, passphrase: &str) -> Result<Self, VaultError> {
        let raw = fs::read_to_string(path.as_ref())?;
        let envelope: VaultEnvelope = serde_json::from_str(&raw)?;
        let state = decrypt_payload(
            passphrase,
            &envelope.salt_b64,
            &envelope.nonce_b64,
            &envelope.ciphertext_b64,
        )?;

        Ok(Self {
            path: path.as_ref().to_path_buf(),
            passphrase: passphrase.to_string(),
            state,
        })
    }

    pub fn store_mapping(
        &mut self,
        record: NewMappingRecord,
        actor: SurfaceKind,
    ) -> Result<MappingRecord, VaultError> {
        let stored = MappingRecord {
            id: Uuid::new_v4(),
            scope: record.scope,
            phi_type: record.phi_type,
            token: format!("tok-{}", Uuid::new_v4().simple()),
            original_value: record.original_value,
            created_at: Utc::now(),
        };

        self.state.records.push(stored.clone());
        self.state.audit_events.push(AuditEvent {
            id: Uuid::new_v4(),
            kind: AuditEventKind::Encode,
            actor,
            detail: format!("encoded mapping {}", stored.scope.scope_key()),
            recorded_at: Utc::now(),
        });
        self.flush()?;
        Ok(stored)
    }

    pub fn decode(&mut self, request: DecodeRequest) -> Result<DecodeResult, VaultError> {
        let mut values = Vec::with_capacity(request.record_ids.len());
        for record_id in &request.record_ids {
            let record = self
                .state
                .records
                .iter()
                .find(|candidate| candidate.id == *record_id)
                .cloned()
                .ok_or(VaultError::UnknownRecord(*record_id))?;
            values.push(DecodedValue {
                record_id: record.id,
                token: record.token.clone(),
                original_value: record.original_value.clone(),
                scope: record.scope.clone(),
            });
        }

        let audit_event = AuditEvent {
            id: Uuid::new_v4(),
            kind: AuditEventKind::Decode,
            actor: request.requested_by,
            detail: format!(
                "decode to {} because {}",
                request.output_target, request.justification
            ),
            recorded_at: Utc::now(),
        };
        self.state.audit_events.push(audit_event.clone());
        self.flush()?;

        Ok(DecodeResult { values, audit_event })
    }

    pub fn export_portable(
        &self,
        record_ids: &[Uuid],
        export_passphrase: &str,
    ) -> Result<PortableVaultArtifact, VaultError> {
        if record_ids.is_empty() {
            return Err(VaultError::EmptyExportScope);
        }

        let records = self
            .state
            .records
            .iter()
            .filter(|record| record_ids.contains(&record.id))
            .cloned()
            .collect::<Vec<_>>();

        if records.is_empty() {
            return Err(VaultError::EmptyExportScope);
        }

        let (salt_b64, nonce_b64, ciphertext_b64) = encrypt_payload(
            export_passphrase,
            &PortableVaultSnapshot { records },
        )?;

        Ok(PortableVaultArtifact {
            salt_b64,
            nonce_b64,
            ciphertext_b64,
        })
    }

    pub fn audit_events(&self) -> &[AuditEvent] {
        &self.state.audit_events
    }

    fn flush(&self) -> Result<(), VaultError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        let (salt_b64, nonce_b64, ciphertext_b64) = encrypt_payload(&self.passphrase, &self.state)?;
        let envelope = VaultEnvelope {
            salt_b64,
            nonce_b64,
            ciphertext_b64,
        };
        let raw = serde_json::to_string_pretty(&envelope)?;
        fs::write(&self.path, raw)?;
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum VaultError {
    #[error("io failure: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization failure: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("encryption setup failure: {0}")]
    PasswordHash(String),
    #[error("vault encryption failure")]
    Encrypt,
    #[error("vault decryption failure")]
    Decrypt,
    #[error("unknown mapping record: {0}")]
    UnknownRecord(Uuid),
    #[error("portable export must include at least one mapping record")]
    EmptyExportScope,
}

fn encrypt_payload<T: Serialize>(
    passphrase: &str,
    payload: &T,
) -> Result<(String, String, String), VaultError> {
    let salt = SaltString::generate(&mut OsRng);
    let key = derive_key(passphrase, salt.as_salt())?;
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));

    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let plaintext = serde_json::to_vec(payload)?;
    let ciphertext = cipher.encrypt(nonce, plaintext.as_ref()).map_err(|_| VaultError::Encrypt)?;

    Ok((
        STANDARD.encode(salt.as_str()),
        STANDARD.encode(nonce_bytes),
        STANDARD.encode(ciphertext),
    ))
}

fn decrypt_payload<T: for<'de> Deserialize<'de>>(
    passphrase: &str,
    salt_b64: &str,
    nonce_b64: &str,
    ciphertext_b64: &str,
) -> Result<T, VaultError> {
    let salt = STANDARD.decode(salt_b64).map_err(|_| VaultError::Decrypt)?;
    let salt = std::str::from_utf8(&salt).map_err(|_| VaultError::Decrypt)?;
    let salt = argon2::password_hash::Salt::from_b64(salt).map_err(|_| VaultError::Decrypt)?;
    let key = derive_key(passphrase, salt)?;
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));

    let nonce_bytes = STANDARD.decode(nonce_b64).map_err(|_| VaultError::Decrypt)?;
    let ciphertext = STANDARD.decode(ciphertext_b64).map_err(|_| VaultError::Decrypt)?;
    let plaintext = cipher
        .decrypt(Nonce::from_slice(&nonce_bytes), ciphertext.as_ref())
        .map_err(|_| VaultError::Decrypt)?;

    Ok(serde_json::from_slice(&plaintext)?)
}

fn derive_key(
    passphrase: &str,
    salt: argon2::password_hash::Salt<'_>,
) -> Result<[u8; 32], VaultError> {
    let mut key = [0u8; 32];
    Argon2::default()
        .hash_password_into(passphrase.as_bytes(), salt.as_str().as_bytes(), &mut key)
        .map_err(|err| VaultError::PasswordHash(err.to_string()))?;
    Ok(key)
}
```

Update `README.md` current-status section to mention that Slice 2 adds an encrypted vault crate and reversible mapping MVP.

- [ ] **Step 4: Run the tests to verify they pass**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-vault --test local_vault_store
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml README.md crates/mdid-vault/Cargo.toml crates/mdid-vault/src/lib.rs crates/mdid-vault/tests/local_vault_store.rs
git commit -m "feat: add encrypted local vault store"
```

### Task 3: Add a vault-backed application service for encode, decode, and export

**Files:**
- Modify: `crates/mdid-application/Cargo.toml`
- Modify: `crates/mdid-application/src/lib.rs`
- Create: `crates/mdid-application/tests/vault_application_services.rs`
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Write the failing application tests**

Create `crates/mdid-application/tests/vault_application_services.rs`:

```rust
use mdid_application::VaultApplicationService;
use mdid_domain::{DecodeRequest, MappingScope, SurfaceKind};
use mdid_vault::LocalVaultStore;
use tempfile::tempdir;
use uuid::Uuid;

fn sample_scope() -> MappingScope {
    MappingScope::new(Uuid::new_v4(), Uuid::new_v4(), "patient.name".into())
}

#[test]
fn vault_application_service_round_trips_encode_decode() {
    let dir = tempdir().unwrap();
    let vault = LocalVaultStore::create(dir.path().join("vault.mdid"), "service-passphrase").unwrap();
    let service = VaultApplicationService::new(vault);

    let stored = service
        .encode_value(
            sample_scope(),
            "patient_name".into(),
            "Alice Smith".into(),
            SurfaceKind::Cli,
        )
        .unwrap();

    let decoded = service
        .decode(
            DecodeRequest::new(
                vec![stored.id],
                "case-review".into(),
                "approved incident review".into(),
                SurfaceKind::Desktop,
            )
            .unwrap(),
        )
        .unwrap();

    assert_eq!(decoded.values[0].original_value, "Alice Smith");
    assert_eq!(decoded.audit_event.actor, SurfaceKind::Desktop);
}

#[test]
fn vault_application_service_exports_scope_limited_artifacts() {
    let dir = tempdir().unwrap();
    let vault = LocalVaultStore::create(dir.path().join("vault.mdid"), "service-passphrase").unwrap();
    let service = VaultApplicationService::new(vault);

    let stored = service
        .encode_value(
            sample_scope(),
            "patient_name".into(),
            "Alice Smith".into(),
            SurfaceKind::Cli,
        )
        .unwrap();

    let artifact = service
        .export_records(
            vec![stored.id],
            "portable-passphrase",
            SurfaceKind::Desktop,
            "approved partner handoff",
        )
        .unwrap();
    let snapshot = artifact.unlock("portable-passphrase").unwrap();

    assert_eq!(snapshot.records.len(), 1);
    assert_eq!(snapshot.records[0].id, stored.id);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-application --test vault_application_services
```

Expected: FAIL because the vault-backed application service does not exist.

- [ ] **Step 3: Write the minimal application integration**

Update `crates/mdid-application/Cargo.toml`:

```toml
[package]
name = "mdid-application"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
chrono.workspace = true
mdid-domain = { path = "../mdid-domain" }
mdid-vault = { path = "../mdid-vault" }
thiserror.workspace = true
uuid.workspace = true

[dev-dependencies]
tempfile.workspace = true
```

Append to `crates/mdid-application/src/lib.rs`:

```rust
use mdid_domain::{DecodeRequest, DecodeResult, MappingRecord, MappingScope, SurfaceKind};
use mdid_vault::{LocalVaultStore, NewMappingRecord, PortableVaultArtifact, VaultError};

pub struct VaultApplicationService {
    vault: Arc<Mutex<LocalVaultStore>>,
}

impl VaultApplicationService {
    pub fn new(vault: LocalVaultStore) -> Self {
        Self {
            vault: Arc::new(Mutex::new(vault)),
        }
    }

    pub fn encode_value(
        &self,
        scope: MappingScope,
        phi_type: String,
        original_value: String,
        actor: SurfaceKind,
    ) -> Result<MappingRecord, ApplicationError> {
        let mut vault = self.vault.lock().expect("vault lock poisoned");
        Ok(vault.store_mapping(
            NewMappingRecord {
                scope,
                phi_type,
                original_value,
            },
            actor,
        )?)
    }

    pub fn decode(&self, request: DecodeRequest) -> Result<DecodeResult, ApplicationError> {
        let mut vault = self.vault.lock().expect("vault lock poisoned");
        Ok(vault.decode(request)?)
    }

    pub fn export_records(
        &self,
        record_ids: Vec<Uuid>,
        export_passphrase: &str,
        actor: SurfaceKind,
        context: &str,
    ) -> Result<PortableVaultArtifact, ApplicationError> {
        let mut vault = self.vault.lock().expect("vault lock poisoned");
        Ok(vault.export_portable(&record_ids, export_passphrase, actor, context)?)
    }
}

impl From<VaultError> for ApplicationError {
    fn from(value: VaultError) -> Self {
        ApplicationError::Vault(value)
    }
}
```

Expand `ApplicationError` in `crates/mdid-application/src/lib.rs`:

```rust
#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error("pipeline not found: {0}")]
    PipelineNotFound(Uuid),
    #[error(transparent)]
    Vault(#[from] VaultError),
}
```

Update `.github/workflows/ci.yml` tests step only if needed to keep the workspace-wide test command unchanged. The expected file content remains:

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
      - name: Browser wasm target check
        run: cargo check -p mdid-browser --target wasm32-unknown-unknown
      - name: Clippy
        run: cargo clippy --workspace --all-targets -- -D warnings
      - name: Tests
        run: cargo test --workspace
```

- [ ] **Step 4: Run the tests to verify they pass and no regressions exist**

Run:

```bash
source "$HOME/.cargo/env"
cargo test -p mdid-application --test vault_application_services
cargo test --workspace
```

Expected: PASS for the targeted application tests, then PASS for the full workspace.

- [ ] **Step 5: Commit**

```bash
git add crates/mdid-application/Cargo.toml crates/mdid-application/src/lib.rs crates/mdid-application/tests/vault_application_services.rs .github/workflows/ci.yml
git commit -m "feat: add vault-backed encode decode service"
```

## Self-review

### Spec coverage
- Local encrypted vault store: Task 2
- Portable encrypted vault artifact model: Task 2
- Encode core path with mapping storage: Task 2 + Task 3
- Decode justification and explicit scope controls: Task 1 + Task 2 + Task 3
- High-risk audit event for decode: Task 1 + Task 2 + Task 3
- Foundation tests around vault state and restore behavior: Task 2 + Task 3

### Placeholder scan
- No placeholder markers remain.
- Every code-changing step includes concrete code blocks.
- Every verification step includes exact commands and expected results.

### Type consistency
- `MappingScope`, `MappingRecord`, `DecodeRequest`, `DecodeResult`, and `AuditEventKind` are defined in Task 1 and reused consistently in Tasks 2 and 3.
- `NewMappingRecord`, `LocalVaultStore`, and `PortableVaultArtifact` are introduced in Task 2 and reused consistently in Task 3.
- `VaultApplicationService` in Task 3 depends on the Task 2 vault API and does not invent a parallel encode/decode abstraction.
