#![cfg(windows)]

use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

#[test]
#[ignore = "requires NASM and Visual Studio C++ build tools"]
fn native_test_runner_reports_passes_failures_and_filters() {
    let project = Project::new("fun main() {}");
    fs::write(
        project.0.join("code/tests.ad"),
        "#[test]\nfun passing() { print.newline(\"pass output\"); }\n#[test]\nfun failing() { panic(\"expected failure\"); }\n",
    )
    .unwrap();
    let all = Command::new(env!("CARGO_BIN_EXE_adamantium"))
        .args(["test", "run"])
        .arg(&project.0)
        .output()
        .unwrap();
    assert_eq!(all.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&all.stdout);
    assert!(stdout.contains("test passing ... ok"));
    assert!(stdout.contains("test failing ... FAILED"));
    assert!(stdout.contains("1 passed; 1 failed"));

    let filtered = Command::new(env!("CARGO_BIN_EXE_adamantium"))
        .args(["test", "run"])
        .arg(&project.0)
        .args(["passing", "--verbose"])
        .output()
        .unwrap();
    assert!(filtered.status.success());
    assert!(String::from_utf8_lossy(&filtered.stdout).contains("pass output"));
}

#[test]
#[ignore = "requires NASM and Visual Studio C++ build tools"]
fn native_exit_is_successful_and_silent() {
    let project = Project::new("fun main() { exit(); panic(\"must not run\"); }");
    let build = Command::new(env!("CARGO_BIN_EXE_adamantium"))
        .arg("build")
        .arg(&project.0)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = Command::new(project.0.join("target/NativeTest.exe"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}

#[test]
#[ignore = "requires NASM and Visual Studio C++ build tools"]
fn native_prints_class_names_and_public_fields() {
    let source = r#"class Item(pub number:int,secret:string,pub values:List[int]) { fun __new__() {} } fun main() { var item=Item(number=7,secret="hidden",values=List[1,2]); print.newline(item); }"#;
    let output = Project::new(source).run();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"Item(number=7, values=[1, 2])\r\n");
}

#[test]
#[ignore = "requires NASM and Visual Studio C++ build tools"]
fn native_traits_and_trait_constrained_generics() {
    let source = r#"
        trait Printable { fun render() result:int; }
        class Number(pub value:int) implements Printable {
            fun __new__() {}
            pub fun render() result:int { result=self.value; }
        }
        fun render_value<T:Printable>(value:T) result:int { result=value.render(); }
        fun main() {
            var number=Number(value=17);
            print.newline(render_value<Number>(number));
        }
    "#;
    let output = Project::new(source).run();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"17\r\n");
}

#[test]
#[ignore = "requires NASM and Visual Studio C++ build tools"]
fn native_nested_lists_optional_lists_and_recursion() {
    let source = "fun sum(n:int) result:int { if n==0 then { result=0; return result; } result=n+sum(n-1); } fun optional($values:List[int]) result:None {} fun main() { var nested=List[List[1,2],List[3,4]]; optional(); optional(nested[0]); print.newline(nested[1][0]); print.newline(sum(5)); }";
    let output = Project::new(source).run();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"3\r\n15\r\n");
}

#[test]
#[ignore = "requires NASM and Visual Studio C++ build tools"]
fn native_class_lifecycle_hooks_run_in_order() {
    let source = "class Counter(pub value:int) { fun __new__() { print.newline(\"new\"); } fun __change__() { print.newline(\"change\"); } fun __remove__() { print.newline(\"remove\"); } } fun main() { var counter=Counter(value=1); counter.value=2; counter.remove; }";
    let output = Project::new(source).run();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"new\r\nchange\r\nremove\r\n");
}

