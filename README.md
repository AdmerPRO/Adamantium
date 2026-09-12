# Adamantium Compiler

A Rust compiler that translates Adamantium source code into NASM assembly and
builds native Windows or Linux x86-64 console executables.

## Requirements

- Rust and Cargo with edition 2024 support
- NASM on PATH or installed in `%ProgramFiles%\NASM`
- Windows: Visual Studio C++ build tools and the Windows SDK
- Linux: a C linker available as `cc`

## Portable Windows package

The `Portable Windows package` GitHub Actions workflow creates
`adamantium-windows-x86_64.zip`. It contains a release build of
`adamantium.exe` and NASM 3.02, so users do not need Rust or a separate NASM
installation. Extract the archive, add its directory to `PATH`, and run
`adamantium --version`.

The compiler searches for `tools/nasm.exe` next to its executable before
checking the usual NASM installation paths. Visual Studio C++ Build Tools and
the Windows SDK are still required to link generated Adamantium programs. A
future installer can install the same portable directory and update `PATH`.

Maintainers can produce the ZIP locally from PowerShell:

```powershell
./scripts/package-windows.ps1
```

The packaging script downloads the official NASM archive, verifies its pinned
SHA-256 checksum, and includes its BSD 2-Clause license.

## Usage

Install the CLI from the `compiler` directory:

```bat
cargo install --path .
```

Then change into an Adamantium project directory and build or run it:

```bat
adamantium build
adamantium run
adamantium check
adamantium clean
adamantium clear
adamantium new MyProject
adamantium new "C:\path\to\MyProject"
```

Both commands accept an optional project directory, for example
`adamantium run "C:\path\to\project"`. Without one they use the current
directory. `adamantium PROJECT_DIRECTORY` remains an alias for
`adamantium build PROJECT_DIRECTORY`. Run `adamantium --help` or
`adamantium --version` for CLI information.

Use `adamantium clean [PROJECT_DIRECTORY]` to remove the project's generated
`target` directory. `adamantium clear` is an exact alias. Both commands validate
the project first and refuse to recursively follow a symbolic `target` path.

Use `adamantium check [PROJECT_DIRECTORY]` to validate project metadata, imports,
syntax, symbol access, aliases, classes, enums and types without invoking NASM or
the linker. It reports unused variables/functions and unreachable code as warnings,
creates no build artifacts, and returns a nonzero status when analysis fails.

### Tests

Put tests in `code/tests.ad` and annotate parameterless functions with `#[test]`:

```adamantium
#[test]
fun addition() {
    if 2 + 2 != 4 then {
        panic("addition failed");
    }
}
```

Use `adamantium test list [PROJECT_DIRECTORY]` to discover tests and
`adamantium test run [PROJECT_DIRECTORY] [TEST_NAME]` to run all tests or one
exactly named test. Add `--verbose` to show output from passing tests. Every test
runs in its own executable process. The summary reports passed and failed tests,
and the command exits with code `1` when any test fails, which is suitable for CI.

### Program arguments

Parameters declared by `main` are populated from named command-line arguments. Missing required
arguments and unknown arguments produce warnings. Invalid values stop the program with an error.
Prefix a parameter with `$` to make it optional:

```adamantium
fun main(number:int,$message:string,$enabled:bool) {
    print.newline(number);
    print.newline(message);
    print.newline(enabled);
}
```

```text
adamantium run --number 7 --message "Hello" --enabled true
adamantium run path/to/project --number 7
```

The built executable accepts the same arguments directly. Supported argument types are integers,
floating-point values, strings and booleans (`true` or `false`). Omitted optional values are
passed to `main` as `None`.

The compiler reads `code/main.ad`, `project.toml` and `requirement.toml`
from the project directory. It writes assembly, an object file and the native
executable into that project's `target` directory, where `name` comes
from `project.toml`. On Windows, the compiler finds Visual Studio with
`vswhere` and configures the x64 linker automatically, so these commands work
from regular PowerShell and Command Prompt sessions. On Linux it generates an
ELF64 object with NASM and links it through `cc`.

