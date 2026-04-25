use mdid_domain::{
    CompetitorProfile, ContinueDecision, LockInReport, MarketMoatSnapshot, MoatStrategy,
    MoatTaskNodeState, MoatType, ResourceBudget,
};
use mdid_runtime::moat::{run_bounded_round, MoatRoundInput, MoatRoundReport};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct MoatRoundOverrides {
    strategy_candidates: Option<u8>,
    spec_generations: Option<u8>,
    implementation_tasks: Option<u8>,
    review_loops: Option<u8>,
    tests_passed: Option<bool>,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match parse_command(&args) {
        Ok(CliCommand::Status) => println!("med-de-id CLI ready"),
        Ok(CliCommand::MoatRound(overrides)) => run_moat_round(&overrides),
        Ok(CliCommand::MoatControlPlane(overrides)) => run_moat_control_plane(&overrides),
        Err(error) => exit_with_usage(error),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CliCommand {
    Status,
    MoatRound(MoatRoundOverrides),
    MoatControlPlane(MoatRoundOverrides),
}

fn parse_command(args: &[String]) -> Result<CliCommand, String> {
    match args {
        [] => Ok(CliCommand::Status),
        [status] if status == "status" => Ok(CliCommand::Status),
        [moat, round, rest @ ..] if moat == "moat" && round == "round" => {
            Ok(CliCommand::MoatRound(parse_moat_round_overrides(rest)?))
        }
        [moat, control_plane, rest @ ..] if moat == "moat" && control_plane == "control-plane" => {
            Ok(CliCommand::MoatControlPlane(parse_moat_round_overrides(
                rest,
            )?))
        }
        _ => Err(format!("unknown command: {}", format_command(args))),
    }
}

fn parse_moat_round_overrides(args: &[String]) -> Result<MoatRoundOverrides, String> {
    let mut overrides = MoatRoundOverrides::default();
    let mut index = 0;

    while index < args.len() {
        let flag = &args[index];

        match flag.as_str() {
            "--strategy-candidates" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| format!("missing value for {flag}"))?;
                overrides.strategy_candidates = Some(parse_u8_flag(flag, value)?);
            }
            "--spec-generations" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| format!("missing value for {flag}"))?;
                overrides.spec_generations = Some(parse_u8_flag(flag, value)?);
            }
            "--implementation-tasks" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| format!("missing value for {flag}"))?;
                overrides.implementation_tasks = Some(parse_u8_flag(flag, value)?);
            }
            "--review-loops" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| format!("missing value for {flag}"))?;
                overrides.review_loops = Some(parse_u8_flag(flag, value)?);
            }
            "--tests-passed" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| format!("missing value for {flag}"))?;
                overrides.tests_passed = Some(parse_bool_flag(flag, value)?);
            }
            _ => return Err(format!("unknown flag: {flag}")),
        }

        index += 2;
    }

    Ok(overrides)
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

fn run_moat_round(overrides: &MoatRoundOverrides) {
    let report = sample_round_report(overrides);

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
    "usage: mdid-cli [status | moat round [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false] | moat control-plane [--strategy-candidates N] [--spec-generations N] [--implementation-tasks N] [--review-loops N] [--tests-passed true|false]]"
}

fn exit_with_usage(message: String) -> ! {
    eprintln!("{message}");
    eprintln!("{}", usage());
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
    fn parse_command_maps_round_and_control_plane_overrides() {
        assert_eq!(
            parse_command(&[
                "moat".into(),
                "round".into(),
                "--review-loops".into(),
                "0".into(),
            ])
            .unwrap(),
            CliCommand::MoatRound(MoatRoundOverrides {
                review_loops: Some(0),
                ..MoatRoundOverrides::default()
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
    }
}
