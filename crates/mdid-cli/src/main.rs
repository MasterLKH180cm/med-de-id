use serde_json::Value;
use std::{fs, process};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match parse_command(&args) {
        Ok(CliCommand::Status) => println!("med-de-id CLI ready"),
        Ok(CliCommand::MoatControllerPlan(command)) => {
            if let Err(error) = run_moat_controller_plan(&command) {
                exit_with_usage(&error);
            }
        }
        Err(error) => exit_with_usage(&error),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CliCommand {
    Status,
    MoatControllerPlan(MoatControllerPlanCommand),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MoatControllerPlanCommand {
    history_path: String,
    round_id: Option<String>,
    role: Option<String>,
    kind: Option<String>,
    node_id: Option<String>,
    depends_on: Option<String>,
    no_dependencies: bool,
    requires_artifacts: bool,
    title_contains: Option<String>,
    spec_ref: Option<String>,
    limit: Option<usize>,
    format: OutputFormat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkPacket {
    node_id: String,
    title: String,
    role: String,
    kind: String,
    state: String,
    spec_ref: Option<String>,
    dependencies: Vec<String>,
}

fn parse_command(args: &[String]) -> Result<CliCommand, String> {
    match args {
        [] => Ok(CliCommand::Status),
        [status] if status == "status" => Ok(CliCommand::Status),
        [moat, controller_plan, rest @ ..]
            if moat == "moat" && controller_plan == "controller-plan" =>
        {
            Ok(CliCommand::MoatControllerPlan(
                parse_moat_controller_plan_command(rest)?,
            ))
        }
        _ => Err(format!("unknown command: {}", args.join(" "))),
    }
}

fn parse_moat_controller_plan_command(
    args: &[String],
) -> Result<MoatControllerPlanCommand, String> {
    let mut history_path = None;
    let mut round_id = None;
    let mut role = None;
    let mut kind = None;
    let mut node_id = None;
    let mut depends_on = None;
    let mut no_dependencies = false;
    let mut requires_artifacts = false;
    let mut title_contains = None;
    let mut spec_ref = None;
    let mut limit = None;
    let mut format = OutputFormat::Text;
    let mut saw_format = false;

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--history-path" => {
                let value =
                    required_flag_value(args, index, "moat controller-plan --history-path")?;
                if history_path.is_some() {
                    return Err("duplicate moat controller-plan --history-path".to_string());
                }
                history_path = Some(value.to_string());
                index += 2;
            }
            "--round-id" => {
                let value = required_flag_value(args, index, "moat controller-plan --round-id")?;
                if round_id.is_some() {
                    return Err("duplicate moat controller-plan --round-id".to_string());
                }
                round_id = Some(value.to_string());
                index += 2;
            }
            "--role" => {
                let value = required_flag_value(args, index, "moat controller-plan --role")?;
                if role.is_some() {
                    return Err("duplicate moat controller-plan --role".to_string());
                }
                role = Some(value.to_string());
                index += 2;
            }
            "--kind" => {
                let value = required_flag_value(args, index, "moat controller-plan --kind")?;
                if kind.is_some() {
                    return Err("duplicate moat controller-plan --kind".to_string());
                }
                kind = Some(value.to_string());
                index += 2;
            }
            "--node-id" => {
                let value = required_flag_value(args, index, "moat controller-plan --node-id")?;
                if node_id.is_some() {
                    return Err("duplicate moat controller-plan --node-id".to_string());
                }
                node_id = Some(value.to_string());
                index += 2;
            }
            "--depends-on" => {
                let value = required_flag_value(args, index, "moat controller-plan --depends-on")?;
                if depends_on.is_some() {
                    return Err("duplicate moat controller-plan --depends-on".to_string());
                }
                depends_on = Some(value.to_string());
                index += 2;
            }
            "--no-dependencies" => {
                if no_dependencies {
                    return Err("duplicate moat controller-plan --no-dependencies".to_string());
                }
                no_dependencies = true;
                index += 1;
            }
            "--requires-artifacts" => {
                if requires_artifacts {
                    return Err("duplicate moat controller-plan --requires-artifacts".to_string());
                }
                requires_artifacts = true;
                index += 1;
            }
            "--title-contains" => {
                let value =
                    required_flag_value(args, index, "moat controller-plan --title-contains")?;
                if title_contains.is_some() {
                    return Err("duplicate moat controller-plan --title-contains".to_string());
                }
                title_contains = Some(value.to_string());
                index += 2;
            }
            "--spec-ref" => {
                let value = required_flag_value(args, index, "moat controller-plan --spec-ref")?;
                if spec_ref.is_some() {
                    return Err("duplicate moat controller-plan --spec-ref".to_string());
                }
                spec_ref = Some(value.to_string());
                index += 2;
            }
            "--limit" => {
                let value = required_flag_value(args, index, "moat controller-plan --limit")?;
                if limit.is_some() {
                    return Err("duplicate moat controller-plan --limit".to_string());
                }
                limit = Some(
                    value
                        .parse::<usize>()
                        .map_err(|_| format!("invalid moat controller-plan --limit: {value}"))?,
                );
                index += 2;
            }
            "--format" => {
                let value = required_flag_value(args, index, "moat controller-plan --format")?;
                if saw_format {
                    return Err("duplicate moat controller-plan --format".to_string());
                }
                format = match value {
                    "text" => OutputFormat::Text,
                    "json" => OutputFormat::Json,
                    other => return Err(format!("unknown moat controller-plan format: {other}")),
                };
                saw_format = true;
                index += 2;
            }
            flag => return Err(format!("unknown option for moat controller-plan: {flag}")),
        }
    }

    if depends_on.is_some() && no_dependencies {
        return Err(
            "moat controller-plan cannot combine --depends-on and --no-dependencies".to_string(),
        );
    }

    Ok(MoatControllerPlanCommand {
        history_path: history_path
            .ok_or_else(|| "missing --history-path for moat controller-plan".to_string())?,
        round_id,
        role,
        kind,
        node_id,
        depends_on,
        no_dependencies,
        requires_artifacts,
        title_contains,
        spec_ref,
        limit,
        format,
    })
}