### WASM packages

See [Creating Adamantium packages](docs/CREATING_PACKAGES.md) for the complete
author guide, including repository layout, Rust WASM builds, GitHub Releases,
automated publishing, installation, versioning, and troubleshooting.

Declare Adamantium WASM packages in `requirement.toml`:

```toml
[packages]
"https://github.com/AdmerPRO/Math" = "1.0.0"
```

Install them with `adamantium install [PROJECT_DIRECTORY]`. Version `1.0.0`
selects the GitHub release tag `adamantium_packet_1_0_0` and downloads its
`adamantium_packet.wasm` asset into
`packages/Math/1.0.0/adamantium_packet.wasm`. Sources can use any public GitHub
repository. Packages outside the `AdmerPRO` and `AdamantiumORG` organizations
are identified as community packages and are not controlled by Adamantium.
Versions must use `MAJOR.MINOR.PATCH`, downloads must remain on HTTPS, and files
are checked for a valid WebAssembly header. Set `ADAMANTIUM_CURL` only when a
custom compatible downloader is required.

Package installation is implemented; importing or executing functions from the
downloaded WASM module is not implemented yet.

### Modules, pack, and use

Adamantium projects may contain multiple `.ad` files under `code`. Load a module
with `pack`; the module path is relative to `code` and omits the `.ad` extension.

```text
// code/main.ad
pack utils;
pack utils/tools;

fun main() {
    utils:calculate();
    utils/tools:run_tool();
}
```

This loads `code/utils.ad` and `code/utils/tools.ad`. A loaded module keeps its
own namespace. Use a colon to access its functions, enums, and classes. To bring
selected symbols into the current module, use a list:

```text
pack utils;
use utils:[calculate,Status,Counter];

fun main() {
    calculate();
    var status = Status.ready;
    var counter = Counter(value=1);
}
```

Every module explicitly declares its own `pack` dependencies and `use` imports.
The compiler reports missing module files, unknown imported symbols, conflicting
imports, invalid module paths, and circular `pack` dependencies.

### Lists

Lists contain values of one type. Their element type is inferred from all values; compatible
numeric values are promoted to a shared type. Empty or incompatible lists require an explicit
type:

```adamantium
var numbers = List[1, 2, 3];
numbers[1] = 10;
print.newline(numbers[1]);

var empty = List[]:List[int];
```

Indexing is checked at runtime. Assigning a list to another variable creates an independent copy.
Lists can be nested and can be used as optional function parameters or class fields. Runtime type
identifiers preserve each `List` and optional wrapper independently.

Set `ADAMANTIUM_NASM` or `ADAMANTIUM_LINKER` to override a tool's executable
path. A custom linker must support Microsoft LINK arguments and have access to
`kernel32.lib` through its environment.

A complete runnable project covering every currently supported language feature
is available in [`example-project`](example-project). Run it from this directory
with `cargo run -- run example-project`.

## Supported syntax

```text
fun main() {
    // Print without a line ending, then finish the line.
    print.sameline("Hello, ");
    print.newline("Adamantium!");
    var a = 10;
    var b = 20;
    b =+ a;
    b.clamp(0,100);
    var result = add(a,b);
    print.newline(result);
}

fun add(a:int,b:int) r:int {
    r = a+b;
}
```

The program requires exactly one parameterless `main` function and can declare
additional functions with typed parameters and a named result.
Single-line (`//`) and block (`/* ... */`) comments are supported, along with
string literals with UTF-8 text and escapes.
`print.newline` appends CRLF; `print.sameline` adds no line ending.
Declare variables with `var a = 10;` and print them with `print.newline(a);` or
`print.sameline(a);`. Names are case-sensitive, start with an ASCII letter or
underscore, and may then contain digits. Declare each name once before using it.
Keywords, mutability modifiers, and type names are reserved words.

`exit();` immediately ends the program with exit code `0` and writes nothing to
stdout or stderr. Code after it is unreachable. Classes can be printed directly;
their name and public fields are shown as `MyClass(field=value)`. Private fields
are omitted, and lists use `[value, value]` formatting.

