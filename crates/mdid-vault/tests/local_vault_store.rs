use mdid_domain::{DecodeRequest, MappingScope, SurfaceKind};
use mdid_vault::{LocalVaultStore, NewMappingRecord, PortableVaultArtifact, VaultError};
use tempfile::tempdir;
use uuid::Uuid;

fn sample_scope(field_path: &str) -> MappingScope {
    MappingScope::new(Uuid::new_v4(), Uuid::new_v4(), field_path.to_string())
}

#[test]
fn local_vault_store_encrypts_disk_state_and_can_decode_a_selected_mapping() {
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
    assert!(raw.contains("\"kdf\""));

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
    assert_eq!(decoded.values[0].record_id, stored.id);
    assert_eq!(decoded.values[0].token, stored.token);
    assert_eq!(decoded.values[0].original_value, "Alice Smith");
    assert_eq!(decoded.values[0].scope, stored.scope);
    assert_eq!(decoded.audit_event.kind.as_str(), "decode");
    assert!(decoded.audit_event.detail.contains("1 record"));
    assert!(decoded.audit_event.detail.contains(&stored.id.to_string()));

    let audit_events = reopened.audit_events();
    assert_eq!(audit_events.len(), 2);
    assert_eq!(audit_events[0].kind.as_str(), "encode");
    assert_eq!(audit_events[1].kind.as_str(), "decode");
}

#[test]
fn ensure_mapping_reuses_existing_record_for_same_scope_phi_type_and_value() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("vault.mdid");
    let mut vault = LocalVaultStore::create(&path, "correct horse battery staple").unwrap();

    let first_scope = sample_scope("rows/1/columns/0/patient_id");
    let first = vault
        .ensure_mapping(
            NewMappingRecord {
                scope: first_scope.clone(),
                phi_type: "patient_id".into(),
                original_value: "MRN-001".into(),
            },
            SurfaceKind::Cli,
        )
        .unwrap();
    let second = vault
        .ensure_mapping(
            NewMappingRecord {
                scope: first_scope,
                phi_type: "patient_id".into(),
                original_value: "MRN-001".into(),
            },
            SurfaceKind::Cli,
        )
        .unwrap();

    assert_eq!(first.id, second.id);
    assert_eq!(first.scope, second.scope);
    assert_eq!(first.token, second.token);
    assert_eq!(vault.audit_events().len(), 1);
}

#[test]
fn ensure_mapping_reuses_token_but_creates_a_new_record_for_a_new_scope() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("vault.mdid");
    let mut vault = LocalVaultStore::create(&path, "correct horse battery staple").unwrap();

    let first = vault
        .ensure_mapping(
            NewMappingRecord {
                scope: sample_scope("rows/1/columns/0/patient_id"),
                phi_type: "patient_id".into(),
                original_value: "MRN-001".into(),
            },
            SurfaceKind::Cli,
        )
        .unwrap();
    let second = vault
        .ensure_mapping(
            NewMappingRecord {
                scope: sample_scope("rows/2/columns/0/patient_id"),
                phi_type: "patient_id".into(),
                original_value: "MRN-001".into(),
            },
            SurfaceKind::Cli,
        )
        .unwrap();

    assert_ne!(first.id, second.id);
    assert_ne!(first.scope, second.scope);
    assert_eq!(first.token, second.token);
    assert_eq!(vault.audit_events().len(), 2);
}

#[test]
fn local_vault_store_debug_redacts_passphrase() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("vault.mdid");

    let vault = LocalVaultStore::create(&path, "correct horse battery staple").unwrap();
    let debug = format!("{vault:?}");

    assert!(debug.contains("LocalVaultStore"));
    assert!(!debug.contains("correct horse battery staple"));
}

