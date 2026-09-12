# Creating Adamantium packages

Adamantium packages are WASI Preview 1 command modules published as GitHub Release assets. They may be hosted in any public GitHub repository. Packages owned by `AdmerPRO` or `AdamantiumORG` are official. The CLI displays a warning for other community packages.

## Release contract

Every release needs these assets:

```text
adamantium_packet.wasm
adamantium_packet.toml
```

Version `1.4.2` uses tag `adamantium_packet_1_4_2`. The WASM file must be a WebAssembly 1.0 `wasm32-wasip1` command module with a `_start` entry point.

## Manifest

```toml
[package]
name = "TextTools"
version = "1.4.2"
abi = "wasi-command-v1"

[permissions]
filesystem = "read-write"

[functions.read_text]
parameters = ["string"]
result = "string"

[functions.exists]
parameters = ["string"]
result = "bool"
command = "file-exists"
```

`package.name` is the identifier used in Adamantium code. It may differ from the repository name. The version must match `requirement.toml`. The current ABI is `wasi-command-v1`.

`permissions.filesystem` accepts `none`, `read`, or `read-write`. With `none`, the package cannot see the project directory. Both filesystem modes currently receive a preopened project directory from the WASI backend, so `read` is advisory for now. Use `none` when file access is unnecessary.

Each function has a required `parameters` array of at most eight types and a required `result`. `command` defaults to the function name. Supported types are:

```text
i8 i16 i32 i64
u4 u8 u16 u32 u64
f32 f64
string bool None
```

Use `None` only as a result. The ABI does not yet support `f128`, lists, classes, enums, offsets, optional values, or generic types.

## Command protocol

Adamantium starts the module once per function call and supplies:

```text
argv[0] = "adamantium-packet"
argv[1] = command
argv[2..] = function arguments
```

Numbers and booleans use text form. Strings are unchanged. Stdout is the return value: strings keep all bytes, while numeric and boolean results are trimmed and parsed. A `None` result ignores stdout. Write errors to stderr and exit nonzero on failure.

A Rust dispatcher can begin like this:

```rust
use std::{env, fs, process::ExitCode};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    match (args.next().as_deref(), args.next()) {
        (Some("read_text"), Some(path)) => match fs::read_to_string(path) {
            Ok(text) => { print!("{text}"); ExitCode::SUCCESS }
            Err(error) => { eprintln!("{error}"); ExitCode::FAILURE }
        },
        _ => { eprintln!("unknown command or invalid arguments"); ExitCode::FAILURE }
    }
}
```

Build it with:

```text
rustup target add wasm32-wasip1
cargo test
cargo build --release --target wasm32-wasip1
cp target/wasm32-wasip1/release/text_tools.wasm adamantium_packet.wasm
```

## Publishing

```text
git tag adamantium_packet_1_4_2
git push origin adamantium_packet_1_4_2
gh release create adamantium_packet_1_4_2 adamantium_packet.wasm adamantium_packet.toml --generate-notes --verify-tag
```

Example release workflow:

```yaml
name: Release Adamantium package
on:
  push:
    tags: ["adamantium_packet_*"]
permissions:
  contents: write
jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-wasip1
      - run: cargo test
      - run: cargo build --release --target wasm32-wasip1
      - run: cp target/wasm32-wasip1/release/text_tools.wasm adamantium_packet.wasm
      - env:
          GH_TOKEN: ${{ github.token }}
        run: gh release create "${{ github.ref_name }}" adamantium_packet.wasm adamantium_packet.toml --generate-notes --verify-tag
```

## Using a package

Add the exact GitHub release version:

```toml
[packages]
"https://github.com/community/TextTools" = "1.4.2"
```

Then install and import it:

```text
adamantium install
```

```adamantium
mod TextTools;
use TextTools:[read_text,exists];

fun main() {
    var text = read_text("notes.txt");
    print.newline(text);
}
```

Single imports such as `use TextTools:read_text;` also work. Without `use`, call `TextTools:read_text("notes.txt")`. Every source file that uses a package declares its own `mod` and `use` statements.

Installed files are stored under `packages/REPOSITORY/VERSION/`. `adamantium check`, `build`, and `run` validate the declaration, manifest, ABI, types, version, and WASM header.

## Troubleshooting

- `package module ... is not installed or declared` - check `requirement.toml`, run `adamantium install`, and compare the module with `package.name`.
- `manifest version ... does not match requirement` - publish a manifest matching the requested version.
- `unsupported package ABI` - use `wasi-command-v1`.
- `function ... is not declared` - add `[functions.NAME]` and import the same name.
- `Adamantium package error` - inspect the package's stderr output.

Dependency locking, checksums, signatures, transitive dependencies, and a central registry are still planned.

## Release checklist

- [ ] The module targets `wasm32-wasip1` and exports `_start`.
- [ ] The manifest describes every public function.
- [ ] Manifest and release versions match.
- [ ] The package requests minimal filesystem access.
- [ ] Package tests pass.
- [ ] The tag uses `adamantium_packet_MAJOR_MINOR_PATCH`.
- [ ] Both assets are attached to the release.
- [ ] A clean project can install, check, build, and run the package.
