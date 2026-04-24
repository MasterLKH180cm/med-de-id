use argon2::{Algorithm, Argon2, Params, Version};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use chacha20poly1305::{
    aead::{Aead, KeyInit, OsRng},
    ChaCha20Poly1305, Nonce,
};
use chrono::Utc;
use mdid_domain::{
    AuditEvent, AuditEventKind, DecodeRequest, DecodeResult, DecodedValue, MappingRecord,
    MappingScope, SurfaceKind,
};
use rand_core::RngCore;
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

#[derive(Clone)]
pub struct NewMappingRecord {
    pub scope: MappingScope,
    pub phi_type: String,
    pub original_value: String,
}

impl std::fmt::Debug for NewMappingRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NewMappingRecord")
            .field("scope", &self.scope)
            .field("phi_type", &self.phi_type)
            .field("original_value", &"<redacted>")
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortableVaultSnapshot {
    pub records: Vec<MappingRecord>,
}

const CHACHA20POLY1305_KEY_LEN: usize = 32;
const CHACHA20POLY1305_NONCE_LEN: usize = 12;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KdfMetadata {
    algorithm: String,
    version: u32,
    memory_cost_kib: u32,
    iterations: u32,
    parallelism: u32,
    output_len: usize,
}

impl Default for KdfMetadata {
    fn default() -> Self {
        default_kdf_metadata()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortableVaultArtifact {
    #[serde(default)]
    kdf: KdfMetadata,
    salt_b64: String,
    nonce_b64: String,
    ciphertext_b64: String,
}

impl PortableVaultArtifact {
    pub fn unlock(&self, passphrase: &str) -> Result<PortableVaultSnapshot, VaultError> {
        decrypt_payload(
            passphrase,
            &self.kdf,
            &self.salt_b64,
            &self.nonce_b64,
            &self.ciphertext_b64,
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct VaultState {
    records: Vec<MappingRecord>,
    audit_events: Vec<AuditEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VaultEnvelope {
    #[serde(default)]
    kdf: KdfMetadata,
    salt_b64: String,
    nonce_b64: String,
    ciphertext_b64: String,
}

pub struct LocalVaultStore {
    path: PathBuf,
    passphrase: String,
    state: VaultState,
}

impl std::fmt::Debug for LocalVaultStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalVaultStore")
            .field("path", &self.path)
            .field("passphrase", &"<redacted>")
            .field("record_count", &self.state.records.len())
            .field("audit_event_count", &self.state.audit_events.len())
            .finish()
    }
}

impl LocalVaultStore {
    pub fn create(path: impl AsRef<Path>, passphrase: &str) -> Result<Self, VaultError> {
        ensure_non_blank_passphrase(passphrase)?;
        let path = path.as_ref().to_path_buf();
        if path.exists() {
            return Err(VaultError::AlreadyExists(path));
        }

        let store = Self {
            path,
            passphrase: passphrase.to_string(),
            state: VaultState::default(),
        };
        store.flush()?;
        Ok(store)
    }

    pub fn unlock(path: impl AsRef<Path>, passphrase: &str) -> Result<Self, VaultError> {
        ensure_non_blank_passphrase(passphrase)?;
        let raw = fs::read_to_string(path.as_ref())?;
        let envelope: VaultEnvelope = serde_json::from_str(&raw)?;
        let state = decrypt_payload(
            passphrase,
            &envelope.kdf,
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

        let mut staged_state = self.state.clone();
        staged_state.records.push(stored.clone());
        staged_state.audit_events.push(AuditEvent {
            id: Uuid::new_v4(),
            kind: AuditEventKind::Encode,
            actor,
            detail: format!("encoded mapping {}", stored.scope.scope_key()),
            recorded_at: Utc::now(),
        });
        self.flush_state(&staged_state)?;
        self.state = staged_state;

        Ok(stored)
    }

    pub fn decode(&mut self, request: DecodeRequest) -> Result<DecodeResult, VaultError> {
        let values = request
            .record_ids()
            .iter()
            .map(|record_id| {
                let record = self
                    .state
                    .records
                    .iter()
                    .find(|candidate| candidate.id == *record_id)
                    .cloned()
                    .ok_or(VaultError::UnknownRecord(*record_id))?;

                Ok(DecodedValue {
                    record_id: record.id,
                    token: record.token,
                    original_value: record.original_value,
                    scope: record.scope,
                })
            })
            .collect::<Result<Vec<_>, VaultError>>()?;

        let decoded_record_ids = values
            .iter()
            .map(|value| value.record_id.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let audit_event = AuditEvent {
            id: Uuid::new_v4(),
            kind: AuditEventKind::Decode,
            actor: request.requested_by(),
            detail: format!(
                "decode to {} because {} decoded {} record{} record_ids=[{}]",
                request.output_target(),
                request.justification(),
                values.len(),
                if values.len() == 1 { "" } else { "s" },
                decoded_record_ids,
            ),
            recorded_at: Utc::now(),
        };
        let mut staged_state = self.state.clone();
        staged_state.audit_events.push(audit_event.clone());
        self.flush_state(&staged_state)?;
        self.state = staged_state;

        Ok(DecodeResult {
            values,
            audit_event,
        })
    }

    pub fn export_portable(
        &mut self,
        record_ids: &[Uuid],
        export_passphrase: &str,
        actor: SurfaceKind,
        context: &str,
    ) -> Result<PortableVaultArtifact, VaultError> {
        ensure_non_blank_passphrase(export_passphrase)?;
        ensure_non_blank_export_context(context)?;
        if record_ids.is_empty() {
            return Err(VaultError::EmptyExportScope);
        }

        let records = record_ids
            .iter()
            .map(|record_id| {
                self.state
                    .records
                    .iter()
                    .find(|candidate| candidate.id == *record_id)
                    .cloned()
                    .ok_or(VaultError::UnknownRecord(*record_id))
            })
            .collect::<Result<Vec<_>, VaultError>>()?;
        let record_ids_detail = records
            .iter()
            .map(|record| record.id.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        let encrypted = encrypt_payload(
            export_passphrase,
            &PortableVaultSnapshot {
                records: records.clone(),
            },
        )?;
        let mut staged_state = self.state.clone();
        staged_state.audit_events.push(AuditEvent {
            id: Uuid::new_v4(),
            kind: AuditEventKind::Export,
            actor,
            detail: format!(
                "portable export context=\"{}\" exported {} record{} record_ids=[{}]",
                context.trim(),
                records.len(),
                if records.len() == 1 { "" } else { "s" },
                record_ids_detail,
            ),
            recorded_at: Utc::now(),
        });
        self.flush_state(&staged_state)?;
        self.state = staged_state;

        Ok(PortableVaultArtifact {
            kdf: encrypted.kdf,
            salt_b64: encrypted.salt_b64,
            nonce_b64: encrypted.nonce_b64,
            ciphertext_b64: encrypted.ciphertext_b64,
        })
    }

    pub fn audit_events(&self) -> &[AuditEvent] {
        &self.state.audit_events
    }

    fn flush(&self) -> Result<(), VaultError> {
        self.flush_state(&self.state)
    }

    fn flush_state(&self, state: &VaultState) -> Result<(), VaultError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        let encrypted = encrypt_payload(&self.passphrase, state)?;
        let envelope = VaultEnvelope {
            kdf: encrypted.kdf,
            salt_b64: encrypted.salt_b64,
            nonce_b64: encrypted.nonce_b64,
            ciphertext_b64: encrypted.ciphertext_b64,
        };
        let raw = serde_json::to_string_pretty(&envelope)?;
        atomic_write(&self.path, &raw)?;
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum VaultError {
    #[error("io failure: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization failure: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("passphrase must not be blank or whitespace")]
    BlankPassphrase,
    #[error("portable export context must not be blank or whitespace")]
    BlankExportContext,
    #[error("vault path already exists: {0}")]
    AlreadyExists(PathBuf),
    #[error("unsupported kdf algorithm: {0}")]
    UnsupportedKdfAlgorithm(String),
    #[error("unsupported kdf version: {0:#x}")]
    UnsupportedKdfVersion(u32),
    #[error("invalid kdf parameters")]
    InvalidKdfParameters,
    #[error("invalid nonce length: expected {expected} bytes, got {actual}")]
    InvalidNonceLength { expected: usize, actual: usize },
    #[error("key derivation failure")]
    KeyDerivation,
    #[error("vault encryption failure")]
    Encrypt,
    #[error("vault decryption failure")]
    Decrypt,
    #[error("unknown mapping record: {0}")]
    UnknownRecord(Uuid),
    #[error("portable export must include at least one mapping record")]
    EmptyExportScope,
}

#[derive(Debug, Clone)]
struct EncryptedPayload {
    kdf: KdfMetadata,
    salt_b64: String,
    nonce_b64: String,
    ciphertext_b64: String,
}

fn encrypt_payload<T: Serialize>(
    passphrase: &str,
    payload: &T,
) -> Result<EncryptedPayload, VaultError> {
    ensure_non_blank_passphrase(passphrase)?;

    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);

    let kdf = default_kdf_metadata();
    let key = derive_key(passphrase, &salt, &kdf)?;
    let cipher = ChaCha20Poly1305::new((&key).into());

    let mut nonce_bytes = [0u8; CHACHA20POLY1305_NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let plaintext = serde_json::to_vec(payload)?;
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_ref())
        .map_err(|_| VaultError::Encrypt)?;

    Ok(EncryptedPayload {
        kdf,
        salt_b64: STANDARD.encode(salt),
        nonce_b64: STANDARD.encode(nonce_bytes),
        ciphertext_b64: STANDARD.encode(ciphertext),
    })
}

fn decrypt_payload<T: for<'de> Deserialize<'de>>(
    passphrase: &str,
    kdf: &KdfMetadata,
    salt_b64: &str,
    nonce_b64: &str,
    ciphertext_b64: &str,
) -> Result<T, VaultError> {
    ensure_non_blank_passphrase(passphrase)?;

    let salt = STANDARD.decode(salt_b64).map_err(|_| VaultError::Decrypt)?;
    let nonce_bytes = STANDARD
        .decode(nonce_b64)
        .map_err(|_| VaultError::Decrypt)?;
    let ciphertext = STANDARD
        .decode(ciphertext_b64)
        .map_err(|_| VaultError::Decrypt)?;

    if nonce_bytes.len() != CHACHA20POLY1305_NONCE_LEN {
        return Err(VaultError::InvalidNonceLength {
            expected: CHACHA20POLY1305_NONCE_LEN,
            actual: nonce_bytes.len(),
        });
    }

    let key = derive_key(passphrase, &salt, kdf)?;
    let cipher = ChaCha20Poly1305::new((&key).into());
    let nonce = Nonce::clone_from_slice(&nonce_bytes);
    let plaintext = cipher
        .decrypt(&nonce, ciphertext.as_ref())
        .map_err(|_| VaultError::Decrypt)?;

    Ok(serde_json::from_slice(&plaintext)?)
}

fn derive_key(
    passphrase: &str,
    salt: &[u8],
    kdf: &KdfMetadata,
) -> Result<[u8; CHACHA20POLY1305_KEY_LEN], VaultError> {
    let algorithm = Algorithm::new(&kdf.algorithm)
        .map_err(|_| VaultError::UnsupportedKdfAlgorithm(kdf.algorithm.clone()))?;
    let version = Version::try_from(kdf.version)
        .map_err(|_| VaultError::UnsupportedKdfVersion(kdf.version))?;
    let params = Params::new(
        kdf.memory_cost_kib,
        kdf.iterations,
        kdf.parallelism,
        Some(kdf.output_len),
    )
    .map_err(|_| VaultError::InvalidKdfParameters)?;

    let mut key = [0u8; CHACHA20POLY1305_KEY_LEN];
    Argon2::new(algorithm, version, params)
        .hash_password_into(passphrase.as_bytes(), salt, &mut key)
        .map_err(|_| VaultError::KeyDerivation)?;
    Ok(key)
}

fn default_kdf_metadata() -> KdfMetadata {
    KdfMetadata {
        algorithm: Algorithm::Argon2id.to_string(),
        version: u32::from(Version::V0x13),
        memory_cost_kib: Params::DEFAULT_M_COST,
        iterations: Params::DEFAULT_T_COST,
        parallelism: Params::DEFAULT_P_COST,
        output_len: CHACHA20POLY1305_KEY_LEN,
    }
}

fn ensure_non_blank_passphrase(passphrase: &str) -> Result<(), VaultError> {
    if passphrase.trim().is_empty() {
        return Err(VaultError::BlankPassphrase);
    }

    Ok(())
}

fn ensure_non_blank_export_context(context: &str) -> Result<(), VaultError> {
    if context.trim().is_empty() {
        return Err(VaultError::BlankExportContext);
    }

    Ok(())
}

fn atomic_write(path: &Path, contents: &str) -> Result<(), VaultError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("vault.mdid");
    let temp_path = path.with_file_name(format!(".{file_name}.{}.tmp", Uuid::new_v4().simple()));

    let mut temp_file = fs::File::create(&temp_path)?;
    temp_file.write_all(contents.as_bytes())?;
    temp_file.sync_all()?;
    drop(temp_file);

    replace_atomic(&temp_path, path)?;

    if let Some(parent) = path.parent() {
        if let Ok(directory) = fs::File::open(parent) {
            let _ = directory.sync_all();
        }
    }

    Ok(())
}

#[cfg(not(windows))]
fn replace_atomic(temp_path: &Path, path: &Path) -> Result<(), VaultError> {
    fs::rename(temp_path, path)?;
    Ok(())
}

#[cfg(windows)]
fn replace_atomic(temp_path: &Path, path: &Path) -> Result<(), VaultError> {
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

#[cfg(test)]
mod tests {
    use super::atomic_write;
    use tempfile::tempdir;

    #[test]
    fn atomic_write_replaces_existing_file_contents() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("vault.mdid");

        atomic_write(&path, "first version").unwrap();
        atomic_write(&path, "second version").unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "second version");
    }
}
