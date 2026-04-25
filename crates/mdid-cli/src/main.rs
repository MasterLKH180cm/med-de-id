use mdid_application::{render_moat_plan_markdown, render_moat_spec_markdown, MoatAgentAssignment};
use mdid_domain::{
    AgentRole, CompetitorProfile, ContinueDecision, LockInReport, MarketMoatSnapshot, MoatStrategy,
    MoatTaskNodeKind, MoatTaskNodeState, MoatType, ResourceBudget,
};
use mdid_runtime::{
    moat::{run_bounded_round, MoatRoundInput, MoatRoundReport},
    moat_history::{LocalMoatHistoryStore, MoatHistorySummary},
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct MoatRoundOverrides {
    strategy_candidates: Option<u8>,
    spec_generations: Option<u8>,
    implementation_tasks: Option<u8>,
    review_loops: Option<u8>,
    tests_passed: Option<bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct MoatRoundCommand {
    overrides: MoatRoundOverrides,
    history_path: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct MoatControlPlaneCommand {
    overrides: MoatRoundOverrides,
    history_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MoatDecisionLogCommand {
    history_path: String,
    role: Option<AgentRole>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MoatAssignmentsCommand {
    history_path: String,
    role: Option<AgentRole>,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match parse_command(&args) {
        Ok(CliCommand::Status) => println!("med-de-id CLI ready"),
        Ok(CliCommand::MoatRound(command)) => {
            if let Err(error) = run_moat_round(&command) {
                exit_with_error(error);
            }
        }
        Ok(CliCommand::MoatControlPlane(command)) => {
            if let Err(error) = run_moat_control_plane(&command) {
                exit_with_error(error);
            }
        }
        Ok(CliCommand::MoatHistory(history_path)) => {
            if let Err(error) = run_moat_history(&history_path) {
                exit_with_error(error);
            }
        }
        Ok(CliCommand::MoatDecisionLog(command)) => {
            if let Err(error) = run_moat_decision_log(&command) {
                exit_with_error(error);
            }
        }
        Ok(CliCommand::MoatAssignments(command)) => {
            if let Err(error) = run_moat_assignments(&command) {
                exit_with_error(error);
            }
        }
        Ok(CliCommand::MoatExportSpecs {
            history_path,
            output_dir,
        }) => {
            if let Err(error) = run_moat_export_specs(&history_path, &output_dir) {
                exit_with_error(error);
            }
        }
        Ok(CliCommand::MoatExportPlans {
            history_path,
            output_dir,
        }) => {
            if let Err(error) = run_moat_export_plans(&history_path, &output_dir) {
                exit_with_error(error);
            }
        }
        Ok(CliCommand::MoatContinue {
            history_path,
            improvement_threshold,
        }) => {
            if let Err(error) = run_moat_continue(&history_path, improvement_threshold) {
                exit_with_error(error);
            }
        }
        Ok(CliCommand::MoatScheduleNext {
            history_path,
            improvement_threshold,
        }) => {
            if let Err(error) = run_moat_schedule_next(&history_path, improvement_threshold) {
                exit_with_error(error);
            }
        }
        Err(error) => exit_with_usage(error),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CliCommand {
    Status,
    MoatRound(MoatRoundCommand),
    MoatControlPlane(MoatControlPlaneCommand),
    MoatHistory(String),
    MoatDecisionLog(MoatDecisionLogCommand),
    MoatAssignments(MoatAssignmentsCommand),
    MoatExportSpecs {
        history_path: String,
        output_dir: String,
    },
    MoatExportPlans {
        history_path: String,
        output_dir: String,
    },
    MoatContinue {
        history_path: String,
        improvement_threshold: i16,
    },
    MoatScheduleNext {
        history_path: String,
        improvement_threshold: i16,
    },
}

fn parse_command(args: &[String]) -> Result<CliCommand, String> {
    match args {
        [] => Ok(CliCommand::Status),
        [status] if status == "status" => Ok(CliCommand::Status),
        [moat, round, rest @ ..] if moat == "moat" && round == "round" => {
            Ok(CliCommand::MoatRound(parse_moat_round_command(rest)?))
        }
        [moat, control_plane, rest @ ..] if moat == "moat" && control_plane == "control-plane" => {
            Ok(CliCommand::MoatControlPlane(
                parse_moat_control_plane_command(rest)?,
            ))
        }
        [moat, history, rest @ ..] if moat == "moat" && history == "history" => {
            Ok(CliCommand::MoatHistory(parse_required_history_path(rest)?))
        }
        [moat, decision_log, rest @ ..] if moat == "moat" && decision_log == "decision-log" => Ok(
            CliCommand::MoatDecisionLog(parse_moat_decision_log_command(rest)?),
        ),
        [moat, assignments, rest @ ..] if moat == "moat" && assignments == "assignments" => Ok(
            CliCommand::MoatAssignments(parse_moat_assignments_command(rest)?),
        ),
        [moat, export_specs, rest @ ..] if moat == "moat" && export_specs == "export-specs" => {
            parse_moat_export_specs_command(rest)
        }
        [moat, export_plans, rest @ ..] if moat == "moat" && export_plans == "export-plans" => {
            parse_moat_export_plans_command(rest)
        }
        [moat, continue_command, rest @ ..] if moat == "moat" && continue_command == "continue" => {
            parse_moat_continue_command(rest)
        }
        [moat, schedule_next, rest @ ..] if moat == "moat" && schedule_next == "schedule-next" => {
            parse_moat_schedule_next_command(rest)
        }
        _ => Err(format!("unknown command: {}", format_command(args))),
    }
}

fn parse_moat_round_command(args: &[String]) -> Result<MoatRoundCommand, String> {
    let (overrides, history_path) = parse_moat_round_overrides(args, true)?;
    Ok(MoatRoundCommand {
        overrides,
        history_path,
    })
}

fn parse_moat_control_plane_command(args: &[String]) -> Result<MoatControlPlaneCommand, String> {
    let (overrides, history_path) = parse_moat_round_overrides(args, true)?;
    if history_path.is_some() && overrides != MoatRoundOverrides::default() {
        return Err("cannot combine --history-path with control-plane override flags".to_string());
    }
    Ok(MoatControlPlaneCommand {
        overrides,
        history_path,
    })
}

fn parse_moat_decision_log_command(args: &[String]) -> Result<MoatDecisionLogCommand, String> {
    let mut history_path = None;
    let mut role = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--history-path" => {
                let value = required_history_path_value(args, index)?.clone();
                if history_path.is_some() {
                    return Err(duplicate_flag_error("--history-path"));
                }
                history_path = Some(value);
            }
            "--role" => {
                let value = required_flag_value(args, index, "--role", false)?;
                if role.is_some() {
                    return Err(duplicate_flag_error("--role"));
                }
                role = Some(parse_agent_role_filter(value)?);
            }
            flag => return Err(format!("unknown flag: {flag}")),
        }

        index += 2;
    }

    Ok(MoatDecisionLogCommand {
        history_path: history_path
            .ok_or_else(|| "missing required flag: --history-path".to_string())?,
        role,
    })
}

fn parse_agent_role_filter(value: &str) -> Result<AgentRole, String> {
    match value {
        "planner" => Ok(AgentRole::Planner),
        "coder" => Ok(AgentRole::Coder),
        "reviewer" => Ok(AgentRole::Reviewer),
        other => Err(format!("unknown moat decision-log role: {other}")),
    }
}

fn parse_moat_assignments_command(args: &[String]) -> Result<MoatAssignmentsCommand, String> {
    let mut history_path = None;
    let mut role = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--history-path" => {
                let value = required_history_path_value(args, index)?.clone();
                if history_path.is_some() {
                    return Err(duplicate_flag_error("--history-path"));
                }
                history_path = Some(value);
            }
            "--role" => {
                let value = required_flag_value(args, index, "--role", false)?;
                if role.is_some() {
                    return Err(duplicate_flag_error("--role"));
                }
                role = Some(parse_moat_assignments_role_filter(value)?);
            }
            flag => return Err(format!("unknown flag: {flag}")),
        }

        index += 2;
    }

    Ok(MoatAssignmentsCommand {
        history_path: history_path
            .ok_or_else(|| "missing required flag: --history-path".to_string())?,
        role,
    })
}

fn parse_moat_assignments_role_filter(value: &str) -> Result<AgentRole, String> {
    match value {
        "planner" => Ok(AgentRole::Planner),
        "coder" => Ok(AgentRole::Coder),
        "reviewer" => Ok(AgentRole::Reviewer),
        other => Err(format!("unknown moat assignments role: {other}")),
    }
}

fn parse_moat_export_specs_command(args: &[String]) -> Result<CliCommand, String> {
    let mut history_path = None;
    let mut output_dir = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--history-path" => {
                let value = required_history_path_value(args, index)?.clone();
                if history_path.is_some() {
                    return Err(duplicate_flag_error("--history-path"));
                }
                history_path = Some(value);
            }
            "--output-dir" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --output-dir".to_string())?;
                if value.starts_with("--") {
                    return Err("missing value for --output-dir".to_string());
                }
                if output_dir.is_some() {
                    return Err(duplicate_flag_error("--output-dir"));
                }
                output_dir = Some(value.clone());
            }
            flag => return Err(format!("unknown flag: {flag}")),
        }

        index += 2;
    }

    Ok(CliCommand::MoatExportSpecs {
        history_path: history_path
            .ok_or_else(|| "missing required flag: --history-path".to_string())?,
        output_dir: output_dir.ok_or_else(|| "missing required flag: --output-dir".to_string())?,
    })
}

