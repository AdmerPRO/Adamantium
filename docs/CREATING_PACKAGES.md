# Creating Adamantium packages

Adamantium packages are distributed as WebAssembly modules through GitHub
Releases. This guide describes the package format currently accepted by the
Adamantium CLI, how to build a package, how to publish a compatible release,
and how users install it.

## Current package support

The current CLI can:

- read package declarations from `requirement.toml`;
- resolve an exact package version to a GitHub Release;
- download `adamantium_packet.wasm` over HTTPS;
- verify that the downloaded file starts with the WebAssembly 1.0 header;
- install it in the project's `packages` directory.

The compiler cannot yet import or execute functions exported by an installed
WASM package. The package ABI, Adamantium bindings, dependency locking,
transitive dependencies, checksums, signing, and a package registry are still
planned. A package created today can be published and installed, but its
exports will become usable only after the package runtime and ABI are added.

## Required repository and release format

A package repository must currently use this URL form:

```text
https://github.com/Owner/PackageName
```

The owner and repository names:

- must use GitHub-compatible ASCII names;
- allow letters, digits, and `-` in the owner name;
- also allow `_` and `.` in the repository name;
- must form exactly two path components after `github.com`.

Packages should be public because the installer currently sends no GitHub
authentication token.

Packages owned by `AdmerPRO` or `AdamantiumORG` are treated as official
packages. A package from any other GitHub owner can still be installed, but the
CLI prints a warning that it is a community package and is not controlled by
Adamantium. This warning describes the package's origin and does not block the
installation.

Every published version needs:

1. A version written as `MAJOR.MINOR.PATCH`, for example `1.4.2`.
2. A Git tag named `adamantium_packet_MAJOR_MINOR_PATCH`.
3. A GitHub Release created for that tag.
4. One release asset named exactly `adamantium_packet.wasm`.

For version `1.4.2`, the resulting download URL is:

```text
https://github.com/Owner/PackageName/releases/download/adamantium_packet_1_4_2/adamantium_packet.wasm
```

The names are case-sensitive. Do not publish the asset under the Cargo crate
name or append the version to its filename.

## Recommended repository layout

The package source can use any language that produces a valid WebAssembly 1.0
module. A small Rust package can use this layout:

```text
PackageName/
|-- .github/
|   `-- workflows/
|       `-- release.yml
|-- src/
|   `-- lib.rs
|-- Cargo.toml
|-- README.md
`-- LICENSE
```

Keep generated files such as `target/` out of Git. The release asset is the
distribution artifact and does not need to be committed to the repository.

## Creating a WASM module with Rust

Install Rust and its WebAssembly target:

```text
rustup target add wasm32-unknown-unknown
```

Create a library project:

```text
cargo new --lib PackageName
cd PackageName
```

Configure the library as a C-compatible dynamic library in `Cargo.toml`:

```toml
[package]
name = "adamantium-package-name"
version = "1.0.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]
```

Add an exported function to `src/lib.rs`:

```rust
#[unsafe(no_mangle)]
pub extern "C" fn add(left: i32, right: i32) -> i32 {
    left + right
}
```

This export is only an example for producing a valid module. Adamantium does
not yet define the final names, parameter encoding, memory layout, or error ABI
for calling package functions.

Build the release module:

```text
cargo build --release --target wasm32-unknown-unknown
```

The generated module is normally located at:

```text
target/wasm32-unknown-unknown/release/adamantium_package_name.wasm
```

Cargo replaces hyphens with underscores in the output filename. Copy or rename
the module before publishing:

### Linux or macOS

```sh
cp target/wasm32-unknown-unknown/release/adamantium_package_name.wasm adamantium_packet.wasm
```

### Windows PowerShell

```powershell
Copy-Item target/wasm32-unknown-unknown/release/adamantium_package_name.wasm adamantium_packet.wasm
```

## Validating the artifact

At minimum, run the package's tests and build the exact release artifact:

```text
cargo test
cargo build --release --target wasm32-unknown-unknown
```

The Adamantium installer checks for the eight-byte WebAssembly 1.0 header:

```text
00 61 73 6d 01 00 00 00
```

This check proves that the file begins like a WASM 1.0 binary. It does not fully
validate the module, its exports, behavior, or security. Package authors should
also validate the module with a WebAssembly tool such as `wasm-tools validate`
when available.

## Publishing manually

For version `1.0.0`, build and rename the artifact, then commit the package
source. Create and push the required tag:

```text
git tag adamantium_packet_1_0_0
git push origin adamantium_packet_1_0_0
```

Create a GitHub Release for `adamantium_packet_1_0_0` and upload the file named
`adamantium_packet.wasm`. With GitHub CLI installed and authenticated, this can
be done with:

```text
gh release create adamantium_packet_1_0_0 adamantium_packet.wasm --title "Adamantium package 1.0.0" --notes "Adamantium package release 1.0.0"
```

Before announcing the release, open its asset list and confirm that the asset
name is exactly `adamantium_packet.wasm`.

## Publishing with GitHub Actions

The following `.github/workflows/release.yml` builds and publishes a package
whenever a matching tag is pushed:

