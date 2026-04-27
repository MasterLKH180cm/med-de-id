use std::process::Command;

#[test]
fn cli_prints_status_banner() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .arg("status")
        .output()
        .expect("failed to run mdid-cli");

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("med-de-id CLI ready"));
}

#[test]
fn cli_prints_ready_banner_with_no_args() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .output()
        .expect("failed to run mdid-cli with no args");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "med-de-id CLI ready\n"
    );
}

#[test]
fn cli_rejects_removed_moat_command_family_with_exact_usage() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .args(["moat", "round"])
        .output()
        .expect("failed to run mdid-cli moat round");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "unknown command: moat round\nusage: mdid-cli [status]\n"
    );
}

#[test]
fn cli_rejects_moat_token_as_unknown_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdid-cli"))
        .arg("moat")
        .output()
        .expect("failed to run mdid-cli moat");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "unknown command: moat\nusage: mdid-cli [status]\n"
    );
}