fn parse_moat_export_plans_command(args: &[String]) -> Result<CliCommand, String> {
    let CliCommand::MoatExportSpecs {
        history_path,
        output_dir,
    } = parse_moat_export_specs_command(args)?
    else {
        unreachable!("export specs parser returns export specs command")
    };
    Ok(CliCommand::MoatExportPlans {
        history_path,
        output_dir,
    })
}

fn parse_moat_continue_command(args: &[String]) -> Result<CliCommand, String> {
    let mut history_path = None;
    let mut improvement_threshold = 3;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--history-path" => {
                let value = required_history_path_value(args, index)?.clone();
                if history_path.is_some() {
                    return Err(duplicate_flag_error("--history-path"));
                }
                history_path = Some(value);
            }
            "--improvement-threshold" => {
                let value = required_flag_value(args, index, "--improvement-threshold", false)?;
                improvement_threshold =
                    parse_non_negative_i16_flag("--improvement-threshold", value)?;
            }
            flag => return Err(format!("unknown flag: {flag}")),
        }

        index += 2;
    }

    Ok(CliCommand::MoatContinue {
        history_path: history_path
            .ok_or_else(|| "missing required flag: --history-path".to_string())?,
        improvement_threshold,
    })
}

fn parse_moat_schedule_next_command(args: &[String]) -> Result<CliCommand, String> {
    let mut history_path = None;
    let mut improvement_threshold = 3;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--history-path" => {
                let value = required_history_path_value(args, index)?.clone();
                if history_path.is_some() {
                    return Err(duplicate_flag_error("--history-path"));
                }
                history_path = Some(value);
            }
            "--improvement-threshold" => {
                let value = required_flag_value(args, index, "--improvement-threshold", false)?;
                improvement_threshold =
                    parse_non_negative_i16_flag("--improvement-threshold", value)?;
            }
            flag => return Err(format!("unknown flag: {flag}")),
        }

        index += 2;
    }

    Ok(CliCommand::MoatScheduleNext {
        history_path: history_path
            .ok_or_else(|| "missing required flag: --history-path".to_string())?,
        improvement_threshold,
    })
}