fn required_flag_value<'a>(
    args: &'a [String],
    index: usize,
    flag_name: &str,
) -> Result<&'a str, String> {
    args.get(index + 1)
        .filter(|value| !value.starts_with("--"))
        .map(String::as_str)
        .ok_or_else(|| format!("missing value for {flag_name}"))
}

fn run_moat_controller_plan(command: &MoatControllerPlanCommand) -> Result<(), String> {
    let contents = fs::read_to_string(&command.history_path)
        .map_err(|error| format!("failed to read moat history file: {error}"))?;
    let history: Value = serde_json::from_str(&contents)
        .map_err(|error| format!("failed to parse moat history file: {error}"))?;
    let entries = history
        .get("entries")
        .and_then(Value::as_array)
        .ok_or_else(|| "invalid moat history file: missing entries array".to_string())?;

    if entries.is_empty() {
        return Err(
            "moat history is empty; provide a history file with at least one round".to_string(),
        );
    }

    let entry = if let Some(round_id) = command.round_id.as_deref() {
        entries
            .iter()
            .find(|entry| entry_round_id(entry).as_deref() == Some(round_id))
            .ok_or_else(|| format!("unknown moat round-id: {round_id}"))?
    } else {
        entries.last().expect("entries not empty")
    };

    let round_id = entry_round_id(entry).unwrap_or_default();
    let nodes = entry
        .get("report")
        .and_then(|value| value.get("control_plane"))
        .and_then(|value| value.get("task_graph"))
        .and_then(|value| value.get("nodes"))
        .and_then(Value::as_array)
        .ok_or_else(|| "invalid moat history file: missing task graph nodes".to_string())?;

    let mut packets = nodes
        .iter()
        .map(parse_work_packet)
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|packet| packet.state == "ready")
        .filter(|packet| {
            command
                .role
                .as_deref()
                .map(|v| packet.role == v)
                .unwrap_or(true)
        })
        .filter(|packet| {
            command
                .kind
                .as_deref()
                .map(|v| packet.kind == v)
                .unwrap_or(true)
        })
        .filter(|packet| {
            command
                .node_id
                .as_deref()
                .map(|v| packet.node_id == v)
                .unwrap_or(true)
        })
        .filter(|packet| {
            command
                .depends_on
                .as_deref()
                .map(|v| packet.dependencies.iter().any(|dep| dep == v))
                .unwrap_or(true)
        })
        .filter(|packet| !command.no_dependencies || packet.dependencies.is_empty())
        .filter(|packet| {
            command
                .title_contains
                .as_deref()
                .map(|v| packet.title.contains(v))
                .unwrap_or(true)
        })
        .filter(|packet| {
            command
                .spec_ref
                .as_deref()
                .map(|v| packet.spec_ref.as_deref() == Some(v))
                .unwrap_or(true)
        })
        .filter(|packet| !command.requires_artifacts || dependencies_have_artifacts(nodes, packet))
        .collect::<Vec<_>>();

    if let Some(limit) = command.limit {
        packets.truncate(limit);
    }

    let acceptance = [
        "Read-only controller packet export only; do not mutate moat history or advertise write-side completion commands.",
    ];

    match command.format {
        OutputFormat::Text => {
            println!("controller_plan_packets={}", packets.len());
            for packet in packets {
                println!(
                    "work_packet={}|{}|{}|{}, title={}, dependencies={}, acceptance_criteria={}",
                    packet.node_id,
                    packet.role,
                    packet.kind,
                    packet.state,
                    packet.title,
                    format_dependencies(&packet.dependencies),
                    acceptance.join("; "),
                );
            }
        }
        OutputFormat::Json => {
            let packets = packets
                .into_iter()
                .map(|packet| {
                    serde_json::json!({
                        "node_id": packet.node_id,
                        "role": packet.role,
                        "kind": packet.kind,
                        "state": packet.state,
                        "title": packet.title,
                        "dependencies": packet.dependencies,
                        "spec_ref": packet.spec_ref,
                        "acceptance_criteria": acceptance,
                    })
                })
                .collect::<Vec<_>>();
            let envelope = serde_json::json!({
                "type": "moat_controller_plan",
                "history_path": command.history_path,
                "round_id": round_id,
                "read_only": true,
                "packet_count": packets.len(),
                "packets": packets,
                "constraints": {
                    "local_only": true,
                    "read_only": true,
                    "no_agent_launch": true,
                    "no_daemon": true,
                    "no_pr_creation": true,
                    "no_cron_creation": true,
                    "no_code_writes": true,
                    "no_artifact_writes": true
                }
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&envelope).map_err(|error| format!(
                    "failed to serialize controller-plan envelope: {error}"
                ))?
            );
        }
    }

    Ok(())
}

