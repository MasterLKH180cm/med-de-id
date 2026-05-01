use std::{
    collections::HashSet,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
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
    PrivacyFilterCorpus(PrivacyFilterCorpusArgs),
    OcrToPrivacyFilter(OcrToPrivacyFilterArgs),
    OcrToPrivacyFilterCorpus(OcrToPrivacyFilterCorpusArgs),
    OcrHandoffCorpus(OcrHandoffCorpusArgs),
    OcrSmallJson(OcrSmallJsonArgs),
    OcrPrivacyEvidence(OcrPrivacyEvidenceArgs),
    OcrHandoff(OcrHandoffArgs),
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
enum PrivacyFilterTextInput {
    Path(PathBuf),
    Stdin,
}

#[derive(Clone, PartialEq, Eq)]
struct PrivacyFilterTextArgs {
    input: PrivacyFilterTextInput,
    runner_path: PathBuf,
    report_path: PathBuf,
    summary_output: Option<PathBuf>,
    python_command: String,
    mock: bool,
}

#[derive(Clone, PartialEq, Eq)]
struct PrivacyFilterCorpusArgs {
    fixture_dir: PathBuf,
    runner_path: PathBuf,
    report_path: PathBuf,
    summary_output: Option<PathBuf>,
    python_command: String,
}

#[derive(Clone, PartialEq, Eq)]
struct OcrHandoffCorpusArgs {
    fixture_dir: PathBuf,
    runner_path: PathBuf,
    report_path: PathBuf,
    summary_output: Option<PathBuf>,
    python_command: String,
}

#[derive(Clone, PartialEq, Eq)]
struct OcrToPrivacyFilterCorpusArgs {
    fixture_dir: PathBuf,
    ocr_runner_path: PathBuf,
    privacy_runner_path: PathBuf,
    bridge_runner_path: PathBuf,
    report_path: PathBuf,
    summary_output: Option<PathBuf>,
    python_command: String,
}

#[derive(Clone, PartialEq, Eq)]
struct OcrToPrivacyFilterArgs {
    image_path: PathBuf,
    ocr_runner_path: PathBuf,
    privacy_runner_path: PathBuf,
    report_path: PathBuf,
    summary_output: Option<PathBuf>,
    python_command: String,
    mock: bool,
}

#[derive(Clone, PartialEq, Eq)]
struct OcrSmallJsonArgs {
    image_path: PathBuf,
    ocr_runner_path: PathBuf,
    report_path: PathBuf,
    summary_output: Option<PathBuf>,
    python_command: String,
    mock: bool,
}

#[derive(Clone, PartialEq, Eq)]
struct OcrPrivacyEvidenceArgs {
    image_path: PathBuf,
    runner_path: PathBuf,
    output_path: PathBuf,
    python_command: String,
    mock: bool,
}

const PRIVACY_FILTER_RUNNER_STDOUT_MAX_BYTES: usize = 1024 * 1024;
const PRIVACY_FILTER_RUNNER_TIMEOUT: Duration = Duration::from_secs(2);
const OCR_RUNNER_STDOUT_MAX_BYTES: usize = 1024 * 1024;
const OCR_PRIVACY_EVIDENCE_REPORT_MAX_BYTES: usize = OCR_RUNNER_STDOUT_MAX_BYTES;
const OCR_RUNNER_TIMEOUT: Duration = PRIVACY_FILTER_RUNNER_TIMEOUT;
const OCR_HANDOFF_BUILDER_TIMEOUT: Duration = PRIVACY_FILTER_RUNNER_TIMEOUT;

#[derive(Clone, PartialEq, Eq)]
struct OcrHandoffArgs {
    image_path: PathBuf,
    ocr_runner_path: PathBuf,
    handoff_builder_path: PathBuf,
    report_path: PathBuf,
    python_command: String,
}

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
        [command, rest @ ..] if command == "privacy-filter-corpus" => {
            parse_privacy_filter_corpus_args(rest).map(CliCommand::PrivacyFilterCorpus)
        }
        [command, rest @ ..] if command == "ocr-to-privacy-filter" => {
            parse_ocr_to_privacy_filter_args(rest).map(CliCommand::OcrToPrivacyFilter)
        }
        [command, rest @ ..] if command == "ocr-to-privacy-filter-corpus" => {
            parse_ocr_to_privacy_filter_corpus_args(rest).map(CliCommand::OcrToPrivacyFilterCorpus)
        }
        [command, rest @ ..] if command == "ocr-handoff-corpus" => {
            parse_ocr_handoff_corpus_args(rest).map(CliCommand::OcrHandoffCorpus)
        }
        [command, rest @ ..] if command == "ocr-small-json" => {
            parse_ocr_small_json_args(rest).map(CliCommand::OcrSmallJson)
        }
        [command, rest @ ..] if command == "ocr-privacy-evidence" => {
            parse_ocr_privacy_evidence_args(rest).map(CliCommand::OcrPrivacyEvidence)
        }
        [command, rest @ ..] if command == "ocr-handoff" => {
            parse_ocr_handoff_args(rest).map(CliCommand::OcrHandoff)
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
    let mut stdin = false;
    let mut runner_path = None;
    let mut report_path = None;
    let mut summary_output = None;
    let mut python_command = None;
    let mut mock = false;

    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        if flag == "--mock" {
            mock = true;
            index += 1;
            continue;
        }
        if flag == "--stdin" {
            stdin = true;
            index += 1;
            continue;
        }
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag {
            "--input-path" => input_path = Some(non_blank_path(value, "--input-path")?),
            "--runner-path" => runner_path = Some(non_blank_path(value, "--runner-path")?),
            "--report-path" => report_path = Some(non_blank_path(value, "--report-path")?),
            "--summary-output" => summary_output = Some(non_blank_path(value, "--summary-output")?),
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

    let input = match (input_path, stdin) {
        (Some(path), false) => PrivacyFilterTextInput::Path(path),
        (None, true) => PrivacyFilterTextInput::Stdin,
        (None, false) => {
            return Err(
                "missing input source: provide exactly one of --input-path or --stdin".to_string(),
            )
        }
        (Some(_), true) => {
            return Err(
                "conflicting input sources: provide exactly one of --input-path or --stdin"
                    .to_string(),
            )
        }
    };

    Ok(PrivacyFilterTextArgs {
        input,
        runner_path: runner_path.ok_or_else(|| "missing --runner-path".to_string())?,
        report_path: report_path.ok_or_else(|| "missing --report-path".to_string())?,
        summary_output,
        python_command: python_command.unwrap_or_else(default_python_command),
        mock,
    })
}

fn parse_privacy_filter_corpus_args(args: &[String]) -> Result<PrivacyFilterCorpusArgs, String> {
    let mut fixture_dir = None;
    let mut runner_path = None;
    let mut report_path = None;
    let mut summary_output = None;
    let mut python_command = None;

    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag {
            "--fixture-dir" => fixture_dir = Some(non_blank_path(value, "--fixture-dir")?),
            "--runner-path" => runner_path = Some(non_blank_path(value, "--runner-path")?),
            "--report-path" => report_path = Some(non_blank_path(value, "--report-path")?),
            "--summary-output" => summary_output = Some(non_blank_path(value, "--summary-output")?),
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

    Ok(PrivacyFilterCorpusArgs {
        fixture_dir: fixture_dir.ok_or_else(|| "missing --fixture-dir".to_string())?,
        runner_path: runner_path.ok_or_else(|| "missing --runner-path".to_string())?,
        report_path: report_path.ok_or_else(|| "missing --report-path".to_string())?,
        summary_output,
        python_command: python_command.unwrap_or_else(default_python_command),
    })
}

fn parse_ocr_handoff_corpus_args(args: &[String]) -> Result<OcrHandoffCorpusArgs, String> {
    let parsed = parse_privacy_filter_corpus_args(args)?;
    Ok(OcrHandoffCorpusArgs {
        fixture_dir: parsed.fixture_dir,
        runner_path: parsed.runner_path,
        report_path: parsed.report_path,
        summary_output: parsed.summary_output,
        python_command: parsed.python_command,
    })
}

fn parse_ocr_to_privacy_filter_args(args: &[String]) -> Result<OcrToPrivacyFilterArgs, String> {
    let mut image_path = None;
    let mut ocr_runner_path = None;
    let mut privacy_runner_path = None;
    let mut report_path = None;
    let mut summary_output = None;
    let mut python_command = None;
    let mut mock = false;
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        if flag == "--mock" {
            mock = true;
            index += 1;
            continue;
        }
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag {
            "--image-path" => image_path = Some(non_blank_path(value, "--image-path")?),
            "--ocr-runner-path" => {
                ocr_runner_path = Some(non_blank_path(value, "--ocr-runner-path")?)
            }
            "--privacy-runner-path" => {
                privacy_runner_path = Some(non_blank_path(value, "--privacy-runner-path")?)
            }
            "--report-path" => report_path = Some(non_blank_path(value, "--report-path")?),
            "--summary-output" => summary_output = Some(non_blank_path(value, "--summary-output")?),
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
    Ok(OcrToPrivacyFilterArgs {
        image_path: image_path.ok_or_else(|| "missing --image-path".to_string())?,
        ocr_runner_path: ocr_runner_path.ok_or_else(|| "missing --ocr-runner-path".to_string())?,
        privacy_runner_path: privacy_runner_path
            .ok_or_else(|| "missing --privacy-runner-path".to_string())?,
        report_path: report_path.ok_or_else(|| "missing --report-path".to_string())?,
        summary_output,
        python_command: python_command.unwrap_or_else(default_python_command),
        mock,
    })
}

fn parse_ocr_to_privacy_filter_corpus_args(
    args: &[String],
) -> Result<OcrToPrivacyFilterCorpusArgs, String> {
    let mut fixture_dir = None;
    let mut ocr_runner_path = None;
    let mut privacy_runner_path = None;
    let mut bridge_runner_path = None;
    let mut report_path = None;
    let mut summary_output = None;
    let mut python_command = None;

    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag {
            "--fixture-dir" => fixture_dir = Some(non_blank_path(value, "--fixture-dir")?),
            "--ocr-runner-path" => {
                ocr_runner_path = Some(non_blank_path(value, "--ocr-runner-path")?)
            }
            "--privacy-runner-path" => {
                privacy_runner_path = Some(non_blank_path(value, "--privacy-runner-path")?)
            }
            "--bridge-runner-path" => {
                bridge_runner_path = Some(non_blank_path(value, "--bridge-runner-path")?)
            }
            "--report-path" => report_path = Some(non_blank_path(value, "--report-path")?),
            "--summary-output" => summary_output = Some(non_blank_path(value, "--summary-output")?),
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

    Ok(OcrToPrivacyFilterCorpusArgs {
        fixture_dir: fixture_dir.ok_or_else(|| "missing --fixture-dir".to_string())?,
        ocr_runner_path: ocr_runner_path.ok_or_else(|| "missing --ocr-runner-path".to_string())?,
        privacy_runner_path: privacy_runner_path
            .ok_or_else(|| "missing --privacy-runner-path".to_string())?,
        bridge_runner_path: bridge_runner_path
            .ok_or_else(|| "missing --bridge-runner-path".to_string())?,
        report_path: report_path.ok_or_else(|| "missing --report-path".to_string())?,
        summary_output,
        python_command: python_command.unwrap_or_else(default_python_command),
    })
}

fn parse_ocr_handoff_args(args: &[String]) -> Result<OcrHandoffArgs, String> {
    let mut image_path = None;
    let mut ocr_runner_path = None;
    let mut handoff_builder_path = None;
    let mut report_path = None;
    let mut python_command = None;

    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag {
            "--image-path" => image_path = Some(non_blank_path(value, "--image-path")?),
            "--ocr-runner-path" => {
                ocr_runner_path = Some(non_blank_path(value, "--ocr-runner-path")?)
            }
            "--handoff-builder-path" => {
                handoff_builder_path = Some(non_blank_path(value, "--handoff-builder-path")?)
            }
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

    Ok(OcrHandoffArgs {
        image_path: image_path.ok_or_else(|| "missing --image-path".to_string())?,
        ocr_runner_path: ocr_runner_path.ok_or_else(|| "missing --ocr-runner-path".to_string())?,
        handoff_builder_path: handoff_builder_path
            .ok_or_else(|| "missing --handoff-builder-path".to_string())?,
        report_path: report_path.ok_or_else(|| "missing --report-path".to_string())?,
        python_command: python_command.unwrap_or_else(default_python_command),
    })
}

fn parse_ocr_small_json_args(args: &[String]) -> Result<OcrSmallJsonArgs, String> {
    let mut image_path = None;
    let mut ocr_runner_path = None;
    let mut report_path = None;
    let mut summary_output = None;
    let mut python_command = None;
    let mut mock = false;

    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        if flag == "--mock" {
            mock = true;
            index += 1;
            continue;
        }
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag {
            "--image-path" => image_path = Some(non_blank_path(value, "--image-path")?),
            "--ocr-runner-path" => {
                ocr_runner_path = Some(non_blank_path(value, "--ocr-runner-path")?)
            }
            "--report-path" => report_path = Some(non_blank_path(value, "--report-path")?),
            "--summary-output" => summary_output = Some(non_blank_path(value, "--summary-output")?),
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

    Ok(OcrSmallJsonArgs {
        image_path: image_path.ok_or_else(|| "missing --image-path".to_string())?,
        ocr_runner_path: ocr_runner_path.ok_or_else(|| "missing --ocr-runner-path".to_string())?,
        report_path: report_path.ok_or_else(|| "missing --report-path".to_string())?,
        summary_output,
        python_command: python_command.unwrap_or_else(default_python_command),
        mock,
    })
}

fn parse_ocr_privacy_evidence_args(args: &[String]) -> Result<OcrPrivacyEvidenceArgs, String> {
    let mut image_path = None;
    let mut runner_path = None;
    let mut output_path = None;
    let mut python_command = None;
    let mut mock = false;
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        if flag == "--mock" {
            mock = true;
            index += 1;
            continue;
        }
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag {
            "--image-path" => image_path = Some(non_blank_path(value, "--image-path")?),
            "--runner-path" => runner_path = Some(non_blank_path(value, "--runner-path")?),
            "--output" => output_path = Some(non_blank_path(value, "--output")?),
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
    Ok(OcrPrivacyEvidenceArgs {
        image_path: image_path.ok_or_else(|| "missing --image-path".to_string())?,
        runner_path: runner_path.ok_or_else(|| "missing --runner-path".to_string())?,
        output_path: output_path.ok_or_else(|| "missing --output".to_string())?,
        python_command: python_command.unwrap_or_else(default_python_command),
        mock,
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
        CliCommand::PrivacyFilterCorpus(args) => run_privacy_filter_corpus(args),
        CliCommand::OcrToPrivacyFilter(args) => run_ocr_to_privacy_filter(args),
        CliCommand::OcrToPrivacyFilterCorpus(args) => run_ocr_to_privacy_filter_corpus(args),
        CliCommand::OcrHandoffCorpus(args) => run_ocr_handoff_corpus(args),
        CliCommand::OcrSmallJson(args) => run_ocr_small_json(args),
        CliCommand::OcrPrivacyEvidence(args) => run_ocr_privacy_evidence(args),
        CliCommand::OcrHandoff(args) => run_ocr_handoff(args),
        CliCommand::VaultAudit(args) => run_vault_audit(args),
        CliCommand::VaultDecode(args) => run_vault_decode(args),
        CliCommand::VaultExport(args) => run_vault_export(args),
        CliCommand::VaultImport(args) => run_vault_import(args),
        CliCommand::VaultInspectArtifact(args) => run_vault_inspect_artifact(args),
    }
}

fn run_ocr_handoff_corpus(args: OcrHandoffCorpusArgs) -> Result<(), String> {
    if let Some(summary_output) = &args.summary_output {
        if summary_output == &args.report_path {
            return Err("OCR handoff corpus report and summary paths must differ".to_string());
        }
    }
    let _ = fs::remove_file(&args.report_path);
    if let Some(summary_output) = &args.summary_output {
        let _ = fs::remove_file(summary_output);
    }
    let result = (|| {
        require_directory(&args.fixture_dir, "missing fixture directory")?;
        require_regular_file(&args.runner_path, "missing runner file")?;
        run_ocr_handoff_corpus_inner(&args)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&args.report_path);
        if let Some(summary_output) = &args.summary_output {
            let _ = fs::remove_file(summary_output);
        }
    }
    result
}

fn run_ocr_handoff_corpus_inner(args: &OcrHandoffCorpusArgs) -> Result<(), String> {
    let mut child = std::process::Command::new(&args.python_command)
        .arg(&args.runner_path)
        .arg("--fixture-dir")
        .arg(&args.fixture_dir)
        .arg("--output")
        .arg(&args.report_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|err| format!("failed to run OCR handoff corpus runner: {err}"))?;

    let status = wait_for_ocr_handoff_builder(&mut child, OCR_HANDOFF_BUILDER_TIMEOUT)
        .map_err(|_| "OCR handoff corpus runner timed out".to_string())?;
    if !status.success() {
        return Err("OCR handoff corpus runner failed".to_string());
    }

    let report_metadata = fs::metadata(&args.report_path)
        .map_err(|err| format!("failed to inspect OCR handoff corpus report: {err}"))?;
    if report_metadata.len() > OCR_RUNNER_STDOUT_MAX_BYTES as u64 {
        return Err("OCR handoff corpus report exceeded limit".to_string());
    }
    let report_text = fs::read_to_string(&args.report_path)
        .map_err(|err| format!("failed to read OCR handoff corpus report: {err}"))?;
    reject_ocr_handoff_corpus_phi_sentinels(&report_text)?;
    let value: Value = serde_json::from_str(&report_text)
        .map_err(|_| "OCR handoff corpus report is not valid JSON".to_string())?;
    validate_ocr_handoff_corpus_report(&value)?;

    if let Some(summary_output) = &args.summary_output {
        let summary_artifact = build_ocr_handoff_corpus_summary(&value);
        let summary_text = format!(
            "{}\n",
            serde_json::to_string_pretty(&summary_artifact)
                .map_err(|err| format!("failed to render OCR handoff corpus summary: {err}"))?
        );
        if summary_text.len() > OCR_RUNNER_STDOUT_MAX_BYTES {
            return Err("OCR handoff corpus summary exceeded limit".to_string());
        }
        fs::write(summary_output, summary_text)
            .map_err(|err| format!("failed to write OCR handoff corpus summary: {err}"))?;
    }

    let summary = json!({
        "command": "ocr-handoff-corpus",
        "report_path": "<redacted>",
        "engine": value["engine"],
        "scope": value["scope"],
        "fixture_count": value["fixture_count"],
        "ready_fixture_count": value["ready_fixture_count"],
        "privacy_filter_contract": value["privacy_filter_contract"],
    });
    println!(
        "{}",
        serde_json::to_string(&summary)
            .map_err(|err| format!("failed to render summary: {err}"))?
    );
    Ok(())
}

fn build_ocr_handoff_corpus_summary(value: &Value) -> Value {
    let fixture_count = value["fixture_count"].as_u64().unwrap_or(0);
    let ready_fixture_count = value["ready_fixture_count"].as_u64().unwrap_or(0);
    json!({
        "artifact": "ocr_handoff_corpus_readiness_summary",
        "candidate": "PP-OCRv5_mobile_rec",
        "engine": value["engine"],
        "scope": value["scope"],
        "privacy_filter_contract": value["privacy_filter_contract"],
        "fixture_count": value["fixture_count"],
        "ready_fixture_count": value["ready_fixture_count"],
        "all_fixtures_ready_for_text_pii_eval": fixture_count > 0 && fixture_count == ready_fixture_count,
        "total_char_count": value["total_char_count"],
        "non_goals": value["non_goals"],
    })
}

fn reject_ocr_handoff_corpus_phi_sentinels(report_text: &str) -> Result<(), String> {
    for sentinel in [
        "Jane Example",
        "MRN-12345",
        "jane@example.com",
        "555-123-4567",
    ] {
        if report_text.contains(sentinel) {
            return Err("OCR handoff corpus report contains raw synthetic PHI".to_string());
        }
    }
    Ok(())
}

fn validate_ocr_handoff_corpus_report(value: &Value) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| "OCR handoff corpus report must be a JSON object".to_string())?;
    let allowed: HashSet<&str> = [
        "engine",
        "scope",
        "fixture_count",
        "ready_fixture_count",
        "total_char_count",
        "fixtures",
        "non_goals",
        "privacy_filter_contract",
    ]
    .into_iter()
    .collect();
    if object.keys().any(|key| !allowed.contains(key.as_str())) {
        return Err("OCR handoff corpus report contains unexpected top-level field".to_string());
    }
    for key in allowed {
        if !object.contains_key(key) {
            return Err("OCR handoff corpus report missing required field".to_string());
        }
    }
    for (key, expected) in [
        ("engine", "PP-OCRv5-mobile-bounded-spike"),
        ("scope", "printed_text_line_extraction_only"),
        ("privacy_filter_contract", "text_only_normalized_input"),
    ] {
        if value[key] != expected {
            return Err("OCR handoff corpus required field has unexpected value".to_string());
        }
    }
    let fixture_count = value["fixture_count"]
        .as_u64()
        .ok_or_else(|| "OCR handoff corpus report has invalid required count shape".to_string())?;
    let ready_fixture_count = value["ready_fixture_count"]
        .as_u64()
        .ok_or_else(|| "OCR handoff corpus report has invalid required count shape".to_string())?;
    value["total_char_count"]
        .as_u64()
        .ok_or_else(|| "OCR handoff corpus report has invalid required count shape".to_string())?;
    let fixtures = value["fixtures"]
        .as_array()
        .ok_or_else(|| "OCR handoff corpus report has invalid fixtures shape".to_string())?;
    if fixtures.len() as u64 != fixture_count {
        return Err("OCR handoff corpus fixture count mismatch".to_string());
    }
    if fixtures
        .iter()
        .filter(|fixture| fixture["ready_for_text_pii_eval"] == true)
        .count() as u64
        != ready_fixture_count
    {
        return Err("OCR handoff corpus ready fixture count mismatch".to_string());
    }
    for fixture in fixtures {
        let fixture_object = fixture
            .as_object()
            .ok_or_else(|| "OCR handoff corpus report has invalid fixture shape".to_string())?;
        let allowed_fixture: HashSet<&str> = ["id", "char_count", "ready_for_text_pii_eval"]
            .into_iter()
            .collect();
        if fixture_object.len() != allowed_fixture.len()
            || fixture_object
                .keys()
                .any(|key| !allowed_fixture.contains(key.as_str()))
        {
            return Err("OCR handoff corpus fixture contains unexpected field".to_string());
        }
        let id = fixture["id"]
            .as_str()
            .ok_or_else(|| "OCR handoff corpus fixture has invalid id".to_string())?;
        if !is_fixture_ordinal_id(id)
            || fixture["char_count"].as_u64().is_none()
            || !fixture["ready_for_text_pii_eval"].is_boolean()
        {
            return Err("OCR handoff corpus fixture has invalid field shape".to_string());
        }
    }
    for expected in ["visual_redaction", "final_pdf_rewrite_export"] {
        if !value["non_goals"]
            .as_array()
            .ok_or_else(|| "OCR handoff corpus report has invalid non-goals shape".to_string())?
            .iter()
            .any(|item| item == expected)
        {
            return Err("OCR handoff corpus missing required non-goal".to_string());
        }
    }
    Ok(())
}

fn is_fixture_ordinal_id(id: &str) -> bool {
    id.len() == 11
        && id.starts_with("fixture_")
        && id[8..].chars().all(|character| character.is_ascii_digit())
}

fn run_ocr_to_privacy_filter(args: OcrToPrivacyFilterArgs) -> Result<(), String> {
    if let Some(summary_output) = &args.summary_output {
        if paths_are_same_existing_or_lexical(&args.report_path, summary_output) {
            return Err(
                "ocr_to_privacy_filter summary path must differ from report path".to_string(),
            );
        }
    }
    let _ = fs::remove_file(&args.report_path);
    if let Some(summary_output) = &args.summary_output {
        let _ = fs::remove_file(summary_output);
    }
    let result = (|| {
        require_regular_file(
            &args.image_path,
            "ocr_to_privacy_filter single-image chain failed",
        )?;
        require_regular_file(
            &args.ocr_runner_path,
            "ocr_to_privacy_filter single-image chain failed",
        )?;
        require_regular_file(
            &args.privacy_runner_path,
            "ocr_to_privacy_filter single-image chain failed",
        )?;
        run_ocr_to_privacy_filter_inner(&args)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&args.report_path);
        if let Some(summary_output) = &args.summary_output {
            let _ = fs::remove_file(summary_output);
        }
    }
    result
}

fn run_ocr_to_privacy_filter_inner(args: &OcrToPrivacyFilterArgs) -> Result<(), String> {
    let mut ocr_command = std::process::Command::new(&args.python_command);
    ocr_command.arg(&args.ocr_runner_path).arg("--json");
    if args.mock {
        ocr_command.arg("--mock");
    }
    let mut ocr_child = ocr_command
        .arg(&args.image_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|_| "ocr_to_privacy_filter single-image chain failed".to_string())?;
    let (ocr_status, ocr_stdout) = wait_for_privacy_filter_runner(
        &mut ocr_child,
        OCR_RUNNER_TIMEOUT,
        OCR_RUNNER_STDOUT_MAX_BYTES,
    )
    .map_err(|_| "ocr_to_privacy_filter single-image chain failed".to_string())?;
    if !ocr_status.success() {
        return Err("ocr_to_privacy_filter single-image chain failed".to_string());
    }
    let ocr_text = String::from_utf8(ocr_stdout)
        .map_err(|_| "ocr_to_privacy_filter single-image chain failed".to_string())?;
    let ocr_value: Value = serde_json::from_str(&ocr_text)
        .map_err(|_| "ocr_to_privacy_filter single-image chain failed".to_string())?;
    let normalized_text = validate_ocr_to_privacy_filter_ocr_json(&ocr_value)?;
    let mut privacy_command = std::process::Command::new(&args.python_command);
    privacy_command
        .arg(&args.privacy_runner_path)
        .arg("--stdin");
    if args.mock {
        privacy_command.arg("--mock");
    }
    let mut privacy_child = privacy_command
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|_| "ocr_to_privacy_filter single-image chain failed".to_string())?;
    let stdin_writer = spawn_privacy_filter_stdin_writer(
        privacy_child
            .stdin
            .take()
            .ok_or_else(|| "ocr_to_privacy_filter single-image chain failed".to_string())?,
        normalized_text.as_bytes().to_vec(),
    );
    let (privacy_status, privacy_stdout) = wait_for_privacy_filter_runner(
        &mut privacy_child,
        PRIVACY_FILTER_RUNNER_TIMEOUT,
        PRIVACY_FILTER_RUNNER_STDOUT_MAX_BYTES,
    )
    .map_err(|_| "ocr_to_privacy_filter single-image chain failed".to_string())?;
    stdin_writer
        .recv_timeout(Duration::from_secs(1))
        .map_err(|_| "ocr_to_privacy_filter single-image chain failed".to_string())?
        .map_err(|_| "ocr_to_privacy_filter single-image chain failed".to_string())?;
    if !privacy_status.success() {
        return Err("ocr_to_privacy_filter single-image chain failed".to_string());
    }
    let privacy_text = String::from_utf8(privacy_stdout)
        .map_err(|_| "ocr_to_privacy_filter single-image chain failed".to_string())?;
    let privacy_value: Value = serde_json::from_str(&privacy_text)
        .map_err(|_| "ocr_to_privacy_filter single-image chain failed".to_string())?;
    validate_privacy_filter_output(&privacy_value)
        .map_err(|_| "ocr_to_privacy_filter single-image chain failed".to_string())?;
    validate_ocr_to_privacy_filter_privacy_json(&privacy_value)?;
    let report = build_ocr_to_privacy_filter_single_report(&privacy_value);
    fs::write(
        &args.report_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&report)
                .map_err(|_| "ocr_to_privacy_filter single-image chain failed".to_string())?
        ),
    )
    .map_err(|_| "ocr_to_privacy_filter single-image chain failed".to_string())?;
    if let Some(summary_output) = &args.summary_output {
        let mut summary = report.clone();
        summary["artifact"] = Value::String("ocr_to_privacy_filter_single_summary".to_string());
        fs::write(
            summary_output,
            format!(
                "{}\n",
                serde_json::to_string_pretty(&summary)
                    .map_err(|_| "ocr_to_privacy_filter single-image chain failed".to_string())?
            ),
        )
        .map_err(|_| "ocr_to_privacy_filter single-image chain failed".to_string())?;
    }
    println!("{}", serde_json::to_string(&json!({"command":"ocr-to-privacy-filter","report_path":"...","artifact":"ocr_to_privacy_filter_single","network_api_called":false,"privacy_filter_detected_span_count":report["privacy_filter_detected_span_count"]})).map_err(|_| "ocr_to_privacy_filter single-image chain failed".to_string())?);
    Ok(())
}

fn validate_ocr_to_privacy_filter_ocr_json(value: &Value) -> Result<&str, String> {
    if value["scope"] != "printed_text_line_extraction_only"
        || value["ready_for_text_pii_eval"] != true
    {
        return Err("ocr_to_privacy_filter single-image chain failed".to_string());
    }
    value["normalized_text"]
        .as_str()
        .filter(|text| !text.is_empty())
        .ok_or_else(|| "ocr_to_privacy_filter single-image chain failed".to_string())
}

fn validate_ocr_to_privacy_filter_privacy_json(value: &Value) -> Result<(), String> {
    let network_api_called =
        value["metadata"]["network_api_called"] == false || value["network_api_called"] == false;
    let engine = value["metadata"]["engine"]
        .as_str()
        .filter(|engine| *engine == "fallback_synthetic_patterns");
    if !network_api_called
        || engine.is_none()
        || value["summary"]["detected_span_count"].as_u64().is_none()
        || !validate_ocr_privacy_category_counts(&value["summary"]["category_counts"])
    {
        return Err("ocr_to_privacy_filter single-image chain failed".to_string());
    }
    Ok(())
}

fn build_ocr_to_privacy_filter_single_report(privacy_value: &Value) -> Value {
    json!({"artifact":"ocr_to_privacy_filter_single","ocr_candidate":"PP-OCRv5_mobile_rec","ocr_engine":"PP-OCRv5-mobile-bounded-spike","ocr_scope":"printed_text_line_extraction_only","privacy_scope":"text_only_pii_detection","privacy_filter_engine":privacy_value["metadata"]["engine"],"privacy_filter_contract":"text_only_normalized_input","ready_for_text_pii_eval":true,"network_api_called":false,"privacy_filter_detected_span_count":privacy_value["summary"]["detected_span_count"],"privacy_filter_category_counts":privacy_value["summary"]["category_counts"],"non_goals":["not_visual_redaction","not_image_pixel_redaction","not_final_pdf_rewrite_export","not_browser_or_desktop_execution","not_model_quality_evidence"]})
}

fn run_ocr_to_privacy_filter_corpus(args: OcrToPrivacyFilterCorpusArgs) -> Result<(), String> {
    let _ = fs::remove_file(&args.report_path);
    if let Some(summary_output) = &args.summary_output {
        let _ = fs::remove_file(summary_output);
    }
    let result = (|| {
        require_directory(&args.fixture_dir, "OCR to privacy filter corpus failed")?;
        require_regular_file(&args.ocr_runner_path, "OCR to privacy filter corpus failed")?;
        require_regular_file(
            &args.privacy_runner_path,
            "OCR to privacy filter corpus failed",
        )?;
        require_regular_file(
            &args.bridge_runner_path,
            "OCR to privacy filter corpus failed",
        )?;
        run_ocr_to_privacy_filter_corpus_inner(&args)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&args.report_path);
        if let Some(summary_output) = &args.summary_output {
            let _ = fs::remove_file(summary_output);
        }
    }
    result
}

fn run_ocr_to_privacy_filter_corpus_inner(
    args: &OcrToPrivacyFilterCorpusArgs,
) -> Result<(), String> {
    let mut child = std::process::Command::new(&args.python_command)
        .arg(&args.bridge_runner_path)
        .arg("--fixture-dir")
        .arg(&args.fixture_dir)
        .arg("--ocr-runner-path")
        .arg(&args.ocr_runner_path)
        .arg("--privacy-runner-path")
        .arg(&args.privacy_runner_path)
        .arg("--output")
        .arg(&args.report_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|_| "OCR to privacy filter corpus failed".to_string())?;

    let status = wait_for_ocr_handoff_builder(&mut child, OCR_HANDOFF_BUILDER_TIMEOUT)
        .map_err(|_| "OCR to privacy filter corpus failed".to_string())?;
    if !status.success() {
        return Err("OCR to privacy filter corpus failed".to_string());
    }

    let report_metadata = fs::metadata(&args.report_path)
        .map_err(|_| "OCR to privacy filter corpus failed".to_string())?;
    if report_metadata.len() > PRIVACY_FILTER_RUNNER_STDOUT_MAX_BYTES as u64 {
        return Err("OCR to privacy filter corpus report exceeded limit".to_string());
    }
    let report_text = fs::read_to_string(&args.report_path)
        .map_err(|_| "OCR to privacy filter corpus failed".to_string())?;
    let value: Value = serde_json::from_str(&report_text)
        .map_err(|_| "OCR to privacy filter corpus report is not valid JSON".to_string())?;
    validate_ocr_to_privacy_filter_corpus_report(&value, &report_text)
        .map_err(|err| format!("invalid OCR to privacy filter corpus report: {err}"))?;
    let wrapper_report = normalize_ocr_to_privacy_filter_corpus_report(&value)?;
    fs::write(
        &args.report_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&wrapper_report)
                .map_err(|err| format!("failed to render report: {err}"))?
        ),
    )
    .map_err(|_| "OCR to privacy filter corpus failed".to_string())?;

    if let Some(summary_output) = &args.summary_output {
        let summary_report = build_ocr_to_privacy_filter_corpus_summary(&wrapper_report);
        let rendered_summary = format!(
            "{}\n",
            serde_json::to_string_pretty(&summary_report)
                .map_err(|err| format!("failed to render summary output: {err}"))?
        );
        if rendered_summary.len() > PRIVACY_FILTER_RUNNER_STDOUT_MAX_BYTES {
            return Err("OCR to privacy filter corpus summary exceeded limit".to_string());
        }
        fs::write(summary_output, rendered_summary)
            .map_err(|_| "OCR to privacy filter corpus failed".to_string())?;
    }

    let summary = json!({
        "command": "ocr-to-privacy-filter-corpus",
        "report_path": "<redacted>",
        "artifact": wrapper_report["artifact"],
        "ocr_engine": wrapper_report["ocr_engine"],
        "privacy_filter_engine": wrapper_report["privacy_filter_engine"],
        "ocr_scope": wrapper_report["ocr_scope"],
        "privacy_scope": wrapper_report["privacy_scope"],
        "fixture_count": wrapper_report["fixture_count"],
        "ready_fixture_count": wrapper_report["ready_fixture_count"],
        "total_detected_span_count": wrapper_report["total_detected_span_count"],
        "privacy_filter_contract": wrapper_report["privacy_filter_contract"],
        "network_api_called": wrapper_report["network_api_called"],
    });
    println!(
        "{}",
        serde_json::to_string(&summary)
            .map_err(|err| format!("failed to render summary: {err}"))?
    );
    Ok(())
}

fn build_ocr_to_privacy_filter_corpus_summary(value: &Value) -> Value {
    json!({
        "artifact": "ocr_to_privacy_filter_corpus_summary",
        "ocr_candidate": value["ocr_candidate"],
        "ocr_engine": value["ocr_engine"],
        "ocr_scope": value["ocr_scope"],
        "privacy_filter_engine": value["privacy_filter_engine"],
        "privacy_filter_contract": value["privacy_filter_contract"],
        "privacy_scope": value["privacy_scope"],
        "fixture_count": value["fixture_count"],
        "ready_fixture_count": value["ready_fixture_count"],
        "total_detected_span_count": value["total_detected_span_count"],
        "category_counts": value["category_counts"],
        "privacy_filter_category_counts": value["privacy_filter_category_counts"],
        "network_api_called": false,
        "non_goals": value["non_goals"],
    })
}

fn normalize_ocr_to_privacy_filter_corpus_report(value: &Value) -> Result<Value, String> {
    Ok(json!({
        "artifact": "ocr_to_privacy_filter_corpus",
        "ocr_candidate": value["ocr_candidate"],
        "ocr_engine": value["ocr_engine"],
        "ocr_scope": "printed_text_line_extraction_only",
        "privacy_filter_engine": value["privacy_filter_engine"],
        "privacy_filter_contract": value["privacy_filter_contract"],
        "privacy_scope": "text_only_pii_detection",
        "fixture_count": value["fixture_count"],
        "ready_fixture_count": value["ready_fixture_count"],
        "total_detected_span_count": value["privacy_filter_detected_span_count"],
        "category_counts": value["category_counts"],
        "privacy_filter_category_counts": value["privacy_filter_category_counts"],
        "fixtures": value["fixtures"],
        "non_goals": value["non_goals"],
        "network_api_called": false,
    }))
}

fn validate_ocr_to_privacy_filter_corpus_report(
    value: &Value,
    report_text: &str,
) -> Result<(), String> {
    for unsafe_text in [
        "Jane Example",
        "MRN-12345",
        "jane@example.com",
        "555-123-4567",
        "synthetic_patient_label_",
        "/home/",
        "/tmp/",
        "/var/",
        "/Users/",
        "fixtures/",
        "../",
        "./",
        "\\\\",
    ] {
        if report_text.contains(unsafe_text) {
            return Err("report contains unsafe output".to_string());
        }
    }
    if contains_unsafe_string_value(value) {
        return Err("report contains unsafe output".to_string());
    }
    let object = value
        .as_object()
        .ok_or_else(|| "report must be a JSON object".to_string())?;
    let allowed_keys = [
        "artifact",
        "ocr_candidate",
        "ocr_engine",
        "scope",
        "privacy_filter_engine",
        "privacy_filter_contract",
        "fixture_count",
        "ready_fixture_count",
        "privacy_filter_detected_span_count",
        "category_counts",
        "privacy_filter_category_counts",
        "fixtures",
        "non_goals",
        "network_api_called",
    ];
    for key in object.keys() {
        if !allowed_keys.contains(&key.as_str()) || is_unsafe_aggregate_field_name(key) {
            return Err("report contains unexpected field".to_string());
        }
    }
    for key in allowed_keys
        .iter()
        .filter(|key| **key != "network_api_called")
    {
        if !object.contains_key(*key) {
            return Err("report missing required field".to_string());
        }
    }
    if object
        .get("network_api_called")
        .is_some_and(|network_api_called| network_api_called != false)
    {
        return Err("report cannot call network APIs".to_string());
    }
    if value["artifact"] != "ocr_to_privacy_filter_corpus_bridge"
        || value["ocr_candidate"] != "PP-OCRv5_mobile_rec"
        || value["ocr_engine"] != "PP-OCRv5-mobile-bounded-spike"
        || value["scope"] != "printed_text_extraction_to_text_pii_detection_only"
        || value["privacy_filter_contract"] != "text_only_normalized_input"
    {
        return Err("report required field has unexpected value".to_string());
    }
    let fixture_count = value["fixture_count"]
        .as_u64()
        .ok_or_else(|| "invalid count".to_string())?;
    let ready_fixture_count = value["ready_fixture_count"]
        .as_u64()
        .ok_or_else(|| "invalid count".to_string())?;
    value["privacy_filter_detected_span_count"]
        .as_u64()
        .ok_or_else(|| "invalid count".to_string())?;
    if !validate_ocr_privacy_category_counts(&value["category_counts"])
        || !validate_ocr_privacy_category_counts(&value["privacy_filter_category_counts"])
    {
        return Err("invalid category counts".to_string());
    }
    let fixtures = value["fixtures"]
        .as_array()
        .ok_or_else(|| "invalid fixtures".to_string())?;
    if fixtures.len() as u64 != fixture_count
        || fixtures
            .iter()
            .filter(|fixture| fixture["ready_for_text_pii_eval"] == true)
            .count() as u64
            != ready_fixture_count
    {
        return Err("fixture count mismatch".to_string());
    }
    for fixture in fixtures {
        let fixture_object = fixture
            .as_object()
            .ok_or_else(|| "invalid fixture".to_string())?;
        let allowed_fixture_keys = ["fixture", "ready_for_text_pii_eval", "detected_span_count"];
        for key in fixture_object.keys() {
            if !allowed_fixture_keys.contains(&key.as_str()) || is_unsafe_aggregate_field_name(key)
            {
                return Err("fixture contains unexpected field".to_string());
            }
        }
        let id = fixture["fixture"]
            .as_str()
            .ok_or_else(|| "invalid fixture id".to_string())?;
        if !is_fixture_ordinal_id(id)
            || !fixture["ready_for_text_pii_eval"].is_boolean()
            || fixture["detected_span_count"].as_u64().is_none()
        {
            return Err("invalid fixture shape".to_string());
        }
    }
    let allowed_non_goals = [
        "visual_redaction",
        "image_pixel_redaction",
        "final_pdf_rewrite_export",
        "handwriting_recognition",
        "browser_ui",
        "desktop_ui",
    ];
    if !value["non_goals"]
        .as_array()
        .ok_or_else(|| "invalid non-goals".to_string())?
        .iter()
        .all(|item| {
            item.as_str()
                .is_some_and(|item| allowed_non_goals.contains(&item))
        })
    {
        return Err("invalid non-goal".to_string());
    }
    Ok(())
}

fn is_unsafe_aggregate_field_name(key: &str) -> bool {
    matches!(
        key,
        "raw_text" | "masked_text" | "spans" | "preview" | "extracted_text" | "normalized_text"
    )
}

fn contains_unsafe_string_value(value: &Value) -> bool {
    match value {
        Value::String(text) => is_unsafe_report_string(text),
        Value::Array(items) => items.iter().any(contains_unsafe_string_value),
        Value::Object(object) => object.values().any(contains_unsafe_string_value),
        _ => false,
    }
}

fn is_unsafe_report_string(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    text.contains("Jane Example")
        || text.contains("MRN-12345")
        || lower.contains("jane@example.com")
        || text.contains("555-123-4567")
        || text.contains("synthetic_patient_label_")
        || looks_like_path_like_report_string(text)
}

fn looks_like_path_like_report_string(text: &str) -> bool {
    if is_allowed_non_goal_token(text) {
        return false;
    }
    text.starts_with('/')
        || text.starts_with("./")
        || text.starts_with("../")
        || text.contains('\\')
        || has_windows_drive_prefix(text)
        || has_relative_filename_path_segment(text)
}

fn is_allowed_non_goal_token(text: &str) -> bool {
    matches!(
        text,
        "visual_redaction"
            | "image_pixel_redaction"
            | "final_pdf_rewrite_export"
            | "handwriting_recognition"
            | "browser_ui"
            | "desktop_ui"
    )
}

fn has_relative_filename_path_segment(text: &str) -> bool {
    text.contains('/')
        && text
            .split('/')
            .any(|segment| has_filename_like_extension(segment))
}

fn has_filename_like_extension(segment: &str) -> bool {
    let Some((stem, extension)) = segment.rsplit_once('.') else {
        return false;
    };
    !stem.is_empty()
        && !extension.is_empty()
        && extension
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
}

fn has_windows_drive_prefix(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
}

fn validate_ocr_privacy_category_counts(value: &Value) -> bool {
    let Some(counts) = value.as_object() else {
        return false;
    };
    let allowed_labels = ["NAME", "MRN", "EMAIL", "PHONE", "ID"];
    counts
        .iter()
        .all(|(label, count)| allowed_labels.contains(&label.as_str()) && count.as_u64().is_some())
}

fn run_ocr_privacy_evidence(args: OcrPrivacyEvidenceArgs) -> Result<(), String> {
    let _ = fs::remove_file(&args.output_path);
    let result = (|| {
        require_regular_file(&args.image_path, "OCR privacy evidence failed")?;
        require_regular_file(&args.runner_path, "OCR privacy evidence failed")?;
        let mut command = std::process::Command::new(&args.python_command);
        command
            .arg(&args.runner_path)
            .arg("--image-path")
            .arg(&args.image_path)
            .arg("--output")
            .arg(&args.output_path);
        if let Some(parent) = args.runner_path.parent() {
            let ocr_runner = parent.join("run_small_ocr.py");
            let privacy_runner = parent
                .parent()
                .unwrap_or(parent)
                .join("privacy_filter/run_privacy_filter.py");
            if ocr_runner.is_file() && privacy_runner.is_file() {
                command.arg("--ocr-runner-path").arg(ocr_runner);
                command.arg("--privacy-runner-path").arg(privacy_runner);
            }
        }
        if args.mock {
            command.arg("--mock");
        }
        let mut child = command
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|_| "OCR privacy evidence failed".to_string())?;
        let status = wait_for_ocr_handoff_builder(&mut child, OCR_HANDOFF_BUILDER_TIMEOUT)
            .map_err(|_| "OCR privacy evidence failed".to_string())?;
        if !status.success() {
            return Err("OCR privacy evidence failed".to_string());
        }
        let metadata = fs::metadata(&args.output_path)
            .map_err(|_| "OCR privacy evidence failed".to_string())?;
        if metadata.len() > OCR_PRIVACY_EVIDENCE_REPORT_MAX_BYTES as u64 {
            return Err("OCR privacy evidence failed".to_string());
        }
        let report_text = fs::read_to_string(&args.output_path)
            .map_err(|_| "OCR privacy evidence failed".to_string())?;
        let value: Value = serde_json::from_str(&report_text)
            .map_err(|_| "OCR privacy evidence failed".to_string())?;
        validate_ocr_privacy_evidence_report(&value, &report_text)?;
        let rendered = format!(
            "{}\n",
            serde_json::to_string_pretty(&value)
                .map_err(|_| "OCR privacy evidence failed".to_string())?
        );
        fs::write(&args.output_path, rendered)
            .map_err(|_| "OCR privacy evidence failed".to_string())?;
        println!("{}", serde_json::to_string(&json!({"command":"ocr-privacy-evidence","artifact":"ocr_privacy_evidence","report_path":"<redacted>","report_written":true,"ocr_scope":"printed_text_line_extraction_only","privacy_scope":"text_only_pii_detection","network_api_called":false})).map_err(|_| "OCR privacy evidence failed".to_string())?);
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&args.output_path);
    }
    result
}

fn validate_ocr_privacy_evidence_report(value: &Value, report_text: &str) -> Result<(), String> {
    if contains_unsafe_string_value(value)
        || [
            "Jane Example",
            "MRN-12345",
            "jane@example.com",
            "555-123-4567",
            "synthetic_printed_phi_line.png",
            "run_ocr_privacy_evidence.py",
            "/tmp/",
            "/home/",
            "\\\\",
        ]
        .iter()
        .any(|text| report_text.contains(text))
    {
        return Err("OCR privacy evidence failed".to_string());
    }
    let object = value
        .as_object()
        .ok_or_else(|| "OCR privacy evidence failed".to_string())?;
    let allowed = [
        "artifact",
        "ocr_candidate",
        "ocr_engine",
        "ocr_scope",
        "ocr_engine_status",
        "privacy_filter_engine",
        "privacy_filter_contract",
        "privacy_scope",
        "ready_for_text_pii_eval",
        "network_api_called",
        "detected_span_count",
        "category_counts",
        "non_goals",
    ];
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) || is_unsafe_aggregate_field_name(key) {
            return Err("OCR privacy evidence failed".to_string());
        }
    }
    for key in allowed {
        if !object.contains_key(key) {
            return Err("OCR privacy evidence failed".to_string());
        }
    }
    if value["artifact"] != "ocr_privacy_evidence"
        || value["ocr_scope"] != "printed_text_line_extraction_only"
        || value["privacy_scope"] != "text_only_pii_detection"
        || value["network_api_called"] != false
        || value["ready_for_text_pii_eval"] != true
        || value["privacy_filter_contract"] != "text_only_normalized_input"
    {
        return Err("OCR privacy evidence failed".to_string());
    }
    value["detected_span_count"]
        .as_u64()
        .ok_or_else(|| "OCR privacy evidence failed".to_string())?;
    if !validate_ocr_privacy_category_counts(&value["category_counts"]) {
        return Err("OCR privacy evidence failed".to_string());
    }
    let allowed_non_goals = [
        "browser_ui",
        "complete_ocr_pipeline",
        "desktop_ui",
        "final_pdf_rewrite_export",
        "handwriting_recognition",
        "image_pixel_redaction",
        "visual_redaction",
    ];
    if !value["non_goals"].as_array().is_some_and(|items| {
        items.iter().all(|item| {
            item.as_str()
                .is_some_and(|item| allowed_non_goals.contains(&item))
        })
    }) {
        return Err("OCR privacy evidence failed".to_string());
    }
    Ok(())
}