#[test]
fn new_mapping_record_debug_redacts_original_value() {
    let record = NewMappingRecord {
        scope: sample_scope("patient.name"),
        phi_type: "patient_name".into(),
        original_value: "Alice Smith".into(),
    };

    let debug = format!("{record:?}");

    assert!(debug.contains("NewMappingRecord"));
    assert!(!debug.contains("Alice Smith"));
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
    let ignored = vault
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
    let artifact_json = serde_json::to_string(&artifact).unwrap();
    assert!(artifact_json.contains("\"kdf\""));
    let snapshot = artifact.unlock("portable-passphrase").unwrap();

    assert_eq!(snapshot.records.len(), 1);
    assert_eq!(snapshot.records[0].id, kept.id);
    assert_ne!(snapshot.records[0].id, ignored.id);

    let audit_events = vault.audit_events();
    assert_eq!(audit_events.len(), 3);
    assert_eq!(audit_events[2].kind.as_str(), "export");
    assert_eq!(audit_events[2].actor, SurfaceKind::Desktop);
    assert!(audit_events[2]
        .detail
        .contains("partner-site transfer package"));
    assert!(audit_events[2].detail.contains("1 record"));
    assert!(audit_events[2].detail.contains(&kept.id.to_string()));
    assert!(!audit_events[2].detail.contains(&ignored.id.to_string()));
}

#[test]
fn portable_export_rejects_blank_context() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("vault.mdid");

    let mut vault = LocalVaultStore::create(&path, "correct horse battery staple").unwrap();
    let stored = vault
        .store_mapping(
            NewMappingRecord {
                scope: sample_scope("patient.id"),
                phi_type: "patient_id".into(),
                original_value: "MRN-001".into(),
            },
            SurfaceKind::Cli,
        )
        .unwrap();

    assert!(matches!(
        vault.export_portable(
            &[stored.id],
            "portable-passphrase",
            SurfaceKind::Desktop,
            "  \n\t  ",
        ),
        Err(VaultError::BlankExportContext)
    ));
}

