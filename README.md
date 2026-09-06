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
    var a = 10;
    var b = 20;
    b =+ a;
    b.clamp(0,100);
    var result = add(a,b);
    print.newline(result);
}

fun add(a:int,b:int) r:int {
    r = a+b;
}
```

The program requires exactly one parameterless `main` function and can declare
additional functions with `int` parameters and a named `int` result.
Single-line comments and string literals with UTF-8 text and escapes are supported.
`print.newline` appends CRLF; `print.sameline` adds no line ending.
Declare integer variables with `var a = 10;` and print them with
`print.newline(a);` or `print.sameline(a);`. Values are signed 64-bit decimal
integers, including negative numbers. Names are case-sensitive, start with an
ASCII letter or underscore, and may then contain digits. Declare each name once,
before using it; `fun`, `var`, `print`, `return`, and `int` are reserved words.

### Changeable and static variables

```text
var a = 10;         // Changeable by default.
var ch b = 10;      // Explicitly changeable (ch = changeable).
var static c = 10;  // Cannot be modified after initialization.
var stc d = 10;     // Short form of static.
a = 20;
b =+ 5;
print.newline(c);
```

`static` and `stc` prevent reassignment, compound assignment, and `clamp` on that
variable. Its initializer can be an expression or a function call. Here `static`
means immutable, not shared storage: the variable remains local to each function
call. Passing its value to a function does not make the parameter immutable.
`static`, `stc`, and `ch` are reserved words, and a declaration accepts at most
one of these modifiers.

### Arithmetic and assignment

Initializers, assignments, print arguments, and function arguments accept integer
expressions. `*` and `/` take precedence over `+` and `-`; operators at the same
precedence are evaluated left to right. Parentheses override precedence.
Division truncates toward zero, so `-7/2` is `-3`.

```text
var a = 10;
var b = 20;
b =+ a;        // b = b + a
b =- a;        // b = b - a
b =* a;        // b = b * a
b =/ a;        // b = b / a
b = a + b;
b = (a + 2) * 3;
b = -5;        // Assign a negative value (space after =).
b = (-5);      // Parentheses also distinguish this from =-.
```

Compound operators are written together: `b =- 5;` subtracts 5, whereas
`b = -5;` assigns -5. Their right-hand side can be a full expression.
All calculations and function calls execute in the generated EXE.

### Clamp

```text
var a = 150;
a.clamp(0,100);
print.newline(a); // 100
a = 150;
print.newline(a); // 150: clamp does not restrict future assignments.
```

`clamp` changes the current value once, using inclusive lower and upper bounds.
Bounds can be integer expressions and are evaluated left to right.

### Functions and named results

```text
fun add(a:int,b:int) r:int {
    r = a+b;
    return r; // Optional: exit immediately with the named result.
}
```

The result variable is declared by the function signature. Assign it before
returning; do not redeclare it with `var`. Reaching the closing brace implicitly
returns that variable. `return r;` exits early and always refers to the result
name declared in the signature; arbitrary return expressions are not supported.

Functions can be declared before or after `main`, called from other functions,
and nested inside expressions. Arguments are evaluated left to right and passed
by value. Each call has its own local variables; changing a parameter does not
change the caller's variable. Only `int` parameters and results are supported.
`main` cannot be called by another function. Loops, conditionals, imports, and
packages are not implemented yet.

Division by zero, signed arithmetic overflow, and reversed clamp bounds stop the
EXE with a diagnostic on stderr and exit code 2. Output failures use exit code 1.

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

From the Windows x64 developer environment with NASM installed, also run the
native regression tests. They compile temporary projects and verify EXE output
and runtime failures:

```bat
cargo test --locked --test native -- --ignored
```

The release compiler is generated at `target\release\adamantium-compiler.exe`.
`src/main.rs` handles configuration and external tools; `src/syntax.rs`
handles parsing and semantic checks. `src/codegen.rs` generates assembly and
`src/runtime.asm` implements output and runtime errors. Parser tests live in
`src/syntax_tests.rs`, and native regression tests live in `tests/native.rs`.

## Continuous integration

GitHub Actions runs on every push and pull request using Ubuntu, macOS, and
Windows runners. Each runner checks formatting and compilation, checks spelling
with Typos, runs Clippy with warnings treated as errors, runs the tests, and
checks the compiler's `--help` command. The final step is `cargo build --locked
--release`.

Clippy provides Rust linting; Flake8 is intended for Python and is not used here.
These checks build and test the compiler on all three systems. Generating and
running Adamantium programs still requires the Windows x64 toolchain described
above. Windows CI additionally installs NASM, configures MSVC, and runs the native
EXE regression tests before the final release build.