fn parse_moat_round_overrides(
    args: &[String],
    allow_history_path: bool,
) -> Result<(MoatRoundOverrides, Option<String>), String> {
    let mut overrides = MoatRoundOverrides::default();
    let mut history_path = None;
    let mut index = 0;

    while index < args.len() {
        let flag = &args[index];

        match flag.as_str() {
            "--strategy-candidates" => {
                let value = required_flag_value(args, index, flag, allow_history_path)?;
                overrides.strategy_candidates = Some(parse_u8_flag(flag, value)?);
            }
            "--spec-generations" => {
                let value = required_flag_value(args, index, flag, allow_history_path)?;
                overrides.spec_generations = Some(parse_u8_flag(flag, value)?);
            }
            "--implementation-tasks" => {
                let value = required_flag_value(args, index, flag, allow_history_path)?;
                overrides.implementation_tasks = Some(parse_u8_flag(flag, value)?);
            }
            "--review-loops" => {
                let value = required_flag_value(args, index, flag, allow_history_path)?;
                overrides.review_loops = Some(parse_u8_flag(flag, value)?);
            }
            "--tests-passed" => {
                let value = required_flag_value(args, index, flag, allow_history_path)?;
                overrides.tests_passed = Some(parse_bool_flag(flag, value)?);
            }
            "--history-path" if allow_history_path => {
                let value = required_flag_value(args, index, flag, allow_history_path)?;
                if history_path.is_some() {
                    return Err(duplicate_flag_error(flag));
                }
                history_path = Some(value.clone());
            }
            _ => return Err(format!("unknown flag: {flag}")),
        }

        index += 2;
    }

    Ok((overrides, history_path))
}

fn required_flag_value<'a>(
    args: &'a [String],
    index: usize,
    flag: &str,
    allow_history_path: bool,
) -> Result<&'a String, String> {
    if allow_history_path && flag == "--history-path" {
        return required_history_path_value(args, index);
    }

    args.get(index + 1)
        .ok_or_else(|| missing_value_error(flag, allow_history_path))
}

