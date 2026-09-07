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
  adamantium new <PROJECT_NAME_OR_PATH>\n\
  adamantium --help\n\
  adamantium --version\n\n\
PROJECT_DIRECTORY defaults to the current directory.\n\
For compatibility, `adamantium PROJECT_DIRECTORY` is the same as `adamantium build PROJECT_DIRECTORY`.\n\
Building requires NASM and Visual Studio C++ build tools.\n\
Override tools with ADAMANTIUM_NASM and ADAMANTIUM_LINKER.";

enum Action {
    Help,
    Version,
    Build(PathBuf),
    Run(PathBuf),
    New(PathBuf),
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
        Action::New(root) => {
            create_project(&root)?;
            println!("Created Adamantium project at {}", root.display());
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
    if first == "new" {
        let root = args
            .next()
            .ok_or("adamantium new requires a project name or path; use --help")?;
        no_more_args(args)?;
        return Ok(Action::New(root.into()));
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

fn valid_project_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err(
            "project name must contain only ASCII letters, digits, underscores or hyphens".into(),
        );
    }
    let upper = name.to_ascii_uppercase();
    if [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ]
    .contains(&upper.as_str())
    {
        return Err("project name is a reserved Windows device name".into());
    }
    Ok(())
}

fn create_project(root: &Path) -> Result<(), String> {
    let name = root
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("project path must end with a valid UTF-8 project name")?;
    valid_project_name(name)?;
    if root.exists() {
        return Err(format!("{} already exists", root.display()));
    }
    fs::create_dir_all(root.join("code"))
        .map_err(|e| format!("could not create {}: {e}", root.display()))?;
    fs::create_dir(root.join("target"))
        .map_err(|e| format!("could not create target directory: {e}"))?;
    fs::write(
        root.join("project.toml"),
        format!("name = \"{name}\"\nversion = \"0.1.0\"\ndescription = \"\"\nauthors = []\n"),
    )
    .map_err(|e| format!("could not create project.toml: {e}"))?;
    fs::write(root.join("requirement.toml"), "[packages]\n")
        .map_err(|e| format!("could not create requirement.toml: {e}"))?;
    fs::write(
        root.join("code/main.ad"),
        "fun main() {\n    print.newline(\"Hello, Adamantium!\");\n}\n",
    )
    .map_err(|e| format!("could not create code/main.ad: {e}"))?;
    fs::write(root.join(".gitignore"), "/target/\n")
        .map_err(|e| format!("could not create .gitignore: {e}"))?;
    Ok(())
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
    valid_project_name(name).map_err(|error| format!("project.toml: {error}"))?;
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
    link(&target, name, &obj, &runtime, &exe)?;
    Ok(exe)
}

fn link(target: &Path, name: &str, obj: &Path, runtime: &Path, exe: &Path) -> Result<(), String> {
    let mut arguments = vec![
        OsString::from("/nologo"),
        OsString::from("/machine:x64"),
        OsString::from("/subsystem:console"),
        OsString::from("/dynamicbase"),
        OsString::from("/nxcompat"),
        format!("/out:{}", exe.display()).into(),
        obj.as_os_str().into(),
        runtime.as_os_str().into(),
    ];
    arguments.extend(
        include_str!(concat!(env!("OUT_DIR"), "/runtime-libraries.txt"))
            .split_whitespace()
            .map(OsString::from),
    );
    arguments.push("kernel32.lib".into());

    if let Some(linker) = env::var_os("ADAMANTIUM_LINKER") {
        return execute(
            Command::new(linker).args(&arguments),
            "Microsoft linker configured by ADAMANTIUM_LINKER",
        );
    }
    if env::var_os("VSCMD_ARG_TGT_ARCH").is_some() {
        return execute(
            Command::new("link.exe").args(&arguments),
            "Microsoft linker",
        );
    }

    let vcvars = find_vcvars64().ok_or(
        "could not find Visual Studio C++ build tools; install the MSVC x64 tools or run from an x64 Native Tools Command Prompt",
    )?;
    let response = target.join(format!("{name}.link.rsp"));
    let response_text = arguments
        .iter()
        .map(|argument| format!("\"{}\"", argument.to_string_lossy().replace('"', "\\\"")))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&response, response_text).map_err(|e| format!("{}: {e}", response.display()))?;
    execute(
        Command::new("cmd.exe")
            .args(["/d", "/c", "call"])
            .arg(&vcvars)
            .args([">", "nul", "&&", "link.exe"])
            .arg(format!("@{}", response.display())),
        "Microsoft linker through the Visual Studio x64 environment",
    )
}

fn find_vcvars64() -> Option<PathBuf> {
    let program_files_x86 = env::var_os("ProgramFiles(x86)")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Program Files (x86)"));
    let vswhere = program_files_x86.join("Microsoft Visual Studio/Installer/vswhere.exe");
    if let Ok(output) = Command::new(vswhere)
        .args([
            "-latest",
            "-products",
            "*",
            "-requires",
            "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
            "-property",
            "installationPath",
        ])
        .output()
        && output.status.success()
        && let Ok(installation) = String::from_utf8(output.stdout)
    {
        let candidate = PathBuf::from(installation.trim()).join("VC/Auxiliary/Build/vcvars64.bat");
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    let program_files = env::var_os("ProgramFiles")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Program Files"));
    for year in ["2022", "2019"] {
        for edition in ["Community", "Professional", "Enterprise", "BuildTools"] {
            let candidate = program_files
                .join("Microsoft Visual Studio")
                .join(year)
                .join(edition)
                .join("VC/Auxiliary/Build/vcvars64.bat");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
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
