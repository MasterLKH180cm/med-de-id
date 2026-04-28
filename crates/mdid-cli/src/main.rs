use std::{fs, path::PathBuf, process};

use mdid_adapters::{FieldPolicy, FieldPolicyAction};
use mdid_application::TabularDeidentificationService;
use mdid_domain::{BatchSummary, SurfaceKind};
use mdid_vault::LocalVaultStore;
use serde::Deserialize;
use serde_json::json;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match parse_command(&args).and_then(run_command) {
        Ok(()) => {}
        Err(error) => exit_with_usage(&error),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CliCommand {
    Status,
    DeidentifyCsv(DeidentifyCsvArgs),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DeidentifyCsvArgs {
    csv_path: PathBuf,
    policies_json: String,
    vault_path: PathBuf,
    passphrase: String,
    output_path: PathBuf,
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

fn run_command(command: CliCommand) -> Result<(), String> {
    match command {
        CliCommand::Status => {
            println!("med-de-id CLI ready");
            Ok(())
        }
        CliCommand::DeidentifyCsv(args) => run_deidentify_csv(args),
    }
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
    "Usage: mdid-cli [status]\n       mdid-cli deidentify-csv --csv-path <path> --policies-json <json> --vault-path <path> --passphrase <value> --output-path <path>\n\nmdid-cli is the local de-identification automation surface.\nCommands:\n  status           Print a readiness banner for the local CLI surface.\n  deidentify-csv   Rewrite a local CSV using explicit field policies."
}
