use mdid_domain::{
    CompetitorProfile, ContinueDecision, LockInReport, MarketMoatSnapshot, MoatStrategy,
    MoatTaskNodeState, MoatType, ResourceBudget,
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

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match parse_command(&args) {
        Ok(CliCommand::Status) => println!("med-de-id CLI ready"),
        Ok(CliCommand::MoatRound(command)) => {
            if let Err(error) = run_moat_round(&command) {
                exit_with_error(error);
            }
        }
        Ok(CliCommand::MoatControlPlane(overrides)) => run_moat_control_plane(&overrides),
        Ok(CliCommand::MoatHistory(history_path)) => {
            if let Err(error) = run_moat_history(&history_path) {
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
        Err(error) => exit_with_usage(error),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CliCommand {
    Status,
    MoatRound(MoatRoundCommand),
    MoatControlPlane(MoatRoundOverrides),
    MoatHistory(String),
    MoatContinue {
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
            let (overrides, _) = parse_moat_round_overrides(rest, false)?;
            Ok(CliCommand::MoatControlPlane(overrides))
        }
        [moat, history, rest @ ..] if moat == "moat" && history == "history" => {
            Ok(CliCommand::MoatHistory(parse_required_history_path(rest)?))
        }
        [moat, continue_command, rest @ ..] if moat == "moat" && continue_command == "continue" => {
            parse_moat_continue_command(rest)
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
                improvement_threshold = value
                    .parse::<i16>()
                    .map_err(|_| format!("invalid value for --improvement-threshold: {value}"))?;
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

fn run_moat_control_plane(overrides: &MoatRoundOverrides) {
    let report = sample_round_report(overrides);
    let control_plane = report.control_plane;
    let ready_nodes = format_ready_nodes(&control_plane.task_graph.ready_node_ids());
    let latest_decision_summary = control_plane
        .memory
        .latest_decision_summary()
        .unwrap_or_else(|| "<none>".to_string());
    let task_states = format_task_states(&control_plane.task_graph.nodes);

    println!("moat control plane snapshot");
    println!("ready_nodes={ready_nodes}");
    println!("latest_decision_summary={latest_decision_summary}");
    println!(
        "improvement_delta={}",
        control_plane.memory.improvement_delta
    );
    println!("task_states={task_states}");
}

fn run_moat_history(history_path: &str) -> Result<(), String> {
    let store = LocalMoatHistoryStore::open_existing(history_path)
        .map_err(|error| format!("failed to open moat history store: {error}"))?;
    print_history_summary(&store.summary());
    Ok(())
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

fn format_task_node_state(state: MoatTaskNodeState) -> &'static str {
    match state {
        MoatTaskNodeState::Pending => "pending",
        MoatTaskNodeState::Ready => "ready",
        MoatTaskNodeState::InProgress => "in_progress",
        MoatTaskNodeState::Completed => "completed",
        MoatTaskNodeState::Blocked => "blocked",
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
    "usage: mdid-cli [status | moat round [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false] [--history-path PATH] | moat control-plane [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false] | moat history --history-path PATH | moat continue --history-path PATH [--improvement-threshold N]]"
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
            CliCommand::MoatControlPlane(MoatRoundOverrides {
                strategy_candidates: Some(0),
                ..MoatRoundOverrides::default()
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
    }
}