fn run_ocr_small_json(args: OcrSmallJsonArgs) -> Result<(), String> {
    if let Some(summary_output) = &args.summary_output {
        if paths_are_same_existing_or_lexical(&args.report_path, summary_output) {
            return Err("OCR small JSON summary path must differ from report path".to_string());
        }
    }
    let _ = fs::remove_file(&args.report_path);
    if let Some(summary_output) = &args.summary_output {
        let _ = fs::remove_file(summary_output);
    }
    let result = (|| {
        require_regular_file(&args.image_path, "OCR small JSON failed")?;
        require_regular_file(&args.ocr_runner_path, "OCR small JSON failed")?;
        run_ocr_small_json_inner(&args)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&args.report_path);
        if let Some(summary_output) = &args.summary_output {
            let _ = fs::remove_file(summary_output);
        }
    }
    result
}

fn paths_are_same_existing_or_lexical(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }
    if let (Ok(left), Ok(right)) = (fs::canonicalize(left), fs::canonicalize(right)) {
        return left == right;
    }

    let (Some(left_file_name), Some(right_file_name)) = (left.file_name(), right.file_name())
    else {
        return false;
    };
    if !path_file_names_same_lexical(left_file_name, right_file_name) {
        return false;
    }
    match (
        canonicalize_parent_or_current(left),
        canonicalize_parent_or_current(right),
    ) {
        (Ok(left_parent), Ok(right_parent)) => left_parent == right_parent,
        _ => false,
    }
}