#[test]
#[ignore = "requires NASM and Visual Studio C++ build tools"]
fn native_aliased_class_is_finalized_after_its_last_name_is_removed() {
    let output = Project::new(
        r#"
        class Resource(pub value:int) {
            fun __new__() {}
            fun __change__() { print.newline("changed"); }
            fun __remove__() { print.newline("removed"); }
        }
        fun main() {
            var resource=Resource(value=1);
            var alias=resource.as_variable;
            resource.remove;
            alias.value=2;
            alias.remove;
        }
    "#,
    )
    .run();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"changed\r\nremoved\r\n");
}

#[test]
#[ignore = "requires NASM and Visual Studio C++ build tools"]
fn native_removed_names_can_be_redeclared() {
    let source = "fun main() { var a=10; var alias=a.as_variable; a.remove; alias=15; var a=20; print.newline(alias); print.newline(a); }";
    let output = Project::new(source).run();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"15\r\n20\r\n");
}

#[test]
#[ignore = "requires NASM and Visual Studio C++ build tools"]
fn native_type_aliases_use_the_original_runtime_types() {
    let source = "define Number=int; define Numbers=List[Number]; fun echo(value:Number) result:Number { result=value; } fun main() { var value=echo(7:Number); var values=List[1,2]:Numbers; print.newline(value); print.newline(values[1]); }";
    let output = Project::new(source).run();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"7\r\n2\r\n");
}

#[test]
#[ignore = "requires NASM and Visual Studio C++ build tools"]
fn native_generic_specializations() {
    let source = "fun identity<T>(value:T) result:T { result=value; } fun pair<A,B>(value:A,ignored:B) result:A { result=value; } class Box<T>(pub value:T) { fun __new__() {} } fun main() { var number=identity<int>(7); var text=identity<string>(\"hello\"); var selected=pair<i64,string>(9:i64,\"ignored\"); var boxed=Box<i64>(value=selected); print.newline(number); print.newline(text); print.newline(boxed.value); }";
    let output = Project::new(source).run();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"7\r\nhello\r\n9\r\n");
}

#[test]
#[ignore = "requires NASM and Visual Studio C++ build tools"]
fn native_lists_create_read_and_update_elements() {
    let output = Project::new(
        "fun main() { var values=List[1,2,3]; print.newline(values[0]); values[1]=9; print.newline(values[1]); var copy=values; copy[0]=7; print.newline(values[0]); print.newline(copy[0]); }",
    ).run();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"1\r\n9\r\n1\r\n7\r\n");

    let output = Project::new("fun main() { var values=List[1]; print.newline(values[1]); }").run();
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("List index 1 is out of bounds for length 1")
    );
}

#[test]
#[ignore = "requires NASM and Visual Studio C++ build tools"]
fn native_wide_optional_values() {
    let source = r#"class Child(pub value:int){fun __new__(){}} class Options(pub &text:string,pub &wide:f128,pub &child:Child){fun __new__(){}} fun main(){var child=Child(value=7);var empty=Options();var full=Options(text="hello",wide=1.25:f128,child=child);print.newline(empty.text);print.newline(full.text);print.newline(full.wide);print.newline(full.child.value);}"#;
    let output = Project::new(source).run();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"None\r\nhello\r\n1.25\r\n7\r\n");

    let missing = r#"class Child(pub value:int){fun __new__(){}} class Options(pub &child:Child){fun __new__(){}} fun main(){var empty=Options();print.newline(empty.child.value);}"#;
    let output = Project::new(missing).run();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("through None"));
}

#[test]
#[ignore = "requires NASM and Visual Studio C++ build tools"]
fn native_main_command_line_arguments() {
    let project = Project::new(
        "fun main(number:int,$text:string,$enabled:bool){print.newline(number);print.newline(text);print.newline(enabled);}",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_adamantium"))
        .arg("run")
        .arg(&project.0)
        .args(["--number", "7", "--text", "hello", "--enabled", "true"])
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"7\r\nhello\r\ntrue\r\n");

    let executable = project.0.join("target/NativeTest.exe");
    let missing = Command::new(&executable)
        .args(["--text", "present", "--unknown", "value"])
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&missing.stderr);
    assert!(stderr.contains("missing required argument '--number'"));
    assert!(stderr.contains("unknown argument '--unknown'"));

    let invalid = Command::new(executable)
        .args(["--number", "wrong"])
        .output()
        .unwrap();
    assert_eq!(invalid.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("argument error"));

    let required_string = Project::new("fun main(text:string){print.newline(text);}");
    let output = required_string.run();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, b"\r\n");
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing required argument '--text'"));
}

