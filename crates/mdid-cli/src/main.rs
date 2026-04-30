use std::{
    collections::HashSet,
    fs,
    io::Read,
    path::{Path, PathBuf},
    process,
};

use mdid_adapters::{
    ConservativeMediaInput, ConservativeMediaMetadataEntry, CsvTabularAdapter, FieldPolicy,
    FieldPolicyAction, XlsxTabularAdapter,
};
use mdid_application::{
    ConservativeMediaDeidentificationService, DicomDeidentificationService,
    PdfDeidentificationService, TabularDeidentificationService,
};
use mdid_domain::{
    AuditEvent, AuditEventKind, BatchSummary, ConservativeMediaFormat, DecodeRequest,
    DicomPrivateTagPolicy, PdfPageRef, PdfScanStatus, SurfaceKind,
};
use mdid_vault::{LocalVaultStore, PortableVaultArtifact};
use rust_xlsxwriter::Workbook;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

const DEFAULT_VAULT_AUDIT_LIMIT: usize = 100;
const MAX_VAULT_AUDIT_LIMIT: usize = 100;
/// Maximum local portable vault artifact size accepted by portable artifact commands.
const MAX_PORTABLE_ARTIFACT_BYTES: u64 = 10 * 1024 * 1024;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match parse_command(&args).and_then(run_command) {
        Ok(()) => {}
        Err(error) => exit_with_usage(&error),
    }
}

#[derive(Clone, PartialEq, Eq)]
enum CliCommand {
    Status,
    VerifyArtifacts(VerifyArtifactsArgs),
    DeidentifyCsv(DeidentifyCsvArgs),
    DeidentifyXlsx(DeidentifyXlsxArgs),
    DeidentifyDicom(DeidentifyDicomArgs),
    DeidentifyPdf(DeidentifyPdfArgs),
    ReviewMedia(ReviewMediaArgs),
    PrivacyFilterText(PrivacyFilterTextArgs),
    VaultAudit(VaultAuditArgs),
    VaultDecode(VaultDecodeArgs),
    VaultExport(VaultExportArgs),
    VaultImport(VaultImportArgs),
    VaultInspectArtifact(VaultInspectArtifactArgs),
}

#[derive(Clone, PartialEq, Eq)]
struct VerifyArtifactsArgs {
    artifact_paths_json: String,
    max_bytes: Option<u64>,
}

#[derive(Clone, PartialEq, Eq)]
struct DeidentifyCsvArgs {
    csv_path: PathBuf,
    policies_json: String,
    vault_path: PathBuf,
    passphrase: String,
    output_path: PathBuf,
}

#[derive(Clone, PartialEq, Eq)]
struct DeidentifyXlsxArgs {
    xlsx_path: PathBuf,
    policies_json: String,
    vault_path: PathBuf,
    passphrase: String,
    output_path: PathBuf,
}

#[derive(Clone, PartialEq, Eq)]
struct DeidentifyDicomArgs {
    dicom_path: PathBuf,
    private_tag_policy: String,
    vault_path: PathBuf,
    passphrase: String,
    output_path: PathBuf,
}

#[derive(Clone, PartialEq, Eq)]
struct DeidentifyPdfArgs {
    pdf_path: PathBuf,
    source_name: String,
    report_path: PathBuf,
}

#[derive(Clone, PartialEq, Eq)]
struct ReviewMediaArgs {
    artifact_label: String,
    format: ConservativeMediaFormat,
    metadata_json: String,
    requires_visual_review: bool,
    unsupported_payload: bool,
    report_path: PathBuf,
}

#[derive(Clone, PartialEq, Eq)]
struct PrivacyFilterTextArgs {
    input_path: PathBuf,
    runner_path: PathBuf,
    report_path: PathBuf,
    python_command: String,
}

const PRIVACY_FILTER_RUNNER_STDOUT_MAX_BYTES: usize = 1024 * 1024;

#[derive(Clone, PartialEq, Eq)]
struct VaultAuditArgs {
    vault_path: PathBuf,
    passphrase: String,
    limit: Option<usize>,
    offset: usize,
}

#[derive(Clone, PartialEq, Eq)]
struct VaultDecodeArgs {
    vault_path: PathBuf,
    passphrase: String,
    record_ids_json: String,
    output_target: String,
    justification: String,
    report_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VaultExportArgs {
    vault_path: PathBuf,
    passphrase: String,
    record_ids_json: String,
    export_passphrase: String,
    context: String,
    artifact_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VaultImportArgs {
    vault_path: PathBuf,
    passphrase: String,
    artifact_path: PathBuf,
    portable_passphrase: String,
    context: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VaultInspectArtifactArgs {
    artifact_path: PathBuf,
    portable_passphrase: String,
}

#[derive(Debug, Deserialize)]
struct PolicyRequest {
    header: String,
    phi_type: String,
    action: PolicyActionRequest,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum PolicyActionRequest {
    Encode,
    Review,
    Ignore,
}

#[derive(Debug, Deserialize)]
struct ConservativeMediaMetadataEntryRequest {
    key: String,
    value: String,
}

fn parse_command(args: &[String]) -> Result<CliCommand, String> {
    match args {
        [] => Ok(CliCommand::Status),
        [status] if status == "status" => Ok(CliCommand::Status),
        [command, rest @ ..] if command == "verify-artifacts" => {
            parse_verify_artifacts_args(rest).map(CliCommand::VerifyArtifacts)
        }
        [command, rest @ ..] if command == "deidentify-csv" => {
            parse_deidentify_csv_args(rest).map(CliCommand::DeidentifyCsv)
        }
        [command, rest @ ..] if command == "deidentify-xlsx" => {
            parse_deidentify_xlsx_args(rest).map(CliCommand::DeidentifyXlsx)
        }
        [command, rest @ ..] if command == "deidentify-dicom" => {
            parse_deidentify_dicom_args(rest).map(CliCommand::DeidentifyDicom)
        }
        [command, rest @ ..] if command == "deidentify-pdf" => {
            parse_deidentify_pdf_args(rest).map(CliCommand::DeidentifyPdf)
        }
        [command, rest @ ..] if command == "review-media" => {
            parse_review_media_args(rest).map(CliCommand::ReviewMedia)
        }
        [command, rest @ ..] if command == "privacy-filter-text" => {
            parse_privacy_filter_text_args(rest).map(CliCommand::PrivacyFilterText)
        }
        [command, rest @ ..] if command == "vault-audit" => {
            parse_vault_audit_args(rest).map(CliCommand::VaultAudit)
        }
        [command, rest @ ..] if command == "vault-decode" => {
            parse_vault_decode_args(rest).map(CliCommand::VaultDecode)
        }
        [command, rest @ ..] if command == "vault-export" => {
            parse_vault_export_args(rest).map(CliCommand::VaultExport)
        }
        [command, rest @ ..] if command == "vault-import" => {
            parse_vault_import_args(rest).map(CliCommand::VaultImport)
        }
        [command, rest @ ..] if command == "vault-inspect-artifact" => {
            parse_vault_inspect_artifact_args(rest).map(CliCommand::VaultInspectArtifact)
        }
        _ => Err("unknown command".to_string()),
    }
}

fn parse_verify_artifacts_args(args: &[String]) -> Result<VerifyArtifactsArgs, String> {
    let mut artifact_paths_json = None;
    let mut max_bytes = None;

    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag {
            "--artifact-paths-json" => artifact_paths_json = Some(value.clone()),
            "--max-bytes" => max_bytes = Some(parse_positive_max_bytes(value)?),
            _ => return Err("unknown flag".to_string()),
        }
        index += 2;
    }

    let artifact_paths_json =
        artifact_paths_json.ok_or_else(|| "missing --artifact-paths-json".to_string())?;
    parse_artifact_paths_json(&artifact_paths_json)?;

    Ok(VerifyArtifactsArgs {
        artifact_paths_json,
        max_bytes,
    })
}

fn parse_artifact_paths_json(artifact_paths_json: &str) -> Result<Vec<String>, String> {
    let paths: Vec<String> = serde_json::from_str(artifact_paths_json)
        .map_err(|err| format!("invalid artifact paths JSON: {err}"))?;
    if paths.is_empty() || paths.iter().any(|path| path.trim().is_empty()) {
        return Err("artifact path list must include at least one non-blank path".to_string());
    }
    Ok(paths)
}

fn parse_positive_max_bytes(max_bytes: &str) -> Result<u64, String> {
    let parsed = max_bytes
        .parse::<u64>()
        .map_err(|_| "invalid --max-bytes".to_string())?;
    if parsed == 0 {
        return Err("invalid --max-bytes".to_string());
    }
    Ok(parsed)
}

fn parse_deidentify_csv_args(args: &[String]) -> Result<DeidentifyCsvArgs, String> {
    let mut csv_path = None;
    let mut policies_json = None;
    let mut vault_path = None;
    let mut passphrase = None;
    let mut output_path = None;

    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag {
            "--csv-path" => csv_path = Some(PathBuf::from(value)),
            "--policies-json" => policies_json = Some(value.clone()),
            "--vault-path" => vault_path = Some(PathBuf::from(value)),
            "--passphrase" => passphrase = Some(value.clone()),
            "--output-path" => output_path = Some(PathBuf::from(value)),
            _ => return Err("unknown flag".to_string()),
        }
        index += 2;
    }

    Ok(DeidentifyCsvArgs {
        csv_path: csv_path.ok_or_else(|| "missing --csv-path".to_string())?,
        policies_json: policies_json.ok_or_else(|| "missing --policies-json".to_string())?,
        vault_path: vault_path.ok_or_else(|| "missing --vault-path".to_string())?,
        passphrase: passphrase.ok_or_else(|| "missing --passphrase".to_string())?,
        output_path: output_path.ok_or_else(|| "missing --output-path".to_string())?,
    })
}

fn parse_deidentify_xlsx_args(args: &[String]) -> Result<DeidentifyXlsxArgs, String> {
    let mut xlsx_path = None;
    let mut policies_json = None;
    let mut vault_path = None;
    let mut passphrase = None;
    let mut output_path = None;

    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag {
            "--xlsx-path" => xlsx_path = Some(PathBuf::from(value)),
            "--policies-json" => policies_json = Some(value.clone()),
            "--vault-path" => vault_path = Some(PathBuf::from(value)),
            "--passphrase" => passphrase = Some(value.clone()),
            "--output-path" => output_path = Some(PathBuf::from(value)),
            _ => return Err("unknown flag".to_string()),
        }
        index += 2;
    }

    Ok(DeidentifyXlsxArgs {
        xlsx_path: xlsx_path.ok_or_else(|| "missing --xlsx-path".to_string())?,
        policies_json: policies_json.ok_or_else(|| "missing --policies-json".to_string())?,
        vault_path: vault_path.ok_or_else(|| "missing --vault-path".to_string())?,
        passphrase: passphrase.ok_or_else(|| "missing --passphrase".to_string())?,
        output_path: output_path.ok_or_else(|| "missing --output-path".to_string())?,
    })
}

fn parse_deidentify_dicom_args(args: &[String]) -> Result<DeidentifyDicomArgs, String> {
    let mut dicom_path = None;
    let mut private_tag_policy = None;
    let mut vault_path = None;
    let mut passphrase = None;
    let mut output_path = None;

    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag {
            "--dicom-path" => dicom_path = Some(PathBuf::from(value)),
            "--private-tag-policy" => private_tag_policy = Some(value.clone()),
            "--vault-path" => vault_path = Some(PathBuf::from(value)),
            "--passphrase" => passphrase = Some(value.clone()),
            "--output-path" => output_path = Some(PathBuf::from(value)),
            _ => return Err("unknown flag".to_string()),
        }
        index += 2;
    }

