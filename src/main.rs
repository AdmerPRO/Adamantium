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
            eprintln!("{}", diagnostics::render_errors(&error));
            ExitCode::FAILURE
        }
    }
}

const HELP: &str = "Adamantium compiler (Windows/Linux x86-64)\n\
Usage:\n\
  adamantium check [PROJECT_DIRECTORY]\n\
  adamantium install [PROJECT_DIRECTORY]\n\
  adamantium clean [PROJECT_DIRECTORY]\n\
  adamantium clear [PROJECT_DIRECTORY]\n\
  adamantium build [PROJECT_DIRECTORY]\n\
  adamantium run [PROJECT_DIRECTORY] [--name value ...]\n\
  adamantium test list [PROJECT_DIRECTORY]\n\
  adamantium test run [PROJECT_DIRECTORY] [TEST_NAME] [--verbose]\n\
  adamantium new <PROJECT_NAME_OR_PATH>\n\
  adamantium --help\n\
  adamantium --version\n\n\
PROJECT_DIRECTORY defaults to the current directory.\n\
For compatibility, `adamantium PROJECT_DIRECTORY` is the same as `adamantium build PROJECT_DIRECTORY`.\n\
Building requires NASM and a platform linker (MSVC on Windows or cc on Linux).\n\
Override tools with ADAMANTIUM_NASM and ADAMANTIUM_LINKER.";

enum Action {
    Help,
    Version,
    Check(PathBuf),
    Install(PathBuf),
    Clean(PathBuf),
    TestList(PathBuf),
    TestRun(PathBuf, Option<String>, bool),
    Build(PathBuf),
    Run(PathBuf, Vec<OsString>),
    New(PathBuf),
}

type SourceFiles = Vec<(String, String)>;
type ProjectSources = (PathBuf, String, SourceFiles);

#[derive(Debug, PartialEq)]
struct Package {
    name: String,
    source: String,
    version: String,
}

fn cli(args: Vec<OsString>) -> Result<ExitCode, String> {
    match action(args)? {
        Action::Help => println!("{HELP}"),
        Action::Version => println!("adamantium {}", env!("CARGO_PKG_VERSION")),
        Action::Check(root) => {
            check(&root)?;
            println!("Checked {}", root.display());
        }
        Action::Install(root) => install_packages(&root)?,
        Action::Clean(root) => clean_project(&root)?,
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
    if first == "install" {
        let root = args.next().map_or_else(current_directory, Ok)?;
        no_more_args(args)?;
        return Ok(Action::Install(root.into()));
    }
    if first == "clean" || first == "clear" {
        let root = args.next().map_or_else(current_directory, Ok)?;
        no_more_args(args)?;
        return Ok(Action::Clean(root.into()));
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
        let command = command.to_string_lossy();
        let help = closest_name(&command, &["list", "run"])
            .map(|name| format!(" Did you mean '{name}'?"))
            .unwrap_or_default();
        return Err(format!(
            "unknown test command '{command}'.{help} use --help."
        ));
    }
    if first.to_string_lossy().starts_with('-') {
        return Err(format!(
            "unknown option '{}'; use --help",
            first.to_string_lossy()
        ));
    }
    let text = first.to_string_lossy();
    if !Path::new(&first).exists()
        && let Some(command) = closest_name(
            &text,
            &[
                "build", "check", "clean", "clear", "install", "new", "run", "test",
            ],
        )
    {
        return Err(format!(
            "unknown command '{text}'. Did you mean '{command}'? use --help."
        ));
    }
    no_more_args(args)?;
    Ok(Action::Build(first.into()))
}

fn closest_name<'a>(input: &str, choices: &'a [&str]) -> Option<&'a str> {
    choices
        .iter()
        .map(|choice| (*choice, edit_distance(input, choice)))
        .filter(|(_, distance)| *distance <= 2)
        .min_by_key(|(_, distance)| *distance)
        .map(|(choice, _)| choice)
}

