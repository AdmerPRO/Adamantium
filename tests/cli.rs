use std::{
    fs,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT_PROJECT: AtomicUsize = AtomicUsize::new(0);

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
    for arguments in [vec!["--unknown"], vec!["build", "one", "two"], vec!["new"]] {
        let output = adamantium().args(arguments).output().unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr).contains("use --help"));
    }
}

#[test]
fn new_creates_a_complete_project_without_overwriting_it() {
    let base = std::env::temp_dir().join(format!(
        "adamantium-cli-new-{}-{}",
        std::process::id(),
        NEXT_PROJECT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&base).unwrap();
    let output = adamantium()
        .current_dir(&base)
        .args(["new", "MyProject"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let project = base.join("MyProject");
    for path in [
        "project.toml",
        "requirement.toml",
        "code/main.ad",
        "target",
        ".gitignore",
    ] {
        assert!(project.join(path).exists(), "missing {path}");
    }
    assert!(
        fs::read_to_string(project.join("project.toml"))
            .unwrap()
            .contains("name = \"MyProject\"")
    );
    let repeated = adamantium()
        .current_dir(&base)
        .args(["new", "MyProject"])
        .output()
        .unwrap();
    assert_eq!(repeated.status.code(), Some(1));
    fs::remove_dir_all(&base).unwrap();
}