#[test]
#[ignore = "requires NASM and Visual Studio C++ build tools"]
fn native_explicit_as_conversions_are_checked() {
    let output = Project::new(
        "fun main() { var source=300:i32; var narrow=source.as(i16); print.newline(narrow); var decimal=narrow.as(f64); print.newline(decimal); }",
    )
    .run();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"300\r\n300\r\n");

    let overflow = Project::new(
        "fun main() { var source=300:i32; var narrow=source.as(u8); print.newline(narrow); }",
    )
    .run();
    assert_eq!(overflow.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&overflow.stderr).contains("runtime error"));
}

#[test]
#[ignore = "requires NASM and Visual Studio C++ build tools"]
fn native_class_operator_overloads() {
    let source = r#"class Number(pub value:int){
        fun __new__(){}
        pub fun __add__(other:Number) r:int{r=self.value+other.value;}
        pub fun __sub__(other:Number) r:int{r=self.value-other.value;}
        pub fun __mul__(other:Number) r:int{r=self.value*other.value;}
        pub fun __div__(other:Number) r:int{r=self.value/other.value;}
        pub fun __eq__(other:Number) r:bool{r=self.value==other.value;}
        pub fun __ne__(other:Number) r:bool{r=self.value!=other.value;}
        pub fun __lt__(other:Number) r:bool{r=self.value<other.value;}
        pub fun __le__(other:Number) r:bool{r=self.value<=other.value;}
        pub fun __gt__(other:Number) r:bool{r=self.value>other.value;}
        pub fun __ge__(other:Number) r:bool{r=self.value>=other.value;}
    } fun main(){var a=Number(value=8);var b=Number(value=2);print.newline(a+b);print.newline(a-b);print.newline(a*b);print.newline(a/b);print.newline(a==b);print.newline(a!=b);print.newline(a<b);print.newline(a<=b);print.newline(a>b);print.newline(a>=b);}"#;
    let output = Project::new(source).run();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        output.stdout,
        b"10\r\n6\r\n16\r\n4\r\nfalse\r\ntrue\r\nfalse\r\nfalse\r\ntrue\r\ntrue\r\n"
    );
}

#[test]
#[ignore = "requires NASM and Visual Studio C++ build tools"]
fn native_logic_remainder_warnings_and_optional_values() {
    let output = Project::new(
        r#"
        class Options(pub &value:int) { fun __new__() {} }
        fun main() {
            print.newline(!false);
            print.newline(not true);
            print.newline(true && true);
            print.newline(false and explode());
            print.newline(true || explode());
            print.newline(false or true);
            print.newline(17 / 5);
            print.newline(17 % 5);
            show();
            show(0);
            show(None);
            var empty=Options();
            var full=Options(value=7);
            print.newline(empty.value);
            print.newline(full.value);
            warn("careful");
        }
        fun explode() result:bool { var zero=0; var bad=1/zero; result=bad==0; }
        fun show($value:int) result:None { print.newline(value); }
    "#,
    )
    .run();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"true\r\nfalse\r\ntrue\r\nfalse\r\ntrue\r\ntrue\r\n3\r\n2\r\nNone\r\n0\r\nNone\r\nNone\r\n7\r\n");
    let warning = String::from_utf8_lossy(&output.stderr);
    assert!(
        warning.contains("Adamantium program warned at line"),
        "{warning}"
    );
    assert!(warning.contains(": careful"), "{warning}");
}

