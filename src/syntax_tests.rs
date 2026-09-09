use super::*;

#[test]
fn rejects_invalid_access_with_specific_diagnostics() {
    for body in [
        "var a = 1; a.missing();",
        "var a = 1; print.newline(a.field);",
        "var a = 1; print.newline(a[0]);",
        "var a = 1; a[0] = 2;",
        "var a = 1; a();",
        "var a = 1; print.newline(a());",
        "print.newline(module::value);",
        "module::call();",
        "var a = 1; var b = a.clamp(0,10);",
        "print.newline((1).field);",
    ] {
        let error = parse(&format!("fun main() {{ {body} }}"))
            .and_then(|program| crate::typed::check(&program).map(|_| ()))
            .unwrap_err();
        assert!(error.contains("invalid access"), "{body}: {error}");
    }
    assert!(parse("fun main() { var a = 1; a.clamp(0,10); print.newline(a:i64); }").is_ok());
}

#[test]
fn parses_enum_declarations_and_values() {
    let program = parse(
        "enum MyTable { option, second_option, } fun main() { var a = MyTable.option; a = MyTable.second_option; print.newline(a); }",
    )
    .unwrap();
    assert!(matches!(
        program.functions[0].statements[0],
        Statement::Assign(_, Expr::EnumVariant(Type::Enum(0), 0))
    ));
    assert!(matches!(
        program.functions[0].statements[1],
        Statement::Assign(_, Expr::EnumVariant(Type::Enum(0), 1))
    ));

    for (source, expected) in [
        ("enum Empty {} fun main() {}", "at least one variant"),
        (
            "enum Duplicate { option, option } fun main() {}",
            "already declared",
        ),
        (
            "enum MyTable { option } fun main() { var a = MyTable.missing; }",
            "has no variant 'missing'",
        ),
        (
            "enum MyTable { option } enum MyTable { other } fun main() {}",
            "enum 'MyTable' is already declared",
        ),
        (
            "enum MyTable { option } fun main() { var MyTable = 1; }",
            "conflicts with an enum",
        ),
    ] {
        let error = parse(source).unwrap_err();
        assert!(error.contains(expected), "{error}");
    }
}

#[test]
fn type_aliases_resolve_to_the_original_type() {
    let program = parse(
        "define Number = int; define Numbers = List[Number]; fun identity(value:Number) result:Number { result=value; } fun main() { var number=identity(7:Number); var numbers=List[1,2]:Numbers; print.newline(number); print.newline(numbers[0]); }",
    )
    .unwrap();
    assert_eq!(program.functions[0].types[0], Some(Type::I32));
    assert_eq!(program.functions[0].types[1], Some(Type::I32));
    assert_eq!(program.functions[1].types[0], None);

    for source in [
        "define Value=int; define Value=i64; fun main() {}",
        "define Value=Missing; fun main() {}",
        "define Value=int; enum Value { item } fun main() {}",
        "define Value=int; class Value() { fun __new__() {} } fun main() {}",
        "define Value=int; fun Value() result:None {} fun main() {}",
        "define Value=int; fun main() { var Value=1; }",
    ] {
        assert!(parse(source).is_err(), "{source}");
    }
}

#[test]
fn imports_type_aliases_between_modules() {
    let files = vec![
        ("utils".into(), "define Number=int;".into()),
        (
            "".into(),
            "pack utils; use utils:[Number]; fun main() { var value=7:Number; print.newline(value); }".into(),
        ),
    ];
    parse_modules(&files).unwrap();
}

#[test]
fn expands_generic_functions_and_classes() {
    let program = parse(
        "fun identity<T>(value:T) result:T { result=value; } fun first<A,B>(value:A,other:B) result:A { result=value; } fun list<T>(value:T) result:List[T] { result=List[value]; } class Box<T>(pub value:T) { fun __new__() {} } fun main() { var number=identity<int>(7); var text=identity<string>(\"hello\"); var chosen=first<i64,string>(9:i64,\"x\"); var numbers=list<int>(4); var boxed=Box<i64>(value=chosen); print.newline(number); print.newline(text); print.newline(numbers[0]); print.newline(boxed.value); }",
    )
    .unwrap();
    assert!(
        program
            .functions
            .iter()
            .any(|function| function.name == "identity__generic__int")
    );
    assert!(
        program
            .functions
            .iter()
            .any(|function| function.name == "identity__generic__string")
    );
    assert!(
        program
            .classes
            .iter()
            .any(|class| class.name == "Box__generic__i64")
    );
}

