use std::process::Command;

fn adamantium() -> Command {
    Command::new(env!("CARGO_BIN_EXE_adamantium"))
}

#[test]
fn help_and_version_are_available() {
    for argument in ["--help", "-h"] {
        let output = adamantium().arg(argument).output().unwrap();
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("adamantium build [PROJECT_DIRECTORY]"));
        assert!(stdout.contains("adamantium run [PROJECT_DIRECTORY]"));
    }

    let output = adamantium().arg("--version").output().unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        format!("adamantium {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn no_arguments_shows_help() {
    let output = adamantium().output().unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Usage:"));
}

#[test]
fn invalid_cli_arguments_are_rejected() {
    for arguments in [vec!["--unknown"], vec!["build", "one", "two"]] {
        let output = adamantium().args(arguments).output().unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr).contains("use --help"));
    }
}
