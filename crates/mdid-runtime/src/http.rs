use axum::{
    extract::{rejection::JsonRejection, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use mdid_adapters::DicomAdapterError;
use mdid_application::{
    ApplicationError, ApplicationService, DicomDeidentificationOutput, DicomDeidentificationService,
};
use mdid_domain::{
    DecodeRequest, DicomDeidentificationSummary, DicomPhiCandidate, DicomPrivateTagPolicy,
    SurfaceKind,
};
use mdid_vault::{LocalVaultStore, VaultError};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tempfile::tempdir;

#[derive(Clone, Default)]
pub struct RuntimeState {
    pub application: ApplicationService,
}

#[derive(Debug, Deserialize)]
struct CreatePipelineRequest {
    name: String,
}

#[derive(Debug, Deserialize)]
struct DicomDeidentifyRequest {
    dicom_bytes_base64: String,
    source_name: String,
    private_tag_policy: DicomPrivateTagPolicy,
}

#[derive(Debug, Deserialize)]
struct VaultDecodeRequest {
    vault_path: PathBuf,
    vault_passphrase: String,
    record_ids: Vec<uuid::Uuid>,
    output_target: String,
    justification: String,
    requested_by: SurfaceKind,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Debug, Serialize)]
struct DicomDeidentifyResponse {
    sanitized_file_name: String,
    rewritten_dicom_bytes_base64: String,
    summary: DicomDeidentificationSummary,
    review_queue: Vec<DicomPhiCandidate>,
}

#[derive(Debug, Serialize)]
struct VaultDecodeResponse {
    values: Vec<mdid_domain::DecodedValue>,
    audit_event: mdid_domain::AuditEvent,
}

#[derive(Debug, Serialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    code: &'static str,
    message: &'static str,
}

pub fn build_router(state: RuntimeState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/pipelines", post(create_pipeline))
        .route("/dicom/deidentify", post(dicom_deidentify))
        .route("/vault/decode", post(vault_decode))
        .with_state(state)
}

pub fn build_default_router() -> Router {
    build_router(RuntimeState::default())
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

async fn dicom_deidentify(
    Json(payload): Json<DicomDeidentifyRequest>,
) -> Response {
    let dicom_bytes = match STANDARD.decode(&payload.dicom_bytes_base64) {
        Ok(bytes) => bytes,
        Err(_) => return invalid_dicom_response().into_response(),
    };

    let temp_dir = match tempdir() {
        Ok(dir) => dir,
        Err(_) => return internal_error_response().into_response(),
    };
    let vault_path = temp_dir.path().join("runtime-dicom-vault.mdid");
    let mut vault = match LocalVaultStore::create(&vault_path, "correct horse battery staple") {
        Ok(vault) => vault,
        Err(_) => return internal_error_response().into_response(),
    };

    let output = match DicomDeidentificationService.deidentify_bytes(
        &dicom_bytes,
        &payload.source_name,
        payload.private_tag_policy,
        &mut vault,
        SurfaceKind::Browser,
    ) {
        Ok(output) => output,
        Err(error) => return map_application_error(&error).into_response(),
    };

    success_response(output).into_response()
}

async fn vault_decode(payload: Result<Json<VaultDecodeRequest>, JsonRejection>) -> Response {
    let Json(payload) = match payload {
        Ok(payload) => payload,
        Err(_) => return invalid_decode_request_response().into_response(),
    };

    let request = match DecodeRequest::new(
        payload.record_ids,
        payload.output_target,
        payload.justification,
        payload.requested_by,
    ) {
        Ok(request) => request,
        Err(_) => return invalid_decode_request_response().into_response(),
    };

    let mut vault = match LocalVaultStore::unlock(&payload.vault_path, &payload.vault_passphrase) {
        Ok(vault) => vault,
        Err(error) => return map_vault_error(&error).into_response(),
    };

    match vault.decode(request) {
        Ok(decoded) => (
            StatusCode::OK,
            Json(VaultDecodeResponse {
                values: decoded.values,
                audit_event: decoded.audit_event,
            }),
        )
            .into_response(),
        Err(error) => map_vault_error(&error).into_response(),
    }
}

fn map_application_error(error: &ApplicationError) -> (StatusCode, Json<ErrorEnvelope>) {
    match error {
        ApplicationError::DicomAdapter(DicomAdapterError::Parse(_))
        | ApplicationError::DicomAdapter(DicomAdapterError::Value(_)) => invalid_dicom_response(),
        _ => internal_error_response(),
    }
}

fn map_vault_error(error: &VaultError) -> (StatusCode, Json<ErrorEnvelope>) {
    match error {
        VaultError::UnknownRecord(_) => unknown_record_response(),
        VaultError::Decrypt => vault_unlock_failed_response(),
        VaultError::BlankPassphrase | VaultError::EmptyExportScope | VaultError::BlankExportContext => {
            invalid_decode_request_response()
        }
        VaultError::Io(_)
        | VaultError::Serde(_)
        | VaultError::UnsupportedKdfAlgorithm(_)
        | VaultError::UnsupportedKdfVersion(_)
        | VaultError::InvalidKdfParameters
        | VaultError::InvalidNonceLength { .. }
        | VaultError::KeyDerivation => invalid_vault_target_response(),
        VaultError::AlreadyExists(_) | VaultError::Encrypt => internal_error_response(),
    }
}

fn success_response(output: DicomDeidentificationOutput) -> (StatusCode, Json<DicomDeidentifyResponse>) {
    (
        StatusCode::OK,
        Json(DicomDeidentifyResponse {
            sanitized_file_name: output.sanitized_file_name,
            rewritten_dicom_bytes_base64: STANDARD.encode(output.bytes),
            summary: output.summary,
            review_queue: output.review_queue,
        }),
    )
}

fn invalid_dicom_response() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(ErrorEnvelope {
            error: ErrorBody {
                code: "invalid_dicom",
                message: "request body did not contain a valid DICOM payload",
            },
        }),
    )
}