#[test]
#[ignore = "requires NASM and Visual Studio C++ build tools"]
fn native_panic_reports_source_line_and_stops() {
    let output = Project::new(
        "fun main() {\n    print.newline(1);\n    panic(\"broken\");\n    print.newline(2);\n}",
    )
    .run();
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stdout, b"1\r\n");
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("Adamantium program panicked at line 3: broken")
    );
}

#[test]
#[ignore = "requires NASM and Visual Studio C++ build tools"]
fn native_try_catches_runtime_errors_and_panics() {
    let output = Project::new(
        r#"
        fun divide() result:int {
            result = 10 / 0;
        }
        fun main() {
            var success = try { print.newline("inside"); };
            print.newline(success);
            var arithmetic = try {
                divide();
                print.newline("unreachable");
            };
            print.newline(arithmetic);
            var panicked = try {
                panic("caught panic");
                print.newline("also unreachable");
            };
            print.newline(panicked);
            print.newline("continued");
        }
    "#,
    )
    .run();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        output.stdout,
        b"inside\r\nNone\r\nAdamantium runtime error: arithmetic overflow, division by zero or invalid clamp range\r\nAdamantium program panicked at line 14: caught panic\r\ncontinued\r\n"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("runtime error"), "{stderr}");
    assert!(!stderr.contains("panicked"), "{stderr}");
}

#[test]
#[ignore = "requires NASM and Visual Studio C++ build tools"]
fn native_offsets_read_current_values_and_preserve_types() {
    let output = Project::new(
        r#"
        fun main() {
            var source = 10:i16;
            var address = source.offset;
            source = 20;
            var copied = address.by_offset;
            print.newline(copied);
            copied = 15;
            print.newline(source);

            var text = "before";
            var text_address = text.get_offset();
            text = "after";
            print.newline(text_address.value_by_offset);
        }
    "#,
    )
    .run();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"20\r\n20\r\nafter\r\n");
}

#[test]
#[ignore = "requires NASM and Visual Studio C++ build tools"]
fn native_optional_enums_methods_and_matching() {
    let output = Project::new(
        r#"
        pub enum Choice { first, second }
        class Reporter(pub &value:int) {
            fun __new__() {}
            pub fun report($fallback:int) result:None {
                print.newline(self.value);
                print.newline(fallback);
            }
        }
        fun main() {
            print.newline(true or false and false);
            print.newline((true or false) and false);
            classify(); classify(0); classify(4);
            show_choice(); show_choice(Choice.second);
            var reporter=Reporter();
            reporter.report();
            reporter.report(0);
            var message="variable warning";
            warn(message);
        }
        fun classify($value:int) result:None {
            match value {
                None => { print.newline("none"); }
                0 => { print.newline("zero"); }
                _ => { print.newline("other"); }
            }
        }
        fun show_choice($value:Choice) result:None { print.newline(value); }
    "#,
    )
    .run();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        output.stdout,
        b"true\r\nfalse\r\nnone\r\nzero\r\nother\r\nNone\r\n1\r\nNone\r\nNone\r\nNone\r\n0\r\n"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("variable warning"));
}

#[test]
#[ignore = "requires NASM and Visual Studio C++ build tools"]
fn native_multifile_pack_qualified_calls_and_use_imports() {
    let project = Project::new(
        r#"
        pack utils;
        pack utils/tools;
        use utils:[add,Choice,Box];
        fun main() {
            print.newline(utils:add(2,3));
            print.newline(add(4,5));
            print.newline(utils/tools:calculate(6));
            var choice=Choice.second;
            print.newline(choice);
            var object=Box(value=7);
            print.newline(object.value);
        }
    "#,
    );
    fs::create_dir_all(project.0.join("code/utils")).unwrap();
    fs::write(
        project.0.join("code/utils.ad"),
        r#"
        pub enum Choice { first, second }
        pub class Box(pub value:int) { fun __new__() {} }
        pub fun add(a:int,b:int) result:int { result=a+b; }
    "#,
    )
    .unwrap();
    fs::write(
        project.0.join("code/utils/tools.ad"),
        r#"
        pack utils;
        use utils:[add];
        pub fun calculate(value:int) result:int { result=add(value,10); }
    "#,
    )
    .unwrap();
    let output = project.run();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"5\r\n9\r\n16\r\n1\r\n7\r\n");
}

