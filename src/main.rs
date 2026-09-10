mod codegen;
mod diagnostics;
mod syntax;
mod typed;
#[allow(dead_code)] // Shared with the separately linked native runtime.
#[path = "../runtime/src/types.rs"]
mod types;

use std::{
    collections::HashSet,
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
  adamantium check [PROJECT_DIRECTORY]\n\
  adamantium build [PROJECT_DIRECTORY]\n\
  adamantium run [PROJECT_DIRECTORY] [--name value ...]\n\
  adamantium test list [PROJECT_DIRECTORY]\n\
  adamantium test run [PROJECT_DIRECTORY] [TEST_NAME] [--verbose]\n\
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
    Check(PathBuf),
    TestList(PathBuf),
    TestRun(PathBuf, Option<String>, bool),
    Build(PathBuf),
    Run(PathBuf, Vec<OsString>),
    New(PathBuf),
}

type SourceFiles = Vec<(String, String)>;
type ProjectSources = (PathBuf, String, SourceFiles);

fn cli(args: Vec<OsString>) -> Result<ExitCode, String> {
    match action(args)? {
        Action::Help => println!("{HELP}"),
        Action::Version => println!("adamantium {}", env!("CARGO_PKG_VERSION")),
        Action::Check(root) => {
            check(&root)?;
            println!("Checked {}", root.display());
        }
        Action::TestList(root) => list_tests(&root)?,
        Action::TestRun(root, filter, verbose) => {
            return run_tests(&root, filter.as_deref(), verbose);
        }
        Action::Build(root) => {
            let executable = build(&root)?;
            println!("Built {}", executable.display());
        }
        Action::Run(root, arguments) => {
            let executable = build(&root)?;
            eprintln!("Built {}", executable.display());
            let status = Command::new(&executable)
                .current_dir(&root)
                .args(arguments)
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
    if first == "build" {
        let root = args.next().map_or_else(current_directory, Ok)?;
        no_more_args(args)?;
        return Ok(Action::Build(root.into()));
    }
    if first == "check" {
        let root = args.next().map_or_else(current_directory, Ok)?;
        no_more_args(args)?;
        return Ok(Action::Check(root.into()));
    }
    if first == "run" {
        let remaining = args.collect::<Vec<_>>();
        let (root, arguments) = if remaining
            .first()
            .is_some_and(|argument| !argument.to_string_lossy().starts_with('-'))
        {
            (PathBuf::from(&remaining[0]), remaining[1..].to_vec())
        } else {
            (PathBuf::from(current_directory()?), remaining)
        };
        return Ok(Action::Run(root, arguments));
    }
    if first == "test" {
        let command = args
            .next()
            .ok_or("adamantium test requires 'list' or 'run'; use --help")?;
        let remaining = args.collect::<Vec<_>>();
        if command == "list" {
            let root = remaining
                .first()
                .map_or_else(current_directory, |value| Ok(value.clone()))?;
            if remaining.len() > 1 {
                return Err("too many arguments for 'adamantium test list'; use --help".into());
            }
            return Ok(Action::TestList(root.into()));
        }
        if command == "run" {
            let verbose = args_contains_verbose(&remaining);
            let mut values = remaining.into_iter().filter(|value| value != "--verbose");
            let values = values.by_ref().collect::<Vec<_>>();
            let (root, filter) = match values.as_slice() {
                [] => (PathBuf::from(current_directory()?), None),
                [one] if Path::new(one).join("project.toml").is_file() => (one.into(), None),
                [one] => (
                    PathBuf::from(current_directory()?),
                    Some(one.to_string_lossy().into_owned()),
                ),
                [root, test] => (root.into(), Some(test.to_string_lossy().into_owned())),
                _ => return Err("too many arguments for 'adamantium test run'; use --help".into()),
            };
            return Ok(Action::TestRun(root, filter, verbose));
        }
        return Err(format!(
            "unknown test command '{}'; use --help",
            command.to_string_lossy()
        ));
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

fn args_contains_verbose(args: &[OsString]) -> bool {
    args.iter().any(|value| value == "--verbose")
}

#[derive(Clone)]
struct TestDefinition {
    name: String,
}

fn test_source(root: &Path) -> Result<(String, Vec<TestDefinition>), String> {
    let path = root.join("code/tests.ad");
    let source =
        fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut output = String::new();
    let mut tests = Vec::new();
    let mut awaiting_test = false;
    for (index, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        let mut output_line = line.to_string();
        if trimmed == "#[test]" {
            if awaiting_test {
                return Err(format!(
                    "{}:{}: duplicate #[test] attribute",
                    path.display(),
                    index + 1
                ));
            }
            awaiting_test = true;
            continue;
        }
        if awaiting_test && !trimmed.is_empty() && !trimmed.starts_with("//") {
            let rest = trimmed.strip_prefix("fun ").ok_or_else(|| {
                format!(
                    "{}:{}: #[test] must annotate a function",
                    path.display(),
                    index + 1
                )
            })?;
            let name = rest.split('(').next().unwrap_or_default().trim();
            if name.is_empty()
                || !name.bytes().enumerate().all(|(position, byte)| {
                    if position == 0 {
                        byte.is_ascii_alphabetic() || byte == b'_'
                    } else {
                        byte.is_ascii_alphanumeric() || byte == b'_'
                    }
                })
            {
                return Err(format!(
                    "{}:{}: invalid test function name",
                    path.display(),
                    index + 1
                ));
            }
            if !rest[name.len()..].trim_start().starts_with("()") {
                return Err(format!(
                    "{}:{}: test functions cannot have parameters",
                    path.display(),
                    index + 1
                ));
            }
            if tests.iter().any(|test: &TestDefinition| test.name == name) {
                return Err(format!(
                    "{}:{}: duplicate test '{name}'",
                    path.display(),
                    index + 1
                ));
            }
            tests.push(TestDefinition { name: name.into() });
            awaiting_test = false;
            output_line = line.replacen("()", "() result:None", 1);
        }
        output.push_str(&output_line);
        output.push('\n');
    }
    if awaiting_test {
        return Err(format!(
            "{}: #[test] must annotate a function",
            path.display()
        ));
    }
    if tests.is_empty() {
        return Err(format!("{}: no #[test] functions found", path.display()));
    }
    Ok((output, tests))
}

fn test_program(
    root: &Path,
    tests_source: &str,
    test: &TestDefinition,
) -> Result<(PathBuf, String, typed::Program), String> {
    let (root, project_name, mut sources) = project_sources(root)?;
    let main = sources
        .iter_mut()
        .find(|(module, _)| module.is_empty())
        .expect("main module is always loaded");
    main.1.push('\n');
    main.1.push_str(tests_source);
    main.1.push_str(&format!(
        "\nfun __adamantium_test_entry() result:None {{ {}(); }}\n",
        test.name
    ));
    let program = analyze_sources(&root, &sources)?;
    Ok((root, project_name, program))
}

fn list_tests(root: &Path) -> Result<(), String> {
    let canonical = root
        .canonicalize()
        .map_err(|error| format!("{}: {error}", root.display()))?;
    let (source, tests) = test_source(&canonical)?;
    for test in &tests {
        test_program(&canonical, &source, test)?;
        println!("{}", test.name);
    }
    Ok(())
}

fn run_tests(root: &Path, filter: Option<&str>, verbose: bool) -> Result<ExitCode, String> {
    let canonical = root
        .canonicalize()
        .map_err(|error| format!("{}: {error}", root.display()))?;
    let (source, all_tests) = test_source(&canonical)?;
    let tests = all_tests
        .into_iter()
        .filter(|test| filter.is_none_or(|filter| test.name == filter))
        .collect::<Vec<_>>();
    if tests.is_empty() {
        return Err(filter.map_or_else(
            || "no tests found".into(),
            |name| format!("test '{name}' was not found"),
        ));
    }
    let mut passed = 0;
    let mut failed = 0;
    println!(
        "running {} test{}",
        tests.len(),
        if tests.len() == 1 { "" } else { "s" }
    );
    for (index, test) in tests.iter().enumerate() {
        let (root, project_name, program) = test_program(&canonical, &source, test)?;
        let output_name = format!("{}_test_{index}", project_name);
        let executable = emit_executable(
            &root,
            &output_name,
            &program,
            "__adamantium_test_entry",
            &output_name,
        )?;
        let output = Command::new(executable)
            .current_dir(&root)
            .output()
            .map_err(|error| format!("could not run test '{}': {error}", test.name))?;
        if output.status.success() {
            passed += 1;
            println!("test {} ... ok", test.name);
        } else {
            failed += 1;
            println!("test {} ... FAILED", test.name);
        }
        if verbose || !output.status.success() {
            if !output.stdout.is_empty() {
                print!("{}", String::from_utf8_lossy(&output.stdout));
            }
            if !output.stderr.is_empty() {
                eprint!("{}", String::from_utf8_lossy(&output.stderr));
            }
        }
    }
    println!(
        "\ntest result: {}. {passed} passed; {failed} failed",
        if failed == 0 { "ok" } else { "FAILED" }
    );
    Ok(if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
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
    let (root, name, statements) = analyze(root)?;
    emit_executable(&root, &name, &statements, "main", &name)
}

fn emit_executable(
    root: &Path,
    project_name: &str,
    statements: &typed::Program,
    entry: &str,
    output_name: &str,
) -> Result<PathBuf, String> {
    let target = root.join("target");
    fs::create_dir_all(&target).map_err(|e| e.to_string())?;
    let asm = target.join(format!("{output_name}.asm"));
    let obj = target.join(format!("{output_name}.obj"));
    let exe = target.join(format!("{output_name}.exe"));
    let runtime = target.join("adamantium_runtime.lib");
    let runtime_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/runtime.lib"));
    if runtime_bytes.is_empty() {
        return Err(
            "building Adamantium executables requires the Windows x64 MSVC compiler build".into(),
        );
    }
    fs::write(&runtime, runtime_bytes).map_err(|e| e.to_string())?;
    fs::write(&asm, codegen::assembly_entry(statements, entry)).map_err(|e| e.to_string())?;
    let nasm = env::var_os("ADAMANTIUM_NASM").unwrap_or_else(|| {
        let bundled = env::current_exe().ok().and_then(|executable| {
            executable
                .parent()
                .map(|parent| parent.join("tools/nasm.exe"))
        });
        if let Some(bundled) = bundled
            && bundled.is_file()
        {
            return bundled.into_os_string();
        }
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
    link(&target, project_name, &obj, &runtime, &exe)?;
    Ok(exe)
}

fn check(root: &Path) -> Result<(), String> {
    analyze(root).map(|_| ())
}

fn analyze(root: &Path) -> Result<(PathBuf, String, typed::Program), String> {
    let (root, name, sources) = project_sources(root)?;
    let statements = analyze_sources(&root, &sources)?;
    Ok((root, name, statements))
}

fn project_sources(root: &Path) -> Result<ProjectSources, String> {
    let requested_root = root;
    let root = requested_root
        .canonicalize()
        .map_err(|e| format!("{}: {e}", requested_root.display()))?;
    let project = read_toml(&root.join("project.toml"))?;
    let name = project
        .get("name")
        .and_then(toml::Value::as_str)
        .ok_or("project.toml: name must be a string")?
        .to_string();
    valid_project_name(&name).map_err(|error| format!("project.toml: {error}"))?;
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
    let sources = load_modules(&root.join("code"))?;
    Ok((root, name, sources))
}

fn analyze_sources(root: &Path, sources: &[(String, String)]) -> Result<typed::Program, String> {
    let source_path = root.join("code/main.ad");
    let parsed =
        syntax::parse_modules(sources).map_err(|e| format!("{}:{e}", source_path.display()))?;
    let statements = typed::check(&parsed).map_err(|e| format!("{}:{e}", source_path.display()))?;
    for warning in diagnostics::warnings(&parsed) {
        eprintln!("{}:{warning}", source_path.display());
    }
    Ok(statements)
}

fn load_modules(code: &Path) -> Result<Vec<(String, String)>, String> {
    fn visit(
        module: &str,
        code: &Path,
        visiting: &mut Vec<String>,
        loaded: &mut HashSet<String>,
        result: &mut Vec<(String, String)>,
    ) -> Result<(), String> {
        if loaded.contains(module) {
            return Ok(());
        }
        if let Some(start) = visiting.iter().position(|item| item == module) {
            let mut cycle = visiting[start..].to_vec();
            cycle.push(module.to_string());
            return Err(format!("circular pack dependency: {}", cycle.join(" -> ")));
        }
        if !module.is_empty()
            && !module.split('/').all(|part| {
                let mut chars = part.chars();
                chars
                    .next()
                    .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                    && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
            })
        {
            return Err(format!("invalid module path '{module}'"));
        }
        let path = if module.is_empty() {
            code.join("main.ad")
        } else {
            code.join(format!("{module}.ad"))
        };
        let source = fs::read_to_string(&path).map_err(|error| {
            format!(
                "could not load module '{}': {}: {error}",
                if module.is_empty() { "main" } else { module },
                path.display()
            )
        })?;
        visiting.push(module.to_string());
        for dependency in syntax::module_dependencies(&source)
            .map_err(|error| format!("{}:{error}", path.display()))?
        {
            visit(&dependency, code, visiting, loaded, result)?;
        }
        visiting.pop();
        loaded.insert(module.to_string());
        result.push((module.to_string(), source));
        Ok(())
    }

    let mut result = Vec::new();
    visit("", code, &mut Vec::new(), &mut HashSet::new(), &mut result)?;
    Ok(result)
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