    Ok(DeidentifyDicomArgs {
        dicom_path: dicom_path.ok_or_else(|| "missing --dicom-path".to_string())?,
        private_tag_policy: private_tag_policy
            .ok_or_else(|| "missing --private-tag-policy".to_string())?,
        vault_path: vault_path.ok_or_else(|| "missing --vault-path".to_string())?,
        passphrase: passphrase.ok_or_else(|| "missing --passphrase".to_string())?,
        output_path: output_path.ok_or_else(|| "missing --output-path".to_string())?,
    })
}

fn parse_deidentify_pdf_args(args: &[String]) -> Result<DeidentifyPdfArgs, String> {
    let mut pdf_path = None;
    let mut source_name = None;
    let mut report_path = None;

    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag {
            "--pdf-path" => pdf_path = Some(PathBuf::from(value)),
            "--source-name" => source_name = Some(value.clone()),
            "--report-path" => report_path = Some(PathBuf::from(value)),
            _ => return Err("unknown flag".to_string()),
        }
        index += 2;
    }

    let source_name = source_name.ok_or_else(|| "missing --source-name".to_string())?;
    if source_name.trim().is_empty() {
        return Err("missing --source-name".to_string());
    }

    Ok(DeidentifyPdfArgs {
        pdf_path: pdf_path.ok_or_else(|| "missing --pdf-path".to_string())?,
        source_name,
        report_path: report_path.ok_or_else(|| "missing --report-path".to_string())?,
    })
}

fn parse_review_media_args(args: &[String]) -> Result<ReviewMediaArgs, String> {
    let mut artifact_label = None;
    let mut format = None;
    let mut metadata_json = None;
    let mut requires_visual_review = None;
    let mut unsupported_payload = None;
    let mut report_path = None;

    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag {
            "--artifact-label" => artifact_label = Some(value.clone()),
            "--format" => format = Some(parse_conservative_media_format(value)?),
            "--metadata-json" => metadata_json = Some(value.clone()),
            "--requires-visual-review" => requires_visual_review = Some(parse_bool_flag(value)?),
            "--unsupported-payload" => unsupported_payload = Some(parse_bool_flag(value)?),
            "--report-path" => report_path = Some(PathBuf::from(value)),
            _ => return Err("unknown flag".to_string()),
        }
        index += 2;
    }

    let artifact_label = artifact_label.ok_or_else(|| "missing --artifact-label".to_string())?;
    if artifact_label.trim().is_empty() {
        return Err("missing --artifact-label".to_string());
    }

    Ok(ReviewMediaArgs {
        artifact_label,
        format: format.ok_or_else(|| "missing --format".to_string())?,
        metadata_json: metadata_json.ok_or_else(|| "missing --metadata-json".to_string())?,
        requires_visual_review: requires_visual_review
            .ok_or_else(|| "missing --requires-visual-review".to_string())?,
        unsupported_payload: unsupported_payload
            .ok_or_else(|| "missing --unsupported-payload".to_string())?,
        report_path: report_path.ok_or_else(|| "missing --report-path".to_string())?,
    })
}

fn parse_privacy_filter_text_args(args: &[String]) -> Result<PrivacyFilterTextArgs, String> {
    let mut input_path = None;
    let mut runner_path = None;
    let mut report_path = None;
    let mut python_command = None;

    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag {
            "--input-path" => input_path = Some(non_blank_path(value, "--input-path")?),
            "--runner-path" => runner_path = Some(non_blank_path(value, "--runner-path")?),
            "--report-path" => report_path = Some(non_blank_path(value, "--report-path")?),
            "--python-command" => {
                if value.trim().is_empty() {
                    return Err("missing --python-command".to_string());
                }
                python_command = Some(value.clone());
            }
            _ => return Err("unknown flag".to_string()),
        }
        index += 2;
    }

    Ok(PrivacyFilterTextArgs {
        input_path: input_path.ok_or_else(|| "missing --input-path".to_string())?,
        runner_path: runner_path.ok_or_else(|| "missing --runner-path".to_string())?,
        report_path: report_path.ok_or_else(|| "missing --report-path".to_string())?,
        python_command: python_command.unwrap_or_else(default_python_command),
    })
}

fn default_python_command() -> String {
    if cfg!(windows) {
        "py".to_string()
    } else {
        "python3".to_string()
    }
}

fn non_blank_path(value: &str, flag: &str) -> Result<PathBuf, String> {
    if value.trim().is_empty() {
        return Err(format!("missing {flag}"));
    }
    Ok(PathBuf::from(value))
}

fn parse_conservative_media_format(value: &str) -> Result<ConservativeMediaFormat, String> {
    match value {
        "image" => Ok(ConservativeMediaFormat::Image),
        "video" => Ok(ConservativeMediaFormat::Video),
        "fcs" => Ok(ConservativeMediaFormat::Fcs),
        _ => Err("invalid --format".to_string()),
    }
}

fn parse_bool_flag(value: &str) -> Result<bool, String> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err("invalid boolean flag".to_string()),
    }
}

fn parse_vault_audit_args(args: &[String]) -> Result<VaultAuditArgs, String> {
    let mut vault_path = None;
    let mut passphrase = None;
    let mut limit = None;
    let mut offset = 0;

    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag {
            "--vault-path" => vault_path = Some(PathBuf::from(value)),
            "--passphrase" => passphrase = Some(value.clone()),
            "--limit" => {
                let parsed = value
                    .parse::<usize>()
                    .map_err(|_| "invalid --limit".to_string())?;
                if parsed == 0 {
                    return Err("invalid --limit".to_string());
                }
                limit = Some(parsed);
            }
            "--offset" => {
                offset = value
                    .parse::<usize>()
                    .map_err(|_| "invalid --offset".to_string())?;
            }
            _ => return Err("unknown flag".to_string()),
        }
        index += 2;
    }

    Ok(VaultAuditArgs {
        vault_path: vault_path.ok_or_else(|| "missing --vault-path".to_string())?,
        passphrase: passphrase.ok_or_else(|| "missing --passphrase".to_string())?,
        limit,
        offset,
    })
}

fn parse_vault_decode_args(args: &[String]) -> Result<VaultDecodeArgs, String> {
    let mut vault_path = None;
    let mut passphrase = None;
    let mut record_ids_json = None;
    let mut output_target = None;
    let mut justification = None;
    let mut report_path = None;

    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag {
            "--vault-path" => vault_path = Some(PathBuf::from(value)),
            "--passphrase" => passphrase = Some(value.clone()),
            "--record-ids-json" => record_ids_json = Some(value.clone()),
            "--output-target" => output_target = Some(value.clone()),
            "--justification" => justification = Some(value.clone()),
            "--report-path" => report_path = Some(PathBuf::from(value)),
            _ => return Err("unknown flag".to_string()),
        }
        index += 2;
    }

    let output_target = output_target.ok_or_else(|| "missing --output-target".to_string())?;
    if output_target.trim().is_empty() {
        return Err("missing --output-target".to_string());
    }
    let justification = justification.ok_or_else(|| "missing --justification".to_string())?;
    if justification.trim().is_empty() {
        return Err("missing --justification".to_string());
    }

    Ok(VaultDecodeArgs {
        vault_path: vault_path.ok_or_else(|| "missing --vault-path".to_string())?,
        passphrase: passphrase.ok_or_else(|| "missing --passphrase".to_string())?,
        record_ids_json: record_ids_json.ok_or_else(|| "missing --record-ids-json".to_string())?,
        output_target,
        justification,
        report_path: report_path.ok_or_else(|| "missing --report-path".to_string())?,
    })
}

fn parse_vault_export_args(args: &[String]) -> Result<VaultExportArgs, String> {
    let mut vault_path = None;
    let mut passphrase = None;
    let mut record_ids_json = None;
    let mut export_passphrase = None;
    let mut context = None;
    let mut artifact_path = None;

    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag {
            "--vault-path" => vault_path = Some(PathBuf::from(value)),
            "--passphrase" => passphrase = Some(value.clone()),
            "--record-ids-json" => record_ids_json = Some(value.clone()),
            "--export-passphrase" => export_passphrase = Some(value.clone()),
            "--context" => context = Some(value.clone()),
            "--artifact-path" => artifact_path = Some(PathBuf::from(value)),
            _ => return Err("unknown flag".to_string()),
        }
        index += 2;
    }

    let context = context.ok_or_else(|| "missing --context".to_string())?;
    if context.trim().is_empty() {
        return Err("missing --context".to_string());
    }

    Ok(VaultExportArgs {
        vault_path: vault_path.ok_or_else(|| "missing --vault-path".to_string())?,
        passphrase: passphrase.ok_or_else(|| "missing --passphrase".to_string())?,
        record_ids_json: record_ids_json.ok_or_else(|| "missing --record-ids-json".to_string())?,
        export_passphrase: export_passphrase
            .ok_or_else(|| "missing --export-passphrase".to_string())?,
        context,
        artifact_path: artifact_path.ok_or_else(|| "missing --artifact-path".to_string())?,
    })
}

