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
fn native_optional_enums_methods_and_matching() {
    let output = Project::new(
        r#"
        enum Choice { first, second }
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
        enum Choice { first, second }
        class Box(pub value:int) { fun __new__() {} }
        fun add(a:int,b:int) result:int { result=a+b; }
    "#,
    )
    .unwrap();
    fs::write(
        project.0.join("code/utils/tools.ad"),
        r#"
        pack utils;
        use utils:[add];
        fun calculate(value:int) result:int { result=add(value,10); }
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
