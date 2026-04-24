fn main() {
    let command = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "status".to_string());

    match command.as_str() {
        "status" => println!("med-de-id CLI ready"),
        other => {
            eprintln!("unknown command: {other}");
            std::process::exit(1);
        }
    }
}
