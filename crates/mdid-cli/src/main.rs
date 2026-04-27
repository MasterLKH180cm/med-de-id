fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match parse_command(&args) {
        Ok(CliCommand::Status) => println!("med-de-id CLI ready"),
        Err(error) => exit_with_usage(&error),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CliCommand {
    Status,
}

fn parse_command(args: &[String]) -> Result<CliCommand, String> {
    match args {
        [] => Ok(CliCommand::Status),
        [status] if status == "status" => Ok(CliCommand::Status),
        _ => Err(format!("unknown command: {}", args.join(" "))),
    }
}

fn exit_with_usage(error: &str) -> ! {
    eprintln!("{error}");
    eprintln!("usage: mdid-cli [status]");
    std::process::exit(1);
}
