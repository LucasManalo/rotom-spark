use std::process::Command;

#[test]
fn cli_starts_and_prints_help() {
    let output = Command::new(env!("CARGO_BIN_EXE_rotom"))
        .output()
        .expect("rotom CLI should start");

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Rotom Spark development CLI"));
}

#[test]
fn cli_reports_workspace_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_rotom"))
        .arg("--version")
        .output()
        .expect("rotom CLI should start");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "rotom 0.1.0\n");
}