#[test]
#[ignore = "requires NASM and Visual Studio C++ build tools"]
fn native_value_and_function_aliases() {
    let output = Project::new(
        r#"
        fun main() {
            var a = 10;
            var b = a.as_variable;
            a = 20;
            print.newline(b);
            b = 15;
            print.newline(a);
            b.disconect;
            a = 25;
            print.newline(a);
            print.newline(b);
            var operation = add().as_variable;
            print.newline(operation(2,3));
            operation = twice().as_variable;
            print.newline(operation(6));
            var short = add();
            print.newline(short(3,4));
        }
        fun add(a:int,b:int) r:int { r=a+b; }
        fun twice(value:int) r:int { r=value*2; }
    "#,
    )
    .run();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"20\r\n15\r\n25\r\n15\r\n5\r\n12\r\n7\r\n");
}

#[test]
#[ignore = "requires NASM and Visual Studio C++ build tools"]
fn native_match_selects_the_first_matching_branch() {
    let output = Project::new(
        r#"
        enum Choice { first, second }
        fun main() {
            var choice = Choice.second;
            match choice {
                Choice.first => { print.newline("first"); }
                Choice.second => { print.newline("second"); }
                _ => { print.newline("fallback"); }
            }
            match 9 { 1 => { print.newline("one"); } _ => { print.newline("other"); } }
        }
    "#,
    )
    .run();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"second\r\nother\r\n");
}

#[test]
#[ignore = "requires NASM and Visual Studio C++ build tools"]
fn native_conditions_and_loops() {
    let output = Project::new(
        r#"
        fun main() {
            var value = 0;
            if value == 0 then { print.newline("if"); } else { print.newline("else"); }
            while value < 2 { print.newline(value); value =+ 1; }
            until value >= 4 { print.newline(value); value =+ 1; }
            for i in 0..3 { print.newline(i); }
            loop {
                value =+ 1;
                if value == 5 then { continue; }
                print.newline(value);
                if value >= 6 then { break; }
            }
        }
    "#,
    )
    .run();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        output.stdout,
        b"if\r\n0\r\n1\r\n2\r\n3\r\n0\r\n1\r\n2\r\n6\r\n"
    );
}

#[test]
#[ignore = "requires NASM and Visual Studio C++ build tools"]
fn native_for_iterates_over_supported_list_values() {
    let output = Project::new(
        r#"
        class Item(pub value:int) { fun __new__() {} }
        fun main() {
            var total=0;
            for number in List[1,2,3,4] {
                if number == 2 then { continue; }
                if number == 4 then { break; }
                total =+ number;
            }
            print.newline(total);

            var words=List["one","two"];
            for word in words { print.newline(word); }

            var items=List[Item(value=7),Item(value=8)];
            for item in items {
                print.newline(item.value);
            }
            print.newline(items[0].value);
        }
    "#,
    )
    .run();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"4\r\none\r\ntwo\r\n7\r\n8\r\n7\r\n");
}

#[test]
#[ignore = "requires NASM and Visual Studio C++ build tools"]
fn native_classes_construct_mutate_and_copy_values() {
    let output = Project::new(
        r#"
        class MyClass(pub value:int,pub other:int) {
            fun __new__() { print.newline("new"); }
            pub fun set_value(new_value:int) result:int {
                self.value = new_value;
                result = self.value;
            }
        }
        fun main() {
            var first = MyClass(value=1,other=2);
            var changed = first.set_value(10);
            var second = first;
            second.set_value(20);
            var third = copy_object(first);
            third.value = 30;
            print.newline(changed);
            print.newline(first.value);
            print.newline(second.value);
            print.newline(second.other);
            print.newline(change_argument(first));
            print.newline(first.value);
            print.newline(third.value);
        }
        fun change_argument(value:MyClass) result:int {
            value.value = 40;
            result = value.value;
        }
        fun copy_object(value:MyClass) result:MyClass { result = value; }
        "#,
    )
    .run();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        output.stdout,
        b"new\r\n10\r\n10\r\n20\r\n2\r\n40\r\n10\r\n30\r\n"
    );
}