fn parse_vault_import_args(args: &[String]) -> Result<VaultImportArgs, String> {
    let mut vault_path = None;
    let mut passphrase = None;
    let mut artifact_path = None;
    let mut portable_passphrase = None;
    let mut context = None;

    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag {
            "--vault-path" => vault_path = Some(PathBuf::from(value)),
            "--passphrase" => passphrase = Some(value.clone()),
            "--artifact-path" => artifact_path = Some(PathBuf::from(value)),
            "--portable-passphrase" => portable_passphrase = Some(value.clone()),
            "--context" => context = Some(value.clone()),
            _ => return Err("unknown flag".to_string()),
        }
        index += 2;
    }

    let context = context.ok_or_else(|| "missing --context".to_string())?;
    if context.trim().is_empty() {
        return Err("missing --context".to_string());
    }

    Ok(VaultImportArgs {
        vault_path: vault_path.ok_or_else(|| "missing --vault-path".to_string())?,
        passphrase: passphrase.ok_or_else(|| "missing --passphrase".to_string())?,
        artifact_path: artifact_path.ok_or_else(|| "missing --artifact-path".to_string())?,
        portable_passphrase: portable_passphrase
            .ok_or_else(|| "missing --portable-passphrase".to_string())?,
        context,
    })
}

fn parse_vault_inspect_artifact_args(args: &[String]) -> Result<VaultInspectArtifactArgs, String> {
    let mut artifact_path = None;
    let mut portable_passphrase = None;

    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag {
            "--artifact-path" => artifact_path = Some(PathBuf::from(value)),
            "--portable-passphrase" => portable_passphrase = Some(value.clone()),
            _ => return Err("unknown flag".to_string()),
        }
        index += 2;
    }

    Ok(VaultInspectArtifactArgs {
        artifact_path: artifact_path.ok_or_else(|| "missing --artifact-path".to_string())?,
        portable_passphrase: portable_passphrase
            .ok_or_else(|| "missing --portable-passphrase".to_string())?,
    })
}

fn run_command(command: CliCommand) -> Result<(), String> {
    match command {
        CliCommand::Status => {
            println!("med-de-id CLI ready");
            Ok(())
        }
        CliCommand::VerifyArtifacts(args) => run_verify_artifacts(args),
        CliCommand::DeidentifyCsv(args) => run_deidentify_csv(args),
        CliCommand::DeidentifyXlsx(args) => run_deidentify_xlsx(args),
        CliCommand::DeidentifyDicom(args) => run_deidentify_dicom(args),
        CliCommand::DeidentifyPdf(args) => run_deidentify_pdf(args),
        CliCommand::ReviewMedia(args) => run_review_media(args),
        CliCommand::PrivacyFilterText(args) => run_privacy_filter_text(args),
        CliCommand::VaultAudit(args) => run_vault_audit(args),
        CliCommand::VaultDecode(args) => run_vault_decode(args),
        CliCommand::VaultExport(args) => run_vault_export(args),
        CliCommand::VaultImport(args) => run_vault_import(args),
        CliCommand::VaultInspectArtifact(args) => run_vault_inspect_artifact(args),
    }
}

fn run_privacy_filter_text(args: PrivacyFilterTextArgs) -> Result<(), String> {
    require_regular_file(&args.input_path, "missing input file")?;
    require_regular_file(&args.runner_path, "missing runner file")?;

    let mut child = std::process::Command::new(&args.python_command)
        .arg(&args.runner_path)
        .arg(&args.input_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|err| format!("failed to run privacy filter runner: {err}"))?;

    let mut stdout_bytes = Vec::new();
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "failed to read privacy filter runner output".to_string())?;
    stdout
        .take((PRIVACY_FILTER_RUNNER_STDOUT_MAX_BYTES + 1) as u64)
        .read_to_end(&mut stdout_bytes)
        .map_err(|err| format!("failed to read privacy filter runner output: {err}"))?;

    if stdout_bytes.len() > PRIVACY_FILTER_RUNNER_STDOUT_MAX_BYTES {
        let _ = child.kill();
        let _ = child.wait();
        return Err("runner output exceeded limit".to_string());
    }

    let status = child
        .wait()
        .map_err(|err| format!("failed to wait for privacy filter runner: {err}"))?;
    if !status.success() {
        return Err("privacy filter runner failed".to_string());
    }

    let stdout = String::from_utf8(stdout_bytes)
        .map_err(|_| "runner returned non-UTF-8 output".to_string())?;
    let value: Value =
        serde_json::from_str(&stdout).map_err(|_| "runner returned non-JSON output".to_string())?;
    validate_privacy_filter_output(&value)?;
    fs::write(&args.report_path, stdout)
        .map_err(|err| format!("failed to write privacy filter report: {err}"))?;

    let summary = json!({
        "command": "privacy-filter-text",
        "report_path": args.report_path,
    });
    println!(
        "{}",
        serde_json::to_string(&summary)
            .map_err(|err| format!("failed to render summary: {err}"))?
    );
    Ok(())
}

fn require_regular_file(path: &Path, message: &str) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => Ok(()),
        Ok(_) => Err(message.to_string()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Err(message.to_string()),
        Err(error) => Err(format!("failed to inspect privacy filter path: {error}")),
    }
}

fn validate_privacy_filter_output(value: &Value) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| "privacy filter output must be a JSON object".to_string())?;
    for key in ["summary", "masked_text", "spans", "metadata"] {
        if !object.contains_key(key) {
            return Err("privacy filter output missing required field".to_string());
        }
    }
    if !value["summary"].is_object()
        || !value["masked_text"].is_string()
        || !value["spans"].is_array()
        || !value["metadata"].is_object()
    {
        return Err("privacy filter output has invalid required field shape".to_string());
    }
    if let Some(called) = value["metadata"].get("network_api_called") {
        if called != false {
            return Err("privacy filter output indicates network API use".to_string());
        }
    }
    Ok(())
}

#[derive(Debug, Serialize)]
struct VerifyArtifactsReport {
    artifact_count: usize,
    existing_count: usize,
    missing_count: usize,
    oversized_count: usize,
    max_bytes: Option<u64>,
    artifacts: Vec<VerifyArtifactEntryReport>,
}

#[derive(Debug, Serialize)]
struct VerifyArtifactEntryReport {
    index: usize,
    exists: bool,
    byte_len: Option<u64>,
    within_max_bytes: Option<bool>,
}

fn run_verify_artifacts(args: VerifyArtifactsArgs) -> Result<(), String> {
    let paths = parse_artifact_paths_json(&args.artifact_paths_json)?;
    let report = build_verify_artifacts_report(&paths, args.max_bytes)?;
    let missing_count = report.missing_count;
    let oversized_count = report.oversized_count;
    println!(
        "{}",
        serde_json::to_string(&report)
            .map_err(|err| format!("failed to render artifact verification report: {err}"))?
    );
    if missing_count > 0 || oversized_count > 0 {
        return Err(format!(
            "artifact verification failed: {missing_count} missing, {oversized_count} oversized"
        ));
    }
    Ok(())
}

fn build_verify_artifacts_report(
    paths: &[String],
    max_bytes: Option<u64>,
) -> Result<VerifyArtifactsReport, String> {
    if paths.is_empty() || paths.iter().any(|path| path.trim().is_empty()) {
        return Err("artifact path list must include at least one non-blank path".to_string());
    }

    let mut seen_paths = std::collections::HashSet::with_capacity(paths.len());
    for path in paths {
        if !seen_paths.insert(path.trim()) {
            return Err("artifact path list must not contain duplicate paths".to_string());
        }
    }

    let mut existing_count = 0;
    let mut missing_count = 0;
    let mut oversized_count = 0;
    let mut artifacts = Vec::with_capacity(paths.len());

    for (index, path) in paths.iter().enumerate() {
        match fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_file() => {
                existing_count += 1;
                let byte_len = metadata.len();
                let within_max_bytes = max_bytes.map(|limit| byte_len <= limit);
                if within_max_bytes == Some(false) {
                    oversized_count += 1;
                }
                artifacts.push(VerifyArtifactEntryReport {
                    index,
                    exists: true,
                    byte_len: Some(byte_len),
                    within_max_bytes,
                });
            }
            Ok(_) => {
                missing_count += 1;
                artifacts.push(VerifyArtifactEntryReport {
                    index,
                    exists: false,
                    byte_len: None,
                    within_max_bytes: None,
                });
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                missing_count += 1;
                artifacts.push(VerifyArtifactEntryReport {
                    index,
                    exists: false,
                    byte_len: None,
                    within_max_bytes: max_bytes.map(|_| false),
                });
            }
            Err(error) => return Err(format!("failed to inspect artifact metadata: {error}")),
        }
    }

    Ok(VerifyArtifactsReport {
        artifact_count: paths.len(),
        existing_count,
        missing_count,
        oversized_count,
        max_bytes,
        artifacts,
    })
}

#[derive(Debug, Serialize)]
struct PdfPageStatusReport {
    page: PdfPageRef,
    status: PdfScanStatus,
}

#[derive(Debug, Serialize)]
struct ConservativeMediaCandidateReport {
    candidate_index: usize,
    format: ConservativeMediaFormat,
    phi_type: String,
    confidence: f32,
    status: mdid_domain::ConservativeMediaScanStatus,
}

fn run_review_media(args: ReviewMediaArgs) -> Result<(), String> {
    let metadata: Vec<ConservativeMediaMetadataEntryRequest> =
        serde_json::from_str(&args.metadata_json)
            .map_err(|err| format!("invalid metadata JSON: {err}"))?;
    let metadata = metadata
        .into_iter()
        .map(|entry| ConservativeMediaMetadataEntry {
            key: entry.key,
            value: entry.value,
        })
        .collect();

    let output = ConservativeMediaDeidentificationService::default()
        .deidentify_metadata(ConservativeMediaInput {
            artifact_label: args.artifact_label,
            format: args.format,
            metadata,
            requires_visual_review: args.requires_visual_review,
            unsupported_payload: args.unsupported_payload,
        })
        .map_err(|err| format!("failed to review media: {err}"))?;

    let review_queue_len = output.review_queue.len();
    let review_queue: Vec<ConservativeMediaCandidateReport> = output
        .review_queue
        .into_iter()
        .enumerate()
        .map(
            |(candidate_index, candidate)| ConservativeMediaCandidateReport {
                candidate_index,
                format: candidate.format,
                phi_type: candidate.phi_type,
                confidence: candidate.confidence,
                status: candidate.status,
            },
        )
        .collect();
    let report = json!({
        "summary": output.summary,
        "review_queue_len": review_queue_len,
        "rewritten_media_bytes": serde_json::Value::Null,
        "review_queue": review_queue,
    });

    fs::write(
        &args.report_path,
        serde_json::to_string_pretty(&report)
            .map_err(|err| format!("failed to render media report: {err}"))?,
    )
    .map_err(|err| format!("failed to write media report: {err}"))?;

    let stdout = json!({
        "report_path": args.report_path,
        "summary": report["summary"].clone(),
        "review_queue_len": report["review_queue_len"].clone(),
        "rewritten_media_bytes": serde_json::Value::Null,
    });
    println!(
        "{}",
        serde_json::to_string(&stdout).map_err(|err| format!("failed to render summary: {err}"))?
    );
    Ok(())
}