fn parse_required_history_path(args: &[String]) -> Result<String, String> {
    let Some(flag) = args.first() else {
        return Err("missing required flag: --history-path".to_string());
    };

    if flag != "--history-path" {
        return Err(format!("unknown flag: {flag}"));
    }

    let history_path = required_history_path_value(args, 0)?.clone();

    if let Some(extra) = args.get(2) {
        if extra == "--history-path" {
            Err(duplicate_flag_error(extra))
        } else {
            Err(format!("unknown flag: {extra}"))
        }
    } else {
        Ok(history_path)
    }
}

fn required_history_path_value<'a>(args: &'a [String], index: usize) -> Result<&'a String, String> {
    let value = args
        .get(index + 1)
        .ok_or_else(|| "missing value for --history-path".to_string())?;

    if history_path_value_is_missing(value) {
        Err("missing value for --history-path".to_string())
    } else {
        Ok(value)
    }
}

fn history_path_value_is_missing(value: &str) -> bool {
    value.starts_with("--")
}

fn missing_value_error(flag: &str, allow_history_path: bool) -> String {
    if allow_history_path && flag == "--history-path" {
        "missing value for --history-path".to_string()
    } else {
        format!("missing value for {flag}")
    }
}

fn duplicate_flag_error(flag: &str) -> String {
    format!("duplicate flag: {flag}")
}

fn parse_u8_flag(flag: &str, value: &str) -> Result<u8, String> {
    value
        .parse::<u8>()
        .map_err(|_| format!("invalid value for {flag}: {value}"))
}

fn parse_non_negative_i16_flag(flag: &str, value: &str) -> Result<i16, String> {
    let parsed = value
        .parse::<i16>()
        .map_err(|_| format!("invalid value for {flag}: {value}"))?;

    if parsed < 0 {
        Err(format!("invalid value for {flag}: {value}"))
    } else {
        Ok(parsed)
    }
}

fn parse_bool_flag(flag: &str, value: &str) -> Result<bool, String> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("invalid value for {flag}: {value}")),
    }
}

fn run_moat_round(command: &MoatRoundCommand) -> Result<(), String> {
    let report = sample_round_report(&command.overrides);

    if let Some(history_path) = &command.history_path {
        append_report_to_history(history_path, &report)?;
    }

    println!("moat round complete");
    println!(
        "continue_decision={}",
        format_continue_decision(report.summary.continue_decision)
    );
    println!("executed_tasks={}", report.executed_tasks.join(","));
    println!(
        "implemented_specs={}",
        format_string_list(&report.summary.implemented_specs)
    );
    println!("moat_score_before={}", report.summary.moat_score_before);
    println!("moat_score_after={}", report.summary.moat_score_after);
    println!(
        "stop_reason={}",
        report.stop_reason.as_deref().unwrap_or("<none>")
    );

    if let Some(history_path) = &command.history_path {
        println!("history_saved_to={history_path}");
    }

    Ok(())
}

fn append_report_to_history(history_path: &str, report: &MoatRoundReport) -> Result<(), String> {
    let mut store = LocalMoatHistoryStore::open(history_path)
        .map_err(|error| format!("failed to open moat history store: {error}"))?;
    store
        .append(std::time::SystemTime::now().into(), report.clone())
        .map_err(|error| format!("failed to append moat history entry: {error}"))
}

fn run_moat_control_plane(command: &MoatControlPlaneCommand) -> Result<(), String> {
    if let Some(history_path) = &command.history_path {
        return run_persisted_moat_control_plane(history_path);
    }

    let report = sample_round_report(&command.overrides);
    print_control_plane_snapshot("sample", None, None, &report);
    Ok(())
}

fn run_persisted_moat_control_plane(history_path: &str) -> Result<(), String> {
    let store = LocalMoatHistoryStore::open_existing(history_path)
        .map_err(|error| format!("failed to open moat history store: {error}"))?;
    let latest = store.entries().last().ok_or_else(|| {
        "moat history is empty; run `mdid-cli moat round --history-path <path>` first".to_string()
    })?;

    let latest_round_id = latest.report.summary.round_id.to_string();
    print_control_plane_snapshot(
        "history",
        Some(history_path),
        Some(latest_round_id.as_str()),
        &latest.report,
    );
    Ok(())
}