fn canonicalize_parent_or_current(path: &Path) -> std::io::Result<PathBuf> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::canonicalize(parent)
}

fn path_file_names_same_lexical(left: &std::ffi::OsStr, right: &std::ffi::OsStr) -> bool {
    if left == right {
        return true;
    }
    cfg!(windows)
        && left
            .to_str()
            .zip(right.to_str())
            .is_some_and(|(left, right)| left.eq_ignore_ascii_case(right))
}

fn run_ocr_small_json_inner(args: &OcrSmallJsonArgs) -> Result<(), String> {
    let mut command = std::process::Command::new(&args.python_command);
    command.arg(&args.ocr_runner_path);
    if args.mock {
        command.arg("--mock");
    }
    let mut child = command
        .arg("--json")
        .arg(&args.image_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|_| "OCR small JSON failed".to_string())?;
    let (status, stdout_bytes) =
        wait_for_ocr_runner(&mut child, OCR_RUNNER_TIMEOUT, OCR_RUNNER_STDOUT_MAX_BYTES)
            .map_err(|_| "OCR small JSON failed".to_string())?;
    if !status.success() {
        return Err("OCR small JSON failed".to_string());
    }
    let report_text = String::from_utf8(stdout_bytes)
        .map_err(|_| "OCR small JSON report is not valid UTF-8".to_string())?;
    if report_text.len() > OCR_RUNNER_STDOUT_MAX_BYTES {
        return Err("OCR small JSON report exceeded limit".to_string());
    }
    let value: Value = serde_json::from_str(&report_text)
        .map_err(|_| "OCR small JSON report is not valid JSON".to_string())?;
    validate_ocr_small_json_report(&value)?;
    let rendered = format!(
        "{}\n",
        serde_json::to_string_pretty(&value)
            .map_err(|err| format!("failed to render OCR small JSON report: {err}"))?
    );
    fs::write(&args.report_path, rendered).map_err(|_| "OCR small JSON failed".to_string())?;

    if let Some(summary_output) = &args.summary_output {
        let summary_artifact = build_ocr_small_json_summary(&value);
        let summary_text = format!(
            "{}\n",
            serde_json::to_string_pretty(&summary_artifact)
                .map_err(|err| format!("failed to render OCR small JSON summary: {err}"))?
        );
        if summary_text.len() > OCR_RUNNER_STDOUT_MAX_BYTES {
            return Err("OCR small JSON summary exceeded limit".to_string());
        }
        fs::write(summary_output, summary_text).map_err(|_| "OCR small JSON failed".to_string())?;
    }

    let mut summary = json!({
        "command": "ocr-small-json",
        "report_written": true,
        "report_path": "<redacted>",
        "candidate": value["candidate"],
        "scope": value["scope"],
        "ready_for_text_pii_eval": value["ready_for_text_pii_eval"],
    });
    if args.summary_output.is_some() {
        summary["summary_written"] = Value::Bool(true);
    }
    println!(
        "{}",
        serde_json::to_string(&summary)
            .map_err(|err| format!("failed to render summary: {err}"))?
    );
    Ok(())
}