fn run_deidentify_pdf(args: DeidentifyPdfArgs) -> Result<(), String> {
    let bytes = fs::read(&args.pdf_path).map_err(|err| format!("failed to read PDF: {err}"))?;
    let output = PdfDeidentificationService
        .deidentify_bytes(&bytes, args.source_name.trim())
        .map_err(|err| format!("failed to review PDF: {err}"))?;

    let review_queue_len = output.review_queue.len();
    let page_statuses: Vec<PdfPageStatusReport> = output
        .page_statuses
        .into_iter()
        .map(|page_status| PdfPageStatusReport {
            page: page_status.page,
            status: page_status.status,
        })
        .collect();
    let report = json!({
        "summary": output.summary,
        "page_statuses": page_statuses,
        "review_queue_len": review_queue_len,
        "rewrite_available": false,
        "rewrite_status": output.rewrite_status,
        "no_rewritten_pdf": output.no_rewritten_pdf,
        "review_only": output.review_only,
        "rewritten_pdf_bytes": serde_json::Value::Null,
    });
    fs::write(
        &args.report_path,
        serde_json::to_string_pretty(&report)
            .map_err(|err| format!("failed to render PDF report: {err}"))?,
    )
    .map_err(|err| format!("failed to write PDF report: {err}"))?;

    let stdout = json!({
        "report_path": args.report_path,
        "summary": report["summary"].clone(),
        "review_queue_len": report["review_queue_len"].clone(),
        "rewrite_available": false,
        "rewrite_status": report["rewrite_status"].clone(),
        "no_rewritten_pdf": report["no_rewritten_pdf"].clone(),
        "review_only": report["review_only"].clone(),
    });
    println!(
        "{}",
        serde_json::to_string(&stdout).map_err(|err| format!("failed to render summary: {err}"))?
    );
    Ok(())
}

#[derive(Debug, Serialize)]
struct VaultAuditReport {
    event_count: usize,
    total_matching_events: usize,
    returned_event_count: usize,
    limit: usize,
    offset: usize,
    next_offset: Option<usize>,
    has_more: bool,
    events: Vec<VaultAuditEventReport>,
}

#[derive(Debug, Serialize)]
struct VaultAuditEventReport {
    id: String,
    kind: String,
    actor: SurfaceKind,
    detail: String,
    recorded_at: String,
}

#[derive(Debug, Serialize)]
struct VaultDecodeReport {
    decoded_value_count: usize,
    values: Vec<VaultDecodeValueReport>,
    audit_event: AuditEvent,
}

#[derive(Debug, Serialize)]
struct VaultDecodeValueReport {
    record_id: String,
    token: String,
    original_value: String,
}

fn parse_record_ids_json(record_ids_json: &str) -> Result<Vec<Uuid>, String> {
    let record_ids: Vec<Uuid> = serde_json::from_str(record_ids_json)
        .map_err(|err| format!("invalid record ids JSON: {err}"))?;
    if record_ids.is_empty() {
        return Err("decode scope must include at least one record id".to_string());
    }
    let mut seen_record_ids = HashSet::with_capacity(record_ids.len());
    for record_id in &record_ids {
        if !seen_record_ids.insert(*record_id) {
            return Err("duplicate record id is not allowed".to_string());
        }
    }
    Ok(record_ids)
}

fn run_vault_export(args: VaultExportArgs) -> Result<(), String> {
    println!("{}", run_vault_export_for_summary(args)?);
    Ok(())
}

fn run_vault_import(args: VaultImportArgs) -> Result<(), String> {
    println!("{}", run_vault_import_for_summary(args)?);
    Ok(())
}

fn run_vault_inspect_artifact(args: VaultInspectArtifactArgs) -> Result<(), String> {
    println!("{}", run_vault_inspect_artifact_for_summary(args)?);
    Ok(())
}

fn read_bounded_portable_artifact(path: &Path) -> Result<Vec<u8>, String> {
    let file = fs::File::open(path)
        .map_err(|err| format!("failed to read vault import artifact: {err}"))?;
    let mut reader = file.take(MAX_PORTABLE_ARTIFACT_BYTES + 1);
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|err| format!("failed to read vault import artifact: {err}"))?;
    if bytes.len() as u64 > MAX_PORTABLE_ARTIFACT_BYTES {
        return Err("portable artifact exceeds maximum size".to_string());
    }
    Ok(bytes)
}

fn run_vault_inspect_artifact_for_summary(
    args: VaultInspectArtifactArgs,
) -> Result<String, String> {
    let artifact_bytes = read_bounded_portable_artifact(&args.artifact_path)?;
    let artifact: PortableVaultArtifact = serde_json::from_slice(&artifact_bytes)
        .map_err(|err| format!("failed to parse portable artifact: {err}"))?;
    let snapshot = artifact
        .unlock(&args.portable_passphrase)
        .map_err(|err| format!("failed to inspect portable artifact: {err}"))?;
    let stdout = json!({
        "command": "vault-inspect-artifact",
        "record_count": snapshot.records.len(),
    });
    serde_json::to_string(&stdout)
        .map_err(|err| format!("failed to render portable inspect summary: {err}"))
}

fn run_vault_import_for_summary(args: VaultImportArgs) -> Result<String, String> {
    let artifact_bytes = read_bounded_portable_artifact(&args.artifact_path)?;
    let artifact: PortableVaultArtifact = serde_json::from_slice(&artifact_bytes)
        .map_err(|err| format!("failed to parse vault import artifact: {err}"))?;
    let mut vault = LocalVaultStore::unlock(&args.vault_path, &args.passphrase)
        .map_err(|err| format!("failed to open vault: {err}"))?;
    let result = vault
        .import_portable(
            artifact,
            &args.portable_passphrase,
            SurfaceKind::Cli,
            &args.context,
        )
        .map_err(|err| format!("failed to import vault records: {err}"))?;
    let stdout = json!({
        "command": "vault-import",
        "imported_records": result.imported_records.len(),
        "duplicate_records": result.duplicate_records.len(),
        "audit_event_id": result.audit_event.id.to_string(),
    });
    serde_json::to_string(&stdout).map_err(|err| format!("failed to render import summary: {err}"))
}

fn run_vault_export_for_summary(args: VaultExportArgs) -> Result<String, String> {
    let record_ids = parse_record_ids_json(&args.record_ids_json)?;
    let mut vault = LocalVaultStore::unlock(&args.vault_path, &args.passphrase)
        .map_err(|err| format!("failed to open vault: {err}"))?;
    let artifact = vault
        .export_portable(
            &record_ids,
            &args.export_passphrase,
            SurfaceKind::Cli,
            &args.context,
        )
        .map_err(|err| format!("failed to export vault records: {err}"))?;
    fs::write(
        &args.artifact_path,
        serde_json::to_vec_pretty(&artifact)
            .map_err(|err| format!("failed to render vault export artifact: {err}"))?,
    )
    .map_err(|err| format!("failed to write vault export artifact: {err}"))?;
    let audit_event_id = vault
        .audit_events()
        .last()
        .map(|event| event.id.to_string())
        .ok_or_else(|| "missing vault export audit event".to_string())?;
    let stdout = json!({
        "command": "vault-export",
        "exported_records": record_ids.len(),
        "artifact_path": args.artifact_path,
        "audit_event_id": audit_event_id,
    });
    serde_json::to_string(&stdout).map_err(|err| format!("failed to render export summary: {err}"))
}

fn run_vault_decode(args: VaultDecodeArgs) -> Result<(), String> {
    let record_ids = parse_record_ids_json(&args.record_ids_json)?;
    let request = DecodeRequest::new(
        record_ids,
        args.output_target,
        args.justification,
        SurfaceKind::Cli,
    )
    .map_err(|err| err.to_string())?;
    let mut vault = LocalVaultStore::unlock(&args.vault_path, &args.passphrase)
        .map_err(|err| format!("failed to open vault: {err}"))?;
    let result = vault
        .decode(request)
        .map_err(|err| format!("failed to decode vault records: {err}"))?;
    let report = VaultDecodeReport {
        decoded_value_count: result.values.len(),
        values: result
            .values
            .into_iter()
            .map(|value| VaultDecodeValueReport {
                record_id: value.record_id.to_string(),
                token: value.token,
                original_value: value.original_value,
            })
            .collect(),
        audit_event: result.audit_event,
    };
    fs::write(
        &args.report_path,
        serde_json::to_string_pretty(&report)
            .map_err(|err| format!("failed to render vault decode report: {err}"))?,
    )
    .map_err(|err| format!("failed to write vault decode report: {err}"))?;
    println!("{}", build_vault_decode_stdout(&args.report_path, &report)?);
    Ok(())
}

fn build_vault_decode_stdout(
    report_path: &PathBuf,
    report: &VaultDecodeReport,
) -> Result<String, String> {
    let audit_event = vault_audit_event_report(&report.audit_event);
    let stdout = json!({
        "report_path": report_path,
        "decoded_value_count": report.decoded_value_count,
        "audit_event": audit_event,
    });
    serde_json::to_string(&stdout).map_err(|err| format!("failed to render decode summary: {err}"))
}

fn run_vault_audit(args: VaultAuditArgs) -> Result<(), String> {
    let vault = LocalVaultStore::unlock(&args.vault_path, &args.passphrase)
        .map_err(|err| format!("failed to open vault: {err}"))?;
    let report = build_vault_audit_report(vault.audit_events(), args.limit, args.offset);
    println!(
        "{}",
        serde_json::to_string(&report)
            .map_err(|err| format!("failed to render audit report: {err}"))?
    );
    Ok(())
}

fn build_vault_audit_report(
    events: &[AuditEvent],
    limit: Option<usize>,
    offset: usize,
) -> VaultAuditReport {
    let event_count = events.len();
    let limit = limit
        .unwrap_or(DEFAULT_VAULT_AUDIT_LIMIT)
        .min(MAX_VAULT_AUDIT_LIMIT);
    let mut selected = events
        .iter()
        .rev()
        .skip(offset)
        .take(limit.saturating_add(1))
        .map(vault_audit_event_report)
        .collect::<Vec<_>>();
    let has_more = selected.len() > limit;
    if has_more {
        selected.truncate(limit);
    }
    VaultAuditReport {
        event_count,
        total_matching_events: event_count,
        returned_event_count: selected.len(),
        limit,
        offset,
        next_offset: has_more.then_some(offset.saturating_add(limit)),
        has_more,
        events: selected,
    }
}