### Types and inference

| Family | Supported types | Default / alias |
| --- | --- | --- |
| Signed integers | `i8`, `i16`, `i32`, `i64` | `int` means `i32` |
| Unsigned integers | `u4`, `u8`, `u16`, `u32`, `u64` | `u` means `u32` |
| Floating point | `f32`, `f64`, `f128` | `float` means `f64` |
| Text | `string` | Inferred from a quoted string |
| Boolean | `bool` | Inferred from `true` or `false` |
| No value | `None` | Inferred from `None` |

```text
var a = 10;                 // i32
var b = 10:i16;              // Explicit type after the initializer.
var c = 10:int;              // i32
var d = 10:u;                // u32
var small = 15:u4;           // Range: 0 through 15.
var large = 18446744073709551615:u64;
var fraction = 1.5;          // f64
var single = 1.5:f32;
var precise = 1e4000:f128;
var text = "Hello":string;
var enabled = true:bool;
var missing = None;
```

An unannotated integer literal defaults to `i32`, including in print arguments.
Use an explicit suffix for values outside its range. Copies and function results
retain their source type. A suffix applies to the whole preceding expression;
use `(expression:type)` to annotate an operand inside a larger expression.
An explicit destination type supplies the numeric context for its initializer.

Each variable keeps its declared or inferred type on subsequent assignments.
Integer literals outside the selected range are compile errors. Numeric values
computed at runtime are checked too: overflow or an out-of-range conversion exits
with code 2. `u4` is an unsigned 4-bit value (0–15), not a packed storage layout.

Numeric destinations permit checked integer conversions, integer-to-float
conversion, and float-width conversion. Float-to-integer conversion, conversions
between numbers and `bool`/`string`/`None`, and string arithmetic are rejected.
Mixed numeric expressions use a common numeric type; an integer/float expression
uses the floating-point type. Signed/unsigned combinations with no common integer
type are rejected. An explicit destination can supply a common numeric context.

Floating-point values use IEEE binary32, binary64, or binary128 semantics, with
round-to-nearest, ties-to-even. `f128` uses software arithmetic and is not an alias
for `f64`. Overflow and division by zero are runtime errors; underflow follows
IEEE rounding. Float literals accept decimal fractions and scientific notation.

Strings contain UTF-8 text and support `\n`, `\r`, `\t`, `\0`, `\"`, and `\\`.
String variables can be copied, reassigned, passed to functions, and returned.
Their contents are immutable literals; concatenation and indexing are not yet
supported. `bool` prints `true` or `false`; `None` prints `None`. A variable of
type `None` can only hold `None`; optional parameters and fields are described below.

### Safe offsets

An offset refers to the storage of a local variable. Reading through it returns
the variable's current value and preserves its type:

```adamantium
var source = 10;
var address = source.offset;
source = 20;
var copy = address.by_offset; // 20
```

`source.get_offset()` is an alias for `source.offset`, and
`address.value_by_offset` is an alias for `address.by_offset`. The retrieved
value is a copy, so changing `copy` does not change `source`.

Offsets cannot be printed, stored in lists, exposed through function or class
signatures, or dereferenced after their target is removed. These rules keep an
offset inside the stack frame where its target remains valid.

Type aliases use `define` at file level before their first use:

```text
define Number = int;
define Numbers = List[Number];
```

An alias resolves directly to its original type and does not create a distinct runtime type.
Aliases can refer to built-in types, enums, classes, lists, or an earlier alias. They can also
be imported from another module with `use`.

Enums are declared at file level before their first use. Variants are separated
with commas, and a trailing comma is optional:

```text
enum MyTable {
    option,
    second_option
}

fun main() {
    var value = MyTable.option;
    value = MyTable.second_option;
    print.newline(value); // Prints the variant index: 1.
}
```

Each enum is a distinct type. Enum values can be assigned, passed to functions,
returned, and printed. Arithmetic and `clamp` are not supported for enums.
Enums are private to their module by default. Add `pub` to export one:

```text
pub enum State { ready, stopped }
```

