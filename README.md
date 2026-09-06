# Adamantium Compiler

A Rust compiler that translates Adamantium source code into NASM assembly
and builds a native Windows x64 console executable.

## Requirements

- Rust and Cargo with edition 2024 support
- NASM on PATH or installed in `%ProgramFiles%\NASM`
- Visual Studio C++ build tools and the Windows SDK

## Usage

Open **x64 Native Tools Command Prompt for VS 2022** and change into this
`compiler` directory. Compile and run the included example:

```bat
cargo run -- ../adamantium-project
..\adamantium-project\target\FirstProject.exe
```

Running `cargo run` without a project argument also builds the included
example. To compile a different project or show help:

```bat
cargo run -- "C:\path\to\project"
cargo run -- --help
```

The compiler reads `code/main.ad`, `project.toml` and `requirement.toml`
from the project directory. It writes `<name>.asm`, `<name>.obj` and
`<name>.exe` into that project's `target` directory, where `name` comes
from `project.toml`.

Set `ADAMANTIUM_NASM` or `ADAMANTIUM_LINKER` to override a tool's executable
path. The linker must support Microsoft LINK arguments and have access to
`kernel32.lib` through the Visual Studio developer environment.

## Supported syntax

```text
fun main() {
    // Print without a line ending, then finish the line.
    print.sameline("Hello, ");
    print.newline("Adamantium!");
}
```

The language currently supports one parameterless `main` function,
single-line comments, and string literals with UTF-8 text and escapes.
`print.newline` appends CRLF; `print.sameline` adds no line ending.
Variables, expressions, additional functions and packages are not supported yet.

See the [project README](../README.md) for project configuration and string
escape details.

## Development

Run these commands from this directory:

```bat
cargo build --release
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

The release compiler is generated at `target\release\adamantium-compiler.exe`.
`src/main.rs` handles configuration and external tools; `src/syntax.rs`
handles parsing and assembly generation.

## Continuous integration

GitHub Actions runs on every push and pull request using Ubuntu, macOS, and
Windows runners. Each runner checks formatting and compilation, checks spelling
with Typos, runs Clippy with warnings treated as errors, runs the tests, and
checks the compiler's `--help` command. The final step is `cargo build --locked
--release`.

Clippy provides Rust linting; Flake8 is intended for Python and is not used here.
These checks build and test the compiler on all three systems. Generating and
running Adamantium programs still requires the Windows x64 toolchain described
above; the CLI smoke test does not invoke NASM or the Windows linker.
