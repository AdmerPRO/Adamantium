use crate::{Package, syntax, typed, types::Type};
use std::{collections::HashSet, fs, path::Path};

pub struct Binding {
    pub canonical: String,
    pub function: typed::PackageFunction,
}

pub fn load_bindings(
    root: &Path,
    requirements: &[Package],
    sources: &mut Vec<(String, String)>,
) -> Result<Vec<Binding>, String> {
    let mut requested = HashSet::new();
    for (_, source) in sources.iter() {
        requested.extend(syntax::package_dependencies(source)?);
    }
    let mut bindings = Vec::new();
    for module in requested {
        if sources.iter().any(|(name, _)| name == &module) {
            return Err(format!(
                "package module '{module}' conflicts with a project module"
            ));
        }
        let mut found = None;
        for package in requirements {
            let directory = root
                .join("packages")
                .join(&package.name)
                .join(&package.version);
            let manifest_path = directory.join("adamantium_packet.toml");
            if !manifest_path.is_file() {
                continue;
            }
            let manifest = parse_manifest(&manifest_path, &package.version)?;
            if manifest.name == module {
                if found.is_some() {
                    return Err(format!(
                        "package module '{module}' is provided more than once"
                    ));
                }
                found = Some((package, directory, manifest));
            }
        }
        let Some((package, directory, manifest)) = found else {
            return Err(format!(
                "package module '{module}' is not installed or declared; add it to requirement.toml and run 'adamantium install'"
            ));
        };
        let wasm = directory.join("adamantium_packet.wasm");
        let bytes = fs::read(&wasm).map_err(|error| {
            format!(
                "could not read installed package '{}': {error}",
                package.name
            )
        })?;
        if !bytes.starts_with(b"\0asm\x01\0\0\0") {
            return Err(format!(
                "installed package '{}' is not valid WASM",
                package.name
            ));
        }
        let mut declarations = String::new();
        for function in manifest.functions {
            let canonical = format!("admod__{}__{}", module.replace('/', "__"), function.name);
            declarations.push_str("pub fun ");
            declarations.push_str(&function.name);
            declarations.push('(');
            for (index, ty) in function.parameters.iter().enumerate() {
                if index != 0 {
                    declarations.push(',');
                }
                declarations.push_str(&format!("_p{index}:{ty}"));
            }
            declarations.push(')');
            if function.result != Type::None {
                declarations.push_str(&format!(
                    " result:{} {{ result = {}; }}\n",
                    function.result,
                    default_value(function.result)
                ));
            } else {
                declarations.push_str(" result:None {}\n");
            }
            bindings.push(Binding {
                canonical,
                function: typed::PackageFunction {
                    wasm_path: wasm.to_string_lossy().into_owned(),
                    command: function.command,
                    result: function.result,
                    filesystem: manifest.filesystem,
                },
            });
        }
        sources.push((module, declarations));
    }
    Ok(bindings)
}

struct Manifest {
    name: String,
    filesystem: u32,
    functions: Vec<ManifestFunction>,
}

struct ManifestFunction {
    name: String,
    command: String,
    parameters: Vec<Type>,
    result: Type,
}

pub fn validate_manifest(path: &Path, expected_version: &str) -> Result<(), String> {
    parse_manifest(path, expected_version).map(|_| ())
}

fn parse_manifest(path: &Path, expected_version: &str) -> Result<Manifest, String> {
    let text = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let value = text
        .parse::<toml::Table>()
        .map_err(|error| format!("{}: {error}", path.display()))?;
    let package = value
        .get("package")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| format!("{}: expected [package]", path.display()))?;
    let field = |name: &str| {
        package
            .get(name)
            .and_then(toml::Value::as_str)
            .ok_or_else(|| format!("{}: package.{name} must be a string", path.display()))
    };
    let name = field("name")?.to_owned();
    if !identifier(&name) {
        return Err(format!(
            "{}: invalid package module name '{name}'",
            path.display()
        ));
    }
    let version = field("version")?;
    if version != expected_version {
        return Err(format!(
            "{}: manifest version '{version}' does not match requirement '{expected_version}'",
            path.display()
        ));
    }
    if field("abi")? != "wasi-command-v1" {
        return Err(format!("{}: unsupported package ABI", path.display()));
    }
    let filesystem = match value
        .get("permissions")
        .and_then(toml::Value::as_table)
        .and_then(|table| table.get("filesystem"))
        .and_then(toml::Value::as_str)
        .unwrap_or("none")
    {
        "none" => 0,
        "read" => 1,
        "read-write" => 2,
        other => {
            return Err(format!(
                "{}: invalid filesystem permission '{other}'",
                path.display()
            ));
        }
    };
    let table = value
        .get("functions")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| format!("{}: expected [functions]", path.display()))?;
    let mut functions = Vec::new();
    for (name, entry) in table {
        if !identifier(name) {
            return Err(format!(
                "{}: invalid function name '{name}'",
                path.display()
            ));
        }
        let entry = entry
            .as_table()
            .ok_or_else(|| format!("{}: functions.{name} must be a table", path.display()))?;
        let parameters = entry
            .get("parameters")
            .and_then(toml::Value::as_array)
            .ok_or_else(|| {
                format!(
                    "{}: functions.{name}.parameters must be an array",
                    path.display()
                )
            })?
            .iter()
            .map(|value| manifest_type(path, value.as_str()))
            .collect::<Result<Vec<_>, _>>()?;
        if parameters.contains(&Type::None) {
            return Err(format!(
                "{}: functions.{name} cannot use None as a parameter",
                path.display()
            ));
        }
        if parameters.len() > 8 {
            return Err(format!(
                "{}: functions.{name} has more than 8 parameters",
                path.display()
            ));
        }
        let result = manifest_type(path, entry.get("result").and_then(toml::Value::as_str))?;
        let command = entry
            .get("command")
            .and_then(toml::Value::as_str)
            .unwrap_or(name)
            .to_owned();
        functions.push(ManifestFunction {
            name: name.clone(),
            command,
            parameters,
            result,
        });
    }
    functions.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(Manifest {
        name,
        filesystem,
        functions,
    })
}

fn manifest_type(path: &Path, name: Option<&str>) -> Result<Type, String> {
    let name = name.ok_or_else(|| format!("{}: package type must be a string", path.display()))?;
    let ty = Type::parse(name)
        .ok_or_else(|| format!("{}: unsupported package type '{name}'", path.display()))?;
    if matches!(ty, Type::F128) {
        return Err(format!(
            "{}: f128 is not supported by wasi-command-v1",
            path.display()
        ));
    }
    Ok(ty)
}

fn identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn default_value(ty: Type) -> &'static str {
    match ty {
        Type::String => "\"\"",
        Type::Bool => "false",
        Type::F32 => "0:f32",
        Type::F64 => "0:f64",
        Type::I8 => "0:i8",
        Type::I16 => "0:i16",
        Type::I32 => "0:i32",
        Type::I64 => "0:i64",
        Type::U4 => "0:u4",
        Type::U8 => "0:u8",
        Type::U16 => "0:u16",
        Type::U32 => "0:u32",
        Type::U64 => "0:u64",
        _ => "None",
    }
}