#[test]
fn imports_generic_declarations_between_modules() {
    let files = vec![
        ("utils".into(), "fun identity<T>(value:T) result:T { result=value; }".into()),
        ("".into(), "pack utils; use utils:[identity]; fun main() { print.newline(identity<int>(7)); print.newline(utils:identity<string>(\"ok\")); }".into()),
    ];
    parse_modules(&files).unwrap();
}

#[test]
fn validates_generic_arguments_and_constraints() {
    for (source, expected) in [
        (
            "fun id<T>(value:T) result:T { result=value; } fun main() { print.newline(id(1)); }",
            "requires explicit type arguments",
        ),
        (
            "fun id<T>(value:T) result:T { result=value; } fun main() { print.newline(id<int,string>(1)); }",
            "expects 1 type arguments",
        ),
        (
            "fun add<T:numeric>(value:T) result:T { result=value+value; } fun main() { print.newline(add<string>(\"x\")); }",
            "does not satisfy generic constraint",
        ),
        (
            "fun id<T:mystery>(value:T) result:T { result=value; } fun main() { print.newline(id<int>(1)); }",
            "unknown generic constraint",
        ),
        (
            "fun id<T,T>(value:T) result:T { result=value; } fun main() {}",
            "declared twice",
        ),
    ] {
        let error = parse(source).unwrap_err();
        assert!(error.contains(expected), "{error}");
    }
}

#[test]
fn reports_imports_without_interpreting_comments_or_strings() {
    for directive in [
        "use utils::add;",
        "use utils::[add,subtract];",
        "pack tools.utils;",
        "use;",
        "pack;",
    ] {
        assert!(
            parse(&format!("{directive}\nfun main() {{}}"))
                .unwrap_err()
                .starts_with("1:1: invalid import:")
        );
        assert!(
            parse(&format!("fun main() {{\n{directive}\n}}"))
                .unwrap_err()
                .starts_with("2:1: invalid import:")
        );
    }
    assert!(
        parse("/* pack missing; */ fun main() { // use missing;\nprint.newline(\"use pack\"); }")
            .is_ok()
    );
    assert!(
        parse("fun main() { var use = 1; }")
            .unwrap_err()
            .contains("reserved words")
    );
}

#[test]
fn block_comments_work_between_tokens_and_preserve_operators() {
    let source = "/* header\nignored @ \" // text */fun/**/main() { var a = 12/* note * / */ / 3 * 2; print.newline(a); }/**/";
    let plain = "fun main() { var a = 12 / 3 * 2; print.newline(a); }";
    let tokens = |text| {
        lex(text)
            .unwrap()
            .into_iter()
            .map(|(token, _)| token)
            .collect::<Vec<_>>()
    };
    assert_eq!(tokens(source), tokens(plain));
    assert!(parse(source).is_ok());
    assert!(parse("fun main() { var ab = 1; print.newline(a/**/b); }").is_err());
}

#[test]
fn comment_delimiters_inside_strings_and_line_comments_are_literal() {
    let statements =
        main_statements("// /* not a block comment\nfun main() { print.newline(\"/* text */\"); }");
    let Statement::Print(Expr::String(bytes), true) = &statements[0] else {
        panic!("string expected");
    };
    assert_eq!(bytes, b"/* text */");
    // Block comments end at the first closing delimiter; nesting is not supported.
    assert!(parse("/* outer /* inner */ fun main() {}").is_ok());
}

#[test]
fn block_comment_diagnostics_preserve_source_locations() {
    for text in ["/*", "/* unfinished", "/* ends with *"] {
        assert_eq!(
            lex(text).unwrap_err(),
            "1:1: unterminated block comment; expected '*/'"
        );
    }
    assert_eq!(
        parse("fun main() {\n  /* unfinished\n}").unwrap_err(),
        "2:3: unterminated block comment; expected '*/'"
    );
    assert!(
        parse("/* Żółw\r\ncomment */\r\nfun main() {\r\n @\r\n}")
            .unwrap_err()
            .starts_with("4:2:")
    );
    assert!(lex("/* Ż */@").unwrap_err().starts_with("1:8:"));
}

