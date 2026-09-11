# Adamantium compilation targets

## Supported targets

| Target | Object format | Calling convention | Linker | Status |
| --- | --- | --- | --- | --- |
| `x86_64-pc-windows-msvc` | COFF (`win64`) | Windows x64 for runtime calls | MSVC `link.exe` | Supported |
| `x86_64-unknown-linux-gnu` | ELF64 | System V AMD64 for runtime calls | `cc` | Supported |

Adamantium function calls use the compiler's internal stack-based value ABI on
both targets. Platform entry points and calls into the Rust runtime use adapters
for the operating system ABI.

## Planned ARM64 targets

Future ARM64 work will use these target names:

* `aarch64-pc-windows-msvc`
* `aarch64-unknown-linux-gnu`
* `aarch64-apple-darwin`

The ARM64 backend must be implemented as a separate instruction encoder. It
must not translate generated x86-64 assembly text. Every `Value` remains 16
bytes, with two 64-bit words, and must keep the same runtime type identifiers.

Before an ARM64 target can be marked supported, it must provide:

* A platform entry point and command-line argument adapter.
* AAPCS64-compatible runtime calls and 16-byte stack alignment.
* COFF, ELF or Mach-O object generation for the selected operating system.
* A linker implementation and runtime static library for that target.
* Native tests for arithmetic, functions, classes, lists, errors and exit codes.
* CI that builds and runs generated programs on real ARM64 hardware or an
  explicitly documented emulator.

Target selection will eventually be exposed through a `--target <triple>` CLI
option. Until that option exists, the compiler builds programs for its host
operating system and x86-64 architecture.
