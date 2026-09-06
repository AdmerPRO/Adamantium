use super::*;

fn checked(source: &str) -> Result<Program, String> {
    check(&syntax::parse(source)?)
}

#[test]
fn infers_default_types_and_accepts_aliases() {
    let program = checked(
        r#"fun main() {
        variable a = 10;
        variable changeable b = 1.5;
        var changeable c = "hello";
        variable static d = true;
        variable stc e = None;
        var f = 10:int;
        var g = 10:float;
        var h = 10:u;
    }"#,
    )
    .unwrap();
    assert_eq!(
        program.functions[0].types,
        [
            Type::I32,
            Type::F64,
            Type::String,
            Type::Bool,
            Type::None,
            Type::I32,
            Type::F64,
            Type::U32
        ]
    );
}

#[test]
fn integer_ranges_are_checked_for_every_width() {
    for ty in [
        Type::I8,
        Type::I16,
        Type::I32,
        Type::I64,
        Type::U4,
        Type::U8,
        Type::U16,
        Type::U32,
        Type::U64,
    ] {
        let (low, high) = ty.bounds();
        let valid = format!("fun main() {{ var a = {low}:{ty}; var b = {high}:{ty}; }}");
        assert!(checked(&valid).is_ok(), "{valid}");
        for value in [low - 1, high + 1] {
            assert!(
                checked(&format!("fun main() {{ var a = {value}:{ty}; }}")).is_err(),
                "{ty}: {value}"
            );
        }
    }
    assert!(checked("fun main() { var a = 2147483648; }").is_err());
    assert!(checked("fun main() { var a = 2147483648:i64; }").is_ok());
}

#[test]
fn rejects_incompatible_types_and_reserved_future_types() {
    for body in [
        "var a = +true;",
        "var a = +None;",
        "var a = 1; a = true;",
        "var a = true:i32;",
        "var a = 1:bool;",
        "var a = \"hello\":i32;",
        "var a = 1.5:int;",
        "var a = true; a.clamp(0,1);",
        "var a = \"a\"+\"b\";",
        "var a = None; a = 1;",
        "var a = 1:u64; print.newline(-a);",
        "var a = 1e100:f32;",
        "var a = 1e400:f64;",
        "var a = 1e5000:f128;",
    ] {
        assert!(
            checked(&format!("fun main() {{ {body} }}")).is_err(),
            "accepted {body}"
        );
    }
    for ty in ["offset", "oofset", "List", "enum", "class", "fun"] {
        assert!(checked(&format!("fun main() {{ var a = 1:{ty}; }}")).is_err());
    }
}

#[test]
fn typed_functions_preserve_results_and_validate_arguments() {
    let program = checked(
        r#"fun main() {
        var a = greet("hello");
        var b = fraction(1.5);
        var c = flag(true);
        var d = nothing();
    }
    fun greet(a:string) r:string { r = a; }
    fun fraction(a:f32) r:f32 { r = a/2; }
    fun flag(a:bool) r:bool { r = a; return r; }
    fun nothing() r:None {}"#,
    )
    .unwrap();
    assert_eq!(
        program.functions[0].types,
        [Type::String, Type::F32, Type::Bool, Type::None]
    );
    assert!(checked("fun main() { var a = f(true); } fun f(a:int) r:int { r = a; }").is_err());
    assert!(checked("fun main() {} fun f() r:bool { r = 10; }").is_err());
}

#[test]
fn floating_formats_round_and_preserve_quad_precision() {
    let single = types::literal("16777217", Type::F32).unwrap();
    assert_eq!(types::display(single, Type::F32).unwrap(), "16777216");
    let a = types::literal("1267650600228229401496703205377", Type::F128).unwrap();
    let b = types::literal("1267650600228229401496703205376", Type::F128).unwrap();
    assert_eq!(
        types::operation(1, Type::F128, a, b, Value::default()).unwrap(),
        types::literal("1", Type::F128).unwrap()
    );
    let large = types::literal("1e4000", Type::F128).unwrap();
    assert_eq!(
        types::operation(3, Type::F128, large, large, Value::default()).unwrap(),
        types::literal("1", Type::F128).unwrap()
    );
    let single = types::literal("0.1", Type::F32).unwrap();
    let double = types::convert(single, Type::F32, Type::F64).unwrap();
    assert_eq!(double.lo, (0.1_f32 as f64).to_bits());
}
