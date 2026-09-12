pub mod types;

use std::cell::{Cell, RefCell};
use std::ffi::{CStr, c_char};
use std::io::Write;
use types::{Type, Value};
use wasmi::{Engine, Linker, Module, Store};
use wasmi_wasi::{
    WasiCtx, WasiCtxBuilder, add_to_linker, ambient_authority, wasi_common::pipe::WritePipe,
};

thread_local! {
    static TRY_DEPTH: Cell<usize> = const { Cell::new(0) };
    static LAST_ERROR: RefCell<Option<String>> = const { RefCell::new(None) };
}

fn report_error(message: String) {
    let handled = TRY_DEPTH.get() != 0;
    if handled {
        LAST_ERROR.with_borrow_mut(|error| *error = Some(message));
    } else {
        eprintln!("{message}");
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ad_try_begin() {
    if TRY_DEPTH.get() == 0 {
        LAST_ERROR.with_borrow_mut(|error| *error = None);
    }
    TRY_DEPTH.set(TRY_DEPTH.get() + 1);
}

/// # Safety
/// `output` must point to writable memory for one `Value`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ad_try_end(output: *mut Value) {
    TRY_DEPTH.set(TRY_DEPTH.get().saturating_sub(1));
    let Some(output) = (unsafe { output.as_mut() }) else {
        return;
    };
    let Some(error) = LAST_ERROR.with_borrow_mut(Option::take) else {
        *output = Value::default();
        return;
    };
    let bytes = error.into_bytes().into_boxed_slice();
    let value = Value {
        lo: bytes.as_ptr() as u64,
        hi: bytes.len() as u64,
    };
    std::mem::forget(bytes);
    *output = Value {
        lo: Box::into_raw(Box::new(value)) as u64,
        hi: 2,
    };
}

#[unsafe(no_mangle)]
pub extern "C" fn ad_has_error() -> u32 {
    u32::from(LAST_ERROR.with_borrow(Option::is_some))
}

#[unsafe(no_mangle)]
pub extern "C" fn ad_is_trying() -> u32 {
    u32::from(TRY_DEPTH.get() != 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn ad_object_new(field_count: usize) -> *mut Value {
    let fields = vec![Value::default(); field_count].into_boxed_slice();
    Box::into_raw(fields) as *mut Value
}

/// # Safety
/// `source` must point to at least `field_count` initialized values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ad_object_clone(source: *const Value, field_count: usize) -> *mut Value {
    if source.is_null() {
        return std::ptr::null_mut();
    }
    let fields = unsafe { std::slice::from_raw_parts(source, field_count) };
    Box::into_raw(fields.to_vec().into_boxed_slice()) as *mut Value
}

#[unsafe(no_mangle)]
pub extern "C" fn ad_list_error(index: usize, length: usize) -> u32 {
    report_error(format!(
        "Adamantium runtime error: List index {index} is out of bounds for length {length}"
    ));
    2
}

#[unsafe(no_mangle)]
pub extern "C" fn ad_optional_error() -> u32 {
    report_error("Adamantium runtime error: cannot access a field or method through None".into());
    2
}

#[repr(C)]
pub struct ArgumentSpec {
    pub name: *const u8,
    pub name_len: usize,
    pub ty: u32,
    pub optional: u32,
}

/// # Safety
/// `argv`, `specs`, and `output` must point to arrays described by their counts.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ad_parse_arguments(
    argc: usize,
    argv: *const *const c_char,
    specs: *const ArgumentSpec,
    count: usize,
    output: *mut Value,
) -> u32 {
    let argv = unsafe { std::slice::from_raw_parts(argv, argc) };
    let specs = unsafe { std::slice::from_raw_parts(specs, count) };
    let output = unsafe { std::slice::from_raw_parts_mut(output, count) };
    output.fill(Value::default());
    let mut seen = vec![false; count];
    let mut index = 1;
    while index < argv.len() {
        let argument = unsafe { CStr::from_ptr(argv[index]) }.to_string_lossy();
        let Some(name) = argument.strip_prefix("--") else {
            eprintln!("Adamantium argument warning: unknown argument '{argument}'");
            index += 1;
            continue;
        };
        let found = specs.iter().position(|spec| {
            let bytes = unsafe { std::slice::from_raw_parts(spec.name, spec.name_len) };
            bytes == name.as_bytes()
        });
        let Some(slot) = found else {
            eprintln!("Adamantium argument warning: unknown argument '--{name}'");
            index += 1;
            if index < argv.len() {
                let next = unsafe { CStr::from_ptr(argv[index]) }.to_bytes();
                if !next.starts_with(b"--") {
                    index += 1;
                }
            }
            continue;
        };
        if index + 1 >= argv.len() {
            eprintln!("Adamantium argument warning: missing value for '--{name}'");
            index += 1;
            continue;
        }
        let bytes = unsafe { CStr::from_ptr(argv[index + 1]) }.to_bytes();
        if bytes.starts_with(b"--") {
            eprintln!("Adamantium argument warning: missing value for '--{name}'");
            index += 1;
            continue;
        }
        let text = match std::str::from_utf8(bytes) {
            Ok(text) => text,
            Err(_) => return 2,
        };
        let declared = Type::from_id(specs[slot].ty).unwrap();
        let inner = if let Type::Optional(inner) = declared {
            Type::from_id(inner).unwrap()
        } else {
            declared
        };
        let value = if inner == Type::String {
            Value {
                lo: bytes.as_ptr() as u64,
                hi: bytes.len() as u64,
            }
        } else if inner == Type::Bool {
            match text {
                "true" => Value { lo: 1, hi: 0 },
                "false" => Value::default(),
                _ => {
                    eprintln!("Adamantium argument error: '--{name}' expects bool, found '{text}'");
                    return 2;
                }
            }
        } else {
            match types::literal(text, inner) {
                Ok(value) => value,
                Err(error) => {
                    eprintln!(
                        "Adamantium argument error: invalid value '{text}' for '--{name}' ({inner}): {error}"
                    );
                    return 2;
                }
            }
        };
        output[slot] = if declared == inner {
            value
        } else {
            types::convert(value, inner, declared).unwrap()
        };
        seen[slot] = true;
        index += 2;
    }
    for (slot, spec) in specs.iter().enumerate() {
        if !seen[slot] && spec.optional == 0 {
            let name = unsafe { std::slice::from_raw_parts(spec.name, spec.name_len) };
            eprintln!(
                "Adamantium argument warning: missing required argument '--{}'",
                String::from_utf8_lossy(name)
            );
        }
    }
    0
}

#[repr(C)]
pub struct Request {
    pub a: Value,
    pub b: Value,
    pub c: Value,
    pub output: Value,
    pub operation: u32,
    pub ty: u32,
    pub from: u32,
    pub reserved: u32,
}

/// # Safety
/// `request` must point to an initialized, writable Request owned by the caller.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ad_evaluate(request: *mut Request) -> u32 {
    let Some(request) = (unsafe { request.as_mut() }) else {
        return 2;
    };
    let Some(ty) = Type::from_id(request.ty) else {
        return 2;
    };
    let result = if request.operation == 5 {
        let Some(from) = Type::from_id(request.from) else {
            return 2;
        };
        types::convert(request.a, from, ty)
    } else {
        types::operation(request.operation, ty, request.a, request.b, request.c)
    };
    match result {
        Ok(value) => {
            request.output = value;
            0
        }
        Err(error) => {
            report_error(format!("Adamantium runtime error: {error}"));
            2
        }
    }
}

