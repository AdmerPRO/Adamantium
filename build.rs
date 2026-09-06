use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=runtime/src");
    println!("cargo:rerun-if-changed=runtime/Cargo.toml");
    println!("cargo:rerun-if-changed=runtime/Cargo.lock");
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let target = env::var("TARGET").unwrap();
    if target != "x86_64-pc-windows-msvc" {
        fs::write(out.join("runtime.lib"), []).unwrap();
        fs::write(out.join("runtime-libraries.txt"), "").unwrap();
        return;
    }
    let result = Command::new(env::var_os("CARGO").unwrap())
        .args([
            "rustc",
            "--manifest-path",
            "runtime/Cargo.toml",
            "--release",
            "--locked",
            "--target",
            &target,
            "--target-dir",
        ])
        .arg(out.join("runtime-build"))
        .args(["--", "--print=native-static-libs"])
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env_remove("RUSTFLAGS")
        .output()
        .expect("could not build the Adamantium runtime");
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(result.status.success(), "runtime build failed:\n{stderr}");
    let libraries = stderr
        .lines()
        .find_map(|line| line.strip_prefix("note: native-static-libs: "))
        .expect("rustc did not report runtime system libraries");
    fs::copy(
        out.join("runtime-build")
            .join(target)
            .join("release/adamantium_runtime.lib"),
        out.join("runtime.lib"),
    )
    .unwrap();
    fs::write(out.join("runtime-libraries.txt"), libraries).unwrap();
}
