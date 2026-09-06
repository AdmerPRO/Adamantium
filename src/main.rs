mod syntax;

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args_os().skip(1);
    let first = args.next();
    if first.as_deref().is_some_and(|a| a == "--help" || a == "-h") {
        println!(
            "Adamantium compiler (Windows x64)\nUsage: adamantium-compiler [PROJECT_DIRECTORY]\nDefault: ../adamantium-project relative to the compiler source directory.\nRequires NASM and the Visual Studio x64 Native Tools environment.\nOverride tools with ADAMANTIUM_NASM and ADAMANTIUM_LINKER."
        );
        return Ok(());
    }
    if args.next().is_some() {
        return Err("expected at most one project directory; use --help".into());
    }
    let root = first
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../adamantium-project"));
    let root = root
        .canonicalize()
        .map_err(|e| format!("{}: {e}", root.display()))?;
    let project = read_toml(&root.join("project.toml"))?;
    let name = project
        .get("name")
        .and_then(toml::Value::as_str)
        .ok_or("project.toml: name must be a string")?;
    if name.is_empty()
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err(
            "project.toml: name must contain only ASCII letters, digits, underscores or hyphens"
                .into(),
        );
    }
    // Reject Windows device names even when an extension is appended.
    let upper = name.to_ascii_uppercase();
    if [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ]
    .contains(&upper.as_str())
    {
        return Err("project.toml: name is a reserved Windows device name".into());
    }
    for field in ["version", "description"] {
        if project.get(field).and_then(toml::Value::as_str).is_none() {
            return Err(format!("project.toml: {field} must be a string"));
        }
    }
    if !project
        .get("authors")
        .and_then(toml::Value::as_array)
        .is_some_and(|a| a.iter().all(toml::Value::is_str))
    {
        return Err("project.toml: authors must be an array of strings".into());
    }
    let requirements = read_toml(&root.join("requirement.toml"))?;
    for (key, value) in &requirements {
        if key != "packages" || !value.as_table().is_some_and(|t| t.is_empty()) {
            return Err(
                "requirement.toml: packages are not supported yet; use an empty [packages] table"
                    .into(),
            );
        }
    }
    let source_path = root.join("code/main.ad");
    let source =
        fs::read_to_string(&source_path).map_err(|e| format!("{}: {e}", source_path.display()))?;
    let statements =
        syntax::parse(&source).map_err(|e| format!("{}:{e}", source_path.display()))?;
    let target = root.join("target");
    fs::create_dir_all(&target).map_err(|e| e.to_string())?;
    let asm = target.join(format!("{name}.asm"));
    let obj = target.join(format!("{name}.obj"));
    let exe = target.join(format!("{name}.exe"));
    fs::write(&asm, syntax::assembly(&statements)).map_err(|e| e.to_string())?;
    let nasm = env::var_os("ADAMANTIUM_NASM").unwrap_or_else(|| {
        let installed = PathBuf::from(
            env::var_os("ProgramFiles").unwrap_or_else(|| "C:\\Program Files".into()),
        )
        .join("NASM/nasm.exe");
        if installed.is_file() {
            installed.into_os_string()
        } else {
            "nasm".into()
        }
    });
    execute(
        Command::new(nasm)
            .arg("-f")
            .arg("win64")
            .arg(&asm)
            .arg("-o")
            .arg(&obj),
        "NASM",
    )?;
    let linker = env::var_os("ADAMANTIUM_LINKER").unwrap_or_else(|| "link.exe".into());
    execute(
        Command::new(linker)
            .args([
                "/nologo",
                "/machine:x64",
                "/subsystem:console",
                "/entry:main",
                "/nodefaultlib",
                "/dynamicbase",
                "/nxcompat",
            ])
            .arg(format!("/out:{}", exe.display()))
            .arg(&obj)
            .arg("kernel32.lib"),
        "Microsoft linker (run from an x64 Native Tools Command Prompt for Visual Studio)",
    )?;
    println!("Built {}", exe.display());
    Ok(())
}

fn read_toml(path: &Path) -> Result<toml::Table, String> {
    fs::read_to_string(path)
        .map_err(|e| format!("{}: {e}", path.display()))?
        .parse()
        .map_err(|e| format!("{}: {e}", path.display()))
}

fn execute(command: &mut Command, label: &str) -> Result<(), String> {
    let status = command
        .status()
        .map_err(|e| format!("could not run {label}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{label} failed ({status})"))
    }
}