#[test]
#[ignore = "requires NASM and Visual Studio C++ build tools"]
fn native_enum_values_can_be_stored_passed_and_printed() {
    let output = Project::new(
        "enum Choice { first, second } fun main() { var value = choose(Choice.second); print.newline(value); } fun choose(value:Choice) result:Choice { result = value; }",
    )
    .run();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, b"1\r\n");
}

#[test]
#[ignore = "requires NASM and Visual Studio C++ build tools"]
fn native_warnings_do_not_block_builds() {
    let project = Project::new(
        "fun main() { var unused = 1; print.newline(value()); } fun value() r:int { r = 7; return r; print.newline(999); } fun unused_function() r:None {}",
    );
    let build = Command::new(env!("CARGO_BIN_EXE_adamantium"))
        .arg("build")
        .arg(&project.0)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(build.status.success(), "{stderr}");
    for code in ["W001", "W002", "W003"] {
        assert!(stderr.contains(&format!("warning[{code}]")), "{stderr}");
    }
    let output = Command::new(project.0.join("target/NativeTest.exe"))
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"7\r\n");
}

#[test]
#[ignore = "requires NASM and Visual Studio C++ build tools"]
fn native_all_types_and_typed_calls() {
    let output = Project::new(
        r#"
        fun main() {
            variable a = -128:i8; print.newline(a);
            var b = -32768:i16; print.newline(b);
            var c = -2147483648:i32; print.newline(c);
            var d = -9223372036854775808:i64; print.newline(d);
            var e = 15:u4; print.newline(e);
            var f = 255:u8; print.newline(f);
            var g = 65535:u16; print.newline(g);
            var h = 4294967295:u32; print.newline(h);
            var i = 18446744073709551615:u64; print.newline(i);
            i =/ 3; print.newline(i);
            var j = 1.5:f32; j =* 2; print.newline(j);
            var k = 2.5:float; k =+ 1; print.newline(k);
            var m = 1.25:f128; m =* 2; m.clamp(0,2); print.newline(m);
            variable changeable text = "hello":string;
            var stc original = text;
            text = echo("Żółw\0!"); print.newline(text); print.newline(original);
            var yes = true:bool; yes = flag(false); print.newline(yes);
            var missing = None; print.newline(missing);
            var none = nothing(); print.newline(none);
            print.newline(fraction(3.5));
            var precise = 1267650600228229401496703205377:f128;
            var previous = 1267650600228229401496703205376:f128;
            var difference = subtract(precise,previous);
            print.newline(difference);
            var huge = 1e4000:f128; print.newline(huge/huge);
            var small = 0.1:f32;
            var wide = small:f64; print.newline(wide);
        }
        fun echo(value:string) result:string { result = value; return result; }
        fun flag(value:bool) result:bool { result = value; }
        fun nothing() result:None {}
        fun fraction(value:f64) result:f64 { result = value/2; }
        fun subtract(a:f128,b:f128) result:f128 { result = a-b; }
    "#,
    )
    .run();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<_> = text.split("\r\n").collect();
    assert_eq!(
        &lines[..12],
        &[
            "-128",
            "-32768",
            "-2147483648",
            "-9223372036854775808",
            "15",
            "255",
            "65535",
            "4294967295",
            "18446744073709551615",
            "6148914691236517205",
            "3",
            "3.5"
        ]
    );
    assert_eq!(lines[12].parse::<f64>().unwrap(), 2.0);
    assert_eq!(
        &lines[13..19],
        &["Żółw\0!", "hello", "false", "None", "None", "1.75"]
    );
    assert_eq!(lines[19].parse::<f64>().unwrap(), 1.0);
    assert_eq!(lines[20].parse::<f64>().unwrap(), 1.0);
    assert_eq!(lines[21].parse::<f64>().unwrap(), 0.1_f32 as f64);
    assert_eq!(lines[22], "");
}

