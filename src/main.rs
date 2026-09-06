mod codegen;
mod diagnostics;
mod syntax;
mod typed;
#[allow(dead_code)] // Shared with the separately linked native runtime.
#[path = "../runtime/src/types.rs"]
mod types;

use std::{
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

fn main() -> ExitCode {
    match cli(env::args_os().skip(1).collect()) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

const HELP: &str = "Adamantium compiler (Windows x64)\n\
Usage:\n\
  adamantium build [PROJECT_DIRECTORY]\n\
  adamantium run [PROJECT_DIRECTORY]\n\
  adamantium --help\n\
  adamantium --version\n\n\
PROJECT_DIRECTORY defaults to the current directory.\n\
For compatibility, `adamantium PROJECT_DIRECTORY` is the same as `adamantium build PROJECT_DIRECTORY`.\n\
Building requires NASM and the Visual Studio x64 Native Tools environment.\n\
Override tools with ADAMANTIUM_NASM and ADAMANTIUM_LINKER.";

enum Action {
    Help,
    Version,
    Build(PathBuf),
    Run(PathBuf),
}

fn cli(args: Vec<OsString>) -> Result<ExitCode, String> {
    match action(args)? {
        Action::Help => println!("{HELP}"),
        Action::Version => println!("adamantium {}", env!("CARGO_PKG_VERSION")),
        Action::Build(root) => {
            let executable = build(&root)?;
            println!("Built {}", executable.display());
        }
        Action::Run(root) => {
            let executable = build(&root)?;
            eprintln!("Built {}", executable.display());
            let status = Command::new(&executable)
                .current_dir(&root)
                .status()
                .map_err(|e| format!("could not run {}: {e}", executable.display()))?;
            return Ok(status
                .code()
                .and_then(|code| u8::try_from(code).ok())
                .map_or(ExitCode::FAILURE, ExitCode::from));
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn action(args: Vec<OsString>) -> Result<Action, String> {
    let mut args = args.into_iter();
    let Some(first) = args.next() else {
        return Ok(Action::Help);
    };
    if first == "--help" || first == "-h" {
        no_more_args(args)?;
        return Ok(Action::Help);
    }
    if first == "--version" || first == "-V" {
        no_more_args(args)?;
        return Ok(Action::Version);
    }
    if first == "build" || first == "run" {
        let root = args.next().map_or_else(current_directory, Ok)?;
        no_more_args(args)?;
        return if first == "build" {
            Ok(Action::Build(root.into()))
        } else {
            Ok(Action::Run(root.into()))
        };
    }
    if first.to_string_lossy().starts_with('-') {
        return Err(format!(
            "unknown option '{}'; use --help",
            first.to_string_lossy()
        ));
    }
    no_more_args(args)?;
    Ok(Action::Build(first.into()))
}

fn current_directory() -> Result<OsString, String> {
    env::current_dir()
        .map(Into::into)
        .map_err(|e| format!("could not read the current directory: {e}"))
}

fn no_more_args(mut args: impl Iterator<Item = OsString>) -> Result<(), String> {
    if args.next().is_some() {
        Err("too many arguments; use --help".into())
    } else {
        Ok(())
    }
}

fn build(root: &Path) -> Result<PathBuf, String> {
    let requested_root = root;
    let root = requested_root
        .canonicalize()
        .map_err(|e| format!("{}: {e}", requested_root.display()))?;
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
    let parsed = syntax::parse(&source).map_err(|e| format!("{}:{e}", source_path.display()))?;
    let statements = typed::check(&parsed).map_err(|e| format!("{}:{e}", source_path.display()))?;
    for warning in diagnostics::warnings(&parsed) {
        eprintln!("{}:{warning}", source_path.display());
    }
    let target = root.join("target");
    fs::create_dir_all(&target).map_err(|e| e.to_string())?;
    let asm = target.join(format!("{name}.asm"));
    let obj = target.join(format!("{name}.obj"));
    let exe = target.join(format!("{name}.exe"));
    let runtime = target.join("adamantium_runtime.lib");
    let runtime_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/runtime.lib"));
    if runtime_bytes.is_empty() {
        return Err(
            "building Adamantium executables requires the Windows x64 MSVC compiler build".into(),
        );
    }
    fs::write(&runtime, runtime_bytes).map_err(|e| e.to_string())?;
    fs::write(&asm, codegen::assembly(&statements)).map_err(|e| e.to_string())?;
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
                "/dynamicbase",
                "/nxcompat",
            ])
            .arg(format!("/out:{}", exe.display()))
            .arg(&obj)
            .arg(&runtime)
            .args(
                include_str!(concat!(env!("OUT_DIR"), "/runtime-libraries.txt")).split_whitespace(),
            )
            .arg("kernel32.lib"),
        "Microsoft linker (run from an x64 Native Tools Command Prompt for Visual Studio)",
    )?;
    Ok(exe)
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
