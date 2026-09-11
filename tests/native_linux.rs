#![cfg(target_os = "linux")]

use std::{fs, process::Command};

#[test]
#[ignore = "requires NASM and a C linker"]
fn builds_and_runs_linux_x86_64_executable() {
    assert_eq!(std::env::consts::ARCH, "x86_64");
    let root = std::env::temp_dir().join(format!("adamantium-linux-native-{}", std::process::id()));
    fs::create_dir_all(root.join("code")).unwrap();
    fs::write(
        root.join("project.toml"),
        "name=\"LinuxNative\"\nversion=\"1.0.0\"\ndescription=\"\"\nauthors=[]\n",
    )
    .unwrap();
    fs::write(root.join("requirement.toml"), "[packages]\n").unwrap();
    fs::write(
        root.join("code/main.ad"),
        "fun main(value:int) { var result=value*2; print.newline(result); }",
    )
    .unwrap();
    let build = Command::new(env!("CARGO_BIN_EXE_adamantium"))
        .args(["build", root.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let executable = root.join("target/LinuxNative");
    let output = Command::new(executable)
        .args(["--value", "21"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, b"42\r\n");
    fs::remove_dir_all(root).unwrap();
}