fn build_ocr_small_json_summary(value: &Value) -> Value {
    json!({
        "artifact": "ocr_small_json_summary",
        "candidate": "PP-OCRv5_mobile_rec",
        "engine": "PP-OCRv5-mobile-bounded-spike",
        "engine_status": value["engine_status"],
        "scope": "printed_text_line_extraction_only",
        "privacy_filter_contract": "text_only_normalized_input",
        "ready_for_text_pii_eval": true,
        "non_goals": [
            "visual_redaction",
            "image_pixel_redaction",
            "final_pdf_rewrite_export",
            "handwriting_recognition",
            "full_page_detection_or_segmentation",
            "complete_ocr_pipeline",
            "browser_ui",
            "desktop_ui",
        ],
    })
}

fn validate_ocr_small_json_report(value: &Value) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| "invalid OCR small JSON report".to_string())?;
    let allowed_keys = [
        "candidate",
        "engine",
        "engine_status",
        "scope",
        "source",
        "extracted_text",
        "normalized_text",
        "ready_for_text_pii_eval",
        "privacy_filter_contract",
        "non_goals",
    ];
    for key in object.keys() {
        if !allowed_keys.contains(&key.as_str()) {
            return Err("invalid OCR small JSON report".to_string());
        }
    }
    for key in [
        "candidate",
        "engine",
        "scope",
        "privacy_filter_contract",
        "engine_status",
        "ready_for_text_pii_eval",
        "extracted_text",
        "normalized_text",
        "non_goals",
    ] {
        if !object.contains_key(key) {
            return Err("invalid OCR small JSON report".to_string());
        }
    }
    for (key, expected) in [
        ("candidate", "PP-OCRv5_mobile_rec"),
        ("engine", "PP-OCRv5-mobile-bounded-spike"),
        ("scope", "printed_text_line_extraction_only"),
        ("privacy_filter_contract", "text_only_normalized_input"),
    ] {
        if value[key] != expected {
            return Err("invalid OCR small JSON report".to_string());
        }
    }
    let engine_status = value["engine_status"]
        .as_str()
        .ok_or_else(|| "invalid OCR small JSON report".to_string())?;
    if ![
        "deterministic_synthetic_fixture_fallback",
        "local_paddleocr_execution",
    ]
    .contains(&engine_status)
    {
        return Err("invalid OCR small JSON report".to_string());
    }
    if value["ready_for_text_pii_eval"] != true
        || !value["extracted_text"].is_string()
        || !value["normalized_text"].is_string()
    {
        return Err("invalid OCR small JSON report".to_string());
    }
    let non_goals = value["non_goals"]
        .as_array()
        .ok_or_else(|| "invalid OCR small JSON report".to_string())?;
    for expected in [
        "visual_redaction",
        "final_pdf_rewrite_export",
        "handwriting_recognition",
        "full_page_detection_or_segmentation",
        "complete_ocr_pipeline",
    ] {
        if !non_goals.iter().any(|item| item == expected) {
            return Err("invalid OCR small JSON report".to_string());
        }
    }
    Ok(())
}