fn invalid_decode_request_response() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(ErrorEnvelope {
            error: ErrorBody {
                code: "invalid_decode_request",
                message: "request body did not contain a valid vault decode request",
            },
        }),
    )
}

fn invalid_vault_target_response() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(ErrorEnvelope {
            error: ErrorBody {
                code: "invalid_vault_target",
                message: "vault target could not be read as a usable vault artifact",
            },
        }),
    )
}

fn unknown_record_response() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorEnvelope {
            error: ErrorBody {
                code: "unknown_record",
                message: "decode scope referenced a record that does not exist",
            },
        }),
    )
}

fn vault_unlock_failed_response() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorEnvelope {
            error: ErrorBody {
                code: "vault_unlock_failed",
                message: "vault could not be unlocked with the supplied passphrase",
            },
        }),
    )
}

fn internal_error_response() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorEnvelope {
            error: ErrorBody {
                code: "internal_error",
                message: "internal server error",
            },
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use mdid_adapters::{DicomAdapter, DicomAdapterError};
    use mdid_domain::DicomPrivateTagPolicy;
    use std::backtrace::Backtrace;

    #[test]
    fn classifies_parse_errors_as_invalid_dicom() {
        let error = ApplicationError::DicomAdapter(
            DicomAdapter::new(DicomPrivateTagPolicy::Remove)
                .extract(b"not-a-dicom-payload", "broken.dcm")
                .expect_err("garbage bytes should fail DICOM parse"),
        );

        assert_eq!(map_application_error(&error).0, StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[test]
    fn classifies_rewrite_meta_errors_as_internal_error() {
        let error = ApplicationError::DicomAdapter(invalid_meta_error().into());

        assert!(matches!(error, ApplicationError::DicomAdapter(DicomAdapterError::Meta(_))));
        assert_eq!(map_application_error(&error).0, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn classifies_rewrite_write_errors_as_internal_error() {
        let error = ApplicationError::DicomAdapter(invalid_write_error().into());

        assert!(matches!(error, ApplicationError::DicomAdapter(DicomAdapterError::Write(_))));
        assert_eq!(map_application_error(&error).0, StatusCode::INTERNAL_SERVER_ERROR);
    }

    fn invalid_meta_error() -> dicom_object::WithMetaError {
        dicom_object::WithMetaError::BuildMetaTable {
            source: dicom_object::meta::Error::MissingElement {
                alias: "Media Storage SOP Class UID",
                backtrace: Backtrace::capture(),
            },
        }
    }

    fn invalid_write_error() -> dicom_object::WriteError {
        dicom_object::WriteError::WritePreamble {
            backtrace: Backtrace::capture(),
            source: std::io::Error::other("simulated write failure"),
        }
    }
}
