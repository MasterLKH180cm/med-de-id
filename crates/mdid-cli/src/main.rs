use serde_json::{json, Value};
use std::{fs, path::Path, process};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match parse_command(&args) {
        Ok(CliCommand::Status) => println!("med-de-id CLI ready"),
        Ok(CliCommand::MoatControllerPlan(command)) => {
            if let Err(error) = run_moat_controller_plan(&command) {
                exit_with_usage(&error);
            }
        }
        Ok(CliCommand::MoatControllerStep(command)) => {
            if let Err(error) = run_moat_controller_step(&command) {
                exit_with_usage(&error);
            }
        }
        Err(error) => exit_with_usage(&error),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CliCommand {
    Status,
    MoatControllerPlan(Box<MoatControllerPlanCommand>),
    MoatControllerStep(Box<MoatControllerStepCommand>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum OutputFormat {
    #[default]
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
struct MoatControllerStepCommand {
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
    agent_id: Option<String>,
    lease_seconds: i64,
    dry_run: bool,
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct DependencyArtifact {
    node_id: String,
    artifact_ref: String,
    artifact_summary: String,
}

const CONTROLLER_ROLES: [&str; 3] = ["planner", "coder", "reviewer"];
const CONTROLLER_ACCEPTANCE: [&str; 1] = ["Use SDD and TDD before completing this task."];

fn parse_command(args: &[String]) -> Result<CliCommand, String> {
    match args {
        [] => Ok(CliCommand::Status),
        [status] if status == "status" => Ok(CliCommand::Status),
        [moat, controller_plan, rest @ ..]
            if moat == "moat" && controller_plan == "controller-plan" =>
        {
            Ok(CliCommand::MoatControllerPlan(Box::new(
                parse_moat_controller_plan_command(rest)?,
            )))
        }
        [moat, controller_step, rest @ ..]
            if moat == "moat" && controller_step == "controller-step" =>
        {
            Ok(CliCommand::MoatControllerStep(Box::new(
                parse_moat_controller_step_command(rest)?,
            )))
        }
        _ => Err(format!("unknown command: {}", args.join(" "))),
    }
}

fn parse_moat_controller_plan_command(
    args: &[String],
) -> Result<MoatControllerPlanCommand, String> {
    let base = parse_controller_flags(args, "moat controller-plan")?;
    if base.agent_id.is_some() {
        return Err("unsupported option for moat controller-plan: --agent-id".to_string());
    }
    if base.lease_seconds.is_some() {
        return Err("unsupported option for moat controller-plan: --lease-seconds".to_string());
    }
    if base.dry_run {
        return Err("unsupported option for moat controller-plan: --dry-run".to_string());
    }
    Ok(MoatControllerPlanCommand {
        history_path: base
            .history_path
            .ok_or_else(|| "missing --history-path for moat controller-plan".to_string())?,
        round_id: base.round_id,
        role: base.role,
        kind: base.kind,
        node_id: base.node_id,
        depends_on: base.depends_on,
        no_dependencies: base.no_dependencies,
        requires_artifacts: base.requires_artifacts,
        title_contains: base.title_contains,
        spec_ref: base.spec_ref,
        limit: base.limit,
        format: base.format,
    })
}

fn parse_moat_controller_step_command(
    args: &[String],
) -> Result<MoatControllerStepCommand, String> {
    let base = parse_controller_flags(args, "moat controller-step")?;
    if base.limit.is_some() {
        return Err("unsupported option for moat controller-step: --limit".to_string());
    }
    let lease_seconds = base.lease_seconds.unwrap_or(900);
    if lease_seconds <= 0 {
        return Err("moat controller-step --lease-seconds must be positive".to_string());
    }
    Ok(MoatControllerStepCommand {
        history_path: base
            .history_path
            .ok_or_else(|| "missing --history-path for moat controller-step".to_string())?,
        round_id: base.round_id,
        role: base.role,
        kind: base.kind,
        node_id: base.node_id,
        depends_on: base.depends_on,
        no_dependencies: base.no_dependencies,
        requires_artifacts: base.requires_artifacts,
        title_contains: base.title_contains,
        spec_ref: base.spec_ref,
        agent_id: base.agent_id,
        lease_seconds,
        dry_run: base.dry_run,
        format: base.format,
    })
}

#[derive(Default)]
struct ControllerArgs {
    history_path: Option<String>,
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
    agent_id: Option<String>,
    lease_seconds: Option<i64>,
    dry_run: bool,
    format: OutputFormat,
}

fn parse_controller_flags(args: &[String], command_name: &str) -> Result<ControllerArgs, String> {
    let mut parsed = ControllerArgs::default();
    let mut saw_format = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--history-path" => {
                let value =
                    required_flag_value(args, index, &format!("{command_name} --history-path"))?;
                if parsed.history_path.is_some() {
                    return Err(format!("duplicate {command_name} --history-path"));
                }
                parsed.history_path = Some(value.to_string());
                index += 2;
            }
            "--round-id" => {
                let value =
                    required_flag_value(args, index, &format!("{command_name} --round-id"))?;
                if parsed.round_id.is_some() {
                    return Err(format!("duplicate {command_name} --round-id"));
                }
                parsed.round_id = Some(value.to_string());
                index += 2;
            }
            "--role" => {
                let value = required_flag_value(args, index, &format!("{command_name} --role"))?;
                if parsed.role.is_some() {
                    return Err(format!("duplicate {command_name} --role"));
                }
                if !CONTROLLER_ROLES.contains(&value) {
                    return Err(format!(
                        "invalid {command_name} --role: {value} (expected planner|coder|reviewer)"
                    ));
                }
                parsed.role = Some(value.to_string());
                index += 2;
            }
            "--kind" => {
                let value = required_flag_value(args, index, &format!("{command_name} --kind"))?;
                if parsed.kind.is_some() {
                    return Err(format!("duplicate {command_name} --kind"));
                }
                parsed.kind = Some(value.to_string());
                index += 2;
            }
            "--node-id" => {
                let value = required_flag_value(args, index, &format!("{command_name} --node-id"))?;
                if parsed.node_id.is_some() {
                    return Err(format!("duplicate {command_name} --node-id"));
                }
                parsed.node_id = Some(value.to_string());
                index += 2;
            }
            "--depends-on" => {
                let value =
                    required_flag_value(args, index, &format!("{command_name} --depends-on"))?;
                if parsed.depends_on.is_some() {
                    return Err(format!("duplicate {command_name} --depends-on"));
                }
                parsed.depends_on = Some(value.to_string());
                index += 2;
            }
            "--no-dependencies" => {
                if parsed.no_dependencies {
                    return Err(format!("duplicate {command_name} --no-dependencies"));
                }
                parsed.no_dependencies = true;
                index += 1;
            }
            "--requires-artifacts" => {
                if parsed.requires_artifacts {
                    return Err(format!("duplicate {command_name} --requires-artifacts"));
                }
                parsed.requires_artifacts = true;
                index += 1;
            }
            "--title-contains" => {
                let value =
                    required_flag_value(args, index, &format!("{command_name} --title-contains"))?;
                if parsed.title_contains.is_some() {
                    return Err(format!("duplicate {command_name} --title-contains"));
                }
                parsed.title_contains = Some(value.to_string());
                index += 2;
            }
            "--spec-ref" => {
                let value =
                    required_flag_value(args, index, &format!("{command_name} --spec-ref"))?;
                if parsed.spec_ref.is_some() {
                    return Err(format!("duplicate {command_name} --spec-ref"));
                }
                parsed.spec_ref = Some(value.to_string());
                index += 2;
            }
            "--limit" => {
                let value = required_flag_value(args, index, &format!("{command_name} --limit"))?;
                if parsed.limit.is_some() {
                    return Err(format!("duplicate {command_name} --limit"));
                }
                parsed.limit = Some(
                    value
                        .parse::<usize>()
                        .map_err(|_| format!("invalid {command_name} --limit: {value}"))?,
                );
                index += 2;
            }
            "--agent-id" => {
                let value =
                    required_flag_value(args, index, &format!("{command_name} --agent-id"))?;
                if parsed.agent_id.is_some() {
                    return Err(format!("duplicate {command_name} --agent-id"));
                }
                parsed.agent_id = Some(value.to_string());
                index += 2;
            }
            "--lease-seconds" => {
                let value =
                    required_flag_value(args, index, &format!("{command_name} --lease-seconds"))?;
                if parsed.lease_seconds.is_some() {
                    return Err(format!("duplicate {command_name} --lease-seconds"));
                }
                parsed.lease_seconds = Some(
                    value
                        .parse::<i64>()
                        .map_err(|_| format!("invalid {command_name} --lease-seconds: {value}"))?,
                );
                index += 2;
            }
            "--dry-run" => {
                if parsed.dry_run {
                    return Err(format!("duplicate {command_name} --dry-run"));
                }
                parsed.dry_run = true;
                index += 1;
            }
            "--format" => {
                let value = required_flag_value(args, index, &format!("{command_name} --format"))?;
                if saw_format {
                    return Err(format!("duplicate {command_name} --format"));
                }
                parsed.format = match value {
                    "text" => OutputFormat::Text,
                    "json" => OutputFormat::Json,
                    other => return Err(format!("unknown {} format: {other}", command_name)),
                };
                saw_format = true;
                index += 2;
            }
            flag => return Err(format!("unknown option for {command_name}: {flag}")),
        }
    }

    if parsed.depends_on.is_some() && parsed.no_dependencies {
        return Err(format!(
            "{command_name} cannot combine --depends-on and --no-dependencies"
        ));
    }

    Ok(parsed)
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
    let history = load_history(&command.history_path)?;
    let (round_id, nodes) = load_round_nodes(&history, command.round_id.as_deref())?;
    let mut packets = filter_work_packets(nodes, command)?;
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
                    json!({
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
            let envelope = json!({
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

fn run_moat_controller_step(command: &MoatControllerStepCommand) -> Result<(), String> {
    let mut history = load_history(&command.history_path)?;

    if !command.dry_run && command.agent_id.is_none() {
        return Err("moat controller-step requires --agent-id unless --dry-run is set".to_string());
    }

    let entry_index = find_entry_index(&history, command.round_id.as_deref())?;
    let round_id = entry_round_id(&history["entries"][entry_index]).unwrap_or_default();
    let nodes = history["entries"][entry_index]["report"]["control_plane"]["task_graph"]["nodes"]
        .as_array()
        .ok_or_else(|| "invalid moat history file: missing task graph nodes".to_string())?
        .clone();

    let mut matches = filter_work_packets(&nodes, command)?;
    let selected = if matches.is_empty() {
        return Err("no ready moat task matched dispatch filters".to_string());
    } else if matches.len() > 1 {
        return Err(format!(
            "moat controller-step matched {} ready tasks; refine filters with --node-id, --role, --kind, or --depends-on",
            matches.len()
        ));
    } else {
        matches.pop().expect("checked non-empty matches")
    };
    let dependency_artifacts = collect_dependency_artifacts(&nodes, &selected.dependencies)?;

    if !command.dry_run {
        let node_array = history["entries"][entry_index]["report"]["control_plane"]["task_graph"]
            ["nodes"]
            .as_array_mut()
            .ok_or_else(|| "invalid moat history file: missing task graph nodes".to_string())?;
        let selected_node = node_array
            .iter_mut()
            .find(|node| {
                node.get("node_id").and_then(Value::as_str) == Some(selected.node_id.as_str())
            })
            .ok_or_else(|| "no ready moat task matched dispatch filters".to_string())?;
        if selected_node.get("state").and_then(Value::as_str) != Some("ready") {
            return Err("selected moat task is no longer ready; retry controller-step".to_string());
        }
        selected_node["state"] = Value::String("in_progress".to_string());
        selected_node["assigned_agent_id"] = command
            .agent_id
            .as_ref()
            .map(|value| Value::String(value.clone()))
            .unwrap_or(Value::Null);
        selected_node["lease_seconds"] = Value::Number(command.lease_seconds.into());
        save_history(&command.history_path, &history)?;
    }

    let work_packet = json!({
        "type": "moat_work_packet",
        "history_path": command.history_path,
        "round_id": round_id,
        "node_id": selected.node_id,
        "role": selected.role,
        "kind": selected.kind,
        "title": selected.title,
        "dependencies": selected.dependencies,
        "state": if command.dry_run { "ready" } else { "in_progress" },
        "spec_ref": selected.spec_ref,
        "dependency_artifacts": dependency_artifacts.iter().map(|artifact| json!({
            "node_id": artifact.node_id,
            "artifact_ref": artifact.artifact_ref,
            "artifact_summary": artifact.artifact_summary,
        })).collect::<Vec<_>>(),
        "acceptance_criteria": CONTROLLER_ACCEPTANCE,
    });

    match command.format {
        OutputFormat::Json => {
            let mut envelope = json!({
                "type": "moat_controller_step",
                "history_path": command.history_path,
                "dry_run": command.dry_run,
                "claimed": !command.dry_run,
                "agent_id": command.agent_id,
                "assigned_agent_id": if command.dry_run { Value::Null } else { command.agent_id.as_ref().map(|value| Value::String(value.clone())).unwrap_or(Value::Null) },
                "round_id": round_id,
                "node_id": selected.node_id,
                "role": selected.role,
                "kind": selected.kind,
                "title": selected.title,
                "dependencies": selected.dependencies,
                "spec_ref": selected.spec_ref,
                "work_packet": work_packet,
                "constraints": {
                    "local_only": true,
                    "bounded_one_task": true,
                    "no_agent_launch": true,
                    "no_daemon": true,
                    "no_background_work": true,
                    "no_crawling": true,
                    "no_pr_creation": true,
                    "no_cron_creation": true,
                    "no_code_writes": true,
                    "no_artifact_writes": true
                }
            });
            if !command.dry_run {
                envelope["previous_state"] = Value::String("ready".to_string());
                envelope["new_state"] = Value::String("in_progress".to_string());
                envelope["lease_seconds"] = Value::Number(command.lease_seconds.into());
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&envelope).map_err(|error| format!(
                    "failed to serialize controller-step envelope: {error}"
                ))?
            );
        }
        OutputFormat::Text => {
            println!("moat controller step");
            println!("history_path={}", command.history_path);
            println!("dry_run={}", command.dry_run);
            println!("claimed={}", !command.dry_run);
            println!(
                "agent_id={}",
                command.agent_id.as_deref().unwrap_or("<none>")
            );
            println!(
                "assigned_agent_id={}",
                if command.dry_run {
                    "<none>"
                } else {
                    command.agent_id.as_deref().unwrap_or("<none>")
                }
            );
            println!("round_id={round_id}");
            println!("node_id={}", selected.node_id);
            println!("role={}", selected.role);
            println!("kind={}", selected.kind);
            println!("title={}", selected.title);
            println!(
                "dependencies={}",
                format_dependencies(&selected.dependencies)
            );
            println!(
                "spec_ref={}",
                selected.spec_ref.as_deref().unwrap_or("<none>")
            );
            for criterion in CONTROLLER_ACCEPTANCE {
                println!("acceptance={criterion}");
            }
            if !command.dry_run {
                println!("lease_seconds={}", command.lease_seconds);
                println!("previous_state=ready");
                println!("new_state=in_progress");
            }
        }
    }

    Ok(())
}

fn load_history(path: &str) -> Result<Value, String> {
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("failed to read moat history file: {error}"))?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("failed to parse moat history file: {error}"))
}

fn save_history(path: &str, history: &Value) -> Result<(), String> {
    let serialized = serde_json::to_string(history)
        .map_err(|error| format!("failed to serialize moat history file: {error}"))?;
    let target = Path::new(path);
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    let file_name = target
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("history.json");
    let temp_path = parent.join(format!(".{file_name}.tmp-{}", process::id()));
    fs::write(&temp_path, serialized)
        .map_err(|error| format!("failed to write moat history temp file: {error}"))?;
    fs::rename(&temp_path, target).map_err(|error| {
        let _ = fs::remove_file(&temp_path);
        format!("failed to replace moat history file: {error}")
    })
}

fn load_round_nodes<'a>(
    history: &'a Value,
    round_id: Option<&str>,
) -> Result<(String, &'a [Value]), String> {
    let entry = select_entry(history, round_id)?;
    let round_id = entry_round_id(entry).unwrap_or_default();
    let nodes = entry
        .get("report")
        .and_then(|value| value.get("control_plane"))
        .and_then(|value| value.get("task_graph"))
        .and_then(|value| value.get("nodes"))
        .and_then(Value::as_array)
        .ok_or_else(|| "invalid moat history file: missing task graph nodes".to_string())?;
    Ok((round_id, nodes))
}

fn select_entry<'a>(history: &'a Value, round_id: Option<&str>) -> Result<&'a Value, String> {
    let entries = history
        .get("entries")
        .and_then(Value::as_array)
        .ok_or_else(|| "invalid moat history file: missing entries array".to_string())?;
    if entries.is_empty() {
        return Err(
            "moat history is empty; provide a history file with at least one round".to_string(),
        );
    }
    if let Some(round_id) = round_id {
        entries
            .iter()
            .find(|entry| entry_round_id(entry).as_deref() == Some(round_id))
            .ok_or_else(|| format!("unknown moat round-id: {round_id}"))
    } else {
        Ok(entries.last().expect("entries not empty"))
    }
}

fn find_entry_index(history: &Value, round_id: Option<&str>) -> Result<usize, String> {
    let entries = history
        .get("entries")
        .and_then(Value::as_array)
        .ok_or_else(|| "invalid moat history file: missing entries array".to_string())?;
    if entries.is_empty() {
        return Err(
            "moat history is empty; provide a history file with at least one round".to_string(),
        );
    }
    if let Some(round_id) = round_id {
        entries
            .iter()
            .position(|entry| entry_round_id(entry).as_deref() == Some(round_id))
            .ok_or_else(|| format!("unknown moat round-id: {round_id}"))
    } else {
        Ok(entries.len() - 1)
    }
}

fn entry_round_id(entry: &Value) -> Option<String> {
    entry
        .get("report")?
        .get("summary")?
        .get("round_id")?
        .as_str()
        .map(ToOwned::to_owned)
}

trait ControllerFilterCommand {
    fn role(&self) -> Option<&str>;
    fn kind(&self) -> Option<&str>;
    fn node_id(&self) -> Option<&str>;
    fn depends_on(&self) -> Option<&str>;
    fn no_dependencies(&self) -> bool;
    fn requires_artifacts(&self) -> bool;
    fn title_contains(&self) -> Option<&str>;
    fn spec_ref(&self) -> Option<&str>;
}

impl ControllerFilterCommand for MoatControllerPlanCommand {
    fn role(&self) -> Option<&str> {
        self.role.as_deref()
    }
    fn kind(&self) -> Option<&str> {
        self.kind.as_deref()
    }
    fn node_id(&self) -> Option<&str> {
        self.node_id.as_deref()
    }
    fn depends_on(&self) -> Option<&str> {
        self.depends_on.as_deref()
    }
    fn no_dependencies(&self) -> bool {
        self.no_dependencies
    }
    fn requires_artifacts(&self) -> bool {
        self.requires_artifacts
    }
    fn title_contains(&self) -> Option<&str> {
        self.title_contains.as_deref()
    }
    fn spec_ref(&self) -> Option<&str> {
        self.spec_ref.as_deref()
    }
}

impl ControllerFilterCommand for MoatControllerStepCommand {
    fn role(&self) -> Option<&str> {
        self.role.as_deref()
    }
    fn kind(&self) -> Option<&str> {
        self.kind.as_deref()
    }
    fn node_id(&self) -> Option<&str> {
        self.node_id.as_deref()
    }
    fn depends_on(&self) -> Option<&str> {
        self.depends_on.as_deref()
    }
    fn no_dependencies(&self) -> bool {
        self.no_dependencies
    }
    fn requires_artifacts(&self) -> bool {
        self.requires_artifacts
    }
    fn title_contains(&self) -> Option<&str> {
        self.title_contains.as_deref()
    }
    fn spec_ref(&self) -> Option<&str> {
        self.spec_ref.as_deref()
    }
}

fn filter_work_packets<C: ControllerFilterCommand>(
    nodes: &[Value],
    command: &C,
) -> Result<Vec<WorkPacket>, String> {
    Ok(nodes
        .iter()
        .map(parse_work_packet)
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|packet| packet.state == "ready")
        .filter(|packet| command.role().map(|v| packet.role == v).unwrap_or(true))
        .filter(|packet| command.kind().map(|v| packet.kind == v).unwrap_or(true))
        .filter(|packet| {
            command
                .node_id()
                .map(|v| packet.node_id == v)
                .unwrap_or(true)
        })
        .filter(|packet| {
            command
                .depends_on()
                .map(|v| packet.dependencies.iter().any(|dep| dep == v))
                .unwrap_or(true)
        })
        .filter(|packet| !command.no_dependencies() || packet.dependencies.is_empty())
        .filter(|packet| {
            command
                .title_contains()
                .map(|v| packet.title.contains(v))
                .unwrap_or(true)
        })
        .filter(|packet| {
            command
                .spec_ref()
                .map(|v| packet.spec_ref.as_deref() == Some(v))
                .unwrap_or(true)
        })
        .filter(|packet| {
            !command.requires_artifacts() || dependencies_have_artifacts(nodes, packet)
        })
        .collect::<Vec<_>>())
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

fn collect_dependency_artifacts(
    nodes: &[Value],
    dependencies: &[String],
) -> Result<Vec<DependencyArtifact>, String> {
    let mut results = Vec::new();
    for dependency in dependencies {
        let Some(node) = nodes
            .iter()
            .find(|node| node.get("node_id").and_then(Value::as_str) == Some(dependency.as_str()))
        else {
            continue;
        };
        if node.get("state").and_then(Value::as_str) != Some("completed") {
            continue;
        }
        if let Some(artifacts) = node.get("artifacts").and_then(Value::as_array) {
            for artifact in artifacts {
                let artifact_ref = artifact
                    .get("artifact_ref")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        format!(
                            "invalid moat history file: dependency {dependency} artifact_ref must be a string"
                        )
                    })?;
                let artifact_summary = artifact
                    .get("summary")
                    .or_else(|| artifact.get("artifact_summary"))
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        format!(
                            "invalid moat history file: dependency {dependency} artifact summary must be a string"
                        )
                    })?;
                results.push(DependencyArtifact {
                    node_id: dependency.clone(),
                    artifact_ref: artifact_ref.to_string(),
                    artifact_summary: artifact_summary.to_string(),
                });
            }
        }
    }
    Ok(results)
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
    eprintln!("usage: mdid-cli [status | moat controller-plan --history-path PATH [--round-id ROUND_ID] [--role planner|coder|reviewer] [--kind KIND] [--node-id NODE_ID] [--depends-on NODE_ID] [--no-dependencies] [--requires-artifacts] [--title-contains TEXT] [--spec-ref SPEC_REF] [--limit N] [--format text|json] | moat controller-step --history-path PATH [--round-id ROUND_ID] [--role planner|coder|reviewer] [--kind KIND] [--node-id NODE_ID] [--depends-on NODE_ID] [--no-dependencies] [--requires-artifacts] [--title-contains TEXT] [--spec-ref SPEC_REF] [--agent-id AGENT_ID] [--lease-seconds N] [--dry-run] [--format text|json]]");
    process::exit(1);
}
