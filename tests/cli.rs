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
        assert!(stdout.contains("adamantium check [PROJECT_DIRECTORY]"));
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
fn check_analyzes_projects_without_build_tools_or_artifacts() {
    let base = std::env::temp_dir().join(format!(
        "adamantium-cli-check-{}-{}",
        std::process::id(),
        NEXT_PROJECT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(base.join("code")).unwrap();
    fs::write(
        base.join("project.toml"),
        "name=\"CheckedProject\"\nversion=\"1.0.0\"\ndescription=\"\"\nauthors=[]\n",
    )
    .unwrap();
    fs::write(base.join("requirement.toml"), "[packages]\n").unwrap();
    fs::write(base.join("code/main.ad"), "fun main() { var unused=1; }").unwrap();

    let output = adamantium()
        .args(["check", base.to_str().unwrap()])
        .env("ADAMANTIUM_NASM", "missing-nasm-for-check-test")
        .env("ADAMANTIUM_LINKER", "missing-linker-for-check-test")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("Checked"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("warning[W002]"));
    assert!(!base.join("target").exists());

    fs::write(
        base.join("code/main.ad"),
        "fun main() { var value=\"wrong\":int; }",
    )
    .unwrap();
    let invalid = adamantium()
        .args(["check", base.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(invalid.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&invalid.stderr);
    assert!(stderr.contains("error[E300]"), "{stderr}");
    assert!(stderr.contains("main.ad"), "{stderr}");
    assert!(stderr.contains("1 | fun main()"), "{stderr}");
    assert!(stderr.contains("context: type checking"), "{stderr}");
    assert!(stderr.contains("help:"), "{stderr}");
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn test_list_discovers_annotated_functions_without_building() {
    let base = std::env::temp_dir().join(format!(
        "adamantium-cli-test-list-{}-{}",
        std::process::id(),
        NEXT_PROJECT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(base.join("code")).unwrap();
    fs::write(
        base.join("project.toml"),
        "name=\"Tests\"\nversion=\"1.0.0\"\ndescription=\"\"\nauthors=[]\n",
    )
    .unwrap();
    fs::write(base.join("requirement.toml"), "[packages]\n").unwrap();
    fs::write(
        base.join("code/main.ad"),
        "fun add(a:int,b:int) r:int { r=a+b; } fun main() {}",
    )
    .unwrap();
    fs::write(
        base.join("code/tests.ad"),
        "#[test]\nfun addition() { print.newline(add(2,2)); }\n\n#[test]\nfun empty() {}\n",
    )
    .unwrap();
    let output = adamantium()
        .args(["test", "list", base.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "addition\nempty\n");
    assert!(!base.join("target").exists());
    fs::remove_dir_all(base).unwrap();
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
    let misspelled_build = String::from_utf8([98, 117, 105, 100].into()).unwrap(); // i cant put there what it means ): ; typos flag it as a typo (it is tho)
    let typo = adamantium().arg(misspelled_build).output().unwrap();
    assert_eq!(typo.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&typo.stderr).contains("Did you mean 'build'?"));
}

#[test]
fn reports_multiple_independent_manifest_errors() {
    let base = std::env::temp_dir().join(format!(
        "adamantium-cli-multiple-errors-{}-{}",
        std::process::id(),
        NEXT_PROJECT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(base.join("code")).unwrap();
    fs::write(
        base.join("project.toml"),
        "name=1\nversion=2\nauthors=\"wrong\"\n",
    )
    .unwrap();
    fs::write(base.join("requirement.toml"), "[packages]\nunknown=\"1\"\n").unwrap();
    fs::write(base.join("code/main.ad"), "fun main() {}").unwrap();
    let output = adamantium()
        .args(["check", base.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.matches("error[E400]").count(), 5, "{stderr}");
    assert!(stderr.contains("name must be a string"));
    assert!(stderr.contains("description must be a string"));
    assert!(stderr.contains("package source"));
    fs::remove_dir_all(base).unwrap();
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
        fs::read_to_string(project.join(".gitignore"))
            .unwrap()
            .contains("/packages/")
    );
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
