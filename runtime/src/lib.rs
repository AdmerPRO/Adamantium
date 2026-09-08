pub mod types;

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