fn run_ocr_handoff(args: OcrHandoffArgs) -> Result<(), String> {
    require_regular_file(&args.image_path, "missing image file")?;
    require_regular_file(&args.ocr_runner_path, "missing OCR runner file")?;
    require_regular_file(&args.handoff_builder_path, "missing handoff builder file")?;

    let _ = fs::remove_file(&args.report_path);
    let temp_path = ocr_temp_path(&args.report_path);
    let _ = fs::remove_file(&temp_path);

    let mut child = std::process::Command::new(&args.python_command)
        .arg(&args.ocr_runner_path)
        .arg("--mock")
        .arg(&args.image_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|err| format!("failed to run OCR runner: {err}"))?;
    let (status, stdout_bytes) =
        wait_for_ocr_runner(&mut child, OCR_RUNNER_TIMEOUT, OCR_RUNNER_STDOUT_MAX_BYTES).map_err(
            |err| {
                let _ = fs::remove_file(&args.report_path);
                let _ = fs::remove_file(&temp_path);
                err
            },
        )?;
    if !status.success() {
        let _ = fs::remove_file(&args.report_path);
        let _ = fs::remove_file(&temp_path);
        return Err("OCR runner failed".to_string());
    }
    let ocr_text = String::from_utf8(stdout_bytes).map_err(|_| {
        let _ = fs::remove_file(&args.report_path);
        let _ = fs::remove_file(&temp_path);
        "OCR runner returned non-UTF-8 output".to_string()
    })?;

    fs::write(&temp_path, ocr_text).map_err(|err| {
        let _ = fs::remove_file(&args.report_path);
        let _ = fs::remove_file(&temp_path);
        format!("failed to write OCR temp text: {err}")
    })?;
    let mut builder_child = std::process::Command::new(&args.python_command)
        .arg(&args.handoff_builder_path)
        .arg("--source")
        .arg(&args.image_path)
        .arg("--input")
        .arg(&temp_path)
        .arg("--output")
        .arg(&args.report_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|err| {
            let _ = fs::remove_file(&args.report_path);
            let _ = fs::remove_file(&temp_path);
            format!("failed to run OCR handoff builder: {err}")
        })?;
    let builder_status =
        wait_for_ocr_handoff_builder(&mut builder_child, OCR_HANDOFF_BUILDER_TIMEOUT).map_err(
            |err| {
                let _ = fs::remove_file(&args.report_path);
                let _ = fs::remove_file(&temp_path);
                err
            },
        )?;
    let _ = fs::remove_file(&temp_path);
    if !builder_status.success() {
        let _ = fs::remove_file(&args.report_path);
        return Err("OCR handoff builder failed".to_string());
    }

    let report_text = fs::read_to_string(&args.report_path).map_err(|err| {
        let _ = fs::remove_file(&args.report_path);
        format!("failed to read OCR handoff report: {err}")
    })?;
    let value: Value = serde_json::from_str(&report_text).map_err(|_| {
        let _ = fs::remove_file(&args.report_path);
        "OCR handoff report is not valid JSON".to_string()
    })?;
    if let Err(error) = validate_ocr_handoff(&value) {
        let _ = fs::remove_file(&args.report_path);
        return Err(error);
    }
    let summary = json!({
        "command": "ocr-handoff",
        "report_path": args.report_path,
        "ready_for_text_pii_eval": value["ready_for_text_pii_eval"],
        "privacy_filter_contract": value["privacy_filter_contract"],
    });
    println!(
        "{}",
        serde_json::to_string(&summary)
            .map_err(|err| format!("failed to render summary: {err}"))?
    );
    Ok(())
}

