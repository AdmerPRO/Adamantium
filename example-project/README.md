# Adamantium Feature Example

This project demonstrates every currently supported Adamantium language feature.
This includes comparisons, `if`/`else`, `for`, `while`, `until`, `loop`,
`break`, and `continue`.
It also demonstrates multiple source files through `pack`, qualified module
calls, nested module paths, and selective `use` imports.
Run it from the compiler repository on Windows with NASM and the Visual Studio
C++ build tools installed:

```powershell
cargo run -- run example-project
```

After installing the CLI with `cargo install --path . --force`, this also works:

```powershell
adamantium run example-project
```

The generated NASM source, object file, runtime library, linker response file,
and executable are written to `example-project/target`.
