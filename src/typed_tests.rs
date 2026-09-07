use super::*;

fn checked(source: &str) -> Result<Program, String> {
    check(&syntax::parse(source)?)
}

#[test]
fn complete_example_project_stays_valid() {
    checked(include_str!("../example-project/code/main.ad")).unwrap();
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
fn enum_types_are_preserved_and_cannot_be_mixed() {
    let program = checked(
        "enum Color { red, green } enum Direction { left, right } fun main() { var color = Color.red; color = choose(Color.green); print.newline(color); } fun choose(value:Color) result:Color { result = value; }",
    )
    .unwrap();
    assert_eq!(program.functions[0].types, [Type::Enum(0)]);
    assert_eq!(program.functions[1].types, [Type::Enum(0), Type::Enum(0)]);

    for source in [
        "enum A { value } enum B { value } fun main() { var a = A.value; a = B.value; }",
        "enum A { value } fun main() { var a = A.value; a = 1; }",
        "enum A { value } fun main() { var a = A.value + A.value; }",
        "enum A { value } fun main() { var a = A.value; a.clamp(A.value,A.value); }",
    ] {
        assert!(checked(source).is_err(), "accepted {source}");
    }
}

#[test]
fn classes_have_typed_fields_constructors_and_methods() {
    let source = r#"
        class MyClass(
            pub value:int,
            pub other:int
        ) {
            fun __new__() { print.newline("new"); }
            pub fun set_value(new_value:int) result:int {
                self.value = new_value;
                result = self.value;
            }
        }
        fun main() {
            var object = MyClass(value=1,other=2);
            object.set_value(10);
            print.newline(object.value);
            var copy = object;
            copy.value = 20;
            print.newline(object.value);
            print.newline(copy.value);
        }
    "#;
    let program = checked(source).unwrap();
    assert_eq!(program.class_sizes, [2]);
    assert_eq!(program.functions.last().unwrap().types[0], Type::Class(0));

    for source in [
        "class C(pub value:int) { fun __new__() {} } fun main() { var c = C(value=1,extra=2); }",
        "class C(pub value:int) { fun __new__() {} } fun main() { var c = C(); }",
        "class C(value:int) { fun __new__() {} } fun main() { var c = C(value=1); print.newline(c.value); }",
        "class C(pub value:int) { fun __new__() {} fun hidden() r:int { r=self.value; } } fun main() { var c=C(value=1); c.hidden(); }",
    ] {
        assert!(checked(source).is_err(), "accepted {source}");
    }
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

#[test]
fn checks_comparisons_conditions_and_integer_ranges() {
    let program = checked(
        r#"fun main() {
        var a = 0;
        if a < 2 then { a =+ 1; } else { a =- 1; }
        while a != 3 { a =+ 1; }
        until a >= 4 { a =+ 1; }
        for i in 0:u8..3:u8 { print.newline(i); }
        loop { break; }
    }"#,
    )
    .unwrap();
    assert_eq!(program.functions[0].types.last(), Some(&Type::U8));
    for source in [
        "fun main() { if 1 then {} }",
        "fun main() { for i in 0.0..2.0 {} }",
        "fun main() { var a = true < false; }",
    ] {
        assert!(checked(source).is_err(), "accepted {source}");
    }
}

#[test]
fn match_patterns_must_have_the_matched_type() {
    assert!(checked("fun main() { match 2 { 1 => {} 2 => {} _ => {} } }").is_ok());
    assert!(checked("enum Choice { yes, no } fun main() { var c=Choice.yes; match c { Choice.yes => {} _ => {} } }").is_ok());
    assert!(checked("fun main() { match 1 { true => {} } }").is_err());
    assert!(checked("fun main() { match \"text\" { \"text\" => {} } }").is_err());
    assert!(checked("fun main() { match 1 { 1 => {} 1 => {} } }").is_err());
    assert!(checked("fun main() { var pattern=1; match 1 { pattern => {} } }").is_err());
}

#[test]
fn aliases_share_types_and_disconnect_scalar_values() {
    let program = checked("fun main() { var a=10; var b=a.as_variable; a=20; b=15; b.disconect; a=25; print.newline(b); }").unwrap();
    assert_eq!(program.functions[0].types, [Type::I32, Type::I32]);
    assert!(
        checked("enum E { one } fun main() { var a=E.one; var b=a.as_variable; b.disconect; }")
            .is_err()
    );
    assert!(
        checked(
            r#"
        enum E { one }
        class C(pub value:int) { fun __new__() {} }
        fun main() {
            var symbol=identity().as_variable;
            symbol=E.as_variable;
            var choice=symbol.one;
            var ClassAlias=C.as_variable;
            var object=ClassAlias(value=1);
            print.newline(choice);
            print.newline(object.value);
        }
        fun identity(value:int) r:int { r=value; }
    "#
        )
        .is_ok()
    );
    assert!(
        crate::syntax::parse(
            "fun main() { var f=work().as_variable; f.disconect; } fun work() r:None {}"
        )
        .unwrap_err()
        .contains("cannot be disconnected")
    );
}