fn ocr_temp_path(report_path: &Path) -> PathBuf {
    let mut path = report_path.to_path_buf().into_os_string();
    path.push(".ocr-text.tmp");
    PathBuf::from(path)
}

fn wait_for_ocr_handoff_builder(
    child: &mut std::process::Child,
    timeout: Duration,
) -> Result<std::process::ExitStatus, String> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|err| format!("failed to wait for OCR handoff builder: {err}"))?
        {
            return Ok(status);
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err("OCR handoff builder timed out".to_string());
        }
        thread::sleep(Duration::from_millis(25));
    }
}

fn validate_ocr_handoff(value: &Value) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| "OCR handoff must be a JSON object".to_string())?;
    for key in [
        "source",
        "extracted_text",
        "normalized_text",
        "ready_for_text_pii_eval",
        "candidate",
        "engine",
        "scope",
        "privacy_filter_contract",
        "non_goals",
    ] {
        if !object.contains_key(key) {
            return Err("OCR handoff missing required field".to_string());
        }
    }
    if !value["source"].is_string()
        || !value["extracted_text"].is_string()
        || !value["normalized_text"].is_string()
        || !value["ready_for_text_pii_eval"].is_boolean()
        || !value["non_goals"].is_array()
    {
        return Err("OCR handoff has invalid required field shape".to_string());
    }
    for (key, expected) in [
        ("candidate", "PP-OCRv5_mobile_rec"),
        ("engine", "PP-OCRv5-mobile-bounded-spike"),
        ("scope", "printed_text_line_extraction_only"),
        ("privacy_filter_contract", "text_only_normalized_input"),
    ] {
        if value[key] != expected {
            return Err("OCR handoff required field has unexpected value".to_string());
        }
    }
    for expected in [
        "visual_redaction",
        "final_pdf_rewrite_export",
        "handwriting_recognition",
        "full_page_detection_or_segmentation",
        "complete_ocr_pipeline",
    ] {
        if !value["non_goals"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item == expected)
        {
            return Err("OCR handoff missing required non-goal".to_string());
        }
    }
    Ok(())
}

fn wait_for_ocr_runner(
    child: &mut std::process::Child,
    timeout: Duration,
    max_stdout_bytes: usize,
) -> Result<(std::process::ExitStatus, Vec<u8>), String> {
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "failed to read OCR runner output".to_string())?;
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut stdout_bytes = Vec::new();
        let result = stdout
            .take((max_stdout_bytes + 1) as u64)
            .read_to_end(&mut stdout_bytes)
            .map(|_| stdout_bytes)
            .map_err(|err| format!("failed to read OCR runner output: {err}"));
        let _ = tx.send(result);
    });

    let deadline = Instant::now() + timeout;
    let mut captured_stdout: Option<Vec<u8>> = None;
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|err| format!("failed to wait for OCR runner: {err}"))?
        {
            break status;
        }
        if captured_stdout.is_none() {
            if let Ok(read_result) = rx.try_recv() {
                let stdout_bytes = read_result?;
                if stdout_bytes.len() > max_stdout_bytes {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err("OCR runner output exceeded limit".to_string());
                }
                captured_stdout = Some(stdout_bytes);
            }
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err("OCR runner timed out".to_string());
        }
        thread::sleep(Duration::from_millis(25));
    };

    let stdout_bytes = match captured_stdout {
        Some(stdout_bytes) => stdout_bytes,
        None => rx
            .recv_timeout(Duration::from_secs(1))
            .map_err(|_| "failed to read OCR runner output".to_string())??,
    };
    if stdout_bytes.len() > max_stdout_bytes {
        return Err("OCR runner output exceeded limit".to_string());
    }
    Ok((status, stdout_bytes))
}

fn run_privacy_filter_corpus(args: PrivacyFilterCorpusArgs) -> Result<(), String> {
    if let Some(summary_output) = &args.summary_output {
        if paths_are_same_existing_or_lexical(&args.report_path, summary_output) {
            return Err(
                "privacy filter corpus summary path must differ from report path".to_string(),
            );
        }
    }
    let _ = fs::remove_file(&args.report_path);
    if let Some(summary_output) = &args.summary_output {
        let _ = fs::remove_file(summary_output);
    }

    let result = (|| {
        require_directory(&args.fixture_dir, "missing fixture directory")?;
        require_regular_file(&args.runner_path, "missing runner file")?;
        run_privacy_filter_corpus_inner(&args)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&args.report_path);
        if let Some(summary_output) = &args.summary_output {
            let _ = fs::remove_file(summary_output);
        }
    }
    result
}

fn run_privacy_filter_corpus_inner(args: &PrivacyFilterCorpusArgs) -> Result<(), String> {
    let is_canonical_fixture_dir = is_canonical_privacy_filter_corpus_dir(&args.fixture_dir);
    let mut child = std::process::Command::new(&args.python_command)
        .arg(&args.runner_path)
        .arg("--fixture-dir")
        .arg(&args.fixture_dir)
        .arg("--output")
        .arg(&args.report_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|err| format!("failed to run privacy filter corpus runner: {err}"))?;

    let status = wait_for_ocr_handoff_builder(&mut child, PRIVACY_FILTER_RUNNER_TIMEOUT)
        .map_err(|_| "privacy filter corpus runner timed out".to_string())?;
    if !status.success() {
        return Err("privacy filter corpus runner failed".to_string());
    }

    let report_metadata = fs::metadata(&args.report_path)
        .map_err(|err| format!("failed to inspect privacy filter corpus report: {err}"))?;
    if report_metadata.len() > PRIVACY_FILTER_RUNNER_STDOUT_MAX_BYTES as u64 {
        return Err("privacy filter corpus report exceeded limit".to_string());
    }
    let report_text = fs::read_to_string(&args.report_path)
        .map_err(|err| format!("failed to read privacy filter corpus report: {err}"))?;
    let mut value: Value = serde_json::from_str(&report_text)
        .map_err(|_| "privacy filter corpus report is not valid JSON".to_string())?;
    sanitize_privacy_filter_corpus_fixture_ids(&mut value)?;
    let report_text = serde_json::to_string_pretty(&value)
        .map_err(|err| format!("failed to render privacy filter corpus report: {err}"))?;
    let report_text_with_newline = format!("{report_text}\n");
    if report_text_with_newline.len() > PRIVACY_FILTER_RUNNER_STDOUT_MAX_BYTES {
        return Err("privacy filter corpus report exceeded limit".to_string());
    }
    fs::write(&args.report_path, report_text_with_newline)
        .map_err(|err| format!("failed to write privacy filter corpus report: {err}"))?;
    validate_privacy_filter_corpus_report(&value, &report_text, is_canonical_fixture_dir)
        .map_err(|err| format!("invalid privacy filter corpus report: {err}"))?;

    if let Some(summary_output) = &args.summary_output {
        let summary = build_privacy_filter_corpus_summary(&value);
        let summary_text = serde_json::to_string_pretty(&summary)
            .map_err(|err| format!("failed to render privacy filter corpus summary: {err}"))?;
        let rendered_summary = format!("{summary_text}\n");
        if rendered_summary.len() > PRIVACY_FILTER_RUNNER_STDOUT_MAX_BYTES {
            return Err("privacy filter corpus summary exceeded limit".to_string());
        }
        fs::write(summary_output, rendered_summary)
            .map_err(|err| format!("failed to write privacy filter corpus summary: {err}"))?;
    }

    let summary = json!({
        "command": "privacy-filter-corpus",
        "report_path": "<redacted>",
        "engine": value["engine"],
        "scope": value["scope"],
        "fixture_count": value["fixture_count"],
        "total_detected_span_count": value["total_detected_span_count"],
    });
    println!(
        "{}",
        serde_json::to_string(&summary)
            .map_err(|err| format!("failed to render summary: {err}"))?
    );
    Ok(())
}

