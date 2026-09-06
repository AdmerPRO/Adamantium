# Contributing to Adamantium

Contributions from beginners and experienced developers are welcome. You can
help by fixing bugs, improving diagnostics, writing documentation, or extending
the compiler.

## Set up your environment

Install Rust with edition 2024 support, NASM, and the Visual Studio C++ build
tools with the Windows SDK. See [README.md](README.md) for tool configuration.

Fork and clone the repository, create a branch for your change, and open an
**x64 Native Tools Command Prompt for VS 2022**. Run the following commands
from the `compiler` directory:

```bat
cargo build
cargo test
cargo run -- ../adamantium-project
..\adamantium-project\target\FirstProject.exe
```

## Report a bug

Search existing issues before opening a new one. Include:

- A short description of the problem and the expected behavior.
- The smallest `main.ad` example that reproduces it, plus relevant project settings.
- The exact command you ran and the complete error output.
- Your Windows version and Rust, NASM, and linker versions.

Remove private information from examples and logs before sharing them.

## Propose a feature

Explain the problem the feature solves and show an example of the intended
Adamantium syntax or compiler behavior. For substantial language changes,
opening an issue first helps contributors discuss the design before implementation.
Keep both beginners and professional users in mind when designing syntax and
error messages.

## Make changes

- Keep each pull request focused on one bug, feature, or documentation improvement.
- Follow the existing Rust style and use `cargo fmt` to format code.
- Write code comments, diagnostics, and documentation in English.
- Add meaningful regression tests for bug fixes and tests for new language behavior,
  including invalid input where relevant.
- Update documentation and examples when supported syntax or commands change.
- Avoid unrelated formatting changes and unnecessary dependencies.
- Keep generated build artifacts out of commits. Keep `Cargo.lock` tracked and
  update it when dependency changes require it.

`src/main.rs` handles project configuration and invokes NASM and the linker.
`src/syntax.rs` parses and validates source code; `src/syntax_tests.rs` tests it.
`src/typed.rs` checks types. `src/codegen.rs` generates assembly, and
`src/runtime.asm` provides the entry point. `runtime/` implements typed operations
and output, including software `f128` arithmetic.
The example project lives in `../adamantium-project`.

## Verify your work

For compiler changes, run:

```bat
cargo fmt --check
cargo fmt --manifest-path runtime/Cargo.toml --check
cargo test
cargo test --locked --test native -- --ignored
cargo clippy --all-targets -- -D warnings
cargo clippy --manifest-path runtime/Cargo.toml --all-targets -- -D warnings
cargo run -- ../adamantium-project
..\adamantium-project\target\FirstProject.exe
```

Check that the generated executable prints the expected output. For assembly
generation changes, also compile and run an example that exercises the changed
behavior. Only run the executable after compilation succeeds, because an older
EXE may remain after a failed build.

For documentation-only changes, check the wording, relative links, and command
paths; rebuilding the compiler is not necessary.

## Submit a pull request

Describe the problem, what your change does, and how you verified it. Link any
related issue and mention limitations or checks you could not run. Include a
small before-and-after example when it helps explain a behavior change.

Keep review discussions respectful and focused on the code. Questions and
requests for clarification are welcome.

## License

Review [LICENCE.md](LICENCE.md) before contributing. Submit only code and other
material that you have the right to contribute under the project's license.
