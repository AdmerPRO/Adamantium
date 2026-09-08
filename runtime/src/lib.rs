pub mod types;

use std::ffi::{CStr, c_char};
use std::io::Write;
use types::{Type, Value};

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
    eprintln!("Adamantium runtime error: List index {index} is out of bounds for length {length}");
    2
}

#[unsafe(no_mangle)]
pub extern "C" fn ad_optional_error() -> u32 {
    eprintln!("Adamantium runtime error: cannot access a field or method through None");
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
            let _ = writeln!(std::io::stderr(), "Adamantium runtime error: {error}");
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
        unsafe { std::slice::from_raw_parts(value.lo as *const u8, value.hi as usize) }
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
        eprintln!("Adamantium program panicked at line {line}: {text}");
    } else {
        eprintln!("Adamantium program warned at line {line}: {text}");
    }
}