fn print_control_plane_snapshot(
    source: &str,
    history_path: Option<&str>,
    latest_round_id: Option<&str>,
    report: &MoatRoundReport,
) {
    let control_plane = &report.control_plane;
    let ready_nodes = format_ready_nodes(&control_plane.task_graph.ready_node_ids());
    let latest_decision_summary = control_plane
        .memory
        .latest_decision_summary()
        .unwrap_or_else(|| "<none>".to_string());
    let task_states = format_task_states(&control_plane.task_graph.nodes);

    println!("moat control plane snapshot");
    println!("source={source}");
    if let Some(latest_round_id) = latest_round_id {
        println!("latest_round_id={latest_round_id}");
    }
    if let Some(history_path) = history_path {
        println!("history_path={history_path}");
    }
    println!("ready_nodes={ready_nodes}");
    println!("latest_decision_summary={latest_decision_summary}");
    println!(
        "improvement_delta={}",
        control_plane.memory.improvement_delta
    );
    println!(
        "agent_assignments={}",
        format_agent_assignments(&control_plane.agent_assignments)
    );
    println!("task_states={task_states}");
}

fn run_moat_history(history_path: &str) -> Result<(), String> {
    let store = LocalMoatHistoryStore::open_existing(history_path)
        .map_err(|error| format!("failed to open moat history store: {error}"))?;
    print_history_summary(&store.summary());
    Ok(())
}

