use std::{fs, process::Command};

#[test]
fn invalid_access_and_imports_fail_before_codegen() {
    let root = std::env::temp_dir().join(format!("adamantium-diagnostics-{}", std::process::id()));
    fs::create_dir_all(root.join("code")).unwrap();
    fs::write(root.join("project.toml"),"name = \"Diagnostics\"\nversion = \"1\"\ndescription = \"Diagnostic tests\"\nauthors = []\n").unwrap();
    fs::write(root.join("requirement.toml"), "[packages]\n").unwrap();
    for (source, message) in [
        (
            "use missing:[value]; fun main() {}",
            "module 'missing' does not export 'value'",
        ),
        (
            "fun main() {\nvar a = 1;\na.unknown();\n}",
            "3:3: invalid access:",
        ),
        (
            "fun main() {\nvar a = 1;\nprint.newline(a[0]);\n}",
            "invalid access:",
        ),
    ] {
        fs::write(root.join("code/main.ad"), source).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_adamantium"))
            .arg("build")
            .arg(&root)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("main.ad:") && stderr.contains(message),
            "{stderr}"
        );
        assert!(!root.join("target").exists());
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn missing_and_circular_packed_modules_are_reported() {
    let root =
        std::env::temp_dir().join(format!("adamantium-module-errors-{}", std::process::id()));
    fs::create_dir_all(root.join("code")).unwrap();
    fs::write(
        root.join("project.toml"),
        "name=\"Modules\"\nversion=\"1\"\ndescription=\"\"\nauthors=[]\n",
    )
    .unwrap();
    fs::write(root.join("requirement.toml"), "[packages]\n").unwrap();

    fs::write(root.join("code/main.ad"), "pack missing; fun main() {}").unwrap();
    let missing = Command::new(env!("CARGO_BIN_EXE_adamantium"))
        .args(["build", root.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&missing.stderr).contains("could not load module 'missing'"));

    fs::write(root.join("code/main.ad"), "pack a; fun main() {}").unwrap();
    fs::write(root.join("code/a.ad"), "pack b; fun a() r:None {}").unwrap();
    fs::write(root.join("code/b.ad"), "pack a; fun b() r:None {}").unwrap();
    let cycle = Command::new(env!("CARGO_BIN_EXE_adamantium"))
        .args(["build", root.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(cycle.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&cycle.stderr).contains("circular pack dependency: a -> b -> a")
    );
    assert!(!root.join("target").exists());
    fs::remove_dir_all(root).unwrap();
}