#[test]
fn failed_store_mapping_persistence_leaves_memory_and_disk_state_unchanged() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("vault.mdid");

    let mut vault = LocalVaultStore::create(&path, "correct horse battery staple").unwrap();
    let original = vault
        .store_mapping(
            NewMappingRecord {
                scope: sample_scope("patient.id"),
                phi_type: "patient_id".into(),
                original_value: "MRN-001".into(),
            },
            SurfaceKind::Cli,
        )
        .unwrap();
    let disk_before_failure = std::fs::read_to_string(&path).unwrap();
    let audit_before_failure = vault
        .audit_events()
        .iter()
        .map(|event| {
            (
                event.kind.as_str().to_string(),
                event.actor,
                event.detail.clone(),
            )
        })
        .collect::<Vec<_>>();

    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();

    assert!(matches!(
        vault.store_mapping(
            NewMappingRecord {
                scope: sample_scope("patient.name"),
                phi_type: "patient_name".into(),
                original_value: "Alice Smith".into(),
            },
            SurfaceKind::Desktop,
        ),
        Err(VaultError::Io(_))
    ));

    let audit_after_failure = vault
        .audit_events()
        .iter()
        .map(|event| {
            (
                event.kind.as_str().to_string(),
                event.actor,
                event.detail.clone(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(audit_after_failure, audit_before_failure);

    std::fs::remove_dir(&path).unwrap();
    std::fs::write(&path, &disk_before_failure).unwrap();

    let mut reopened = LocalVaultStore::unlock(&path, "correct horse battery staple").unwrap();
    assert_eq!(reopened.audit_events().len(), 1);
    assert!(reopened.audit_events()[0]
        .detail
        .contains("encoded mapping"));

    let artifact = reopened
        .export_portable(
            &[original.id],
            "portable-passphrase",
            SurfaceKind::Desktop,
            "recovery verification",
        )
        .unwrap();
    let snapshot = artifact.unlock("portable-passphrase").unwrap();
    assert_eq!(snapshot.records.len(), 1);
    assert_eq!(snapshot.records[0].id, original.id);

    std::fs::write(&path, &disk_before_failure).unwrap();

    let decoded = vault
        .decode(
            DecodeRequest::new(
                vec![original.id],
                "stdout".into(),
                "post-failure verification".into(),
                SurfaceKind::Desktop,
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(decoded.values.len(), 1);
    assert_eq!(decoded.values[0].record_id, original.id);
}

#[test]
fn blank_passphrases_are_rejected_for_create_unlock_and_export() {
    let dir = tempdir().unwrap();
    let blank_path = dir.path().join("blank.mdid");
    assert!(matches!(
        LocalVaultStore::create(&blank_path, " \n\t "),
        Err(VaultError::BlankPassphrase)
    ));

    let path = dir.path().join("vault.mdid");
    let mut vault = LocalVaultStore::create(&path, "correct horse battery staple").unwrap();
    let stored = vault
        .store_mapping(
            NewMappingRecord {
                scope: sample_scope("patient.id"),
                phi_type: "patient_id".into(),
                original_value: "MRN-001".into(),
            },
            SurfaceKind::Cli,
        )
        .unwrap();

    assert!(matches!(
        LocalVaultStore::unlock(&path, "   "),
        Err(VaultError::BlankPassphrase)
    ));
    assert!(matches!(
        vault.export_portable(&[stored.id], "\t\n", SurfaceKind::Cli, "partner handoff"),
        Err(VaultError::BlankPassphrase)
    ));
}

#[test]
fn wrong_passphrases_fail_cleanly_for_vault_and_portable_artifact_unlock() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("vault.mdid");

    let mut vault = LocalVaultStore::create(&path, "correct horse battery staple").unwrap();
    let stored = vault
        .store_mapping(
            NewMappingRecord {
                scope: sample_scope("patient.id"),
                phi_type: "patient_id".into(),
                original_value: "MRN-001".into(),
            },
            SurfaceKind::Cli,
        )
        .unwrap();

    let artifact = vault
        .export_portable(
            &[stored.id],
            "portable-passphrase",
            SurfaceKind::Desktop,
            "incident package",
        )
        .unwrap();

    assert!(matches!(
        LocalVaultStore::unlock(&path, "totally wrong passphrase"),
        Err(VaultError::Decrypt)
    ));
    assert!(matches!(
        artifact.unlock("totally wrong passphrase"),
        Err(VaultError::Decrypt)
    ));
}

#[test]
fn malformed_vault_nonce_returns_an_error_instead_of_panicking() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("vault.mdid");
    std::fs::write(
        &path,
        r#"{
  "salt_b64": "AAAAAAAAAAAAAAAAAAAAAA==",
  "nonce_b64": "AA==",
  "ciphertext_b64": "AA=="
}"#,
    )
    .unwrap();

    assert!(matches!(
        LocalVaultStore::unlock(&path, "correct horse battery staple"),
        Err(VaultError::InvalidNonceLength {
            expected: 12,
            actual: 1,
        })
    ));
}

#[test]
fn malformed_portable_artifact_nonce_returns_an_error_instead_of_panicking() {
    let artifact: PortableVaultArtifact = serde_json::from_str(
        r#"{
  "salt_b64": "AAAAAAAAAAAAAAAAAAAAAA==",
  "nonce_b64": "AA==",
  "ciphertext_b64": "AA=="
}"#,
    )
    .unwrap();

    assert!(matches!(
        artifact.unlock("portable-passphrase"),
        Err(VaultError::InvalidNonceLength {
            expected: 12,
            actual: 1,
        })
    ));
}

#[test]
fn create_refuses_to_overwrite_an_existing_vault_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("vault.mdid");
    let original = "existing vault contents";
    std::fs::write(&path, original).unwrap();

    assert!(matches!(
        LocalVaultStore::create(&path, "correct horse battery staple"),
        Err(VaultError::AlreadyExists(existing_path)) if existing_path == path
    ));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
}