Other modules may then import it with `use` or access it as `module:State.variant`. Private enum
access from another module is a compile error. Enum variants can be used as `match` patterns.

Top-level functions and classes follow the same visibility rules. They are private
to their module by default; `priv` makes that choice explicit and `pub` exports
them for qualified access or `use` imports:

```adamantium
pub fun calculate(value:int) result:int {
    result = value * 2;
}

priv fun helper() result:None {}

pub class Counter(pub value:int) {
    fun __new__() {}
}
```

`main` remains the private executable entry point and never requires `pub`.

### Generics

Generic functions and classes declare type parameters between `<` and `>`. Calls and object
construction provide concrete types explicitly:

```text
fun identity<T>(value:T) result:T {
    result = value;
}

class Box<T>(pub value:T) {
    fun __new__() {}
}

var number = identity<int>(7);
var box = Box<string>(value="hello");
```

Multiple parameters are supported. Generic parameters can appear inside `List`, such as
`List[T]`. Available constraints are `any`, `numeric`, `integer`, `float`, and `comparable`:

```text
fun twice<T:numeric>(value:T) result:T {
    result = value + value;
}
```

Each used combination of concrete types creates a specialized function or class during
compilation. Generic declarations therefore add no dynamic type dispatch to generated code.

### Traits

A trait declares public methods that implementing classes must provide:

```text
trait Printable {
    fun render(prefix:string) result:string;
}

class Document(pub text:string) implements Printable {
    fun __new__() {}

    pub fun render(prefix:string) result:string {
        result = self.text;
    }
}
```

Trait requirements contain signatures followed by semicolons and do not contain method bodies.
The compiler checks that every required method exists, is public, and has identical parameter and
result types. A class can implement multiple traits by separating their names with commas.

Trait names can be used as generic constraints:

```text
fun keep<T:Printable>(value:T) result:T {
    result = value;
}

var document = Document(text="hello");
var kept = keep<Document>(document);
```

Supplying a class that does not declare `implements Printable` is a compile error. Traits are
compile-time requirements and add no runtime dispatch or object metadata.

### Classes

Classes declare typed fields in parentheses and methods in their body. Every
class must define a parameterless `__new__` method. Object creation initializes
every field from a named argument and then calls `__new__` automatically:

```text
class MyClass(
    pub value:int,
    pub other:int
) {
    fun __new__() {
        print.newline("New object");
    }

    pub fun set_value(new_value:int) result:int {
        self.value = new_value;
        result = self.value;
    }
}

fun main() {
    var first = MyClass(value=1,other=2);
    first.set_value(10);
    print.newline(first.value);

    var second = first;
    second.value = 20;
    print.newline(first.value);  // 10
    print.newline(second.value); // 20
}
```

Fields and methods are private unless marked `pub`; private members are available
through `self` inside the same class. Classes are private to their module by
default. `pub class` exports a class, while `priv class` states the default
explicitly. Class declarations belong to module scope, fields and methods belong
to their class scope, and `self` is available only inside methods of that class.
Code outside the owning class cannot access its private fields or methods, and
code in another module cannot construct a private class. Class values use copy semantics, including
assignments and ordinary function arguments. Methods modify their receiver.
All fields are currently required, direct printing of an object is unsupported,
and class-typed fields are reserved until independent deep copies are implemented.
Lifecycle hooks are private and take no parameters. Construction initializes fields and then calls
`__new__`. A successful field assignment calls `__change__` after storing the new value. Removing
an object with `object.remove;` calls `__remove__` before the name disappears. The compiler rejects
direct field mutation from `__change__` and recursive object removal from `__remove__`.

### Comments

```text
// A single-line comment.
/* A comment that spans
   multiple lines. */
var a = 10 /* inline comment */ + 2;
```

Block comments end at the first `*/` and do not nest. An unclosed comment reports
the line and column of its opening `/*`. Comment delimiters inside strings are
ordinary text.

### Changeable and static variables

