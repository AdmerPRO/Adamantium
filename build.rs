use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

fn main() {
    println!("cargo:rerun-if-changed=runtime/src");
    println!("cargo:rerun-if-changed=runtime/Cargo.toml");
    println!("cargo:rerun-if-changed=runtime/Cargo.lock");
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let target = env::var("TARGET").unwrap();
    if target != "x86_64-pc-windows-msvc" && target != "x86_64-unknown-linux-gnu" {
        fs::write(out.join("runtime.lib"), []).unwrap();
        fs::write(out.join("runtime-libraries.txt"), "").unwrap();
        fs::write(out.join("runtime-auxiliary-libraries.bin"), []).unwrap();
        return;
    }
    let runtime_target = out.join("runtime-build");
    let mut result = build_runtime(&target, &runtime_target);
    let mut libraries = native_libraries(&result.stderr);
    if result.status.success() && libraries.is_none() {
        let clean = Command::new(env::var_os("CARGO").unwrap())
            .args([
                "clean",
                "--manifest-path",
                "runtime/Cargo.toml",
                "--target-dir",
            ])
            .arg(&runtime_target)
            .output()
            .expect("could not clean the cached Adamantium runtime");
        assert!(clean.status.success(), "runtime cache cleanup failed");
        result = build_runtime(&target, &runtime_target);
        libraries = native_libraries(&result.stderr);
    }
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(result.status.success(), "runtime build failed:\n{stderr}");
    let libraries = libraries.expect("rustc did not report runtime system libraries");
    let library = if target == "x86_64-pc-windows-msvc" {
        "adamantium_runtime.lib"
    } else {
        "libadamantium_runtime.a"
    };
    fs::copy(
        runtime_target.join(target).join("release").join(library),
        out.join("runtime.lib"),
    )
    .unwrap();
    fs::write(out.join("runtime-libraries.txt"), &libraries).unwrap();
    embed_auxiliary_libraries(&out, &libraries);
}

fn embed_auxiliary_libraries(out: &Path, libraries: &str) {
    let mut bundle = Vec::new();
    for name in libraries
        .split_whitespace()
        .filter(|name| name.starts_with("windows.") && name.ends_with(".lib"))
    {
        let path = find_cargo_registry_file(name)
            .unwrap_or_else(|| panic!("could not find Rust runtime import library {name}"));
        let bytes = fs::read(path).unwrap();
        let name = name.as_bytes();
        bundle.extend_from_slice(&(name.len() as u32).to_le_bytes());
        bundle.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        bundle.extend_from_slice(name);
        bundle.extend_from_slice(&bytes);
    }
    fs::write(out.join("runtime-auxiliary-libraries.bin"), bundle).unwrap();
}

fn find_cargo_registry_file(name: &str) -> Option<PathBuf> {
    let cargo_home = env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join(".cargo")))
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".cargo")))?;
    let sources = cargo_home.join("registry/src");
    for registry in fs::read_dir(sources).ok()?.flatten() {
        for package in fs::read_dir(registry.path()).ok()?.flatten() {
            let candidate = package.path().join("lib").join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn build_runtime(target: &str, runtime_target: &Path) -> std::process::Output {
    Command::new(env::var_os("CARGO").unwrap())
        .args([
            "rustc",
            "--manifest-path",
            "runtime/Cargo.toml",
            "--release",
            "--locked",
            "--target",
            target,
            "--target-dir",
        ])
        .arg(runtime_target)
        .args(["--", "--print=native-static-libs"])
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env_remove("RUSTFLAGS")
        .output()
        .expect("could not build the Adamantium runtime")
}

fn native_libraries(stderr: &[u8]) -> Option<String> {
    let stderr = strip_ansi(&String::from_utf8_lossy(stderr));
    stderr
        .lines()
        .find_map(|line| {
            line.split_once("native-static-libs: ")
                .map(|(_, value)| value)
        })
        .map(str::trim)
        .map(str::to_owned)
}

fn strip_ansi(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut characters = text.chars().peekable();
    while let Some(character) = characters.next() {
        if character == '\u{1b}' && characters.peek() == Some(&'[') {
            characters.next();
            for part in characters.by_ref() {
                if ('@'..='~').contains(&part) {
                    break;
                }
            }
        } else {
            result.push(character);
        }
    }
    result
}