fn build_privacy_filter_corpus_summary(value: &Value) -> Value {
    json!({
        "artifact": "privacy_filter_corpus_summary",
        "engine": value["engine"],
        "scope": value["scope"],
        "fixture_count": value["fixture_count"],
        "total_detected_span_count": value["total_detected_span_count"],
        "category_counts": value["category_counts"],
        "network_api_called": false,
        "non_goals": value["non_goals"],
    })
}

fn sanitize_privacy_filter_corpus_fixture_ids(value: &mut Value) -> Result<(), String> {
    let fixtures = value["fixtures"].as_array_mut().ok_or_else(|| {
        "privacy filter corpus report has invalid required field shape".to_string()
    })?;
    for (index, fixture) in fixtures.iter_mut().enumerate() {
        let fixture_object = fixture
            .as_object_mut()
            .ok_or_else(|| "privacy filter corpus report has invalid fixture shape".to_string())?;
        fixture_object.insert(
            "fixture".to_string(),
            Value::String(format!("fixture_{:03}", index + 1)),
        );
    }
    Ok(())
}

fn require_directory(path: &Path, message: &str) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_dir() => Ok(()),
        Ok(_) => Err(message.to_string()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Err(message.to_string()),
        Err(error) => Err(format!("failed to inspect privacy filter path: {error}")),
    }
}

fn is_canonical_privacy_filter_corpus_dir(fixture_dir: &Path) -> bool {
    let canonical_fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("scripts/privacy_filter/fixtures/corpus");
    match (
        fs::canonicalize(fixture_dir),
        fs::canonicalize(canonical_fixture_dir),
    ) {
        (Ok(requested), Ok(canonical)) => requested == canonical,
        _ => false,
    }
}

fn validate_privacy_filter_corpus_report(
    value: &Value,
    report_text: &str,
    require_canonical_counts: bool,
) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| "privacy filter corpus report must be a JSON object".to_string())?;
    let required_keys = [
        "engine",
        "scope",
        "fixture_count",
        "total_detected_span_count",
        "fixtures",
        "category_counts",
        "non_goals",
    ];
    let allowed_keys = [
        "engine",
        "scope",
        "fixture_count",
        "total_detected_span_count",
        "fixtures",
        "category_counts",
        "non_goals",
        "network_api_called",
    ];
    for key in required_keys {
        if !object.contains_key(key) {
            return Err("privacy filter corpus report missing required field".to_string());
        }
    }
    for key in object.keys() {
        if !allowed_keys.contains(&key.as_str()) {
            return Err("privacy filter corpus report has unexpected field".to_string());
        }
    }
    if object
        .get("network_api_called")
        .is_some_and(|network_api_called| network_api_called != false)
    {
        return Err("privacy filter corpus report cannot call network APIs".to_string());
    }
    if value["engine"] != "fallback_synthetic_patterns"
        || value["scope"] != "text_only_synthetic_corpus"
    {
        return Err("privacy filter corpus report required field has unexpected value".to_string());
    }
    let fixture_count = value["fixture_count"].as_u64().unwrap_or(0);
    let total_detected_span_count = value["total_detected_span_count"].as_u64().unwrap_or(0);
    let fixtures = value["fixtures"].as_array();
    if fixture_count == 0
        || fixtures.is_none()
        || fixtures.unwrap().len() as u64 != fixture_count
        || total_detected_span_count == 0
        || !validate_privacy_filter_category_counts(&value["category_counts"])
        || !value["non_goals"].is_array()
    {
        return Err("privacy filter corpus report has invalid required field shape".to_string());
    }
    if fixture_count == 2 || require_canonical_counts {
        if total_detected_span_count < 4
            || fixture_count != 2
            || value["category_counts"]["NAME"].as_u64().unwrap_or(0) != 2
            || value["category_counts"]["MRN"].as_u64().unwrap_or(0) != 2
            || value["category_counts"]["EMAIL"].as_u64().unwrap_or(0) != 1
            || value["category_counts"]["PHONE"].as_u64().unwrap_or(0) != 2
        {
            return Err(
                "privacy filter corpus report aggregate counts did not match requirements"
                    .to_string(),
            );
        }
    }
    let non_goals = value["non_goals"].as_array().unwrap();
    let allowed_non_goals = [
        "ocr",
        "visual_redaction",
        "image_pixel_redaction",
        "final_pdf_rewrite_export",
        "browser_ui",
        "desktop_ui",
    ];
    if !non_goals.iter().all(|item| {
        item.as_str()
            .is_some_and(|non_goal| allowed_non_goals.contains(&non_goal))
    }) {
        return Err("privacy filter corpus report has invalid non-goal".to_string());
    }
    if !non_goals.iter().any(|item| item == "visual_redaction") {
        return Err("privacy filter corpus report missing required non-goal".to_string());
    }
    let fixture_allowed_keys = ["fixture", "detected_span_count", "category_counts"];
    for fixture in value["fixtures"].as_array().unwrap() {
        let fixture_object = fixture
            .as_object()
            .ok_or_else(|| "privacy filter corpus report has invalid fixture shape".to_string())?;
        for key in fixture_object.keys() {
            if !fixture_allowed_keys.contains(&key.as_str()) {
                return Err("privacy filter corpus report fixture has unexpected field".to_string());
            }
        }
        if !fixture_object
            .get("fixture")
            .is_some_and(|fixture| fixture.is_string())
            || !fixture_object
                .get("detected_span_count")
                .is_some_and(|count| count.is_u64())
            || !fixture_object
                .get("category_counts")
                .is_some_and(validate_privacy_filter_category_counts)
        {
            return Err("privacy filter corpus report has invalid fixture shape".to_string());
        }
    }
    for raw_phi in [
        "Jane Example",
        "MRN-12345",
        "jane@example.test",
        "555-111-2222",
    ] {
        if report_text.contains(raw_phi) {
            return Err("privacy filter corpus report leaked raw PHI".to_string());
        }
    }
    Ok(())
}

fn validate_privacy_filter_category_counts(value: &Value) -> bool {
    let Some(counts) = value.as_object() else {
        return false;
    };
    let allowed_labels = ["NAME", "MRN", "EMAIL", "PHONE", "ID"];
    counts
        .iter()
        .all(|(label, count)| allowed_labels.contains(&label.as_str()) && count.as_u64().is_some())
}

fn validate_privacy_filter_text_summary_artifact_fields(value: &Value) -> Result<(), String> {
    let metadata = value
        .get("metadata")
        .and_then(Value::as_object)
        .ok_or_else(|| "privacy filter summary has invalid required field shape".to_string())?;
    let summary = value
        .get("summary")
        .and_then(Value::as_object)
        .ok_or_else(|| "privacy filter summary has invalid required field shape".to_string())?;

    let engine = metadata
        .get("engine")
        .and_then(Value::as_str)
        .ok_or_else(|| "privacy filter summary has invalid required field shape".to_string())?;
    let preview_policy = metadata
        .get("preview_policy")
        .and_then(Value::as_str)
        .ok_or_else(|| "privacy filter summary has invalid required field shape".to_string())?;
    if !is_safe_summary_identifier(engine) || !is_safe_summary_identifier(preview_policy) {
        return Err("privacy filter summary has invalid required field shape".to_string());
    }
    if metadata.get("network_api_called") != Some(&Value::Bool(false)) {
        return Err("privacy filter summary has invalid required field shape".to_string());
    }
    if !summary
        .get("input_char_count")
        .is_some_and(|count| count.as_u64().is_some())
        || !summary
            .get("detected_span_count")
            .is_some_and(|count| count.as_u64().is_some())
        || !summary
            .get("category_counts")
            .is_some_and(validate_privacy_filter_category_counts)
    {
        return Err("privacy filter summary has invalid required field shape".to_string());
    }
    Ok(())
}

fn is_safe_summary_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && !is_unsafe_report_string(value)
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
}

const PRIVACY_FILTER_STDIN_MAX_BYTES: usize = 1024 * 1024;

fn run_privacy_filter_text(args: PrivacyFilterTextArgs) -> Result<(), String> {
    if let Some(summary_output) = &args.summary_output {
        if paths_are_same_existing_or_lexical(&args.report_path, summary_output) {
            return Err("privacy filter summary path must differ from report path".to_string());
        }
    }
    let _ = fs::remove_file(&args.report_path);
    if let Some(summary_output) = &args.summary_output {
        let _ = fs::remove_file(summary_output);
    }

    let result = (|| match &args.input {
        PrivacyFilterTextInput::Path(input_path) => {
            require_regular_file(input_path, "missing input file")?;
            require_regular_file(&args.runner_path, "missing runner file")?;
            run_privacy_filter_text_inner(&args, input_path)
        }
        PrivacyFilterTextInput::Stdin => {
            require_regular_file(&args.runner_path, "missing runner file")?;
            let stdin_bytes = read_privacy_filter_stdin_bytes()?;
            run_privacy_filter_text_stdin_inner(&args, stdin_bytes)
        }
    })();
    if result.is_err() {
        let _ = fs::remove_file(&args.report_path);
        if let Some(summary_output) = &args.summary_output {
            let _ = fs::remove_file(summary_output);
        }
    }
    result
}

fn read_privacy_filter_stdin_bytes() -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    let limit = PRIVACY_FILTER_STDIN_MAX_BYTES + 1;
    std::io::stdin()
        .take(limit as u64)
        .read_to_end(&mut bytes)
        .map_err(|err| format!("failed to read stdin input: {err}"))?;
    if bytes.is_empty() {
        return Err("missing stdin input".to_string());
    }
    if bytes.len() > PRIVACY_FILTER_STDIN_MAX_BYTES {
        return Err("stdin input exceeds 1048576 byte limit".to_string());
    }
    Ok(bytes)
}

fn run_privacy_filter_text_inner(
    args: &PrivacyFilterTextArgs,
    input_path: &Path,
) -> Result<(), String> {
    let mut command = std::process::Command::new(&args.python_command);
    command.arg(&args.runner_path);
    if args.mock {
        command.arg("--mock");
    }
    let mut child = command
        .arg(input_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|err| format!("failed to run privacy filter runner: {err}"))?;

    let (stdout, value) = finish_privacy_filter_text_child(args, &mut child)?;
    emit_privacy_filter_text_success(args, stdout, &value)
}