```yaml
name: Release Adamantium package

on:
  push:
    tags:
      - "adamantium_packet_*"

permissions:
  contents: write

jobs:
  release:
    runs-on: ubuntu-latest

    steps:
      - name: Check out repository
        uses: actions/checkout@v6
        with:
          persist-credentials: false

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown

      - name: Test package
        run: cargo test --locked

      - name: Build WASM package
        run: cargo build --locked --release --target wasm32-unknown-unknown

      - name: Prepare release asset
        run: cp target/wasm32-unknown-unknown/release/adamantium_package_name.wasm adamantium_packet.wasm

      - name: Create GitHub Release
        env:
          GH_TOKEN: ${{ github.token }}
        run: gh release create "${{ github.ref_name }}" adamantium_packet.wasm --generate-notes --verify-tag
```

Replace `adamantium_package_name.wasm` with the filename produced by your own
Cargo package. Keep `adamantium_packet.wasm` unchanged.

Publish version `1.0.0` with:

```text
git tag adamantium_packet_1_0_0
git push origin adamantium_packet_1_0_0
```

The workflow requires the tag to exist because it uses `--verify-tag`.

## Declaring the package in an Adamantium project

Add the repository URL and exact version to the project's `requirement.toml`:

```toml
[packages]
"https://github.com/Owner/PackageName" = "1.0.0"
```

Multiple packages belong in the same table:

```toml
[packages]
"https://github.com/AdmerPRO/Math" = "1.0.0"
"https://github.com/community/TextTools" = "2.3.1"
```

Installing this example does not warn about `AdmerPRO/Math`. It warns that
`community/TextTools` is a community package and is not controlled by
Adamantium.

Only the `[packages]` top-level table is currently supported. Versions must be
quoted strings with exactly three numeric components. Version ranges, names
such as `latest`, prerelease suffixes, Git branches, and commit hashes are not
accepted.

## Installing packages

From the Adamantium project directory, run:

```text
adamantium install
```

You can also provide the project path:

```text
adamantium install path/to/project
```

For `PackageName` version `1.0.0`, the CLI installs the module at:

```text
packages/PackageName/1.0.0/adamantium_packet.wasm
```

The installer downloads to `adamantium_packet.wasm.download`, validates the
header, and then renames the file. A failed download or invalid header removes
the temporary file. Installing the same version again downloads it again and
replaces the existing module. There is currently no lock file or shared cache.

Projects created by `adamantium new` already ignore `/packages/` in Git. If the
project was created manually, add this entry to `.gitignore`:

```gitignore
/packages/
```

## Testing a published package

Create a clean Adamantium project and add the package declaration:

```text
adamantium new PackageConsumer
cd PackageConsumer
adamantium install
```

Confirm that the CLI reports the installed package and that this file exists:

```text
packages/PackageName/1.0.0/adamantium_packet.wasm
```

At present this tests release resolution, downloading, and basic WASM header
validation. It cannot test calls from Adamantium code until package imports are
implemented.

## Updating a package

For every new version:

1. Update and test the package source.
2. Update the source project's version if it has one.
3. Build a new release WASM module.
4. Rename it to `adamantium_packet.wasm`.
5. Create a new tag such as `adamantium_packet_1_1_0`.
6. Create the GitHub Release and upload the asset.
7. Update consumers from `"1.0.0"` to `"1.1.0"`.
8. Run `adamantium install` again in each consumer project.

Do not replace an existing release asset with incompatible content. Publish a
new version so projects continue to resolve reproducibly once locking is added.

## Troubleshooting

### Package source is rejected

Use an exact HTTPS GitHub repository URL:

```text
https://github.com/Owner/PackageName
```

Remove trailing slashes, `.git`, subdirectories, query strings, and fragments.

### Version is rejected

Use a quoted `MAJOR.MINOR.PATCH` value containing digits only:

```toml
"https://github.com/Owner/PackageName" = "1.0.0"
```

### Download returns HTTP 404

Check all three release identifiers:

- repository: `Owner/PackageName`;
- tag for `1.0.0`: `adamantium_packet_1_0_0`;
- asset: `adamantium_packet.wasm`.

Also confirm that the GitHub Release is published and the repository is public.

### The file is reported as invalid WebAssembly

Confirm that the uploaded asset is the compiled `.wasm` binary rather than a
ZIP archive, HTML download page, Git LFS pointer, or source file. Rebuild it for
`wasm32-unknown-unknown` and upload it again under the required asset name.

### The downloader cannot run

Adamantium uses `curl.exe` on Windows and `curl` on other systems. Ensure it is
available on `PATH`. `ADAMANTIUM_CURL` may point to a compatible executable, but
that executable must accept the curl options used by the installer, including
`-fL`, HTTPS protocol restrictions, and `--output`.

### The package installs but cannot be imported

This is the current expected limitation. Installation is implemented, while
WASM package imports, bindings, and execution are still pending.

## Release checklist

- [ ] Package tests pass.
- [ ] The release build targets `wasm32-unknown-unknown`.
- [ ] The output is a valid WebAssembly 1.0 module.
- [ ] The artifact is named `adamantium_packet.wasm`.
- [ ] The version uses `MAJOR.MINOR.PATCH`.
- [ ] The tag uses `adamantium_packet_MAJOR_MINOR_PATCH`.
- [ ] A GitHub Release exists for the exact tag.
- [ ] The release contains the exact asset name.
- [ ] A clean Adamantium project can run `adamantium install` successfully.
- [ ] Consumers use the intended exact version in `requirement.toml`.