fn edit_distance(left: &str, right: &str) -> usize {
    let mut row = (0..=right.chars().count()).collect::<Vec<_>>();
    for (left_index, left_char) in left.chars().enumerate() {
        let mut previous = row[0];
        row[0] = left_index + 1;
        for (right_index, right_char) in right.chars().enumerate() {
            let replaced = previous + usize::from(left_char != right_char);
            previous = row[right_index + 1];
            row[right_index + 1] = (row[right_index + 1] + 1)
                .min(row[right_index] + 1)
                .min(replaced);
        }
    }
    row[right.chars().count()]
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
    fs::write(root.join(".gitignore"), "/target/\n/packages/\n")
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
    let obj = target.join(if cfg!(target_os = "linux") {
        format!("{output_name}.o")
    } else {
        format!("{output_name}.obj")
    });
    let exe = target.join(if cfg!(target_os = "linux") {
        output_name.to_string()
    } else {
        format!("{output_name}.exe")
    });
    let runtime = target.join("adamantium_runtime.lib");
    let runtime_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/runtime.lib"));
    if runtime_bytes.is_empty() {
        return Err(
            "building Adamantium executables is supported on Windows and Linux x86-64".into(),
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
            .arg(if cfg!(target_os = "linux") {
                "elf64"
            } else {
                "win64"
            })
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

fn clean_project(root: &Path) -> Result<(), String> {
    let requested_root = root;
    let root = requested_root
        .canonicalize()
        .map_err(|error| format!("{}: {error}", requested_root.display()))?;
    read_toml(&root.join("project.toml"))?;
    let target = root.join("target");
    if !target.exists() {
        println!("Project is already clean");
        return Ok(());
    }
    let metadata = fs::symlink_metadata(&target)
        .map_err(|error| format!("could not inspect {}: {error}", target.display()))?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "refusing to clean symbolic link {}",
            target.display()
        ));
    }
    if target.parent() != Some(root.as_path())
        || target.file_name().is_none_or(|name| name != "target")
    {
        return Err("refusing to clean a target outside the project root".into());
    }
    fs::remove_dir_all(&target)
        .map_err(|error| format!("could not clean {}: {error}", target.display()))?;
    println!("Cleaned {}", target.display());
    Ok(())
}

fn packages(table: &toml::Table) -> Result<Vec<Package>, String> {
    let Some(values) = table.get("packages").and_then(toml::Value::as_table) else {
        return Err("requirement.toml: expected a [packages] table".into());
    };
    if table.keys().any(|key| key != "packages") {
        return Err("requirement.toml: only the [packages] table is supported".into());
    }
    let mut result = Vec::new();
    for (source, value) in values {
        let version = value.as_str().ok_or_else(|| {
            format!("requirement.toml: package '{source}' version must be a string")
        })?;
        let path = source
            .strip_prefix("https://github.com/")
            .and_then(|path| path.split_once('/'))
            .filter(|(owner, name)| valid_github_owner(owner) && valid_github_repository(name))
            .ok_or_else(|| {
                format!(
                    "requirement.toml: package source '{source}' must be an https://github.com/<owner>/<name> URL"
                )
            })?;
        let name = path.1;
        let parts = version.split('.').collect::<Vec<_>>();
        if parts.len() != 3
            || parts
                .iter()
                .any(|part| part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()))
        {
            return Err(format!(
                "requirement.toml: package '{source}' version must use MAJOR.MINOR.PATCH"
            ));
        }
        result.push(Package {
            name: name.into(),
            source: source.into(),
            version: version.into(),
        });
    }
    result.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(result)
}