fn vault_audit_event_report(event: &AuditEvent) -> VaultAuditEventReport {
    VaultAuditEventReport {
        id: event.id.to_string(),
        kind: event.kind.as_str().to_string(),
        actor: event.actor,
        detail: sanitized_audit_detail(event),
        recorded_at: event.recorded_at.to_rfc3339(),
    }
}

fn sanitized_audit_detail(event: &AuditEvent) -> String {
    match event.kind {
        AuditEventKind::Encode => "encoded mapping".to_string(),
        AuditEventKind::Decode => "decode event".to_string(),
        AuditEventKind::Export => "portable export event".to_string(),
        AuditEventKind::Import => "portable import event".to_string(),
    }
}

fn parse_private_tag_policy(policy: &str) -> Result<DicomPrivateTagPolicy, String> {
    match policy {
        "remove" => Ok(DicomPrivateTagPolicy::Remove),
        "review" | "review-required" | "required" => Ok(DicomPrivateTagPolicy::ReviewRequired),
        "keep" => Ok(DicomPrivateTagPolicy::Keep),
        _ => Err("invalid --private-tag-policy".to_string()),
    }
}

fn run_deidentify_dicom(args: DeidentifyDicomArgs) -> Result<(), String> {
    let policy = parse_private_tag_policy(&args.private_tag_policy)?;
    let bytes = fs::read(&args.dicom_path).map_err(|err| format!("failed to read DICOM: {err}"))?;
    let mut vault = if args.vault_path.exists() {
        LocalVaultStore::unlock(&args.vault_path, &args.passphrase)
    } else {
        LocalVaultStore::create(&args.vault_path, &args.passphrase)
    }
    .map_err(|err| format!("failed to open vault: {err}"))?;

    let output = DicomDeidentificationService
        .deidentify_bytes(
            &bytes,
            "dicom-input.dcm",
            policy,
            &mut vault,
            SurfaceKind::Cli,
        )
        .map_err(|err| format!("failed to deidentify DICOM: {err}"))?;
    fs::write(&args.output_path, &output.bytes)
        .map_err(|err| format!("failed to write output DICOM: {err}"))?;

    let payload = json!({
        "output_path": args.output_path,
        "sanitized_file_name": output.sanitized_file_name,
        "summary": output.summary,
        "review_queue_len": output.review_queue.len(),
    });
    println!(
        "{}",
        serde_json::to_string(&payload)
            .map_err(|err| format!("failed to render summary: {err}"))?
    );
    Ok(())
}

fn run_deidentify_csv(args: DeidentifyCsvArgs) -> Result<(), String> {
    let policies = parse_policies(&args.policies_json)?;
    let csv =
        fs::read_to_string(&args.csv_path).map_err(|err| format!("failed to read CSV: {err}"))?;
    let mut vault = if args.vault_path.exists() {
        LocalVaultStore::unlock(&args.vault_path, &args.passphrase)
    } else {
        LocalVaultStore::create(&args.vault_path, &args.passphrase)
    }
    .map_err(|err| format!("failed to open vault: {err}"))?;

    let output = TabularDeidentificationService
        .deidentify_csv(&csv, &policies, &mut vault, SurfaceKind::Cli)
        .map_err(|err| format!("failed to deidentify CSV: {err}"))?;
    fs::write(&args.output_path, &output.csv)
        .map_err(|err| format!("failed to write output CSV: {err}"))?;

    print_summary(
        &args.output_path,
        &output.summary,
        output.review_queue.len(),
    )?;
    Ok(())
}

fn run_deidentify_xlsx(args: DeidentifyXlsxArgs) -> Result<(), String> {
    let policies = parse_policies(&args.policies_json)?;
    let workbook_bytes =
        fs::read(&args.xlsx_path).map_err(|err| format!("failed to read XLSX workbook: {err}"))?;
    let extracted = XlsxTabularAdapter::new(policies)
        .extract(&workbook_bytes)
        .map_err(|err| format!("failed to read XLSX workbook: {err}"))?;
    let mut vault = if args.vault_path.exists() {
        LocalVaultStore::unlock(&args.vault_path, &args.passphrase)
    } else {
        LocalVaultStore::create(&args.vault_path, &args.passphrase)
    }
    .map_err(|err| format!("failed to open vault: {err}"))?;

    let output = TabularDeidentificationService
        .deidentify_extracted(extracted, &mut vault, SurfaceKind::Cli)
        .map_err(|err| format!("failed to deidentify XLSX: {err}"))?;
    let rendered = render_xlsx_output(&output.csv)
        .map_err(|err| format!("failed to render XLSX output: {err}"))?;
    fs::write(&args.output_path, rendered)
        .map_err(|err| format!("failed to write output XLSX: {err}"))?;

    print_summary(
        &args.output_path,
        &output.summary,
        output.review_queue.len(),
    )?;
    Ok(())
}

fn render_xlsx_output(csv: &str) -> Result<Vec<u8>, String> {
    let extracted = CsvTabularAdapter::new(Vec::new())
        .extract(csv.as_bytes())
        .map_err(|err| err.to_string())?;
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    for (column_index, column) in extracted.columns.iter().enumerate() {
        worksheet
            .write_string(0, column_index as u16, &column.name)
            .map_err(|err| err.to_string())?;
    }
    for (row_index, row) in extracted.rows.iter().enumerate() {
        for (column_index, value) in row.iter().enumerate() {
            worksheet
                .write_string((row_index + 1) as u32, column_index as u16, value)
                .map_err(|err| err.to_string())?;
        }
    }
    workbook.save_to_buffer().map_err(|err| err.to_string())
}

fn parse_policies(policies_json: &str) -> Result<Vec<FieldPolicy>, String> {
    let requests: Vec<PolicyRequest> = serde_json::from_str(policies_json)
        .map_err(|err| format!("invalid policies JSON: {err}"))?;
    Ok(requests
        .into_iter()
        .map(|request| FieldPolicy {
            header: request.header,
            phi_type: request.phi_type,
            action: match request.action {
                PolicyActionRequest::Encode => FieldPolicyAction::Encode,
                PolicyActionRequest::Review => FieldPolicyAction::Review,
                PolicyActionRequest::Ignore => FieldPolicyAction::Ignore,
            },
        })
        .collect())
}

fn print_summary(
    output_path: &PathBuf,
    summary: &BatchSummary,
    review_queue_len: usize,
) -> Result<(), String> {
    let payload = json!({
        "output_path": output_path,
        "summary": summary,
        "review_queue_len": review_queue_len,
    });
    println!(
        "{}",
        serde_json::to_string(&payload)
            .map_err(|err| format!("failed to render summary: {err}"))?
    );
    Ok(())
}

fn exit_with_usage(error: &str) -> ! {
    eprintln!("{error}");
    eprintln!();
    eprintln!("{}", usage());
    process::exit(2);
}

fn usage() -> &'static str {
    "Usage: mdid-cli [status]\n       mdid-cli verify-artifacts --artifact-paths-json <json-array> [--max-bytes <bytes>]\n       mdid-cli deidentify-csv --csv-path <path> --policies-json <json> --vault-path <path> --passphrase <value> --output-path <path>\n       mdid-cli deidentify-xlsx --xlsx-path <path> --policies-json <json> --vault-path <path> --passphrase <value> --output-path <path>\n       mdid-cli deidentify-dicom --dicom-path <input.dcm> --private-tag-policy <remove|review|required|keep> --vault-path <vault.json> --passphrase <passphrase> --output-path <output.dcm>\n       mdid-cli deidentify-pdf --pdf-path <input.pdf> --source-name <name.pdf> --report-path <report.json>\n       mdid-cli review-media --artifact-label <label> --format <image|video|fcs> --metadata-json <json> --requires-visual-review <true|false> --unsupported-payload <true|false> --report-path <report.json>\n       mdid-cli privacy-filter-text --input-path <path> --runner-path <path> --report-path <report.json> [--python-command <path-or-command>]\n       mdid-cli vault-audit --vault-path <vault.json> --passphrase <passphrase> [--limit <count>] [--offset <count>]\n       mdid-cli vault-decode --vault-path <vault.json> --passphrase <passphrase> --record-ids-json <json> --output-target <target> --justification <text> --report-path <report.json>\n       mdid-cli vault-export --vault-path <vault.json> --passphrase <passphrase> --record-ids-json <json> --export-passphrase <passphrase> --context <text> --artifact-path <export.json>\n       mdid-cli vault-import --vault-path <vault.json> --passphrase <passphrase> --artifact-path <export.json> --portable-passphrase <passphrase> --context <text>\n       mdid-cli vault-inspect-artifact --artifact-path <export.json> --portable-passphrase <passphrase>\n\nmdid-cli is the local de-identification automation surface.\nCommands:\n  status              Print a readiness banner for the local CLI surface.\n  verify-artifacts    Verify local artifact existence and size with metadata-only PHI-safe JSON.\n  deidentify-csv      Rewrite a local CSV using explicit field policies.\n  deidentify-xlsx     Rewrite a bounded local XLSX using explicit field policies.\n  deidentify-dicom    Rewrite a bounded local DICOM file with a PHI-safe summary.\n  deidentify-pdf      Review a bounded local PDF and write a PHI-safe JSON report; no OCR or PDF rewrite/export.\n  review-media        Review conservative media metadata and write a PHI-safe JSON report; no media rewrite/export.\n  privacy-filter-text Run a local privacy filter runner for text and write its bounded JSON report.\n  vault-audit         Print bounded PHI-safe vault audit event metadata in reverse chronological order; read-only.\n  vault-decode        Decode explicitly scoped vault records to a report file and print a PHI-safe summary.\n  vault-export        Export explicitly scoped vault records to an encrypted portable artifact and print a PHI-safe summary.\n  vault-import        Import encrypted portable vault records into a local vault and print a PHI-safe summary.\n  vault-inspect-artifact Inspect an encrypted portable vault artifact and print only a PHI-safe record count."
}

#[cfg(test)]
mod tests {
    use super::*;
    use mdid_domain::MappingScope;
    use mdid_vault::NewMappingRecord;

    #[test]
    fn parses_verify_artifacts_command_without_requiring_debug() {
        let command = parse_command(&[
            "verify-artifacts".to_string(),
            "--artifact-paths-json".to_string(),
            "[\"/tmp/a.csv\",\"/tmp/b.json\"]".to_string(),
            "--max-bytes".to_string(),
            "1024".to_string(),
        ])
        .expect("verify artifacts command should parse");

        match command {
            CliCommand::VerifyArtifacts(args) => {
                assert_eq!(args.artifact_paths_json, "[\"/tmp/a.csv\",\"/tmp/b.json\"]");
                assert_eq!(args.max_bytes, Some(1024));
            }
            _ => panic!("expected VerifyArtifacts command"),
        }
    }