```text
var a = 10;         // Changeable by default.
var ch b = 10;      // Explicitly changeable (ch = changeable).
var static c = 10;  // Cannot be modified after initialization.
var stc d = 10;     // Short form of static.
a = 20;
b =+ 5;
print.newline(c);
variable changeable score = 10:i32; // Full aliases of var and ch.
```

`static` and `stc` prevent reassignment, compound assignment, and `clamp` on that
variable. Its initializer can be an expression or a function call. Here `static`
means immutable, not shared storage: the variable remains local to each function
call. Passing its value to a function does not make the parameter immutable.
`static`, `stc`, and `ch` are reserved words, and a declaration accepts at most
one of these modifiers.
`variable` is an exact alias of `var`, and `changeable` is an exact alias of `ch`.
Both forms work with type suffixes, `static`, and `stc`.

An active variable name cannot be declared again in the same function. `remove` deletes the
name, after which it cannot be read or assigned until it is declared again. The new declaration
creates an independent variable:

```text
var value = 10;
value.remove;
var value = 20;
```

Removing one name does not remove aliases that still refer to the same storage.

### Variable and symbol aliases

`as_variable` creates an alias. For ordinary values, both names share the same
storage until the alias is disconnected:

```text
var a = 10;
var b = a.as_variable;
a = 20;          // b is now 20.
b = 15;          // a is now 15.
b.disconect;     // `disconnect` is also accepted.
a = 25;          // b remains 15.
```

Scalar aliases may be disconnected. Enum and class value aliases cannot be
disconnected. Functions, enum types, and class types can also be aliased as
symbols. A symbol alias may be redirected to another symbol, even one of a
different kind or function signature, but it cannot be disconnected.

```text
var operation = add().as_variable;
print.newline(operation(2,3));
operation = twice().as_variable;
print.newline(operation(6));

var StatusAlias = Status.as_variable;
var state = StatusAlias.ready;
var CounterAlias = Counter.as_variable;
var counter = CounterAlias(value=1,step=2);
```

A function value can also use the shorter form. The empty parentheses identify the function;
arguments are supplied when the variable is called:

```adamantium
var operation = add();
print.newline(operation(2,3));
```

### Arithmetic and assignment

Classes can overload arithmetic and comparison operators with public special methods. Each method
accepts exactly one required value of the same class. Comparison and equality methods must return
`bool`:

```adamantium
class Number(pub value:int) {
    fun __new__() {}

    pub fun __add__(other:Number) result:int {
        result = self.value + other.value;
    }

    pub fun __eq__(other:Number) result:bool {
        result = self.value == other.value;
    }
}
```

The supported names are `__add__`, `__sub__`, `__mul__`, `__div__`, `__eq__`, `__ne__`,
`__lt__`, `__le__`, `__gt__`, and `__ge__`.

Use `.as(Type)` for an explicit checked numeric conversion:

```adamantium
var wide = 300:i32;
var small = wide.as(i16);
var decimal = small.as(f64);
```

Conversions that overflow at runtime stop the program with an error. Incompatible conversions are
rejected by the compiler.

Initializers, assignments, print arguments, and function arguments accept numeric
expressions. `*`, `/`, and `%` take precedence over `+` and `-`; operators at the same
precedence are evaluated left to right. Parentheses override precedence.
Division truncates toward zero, so `-7/2` is `-3`.
For integers, `/` returns the quotient without the remainder and `%` returns the remainder.

```text
var a = 10;
var b = 20;
b =+ a;        // b = b + a
b =- a;        // b = b - a
b =* a;        // b = b * a
b =/ a;        // b = b / a
b = a + b;
b = (a + 2) * 3;
b = -5;        // Assign a negative value (space after =).
b = (-5);      // Parentheses also distinguish this from =-.
```

Compound operators are written together: `b =- 5;` subtracts 5, whereas
`b = -5;` assigns -5. Their right-hand side can be a full expression.
All calculations and function calls execute in the generated EXE.

### Conditions and loops

Comparisons use `==`, `!=`, `<`, `<=`, `>`, and `>=`. Conditions must have the
`bool` type. `if` uses the `then` keyword, and `else` is optional:

