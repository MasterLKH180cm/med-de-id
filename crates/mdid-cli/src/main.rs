use mdid_domain::{
    CompetitorProfile, ContinueDecision, LockInReport, MarketMoatSnapshot, MoatStrategy, MoatType,
    ResourceBudget,
};
use mdid_runtime::moat::{run_bounded_round, MoatRoundInput};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match args.as_slice() {
        [] => println!("med-de-id CLI ready"),
        [status] if status == "status" => println!("med-de-id CLI ready"),
        [moat, round] if moat == "moat" && round == "round" => run_moat_round(),
        _ => exit_unknown_command(&args),
    }
}

fn run_moat_round() {
    let report = run_bounded_round(sample_round_input());

    println!("moat round complete");
    println!(
        "continue_decision={}",
        format_continue_decision(report.summary.continue_decision)
    );
    println!("executed_tasks={}", report.executed_tasks.join(","));
    println!("moat_score_before={}", report.summary.moat_score_before);
    println!("moat_score_after={}", report.summary.moat_score_after);
}

fn format_continue_decision(continue_decision: ContinueDecision) -> &'static str {
    match continue_decision {
        ContinueDecision::Continue => "Continue",
        ContinueDecision::Stop => "Stop",
        ContinueDecision::Pivot => "Pivot",
    }
}

fn sample_round_input() -> MoatRoundInput {
    MoatRoundInput {
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
    }
}

fn exit_unknown_command(args: &[String]) -> ! {
    let command = if args.is_empty() {
        "<none>".to_string()
    } else {
        args.join(" ")
    };

    eprintln!("unknown command: {command}");
    eprintln!("usage: mdid-cli [status | moat round]");
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
}
