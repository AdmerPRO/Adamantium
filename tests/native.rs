#![cfg(windows)]

use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

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
        let build = Command::new(env!("CARGO_BIN_EXE_adamantium-compiler"))
            .arg(&self.0)
            .output()
            .unwrap();
        assert!(
            build.status.success(),
            "compiler failed:\n{}\n{}",
            String::from_utf8_lossy(&build.stdout),
            String::from_utf8_lossy(&build.stderr)
        );
        Command::new(self.0.join("target/NativeTest.exe"))
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
#[ignore = "requires NASM and the Visual Studio x64 developer environment"]
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
            print.newline(-9223372036854775808);
            print.newline(9223372036854775807);
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
#[ignore = "requires NASM and the Visual Studio x64 developer environment"]
fn native_runtime_errors_are_reported() {
    for body in [
        "print.newline(1/0);",
        "print.newline(9223372036854775807+1);",
        "print.newline(-9223372036854775808-1);",
        "print.newline(9223372036854775807*2);",
        "print.newline(-9223372036854775808/-1);",
        "var a = -9223372036854775808; print.newline(-a);",
        "var a = 10; a.clamp(100,0);",
    ] {
        let output = Project::new(&format!("fun main() {{ {body} }}")).run();
        assert_eq!(output.status.code(), Some(2), "{body}: {output:?}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("Adamantium runtime error"));
        assert!(output.stdout.is_empty());
    }
}

#[test]
#[ignore = "requires NASM and the Visual Studio x64 developer environment"]
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