fn entry_round_id(entry: &Value) -> Option<String> {
    entry
        .get("report")?
        .get("summary")?
        .get("round_id")?
        .as_str()
        .map(ToOwned::to_owned)
}

fn parse_work_packet(node: &Value) -> Result<WorkPacket, String> {
    let node_id = required_node_string(node, "node_id")?.to_string();

    Ok(WorkPacket {
        node_id: node_id.clone(),
        title: required_node_string(node, "title")?.to_string(),
        role: required_node_string(node, "role")?.to_string(),
        kind: required_node_string(node, "kind")?.to_string(),
        state: required_node_string(node, "state")?.to_string(),
        spec_ref: match node.get("spec_ref") {
            Some(Value::Null) | None => None,
            Some(value) => Some(
                value
                    .as_str()
                    .ok_or_else(|| {
                        format!(
                            "invalid moat history file: node {node_id} field spec_ref must be a string or null"
                        )
                    })?
                    .to_string(),
            ),
        },
        dependencies: parse_dependencies(node, &node_id)?,
    })
}

fn required_node_string<'a>(node: &'a Value, field: &str) -> Result<&'a str, String> {
    node.get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("invalid moat history file: node field {field} must be a string"))
}

fn parse_dependencies(node: &Value, node_id: &str) -> Result<Vec<String>, String> {
    let Some(value) = node.get("depends_on") else {
        return Ok(Vec::new());
    };
    let dependencies = value.as_array().ok_or_else(|| {
        format!("invalid moat history file: node {node_id} field depends_on must be an array")
    })?;

    dependencies
        .iter()
        .map(|dependency| {
            dependency
                .as_str()
                .map(ToOwned::to_owned)
                .ok_or_else(|| {
                    format!(
                        "invalid moat history file: node {node_id} field depends_on entries must be strings"
                    )
                })
        })
        .collect()
}

fn dependencies_have_artifacts(nodes: &[Value], packet: &WorkPacket) -> bool {
    packet.dependencies.iter().all(|dependency| {
        nodes
            .iter()
            .find(|node| node.get("node_id").and_then(Value::as_str) == Some(dependency.as_str()))
            .map(node_has_artifacts)
            .unwrap_or(false)
    })
}

fn node_has_artifacts(node: &Value) -> bool {
    node.get("artifacts")
        .and_then(Value::as_array)
        .map(|artifacts| !artifacts.is_empty())
        .unwrap_or_else(|| {
            node.get("artifact_ref").and_then(Value::as_str).is_some()
                || node
                    .get("artifact_summary")
                    .and_then(Value::as_str)
                    .is_some()
        })
}

fn format_dependencies(dependencies: &[String]) -> String {
    if dependencies.is_empty() {
        "<none>".to_string()
    } else {
        dependencies.join(",")
    }
}

fn exit_with_usage(error: &str) -> ! {
    eprintln!("{error}");
    eprintln!("usage: mdid-cli [status | moat controller-plan --history-path PATH [--round-id ROUND_ID] [--role planner|coder|reviewer] [--kind KIND] [--node-id NODE_ID] [--depends-on NODE_ID] [--no-dependencies] [--requires-artifacts] [--title-contains TEXT] [--spec-ref SPEC_REF] [--limit N] [--format text|json]]");
    process::exit(1);
}