```text
if score >= 100 then {
    print.newline("Complete");
} else {
    print.newline("Keep going");
}
```

Adamantium supports `while`, `until`, exclusive integer `for` ranges, List
iteration, and infinite `loop` blocks. `until` executes its body while its
condition is false.
Both `break` and `continue` may be used inside any loop.

Boolean expressions support `!` or `not`, `&&` or `and`, and `||` or `or`.
`&&` and `||` use short-circuit evaluation, so the right side is evaluated only
when it can affect the result.

```text
var value = 0;
while value < 2 {
    value =+ 1;
}
until value >= 4 {
    value =+ 1;
}
for index in 0..10 {
    print.newline(index); // Prints 0 through 9.
}
var values = List[10, 20, 30];
for item in values {
    print.newline(item);
}
loop {
    if value == 4 then { break; }
    continue;
}
```

`match` selects the first branch whose pattern equals the matched value. Patterns
must have the same type as the value. The optional `_` fallback must be last.

```text
match status {
    Status.ready => { print.newline("Ready"); }
    Status.finished => { print.newline("Finished"); }
    _ => { print.newline("Another status"); }
}
```

### Creating a project

`adamantium new MyProject` creates `MyProject` inside the current directory.
A relative or absolute path creates the project at that exact path. The command
creates `project.toml`, `requirement.toml`, `code/main.ad`, `target`, and
`.gitignore`. It refuses to overwrite an existing path.

### Clamp

```text
var a = 150;
a.clamp(0,100);
print.newline(a); // 100
a = 150;
print.newline(a); // 150: clamp does not restrict future assignments.
```

`clamp` changes the current value once, using inclusive lower and upper bounds.
Bounds can be numeric expressions and are evaluated left to right. The bounds
must be compatible with the variable's type; `clamp` rejects nonnumeric variables.

### Optional values

Prefix a function parameter name with `$` to make it optional. Prefix a class
field name with `&` for the same behavior. An omitted value becomes `None`, while
a supplied zero remains distinct from `None`.

```text
fun show($value:int) result:None {
    print.newline(value);
}

class Options(pub &value:int) {
    fun __new__() {}
}

show();       // Prints None.
show(0);      // Prints 0.
var options = Options();
print.newline(options.value); // None
```

Required function parameters must come before optional ones. Strings and `f128` values preserve
their complete value when optional. Optional nested class values can be omitted or accessed like
ordinary objects; accessing a field or method through `None` produces a runtime error.
`List` values can be optional and nested.

### Panic and warnings

Use `try { ... }` when a runtime failure should become a value instead of
terminating the program. It returns `None` after a successful block and an
error string after a failed block. Execution resumes after the block, and
statements after the failure inside that block are skipped.

```adamantium
var error = try {
    var value = 10 / 0;
};
print.newline(error);
```

`warn("message");` writes a warning and its source line to stderr, then continues.
`panic("message");` writes the panic and source line to stderr, then immediately
terminates the program with exit code `2`.

### Functions and named results

```text
fun add(a:int,b:int) r:int {
    r = a+b;
    return r; // Optional: exit immediately with the named result.
}
```

The result variable is declared by the function signature. Assign it before
returning; do not redeclare it with `var`. Reaching the closing brace implicitly
returns that variable. `return r;` exits early and always refers to the result
name declared in the signature; arbitrary return expressions are not supported.

Functions can be declared before or after `main`, called from other functions,
called recursively, and nested inside expressions. Arguments are evaluated left to right and passed
by value. Each call has its own local variables; changing a parameter does not
change the caller's variable. Parameters and results accept every type listed
above, including strings, booleans, and `f128`. A `None` result starts as `None`,
so `fun notify() result:None { print.newline("Done"); }` needs no assignment.
`main` cannot be called by another function.

Division by zero, signed arithmetic overflow, and reversed clamp bounds stop the
EXE with a diagnostic on stderr and exit code 2. Output failures use exit code 1.

The compiler's existing project metadata and package manifest format is unchanged.

## Development

### Compiler diagnostics