fn run_privacy_filter_text_stdin_inner(
    args: &PrivacyFilterTextArgs,
    stdin_bytes: Vec<u8>,
) -> Result<(), String> {
    let mut command = std::process::Command::new(&args.python_command);
    command.arg(&args.runner_path);
    if args.mock {
        command.arg("--mock");
    }
    let mut child = command
        .arg("--stdin")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|err| format!("failed to run privacy filter runner: {err}"))?;
    let stdin_writer = spawn_privacy_filter_stdin_writer(
        child
            .stdin
            .take()
            .ok_or_else(|| "failed to write privacy filter runner stdin".to_string())?,
        stdin_bytes,
    );

    let (stdout, value) = finish_privacy_filter_text_child(args, &mut child)?;
    stdin_writer
        .recv_timeout(Duration::from_secs(1))
        .map_err(|_| "failed to write privacy filter runner stdin".to_string())??;
    emit_privacy_filter_text_success(args, stdout, &value)
}

fn spawn_privacy_filter_stdin_writer(
    mut child_stdin: std::process::ChildStdin,
    stdin_bytes: Vec<u8>,
) -> mpsc::Receiver<Result<(), String>> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let result = child_stdin
            .write_all(&stdin_bytes)
            .map_err(|err| format!("failed to write privacy filter runner stdin: {err}"));
        drop(child_stdin);
        let _ = tx.send(result);
    });
    rx
}

fn finish_privacy_filter_text_child(
    args: &PrivacyFilterTextArgs,
    child: &mut std::process::Child,
) -> Result<(String, Value), String> {
    let (status, stdout_bytes) = wait_for_privacy_filter_runner(
        child,
        PRIVACY_FILTER_RUNNER_TIMEOUT,
        PRIVACY_FILTER_RUNNER_STDOUT_MAX_BYTES,
    )?;
    if !status.success() {
        return Err("privacy filter runner failed".to_string());
    }

    let stdout = String::from_utf8(stdout_bytes)
        .map_err(|_| "runner returned non-UTF-8 output".to_string())?;
    let value: Value =
        serde_json::from_str(&stdout).map_err(|_| "runner returned non-JSON output".to_string())?;
    validate_privacy_filter_output(&value)?;
    if args.summary_output.is_some() {
        validate_privacy_filter_text_summary_artifact_fields(&value)?;
    }
    Ok((stdout, value))
}

fn emit_privacy_filter_text_success(
    args: &PrivacyFilterTextArgs,
    stdout: String,
    value: &Value,
) -> Result<(), String> {
    fs::write(&args.report_path, stdout)
        .map_err(|err| format!("failed to write privacy filter report: {err}"))?;

    if let Some(summary_output) = &args.summary_output {
        let artifact_summary = privacy_filter_text_summary_artifact(&value);
        fs::write(
            summary_output,
            format!(
                "{}\n",
                serde_json::to_string_pretty(&artifact_summary)
                    .map_err(|err| format!("failed to render privacy filter summary: {err}"))?
            ),
        )
        .map_err(|err| format!("failed to write privacy filter summary: {err}"))?;
    }

    let summary = json!({
        "command": "privacy-filter-text",
        "report_path": "<redacted>",
        "report_written": true,
        "engine": value["metadata"]["engine"],
        "network_api_called": value["metadata"]["network_api_called"],
        "detected_span_count": value["summary"]["detected_span_count"],
    });
    println!(
        "{}",
        serde_json::to_string(&summary)
            .map_err(|err| format!("failed to render summary: {err}"))?
    );
    Ok(())
}

fn privacy_filter_text_summary_artifact(value: &Value) -> Value {
    json!({
        "artifact": "privacy_filter_text_summary",
        "scope": "text_only_single_report_summary",
        "engine": value["metadata"]["engine"],
        "network_api_called": value["metadata"]["network_api_called"],
        "preview_policy": value["metadata"]["preview_policy"],
        "input_char_count": value["summary"]["input_char_count"],
        "detected_span_count": value["summary"]["detected_span_count"],
        "category_counts": value["summary"]["category_counts"],
        "non_goals": [
            "ocr",
            "visual_redaction",
            "image_pixel_redaction",
            "final_pdf_rewrite_export",
            "browser_ui",
            "desktop_ui"
        ],
    })
}

fn wait_for_privacy_filter_runner(
    child: &mut std::process::Child,
    timeout: Duration,
    max_stdout_bytes: usize,
) -> Result<(std::process::ExitStatus, Vec<u8>), String> {
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "failed to read privacy filter runner output".to_string())?;
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut stdout_bytes = Vec::new();
        let result = stdout
            .take((max_stdout_bytes + 1) as u64)
            .read_to_end(&mut stdout_bytes)
            .map(|_| stdout_bytes)
            .map_err(|err| format!("failed to read privacy filter runner output: {err}"));
        let _ = tx.send(result);
    });

    let deadline = Instant::now() + timeout;
    let mut captured_stdout: Option<Vec<u8>> = None;
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|err| format!("failed to wait for privacy filter runner: {err}"))?
        {
            break status;
        }
        if captured_stdout.is_none() {
            if let Ok(read_result) = rx.try_recv() {
                let stdout_bytes = read_result?;
                if stdout_bytes.len() > max_stdout_bytes {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err("runner output exceeded limit".to_string());
                }
                captured_stdout = Some(stdout_bytes);
            }
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err("privacy filter runner timed out".to_string());
        }
        thread::sleep(Duration::from_millis(25));
    };

    let stdout_bytes = match captured_stdout {
        Some(stdout_bytes) => stdout_bytes,
        None => rx
            .recv_timeout(Duration::from_secs(1))
            .map_err(|_| "failed to read privacy filter runner output".to_string())??,
    };
    if stdout_bytes.len() > max_stdout_bytes {
        return Err("runner output exceeded limit".to_string());
    }
    Ok((status, stdout_bytes))
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
    for key in ["engine", "network_api_called", "preview_policy"] {
        if value["metadata"].get(key).is_none() {
            return Err("privacy filter output missing required metadata field".to_string());
        }
    }
    if !value["metadata"]["engine"].is_string() || !value["metadata"]["preview_policy"].is_string()
    {
        return Err("privacy filter output has invalid metadata field shape".to_string());
    }
    if value["metadata"]["network_api_called"] != false {
        return Err("privacy filter output indicates network API use".to_string());
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
    "Usage: mdid-cli [status]\n       mdid-cli verify-artifacts --artifact-paths-json <json-array> [--max-bytes <bytes>]\n       mdid-cli deidentify-csv --csv-path <path> --policies-json <json> --vault-path <path> --passphrase <value> --output-path <path>\n       mdid-cli deidentify-xlsx --xlsx-path <path> --policies-json <json> --vault-path <path> --passphrase <value> --output-path <path>\n       mdid-cli deidentify-dicom --dicom-path <input.dcm> --private-tag-policy <remove|review|required|keep> --vault-path <vault.json> --passphrase <passphrase> --output-path <output.dcm>\n       mdid-cli deidentify-pdf --pdf-path <input.pdf> --source-name <name.pdf> --report-path <report.json>\n       mdid-cli review-media --artifact-label <label> --format <image|video|fcs> --metadata-json <json> --requires-visual-review <true|false> --unsupported-payload <true|false> --report-path <report.json>\n       mdid-cli privacy-filter-text (--input-path <text> | --stdin) --runner-path <path> --report-path <report.json> [--summary-output <summary.json>] [--python-command <path-or-command>] [--mock]\n       mdid-cli privacy-filter-corpus --fixture-dir <dir> --runner-path <path> --report-path <report.json> [--summary-output <summary.json>] [--python-command <path-or-command>]\n       mdid-cli ocr-to-privacy-filter --image-path <path> --ocr-runner-path <path> --privacy-runner-path <path> --report-path <report.json> [--summary-output <summary.json>] [--python-command <cmd>] [--mock]\n       mdid-cli ocr-to-privacy-filter-corpus --fixture-dir <dir> --ocr-runner-path <path> --privacy-runner-path <path> --bridge-runner-path <path> --report-path <report.json> [--summary-output <summary.json>] [--python-command <path-or-command>]\n       mdid-cli ocr-handoff-corpus --fixture-dir <dir> --runner-path <path> --report-path <path> [--summary-output <summary.json>] [--python-command <cmd>]\n       mdid-cli ocr-small-json --image-path <path> --ocr-runner-path <path> --report-path <report.json> [--summary-output <summary.json>] [--python-command <cmd>] [--mock]\n       mdid-cli ocr-privacy-evidence --image-path <image> --runner-path <runner.py> --output <report.json> [--python-command <cmd>] [--mock]\n       mdid-cli ocr-handoff --image-path <image> --ocr-runner-path <path> --handoff-builder-path <path> --report-path <report.json> [--python-command <path-or-command>]\n       mdid-cli vault-audit --vault-path <vault.json> --passphrase <passphrase> [--limit <count>] [--offset <count>]\n       mdid-cli vault-decode --vault-path <vault.json> --passphrase <passphrase> --record-ids-json <json> --output-target <target> --justification <text> --report-path <report.json>\n       mdid-cli vault-export --vault-path <vault.json> --passphrase <passphrase> --record-ids-json <json> --export-passphrase <passphrase> --context <text> --artifact-path <export.json>\n       mdid-cli vault-import --vault-path <vault.json> --passphrase <passphrase> --artifact-path <export.json> --portable-passphrase <passphrase> --context <text>\n       mdid-cli vault-inspect-artifact --artifact-path <export.json> --portable-passphrase <passphrase>\n\nmdid-cli is the local de-identification automation surface.\nCommands:\n  status              Print a readiness banner for the local CLI surface.\n  verify-artifacts    Verify local artifact existence and size with metadata-only PHI-safe JSON.\n  deidentify-csv      Rewrite a local CSV using explicit field policies.\n  deidentify-xlsx     Rewrite a bounded local XLSX using explicit field policies.\n  deidentify-dicom    Rewrite a bounded local DICOM file with a PHI-safe summary.\n  deidentify-pdf      Review a bounded local PDF and write a PHI-safe JSON report; no OCR or PDF rewrite/export.\n  review-media        Review conservative media metadata and write a PHI-safe JSON report; no media rewrite/export.\n  privacy-filter-text Run a local privacy filter runner for text and write its bounded JSON report.\n  privacy-filter-corpus Run a local synthetic text corpus privacy filter and print aggregate PHI-safe JSON.\n  ocr-handoff-corpus Run a local OCR handoff corpus runner and print aggregate PHI-safe JSON.\n  ocr-privacy-evidence Run local OCR privacy evidence and write a bounded PHI-safe JSON report.\n  ocr-handoff        Run bounded synthetic OCR extraction handoff and validate its JSON report.\n  vault-audit         Print bounded PHI-safe vault audit event metadata in reverse chronological order; read-only.\n  vault-decode        Decode explicitly scoped vault records to a report file and print a PHI-safe summary.\n  vault-export        Export explicitly scoped vault records to an encrypted portable artifact and print a PHI-safe summary.\n  vault-import        Import encrypted portable vault records into a local vault and print a PHI-safe summary.\n  vault-inspect-artifact Inspect an encrypted portable vault artifact and print only a PHI-safe record count."
}

#[cfg(test)]
mod tests {
    use super::*;
    use mdid_domain::MappingScope;
    use mdid_vault::NewMappingRecord;

    #[test]
    fn windows_ascii_case_filename_equivalence_matches_same_canonical_parent() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let report_path = temp_dir.path().join("report.json");
        let summary_path = temp_dir.path().join("REPORT.json");

        assert_eq!(
            paths_are_same_existing_or_lexical(&report_path, &summary_path),
            cfg!(windows)
        );
    }

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