fn valid_github_owner(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

fn valid_github_repository(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn is_official_package_source(source: &str) -> bool {
    source
        .strip_prefix("https://github.com/")
        .and_then(|path| path.split_once('/'))
        .is_some_and(|(owner, _)| {
            owner.eq_ignore_ascii_case("AdmerPRO") || owner.eq_ignore_ascii_case("AdamantiumORG")
        })
}

fn community_package_warning(package: &Package) -> Option<String> {
    (!is_official_package_source(&package.source)).then(|| {
        format!(
            "warning: package '{}' is a community package and is not controlled by Adamantium",
            package.name
        )
    })
}

fn install_packages(root: &Path) -> Result<(), String> {
    let requested_root = root;
    let root = requested_root
        .canonicalize()
        .map_err(|error| format!("{}: {error}", requested_root.display()))?;
    let requirements = read_toml(&root.join("requirement.toml"))?;
    let packages = packages(&requirements)?;
    if packages.is_empty() {
        println!("No packages to install");
        return Ok(());
    }
    for package in &packages {
        if let Some(warning) = community_package_warning(package) {
            eprintln!("{warning}");
        }
        let directory = root
            .join("packages")
            .join(&package.name)
            .join(&package.version);
        fs::create_dir_all(&directory)
            .map_err(|error| format!("could not create {}: {error}", directory.display()))?;
        let destination = directory.join("adamantium_packet.wasm");
        let temporary = directory.join("adamantium_packet.wasm.download");
        let tag = format!("adamantium_packet_{}", package.version.replace('.', "_"));
        let url = format!(
            "{}/releases/download/{tag}/adamantium_packet.wasm",
            package.source
        );
        let downloader = env::var_os("ADAMANTIUM_CURL").unwrap_or_else(|| {
            if cfg!(windows) {
                "curl.exe".into()
            } else {
                "curl".into()
            }
        });
        let status = Command::new(downloader)
            .args([
                "-fL",
                "--proto",
                "=https",
                "--proto-redir",
                "=https",
                "--output",
            ])
            .arg(&temporary)
            .arg(&url)
            .status()
            .map_err(|error| format!("could not download package '{}': {error}", package.name))?;
        if !status.success() {
            let _ = fs::remove_file(&temporary);
            return Err(format!(
                "failed to download package '{}' version {} from {url}",
                package.name, package.version
            ));
        }
        let bytes = fs::read(&temporary)
            .map_err(|error| format!("could not read downloaded package: {error}"))?;
        if !bytes.starts_with(b"\0asm\x01\0\0\0") {
            let _ = fs::remove_file(&temporary);
            return Err(format!(
                "package '{}' did not contain a valid WebAssembly binary",
                package.name
            ));
        }
        if destination.exists() {
            fs::remove_file(&destination)
                .map_err(|error| format!("could not replace {}: {error}", destination.display()))?;
        }
        fs::rename(&temporary, &destination)
            .map_err(|error| format!("could not install {}: {error}", destination.display()))?;
        println!("Installed {} {}", package.name, package.version);
    }
    println!(
        "Installed {} package{}",
        packages.len(),
        if packages.len() == 1 { "" } else { "s" }
    );
    Ok(())
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
    let mut errors = Vec::new();
    let name = project
        .get("name")
        .and_then(toml::Value::as_str)
        .map(str::to_string);
    if let Some(name) = &name {
        if let Err(error) = valid_project_name(name) {
            errors.push(format!("project.toml: {error}"));
        }
    } else {
        errors.push("project.toml: name must be a string".into());
    }
    for field in ["version", "description"] {
        if project.get(field).and_then(toml::Value::as_str).is_none() {
            errors.push(format!("project.toml: {field} must be a string"));
        }
    }
    if !project
        .get("authors")
        .and_then(toml::Value::as_array)
        .is_some_and(|a| a.iter().all(toml::Value::is_str))
    {
        errors.push("project.toml: authors must be an array of strings".into());
    }
    let requirements = read_toml(&root.join("requirement.toml"))?;
    if let Err(error) = packages(&requirements) {
        errors.push(error);
    }
    if !errors.is_empty() {
        return Err(diagnostics::multiple_errors(errors));
    }
    let sources = load_modules(&root.join("code"))?;
    Ok((root, name.expect("validated project name"), sources))
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
    if cfg!(target_os = "linux") {
        let libraries = include_str!(concat!(env!("OUT_DIR"), "/runtime-libraries.txt"));
        let mut command =
            Command::new(env::var_os("ADAMANTIUM_LINKER").unwrap_or_else(|| "cc".into()));
        command.arg("-no-pie").arg(obj).arg(runtime);
        command
            .args(libraries.split_whitespace())
            .arg("-o")
            .arg(exe);
        return execute(&mut command, "Linux C linker");
    }
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

#[cfg(test)]
mod package_tests {
    use super::*;

    #[test]
    fn parses_github_wasm_packages_and_versions() {
        let table = r#"[packages]
"https://github.com/AdmerPRO/Math" = "1.2.3"
"https://github.com/community/text-tools" = "0.4.0"
"#
        .parse::<toml::Table>()
        .unwrap();
        assert_eq!(
            packages(&table).unwrap(),
            [
                Package {
                    name: "Math".into(),
                    source: "https://github.com/AdmerPRO/Math".into(),
                    version: "1.2.3".into(),
                },
                Package {
                    name: "text-tools".into(),
                    source: "https://github.com/community/text-tools".into(),
                    version: "0.4.0".into(),
                },
            ]
        );
    }

    #[test]
    fn identifies_official_package_owners() {
        assert!(is_official_package_source(
            "https://github.com/AdmerPRO/Math"
        ));
        assert!(is_official_package_source(
            "https://github.com/AdamantiumORG/Math"
        ));
        assert!(!is_official_package_source(
            "https://github.com/community/Math"
        ));
    }

    #[test]
    fn warns_about_community_packages() {
        let community = Package {
            name: "some.package".into(),
            source: "https://github.com/community/some.package".into(),
            version: "1.0.0".into(),
        };
        assert_eq!(
            community_package_warning(&community).as_deref(),
            Some(
                "warning: package 'some.package' is a community package and is not controlled by Adamantium"
            )
        );

        let official = Package {
            name: "Math".into(),
            source: "https://github.com/AdamantiumORG/Math".into(),
            version: "1.0.0".into(),
        };
        assert_eq!(community_package_warning(&official), None);
    }

    #[test]
    fn rejects_invalid_package_sources_and_versions() {
        for manifest in [
            "[packages]\n\"https://gitlab.com/Other/Name\"=\"1.0.0\"",
            "[packages]\n\"https://github.com/Owner/Name/extra\"=\"1.0.0\"",
            "[packages]\n\"https://github.com/AdmerPRO/../Name\"=\"1.0.0\"",
            "[packages]\n\"https://github.com/AdmerPRO/Name\"=\"latest\"",
        ] {
            assert!(packages(&manifest.parse().unwrap()).is_err(), "{manifest}");
        }
    }
}