Compiler errors use category codes such as `E100` for syntax, `E200` for
name/access resolution, `E300` for types, `E400` for project configuration and
`E500` for the native toolchain. Source errors include the affected line and a
caret, followed by context and an actionable suggestion when one is available.
CLI command typos provide a `Did you mean ...?` hint. Independent manifest
problems are reported together instead of stopping after the first invalid field.

Warnings include the source path, line, column, and a stable code. They are written
to stderr and do not block a build:

| Code | Meaning |
| --- | --- |
| `W001` | Unreachable code after an unconditional `return` (one warning per trailing block) |
| `W002` | A local variable or parameter is never read in reachable statements |
| `W003` | A function cannot be reached through calls starting at `main` |

Assignments alone do not count as reads. The named result is implicitly read when
the function returns. Reads and calls after `return` do not suppress unused
warnings. Disconnected recursive groups count as unused functions. Prefix an
intentionally unused variable, parameter, or function name with `_` to suppress
its unused warning. Initializers and other side effects are still executed.

Invalid access is a compile error: undeclared or uninitialized variables, writes
to static variables, calls on variable values, unsupported fields/indexing/qualified
names, unknown methods, and nonnumeric `clamp` operations are rejected.

`use` and `pack` produce an explicit invalid-import error at the directive. Modules
and imports are not implemented yet, so there are currently no resolvable imports.
Checks for missing module files, private exports, and duplicate imports remain
part of the future module system. These checks do not add `pub`/`priv` or memory
access support. Unreachable code is still parsed and type-checked.

Run these commands from this directory:

```bat
cargo build --release
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo fmt --manifest-path runtime/Cargo.toml --check
cargo clippy --manifest-path runtime/Cargo.toml --all-targets -- -D warnings
```

From the Windows x64 developer environment with NASM installed, also run the
native regression tests. They compile temporary projects and verify EXE output
and runtime failures:

```bat
cargo test --locked --test native -- --ignored
```

On Linux x86-64 with NASM and `cc`, run the Linux backend regression test:

```sh
cargo test --locked --test native_linux -- --ignored
```

The release compiler is generated at `target\release\adamantium.exe`.
`src/main.rs` handles configuration and external tools; `src/syntax.rs`
handles parsing and name checks. `src/typed.rs` checks types and conversions.
`src/codegen.rs` generates NASM assembly. `src/runtime.asm` and
`src/runtime-linux.asm` provide platform entry points. `runtime/` contains
shared type semantics, software floating point, and
output helpers. Type tests run on every CI platform; native regression tests
live in `tests/native.rs`.

On Windows and Linux x86-64, `build.rs` builds and embeds a Rust static runtime
library into the compiler. Compiling a project writes this library into its
`target` directory and links it alongside the NASM object and system libraries.
Windows uses the MSVC linker and Windows SDK. Linux generates ELF64 and uses
`cc`. macOS currently supports compiler checks but has no native program backend.

## Continuous integration

GitHub Actions runs on every push and pull request using Ubuntu, macOS, and
Windows runners. Each runner checks formatting and compilation, checks spelling
with Typos, runs Clippy with warnings treated as errors, runs the tests, and
checks the compiler's `--help` command. The final step is `cargo build --locked
--release`.

Clippy provides Rust linting; Flake8 is intended for Python and is not used here.
These checks build and test the compiler on all three systems. Windows CI
installs NASM, configures MSVC, and runs the native EXE regressions. Linux CI
installs NASM and runs a generated ELF64 executable. macOS runs compiler-only
checks until its native backend is implemented.

Separate Windows and Linux CLI workflows install `adamantium` with
`cargo install`, create a fresh project, and test `check`, `build`, `run`,
`test list`, `test run`, `clean`, and `clear` using the installed command.

See [`docs/TARGETS.md`](docs/TARGETS.md) for supported targets and the future
ARM64 contract.

See [`docs/MEMORY_SAFETY.md`](docs/MEMORY_SAFETY.md) for ownership, alias,
offset, object lifetime, and lifecycle-hook rules.