fn run_moat_decision_log(command: &MoatDecisionLogCommand) -> Result<(), String> {
    let store = LocalMoatHistoryStore::open_existing(&command.history_path)
        .map_err(|error| format!("failed to open moat history store: {error}"))?;
    let latest = store.entries().last().ok_or_else(|| {
        "moat history is empty; run `mdid-cli moat round --history-path <path>` first".to_string()
    })?;
    let decisions = latest
        .report
        .control_plane
        .memory
        .decisions
        .iter()
        .filter(|decision| {
            command
                .role
                .map(|role| decision.author_role == role)
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();

    println!("decision_log_entries={}", decisions.len());
    for decision in decisions {
        println!(
            "decision={}|{}|{}",
            format_agent_role(decision.author_role),
            decision.summary,
            decision.rationale
        );
    }

    Ok(())
}

fn run_moat_assignments(command: &MoatAssignmentsCommand) -> Result<(), String> {
    let store = LocalMoatHistoryStore::open_existing(&command.history_path)
        .map_err(|error| format!("failed to open moat history store: {error}"))?;
    let latest = store.entries().last().ok_or_else(|| {
        "moat history is empty; run `mdid-cli moat round --history-path <path>` first".to_string()
    })?;

    let assignments = latest
        .report
        .control_plane
        .agent_assignments
        .iter()
        .filter(|assignment| {
            command
                .role
                .map(|role| assignment.role == role)
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();

    println!("moat assignments");
    println!("assignment_entries={}", assignments.len());
    for assignment in assignments {
        println!(
            "assignment={}|{}|{}|{}|{}",
            format_agent_role(assignment.role),
            escape_assignment_output_field(&assignment.node_id),
            escape_assignment_output_field(&assignment.title),
            format_moat_task_kind(assignment.kind),
            escape_assignment_output_field(assignment.spec_ref.as_deref().unwrap_or("<none>"))
        );
    }

    Ok(())
}

fn escape_assignment_output_field(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn run_moat_continue(history_path: &str, improvement_threshold: i16) -> Result<(), String> {
    let store = LocalMoatHistoryStore::open_existing(history_path)
        .map_err(|error| format!("failed to open moat history store: {error}"))?;
    let gate = store.continuation_gate(improvement_threshold);

    println!("moat continuation gate");
    println!(
        "latest_round_id={}",
        gate.latest_round_id.as_deref().unwrap_or("<none>")
    );
    println!(
        "latest_continue_decision={}",
        gate.latest_continue_decision
            .map(format_continue_decision)
            .unwrap_or("<none>")
    );
    println!(
        "latest_tests_passed={}",
        format_optional_bool(gate.latest_tests_passed)
    );
    println!(
        "latest_improvement_delta={}",
        format_optional_i16(gate.latest_improvement_delta)
    );
    println!(
        "latest_stop_reason={}",
        gate.latest_stop_reason.as_deref().unwrap_or("<none>")
    );
    println!(
        "evaluation_completed={}",
        if gate.evaluation_completed {
            "true"
        } else {
            "false"
        }
    );
    println!(
        "can_continue={}",
        if gate.can_continue { "true" } else { "false" }
    );
    println!("reason={}", gate.reason);
    println!(
        "required_improvement_threshold={}",
        gate.required_improvement_threshold
    );

    Ok(())
}

fn run_moat_schedule_next(history_path: &str, improvement_threshold: i16) -> Result<(), String> {
    let mut store = LocalMoatHistoryStore::open_existing(history_path)
        .map_err(|error| format!("failed to open moat history store: {error}"))?;
    let gate = store.continuation_gate(improvement_threshold);
    let mut scheduled_round_id = None;

    if gate.can_continue {
        let report = sample_round_report(&MoatRoundOverrides::default());
        scheduled_round_id = Some(report.summary.round_id.to_string());
        store
            .append(std::time::SystemTime::now().into(), report)
            .map_err(|error| format!("failed to append moat history entry: {error}"))?;
    }

    println!("moat schedule next");
    println!(
        "scheduled={}",
        if gate.can_continue { "true" } else { "false" }
    );
    println!("reason={}", gate.reason);
    println!(
        "scheduled_round_id={}",
        scheduled_round_id.as_deref().unwrap_or("<none>")
    );
    println!("history_path={history_path}");

    Ok(())
}

fn run_moat_export_specs(history_path: &str, output_dir: &str) -> Result<(), String> {
    let store = LocalMoatHistoryStore::open_existing(history_path)
        .map_err(|error| format!("failed to open moat history store: {error}"))?;
    let latest = store.entries().last().ok_or_else(|| {
        "moat history is empty; run `mdid-cli moat round --history-path <path>` first".to_string()
    })?;

    if latest.report.summary.implemented_specs.is_empty() {
        return Err("latest moat round does not contain implemented_specs handoffs".to_string());
    }

    std::fs::create_dir_all(output_dir)
        .map_err(|error| format!("failed to create export directory: {error}"))?;

    let mut written_files = Vec::new();
    for handoff_id in &latest.report.summary.implemented_specs {
        let markdown = render_moat_spec_markdown(
            handoff_id,
            &latest.report.summary,
            &latest.report.summary.selected_strategies,
        )?;
        let file_name = format!(
            "{}.md",
            handoff_id
                .strip_prefix("moat-spec/")
                .ok_or_else(|| format!("expected moat-spec/ handoff id, got {handoff_id}"))?
        );
        let output_path = std::path::Path::new(output_dir).join(&file_name);
        std::fs::write(&output_path, markdown)
            .map_err(|error| format!("failed to write exported spec markdown: {error}"))?;
        written_files.push(file_name);
    }

    println!("moat spec export complete");
    println!("round_id={}", latest.report.summary.round_id);
    println!(
        "exported_specs={}",
        latest.report.summary.implemented_specs.join(",")
    );
    println!("written_files={}", written_files.join(","));

    Ok(())
}

fn run_moat_export_plans(history_path: &str, output_dir: &str) -> Result<(), String> {
    let store = LocalMoatHistoryStore::open_existing(history_path)
        .map_err(|error| format!("failed to open moat history store: {error}"))?;
    let latest = store.entries().last().ok_or_else(|| {
        "moat history is empty; run `mdid-cli moat round --history-path <path>` first".to_string()
    })?;

    if latest.report.summary.implemented_specs.is_empty() {
        return Err("latest moat round does not contain implemented_specs handoffs".to_string());
    }

    std::fs::create_dir_all(output_dir)
        .map_err(|error| format!("failed to create export directory: {error}"))?;

    let mut written_files = Vec::new();
    for handoff_id in &latest.report.summary.implemented_specs {
        let markdown = render_moat_plan_markdown(
            handoff_id,
            &latest.report.summary,
            &latest.report.summary.selected_strategies,
        )?;
        let slug = handoff_id
            .strip_prefix("moat-spec/")
            .ok_or_else(|| format!("expected moat-spec/ handoff id, got {handoff_id}"))?;
        let file_name = format!("{slug}-implementation-plan.md");
        let output_path = std::path::Path::new(output_dir).join(&file_name);
        std::fs::write(&output_path, markdown)
            .map_err(|error| format!("failed to write exported plan markdown: {error}"))?;
        written_files.push(file_name);
    }

    println!("moat plan export");
    println!(
        "exported_plans={}",
        latest.report.summary.implemented_specs.join(",")
    );
    println!("written_files={}", written_files.join(","));
    println!("output_dir={output_dir}");

    Ok(())
}

fn print_history_summary(summary: &MoatHistorySummary) {
    println!("moat history summary");
    println!("entries={}", summary.entry_count);
    println!(
        "latest_round_id={}",
        summary.latest_round_id.as_deref().unwrap_or("<none>")
    );
    println!(
        "latest_continue_decision={}",
        summary
            .latest_continue_decision
            .map(format_continue_decision)
            .unwrap_or("<none>")
    );
    println!(
        "latest_stop_reason={}",
        summary.latest_stop_reason.as_deref().unwrap_or("<none>")
    );
    println!(
        "latest_decision_summary={}",
        summary
            .latest_decision_summary
            .as_deref()
            .unwrap_or("<none>")
    );
    println!(
        "latest_implemented_specs={}",
        format_string_list(&summary.latest_implemented_specs)
    );
    println!(
        "latest_moat_score_after={}",
        format_optional_i16(summary.latest_moat_score_after)
    );
    println!(
        "best_moat_score_after={}",
        format_optional_i16(summary.best_moat_score_after)
    );
    println!(
        "improvement_deltas={}",
        format_improvement_deltas(&summary.improvement_deltas)
    );
}

fn format_optional_i16(value: Option<i16>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "<none>".to_string())
}

fn format_optional_bool(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "true",
        Some(false) => "false",
        None => "<none>",
    }
}

fn format_improvement_deltas(values: &[i16]) -> String {
    if values.is_empty() {
        "<none>".to_string()
    } else {
        values
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(",")
    }
}

fn format_string_list(values: &[String]) -> String {
    if values.is_empty() {
        "<none>".to_string()
    } else {
        values.join(",")
    }
}

fn sample_round_report(overrides: &MoatRoundOverrides) -> MoatRoundReport {
    run_bounded_round(sample_round_input(overrides))
}

fn format_continue_decision(continue_decision: ContinueDecision) -> &'static str {
    match continue_decision {
        ContinueDecision::Continue => "Continue",
        ContinueDecision::Stop => "Stop",
        ContinueDecision::Pivot => "Pivot",
    }
}

fn format_ready_nodes(ready_nodes: &[String]) -> String {
    if ready_nodes.is_empty() {
        "<none>".to_string()
    } else {
        ready_nodes.join(",")
    }
}

fn format_task_states(nodes: &[mdid_domain::MoatTaskNode]) -> String {
    nodes
        .iter()
        .map(|node| format!("{}:{}", node.node_id, format_task_node_state(node.state)))
        .collect::<Vec<_>>()
        .join(",")
}

fn format_agent_assignments(assignments: &[MoatAgentAssignment]) -> String {
    if assignments.is_empty() {
        "<none>".to_string()
    } else {
        assignments
            .iter()
            .map(|assignment| {
                format!(
                    "{}:{}",
                    format_agent_role(assignment.role),
                    assignment.node_id
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    }
}

fn format_agent_role(role: AgentRole) -> &'static str {
    match role {
        AgentRole::Planner => "planner",
        AgentRole::Coder => "coder",
        AgentRole::Reviewer => "reviewer",
    }
}

fn format_task_node_state(state: MoatTaskNodeState) -> &'static str {
    match state {
        MoatTaskNodeState::Pending => "pending",
        MoatTaskNodeState::Ready => "ready",
        MoatTaskNodeState::InProgress => "in_progress",
        MoatTaskNodeState::Completed => "completed",
        MoatTaskNodeState::Blocked => "blocked",
    }
}

fn format_moat_task_kind(kind: MoatTaskNodeKind) -> &'static str {
    match kind {
        MoatTaskNodeKind::MarketScan => "market_scan",
        MoatTaskNodeKind::CompetitorAnalysis => "competitor_analysis",
        MoatTaskNodeKind::LockInAnalysis => "lockin_analysis",
        MoatTaskNodeKind::StrategyGeneration => "strategy_generation",
        MoatTaskNodeKind::SpecPlanning => "spec_planning",
        MoatTaskNodeKind::Implementation => "implementation",
        MoatTaskNodeKind::Review => "review",
        MoatTaskNodeKind::Evaluation => "evaluation",
    }
}

fn sample_round_input(overrides: &MoatRoundOverrides) -> MoatRoundInput {
    let mut input = MoatRoundInput {
        market: MarketMoatSnapshot {
            market_id: "healthcare-deid".into(),
            moat_score: 45,
            ..MarketMoatSnapshot::default()
        },
        competitor: CompetitorProfile {
            competitor_id: "comp-1".into(),
            threat_score: 30,
            ..CompetitorProfile::default()
        },
        lock_in: LockInReport {
            lockin_score: 60,
            workflow_dependency_strength: 72,
            ..LockInReport::default()
        },
        strategies: vec![MoatStrategy {
            strategy_id: "workflow-audit".into(),
            title: "Workflow audit moat".into(),
            target_moat_type: MoatType::WorkflowLockIn,
            implementation_cost: 2,
            expected_moat_gain: 8,
            ..MoatStrategy::default()
        }],
        budget: ResourceBudget {
            max_round_minutes: 30,
            max_parallel_tasks: 3,
            max_strategy_candidates: 2,
            max_spec_generations: 1,
            max_implementation_tasks: 1,
            max_review_loops: 1,
        },
        improvement_threshold: 3,
        tests_passed: true,
    };

    if let Some(value) = overrides.strategy_candidates {
        input.budget.max_strategy_candidates = value;
    }
    if let Some(value) = overrides.spec_generations {
        input.budget.max_spec_generations = value;
    }
    if let Some(value) = overrides.implementation_tasks {
        input.budget.max_implementation_tasks = value;
    }
    if let Some(value) = overrides.review_loops {
        input.budget.max_review_loops = value;
    }
    if let Some(value) = overrides.tests_passed {
        input.tests_passed = value;
    }

    input
}

fn format_command(args: &[String]) -> String {
    if args.is_empty() {
        "<none>".to_string()
    } else {
        args.join(" ")
    }
}

fn usage() -> &'static str {
    "usage: mdid-cli [status | moat round [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false] [--history-path PATH] | moat control-plane [--history-path PATH] [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false] | moat history --history-path PATH | moat decision-log --history-path PATH [--role planner|coder|reviewer] | moat assignments --history-path PATH [--role planner|coder|reviewer] | moat continue --history-path PATH [--improvement-threshold N] | moat schedule-next --history-path PATH [--improvement-threshold N] | moat export-specs --history-path PATH --output-dir DIR | moat export-plans --history-path PATH --output-dir DIR]"
}

fn exit_with_usage(message: String) -> ! {
    eprintln!("{message}");
    eprintln!("{}", usage());
    std::process::exit(1);
}

fn exit_with_error(message: String) -> ! {
    eprintln!("{message}");
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use mdid_domain::ContinueDecision;

    #[test]
    fn continue_decision_formatter_uses_stable_contract_strings() {
        assert_eq!(
            format_continue_decision(ContinueDecision::Continue),
            "Continue"
        );
        assert_eq!(format_continue_decision(ContinueDecision::Stop), "Stop");
        assert_eq!(format_continue_decision(ContinueDecision::Pivot), "Pivot");
    }

    #[test]
    fn task_node_state_formatter_uses_stable_contract_strings() {
        assert_eq!(
            format_task_node_state(MoatTaskNodeState::Pending),
            "pending"
        );
        assert_eq!(format_task_node_state(MoatTaskNodeState::Ready), "ready");
        assert_eq!(
            format_task_node_state(MoatTaskNodeState::InProgress),
            "in_progress"
        );
        assert_eq!(
            format_task_node_state(MoatTaskNodeState::Completed),
            "completed"
        );
        assert_eq!(
            format_task_node_state(MoatTaskNodeState::Blocked),
            "blocked"
        );
    }

    #[test]
    fn format_string_list_uses_none_for_empty_and_commas_for_values() {
        assert_eq!(format_string_list(&[]), "<none>");
        assert_eq!(
            format_string_list(&[
                "moat-spec/workflow-audit".to_string(),
                "moat-spec/compliance-ledger".to_string(),
            ]),
            "moat-spec/workflow-audit,moat-spec/compliance-ledger"
        );
    }

    #[test]
    fn parse_command_maps_round_control_plane_history_and_continue_commands() {
        assert_eq!(
            parse_command(&[
                "moat".into(),
                "round".into(),
                "--review-loops".into(),
                "0".into(),
            ])
            .unwrap(),
            CliCommand::MoatRound(MoatRoundCommand {
                overrides: MoatRoundOverrides {
                    review_loops: Some(0),
                    ..MoatRoundOverrides::default()
                },
                history_path: None,
            })
        );
        assert_eq!(
            parse_command(&[
                "moat".into(),
                "control-plane".into(),
                "--strategy-candidates".into(),
                "0".into(),
            ])
            .unwrap(),
            CliCommand::MoatControlPlane(MoatControlPlaneCommand {
                overrides: MoatRoundOverrides {
                    strategy_candidates: Some(0),
                    ..MoatRoundOverrides::default()
                },
                history_path: None,
            })
        );
        assert_eq!(
            parse_command(&[
                "moat".into(),
                "history".into(),
                "--history-path".into(),
                "history.json".into(),
            ])
            .unwrap(),
            CliCommand::MoatHistory("history.json".into())
        );
        assert_eq!(
            parse_command(&[
                "moat".into(),
                "continue".into(),
                "--history-path".into(),
                "history.json".into(),
                "--improvement-threshold".into(),
                "4".into(),
            ])
            .unwrap(),
            CliCommand::MoatContinue {
                history_path: "history.json".into(),
                improvement_threshold: 4,
            }
        );
        assert_eq!(
            parse_command(&[
                "moat".into(),
                "schedule-next".into(),
                "--history-path".into(),
                "history.json".into(),
                "--improvement-threshold".into(),
                "5".into(),
            ])
            .unwrap(),
            CliCommand::MoatScheduleNext {
                history_path: "history.json".into(),
                improvement_threshold: 5,
            }
        );
    }
}