#[test]
fn static_aliases_reject_every_mutation() {
    for modifier in ["static", "stc"] {
        for mutation in [
            "a = 20;",
            "a =+ 1;",
            "a =- 1;",
            "a =* 2;",
            "a =/ 2;",
            "a.clamp(0,100);",
        ] {
            let source = format!("fun main() {{\nvar {modifier} a = 10;\n{mutation}\n}}");
            let error = parse(&source).unwrap_err();
            assert_eq!(error, "3:1: cannot modify static variable 'a'", "{source}");
        }
        assert!(parse(&format!("fun main() {{ var {modifier} a = add(2,3); print.newline(a); }} fun add(a:int,b:int) r:int {{ r = a+b; }}")).is_ok());
    }
}

#[test]
fn removes_variable_names_and_allows_fresh_redeclarations() {
    assert!(parse("fun main() { var a=10; a.remove; var a=20; print.newline(a); }").is_ok());
    assert!(
        parse("fun main() { var a=10; var b=a.as_variable; a.remove; b=20; print.newline(b); }")
            .is_ok()
    );

    for source in [
        "fun main() { var a=10; var a=20; }",
        "fun main() { var a=10; a.remove; print.newline(a); }",
        "fun main() { var a=10; a.remove; a=20; }",
        "fun main() { var a=10; a.remove; a.remove; }",
        "fun main() { var a=10; a.missing; }",
    ] {
        assert!(parse(source).is_err(), "{source}");
    }
    let error = parse("fun main() { var a=10; a.remove; print.newline(a); }").unwrap_err();
    assert!(error.contains("variable 'a' was removed"), "{error}");
}

#[test]
fn changeable_variables_keep_existing_behavior() {
    for modifier in ["", "ch"] {
        let source = format!(
            "fun main() {{ var {modifier} a = 10; a = 20; a =+ 1; a =- 1; a =* 2; a =/ 2; a.clamp(0,100); }}"
        );
        assert!(parse(&source).is_ok(), "{source}");
    }
    for declaration in [
        "var static ch a = 1;",
        "var ch static a = 1;",
        "var stc static a = 1;",
        "var static = 1;",
    ] {
        assert!(parse(&format!("fun main() {{ {declaration} }}")).is_err());
    }
}

fn main_statements(source: &str) -> Vec<Statement> {
    parse(source)
        .unwrap()
        .functions
        .into_iter()
        .find(|f| f.name == "main")
        .unwrap()
        .statements
}

#[test]
fn precedence_and_left_associativity() {
    let statements =
        main_statements("fun main() { var a = 2+3*4; var b = (2+3)*4; var c = 20/2/5; }");
    let Statement::Assign(_, Expr::Binary(Operator::Add, _, right)) = &statements[0] else {
        panic!("addition expected");
    };
    assert!(matches!(**right, Expr::Binary(Operator::Multiply, _, _)));
    let Statement::Assign(_, Expr::Binary(Operator::Multiply, left, _)) = &statements[1] else {
        panic!("multiplication expected");
    };
    assert!(matches!(**left, Expr::Binary(Operator::Add, _, _)));
    let Statement::Assign(_, Expr::Binary(Operator::Divide, left, _)) = &statements[2] else {
        panic!("division expected");
    };
    assert!(matches!(**left, Expr::Binary(Operator::Divide, _, _)));
}

#[test]
fn assignments_and_negative_values_are_distinct() {
    let statements = main_statements(
        "fun main() { var a = 10; a =+ 2; a =- 3; a =* 4; a =/ 2; a = -5; a = (-6); a = a-1; }",
    );
    for (statement, expected) in statements[1..5].iter().zip([
        Operator::Add,
        Operator::Subtract,
        Operator::Multiply,
        Operator::Divide,
    ]) {
        let Statement::Assign(0, Expr::Binary(operator, left, _)) = statement else {
            panic!("compound assignment expected");
        };
        assert_eq!(
            std::mem::discriminant(operator),
            std::mem::discriminant(&expected)
        );
        assert!(matches!(**left, Expr::Variable(0)));
    }
    assert!(matches!(
        statements[5],
        Statement::Assign(0, Expr::Integer(-5))
    ));
    assert!(matches!(
        statements[6],
        Statement::Assign(0, Expr::Integer(-6))
    ));
}

