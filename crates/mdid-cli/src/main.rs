use chrono::Utc;
use mdid_application::{render_moat_plan_markdown, render_moat_spec_markdown, MoatAgentAssignment};
use mdid_domain::{
    AgentRole, CompetitorProfile, ContinueDecision, LockInReport, MarketMoatSnapshot, MoatStrategy,
    MoatTaskNodeKind, MoatTaskNodeState, MoatType, ResourceBudget,
};
use mdid_runtime::{
    moat::{run_bounded_round, MoatRoundInput, MoatRoundReport},
    moat_history::{
        CompleteInProgressTaskError, CompleteTaskArtifact, LocalMoatHistoryStore, MoatHistoryEntry,
        MoatHistorySummary,
    },
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
struct MoatHistoryCommand {
    history_path: String,
    round_id: Option<String>,
    decision: Option<ContinueDecision>,
    contains: Option<String>,
    stop_reason_contains: Option<String>,
    min_score: Option<u32>,
    tests_passed: Option<bool>,
    limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MoatDecisionLogCommand {
    history_path: String,
    round_id: Option<String>,
    role: Option<AgentRole>,
    contains: Option<String>,
    summary_contains: Option<String>,
    rationale_contains: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MoatAssignmentsCommand {
    history_path: String,
    round_id: Option<String>,
    role: Option<AgentRole>,
    state: Option<MoatTaskNodeState>,
    kind: Option<MoatTaskNodeKind>,
    node_id: Option<String>,
    depends_on: Option<String>,
    no_dependencies: bool,
    title_contains: Option<String>,
    spec_ref: Option<String>,
    contains: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MoatTaskGraphCommand {
    history_path: String,
    round_id: Option<String>,
    role: Option<AgentRole>,
    state: Option<MoatTaskNodeState>,
    kind: Option<MoatTaskNodeKind>,
    node_id: Option<String>,
    depends_on: Option<String>,
    no_dependencies: bool,
    title_contains: Option<String>,
    spec_ref: Option<String>,
    contains: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MoatReadyTasksCommand {
    history_path: String,
    round_id: Option<String>,
    role: Option<AgentRole>,
    kind: Option<MoatTaskNodeKind>,
    node_id: Option<String>,
    depends_on: Option<String>,
    no_dependencies: bool,
    title_contains: Option<String>,
    spec_ref: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MoatDispatchNextCommand {
    history_path: String,
    round_id: Option<String>,
    role: Option<AgentRole>,
    kind: Option<MoatTaskNodeKind>,
    dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MoatArtifactsCommand {
    history_path: String,
    round_id: Option<String>,
    role: Option<AgentRole>,
    state: Option<MoatTaskNodeState>,
    kind: Option<MoatTaskNodeKind>,
    node_id: Option<String>,
    contains: Option<String>,
    artifact_ref: Option<String>,
    artifact_summary: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MoatClaimTaskCommand {
    history_path: String,
    round_id: Option<String>,
    node_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MoatCompleteTaskCommand {
    history_path: String,
    round_id: Option<String>,
    node_id: String,
    artifact_ref: Option<String>,
    artifact_summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MoatReleaseTaskCommand {
    history_path: String,
    round_id: Option<String>,
    node_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MoatBlockTaskCommand {
    history_path: String,
    round_id: Option<String>,
    node_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MoatUnblockTaskCommand {
    history_path: String,
    round_id: Option<String>,
    node_id: String,
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
        Ok(CliCommand::MoatHistory(command)) => {
            if let Err(error) = run_moat_history(&command) {
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
        Ok(CliCommand::MoatTaskGraph(command)) => {
            if let Err(error) = run_moat_task_graph(&command) {
                exit_with_error(error);
            }
        }
        Ok(CliCommand::MoatReadyTasks(command)) => {
            if let Err(error) = run_moat_ready_tasks(&command) {
                exit_with_error(error);
            }
        }
        Ok(CliCommand::MoatDispatchNext(command)) => {
            if let Err(error) = run_moat_dispatch_next(&command) {
                exit_with_error(error);
            }
        }
        Ok(CliCommand::MoatArtifacts(command)) => {
            if let Err(error) = run_moat_artifacts(&command) {
                exit_with_error(error);
            }
        }
        Ok(CliCommand::MoatClaimTask(command)) => {
            if let Err(error) = run_moat_claim_task(&command) {
                exit_with_error(error);
            }
        }
        Ok(CliCommand::MoatCompleteTask(command)) => {
            if let Err(error) = run_moat_complete_task(&command) {
                exit_with_error(error);
            }
        }
        Ok(CliCommand::MoatReleaseTask(command)) => {
            if let Err(error) = run_moat_release_task(&command) {
                exit_with_error(error);
            }
        }
        Ok(CliCommand::MoatBlockTask(command)) => {
            if let Err(error) = run_moat_block_task(&command) {
                exit_with_error(error);
            }
        }
        Ok(CliCommand::MoatUnblockTask(command)) => {
            if let Err(error) = run_moat_unblock_task(&command) {
                exit_with_error(error);
            }
        }
        Ok(CliCommand::MoatExportSpecs {
            history_path,
            output_dir,
            round_id,
        }) => {
            if let Err(error) =
                run_moat_export_specs(&history_path, &output_dir, round_id.as_deref())
            {
                exit_with_error(error);
            }
        }
        Ok(CliCommand::MoatExportPlans {
            history_path,
            output_dir,
            round_id,
        }) => {
            if let Err(error) =
                run_moat_export_plans(&history_path, &output_dir, round_id.as_deref())
            {
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
    MoatHistory(MoatHistoryCommand),
    MoatDecisionLog(MoatDecisionLogCommand),
    MoatAssignments(MoatAssignmentsCommand),
    MoatTaskGraph(MoatTaskGraphCommand),
    MoatReadyTasks(MoatReadyTasksCommand),
    MoatDispatchNext(MoatDispatchNextCommand),
    MoatArtifacts(MoatArtifactsCommand),
    MoatClaimTask(MoatClaimTaskCommand),
    MoatCompleteTask(MoatCompleteTaskCommand),
    MoatReleaseTask(MoatReleaseTaskCommand),
    MoatBlockTask(MoatBlockTaskCommand),
    MoatUnblockTask(MoatUnblockTaskCommand),
    MoatExportSpecs {
        history_path: String,
        output_dir: String,
        round_id: Option<String>,
    },
    MoatExportPlans {
        history_path: String,
        output_dir: String,
        round_id: Option<String>,
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
            Ok(CliCommand::MoatHistory(parse_moat_history_command(rest)?))
        }
        [moat, decision_log, rest @ ..] if moat == "moat" && decision_log == "decision-log" => Ok(
            CliCommand::MoatDecisionLog(parse_moat_decision_log_command(rest)?),
        ),
        [moat, assignments, rest @ ..] if moat == "moat" && assignments == "assignments" => Ok(
            CliCommand::MoatAssignments(parse_moat_assignments_command(rest)?),
        ),
        [moat, task_graph, rest @ ..] if moat == "moat" && task_graph == "task-graph" => Ok(
            CliCommand::MoatTaskGraph(parse_moat_task_graph_command(rest)?),
        ),
        [moat, ready_tasks, rest @ ..] if moat == "moat" && ready_tasks == "ready-tasks" => Ok(
            CliCommand::MoatReadyTasks(parse_moat_ready_tasks_command(rest)?),
        ),
        [moat, dispatch_next, rest @ ..] if moat == "moat" && dispatch_next == "dispatch-next" => {
            Ok(CliCommand::MoatDispatchNext(
                parse_moat_dispatch_next_command(rest)?,
            ))
        }
        [moat, artifacts, rest @ ..] if moat == "moat" && artifacts == "artifacts" => Ok(
            CliCommand::MoatArtifacts(parse_moat_artifacts_command(rest)?),
        ),
        [moat, claim_task, rest @ ..] if moat == "moat" && claim_task == "claim-task" => Ok(
            CliCommand::MoatClaimTask(parse_moat_claim_task_command(rest)?),
        ),
        [moat, complete_task, rest @ ..] if moat == "moat" && complete_task == "complete-task" => {
            Ok(CliCommand::MoatCompleteTask(
                parse_moat_complete_task_command(rest)?,
            ))
        }
        [moat, release_task, rest @ ..] if moat == "moat" && release_task == "release-task" => Ok(
            CliCommand::MoatReleaseTask(parse_moat_release_task_command(rest)?),
        ),
        [moat, block_task, rest @ ..] if moat == "moat" && block_task == "block-task" => Ok(
            CliCommand::MoatBlockTask(parse_moat_block_task_command(rest)?),
        ),
        [moat, unblock_task, rest @ ..] if moat == "moat" && unblock_task == "unblock-task" => Ok(
            CliCommand::MoatUnblockTask(parse_moat_unblock_task_command(rest)?),
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

fn parse_moat_history_command(args: &[String]) -> Result<MoatHistoryCommand, String> {
    let mut history_path = None;
    let mut round_id = None;
    let mut decision = None;
    let mut contains = None;
    let mut stop_reason_contains = None;
    let mut min_score = None;
    let mut tests_passed = None;
    let mut limit = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--history-path" => {
                let value = required_history_path_value(args, index)?.clone();
                if history_path.is_some() {
                    return Err(duplicate_flag_error("--history-path"));
                }
                history_path = Some(value);
                index += 2;
            }
            "--decision" => {
                let value = required_flag_value(args, index, "--decision", false)?;
                if decision.is_some() {
                    return Err(duplicate_flag_error("--decision"));
                }
                decision = Some(parse_continue_decision_filter(value)?);
                index += 2;
            }
            "--round-id" => {
                let value = required_flag_value(args, index, "--round-id", false)?;
                if round_id.is_some() {
                    return Err(duplicate_flag_error("--round-id"));
                }
                round_id = Some(value.clone());
                index += 2;
            }
            "--contains" => {
                let value = args
                    .get(index + 1)
                    .filter(|value| !value.starts_with("--"))
                    .ok_or_else(|| "--contains requires a value".to_string())?;
                if contains.is_some() {
                    return Err(duplicate_flag_error("--contains"));
                }
                contains = Some(value.clone());
                index += 2;
            }
            "--stop-reason-contains" => {
                let value = args
                    .get(index + 1)
                    .filter(|value| !value.starts_with("--"))
                    .ok_or_else(|| "--stop-reason-contains requires a value".to_string())?;
                if stop_reason_contains.is_some() {
                    return Err(duplicate_flag_error("--stop-reason-contains"));
                }
                stop_reason_contains = Some(value.clone());
                index += 2;
            }
            "--limit" => {
                let value = required_flag_value(args, index, "--limit", false)?;
                if limit.is_some() {
                    return Err(duplicate_flag_error("--limit"));
                }
                limit = Some(parse_limit_value(value)?);
                index += 2;
            }
            "--min-score" => {
                let value = required_flag_value(args, index, "--min-score", true)?;
                if min_score.is_some() {
                    return Err(duplicate_flag_error("--min-score"));
                }
                min_score = Some(parse_min_score_value(value)?);
                index += 2;
            }
            "--tests-passed" => {
                let value = required_flag_value(args, index, "--tests-passed", false)?;
                if tests_passed.is_some() {
                    return Err(duplicate_flag_error("--tests-passed"));
                }
                tests_passed = Some(parse_bool_flag("--tests-passed", value)?);
                index += 2;
            }
            flag => return Err(format!("unknown moat history flag: {flag}")),
        }
    }

    Ok(MoatHistoryCommand {
        history_path: history_path
            .ok_or_else(|| "missing required flag: --history-path".to_string())?,
        round_id,
        decision,
        contains,
        stop_reason_contains,
        min_score,
        tests_passed,
        limit,
    })
}

fn parse_min_score_value(value: &str) -> Result<u32, String> {
    value.parse::<u32>().map_err(|_| {
        format!("invalid value for --min-score: expected non-negative integer, got {value}")
    })
}

fn parse_continue_decision_filter(value: &str) -> Result<ContinueDecision, String> {
    match value {
        "Continue" => Ok(ContinueDecision::Continue),
        "Stop" => Ok(ContinueDecision::Stop),
        "Pivot" => Ok(ContinueDecision::Pivot),
        other => Err(format!("unknown moat history decision: {other}")),
    }
}

fn parse_moat_decision_log_command(args: &[String]) -> Result<MoatDecisionLogCommand, String> {
    let mut history_path = None;
    let mut round_id = None;
    let mut role = None;
    let mut contains = None;
    let mut summary_contains = None;
    let mut rationale_contains = None;
    let mut limit = None;
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
            "--round-id" => {
                let value = required_flag_value(args, index, "--round-id", true)?;
                if round_id.is_some() {
                    return Err(duplicate_flag_error("--round-id"));
                }
                round_id = Some(value.to_string());
            }
            "--role" => {
                let value = required_flag_value(args, index, "--role", false)?;
                if role.is_some() {
                    return Err(duplicate_flag_error("--role"));
                }
                role = Some(parse_agent_role_filter(value)?);
            }
            "--contains" => {
                let value = required_flag_value(args, index, "--contains", true)?;
                if contains.is_some() {
                    return Err(duplicate_flag_error("--contains"));
                }
                contains = Some(value.clone());
            }
            "--summary-contains" => {
                let value = required_flag_value(args, index, "--summary-contains", true)?;
                if summary_contains.is_some() {
                    return Err(duplicate_flag_error("--summary-contains"));
                }
                summary_contains = Some(value.clone());
            }
            "--rationale-contains" => {
                let value = required_flag_value(args, index, "--rationale-contains", true)?;
                if rationale_contains.is_some() {
                    return Err(duplicate_flag_error("--rationale-contains"));
                }
                rationale_contains = Some(value.clone());
            }
            "--limit" => {
                let value = required_flag_value(args, index, "--limit", false)?;
                if limit.is_some() {
                    return Err(duplicate_flag_error("--limit"));
                }
                limit = Some(parse_positive_usize_flag("--limit", value)?);
            }
            flag => return Err(format!("unknown flag: {flag}")),
        }

        index += 2;
    }

    Ok(MoatDecisionLogCommand {
        history_path: history_path
            .ok_or_else(|| "missing required flag: --history-path".to_string())?,
        round_id,
        role,
        contains,
        summary_contains,
        rationale_contains,
        limit,
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
    let mut round_id = None;
    let mut role = None;
    let mut state = None;
    let mut kind = None;
    let mut node_id = None;
    let mut depends_on = None;
    let mut no_dependencies = false;
    let mut title_contains = None;
    let mut spec_ref = None;
    let mut contains = None;
    let mut limit = None;
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
            "--round-id" => {
                let value = required_flag_value(args, index, "--round-id", true)?;
                if round_id.is_some() {
                    return Err(duplicate_flag_error("--round-id"));
                }
                round_id = Some(value.to_string());
            }
            "--role" => {
                let value = required_flag_value(args, index, "--role", false)?;
                if role.is_some() {
                    return Err(duplicate_flag_error("--role"));
                }
                role = Some(parse_moat_assignments_role_filter(value)?);
            }
            "--state" => {
                let value = required_flag_value(args, index, "--state", false)?;
                if state.is_some() {
                    return Err(duplicate_flag_error("--state"));
                }
                state = Some(parse_moat_assignments_state_filter(value)?);
            }
            "--kind" => {
                let value = required_flag_value(args, index, "--kind", true)?;
                if kind.is_some() {
                    return Err(duplicate_flag_error("--kind"));
                }
                kind = Some(parse_moat_assignments_kind_filter(value)?);
            }
            "--node-id" => {
                let value = required_flag_value(args, index, "--node-id", false)?;
                if node_id.is_some() {
                    return Err(duplicate_flag_error("--node-id"));
                }
                node_id = Some(value.clone());
            }
            "--depends-on" => {
                let value = required_flag_value(args, index, "--depends-on", false)
                    .map_err(|_| "--depends-on requires a value".to_string())?;
                if depends_on.is_some() {
                    return Err(duplicate_flag_error("--depends-on"));
                }
                depends_on = Some(value.clone());
            }
            "--no-dependencies" => {
                if no_dependencies {
                    return Err(duplicate_flag_error("--no-dependencies"));
                }
                no_dependencies = true;
                index += 1;
                continue;
            }
            "--title-contains" => {
                let value = required_flag_value(args, index, "--title-contains", true)?;
                if title_contains.is_some() {
                    return Err(duplicate_flag_error("--title-contains"));
                }
                title_contains = Some(value.clone());
            }
            "--spec-ref" => {
                let value = required_flag_value(args, index, "--spec-ref", true)?;
                if spec_ref.is_some() {
                    return Err(duplicate_flag_error("--spec-ref"));
                }
                spec_ref = Some(value.clone());
            }
            "--contains" => {
                let value = required_flag_value(args, index, "--contains", true)?;
                if contains.is_some() {
                    return Err(duplicate_flag_error("--contains"));
                }
                contains = Some(value.clone());
            }
            "--limit" => {
                let value = required_flag_value(args, index, "--limit", true)?;
                if limit.is_some() {
                    return Err(duplicate_flag_error("--limit"));
                }
                limit = Some(parse_positive_usize_flag("--limit", value)?);
            }
            flag => return Err(format!("unknown flag: {flag}")),
        }

        index += 2;
    }

    Ok(MoatAssignmentsCommand {
        history_path: history_path
            .ok_or_else(|| "missing required flag: --history-path".to_string())?,
        round_id,
        role,
        state,
        kind,
        node_id,
        depends_on,
        no_dependencies,
        title_contains,
        spec_ref,
        contains,
        limit,
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

fn parse_moat_assignments_kind_filter(value: &str) -> Result<MoatTaskNodeKind, String> {
    parse_moat_task_graph_kind_filter(value)
        .map_err(|_| format!("unknown moat assignments kind: {value}"))
}

fn parse_moat_assignments_state_filter(value: &str) -> Result<MoatTaskNodeState, String> {
    match value {
        "pending" => Ok(MoatTaskNodeState::Pending),
        "ready" => Ok(MoatTaskNodeState::Ready),
        "in_progress" => Ok(MoatTaskNodeState::InProgress),
        "completed" => Ok(MoatTaskNodeState::Completed),
        "blocked" => Ok(MoatTaskNodeState::Blocked),
        other => Err(format!("unknown moat assignments state: {other}")),
    }
}

fn parse_moat_task_graph_command(args: &[String]) -> Result<MoatTaskGraphCommand, String> {
    let mut history_path = None;
    let mut round_id = None;
    let mut role = None;
    let mut state = None;
    let mut kind = None;
    let mut node_id = None;
    let mut depends_on = None;
    let mut no_dependencies = false;
    let mut title_contains = None;
    let mut spec_ref = None;
    let mut contains = None;
    let mut limit = None;
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
            "--round-id" => {
                let value = required_flag_value(args, index, "--round-id", true)?;
                if round_id.is_some() {
                    return Err(duplicate_flag_error("--round-id"));
                }
                round_id = Some(value.to_string());
            }
            "--role" => {
                let value = required_flag_value(args, index, "--role", false)?;
                if role.is_some() {
                    return Err(duplicate_flag_error("--role"));
                }
                role = Some(parse_moat_task_graph_role_filter(value)?);
            }
            "--state" => {
                let value = required_flag_value(args, index, "--state", false)?;
                if state.is_some() {
                    return Err(duplicate_flag_error("--state"));
                }
                state = Some(parse_moat_task_graph_state_filter(value)?);
            }
            "--kind" => {
                let value = required_flag_value(args, index, "--kind", true)?;
                if kind.is_some() {
                    return Err(duplicate_flag_error("--kind"));
                }
                kind = Some(parse_moat_task_graph_kind_filter(value)?);
            }
            "--node-id" => {
                let value = required_flag_value(args, index, "--node-id", false)?;
                if node_id.is_some() {
                    return Err(duplicate_flag_error("--node-id"));
                }
                node_id = Some(value.clone());
            }
            "--depends-on" => {
                let value = required_flag_value(args, index, "--depends-on", false)?;
                if depends_on.is_some() {
                    return Err(duplicate_flag_error("--depends-on"));
                }
                depends_on = Some(value.clone());
            }
            "--no-dependencies" => {
                if no_dependencies {
                    return Err(duplicate_flag_error("--no-dependencies"));
                }
                no_dependencies = true;
                index += 1;
                continue;
            }
            "--title-contains" => {
                let value = required_flag_value(args, index, "--title-contains", true)?;
                if title_contains.is_some() {
                    return Err(duplicate_flag_error("--title-contains"));
                }
                title_contains = Some(value.to_string());
            }
            "--spec-ref" => {
                let value = required_flag_value(args, index, "--spec-ref", true)?;
                if spec_ref.is_some() {
                    return Err(duplicate_flag_error("--spec-ref"));
                }
                spec_ref = Some(value.to_string());
            }
            "--contains" => {
                let value = required_flag_value(args, index, "--contains", true)?;
                if contains.is_some() {
                    return Err(duplicate_flag_error("--contains"));
                }
                contains = Some(value.to_string());
            }
            "--limit" => {
                let value = required_flag_value(args, index, "--limit", true)?;
                if limit.is_some() {
                    return Err(duplicate_flag_error("--limit"));
                }
                limit = Some(parse_task_graph_limit_value(value)?);
            }
            flag => return Err(format!("unknown flag: {flag}")),
        }

        index += 2;
    }

    Ok(MoatTaskGraphCommand {
        history_path: history_path
            .ok_or_else(|| "missing required flag: --history-path".to_string())?,
        round_id,
        role,
        state,
        kind,
        node_id,
        depends_on,
        no_dependencies,
        title_contains,
        spec_ref,
        contains,
        limit,
    })
}

fn parse_moat_ready_tasks_command(args: &[String]) -> Result<MoatReadyTasksCommand, String> {
    let mut history_path = None;
    let mut round_id = None;
    let mut role = None;
    let mut kind = None;
    let mut node_id = None;
    let mut depends_on = None;
    let mut no_dependencies = false;
    let mut title_contains = None;
    let mut spec_ref = None;
    let mut limit = None;
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
            "--round-id" => {
                let value = required_flag_value(args, index, "--round-id", true)?;
                if round_id.is_some() {
                    return Err(duplicate_flag_error("--round-id"));
                }
                round_id = Some(value.to_string());
            }
            "--role" => {
                let value = required_flag_value(args, index, "--role", false)?;
                if role.is_some() {
                    return Err(duplicate_flag_error("--role"));
                }
                role = Some(parse_moat_task_graph_role_filter(value)?);
            }
            "--kind" => {
                let value = required_flag_value(args, index, "--kind", true)?;
                if kind.is_some() {
                    return Err(duplicate_flag_error("--kind"));
                }
                kind = Some(parse_moat_task_graph_kind_filter(value)?);
            }
            "--node-id" => {
                let value = required_flag_value(args, index, "--node-id", true)?;
                if node_id.is_some() {
                    return Err(duplicate_flag_error("--node-id"));
                }
                node_id = Some(value.to_string());
            }
            "--depends-on" => {
                let value = required_flag_value(args, index, "--depends-on", false)?;
                if depends_on.is_some() {
                    return Err(duplicate_flag_error("--depends-on"));
                }
                depends_on = Some(value.to_string());
            }
            "--no-dependencies" => {
                if no_dependencies {
                    return Err(duplicate_flag_error("--no-dependencies"));
                }
                no_dependencies = true;
                index += 1;
                continue;
            }
            "--title-contains" => {
                let value = required_flag_value(args, index, "--title-contains", true)?;
                if title_contains.is_some() {
                    return Err(duplicate_flag_error("--title-contains"));
                }
                title_contains = Some(value.to_string());
            }
            "--spec-ref" => {
                let value = required_flag_value(args, index, "--spec-ref", true)?;
                if spec_ref.is_some() {
                    return Err(duplicate_flag_error("--spec-ref"));
                }
                spec_ref = Some(value.to_string());
            }
            "--limit" => {
                let value = required_flag_value(args, index, "--limit", true)?;
                if limit.is_some() {
                    return Err(duplicate_flag_error("--limit"));
                }
                limit = Some(parse_task_graph_limit_value(value)?);
            }
            flag => return Err(format!("unknown flag: {flag}")),
        }

        index += 2;
    }

    Ok(MoatReadyTasksCommand {
        history_path: history_path
            .ok_or_else(|| "missing required flag: --history-path".to_string())?,
        round_id,
        role,
        kind,
        node_id,
        depends_on,
        no_dependencies,
        title_contains,
        spec_ref,
        limit,
    })
}

fn parse_moat_artifacts_command(args: &[String]) -> Result<MoatArtifactsCommand, String> {
    let mut history_path = None;
    let mut round_id = None;
    let mut role = None;
    let mut state = None;
    let mut kind = None;
    let mut node_id = None;
    let mut contains = None;
    let mut artifact_ref = None;
    let mut artifact_summary = None;
    let mut limit = None;
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
            "--round-id" => {
                let value = required_flag_value(args, index, "--round-id", true)?;
                if round_id.is_some() {
                    return Err(duplicate_flag_error("--round-id"));
                }
                round_id = Some(value.to_string());
            }
            "--role" => {
                let value = required_flag_value(args, index, "--role", false)?;
                if role.is_some() {
                    return Err(duplicate_flag_error("--role"));
                }
                role = Some(parse_moat_artifacts_role_filter(value)?);
            }
            "--state" => {
                let value = required_flag_value(args, index, "--state", false)?;
                if state.is_some() {
                    return Err(duplicate_flag_error("--state"));
                }
                state = Some(parse_moat_artifacts_state_filter(value)?);
            }
            "--kind" => {
                let value = required_flag_value(args, index, "--kind", false)?;
                if kind.is_some() {
                    return Err(duplicate_flag_error("--kind"));
                }
                kind = Some(parse_moat_artifacts_kind_filter(value)?);
            }
            "--node-id" => {
                let value = required_flag_value(args, index, "--node-id", false)?;
                if node_id.is_some() {
                    return Err(duplicate_flag_error("--node-id"));
                }
                node_id = Some(value.to_string());
            }
            "--contains" => {
                let value = required_flag_value(args, index, "--contains", false)?;
                if contains.is_some() {
                    return Err(duplicate_flag_error("--contains"));
                }
                contains = Some(value.to_string());
            }
            "--artifact-ref" => {
                let value = required_flag_value(args, index, "--artifact-ref", false)?;
                if artifact_ref.is_some() {
                    return Err(duplicate_flag_error("--artifact-ref"));
                }
                artifact_ref = Some(value.to_string());
            }
            "--artifact-summary" => {
                let value = required_flag_value(args, index, "--artifact-summary", false)?;
                if artifact_summary.is_some() {
                    return Err(duplicate_flag_error("--artifact-summary"));
                }
                artifact_summary = Some(value.to_string());
            }
            "--limit" => {
                let value = required_flag_value(args, index, "--limit", false)?;
                if limit.is_some() {
                    return Err(duplicate_flag_error("--limit"));
                }
                limit = Some(parse_task_graph_limit_value(value)?);
            }
            flag => return Err(format!("unknown flag: {flag}")),
        }

        index += 2;
    }

    Ok(MoatArtifactsCommand {
        history_path: history_path
            .ok_or_else(|| "missing required flag: --history-path".to_string())?,
        round_id,
        role,
        state,
        kind,
        node_id,
        contains,
        artifact_ref,
        artifact_summary,
        limit,
    })
}

fn parse_moat_artifacts_role_filter(value: &str) -> Result<AgentRole, String> {
    match value {
        "planner" => Ok(AgentRole::Planner),
        "coder" => Ok(AgentRole::Coder),
        "reviewer" => Ok(AgentRole::Reviewer),
        other => Err(format!("unknown moat artifacts role: {other}")),
    }
}

fn parse_moat_artifacts_kind_filter(value: &str) -> Result<MoatTaskNodeKind, String> {
    parse_moat_task_graph_kind_filter(value)
        .map_err(|_| format!("unknown moat artifacts kind: {value}"))
}

fn parse_moat_artifacts_state_filter(value: &str) -> Result<MoatTaskNodeState, String> {
    match value {
        "pending" => Ok(MoatTaskNodeState::Pending),
        "ready" => Ok(MoatTaskNodeState::Ready),
        "in_progress" => Ok(MoatTaskNodeState::InProgress),
        "completed" => Ok(MoatTaskNodeState::Completed),
        "blocked" => Ok(MoatTaskNodeState::Blocked),
        other => Err(format!("unknown moat artifacts state: {other}")),
    }
}

fn parse_moat_dispatch_next_command(args: &[String]) -> Result<MoatDispatchNextCommand, String> {
    let mut history_path = None;
    let mut round_id = None;
    let mut role = None;
    let mut kind = None;
    let mut dry_run = false;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--history-path" => {
                let value = required_history_path_value(args, index)?.clone();
                if history_path.is_some() {
                    return Err(duplicate_flag_error("--history-path"));
                }
                history_path = Some(value);
                index += 2;
            }
            "--round-id" => {
                let value = required_flag_value(args, index, "--round-id", true)?;
                if round_id.is_some() {
                    return Err(duplicate_flag_error("--round-id"));
                }
                round_id = Some(value.to_string());
                index += 2;
            }
            "--role" => {
                let value = required_flag_value(args, index, "--role", true)?;
                if role.is_some() {
                    return Err(duplicate_flag_error("--role"));
                }
                role = Some(parse_moat_dispatch_next_role_filter(value)?);
                index += 2;
            }
            "--kind" => {
                let value = required_flag_value(args, index, "--kind", true)?;
                if kind.is_some() {
                    return Err(duplicate_flag_error("--kind"));
                }
                kind = Some(parse_moat_dispatch_next_kind_filter(value)?);
                index += 2;
            }
            "--dry-run" => {
                if dry_run {
                    return Err(duplicate_flag_error("--dry-run"));
                }
                dry_run = true;
                index += 1;
            }
            flag => return Err(format!("unknown flag: {flag}")),
        }
    }

    Ok(MoatDispatchNextCommand {
        history_path: history_path
            .ok_or_else(|| "missing required flag: --history-path".to_string())?,
        round_id,
        role,
        kind,
        dry_run,
    })
}

fn parse_moat_dispatch_next_role_filter(value: &str) -> Result<AgentRole, String> {
    parse_moat_task_graph_role_filter(value)
        .map_err(|_| format!("unknown moat dispatch-next role: {value}"))
}

fn parse_moat_dispatch_next_kind_filter(value: &str) -> Result<MoatTaskNodeKind, String> {
    parse_moat_task_graph_kind_filter(value)
        .map_err(|_| format!("unknown moat dispatch-next kind: {value}"))
}

fn parse_moat_claim_task_command(args: &[String]) -> Result<MoatClaimTaskCommand, String> {
    let mut history_path = None;
    let mut round_id = None;
    let mut node_id = None;
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
            "--round-id" => {
                let value = required_flag_value(args, index, "--round-id", true)?;
                if round_id.is_some() {
                    return Err(duplicate_flag_error("--round-id"));
                }
                round_id = Some(value.to_string());
            }
            "--node-id" => {
                let value = required_flag_value(args, index, "--node-id", true)?;
                if node_id.is_some() {
                    return Err(duplicate_flag_error("--node-id"));
                }
                node_id = Some(value.to_string());
            }
            flag => return Err(format!("unknown flag: {flag}")),
        }

        index += 2;
    }

    Ok(MoatClaimTaskCommand {
        history_path: history_path
            .ok_or_else(|| "missing required flag: --history-path".to_string())?,
        round_id,
        node_id: node_id.ok_or_else(|| "missing required flag: --node-id".to_string())?,
    })
}

fn parse_moat_complete_task_command(args: &[String]) -> Result<MoatCompleteTaskCommand, String> {
    let mut history_path = None;
    let mut round_id = None;
    let mut node_id = None;
    let mut artifact_ref = None;
    let mut artifact_summary = None;
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
            "--round-id" => {
                let value = required_flag_value(args, index, "--round-id", true)?;
                if round_id.is_some() {
                    return Err(duplicate_flag_error("--round-id"));
                }
                round_id = Some(value.to_string());
            }
            "--node-id" => {
                let value = required_flag_value(args, index, "--node-id", true)?;
                if node_id.is_some() {
                    return Err(duplicate_flag_error("--node-id"));
                }
                node_id = Some(value.to_string());
            }
            "--artifact-ref" => {
                let value = required_flag_value(args, index, "--artifact-ref", false)?;
                if artifact_ref.is_some() {
                    return Err(duplicate_flag_error("--artifact-ref"));
                }
                artifact_ref = Some(value.to_string());
            }
            "--artifact-summary" => {
                let value = required_flag_value(args, index, "--artifact-summary", false)?;
                if artifact_summary.is_some() {
                    return Err(duplicate_flag_error("--artifact-summary"));
                }
                artifact_summary = Some(value.to_string());
            }
            flag => return Err(format!("unknown flag: {flag}")),
        }

        index += 2;
    }

    if artifact_ref.is_some() != artifact_summary.is_some() {
        return Err("--artifact-ref and --artifact-summary must be supplied together".to_string());
    }

    Ok(MoatCompleteTaskCommand {
        history_path: history_path
            .ok_or_else(|| "missing required flag: --history-path".to_string())?,
        round_id,
        node_id: node_id.ok_or_else(|| "missing required flag: --node-id".to_string())?,
        artifact_ref,
        artifact_summary,
    })
}

fn parse_moat_release_task_command(args: &[String]) -> Result<MoatReleaseTaskCommand, String> {
    let command = parse_moat_claim_task_command(args)?;
    Ok(MoatReleaseTaskCommand {
        history_path: command.history_path,
        round_id: command.round_id,
        node_id: command.node_id,
    })
}

fn parse_moat_block_task_command(args: &[String]) -> Result<MoatBlockTaskCommand, String> {
    let command = parse_moat_claim_task_command(args)?;
    Ok(MoatBlockTaskCommand {
        history_path: command.history_path,
        round_id: command.round_id,
        node_id: command.node_id,
    })
}

fn parse_moat_unblock_task_command(args: &[String]) -> Result<MoatUnblockTaskCommand, String> {
    let command = parse_moat_claim_task_command(args)?;
    Ok(MoatUnblockTaskCommand {
        history_path: command.history_path,
        round_id: command.round_id,
        node_id: command.node_id,
    })
}

fn parse_task_graph_limit_value(value: &str) -> Result<usize, String> {
    let parsed = value.parse::<usize>().map_err(|_| {
        format!("invalid value for --limit: expected positive integer, got {value}")
    })?;

    if parsed == 0 {
        Err(format!(
            "invalid value for --limit: expected positive integer, got {value}"
        ))
    } else {
        Ok(parsed)
    }
}

fn parse_moat_task_graph_role_filter(value: &str) -> Result<AgentRole, String> {
    match value {
        "planner" => Ok(AgentRole::Planner),
        "coder" => Ok(AgentRole::Coder),
        "reviewer" => Ok(AgentRole::Reviewer),
        other => Err(format!("unknown moat task-graph role: {other}")),
    }
}

fn parse_moat_task_graph_state_filter(value: &str) -> Result<MoatTaskNodeState, String> {
    match value {
        "pending" => Ok(MoatTaskNodeState::Pending),
        "ready" => Ok(MoatTaskNodeState::Ready),
        "in_progress" => Ok(MoatTaskNodeState::InProgress),
        "completed" => Ok(MoatTaskNodeState::Completed),
        "blocked" => Ok(MoatTaskNodeState::Blocked),
        other => Err(format!("unknown moat task-graph state: {other}")),
    }
}

fn parse_moat_task_graph_kind_filter(value: &str) -> Result<MoatTaskNodeKind, String> {
    match value {
        "market_scan" => Ok(MoatTaskNodeKind::MarketScan),
        "competitor_analysis" => Ok(MoatTaskNodeKind::CompetitorAnalysis),
        "lock_in_analysis" => Ok(MoatTaskNodeKind::LockInAnalysis),
        "strategy_generation" => Ok(MoatTaskNodeKind::StrategyGeneration),
        "spec_planning" => Ok(MoatTaskNodeKind::SpecPlanning),
        "implementation" => Ok(MoatTaskNodeKind::Implementation),
        "review" => Ok(MoatTaskNodeKind::Review),
        "evaluation" => Ok(MoatTaskNodeKind::Evaluation),
        other => Err(format!("unknown moat task-graph kind: {other}")),
    }
}

fn parse_moat_export_specs_command(args: &[String]) -> Result<CliCommand, String> {
    let mut history_path = None;
    let mut output_dir = None;
    let mut round_id = None;
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
            "--round-id" => {
                let value = required_flag_value(args, index, "--round-id", false)?;
                if round_id.is_some() {
                    return Err(duplicate_flag_error("--round-id"));
                }
                round_id = Some(value.clone());
            }
            flag => return Err(format!("unknown flag: {flag}")),
        }

        index += 2;
    }

    Ok(CliCommand::MoatExportSpecs {
        history_path: history_path
            .ok_or_else(|| "missing required flag: --history-path".to_string())?,
        output_dir: output_dir.ok_or_else(|| "missing required flag: --output-dir".to_string())?,
        round_id,
    })
}

fn parse_moat_export_plans_command(args: &[String]) -> Result<CliCommand, String> {
    let CliCommand::MoatExportSpecs {
        history_path,
        output_dir,
        round_id,
    } = parse_moat_export_specs_command(args)?
    else {
        unreachable!("export specs parser returns export specs command")
    };
    Ok(CliCommand::MoatExportPlans {
        history_path,
        output_dir,
        round_id,
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

    let value = args
        .get(index + 1)
        .ok_or_else(|| missing_value_error(flag, allow_history_path))?;

    if allow_history_path && value.starts_with("--") {
        Err(missing_value_error(flag, allow_history_path))
    } else {
        Ok(value)
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

fn parse_positive_usize_flag(flag: &str, value: &str) -> Result<usize, String> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| format!("invalid value for {flag}: {value}"))?;

    if parsed == 0 {
        Err(format!("{flag} must be greater than 0"))
    } else {
        Ok(parsed)
    }
}

fn parse_limit_value(value: &str) -> Result<usize, String> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| format!("invalid value for --limit: {value}"))?;

    if parsed == 0 {
        Err("limit must be greater than zero".to_string())
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

fn run_moat_history(command: &MoatHistoryCommand) -> Result<(), String> {
    let store = LocalMoatHistoryStore::open_existing(&command.history_path)
        .map_err(|error| format!("failed to open moat history store: {error}"))?;
    let entries = store.entries();
    let mut filtered_entries = entries
        .iter()
        .filter(|entry| {
            command
                .round_id
                .as_ref()
                .map(|round_id| entry.report.summary.round_id.to_string() == *round_id)
                .unwrap_or(true)
        })
        .filter(|entry| {
            command
                .decision
                .map(|decision| entry.report.summary.continue_decision == decision)
                .unwrap_or(true)
        })
        .filter(|entry| {
            command
                .contains
                .as_ref()
                .map(|needle| moat_history_entry_search_text(entry).contains(needle))
                .unwrap_or(true)
        })
        .filter(|entry| {
            command
                .stop_reason_contains
                .as_ref()
                .map(|needle| {
                    entry
                        .report
                        .stop_reason
                        .as_deref()
                        .map(|stop_reason| stop_reason.contains(needle))
                        .unwrap_or(false)
                })
                .unwrap_or(true)
        })
        .filter(|entry| {
            command
                .min_score
                .map(|min_score| {
                    u32::try_from(entry.report.summary.moat_score_after)
                        .map(|score_after| score_after >= min_score)
                        .unwrap_or(false)
                })
                .unwrap_or(true)
        })
        .filter(|entry| {
            command
                .tests_passed
                .map(|tests_passed| entry.report.summary.tests_passed == tests_passed)
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();

    if command.contains.is_some()
        || command.stop_reason_contains.is_some()
        || command.min_score.is_some()
        || command.tests_passed.is_some()
    {
        if filtered_entries.is_empty() {
            print_empty_filtered_history_summary();
        } else {
            print_filtered_history_summary(&filtered_entries);
        }
    } else {
        print_history_summary(&store.summary());
    }

    if command.limit.is_some() || command.round_id.is_some() || command.tests_passed.is_some() {
        if let Some(limit) = command.limit {
            let excess = filtered_entries.len().saturating_sub(limit);
            if excess > 0 {
                filtered_entries.drain(0..excess);
            }
        }
        println!("history_rounds={}", filtered_entries.len());
        for entry in filtered_entries {
            println!(
                "round={}|{}|{}|{}",
                entry.report.summary.round_id,
                format_continue_decision(entry.report.summary.continue_decision),
                entry.report.summary.moat_score_after,
                entry
                    .report
                    .stop_reason
                    .as_deref()
                    .map(escape_assignment_output_field)
                    .unwrap_or_else(|| "<none>".to_string())
            );
        }
    }

    Ok(())
}

fn run_moat_decision_log(command: &MoatDecisionLogCommand) -> Result<(), String> {
    let store = LocalMoatHistoryStore::open_existing(&command.history_path)
        .map_err(|error| format!("failed to open moat history store: {error}"))?;
    let maybe_entry = match command.round_id.as_deref() {
        Some(round_id) => store
            .entries()
            .iter()
            .find(|entry| entry.report.summary.round_id.to_string() == round_id),
        None => Some(store.entries().last().ok_or_else(|| {
            "moat history is empty; run `mdid-cli moat round --history-path <path>` first"
                .to_string()
        })?),
    };
    let Some(latest) = maybe_entry else {
        println!("decision_log_entries=0");
        return Ok(());
    };
    let mut decisions = latest
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
        .filter(|decision| {
            command
                .contains
                .as_ref()
                .map(|needle| {
                    decision.summary.contains(needle) || decision.rationale.contains(needle)
                })
                .unwrap_or(true)
        })
        .filter(|decision| {
            command
                .summary_contains
                .as_ref()
                .map(|needle| decision.summary.contains(needle))
                .unwrap_or(true)
        })
        .filter(|decision| {
            command
                .rationale_contains
                .as_ref()
                .map(|needle| decision.rationale.contains(needle))
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();
    if let Some(limit) = command.limit {
        let excess = decisions.len().saturating_sub(limit);
        if excess > 0 {
            decisions.drain(..excess);
        }
    }

    println!("decision_log_entries={}", decisions.len());
    for decision in decisions {
        println!(
            "decision={}|{}|{}",
            format_agent_role(decision.author_role),
            escape_assignment_output_field(&decision.summary),
            escape_assignment_output_field(&decision.rationale)
        );
    }

    Ok(())
}

fn run_moat_assignments(command: &MoatAssignmentsCommand) -> Result<(), String> {
    let store = LocalMoatHistoryStore::open_existing(&command.history_path)
        .map_err(|error| format!("failed to open moat history store: {error}"))?;
    let maybe_entry = match command.round_id.as_deref() {
        Some(round_id) => store
            .entries()
            .iter()
            .find(|entry| entry.report.summary.round_id.to_string() == round_id),
        None => Some(store.entries().last().ok_or_else(|| {
            "moat history is empty; run `mdid-cli moat round --history-path <path>` first"
                .to_string()
        })?),
    };
    let Some(latest) = maybe_entry else {
        println!("moat assignments");
        println!("assignment_entries=0");
        return Ok(());
    };

    let mut assignments = latest
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
        .filter(|assignment| {
            command
                .state
                .map(|expected_state| {
                    latest
                        .report
                        .control_plane
                        .task_graph
                        .nodes
                        .iter()
                        .find(|node| node.node_id == assignment.node_id)
                        .map(|node| node.state == expected_state)
                        .unwrap_or(false)
                })
                .unwrap_or(true)
        })
        .filter(|assignment| {
            command
                .kind
                .map(|kind| assignment.kind == kind)
                .unwrap_or(true)
        })
        .filter(|assignment| {
            command
                .node_id
                .as_ref()
                .map(|node_id| assignment.node_id == *node_id)
                .unwrap_or(true)
        })
        .filter(|assignment| {
            command
                .depends_on
                .as_deref()
                .map(|expected_dependency| {
                    latest
                        .report
                        .control_plane
                        .task_graph
                        .nodes
                        .iter()
                        .find(|node| node.node_id == assignment.node_id)
                        .map(|node| {
                            node.depends_on
                                .iter()
                                .any(|dependency| dependency == expected_dependency)
                        })
                        .unwrap_or(false)
                })
                .unwrap_or(true)
        })
        .filter(|assignment| {
            if command.no_dependencies {
                latest
                    .report
                    .control_plane
                    .task_graph
                    .nodes
                    .iter()
                    .find(|node| node.node_id == assignment.node_id)
                    .map(|node| node.depends_on.is_empty())
                    .unwrap_or(false)
            } else {
                true
            }
        })
        .filter(|assignment| {
            command
                .title_contains
                .as_deref()
                .map(|expected_title| assignment.title.contains(expected_title))
                .unwrap_or(true)
        })
        .filter(|assignment| {
            command
                .spec_ref
                .as_deref()
                .map(|expected_spec_ref| assignment.spec_ref.as_deref() == Some(expected_spec_ref))
                .unwrap_or(true)
        })
        .filter(|assignment| {
            command
                .contains
                .as_deref()
                .map(|needle| {
                    assignment.node_id.contains(needle)
                        || assignment.title.contains(needle)
                        || assignment
                            .spec_ref
                            .as_deref()
                            .map(|spec_ref| spec_ref.contains(needle))
                            .unwrap_or(false)
                })
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();

    if let Some(limit) = command.limit {
        assignments.truncate(limit);
    }

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

fn run_moat_task_graph(command: &MoatTaskGraphCommand) -> Result<(), String> {
    let store = LocalMoatHistoryStore::open_existing(&command.history_path)
        .map_err(|error| format!("failed to open moat history store: {error}"))?;
    if store.entries().is_empty() {
        return Err(
            "moat history is empty; run `mdid-cli moat round --history-path <path>` first"
                .to_string(),
        );
    }
    let selected = if let Some(round_id) = command.round_id.as_deref() {
        store
            .entries()
            .iter()
            .find(|entry| entry.report.summary.round_id.to_string() == round_id)
    } else {
        Some(store.entries().last().ok_or_else(|| {
            "moat history is empty; run `mdid-cli moat round --history-path <path>` first"
                .to_string()
        })?)
    };
    let Some(latest) = selected else {
        println!("moat task graph");
        return Ok(());
    };

    println!("moat task graph");
    let limit = command.limit.unwrap_or(usize::MAX);
    for node in latest
        .report
        .control_plane
        .task_graph
        .nodes
        .iter()
        .filter(|node| command.role.map(|role| node.role == role).unwrap_or(true))
        .filter(|node| {
            command
                .state
                .map(|state| node.state == state)
                .unwrap_or(true)
        })
        .filter(|node| command.kind.map(|kind| node.kind == kind).unwrap_or(true))
        .filter(|node| {
            command
                .node_id
                .as_ref()
                .map(|node_id| node.node_id == *node_id)
                .unwrap_or(true)
        })
        .filter(|node| {
            command
                .depends_on
                .as_ref()
                .map(|dependency| {
                    node.depends_on
                        .iter()
                        .any(|candidate| candidate == dependency)
                })
                .unwrap_or(true)
        })
        .filter(|node| {
            if command.no_dependencies {
                node.depends_on.is_empty()
            } else {
                true
            }
        })
        .filter(|node| {
            command
                .title_contains
                .as_deref()
                .map(|expected_title| node.title.contains(expected_title))
                .unwrap_or(true)
        })
        .filter(|node| {
            command
                .spec_ref
                .as_deref()
                .map(|expected_spec_ref| node.spec_ref.as_deref() == Some(expected_spec_ref))
                .unwrap_or(true)
        })
        .filter(|node| {
            command
                .contains
                .as_deref()
                .map(|needle| task_graph_node_contains(node, needle))
                .unwrap_or(true)
        })
        .take(limit)
    {
        println!(
            "node={}|{}|{}|{}|{}|{}|{}",
            format_agent_role(node.role),
            escape_assignment_output_field(&node.node_id),
            escape_assignment_output_field(&node.title),
            format_moat_task_kind(node.kind),
            format_task_node_state(node.state),
            format_task_graph_dependencies(&node.depends_on),
            node.spec_ref
                .as_deref()
                .map(escape_assignment_output_field)
                .unwrap_or_else(|| "<none>".to_string())
        );
    }

    Ok(())
}

fn run_moat_artifacts(command: &MoatArtifactsCommand) -> Result<(), String> {
    let store = LocalMoatHistoryStore::open_existing(&command.history_path)
        .map_err(|error| format!("failed to open moat history store: {error}"))?;

    let selected = if let Some(round_id) = command.round_id.as_deref() {
        store
            .entries()
            .iter()
            .find(|entry| entry.report.summary.round_id.to_string() == round_id)
    } else {
        store.entries().last()
    };

    println!("moat artifacts");
    let Some(entry) = selected else {
        println!("artifact_entries=0");
        return Ok(());
    };

    let round_id = entry.report.summary.round_id.to_string();
    println!("round_id={round_id}");

    let mut artifacts = entry
        .report
        .control_plane
        .task_graph
        .nodes
        .iter()
        .filter(|node| command.role.map(|role| node.role == role).unwrap_or(true))
        .filter(|node| {
            command
                .state
                .map(|state| node.state == state)
                .unwrap_or(true)
        })
        .filter(|node| command.kind.map(|kind| node.kind == kind).unwrap_or(true))
        .flat_map(|node| {
            node.artifacts
                .iter()
                .map(move |artifact| (node.node_id.as_str(), artifact))
        })
        .filter(|(node_id, _artifact)| {
            command
                .node_id
                .as_deref()
                .map(|expected| *node_id == expected)
                .unwrap_or(true)
        })
        .filter(|(node_id, artifact)| {
            command
                .contains
                .as_deref()
                .map(|needle| {
                    node_id.contains(needle)
                        || artifact.artifact_ref.contains(needle)
                        || artifact.summary.contains(needle)
                })
                .unwrap_or(true)
        })
        .filter(|(_node_id, artifact)| {
            command
                .artifact_ref
                .as_deref()
                .map(|needle| artifact.artifact_ref.contains(needle))
                .unwrap_or(true)
        })
        .filter(|(_node_id, artifact)| {
            command
                .artifact_summary
                .as_deref()
                .map(|needle| artifact.summary.contains(needle))
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();

    if let Some(limit) = command.limit {
        artifacts.truncate(limit);
    }

    println!("artifact_entries={}", artifacts.len());
    for (node_id, artifact) in artifacts {
        println!(
            "artifact={}|{}|{}|{}",
            escape_assignment_output_field(&round_id),
            escape_assignment_output_field(node_id),
            escape_assignment_output_field(&artifact.artifact_ref),
            escape_assignment_output_field(&artifact.summary)
        );
    }

    Ok(())
}

fn run_moat_dispatch_next(command: &MoatDispatchNextCommand) -> Result<(), String> {
    let store = LocalMoatHistoryStore::open_existing(&command.history_path)
        .map_err(|error| format!("failed to open moat history store: {error}"))?;
    if store.entries().is_empty() {
        return Err(
            "moat history is empty; run `mdid-cli moat round --history-path <path>` first"
                .to_string(),
        );
    }

    let entry = if let Some(round_id) = command.round_id.as_deref() {
        store
            .entries()
            .iter()
            .find(|entry| entry.report.summary.round_id.to_string() == round_id)
            .ok_or_else(|| "no ready moat task matched dispatch filters".to_string())?
    } else {
        store.entries().last().ok_or_else(|| {
            "moat history is empty; run `mdid-cli moat round --history-path <path>` first"
                .to_string()
        })?
    };
    let round_id = entry.report.summary.round_id.to_string();
    let ready_ids = entry.report.control_plane.task_graph.ready_node_ids();
    let selected = entry
        .report
        .control_plane
        .task_graph
        .nodes
        .iter()
        .find(|node| {
            ready_ids.iter().any(|ready_id| ready_id == &node.node_id)
                && command.role.map(|role| node.role == role).unwrap_or(true)
                && command.kind.map(|kind| node.kind == kind).unwrap_or(true)
        })
        .cloned()
        .ok_or_else(|| "no ready moat task matched dispatch filters".to_string())?;
    drop(store);

    if !command.dry_run {
        let mut claim_store = LocalMoatHistoryStore::open_existing(&command.history_path)
            .map_err(|error| format!("failed to open moat history store: {error}"))?;
        claim_store
            .claim_ready_task(Some(&round_id), &selected.node_id)
            .map_err(|error| format!("failed to claim moat task: {error}"))?;
    }

    println!("moat dispatch next");
    println!("dry_run={}", command.dry_run);
    println!("claimed={}", !command.dry_run);
    println!("round_id={round_id}");
    println!("node_id={}", selected.node_id);
    println!("role={}", format_agent_role(selected.role));
    println!("kind={}", format_moat_task_kind(selected.kind));
    println!("title={}", escape_assignment_output_field(&selected.title));
    println!(
        "dependencies={}",
        format_task_graph_dependencies(&selected.depends_on)
    );
    println!(
        "spec_ref={}",
        selected
            .spec_ref
            .as_deref()
            .map(escape_assignment_output_field)
            .unwrap_or_else(|| "<none>".to_string())
    );
    println!(
        "complete_command=mdid-cli moat complete-task --history-path {} --node-id {} --artifact-ref <artifact-ref> --artifact-summary <artifact-summary>",
        command.history_path,
        selected.node_id
    );
    if !command.dry_run {
        println!("previous_state=ready");
        println!("new_state=in_progress");
    }

    Ok(())
}

fn run_moat_claim_task(command: &MoatClaimTaskCommand) -> Result<(), String> {
    let mut store = LocalMoatHistoryStore::open_existing(&command.history_path)
        .map_err(|error| format!("failed to open moat history store: {error}"))?;
    if store.entries().is_empty() {
        return Err(
            "moat history is empty; run `mdid-cli moat round --history-path <path>` first"
                .to_string(),
        );
    }

    let selected_round_id = if let Some(round_id) = command.round_id.as_deref() {
        let entry = store
            .entries()
            .iter()
            .find(|entry| entry.report.summary.round_id.to_string() == round_id)
            .ok_or_else(|| format!("moat round not found: {round_id}"))?;
        entry.report.summary.round_id.to_string()
    } else {
        store
            .entries()
            .last()
            .ok_or_else(|| {
                "moat history is empty; run `mdid-cli moat round --history-path <path>` first"
                    .to_string()
            })?
            .report
            .summary
            .round_id
            .to_string()
    };

    store
        .claim_ready_task(command.round_id.as_deref(), &command.node_id)
        .map_err(|error| format!("failed to claim moat task: {error}"))?;

    println!("moat task claimed");
    println!("round_id={selected_round_id}");
    println!("node_id={}", command.node_id);
    println!("previous_state=ready");
    println!("new_state=in_progress");
    println!("history_path={}", command.history_path);

    Ok(())
}

fn run_moat_complete_task(command: &MoatCompleteTaskCommand) -> Result<(), String> {
    let mut store = LocalMoatHistoryStore::open_existing(&command.history_path)
        .map_err(|error| format!("failed to open moat history store: {error}"))?;
    if store.entries().is_empty() {
        return Err(
            "moat history is empty; run `mdid-cli moat round --history-path <path>` first"
                .to_string(),
        );
    }

    let artifact = match (&command.artifact_ref, &command.artifact_summary) {
        (Some(artifact_ref), Some(artifact_summary)) => Some(CompleteTaskArtifact {
            artifact_ref: artifact_ref.clone(),
            artifact_summary: artifact_summary.clone(),
            recorded_at: Utc::now(),
        }),
        _ => None,
    };

    let selected_round_id = store
        .complete_in_progress_task_with_artifact(
            command.round_id.as_deref(),
            &command.node_id,
            artifact,
        )
        .map_err(|error| format!("failed to complete moat task: {error}"))?;

    println!("moat task completed");
    println!("round_id={selected_round_id}");
    println!("node_id={}", command.node_id);
    println!("previous_state=in_progress");
    println!("new_state=completed");
    println!("history_path={}", command.history_path);
    println!(
        "artifact_recorded={}",
        command.artifact_ref.is_some() && command.artifact_summary.is_some()
    );
    println!(
        "artifact_ref={}",
        command
            .artifact_ref
            .as_deref()
            .map(escape_assignment_output_field)
            .unwrap_or_else(|| "<none>".to_string())
    );
    println!(
        "artifact_summary={}",
        command
            .artifact_summary
            .as_deref()
            .map(escape_assignment_output_field)
            .unwrap_or_else(|| "<none>".to_string())
    );

    let updated_store = LocalMoatHistoryStore::open_existing(&command.history_path)
        .map_err(|error| format!("failed to reload moat history store: {error}"))?;
    let updated_entry = updated_store
        .entries()
        .iter()
        .find(|entry| entry.report.summary.round_id.to_string() == selected_round_id)
        .ok_or_else(|| format!("moat round not found after completion: {selected_round_id}"))?;
    let ready_ids = updated_entry
        .report
        .control_plane
        .task_graph
        .ready_node_ids();
    let next_ready_nodes = updated_entry
        .report
        .control_plane
        .task_graph
        .nodes
        .iter()
        .filter(|node| ready_ids.iter().any(|ready_id| ready_id == &node.node_id))
        .collect::<Vec<_>>();

    println!("next_ready_task_entries={}", next_ready_nodes.len());
    for node in next_ready_nodes {
        println!(
            "next_ready_task={}|{}|{}|{}|{}",
            format_agent_role(node.role),
            escape_assignment_output_field(&node.node_id),
            escape_assignment_output_field(&node.title),
            format_moat_task_kind(node.kind),
            node.spec_ref
                .as_deref()
                .map(escape_assignment_output_field)
                .unwrap_or_else(|| "<none>".to_string())
        );
    }

    Ok(())
}

fn run_moat_release_task(command: &MoatReleaseTaskCommand) -> Result<(), String> {
    let mut store = LocalMoatHistoryStore::open_existing(&command.history_path)
        .map_err(|error| format!("failed to open moat history store: {error}"))?;
    if store.entries().is_empty() {
        return Err(
            "moat history is empty; run `mdid-cli moat round --history-path <path>` first"
                .to_string(),
        );
    }

    let selected_round_id = store
        .release_in_progress_task(command.round_id.as_deref(), &command.node_id)
        .map_err(|error| match error {
            CompleteInProgressTaskError::NodeNotInProgress { node_id, state, .. } => format!(
                "failed to release moat task: node '{node_id}' is {}, expected in_progress",
                format_task_node_state(state)
            ),
            other => format!("failed to release moat task: {other}"),
        })?;

    println!("moat task released");
    println!("round_id={selected_round_id}");
    println!("node_id={}", command.node_id);

    Ok(())
}

fn run_moat_block_task(command: &MoatBlockTaskCommand) -> Result<(), String> {
    let mut store = LocalMoatHistoryStore::open_existing(&command.history_path)
        .map_err(|error| format!("failed to open moat history store: {error}"))?;
    if store.entries().is_empty() {
        return Err(
            "moat history is empty; run `mdid-cli moat round --history-path <path>` first"
                .to_string(),
        );
    }

    let selected_round_id = store
        .block_in_progress_task(command.round_id.as_deref(), &command.node_id)
        .map_err(|error| format!("failed to block moat task: {error}"))?;

    println!("moat task blocked");
    println!("round_id={selected_round_id}");
    println!("node_id={}", command.node_id);
    println!("previous_state=in_progress");
    println!("new_state=blocked");
    println!("history_path={}", command.history_path);

    Ok(())
}

fn run_moat_unblock_task(command: &MoatUnblockTaskCommand) -> Result<(), String> {
    let mut store = LocalMoatHistoryStore::open_existing(&command.history_path)
        .map_err(|error| format!("failed to open moat history store: {error}"))?;
    if store.entries().is_empty() {
        return Err(
            "moat history is empty; run `mdid-cli moat round --history-path <path>` first"
                .to_string(),
        );
    }

    let selected_round_id = store
        .unblock_blocked_task(command.round_id.as_deref(), &command.node_id)
        .map_err(|error| match error {
            CompleteInProgressTaskError::NodeNotInExpectedState {
                node_id,
                state,
                expected_state,
                ..
            } => format!(
                "error: node '{node_id}' is {}, expected {}",
                format_task_node_state(state),
                format_task_node_state(expected_state)
            ),
            other => format!("failed to unblock moat task: {other}"),
        })?;

    println!("moat task unblocked");
    println!("round_id={selected_round_id}");
    println!("node_id={}", command.node_id);
    println!("previous_state=blocked");
    println!("new_state=ready");
    println!("history_path={}", command.history_path);

    Ok(())
}

fn run_moat_ready_tasks(command: &MoatReadyTasksCommand) -> Result<(), String> {
    let store = LocalMoatHistoryStore::open_existing(&command.history_path)
        .map_err(|error| format!("failed to open moat history store: {error}"))?;
    if store.entries().is_empty() {
        return Err(
            "moat history is empty; run `mdid-cli moat round --history-path <path>` first"
                .to_string(),
        );
    }

    let selected = if let Some(round_id) = command.round_id.as_deref() {
        store
            .entries()
            .iter()
            .find(|entry| entry.report.summary.round_id.to_string() == round_id)
    } else {
        Some(store.entries().last().ok_or_else(|| {
            "moat history is empty; run `mdid-cli moat round --history-path <path>` first"
                .to_string()
        })?)
    };

    let Some(entry) = selected else {
        println!("moat ready tasks");
        println!("ready_task_entries=0");
        return Ok(());
    };

    let ready_ids = entry.report.control_plane.task_graph.ready_node_ids();
    let mut ready_nodes = entry
        .report
        .control_plane
        .task_graph
        .nodes
        .iter()
        .filter(|node| ready_ids.iter().any(|ready_id| ready_id == &node.node_id))
        .filter(|node| command.role.map(|role| node.role == role).unwrap_or(true))
        .filter(|node| command.kind.map(|kind| node.kind == kind).unwrap_or(true))
        .filter(|node| {
            command
                .node_id
                .as_deref()
                .map(|expected_node_id| node.node_id == expected_node_id)
                .unwrap_or(true)
        })
        .filter(|node| {
            command
                .depends_on
                .as_deref()
                .map(|dependency| {
                    node.depends_on
                        .iter()
                        .any(|candidate| candidate == dependency)
                })
                .unwrap_or(true)
        })
        .filter(|node| !command.no_dependencies || node.depends_on.is_empty())
        .filter(|node| {
            command
                .title_contains
                .as_deref()
                .map(|title_contains| node.title.contains(title_contains))
                .unwrap_or(true)
        })
        .filter(|node| {
            command
                .spec_ref
                .as_deref()
                .map(|spec_ref| node.spec_ref.as_deref() == Some(spec_ref))
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();

    if let Some(limit) = command.limit {
        ready_nodes.truncate(limit);
    }

    println!("moat ready tasks");
    println!("ready_task_entries={}", ready_nodes.len());
    for node in ready_nodes {
        println!(
            "ready_task={}|{}|{}|{}|{}",
            format_agent_role(node.role),
            format_moat_task_kind(node.kind),
            escape_assignment_output_field(&node.node_id),
            escape_assignment_output_field(&node.title),
            node.spec_ref
                .as_deref()
                .map(escape_assignment_output_field)
                .unwrap_or_else(|| "<none>".to_string())
        );
    }

    Ok(())
}

fn task_graph_node_contains(node: &mdid_domain::MoatTaskNode, needle: &str) -> bool {
    node.node_id.contains(needle)
        || node.title.contains(needle)
        || format_moat_task_kind(node.kind).contains(needle)
        || format_task_node_state(node.state).contains(needle)
        || node
            .depends_on
            .iter()
            .any(|dependency| dependency.contains(needle))
        || node
            .spec_ref
            .as_deref()
            .map(|spec_ref| spec_ref.contains(needle))
            .unwrap_or(false)
}

fn format_task_graph_dependencies(depends_on: &[String]) -> String {
    if depends_on.is_empty() {
        "<none>".to_string()
    } else {
        depends_on
            .iter()
            .map(|dependency| escape_assignment_output_field(dependency))
            .collect::<Vec<_>>()
            .join(",")
    }
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

fn select_moat_export_entry<'a>(
    entries: &'a [MoatHistoryEntry],
    round_id: Option<&str>,
) -> Result<&'a MoatHistoryEntry, String> {
    match round_id {
        Some(round_id) => entries
            .iter()
            .find(|entry| entry.report.summary.round_id.to_string() == round_id)
            .ok_or_else(|| format!("error: no moat history entry matched round_id {round_id}")),
        None => entries.last().ok_or_else(|| {
            "moat history is empty; run `mdid-cli moat round --history-path <path>` first"
                .to_string()
        }),
    }
}

fn run_moat_export_specs(
    history_path: &str,
    output_dir: &str,
    round_id: Option<&str>,
) -> Result<(), String> {
    let store = LocalMoatHistoryStore::open_existing(history_path)
        .map_err(|error| format!("failed to open moat history store: {error}"))?;
    let latest = select_moat_export_entry(store.entries(), round_id)?;

    if latest.report.summary.implemented_specs.is_empty() {
        return Err("selected moat round does not contain implemented_specs handoffs".to_string());
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

fn run_moat_export_plans(
    history_path: &str,
    output_dir: &str,
    round_id: Option<&str>,
) -> Result<(), String> {
    let store = LocalMoatHistoryStore::open_existing(history_path)
        .map_err(|error| format!("failed to open moat history store: {error}"))?;
    let latest = select_moat_export_entry(store.entries(), round_id)?;

    if latest.report.summary.implemented_specs.is_empty() {
        return Err("selected moat round does not contain implemented_specs handoffs".to_string());
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
    println!("round_id={}", latest.report.summary.round_id);
    println!(
        "exported_plans={}",
        latest.report.summary.implemented_specs.join(",")
    );
    println!("written_files={}", written_files.join(","));
    println!("output_dir={output_dir}");

    Ok(())
}

fn print_empty_filtered_history_summary() {
    println!("moat history summary");
    println!("entries=0");
    println!("latest_round_id=none");
    println!("latest_decision=none");
}

fn print_filtered_history_summary(entries: &[&MoatHistoryEntry]) {
    let latest = entries
        .last()
        .expect("filtered history summary should have at least one entry");
    let summary = MoatHistorySummary {
        entry_count: entries.len(),
        latest_round_id: Some(latest.report.summary.round_id.to_string()),
        latest_continue_decision: Some(latest.report.summary.continue_decision),
        latest_stop_reason: latest.report.summary.stop_reason.clone(),
        latest_decision_summary: latest.report.control_plane.memory.latest_decision_summary(),
        latest_implemented_specs: latest.report.summary.implemented_specs.clone(),
        latest_moat_score_after: Some(latest.report.summary.moat_score_after),
        best_moat_score_after: entries
            .iter()
            .map(|entry| entry.report.summary.moat_score_after)
            .max(),
        improvement_deltas: entries
            .iter()
            .map(|entry| entry.report.summary.improvement())
            .collect(),
    };
    print_history_summary(&summary);
}

fn moat_history_entry_search_text(entry: &MoatHistoryEntry) -> String {
    let summary = &entry.report.summary;
    let mut text = format!(
        "round id {} selected strategies {} implemented specs {} tests_passed={} moat score before {} moat score after {} continue decision {}",
        summary.round_id,
        summary.selected_strategies.join(" "),
        summary.implemented_specs.join(" "),
        summary.tests_passed,
        summary.moat_score_before,
        summary.moat_score_after,
        format_continue_decision(summary.continue_decision)
    );

    if let Some(reason) = &summary.stop_reason {
        text.push(' ');
        text.push_str(reason);
    }
    if let Some(reason) = &summary.pivot_reason {
        text.push(' ');
        text.push_str(reason);
    }
    if let Some(decision_summary) = entry.report.control_plane.memory.latest_decision_summary() {
        text.push(' ');
        text.push_str(&decision_summary);
    }
    for decision in &entry.report.control_plane.memory.decisions {
        text.push(' ');
        text.push_str(&decision.summary);
        text.push(' ');
        text.push_str(&decision.rationale);
    }
    text.push(' ');
    text.push_str(&format!("{:?}", entry.report));
    text
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
        MoatTaskNodeKind::LockInAnalysis => "lock_in_analysis",
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
    "usage: mdid-cli [status | moat round [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false] [--history-path PATH] | moat control-plane [--history-path PATH] [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false] | moat history --history-path PATH [--round-id ROUND_ID] [--decision Continue|Stop|Pivot] [--contains TEXT] [--stop-reason-contains TEXT] [--min-score N] [--tests-passed true|false] [--limit N] | moat decision-log --history-path PATH [--round-id ROUND_ID] [--role planner|coder|reviewer] [--contains TEXT] [--summary-contains TEXT] [--rationale-contains TEXT] [--limit N] | moat assignments --history-path PATH [--round-id ROUND_ID] [--role planner|coder|reviewer] [--state pending|ready|in_progress|completed|blocked] [--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation] [--node-id NODE_ID] [--depends-on NODE_ID] [--no-dependencies] [--title-contains TEXT] [--spec-ref SPEC_REF] [--contains TEXT] [--limit N] | moat task-graph --history-path PATH [--round-id ROUND_ID] [--role planner|coder|reviewer] [--state pending|ready|in_progress|completed|blocked] [--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation] [--node-id NODE_ID] [--depends-on NODE_ID] [--no-dependencies] [--title-contains TEXT] [--spec-ref SPEC_REF] [--contains TEXT] [--limit N] | moat ready-tasks --history-path PATH [--round-id ROUND_ID] [--role planner|coder|reviewer] [--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation] [--node-id NODE_ID] [--depends-on NODE_ID] [--no-dependencies] [--title-contains TEXT] [--spec-ref SPEC_REF] [--limit N] | moat artifacts --history-path PATH [--round-id ROUND_ID] [--role planner|coder|reviewer] [--state pending|ready|in_progress|completed|blocked] [--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation] [--node-id NODE_ID] [--contains TEXT] [--artifact-ref TEXT] [--artifact-summary TEXT] [--limit N] | moat dispatch-next --history-path PATH [--round-id ROUND_ID] [--role planner|coder|reviewer] [--kind market_scan|competitor_analysis|lock_in_analysis|strategy_generation|spec_planning|implementation|review|evaluation] [--dry-run] | moat claim-task --history-path PATH --node-id NODE_ID [--round-id ROUND_ID] | moat complete-task --history-path PATH --node-id NODE_ID [--round-id ROUND_ID] [--artifact-ref TEXT --artifact-summary TEXT] | moat release-task --history-path PATH --node-id NODE_ID [--round-id ROUND_ID] | moat block-task --history-path PATH --node-id NODE_ID [--round-id ROUND_ID] | moat unblock-task --history-path PATH --node-id NODE_ID [--round-id ROUND_ID] | moat continue --history-path PATH [--improvement-threshold N] | moat schedule-next --history-path PATH [--improvement-threshold N] | moat export-specs --history-path PATH [--round-id ROUND_ID] --output-dir DIR | moat export-plans --history-path PATH [--round-id ROUND_ID] --output-dir DIR]"
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
            CliCommand::MoatHistory(MoatHistoryCommand {
                history_path: "history.json".into(),
                round_id: None,
                decision: None,
                contains: None,
                stop_reason_contains: None,
                tests_passed: None,
                min_score: None,
                limit: None,
            })
        );
        assert_eq!(
            parse_command(&[
                "moat".into(),
                "history".into(),
                "--history-path".into(),
                "history.json".into(),
                "--stop-reason-contains".into(),
                "budget".into(),
            ])
            .unwrap(),
            CliCommand::MoatHistory(MoatHistoryCommand {
                history_path: "history.json".into(),
                round_id: None,
                decision: None,
                contains: None,
                stop_reason_contains: Some("budget".into()),
                tests_passed: None,
                min_score: None,
                limit: None,
            })
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

    #[test]
    fn parse_moat_history_command_parses_stop_reason_contains() {
        assert_eq!(
            parse_moat_history_command(&[
                "--history-path".into(),
                "history.json".into(),
                "--stop-reason-contains".into(),
                "budget".into(),
                "--limit".into(),
                "5".into(),
            ])
            .unwrap(),
            MoatHistoryCommand {
                history_path: "history.json".into(),
                round_id: None,
                decision: None,
                contains: None,
                stop_reason_contains: Some("budget".into()),
                tests_passed: None,
                min_score: None,
                limit: Some(5),
            }
        );
    }

    #[test]
    fn parse_moat_history_command_parses_round_id() {
        assert_eq!(
            parse_moat_history_command(&[
                "--history-path".into(),
                "history.json".into(),
                "--round-id".into(),
                "round-123".into(),
            ])
            .unwrap(),
            MoatHistoryCommand {
                history_path: "history.json".into(),
                round_id: Some("round-123".into()),
                decision: None,
                contains: None,
                stop_reason_contains: None,
                tests_passed: None,
                min_score: None,
                limit: None,
            }
        );
    }

    #[test]
    fn parse_moat_decision_log_rejects_flag_like_round_id_value() {
        assert_eq!(
            parse_moat_decision_log_command(&[
                "--history-path".into(),
                "history.json".into(),
                "--round-id".into(),
                "--role".into(),
                "planner".into(),
            ]),
            Err("missing value for --round-id".into())
        );
    }

    #[test]
    fn parses_moat_release_task_command() {
        let args = vec![
            "moat".to_string(),
            "release-task".to_string(),
            "--history-path".to_string(),
            "history.json".to_string(),
            "--round-id".to_string(),
            "round-1".to_string(),
            "--node-id".to_string(),
            "strategy_generation".to_string(),
        ];

        assert_eq!(
            parse_command(&args),
            Ok(CliCommand::MoatReleaseTask(MoatReleaseTaskCommand {
                history_path: "history.json".to_string(),
                round_id: Some("round-1".to_string()),
                node_id: "strategy_generation".to_string(),
            }))
        );
    }

    #[test]
    fn parse_moat_artifacts_command_requires_history_path() {
        let args = vec!["moat".to_string(), "artifacts".to_string()];

        assert_eq!(
            parse_command(&args),
            Err("missing required flag: --history-path".to_string())
        );
    }

    #[test]
    fn parse_moat_artifacts_command_accepts_round_node_contains_and_limit_filters() {
        let args = vec![
            "moat".to_string(),
            "artifacts".to_string(),
            "--history-path".to_string(),
            "history.json".to_string(),
            "--round-id".to_string(),
            "round-7".to_string(),
            "--node-id".to_string(),
            "implementation-task".to_string(),
            "--contains".to_string(),
            "handoff".to_string(),
            "--artifact-ref".to_string(),
            "review handoff".to_string(),
            "--artifact-summary".to_string(),
            "approved release".to_string(),
            "--limit".to_string(),
            "2".to_string(),
        ];

        assert_eq!(
            parse_command(&args),
            Ok(CliCommand::MoatArtifacts(MoatArtifactsCommand {
                history_path: "history.json".to_string(),
                round_id: Some("round-7".to_string()),
                role: None,
                state: None,
                kind: None,
                node_id: Some("implementation-task".to_string()),
                contains: Some("handoff".to_string()),
                artifact_ref: Some("review handoff".to_string()),
                artifact_summary: Some("approved release".to_string()),
                limit: Some(2),
            }))
        );
    }
}