#[test]
#[ignore = "requires NASM and Visual Studio C++ build tools"]
fn native_static_and_changeable_variables() {
    let output = Project::new(
        r#"
        fun main() {
            var static a = 10;
            var stc b = add(a,5);
            var ch c = b;
            c =+ 5;
            c.clamp(0,18);
            print.newline(a);
            print.newline(b);
            print.newline(c);
            print.newline(change(a));
            print.newline(a);
            print.newline(local());
            print.newline(local());
        }
        fun add(a:int,b:int) r:int { r = a+b; }
        fun change(a:int) r:int { a =+ 1; r = a; }
        fun local() r:int { var static a = 7; r = a; }
    "#,
    )
    .run();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, b"10\r\n15\r\n18\r\n11\r\n10\r\n7\r\n7\r\n");
}

struct Project(PathBuf);
impl Project {
    fn new(source: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "adamantium-native-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("code")).unwrap();
        fs::write(root.join("code/main.ad"), source).unwrap();
        fs::write(root.join("project.toml"), "name = \"NativeTest\"\nversion = \"1.0.0\"\ndescription = \"Native regression test\"\nauthors = []\n").unwrap();
        fs::write(root.join("requirement.toml"), "[packages]\n").unwrap();
        Self(root)
    }
    fn run(&self) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_adamantium"))
            .arg("run")
            .arg(&self.0)
            .output()
            .unwrap()
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
#[ignore = "requires NASM and Visual Studio C++ build tools"]
fn native_generic_wasi_package_binding() {
    let project = Project::new(
        r#"mod ExamplePackage;
        use ExamplePackage:ping;
        fun main() {
            var message = ping();
            print.newline(message);
        }"#,
    );
    fs::write(
        project.0.join("requirement.toml"),
        "[packages]\n\"https://github.com/community/example-package\" = \"1.0.0\"\n",
    )
    .unwrap();
    let package = project
        .0
        .join("packages/example-package/1.0.0/adamantium_packet.wasm");
    fs::create_dir_all(package.parent().unwrap()).unwrap();
    fs::write(
        package.parent().unwrap().join("adamantium_packet.toml"),
        "[package]\nname=\"ExamplePackage\"\nversion=\"1.0.0\"\nabi=\"wasi-command-v1\"\n\n[functions.ping]\nparameters=[]\nresult=\"string\"\n",
    )
    .unwrap();
    fs::write(
        package,
        wat::parse_str(
            r#"
        (module
            (import "wasi_snapshot_preview1" "fd_write"
                (func $fd_write (param i32 i32 i32 i32) (result i32)))
            (memory (export "memory") 1)
            (data (i32.const 16) "packet output")
            (func (export "_start")
                (i32.store (i32.const 0) (i32.const 16))
                (i32.store (i32.const 4) (i32.const 13))
                (drop (call $fd_write
                    (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 8))))
        )
    "#,
        )
        .unwrap(),
    )
    .unwrap();

    let output = project.run();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"packet output\r\n");
}