    #[test]
    fn verify_artifacts_report_checks_metadata_without_printing_phi_paths_or_contents() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let phi_path = temp_dir.path().join("Jane-Doe-MRN-123-output.csv");
        std::fs::write(&phi_path, "name\nJane Doe\n").expect("write fixture");

        let report =
            build_verify_artifacts_report(&[phi_path.to_string_lossy().to_string()], Some(1024))
                .expect("report");
        let json = serde_json::to_string(&report).expect("json");

        assert_eq!(report.artifact_count, 1);
        assert_eq!(report.existing_count, 1);
        assert_eq!(report.oversized_count, 0);
        assert_eq!(report.missing_count, 0);
        assert!(!json.contains("Jane"));
        assert!(!json.contains("MRN"));
        assert!(!json.contains("name"));
    }

    #[test]
    fn verify_artifacts_report_treats_directory_as_missing_without_printing_phi_name() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let phi_dir = temp_dir.path().join("Jane-Doe-MRN-123-artifact-dir");
        std::fs::create_dir(&phi_dir).expect("create directory fixture");

        let report =
            build_verify_artifacts_report(&[phi_dir.to_string_lossy().to_string()], Some(1024))
                .expect("report");
        let json = serde_json::to_string(&report).expect("json");

        assert_eq!(report.artifact_count, 1);
        assert_eq!(report.existing_count, 0);
        assert_eq!(report.missing_count, 1);
        assert_eq!(report.oversized_count, 0);
        assert_eq!(report.artifacts[0].index, 0);
        assert!(!report.artifacts[0].exists);
        assert_eq!(report.artifacts[0].byte_len, None);
        assert_eq!(report.artifacts[0].within_max_bytes, None);
        assert!(!json.contains("Jane"));
        assert!(!json.contains("MRN"));
        assert!(!json.contains("artifact-dir"));
    }

    #[cfg(unix)]
    #[test]
    fn verify_artifacts_report_does_not_follow_symlinked_file() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let target_path = temp_dir.path().join("target.bin");
        let symlink_path = temp_dir.path().join("artifact-link.bin");
        std::fs::write(&target_path, b"abcd").expect("write target fixture");
        std::os::unix::fs::symlink(&target_path, &symlink_path).expect("create symlink fixture");

        let report = build_verify_artifacts_report(
            &[symlink_path.to_string_lossy().to_string()],
            Some(1024),
        )
        .expect("report");

        assert_eq!(report.artifact_count, 1);
        assert_eq!(report.existing_count, 0);
        assert_eq!(report.missing_count, 1);
        assert_eq!(report.oversized_count, 0);
        assert!(!report.artifacts[0].exists);
        assert_eq!(report.artifacts[0].byte_len, None);
        assert_eq!(report.artifacts[0].within_max_bytes, None);
    }

    #[test]
    fn verify_artifacts_report_counts_missing_file_as_missing() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let missing_path = temp_dir.path().join("missing-output.csv");

        let report = build_verify_artifacts_report(
            &[missing_path.to_string_lossy().to_string()],
            Some(1024),
        )
        .expect("report");

        assert_eq!(report.artifact_count, 1);
        assert_eq!(report.existing_count, 0);
        assert_eq!(report.missing_count, 1);
        assert_eq!(report.oversized_count, 0);
        assert!(!report.artifacts[0].exists);
        assert_eq!(report.artifacts[0].byte_len, None);
        assert_eq!(report.artifacts[0].within_max_bytes, Some(false));
    }

    #[test]
    fn verify_artifacts_report_counts_oversized_file_as_existing_and_oversized() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let artifact_path = temp_dir.path().join("artifact.bin");
        std::fs::write(&artifact_path, b"abcd").expect("write fixture");

        let report =
            build_verify_artifacts_report(&[artifact_path.to_string_lossy().to_string()], Some(3))
                .expect("report");

        assert_eq!(report.artifact_count, 1);
        assert_eq!(report.existing_count, 1);
        assert_eq!(report.missing_count, 0);
        assert_eq!(report.oversized_count, 1);
        assert!(report.artifacts[0].exists);
        assert_eq!(report.artifacts[0].byte_len, Some(4));
        assert_eq!(report.artifacts[0].within_max_bytes, Some(false));
    }

    #[test]
    fn verify_artifacts_rejects_empty_path_list_and_non_positive_max_bytes() {
        assert!(parse_artifact_paths_json("[]").is_err());
        assert!(parse_positive_max_bytes("0").is_err());
        assert!(parse_positive_max_bytes("not-a-number").is_err());
    }

    #[test]
    fn verify_artifacts_report_rejects_duplicate_paths_without_echoing_path() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let phi_path = temp_dir.path().join("Jane-Doe-MRN-123-output.csv");
        std::fs::write(&phi_path, "name\nJane Doe\n").expect("write fixture");
        let path = phi_path.to_string_lossy().to_string();

        let error = build_verify_artifacts_report(&[path.clone(), path], Some(1024))
            .expect_err("duplicate artifact path should be rejected");

        assert_eq!(error, "artifact path list must not contain duplicate paths");
        assert!(!error.contains("Jane"));
        assert!(!error.contains("MRN"));
        assert!(!error.contains("output.csv"));
    }

    #[test]
    fn parses_deidentify_csv_command_without_requiring_debug() {
        let policies_json = r#"[{"header":"n","phi_type":"NAME","action":"encode"}]"#;
        let args = vec![
            "deidentify-csv".to_string(),
            "--csv-path".to_string(),
            "input.csv".to_string(),
            "--policies-json".to_string(),
            policies_json.to_string(),
            "--vault-path".to_string(),
            "vault.mdid".to_string(),
            "--passphrase".to_string(),
            "secret-passphrase".to_string(),
            "--output-path".to_string(),
            "output.csv".to_string(),
        ];

        assert!(
            parse_command(&args)
                == Ok(CliCommand::DeidentifyCsv(DeidentifyCsvArgs {
                    csv_path: PathBuf::from("input.csv"),
                    policies_json: policies_json.to_string(),
                    vault_path: PathBuf::from("vault.mdid"),
                    passphrase: "secret-passphrase".to_string(),
                    output_path: PathBuf::from("output.csv"),
                }))
        );
    }

    #[test]
    fn parses_deidentify_xlsx_command_without_requiring_debug() {
        let policies_json = r#"[{"header":"patient_name","phi_type":"NAME","action":"encode"}]"#;
        let args = vec![
            "deidentify-xlsx".to_string(),
            "--xlsx-path".to_string(),
            "input.xlsx".to_string(),
            "--policies-json".to_string(),
            policies_json.to_string(),
            "--vault-path".to_string(),
            "vault.mdid".to_string(),
            "--passphrase".to_string(),
            "secret-passphrase".to_string(),
            "--output-path".to_string(),
            "output.xlsx".to_string(),
        ];

        assert!(
            parse_command(&args)
                == Ok(CliCommand::DeidentifyXlsx(DeidentifyXlsxArgs {
                    xlsx_path: PathBuf::from("input.xlsx"),
                    policies_json: policies_json.to_string(),
                    vault_path: PathBuf::from("vault.mdid"),
                    passphrase: "secret-passphrase".to_string(),
                    output_path: PathBuf::from("output.xlsx"),
                }))
        );
    }

    #[test]
    fn parses_deidentify_pdf_command_without_requiring_debug() {
        let args = vec![
            "deidentify-pdf".to_string(),
            "--pdf-path".to_string(),
            "input.pdf".to_string(),
            "--source-name".to_string(),
            "scan.pdf".to_string(),
            "--report-path".to_string(),
            "report.json".to_string(),
        ];

        assert!(
            parse_command(&args)
                == Ok(CliCommand::DeidentifyPdf(DeidentifyPdfArgs {
                    pdf_path: PathBuf::from("input.pdf"),
                    source_name: "scan.pdf".to_string(),
                    report_path: PathBuf::from("report.json"),
                }))
        );
    }

    #[test]
    fn parses_vault_decode_command() {
        let record_ids_json = r#"["00000000-0000-0000-0000-000000000001"]"#;
        let args = vec![
            "vault-decode".to_string(),
            "--vault-path".to_string(),
            "vault.mdid".to_string(),
            "--passphrase".to_string(),
            "secret-passphrase".to_string(),
            "--record-ids-json".to_string(),
            record_ids_json.to_string(),
            "--output-target".to_string(),
            "report-only".to_string(),
            "--justification".to_string(),
            "patient request".to_string(),
            "--report-path".to_string(),
            "decode-report.json".to_string(),
        ];

        assert!(
            parse_command(&args)
                == Ok(CliCommand::VaultDecode(VaultDecodeArgs {
                    vault_path: PathBuf::from("vault.mdid"),
                    passphrase: "secret-passphrase".to_string(),
                    record_ids_json: record_ids_json.to_string(),
                    output_target: "report-only".to_string(),
                    justification: "patient request".to_string(),
                    report_path: PathBuf::from("decode-report.json"),
                }))
        );
    }

    #[test]
    fn parses_vault_export_command() {
        let record_ids_json = r#"["00000000-0000-0000-0000-000000000001"]"#;
        let args = vec![
            "vault-export".to_string(),
            "--vault-path".to_string(),
            "vault.mdid".to_string(),
            "--passphrase".to_string(),
            "secret-passphrase".to_string(),
            "--record-ids-json".to_string(),
            record_ids_json.to_string(),
            "--export-passphrase".to_string(),
            "portable-secret".to_string(),
            "--context".to_string(),
            "handoff".to_string(),
            "--artifact-path".to_string(),
            "portable-export.json".to_string(),
        ];

        assert!(
            parse_command(&args)
                == Ok(CliCommand::VaultExport(VaultExportArgs {
                    vault_path: PathBuf::from("vault.mdid"),
                    passphrase: "secret-passphrase".to_string(),
                    record_ids_json: record_ids_json.to_string(),
                    export_passphrase: "portable-secret".to_string(),
                    context: "handoff".to_string(),
                    artifact_path: PathBuf::from("portable-export.json"),
                }))
        );
    }

    #[test]
    fn parse_vault_export_rejects_blank_context_without_echoing_value() {
        let error = parse_vault_export_args(&[
            "--vault-path".to_string(),
            "vault.json".to_string(),
            "--passphrase".to_string(),
            "secret".to_string(),
            "--record-ids-json".to_string(),
            "[\"record-1\"]".to_string(),
            "--export-passphrase".to_string(),
            "portable-secret".to_string(),
            "--context".to_string(),
            "   ".to_string(),
            "--artifact-path".to_string(),
            "artifact.json".to_string(),
        ])
        .expect_err("blank export context should be rejected");

        assert_eq!(error, "missing --context");
        assert!(!error.contains("record-1"));
        assert!(!error.contains("portable-secret"));
    }

    #[test]
    fn parse_vault_import_rejects_blank_context_without_echoing_value() {
        let error = parse_vault_import_args(&[
            "--vault-path".to_string(),
            "vault.json".to_string(),
            "--passphrase".to_string(),
            "secret".to_string(),
            "--artifact-path".to_string(),
            "artifact.json".to_string(),
            "--portable-passphrase".to_string(),
            "portable-secret".to_string(),
            "--context".to_string(),
            "\t".to_string(),
        ])
        .expect_err("blank import context should be rejected");

        assert_eq!(error, "missing --context");
        assert!(!error.contains("artifact.json"));
        assert!(!error.contains("portable-secret"));
    }

    #[test]
    fn parses_vault_import_command() {
        let args = vec![
            "vault-import".to_string(),
            "--vault-path".to_string(),
            "target-vault.mdid".to_string(),
            "--passphrase".to_string(),
            "target-secret".to_string(),
            "--artifact-path".to_string(),
            "portable-export.json".to_string(),
            "--portable-passphrase".to_string(),
            "portable-secret".to_string(),
            "--context".to_string(),
            "import".to_string(),
        ];

        assert!(
            parse_command(&args)
                == Ok(CliCommand::VaultImport(VaultImportArgs {
                    vault_path: PathBuf::from("target-vault.mdid"),
                    passphrase: "target-secret".to_string(),
                    artifact_path: PathBuf::from("portable-export.json"),
                    portable_passphrase: "portable-secret".to_string(),
                    context: "import".to_string(),
                }))
        );
    }

    #[test]
    fn vault_export_writes_artifact_and_returns_phi_safe_summary() {
        let temp_dir = tempfile::tempdir().unwrap();
        let vault_path = temp_dir.path().join("vault.mdid");
        let artifact_path = temp_dir.path().join("portable-export.json");
        let mut vault = LocalVaultStore::create(&vault_path, "secret-passphrase").unwrap();
        let record = vault
            .store_mapping(
                NewMappingRecord {
                    scope: MappingScope::new(
                        Uuid::new_v4(),
                        Uuid::new_v4(),
                        "patient.name".to_string(),
                    ),
                    phi_type: "NAME".to_string(),
                    original_value: "Alice Example".to_string(),
                },
                SurfaceKind::Cli,
            )
            .unwrap();

        let summary = run_vault_export_for_summary(VaultExportArgs {
            vault_path,
            passphrase: "secret-passphrase".to_string(),
            record_ids_json: format!(r#"["{}"]"#, record.id),
            export_passphrase: "portable-secret".to_string(),
            context: "handoff".to_string(),
            artifact_path: artifact_path.clone(),
        })
        .unwrap();

        assert!(artifact_path.exists());
        let artifact_json = fs::read_to_string(&artifact_path).unwrap();
        assert!(artifact_json.contains("ciphertext_b64"));
        let summary_json: serde_json::Value = serde_json::from_str(&summary).unwrap();
        assert_eq!(summary_json["command"], "vault-export");
        assert_eq!(summary_json["exported_records"], 1);
        assert_eq!(
            summary_json["artifact_path"].as_str().unwrap(),
            artifact_path.to_string_lossy()
        );
        assert!(Uuid::parse_str(summary_json["audit_event_id"].as_str().unwrap()).is_ok());
        for secret in ["Alice Example", "secret-passphrase", "portable-secret"] {
            assert!(!summary.contains(secret));
        }
    }

    #[test]
    fn parses_vault_inspect_artifact_command() {
        let args = vec![
            "vault-inspect-artifact".to_string(),
            "--artifact-path".to_string(),
            "portable-export.json".to_string(),
            "--portable-passphrase".to_string(),
            "portable-secret".to_string(),
        ];

        assert!(
            parse_command(&args)
                == Ok(CliCommand::VaultInspectArtifact(VaultInspectArtifactArgs {
                    artifact_path: PathBuf::from("portable-export.json"),
                    portable_passphrase: "portable-secret".to_string(),
                }))
        );
    }

    #[test]
    fn vault_inspect_artifact_returns_phi_safe_record_count() {
        let temp_dir = tempfile::tempdir().unwrap();
        let source_vault_path = temp_dir.path().join("source-vault.mdid");
        let artifact_path = temp_dir.path().join("portable-export.json");
        let mut source_vault =
            LocalVaultStore::create(&source_vault_path, "source-secret").unwrap();
        let record = source_vault
            .store_mapping(
                NewMappingRecord {
                    scope: MappingScope::new(
                        Uuid::new_v4(),
                        Uuid::new_v4(),
                        "patient.name".to_string(),
                    ),
                    phi_type: "NAME".to_string(),
                    original_value: "Alice Example".to_string(),
                },
                SurfaceKind::Cli,
            )
            .unwrap();
        run_vault_export_for_summary(VaultExportArgs {
            vault_path: source_vault_path.clone(),
            passphrase: "source-secret".to_string(),
            record_ids_json: format!(r#"["{}"]"#, record.id),
            export_passphrase: "portable-secret".to_string(),
            context: "handoff".to_string(),
            artifact_path: artifact_path.clone(),
        })
        .unwrap();

        let summary = run_vault_inspect_artifact_for_summary(VaultInspectArtifactArgs {
            artifact_path: artifact_path.clone(),
            portable_passphrase: "portable-secret".to_string(),
        })
        .unwrap();

        let summary_json: serde_json::Value = serde_json::from_str(&summary).unwrap();
        assert_eq!(summary_json["command"], "vault-inspect-artifact");
        assert_eq!(summary_json["record_count"], 1);
        for secret in [
            "Alice Example",
            "source-secret",
            "portable-secret",
            "MDID-NAME",
            &source_vault_path.to_string_lossy(),
            "ciphertext_b64",
            "nonce_b64",
            "salt_b64",
        ] {
            assert!(
                !summary.contains(secret),
                "summary leaked {secret}: {summary}"
            );
        }
    }

    #[test]
    fn vault_import_bounded_read_rejects_sparse_oversized_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let artifact_path = temp_dir
            .path()
            .join("sparse-oversized-portable-export.json");
        std::fs::File::create(&artifact_path)
            .unwrap()
            .set_len(MAX_PORTABLE_ARTIFACT_BYTES + 1)
            .unwrap();

        let error = read_bounded_portable_artifact(&artifact_path).unwrap_err();

        assert_eq!(error, "portable artifact exceeds maximum size");
    }

    #[test]
    fn vault_import_rejects_oversized_artifact_before_parsing() {
        let temp_dir = tempfile::tempdir().unwrap();
        let vault_path = temp_dir.path().join("target-vault.mdid");
        let artifact_path = temp_dir.path().join("oversized-portable-export.json");
        LocalVaultStore::create(&vault_path, "target-secret").unwrap();
        std::fs::File::create(&artifact_path)
            .unwrap()
            .set_len(MAX_PORTABLE_ARTIFACT_BYTES + 1)
            .unwrap();

        let error = run_vault_import_for_summary(VaultImportArgs {
            vault_path,
            passphrase: "target-secret".to_string(),
            artifact_path,
            portable_passphrase: "portable-secret".to_string(),
            context: "import".to_string(),
        })
        .unwrap_err();

        assert!(
            error.contains("portable artifact exceeds maximum size"),
            "unexpected error: {error}"
        );
        for secret in ["target-secret", "portable-secret"] {
            assert!(!error.contains(secret));
        }
    }

    #[test]
    fn vault_import_after_export_returns_phi_safe_summary() {
        let temp_dir = tempfile::tempdir().unwrap();
        let source_vault_path = temp_dir.path().join("source-vault.mdid");
        let target_vault_path = temp_dir.path().join("target-vault.mdid");
        let artifact_path = temp_dir.path().join("portable-export.json");
        let mut source_vault =
            LocalVaultStore::create(&source_vault_path, "source-secret").unwrap();
        LocalVaultStore::create(&target_vault_path, "target-secret").unwrap();
        let record = source_vault
            .store_mapping(
                NewMappingRecord {
                    scope: MappingScope::new(
                        Uuid::new_v4(),
                        Uuid::new_v4(),
                        "patient.name".to_string(),
                    ),
                    phi_type: "NAME".to_string(),
                    original_value: "Alice Example".to_string(),
                },
                SurfaceKind::Cli,
            )
            .unwrap();
        run_vault_export_for_summary(VaultExportArgs {
            vault_path: source_vault_path,
            passphrase: "source-secret".to_string(),
            record_ids_json: format!(r#"["{}"]"#, record.id),
            export_passphrase: "portable-secret".to_string(),
            context: "handoff".to_string(),
            artifact_path: artifact_path.clone(),
        })
        .unwrap();

        let summary = run_vault_import_for_summary(VaultImportArgs {
            vault_path: target_vault_path,
            passphrase: "target-secret".to_string(),
            artifact_path,
            portable_passphrase: "portable-secret".to_string(),
            context: "import".to_string(),
        })
        .unwrap();

        let summary_json: serde_json::Value = serde_json::from_str(&summary).unwrap();
        assert_eq!(summary_json["command"], "vault-import");
        assert_eq!(summary_json["imported_records"], 1);
        assert_eq!(summary_json["duplicate_records"], 0);
        assert!(Uuid::parse_str(summary_json["audit_event_id"].as_str().unwrap()).is_ok());
        for secret in [
            "Alice Example",
            "source-secret",
            "target-secret",
            "portable-secret",
        ] {
            assert!(!summary.contains(secret));
        }
    }

    #[test]
    fn vault_import_rerun_reports_duplicates_without_leaking_values() {
        let temp_dir = tempfile::tempdir().unwrap();
        let source_vault_path = temp_dir.path().join("source-vault.mdid");
        let target_vault_path = temp_dir.path().join("target-vault.mdid");
        let artifact_path = temp_dir.path().join("portable-export.json");
        let mut source_vault =
            LocalVaultStore::create(&source_vault_path, "source-secret").unwrap();
        LocalVaultStore::create(&target_vault_path, "target-secret").unwrap();
        let record = source_vault
            .store_mapping(
                NewMappingRecord {
                    scope: MappingScope::new(
                        Uuid::new_v4(),
                        Uuid::new_v4(),
                        "patient.name".to_string(),
                    ),
                    phi_type: "NAME".to_string(),
                    original_value: "Alice Example".to_string(),
                },
                SurfaceKind::Cli,
            )
            .unwrap();
        run_vault_export_for_summary(VaultExportArgs {
            vault_path: source_vault_path,
            passphrase: "source-secret".to_string(),
            record_ids_json: format!(r#"["{}"]"#, record.id),
            export_passphrase: "portable-secret".to_string(),
            context: "handoff".to_string(),
            artifact_path: artifact_path.clone(),
        })
        .unwrap();
        let args = VaultImportArgs {
            vault_path: target_vault_path,
            passphrase: "target-secret".to_string(),
            artifact_path,
            portable_passphrase: "portable-secret".to_string(),
            context: "import".to_string(),
        };
        run_vault_import_for_summary(args.clone()).unwrap();

        let duplicate_summary = run_vault_import_for_summary(args).unwrap();

        let summary_json: serde_json::Value = serde_json::from_str(&duplicate_summary).unwrap();
        assert_eq!(summary_json["command"], "vault-import");
        assert_eq!(summary_json["imported_records"], 0);
        assert_eq!(summary_json["duplicate_records"], 1);
        for secret in [
            "Alice Example",
            "MDID-NAME",
            "source-secret",
            "target-secret",
            "portable-secret",
        ] {
            assert!(!duplicate_summary.contains(secret));
        }
    }

    #[test]
    fn parses_vault_audit_command_without_requiring_debug() {
        let args = vec![
            "vault-audit".to_string(),
            "--vault-path".to_string(),
            "vault.mdid".to_string(),
            "--passphrase".to_string(),
            "secret-passphrase".to_string(),
            "--limit".to_string(),
            "10".to_string(),
        ];

        assert!(
            parse_command(&args)
                == Ok(CliCommand::VaultAudit(VaultAuditArgs {
                    vault_path: PathBuf::from("vault.mdid"),
                    passphrase: "secret-passphrase".to_string(),
                    limit: Some(10),
                    offset: 0,
                }))
        );
    }

    #[test]
    fn parses_record_ids_json_for_vault_decode() {
        let ids = parse_record_ids_json(
            r#"["00000000-0000-0000-0000-000000000001","00000000-0000-0000-0000-000000000002"]"#,
        )
        .unwrap();

        assert_eq!(ids.len(), 2);
        assert_eq!(ids[0].to_string(), "00000000-0000-0000-0000-000000000001");
        assert_eq!(ids[1].to_string(), "00000000-0000-0000-0000-000000000002");
    }

    #[test]
    fn rejects_empty_record_ids_json_for_vault_decode() {
        assert_eq!(
            parse_record_ids_json("[]"),
            Err("decode scope must include at least one record id".to_string())
        );
    }

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

    #[test]
    fn vault_decode_report_keeps_values_in_report_but_not_stdout_summary() {
        let report_path = PathBuf::from("decode-report.json");
        let report = VaultDecodeReport {
            decoded_value_count: 1,
            values: vec![VaultDecodeValueReport {
                record_id: "00000000-0000-0000-0000-000000000001".to_string(),
                token: "MDID-NAME-000001".to_string(),
                original_value: "Alice Example".to_string(),
            }],
            audit_event: serde_json::from_value(json!({
                "id":"00000000-0000-0000-0000-000000000000",
                "kind":"decode",
                "actor":"cli",
                "detail":"approved disclosure for Alice Example; case packet for Alice Example",
                "recorded_at":"2026-04-29T00:00:00Z"
            }))
            .unwrap(),
        };

        let report_json = serde_json::to_string(&report).unwrap();
        let stdout_json = build_vault_decode_stdout(&report_path, &report).unwrap();

        assert!(report_json.contains("Alice Example"));
        assert!(report_json.contains("original_value"));
        assert!(!stdout_json.contains("Alice Example"));
        assert!(!stdout_json.contains("original_value"));
        assert!(!stdout_json.contains("approved disclosure for Alice Example"));
        assert!(!stdout_json.contains("case packet for Alice Example"));
        assert!(stdout_json.contains("decode event"));
        assert!(stdout_json.contains("decoded_value_count"));
        assert!(stdout_json.contains("audit_event"));
    }

    #[test]
    fn rejects_invalid_and_zero_vault_audit_limits() {
        for limit in ["0", "not-a-number"] {
            let args = vec![
                "vault-audit".to_string(),
                "--vault-path".to_string(),
                "vault.mdid".to_string(),
                "--passphrase".to_string(),
                "secret-passphrase".to_string(),
                "--limit".to_string(),
                limit.to_string(),
            ];

            assert!(parse_command(&args) == Err("invalid --limit".to_string()));
        }
    }

    #[test]
    fn vault_audit_report_applies_default_and_max_limit_bounds() {
        let events = vault_audit_events(150, AuditEventKind::Encode);

        let default_report = build_vault_audit_report(&events, None, 0);
        let clamped_report = build_vault_audit_report(&events, Some(150), 0);

        assert_eq!(default_report.event_count, 150);
        assert_eq!(default_report.total_matching_events, 150);
        assert_eq!(default_report.returned_event_count, 100);
        assert_eq!(default_report.events.len(), 100);
        assert_eq!(clamped_report.returned_event_count, 100);
        assert_eq!(clamped_report.events.len(), 100);
    }

    #[test]
    fn vault_audit_report_returns_multiple_events_in_reverse_chronological_order() {
        let events = vault_audit_events(4, AuditEventKind::Encode);

        let report = build_vault_audit_report(&events, Some(3), 0);

        assert_eq!(report.returned_event_count, 3);
        assert_eq!(report.events[0].recorded_at, "2026-04-29T00:03:00+00:00");
        assert_eq!(report.events[1].recorded_at, "2026-04-29T00:02:00+00:00");
        assert_eq!(report.events[2].recorded_at, "2026-04-29T00:01:00+00:00");
    }

    #[test]
    fn vault_audit_report_applies_offset_and_next_metadata() {
        let events = vault_audit_events(5, AuditEventKind::Encode);

        let page = build_vault_audit_report(&events, Some(2), 1);
        let final_page = build_vault_audit_report(&events, Some(2), 4);

        assert_eq!(page.returned_event_count, 2);
        assert_eq!(page.total_matching_events, 5);
        assert_eq!(page.limit, 2);
        assert_eq!(page.offset, 1);
        assert_eq!(page.next_offset, Some(3));
        assert!(page.has_more);
        assert_eq!(page.events[0].recorded_at, "2026-04-29T00:03:00+00:00");
        assert_eq!(page.events[1].recorded_at, "2026-04-29T00:02:00+00:00");

        assert_eq!(final_page.returned_event_count, 1);
        assert_eq!(final_page.total_matching_events, 5);
        assert_eq!(final_page.offset, 4);
        assert_eq!(final_page.next_offset, None);
        assert!(!final_page.has_more);
    }

    #[test]
    fn vault_audit_parser_accepts_offset_and_rejects_invalid_offset() {
        let args = vec![
            "vault-audit".to_string(),
            "--vault-path".to_string(),
            "vault.mdid".to_string(),
            "--passphrase".to_string(),
            "secret-passphrase".to_string(),
            "--limit".to_string(),
            "2".to_string(),
            "--offset".to_string(),
            "3".to_string(),
        ];

        match parse_command(&args).unwrap() {
            CliCommand::VaultAudit(parsed) => assert_eq!(parsed.offset, 3),
            _ => panic!("expected vault audit command"),
        }

        let bad_args = vec![
            "vault-audit".to_string(),
            "--vault-path".to_string(),
            "vault.mdid".to_string(),
            "--passphrase".to_string(),
            "secret-passphrase".to_string(),
            "--offset".to_string(),
            "not-a-number".to_string(),
        ];
        assert!(parse_command(&bad_args) == Err("invalid --offset".to_string()));
    }

    #[test]
    fn usage_documents_vault_audit_offset() {
        assert!(usage().contains(
            "mdid-cli vault-audit --vault-path <vault.json> --passphrase <passphrase> [--limit <count>] [--offset <count>]"
        ));
    }

    #[test]
    fn vault_audit_report_uses_stable_kinds_and_sanitizes_all_details() {
        let events: Vec<mdid_domain::AuditEvent> = serde_json::from_value(json!([
            {"id":"00000000-0000-0000-0000-000000000000","kind":"encode","actor":"cli","detail":"Alice Example encoded","recorded_at":"2026-04-29T00:00:00Z"},
            {"id":"00000000-0000-0000-0000-000000000000","kind":"decode","actor":"cli","detail":"decoded Bob Example","recorded_at":"2026-04-29T00:01:00Z"},
            {"id":"00000000-0000-0000-0000-000000000000","kind":"export","actor":"cli","detail":"exported Carol Example","recorded_at":"2026-04-29T00:02:00Z"},
            {"id":"00000000-0000-0000-0000-000000000000","kind":"import","actor":"cli","detail":"imported Dan Example","recorded_at":"2026-04-29T00:03:00Z"}
        ]))
        .unwrap();

        let report = build_vault_audit_report(&events, Some(4), 0);
        let rendered = serde_json::to_string(&report).unwrap();

        assert_eq!(
            report
                .events
                .iter()
                .map(|event| (event.kind.as_str(), event.detail.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("import", "portable import event"),
                ("export", "portable export event"),
                ("decode", "decode event"),
                ("encode", "encoded mapping"),
            ]
        );
        for phi in [
            "Alice", "Bob", "Carol", "Dan", "Encode", "Decode", "Export", "Import",
        ] {
            assert!(!rendered.contains(phi));
        }
    }

    fn vault_audit_events(count: usize, kind: AuditEventKind) -> Vec<mdid_domain::AuditEvent> {
        (0..count)
            .map(|index| {
                serde_json::from_value(json!({
                    "id": "00000000-0000-0000-0000-000000000000",
                    "kind": kind.as_str(),
                    "actor": "cli",
                    "detail": format!("raw detail {index} Alice Example"),
                    "recorded_at": format!("2026-04-29T00:{:02}:00Z", index % 60)
                }))
                .unwrap()
            })
            .collect()
    }

    #[test]
    fn parses_deidentify_dicom_command_without_requiring_debug() {
        let args = vec![
            "deidentify-dicom".to_string(),
            "--dicom-path".to_string(),
            "input.dcm".to_string(),
            "--private-tag-policy".to_string(),
            "review".to_string(),
            "--vault-path".to_string(),
            "vault.mdid".to_string(),
            "--passphrase".to_string(),
            "secret-passphrase".to_string(),
            "--output-path".to_string(),
            "output.dcm".to_string(),
        ];

        assert!(
            parse_command(&args)
                == Ok(CliCommand::DeidentifyDicom(DeidentifyDicomArgs {
                    dicom_path: PathBuf::from("input.dcm"),
                    private_tag_policy: "review".to_string(),
                    vault_path: PathBuf::from("vault.mdid"),
                    passphrase: "secret-passphrase".to_string(),
                    output_path: PathBuf::from("output.dcm"),
                }))
        );
    }
}