/// # Safety
/// `value` must point to an initialized Value. String pointers must reference
/// `hi` readable UTF-8 bytes for the duration of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ad_print(value: *const Value, ty: u32, newline: u32) -> u32 {
    let Some(value) = (unsafe { value.as_ref() }) else {
        return 1;
    };
    let Some(ty) = Type::from_id(ty) else {
        return 1;
    };
    let rendered;
    let bytes = if ty == Type::String {
        if value.lo == 0 && value.hi == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(value.lo as *const u8, value.hi as usize) }
        }
    } else {
        rendered = match types::display(*value, ty) {
            Ok(text) => text,
            Err(_) => return 1,
        };
        rendered.as_bytes()
    };
    let mut stdout = std::io::stdout().lock();
    if stdout.write_all(bytes).is_err()
        || (newline != 0 && stdout.write_all(b"\r\n").is_err())
        || stdout.flush().is_err()
    {
        1
    } else {
        0
    }
}

#[repr(C)]
pub struct PackageCall {
    wasm: Value,
    command: Value,
    arguments: [Value; 8],
    types: [u32; 8],
    count: u32,
    result_type: u32,
    filesystem: u32,
    reserved: u32,
    output: Value,
}

unsafe fn value_text(value: Value) -> Result<String, String> {
    if value.lo == 0 && value.hi == 0 {
        return Ok(String::new());
    }
    let bytes = unsafe { std::slice::from_raw_parts(value.lo as *const u8, value.hi as usize) };
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|error| format!("invalid UTF-8: {error}"))
}

