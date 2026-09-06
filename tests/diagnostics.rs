use std::{fs, process::Command};

#[test]
fn invalid_access_and_imports_fail_before_codegen() {
    let root = std::env::temp_dir().join(format!("adamantium-diagnostics-{}", std::process::id()));
    fs::create_dir_all(root.join("code")).unwrap();
    fs::write(root.join("project.toml"),"name = \"Diagnostics\"\nversion = \"1\"\ndescription = \"Diagnostic tests\"\nauthors = []\n").unwrap();
    fs::write(root.join("requirement.toml"), "[packages]\n").unwrap();
    for (source, message) in [
        ("use missing::value; fun main() {}", "1:1: invalid import:"),
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
        let output = Command::new(env!("CARGO_BIN_EXE_adamantium-compiler"))
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