#[test]
#[ignore = "requires NASM and Visual Studio C++ build tools"]
fn native_arithmetic_clamp_calls_and_returns() {
    let project = Project::new(
        r#"
        fun main() {
            var a = 10;
            var b = 20;
            b =+ a; print.newline(b);
            b =* a; print.newline(b);
            b =/ a; print.newline(b);
            b =- a; print.newline(b);
            b = a+b; print.newline(b);
            print.newline(2+3*4);
            print.newline((2+3)*4);
            print.newline(20/2/5);
            print.newline(-7/2);
            print.newline(7/-2);
            print.newline(-7/-2);
            a = -5;
            a.clamp(0,100); print.newline(a);
            a = 150;
            a.clamp(0,100); print.newline(a);
            a = 150; print.newline(a);
            a = 40; a.clamp(0,100); print.newline(a);
            a.clamp(5,5); print.newline(a);
            var result = add(10,20); print.newline(result);
            print.newline(add(add(1,2),add(3,4))*add(2,3));
            print.newline(early(9));
            print.newline(six(1,2,3,4,5,6));
            print.newline(add(mark(1),mark(2)));
            print.newline(a);
            a.clamp(add(0,10),add(20,30)); print.newline(a);
            print.newline(-9223372036854775808:i64);
            print.newline(9223372036854775807:i64);
            print.sameline(0); print.newline("");
            print.newline("Żółw");
            print.sameline("a\0b");
        }
        fun add(a:int,b:int) r:int { r = a+b; }
        fun early(a:int) r:int { r = a*2; return r; r = 999; print.newline("unreachable"); }
        fun six(a:int,b:int,c:int,d:int,e:int,f:int) r:int { r = a+b+c+d+e+f; }
        fun mark(a:int) r:int { print.sameline(a); a =+ 10; r = a; }
    "#,
    );
    let output = project.run();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, "30\r\n300\r\n30\r\n20\r\n30\r\n14\r\n20\r\n2\r\n-3\r\n-3\r\n3\r\n0\r\n100\r\n150\r\n40\r\n5\r\n30\r\n50\r\n18\r\n21\r\n1223\r\n5\r\n10\r\n-9223372036854775808\r\n9223372036854775807\r\n0\r\nŻółw\r\na\0b".as_bytes());
}

#[test]
#[ignore = "requires NASM and Visual Studio C++ build tools"]
fn native_runtime_errors_are_reported() {
    for body in [
        "print.newline(1/0);",
        "print.newline(9223372036854775807+1:i64);",
        "print.newline(-9223372036854775808-1:i64);",
        "print.newline(9223372036854775807*2:i64);",
        "print.newline(-9223372036854775808/-1:i64);",
        "var a = -9223372036854775808:i64; print.newline(-a);",
        "var a = 10; a.clamp(100,0);",
        "var a = 127:i8; a =+ 1;",
        "var a = 15:u4; a =+ 1;",
        "var a = 0:u64; a =- 1;",
        "var a = 18446744073709551615:u64; a =* 2;",
        "var a = 300; var b = a:u8;",
        "var a = 1.5:f32; a =/ 0;",
        "var a = 1e38:f32; a =* 10;",
        "var a = 1e4000:f128; a =* a;",
        "var a = 1.5:f128; a.clamp(2,1);",
    ] {
        let output = Project::new(&format!("fun main() {{ {body} }}")).run();
        assert_eq!(output.status.code(), Some(2), "{body}: {output:?}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("Adamantium runtime error"));
        assert!(output.stdout.is_empty());
    }
}

#[test]
#[ignore = "requires NASM and Visual Studio C++ build tools"]
fn native_empty_function_and_large_stack_frame() {
    assert!(Project::new("fun main() {}").run().status.success());
    let mut source = String::from("fun main() {");
    for i in 0..700 {
        source.push_str(&format!("var value{i} = {i};"));
    }
    source.push_str("print.newline(value0+value699); }");
    let output = Project::new(&source).run();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, b"699\r\n");

    let arguments = (0..700)
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let parameters = (0..700)
        .map(|i| format!("value{i}:int"))
        .collect::<Vec<_>>()
        .join(",");
    let source = format!(
        "fun main() {{ print.newline(many({arguments})); }} fun many({parameters}) r:int {{ r = value0+value699; }}"
    );
    let output = Project::new(&source).run();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, b"699\r\n");
}