#[test]
fn functions_forward_calls_and_named_results() {
    let program = parse("fun main() { var result = add(2,add(3,4)); print.newline(result); } fun add(a:int,b:int) r:int { r = a+b; return r; }").unwrap();
    assert_eq!(program.functions[1].parameters, 2);
    assert_eq!(program.functions[1].result, Some(2));
    assert_eq!(program.functions[1].types.len(), 3);
    assert!(parse("fun main() {} fun empty() r:int { r = 0; }").is_ok());
    assert!(parse("fun main() { var a = 1; a.clamp(0,100); }").is_ok());
}

#[test]
fn preserves_strings_comments_and_integer_boundaries() {
    let statements = main_statements(
        "// comment\nfun main() { print.sameline(\"Żółw // \\\"\\\\\\t\\0\"); print.newline(\"Hi\\n\"); var low = -9223372036854775808; var high = 9223372036854775807; }",
    );
    let Statement::Print(Expr::String(bytes), false) = &statements[0] else {
        panic!("string expected");
    };
    assert_eq!(bytes, "Żółw // \"\\\t\0".as_bytes());
    let Statement::Print(Expr::String(bytes), true) = &statements[1] else {
        panic!("string expected");
    };
    assert_eq!(bytes, b"Hi\n");
    assert!(matches!(
        statements[2],
        Statement::Assign(_, Expr::Integer(-9223372036854775808))
    ));
    assert!(matches!(
        statements[3],
        Statement::Assign(_, Expr::Integer(9223372036854775807))
    ));
}

#[test]
fn rejects_invalid_programs_with_diagnostics() {
    for (source, expected) in [
        ("", "main"),
        ("fun main() {} fun main() {}", "already declared"),
        (
            "class C(value:int) { fun __new__() {} } fun main(value:C) {}",
            "cannot be read from the command line",
        ),
        ("fun main() {", "expected a statement"),
        ("fun main() { var a = a; }", "not declared"),
        ("fun main() { var a = 1; var a = 2; }", "already declared"),
        (
            "fun main() { var a = 1; print.newline(A); }",
            "not declared",
        ),
        ("fun main() { var print = 1; }", "reserved words"),
        ("fun main() { var a = 1.5.5; }", "expected a name"),
        ("fun main() { var a = 1 }", "expected Symbol(';')"),
        ("fun main() { var a = ; }", "expected an expression"),
        (
            "fun main() { var a = 999999999999999999999999999999999999999999; }",
            "integer literal is too large",
        ),
        (
            "fun main() { var a = -999999999999999999999999999999999999999999; }",
            "integer literal is too large",
        ),
        ("fun main() { a = 1; }", "not declared"),
        ("fun main() { a.clamp(0,1); }", "not declared"),
        (
            "fun main() { var a = 1; a.clamp(0); }",
            "expected Symbol(',')",
        ),
        (
            "fun main() { var a = missing(1); }",
            "function 'missing' is not declared",
        ),
        (
            "fun main() { var a = add(1); } fun add(a:int,b:int) r:int { r = a+b; }",
            "expects 2 arguments",
        ),
        ("fun main() { var a = main(); }", "main cannot be called"),
        (
            "fun main() {} fun f(a:int,a:int) r:int { r = a; }",
            "already declared",
        ),
        (
            "fun main() {} fun f(a:int) a:int { a = 1; }",
            "already declared",
        ),
        ("fun main() {} fun f() r:int {}", "not initialized"),
        (
            "fun main() {} fun f() r:int { r = r+1; }",
            "not initialized",
        ),
        (
            "fun main() {} fun f() r:int { return r; r = 1; }",
            "not initialized",
        ),
        (
            "fun main() {} fun f() r:int { r = 1; return 2; }",
            "expected Word(\"r\")",
        ),
        (
            "fun main() {} fun f(a:offset) r:int { r = 1; }",
            "unsupported type 'offset'",
        ),
        (
            "fun main() { print.newline(\"\\q\"); }",
            "unsupported string escape",
        ),
        (
            "fun main() { print.newline(\"oops); }",
            "unterminated string",
        ),
    ] {
        let error = parse(source).unwrap_err();
        assert!(error.contains(expected), "{source}: {error}");
    }
    assert!(
        parse("fun main() {\n @\n}")
            .unwrap_err()
            .starts_with("2:2:")
    );
}

