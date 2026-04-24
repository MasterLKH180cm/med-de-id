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
