use super::*;

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
    let Statement::PrintString(bytes) = &statements[0] else {
        panic!("string expected");
    };
    assert_eq!(bytes, b"/* text */\r\n");
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
    assert_eq!(program.functions[1].variables, 3);
    assert!(parse("fun main() {} fun empty() r:int { r = 0; }").is_ok());
    assert!(parse("fun main() { var a = 1; a.clamp(0,100); }").is_ok());
}

#[test]
fn preserves_strings_comments_and_integer_boundaries() {
    let statements = main_statements(
        "// comment\nfun main() { print.sameline(\"Żółw // \\\"\\\\\\t\\0\"); print.newline(\"Hi\\n\"); var low = -9223372036854775808; var high = 9223372036854775807; }",
    );
    let Statement::PrintString(bytes) = &statements[0] else {
        panic!("string expected");
    };
    assert_eq!(bytes, "Żółw // \"\\\t\0".as_bytes());
    let Statement::PrintString(bytes) = &statements[1] else {
        panic!("string expected");
    };
    assert_eq!(bytes, b"Hi\n\r\n");
    assert!(matches!(
        statements[2],
        Statement::Assign(_, Expr::Integer(i64::MIN))
    ));
    assert!(matches!(
        statements[3],
        Statement::Assign(_, Expr::Integer(i64::MAX))
    ));
}

#[test]
fn rejects_invalid_programs_with_diagnostics() {
    for (source, expected) in [
        ("", "main"),
        ("fun main() {} fun main() {}", "already declared"),
        ("fun main(a:int) {}", "no parameters"),
        ("fun main() {", "expected a statement"),
        ("fun main() { var a = a; }", "not declared"),
        ("fun main() { var a = 1; var a = 2; }", "already declared"),
        (
            "fun main() { var a = 1; print.newline(A); }",
            "not declared",
        ),
        ("fun main() { var print = 1; }", "reserved words"),
        ("fun main() { var a = 1.5; }", "expected Symbol(';')"),
        ("fun main() { var a = 1 }", "expected Symbol(';')"),
        ("fun main() { var a = \"1\"; }", "integer expression"),
        (
            "fun main() { var a = 9223372036854775808; }",
            "signed 64-bit",
        ),
        (
            "fun main() { var a = -9223372036854775809; }",
            "signed 64-bit",
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
            "fun main() {} fun f(a:float) r:int { r = 1; }",
            "expected Word(\"int\")",
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
