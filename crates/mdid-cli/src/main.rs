use std::{fs, path::PathBuf, process};

use mdid_adapters::{CsvTabularAdapter, FieldPolicy, FieldPolicyAction, XlsxTabularAdapter};
use mdid_application::{
    DicomDeidentificationService, PdfDeidentificationService, TabularDeidentificationService,
};
use mdid_domain::{
    AuditEvent, AuditEventKind, BatchSummary, DicomPrivateTagPolicy, PdfPageRef, PdfScanStatus,
    SurfaceKind,
};
use mdid_vault::LocalVaultStore;
use rust_xlsxwriter::Workbook;
use serde::{Deserialize, Serialize};
use serde_json::json;

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
    DeidentifyCsv(DeidentifyCsvArgs),
    DeidentifyXlsx(DeidentifyXlsxArgs),
    DeidentifyDicom(DeidentifyDicomArgs),
    DeidentifyPdf(DeidentifyPdfArgs),
    VaultAudit(VaultAuditArgs),
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
struct VaultAuditArgs {
    vault_path: PathBuf,
    passphrase: String,
    limit: Option<usize>,
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

fn parse_command(args: &[String]) -> Result<CliCommand, String> {
    match args {
        [] => Ok(CliCommand::Status),
        [status] if status == "status" => Ok(CliCommand::Status),
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
        [command, rest @ ..] if command == "vault-audit" => {
            parse_vault_audit_args(rest).map(CliCommand::VaultAudit)
        }
        _ => Err("unknown command".to_string()),
    }
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

fn parse_vault_audit_args(args: &[String]) -> Result<VaultAuditArgs, String> {
    let mut vault_path = None;
    let mut passphrase = None;
    let mut limit = None;

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
            _ => return Err("unknown flag".to_string()),
        }
        index += 2;
    }

    Ok(VaultAuditArgs {
        vault_path: vault_path.ok_or_else(|| "missing --vault-path".to_string())?,
        passphrase: passphrase.ok_or_else(|| "missing --passphrase".to_string())?,
        limit,
    })
}

fn run_command(command: CliCommand) -> Result<(), String> {
    match command {
        CliCommand::Status => {
            println!("med-de-id CLI ready");
            Ok(())
        }
        CliCommand::DeidentifyCsv(args) => run_deidentify_csv(args),
        CliCommand::DeidentifyXlsx(args) => run_deidentify_xlsx(args),
        CliCommand::DeidentifyDicom(args) => run_deidentify_dicom(args),
        CliCommand::DeidentifyPdf(args) => run_deidentify_pdf(args),
        CliCommand::VaultAudit(args) => run_vault_audit(args),
    }
}

#[derive(Debug, Serialize)]
struct PdfPageStatusReport {
    page: PdfPageRef,
    status: PdfScanStatus,
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
    returned_event_count: usize,
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

fn run_vault_audit(args: VaultAuditArgs) -> Result<(), String> {
    let vault = LocalVaultStore::unlock(&args.vault_path, &args.passphrase)
        .map_err(|err| format!("failed to open vault: {err}"))?;
    let report = build_vault_audit_report(vault.audit_events(), args.limit);
    println!(
        "{}",
        serde_json::to_string(&report)
            .map_err(|err| format!("failed to render audit report: {err}"))?
    );
    Ok(())
}

fn build_vault_audit_report(events: &[AuditEvent], limit: Option<usize>) -> VaultAuditReport {
    let event_count = events.len();
    let selected = events.iter().rev().take(limit.unwrap_or(event_count));
    let events = selected
        .map(|event| VaultAuditEventReport {
            id: event.id.to_string(),
            kind: format!("{:?}", event.kind),
            actor: event.actor.clone(),
            detail: sanitized_audit_detail(event),
            recorded_at: event.recorded_at.to_rfc3339(),
        })
        .collect::<Vec<_>>();
    VaultAuditReport {
        event_count,
        returned_event_count: events.len(),
        events,
    }
}

fn sanitized_audit_detail(event: &AuditEvent) -> String {
    match event.kind {
        AuditEventKind::Encode => "encoded mapping".to_string(),
        _ => event.detail.clone(),
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
    "Usage: mdid-cli [status]\n       mdid-cli deidentify-csv --csv-path <path> --policies-json <json> --vault-path <path> --passphrase <value> --output-path <path>\n       mdid-cli deidentify-xlsx --xlsx-path <path> --policies-json <json> --vault-path <path> --passphrase <value> --output-path <path>\n       mdid-cli deidentify-dicom --dicom-path <input.dcm> --private-tag-policy <remove|review|required|keep> --vault-path <vault.json> --passphrase <passphrase> --output-path <output.dcm>\n       mdid-cli deidentify-pdf --pdf-path <input.pdf> --source-name <name.pdf> --report-path <report.json>\n       mdid-cli vault-audit --vault-path <vault.json> --passphrase <passphrase> [--limit <count>]\n\nmdid-cli is the local de-identification automation surface.\nCommands:\n  status              Print a readiness banner for the local CLI surface.\n  deidentify-csv      Rewrite a local CSV using explicit field policies.\n  deidentify-xlsx     Rewrite a bounded local XLSX using explicit field policies.\n  deidentify-dicom    Rewrite a bounded local DICOM file with a PHI-safe summary.\n  deidentify-pdf      Review a bounded local PDF and write a PHI-safe JSON report; no OCR or PDF rewrite/export.\n  vault-audit         Print bounded PHI-safe vault audit event metadata in reverse chronological order; read-only."
}

#[cfg(test)]
mod tests {
    use super::*;

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
                }))
        );
    }

    #[test]
    fn vault_audit_report_limits_events_without_exposing_phi_values() {
        let events: Vec<mdid_domain::AuditEvent> = serde_json::from_value(json!([
            {
                "id": "00000000-0000-0000-0000-000000000000",
                "kind": "encode",
                "actor": "cli",
                "detail": "encoded mapping row:1:patient_name containing Alice Example",
                "recorded_at": "2026-04-29T00:00:00Z"
            },
            {
                "id": "00000000-0000-0000-0000-000000000000",
                "kind": "decode",
                "actor": "desktop",
                "detail": "decode to screen because break-glass decoded 1 record record_ids=[abc]",
                "recorded_at": "2026-04-29T01:00:00Z"
            }
        ]))
        .unwrap();

        let report = build_vault_audit_report(&events, Some(1));
        let rendered = serde_json::to_string(&report).unwrap();

        assert!(rendered.contains("event_count"));
        assert!(rendered.contains("returned_event_count"));
        assert!(rendered.contains("Decode"));
        assert!(!rendered.contains("Alice Example"));
        assert_eq!(report.event_count, 2);
        assert_eq!(report.returned_event_count, 1);
        assert_eq!(
            report.events[0].detail,
            "decode to screen because break-glass decoded 1 record record_ids=[abc]"
        );
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