#[test]
fn parses_conditions_and_all_loop_forms() {
    let program = parse(
        r#"fun main() {
        var a = 0;
        if a == 0 then { a =+ 1; } else { a =- 1; }
        while a < 3 { a =+ 1; }
        until a >= 5 { a =+ 1; }
        for i in 0..3 { print.newline(i); }
        loop { if a != 0 then { break; } continue; }
    }"#,
    )
    .unwrap();
    assert!(matches!(
        program.functions[0].statements[1],
        Statement::If(..)
    ));
    assert!(matches!(
        program.functions[0].statements[2],
        Statement::While(..)
    ));
    assert!(matches!(
        program.functions[0].statements[3],
        Statement::Until(..)
    ));
    assert!(matches!(
        program.functions[0].statements[4],
        Statement::For(..)
    ));
    assert!(matches!(
        program.functions[0].statements[5],
        Statement::Loop(..)
    ));
    assert!(
        parse("fun main() { break; }")
            .unwrap_err()
            .contains("inside a loop")
    );
}

#[test]
fn parses_match_branches_and_fallback() {
    let program = parse(
        r#"enum State { ready, done } fun main() {
        var state = State.ready;
        match state {
            State.ready => { print.newline("ready"); },
            State.done => { print.newline("done"); }
            _ => { print.newline("unknown"); }
        }
    }"#,
    )
    .unwrap();
    assert!(
        matches!(program.functions[0].statements[1], Statement::Match(_, ref arms, Some(_)) if arms.len() == 2)
    );
    assert!(
        parse("fun main() { match 1 {} }")
            .unwrap_err()
            .contains("at least one branch")
    );
    assert!(
        parse("fun main() { match 1 { _ => {} 1 => {} } }")
            .unwrap_err()
            .contains("must be last")
    );
}

#[test]
fn parses_value_and_symbol_aliases() {
    assert!(
        parse(
            r#"
        enum Choice { first, second }
        class Box(pub value:int) { fun __new__() {} }
        fun main() {
            var value=10;
            var alias=value.as_variable;
            alias=20;
            alias.disconect;
            var operation=add().as_variable;
            operation(1,2);
            operation=nothing().as_variable;
            operation();
            var short=nothing();
            short();
            var E=Choice.as_variable;
            var choice=E.first;
            var B=Box.as_variable;
            var boxed=B(value=choice);
        }
        fun add(a:int,b:int) r:int { r=a+b; }
        fun nothing() r:None {}
    "#
        )
        .is_ok()
    );
    assert!(parse("fun main() { var alias=missing().as_variable; }").is_err());
    assert!(parse("fun main() { var alias=missing(); alias(); }").is_err());
}

#[test]
fn shorthand_function_alias_detection_stays_inside_its_function() {
    let error = parse(
        "fun main() { var value=result(); print.newline(value); } fun other() r:None { value(); } fun result() r:int { r=7; }",
    )
    .unwrap_err();
    assert!(
        error.contains("function 'value' is not declared"),
        "{error}"
    );
}

#[test]
fn combines_namespaced_modules_and_use_imports() {
    let files = vec![
        ("utils".into(), "fun value(a:int) r:int { r=a+1; } fun other() r:int { r=2; } enum State { ready }".into()),
        ("utils/tools".into(), "pack utils; use utils:[value]; fun calculate() r:int { r=value(4); }".into()),
        ("".into(), "pack utils; pack utils/tools; use utils:[value]; fun main() { print.newline(utils:value(1)); print.newline(value(2)); print.newline(utils/tools:calculate()); var state=utils:State.ready; }".into()),
    ];
    let program = parse_modules(&files).unwrap();
    assert!(
        program
            .functions
            .iter()
            .any(|function| function.name == "admod__utils__value")
    );
    assert!(
        program
            .functions
            .iter()
            .any(|function| function.name == "admod__utils__tools__calculate")
    );

    let missing = vec![("".into(), "use utils:[missing]; fun main() {}".into())];
    assert!(
        parse_modules(&missing)
            .unwrap_err()
            .contains("does not export")
    );
}

#[test]
fn parses_list_literals_indexing_and_assignment() {
    let statements = main_statements(
        "fun main() { var values = List[1, 2, 3]; values[1] = 9; print.newline(values[1]); }",
    );
    assert!(matches!(statements[0], Statement::Assign(_, Expr::List(_))));
    assert!(matches!(statements[1], Statement::SetIndex(_, _, _)));
    assert!(matches!(
        statements[2],
        Statement::Print(Expr::Index(_, _, _), true)
    ));
}