fn package_result(bytes: &[u8], ty: Type) -> Result<Value, String> {
    if ty == Type::None {
        return Ok(Value::default());
    }
    if ty == Type::String {
        let bytes = bytes.to_vec().into_boxed_slice();
        let value = Value {
            lo: bytes.as_ptr() as u64,
            hi: bytes.len() as u64,
        };
        std::mem::forget(bytes);
        return Ok(value);
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|error| format!("package returned non-UTF-8 output: {error}"))?
        .trim();
    if ty == Type::Bool {
        return match text {
            "true" => Ok(Value { lo: 1, hi: 0 }),
            "false" => Ok(Value::default()),
            _ => Err(format!("package returned invalid bool '{text}'")),
        };
    }
    types::literal(text, ty)
}

/// # Safety
/// `request` must point to an initialized, writable `PackageCall` and all
/// contained string values must remain readable for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ad_package_call(request: *mut PackageCall) -> u32 {
    let Some(request) = (unsafe { request.as_mut() }) else {
        return 2;
    };
    let result = (|| -> Result<Value, String> {
        if request.count as usize > request.arguments.len() {
            return Err("package call has too many arguments".into());
        }
        if request.filesystem > 2 {
            return Err("package call has an invalid filesystem permission".into());
        }
        let wasm = unsafe { value_text(request.wasm)? };
        let command = unsafe { value_text(request.command)? };
        let mut arguments = vec!["adamantium-packet".to_owned(), command];
        for index in 0..request.count as usize {
            let ty = Type::from_id(request.types[index]).ok_or("invalid package argument type")?;
            let value = request.arguments[index];
            arguments.push(if ty == Type::String {
                unsafe { value_text(value)? }
            } else {
                types::display(value, ty)?
            });
        }

        let stdout = WritePipe::new_in_memory();
        let stderr = WritePipe::new_in_memory();
        let mut builder = WasiCtxBuilder::new();
        builder
            .args(&arguments)
            .map_err(|error| error.to_string())?
            .stdout(Box::new(stdout.clone()))
            .stderr(Box::new(stderr.clone()));
        if request.filesystem != 0 {
            let directory = wasmi_wasi::Dir::open_ambient_dir(
                std::env::current_dir().map_err(|error| error.to_string())?,
                ambient_authority(),
            )
            .map_err(|error| error.to_string())?;
            builder
                .preopened_dir(directory, ".")
                .map_err(|error| error.to_string())?;
        }
        let engine = Engine::default();
        let bytes = std::fs::read(&wasm).map_err(|error| error.to_string())?;
        let module = Module::new(&engine, &bytes).map_err(|error| error.to_string())?;
        let mut linker: Linker<WasiCtx> = Linker::new(&engine);
        add_to_linker(&mut linker, |context| context).map_err(|error| error.to_string())?;
        let mut store = Store::new(&engine, builder.build());
        let execution = linker
            .instantiate_and_start(&mut store, &module)
            .and_then(|instance| {
                instance
                    .get_typed_func::<(), ()>(&store, "_start")?
                    .call(&mut store, ())
            });
        if let Err(error) = execution {
            if error.i32_exit_status() != Some(0) {
                drop(store);
                let stderr = stderr
                    .try_into_inner()
                    .map_err(|_| "could not read package stderr".to_owned())?
                    .into_inner();
                let stderr = String::from_utf8_lossy(&stderr).into_owned();
                return Err(if stderr.trim().is_empty() {
                    error.to_string()
                } else {
                    stderr
                });
            }
        }
        drop(store);
        let stdout = stdout
            .try_into_inner()
            .map_err(|_| "could not read package stdout".to_owned())?
            .into_inner();
        let result_type =
            Type::from_id(request.result_type).ok_or("invalid package result type")?;
        package_result(&stdout, result_type)
    })();
    match result {
        Ok(value) => {
            request.output = value;
            0
        }
        Err(error) => {
            report_error(format!("Adamantium package error: {}", error.trim()));
            2
        }
    }
}

/// # Safety
/// `message` must point to a string `Value` whose pointer and length are valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ad_message(message: *const Value, line: usize, panic: u32) {
    let Some(message) = (unsafe { message.as_ref() }) else {
        return;
    };
    let bytes = unsafe { std::slice::from_raw_parts(message.lo as *const u8, message.hi as usize) };
    let text = String::from_utf8_lossy(bytes);
    if panic != 0 {
        report_error(format!(
            "Adamantium program panicked at line {line}: {text}"
        ));
    } else {
        eprintln!("Adamantium program warned at line {line}: {text}");
    }
}
