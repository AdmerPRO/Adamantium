# Adamantium — TODO

---

## 1. Project Foundation

* [x] Create the `adamantium` CLI — `build` and `run` with an optional project directory
* [x] Add version information
* [x] Add `--help`
* [x] Add `--version`
* [x] Add compiler error handling
* [x] Add compiler warning system — nonfatal warnings with codes and source locations
* [x] Create basic documentation structure

---

## 2. Project Structure

Implement the standard Adamantium project structure:

```text
MyProject/
├── project.toml
├── requirements.toml
├── code/
│   ├── main.ad
│   ├── utils.ad
│   └── tests.ad
└── target/
```

* [x] Implement `project.toml`
* [ ] Implement `requirements.toml`
* [x] Implement `/code`
* [x] Implement `/target`
* [x] Validate project structure — required files and metadata; `target` is created when needed
* [x] Detect invalid project structures — missing source/manifests and invalid TOML/metadata
* [x] Add project name
* [x] Add project version
* [x] Add project description
* [x] Add project authors

---

# 3. Compiler Pipeline

Implement:

```text
Adamantium
    ↓
Lexer
    ↓
Parser
    ↓
AST
    ↓
Name Resolution
    ↓
Type Checker
    ↓
Semantic Analysis
    ↓
Code Generation
    ↓
NASM Assembly
    ↓
Object File
    ↓
Linker
    ↓
Executable
```

* [x] Implement lexer
* [x] Implement parser
* [x] Implement AST
* [x] Implement name resolution
* [x] Implement scope resolution — function-local variables; nested block scopes are pending
* [x] Implement type checking
* [x] Implement semantic analysis — names, arity, initialization, named returns, and mutability
* [x] Implement unreachable-code detection — statements after unconditional `return`
* [x] Implement unused-variable detection — reachable reads of locals and parameters
* [x] Implement unused-function detection — call graph rooted at `main`, including disconnected cycles
* [x] Implement invalid-access detection — current names, mutability, calls, methods and unsupported member/index/qualified access
* [x] Implement invalid-import detection — missing modules/symbols, conflicts, paths and dependency cycles
* [x] Implement code generation
* [x] Implement NASM backend
* [x] Implement object file generation
* [x] Implement linker integration
* [x] Implement executable generation

---

# 4. Basic Syntax

* [x] Implement statements
* [x] Implement expressions — typed numeric arithmetic and function calls
* [x] Implement blocks — function and control-flow bodies
* [x] Implement semicolons
* [x] Implement identifiers
* [x] Implement literals — integers, floats, strings, booleans, and `None`
* [x] Implement function calls
* [x] Implement operators — numeric `+`, `-`, `*`, `/`, `%`, comparisons, boolean operators and compound assignments
* [x] Implement operator precedence
* [x] Implement parentheses
* [x] Implement `{ }` blocks — function and control-flow bodies

---

# 5. Comments

Single-line comments:

```adamantium
// comment
```

Multi-line comments:

```adamantium
/*
    comment
*/
```

* [x] Implement `//`
* [x] Implement `/* ... */` — non-nested block comments, including multiple lines
* [x] Detect unterminated multi-line comments — report the opening line and column

---

# 6. Variables

Basic syntax:

```adamantium
var a = 10;
```

* [x] Implement `var`
* [x] Implement `variable`
* [x] Implement variable declaration
* [x] Implement variable assignment
* [x] Implement `=+`, `=-`, `=*`, and `=/` compound assignments
* [x] Implement one-time `clamp(min,max)` on changeable numeric variables
* [x] Implement variable scope — one local scope per function call
* [x] Implement variable shadowing rules
* [x] Prevent use of removed variables
* [x] Prevent invalid reassignment
* [x] Implement fixed variable types

---

# 7. Variable Mutability

Default variables are changeable.

```adamantium
var a = 10;
```

Explicit forms:

```adamantium
var ch a = 10;
var stc b = 20;
```

Full forms:

```adamantium
variable changeable a = 10;
variable static b = 20;
```

* [x] Implement `ch`
* [x] Implement `changeable`
* [x] Implement `stc`
* [x] Implement `static`
* [x] Make variables `changeable` by default
* [x] Prevent modification of static variables
* [x] Add compiler diagnostics for invalid modification

---

# 8. Types

Implement:

### Signed integers

* [x] `i8`
* [x] `i16`
* [x] `i32`
* [x] `i64`

### Unsigned integers

* [x] `u4`
* [x] `u8`
* [x] `u16`
* [x] `u32`
* [x] `u64`

### Floating point

* [x] `f32`
* [x] `f64`
* [x] `f128`

### Other types

* [x] `string`
* [x] `bool`
* [x] `None`
* [ ] `offset`
* [x] `List` — homogeneous literals, explicit element types, indexing and element assignment
* [x] `enum` — named variants with distinct inferred types
* [x] `class` — required typed fields, methods, construction and value copying
* [x] `fun` — callable function values through variable aliases

### Type aliases

```adamantium
define Numbers = List[int];
```

* [x] Implement `define`
* [x] Make aliases refer to the original type
* [x] Prevent aliases from creating distinct runtime types

---

# 9. Type Inference

Examples:

```adamantium
var a = 10;       // i32
var b = 10.5;     // f64
var c = "Hello";  // string
```

* [x] Implement integer literal inference
* [x] Implement default `i32`
* [x] Implement float literal inference
* [x] Implement default `f64`
* [x] Implement string inference
* [x] Implement boolean inference
* [x] Implement inferred list types — inspect every element and promote compatible numeric types
* [x] Detect ambiguous types — empty and incompatible lists require an explicit element type

---

# 10. Type Conversion

Implement:

```adamantium
var a = b.as(i32);
```

* [x] Implement `as(Type)`
* [x] Implement explicit numeric conversions — numeric type suffixes and `as(Type)`
* [x] Implement automatic numeric promotion
* [x] Define safe conversion rules
* [x] Define narrowing conversion rules
* [x] Detect invalid conversions
* [x] Preserve type safety

Example:

```adamantium
var b = 10.5:f64;
var c = 5:u32;
var a = b + c;
```

* [x] Automatically promote `c` to `f64`
* [x] Infer `a` as `f64`

---

# 11. Optional Values

Implement optional function parameters and class fields:

```adamantium
fun example($value:int) result:None {}
class Example(&value:int) { ... }
```

Possible value:

```text
None
```

or:

```text
true
```

* [x] Implement `$name:Type` optional function parameters for supported value types
* [x] Implement `&name:Type` optional class fields for supported value types
* [x] Allow omitted optional class fields
* [x] Allow optional function parameters
* [x] Represent missing values as `None`
* [x] Add optional-value type checking
* [x] Prevent arithmetic and other unsafe access to optional values
* [x] Support optional strings, f128 values and nested class values
* [x] Add a recursive type representation for nested and optional `List` values

---

# 12. Functions

Syntax:

```adamantium
fun add(a:int, b:int) r:int {
    r = a + b;
}
```

* [x] Implement functions
* [x] Implement parameters — supported value types, passed by value
* [x] Implement named return variables
* [x] Implement return types — supported value types and `None`
* [x] Implement implicit final return
* [x] Implement `return` — `return <named-result>;` only
* [x] Implement early returns
* [x] Implement `None` return type
* [x] Implement function calls
* [x] Implement recursion
* [x] Implement function values — `var callable = function();` and explicit `as_variable`

---

# 13. `main`

Every executable project has:

```adamantium
fun main(...) {
}
```

* [x] Detect `main`
* [x] Validate `main`
* [x] Generate executable entry point
* [x] Implement command-line arguments — named `--name value` pairs
* [x] Map CLI arguments to `main` parameters
* [x] Warn about missing required arguments
* [x] Warn about unknown arguments
* [x] Error on invalid argument types
* [x] Implement optional `$` arguments

Example:

```text
adamantium run --numberone 1 --numbertwo 2
```

---

# 14. Visibility

Implement:

```adamantium
pub
priv
```

* [ ] Implement private functions by default
* [ ] Implement `pub`
* [ ] Implement `priv`
* [ ] Implement public classes
* [x] Implement public enums
* [ ] Prevent access to private symbols from other modules
* [x] Make `main` special and not require `pub`

---

# 15. Modules and `pack`

Implement:

```adamantium
pack utils;
```

* [x] Implement `pack`
* [x] Find explicitly packed `.ad` files
* [x] Resolve module paths relative to `code`
* [x] Implement nested module paths such as `utils/tools`
* [x] Implement module namespaces and `module:symbol` access
* [x] Detect circular module dependencies
* [x] Detect missing modules

Example:

```adamantium
pack tools.utils;
```

---

# 16. `use`

Implement:

```adamantium
use utils::funkcja;
```

Multiple imports:

```adamantium
use utils::[dodawanie, odejmowanie];
```

* [x] Implement imports
* [x] Implement multiple imports with `module:[a,b]`
* [x] Implement qualified module access with `module:symbol`
* [ ] Import only public symbols — top-level visibility is pending
* [x] Detect duplicate and conflicting imports
* [x] Detect missing symbols

---

# 17. Classes

Implement:

```adamantium
class Player (
    name: str,
    hp: i32
) {
    ...
}
```

* [x] Implement class declarations
* [x] Implement fields — required named initialization; class-typed fields pending deep copy support
* [x] Implement field types
* [x] Implement methods
* [x] Implement `self`
* [ ] Implement class visibility
* [x] Implement object creation — named field arguments with value-copy semantics
* [x] Implement object field access
* [x] Implement object method calls
* [x] Implement optional fields
* [ ] Implement class scope rules

---

# 18. Class Lifecycle

Implement:

```adamantium
__new__
__change__
__remove__
```

* [x] Implement `__new__`
* [x] Automatically call `__new__` on object creation
* [x] Implement `__change__`
* [x] Call `__change__` on class/object changes
* [x] Implement `__remove__`
* [x] Call `__remove__` when an object is removed
* [x] Define lifecycle ordering
* [x] Prevent unsafe recursive lifecycle behavior

---

# 19. Enums

Implement:

```adamantium
enum Direction {
    North,
    South,
    East,
    West
}
```

* [x] Implement enum declarations
* [x] Implement enum variants
* [x] Implement enum values — assignment, function arguments/results and numeric-index printing
* [x] Implement enum comparison — `==` and `!=`
* [x] Implement enum matching
* [x] Implement public enums
* [x] Prevent access to private enums from other modules

---

# 20. Control Flow

## `if`

* [x] Implement `if`
* [x] Implement `then`
* [x] Implement `else`
* [x] Implement nested conditions

Example:

```adamantium
if hp <= 0 then {
    ...
}
```

## `match`

```adamantium
match value {
    1 => ...
    2 => ...
    _ => ...
}
```

* [x] Implement `match`
* [x] Implement literal and enum pattern matching
* [x] Implement wildcard `_`
* [x] Implement enum matching
* [ ] Detect non-exhaustive matches where required
* [x] Detect unreachable branches after wildcard `_`
* [x] Detect duplicate match patterns

---

# 21. Loops

## `for`

```adamantium
for i in 0..10 {
    ...
}
```

* [x] Implement `for`
* [x] Implement exclusive integer ranges
* [ ] Implement iteration over lists
* [ ] Implement iteration over supported collections

## `while`

```adamantium
while hp > 0 {
    ...
}
```

* [x] Implement `while`

## `until`

```adamantium
until ready == true {
    ...
}
```

* [x] Implement `until` — repeat while the condition is false

## `loop`

```adamantium
loop {
    ...
}
```

* [x] Implement infinite loops

## Loop control

* [x] Implement `break`
* [x] Implement `continue`
* [x] Validate loop-only statements

---

# 22. Error Handling

## `try`

```adamantium
var err = try {
    ...
}
```

* [ ] Implement `try`
* [ ] Return `None` on success
* [ ] Return error string on failure
* [ ] Prevent unhandled runtime errors where required

## `panic`

```adamantium
panic("Something went wrong!");
```

* [x] Implement `panic`
* [x] Print panic message
* [x] Print source line
* [x] Terminate program with exit code 2

## `warn`

```adamantium
warn("HP is low!");
```

* [x] Implement runtime warnings
* [x] Print source line
* [x] Continue program execution

---

# 23. Program Exit

Implement:

```adamantium
exit();
```

and:

```adamantium
exit(code = 45);
```

* [x] Implement normal program termination
* [x] Implement exit codes — automatic codes 0, 1, and 2; explicit exit API is pending
* [x] Return exit code to operating system
* [x] Ensure `exit()` produces no error output

---

# 24. Printing

Implement:

```adamantium
print.newline("Hello");
print.sameline("Hello ");
```

* [x] Implement `print.newline`
* [x] Implement `print.sameline`
* [x] Support strings — UTF-8 literals and string variables
* [x] Support numbers — signed/unsigned integers and f32/f64/f128
* [x] Support booleans
* [x] Support objects where appropriate â€” classes print their names and public fields
* [x] Reject invalid `print(...)` syntax

---

# 25. Alias System

Implement:

```adamantium
var a = b.as_variable;
```

* [x] Implement variable aliases with `as_variable`
* [x] Ensure connected aliases share values instead of copying
* [x] Implement alias chains through shared storage
* [x] Implement function, enum and class symbol aliases
* [x] Allow symbol aliases to be redirected to another symbol
* [ ] Implement `get_parent()`
* [ ] Implement `get_root()`
* [ ] Implement `is_alias()`
* [ ] Implement `is_synced()`
* [ ] Implement `alias_of()`
* [ ] Implement `alias_count()`
* [ ] Implement `desync()`
* [ ] Implement `change_only()`
* [ ] Implement `sync()`
* [x] Implement `disconect` and `disconnect` for scalar value aliases
* [ ] Implement `detach()` as a separate API
* [ ] Implement `reattach()`
* [ ] Define alias lifetime rules
* [ ] Prevent alias use-after-lifetime
* [ ] Integrate aliases with memory safety

---

# 26. Memory and Offsets

Implement:

```adamantium
var offset = a.get_offset();
var value = offset.value_by_offset;
```

* [ ] Implement `offset`
* [ ] Implement `get_offset()`
* [ ] Implement `value_by_offset`
* [ ] Preserve original value type where possible
* [ ] Define offset lifetime rules
* [ ] Prevent invalid memory access
* [ ] Prevent use-after-free
* [ ] Integrate offsets with the memory-safety system

---

# 27. Variable Renaming

Implement:

```adamantium
b.changename(a);
```

* [ ] Implement `changename`
* [ ] Change identifier without moving the underlying storage
* [ ] Preserve type
* [ ] Preserve value
* [ ] Preserve memory location
* [ ] Update compiler symbol tables
* [ ] Define behavior with aliases

---

# 28. Decorators

Implement:

```adamantium
#[test]
fun test_add() {
    ...
}
```

* [ ] Implement decorator syntax
* [ ] Implement function decorators
* [ ] Implement multiple decorators
* [ ] Implement decorator execution
* [ ] Implement decorators accepting functions
* [ ] Implement class decorators
* [ ] Apply class decorators to methods
* [ ] Implement decorator exclusion

Example:

```adamantium
#[!log]
pub fun attack() {
}
```

---

# 29. Professional Mode

Configuration:

```toml
professional = true
```

* [ ] Implement `professional` setting
* [ ] Require explicit mutability in professional mode
* [ ] Require explicit concrete types
* [ ] Reject implicit variable declarations where required
* [ ] Reject generic `int` where an exact type is required
* [ ] Improve compiler diagnostics for professional mode

Example:

```adamantium
var stc a = 10:i32;
```

---

# 30. Generics

* [x] Design generic syntax
* [x] Implement generic functions
* [x] Implement generic classes
* [x] Implement generic lists
* [x] Implement generic constraints
* [x] Implement generic type checking
* [x] Implement generic code generation
* [x] Produce useful generic compiler errors

---

# 31. Traits / Interfaces

* [x] Design `trait` syntax
* [x] Implement traits
* [x] Implement trait methods
* [x] Implement trait requirements
* [x] Implement `implements`
* [x] Implement trait type checking
* [x] Implement trait-based generic constraints

---

# 32. Operator Overloading

* [x] Design operator overload syntax — public `__add__`, `__sub__`, `__mul__`, `__div__`, `__eq__`, `__ne__`, `__lt__`, `__le__`, `__gt__`, and `__ge__` methods
* [x] Implement `+`
* [x] Implement `-`
* [x] Implement `*`
* [x] Implement `/`
* [x] Implement comparison operators
* [x] Implement equality operators
* [x] Validate operator implementations
* [x] Prevent unsafe operator behavior — public methods, one required same-class operand, checked result types

---

# 33. Async

Async is an official Adamantium feature provided through:

```toml
[dependencies]
adamantium-async = "1.0"
```

* [ ] Create `adamantium-async`
* [ ] Implement async runtime
* [ ] Implement `async`
* [ ] Implement `await`
* [ ] Implement async functions
* [ ] Implement async return values
* [ ] Implement task spawning
* [ ] Implement task joining
* [ ] Implement async error handling
* [ ] Integrate async with the compiler
* [ ] Integrate async with memory safety
* [ ] Add async documentation
* [ ] Add async tests

---

# 34. File System — `AdamantiumFiles`

Official file-system library.

* [ ] Create `AdamantiumFiles`
* [ ] Implement file opening
* [ ] Implement file reading
* [ ] Implement file writing
* [ ] Implement file appending
* [ ] Implement file creation
* [ ] Implement file deletion
* [ ] Implement file existence checks
* [ ] Implement directory creation
* [ ] Implement directory deletion
* [ ] Implement directory existence checks
* [ ] Implement directory listing
* [ ] Implement file metadata
* [ ] Implement safe file errors
* [ ] Add documentation
* [ ] Add tests

---

# 35. JSON — `AdamantiumJson`

Official JSON library.

* [ ] Create `AdamantiumJson`
* [ ] Implement JSON parsing
* [ ] Implement JSON serialization
* [ ] Implement JSON objects
* [ ] Implement JSON arrays
* [ ] Implement JSON strings
* [ ] Implement JSON numbers
* [ ] Implement JSON booleans
* [ ] Implement JSON `null`
* [ ] Implement JSON type checking
* [ ] Implement JSON file loading
* [ ] Implement JSON file saving
* [ ] Implement malformed JSON errors
* [ ] Add documentation
* [ ] Add tests

---

# 36. Test System

Tests are stored in:

```text
/code/tests.ad
```

Example:

```adamantium
&TestsFile:Parallel[5]
&TestsFile:StopOnFailed:DontStopStarted

#[test]
fun test_add() {
    assert(2 + 2 == 4);
}
```

* [x] Implement `tests.ad`
* [x] Implement `#[test]`
* [x] Discover test functions
* [x] Implement test runner
* [x] Implement test result reporting
* [x] Implement passed tests
* [x] Implement failed tests
* [x] Implement test errors
* [x] Implement test filtering
* [x] Implement isolated test execution where required

---

# 37. Test CLI

Implement:

```text
adamantium test run
adamantium test list
adamantium test run <test_name>
```

* [x] Implement `adamantium test`
* [x] Implement `adamantium test run`
* [x] Implement `adamantium test list`
* [x] Implement running a single test
* [x] Implement test summaries
* [x] Implement exit codes for CI
* [x] Implement readable test output
* [x] Implement verbose test output

---

# 38. Test File Directives

Implement:

```adamantium
&TestsFile:Parallel
```

and:

```adamantium
&TestsFile:Parallel[5]
```

* [ ] Implement `Parallel`
* [ ] Implement `Parallel[n]`
* [ ] Automatically determine parallelism when no limit is specified
* [ ] Limit concurrently running tests
* [ ] Implement `StopOnFailed`
* [ ] Implement `StopOnFailed:DontStopStarted`
* [ ] Stop scheduling new tests after failure
* [ ] Allow already-started tests to finish
* [ ] Wait for started tests before finishing
* [ ] Report all completed results
* [ ] Validate invalid `TestsFile` directives

---

# 39. Assertions

Implement:

```adamantium
assert(value);
```

and:

```adamantium
assert(value, "message");
```

* [ ] Implement `assert`
* [ ] Implement assertion messages
* [ ] Show source line on failure
* [ ] Show expected/actual values where possible
* [ ] Integrate assertions with the test runner

---

# 40. Package / Dependency Manager

* [ ] Design package format
* [ ] Implement `requirements.toml`
* [ ] Implement dependency resolution
* [ ] Implement dependency installation
* [ ] Implement dependency versions
* [ ] Implement dependency locking
* [ ] Implement package cache
* [ ] Implement package publishing
* [ ] Implement package registry
* [ ] Detect dependency conflicts
* [ ] Detect dependency cycles
* [ ] Add `adamantium install`

---

# 41. CLI Commands

Implement:

```text
adamantium run
adamantium build
adamantium check
adamantium install
adamantium test run
adamantium test list
adamantium clean
adamantium clear
adamantium new <project_name_or_path>
```

* [x] Implement `run`
* [x] Implement `build`
* [x] Implement `new`
* [x] Implement `check`
* [ ] Implement `install`
* [x] Implement `test`
* [ ] Implement `clean`
* [ ] Implement `clear`
* [ ] Make `clean` and `clear` aliases
* [x] Add command error handling — unknown options and excess arguments are rejected
* [x] Add command help — top-level help documents the available commands

---

# 42. `adamantium check`

Static analysis should detect:

* [x] Type errors
* [x] Unused variables
* [x] Unused functions
* [x] Unreachable code
* [x] Invalid imports
* [x] Private symbol access
* [x] Missing return values
* [x] Invalid assignments
* [x] Invalid conversions
* [x] Invalid aliases
* [x] Invalid offsets
* [x] Invalid decorators
* [x] Invalid class usage
* [x] Invalid enum usage

---

# 43. Memory Safety

Adamantium must remain memory-safe.

* [ ] Define ownership model
* [ ] Define borrowing/reference rules
* [ ] Define object lifetime rules
* [ ] Define alias lifetime rules
* [ ] Define offset lifetime rules
* [ ] Prevent use-after-free
* [ ] Prevent double-free
* [ ] Prevent invalid memory access
* [ ] Prevent dangling aliases
* [ ] Prevent invalid offsets
* [ ] Validate class lifecycle memory safety
* [ ] Add compiler diagnostics for memory-safety violations
* [ ] Add memory-safety tests

---

# 44. Runtime

* [ ] Design Adamantium runtime
* [x] Implement runtime startup
* [x] Implement runtime shutdown
* [x] Implement printing — strings, integers, floats, booleans, and `None`
* [x] Implement panic handling
* [x] Implement warning handling
* [x] Implement exit codes — normal completion, output failures, arithmetic/range failures
* [ ] Implement memory management
* [x] Implement string runtime — immutable literal storage and value copies; string operations are pending
* [ ] Implement list runtime
* [x] Implement object runtime — allocation and independent class-value copying
* [x] Report arithmetic overflow, division by zero, and invalid clamp ranges
* [ ] Implement error runtime
* [ ] Optimize runtime overhead

---

# 45. NASM Backend

* [x] Generate valid NASM syntax
* [x] Generate functions
* [x] Generate variables — typed local values in stack slots
* [x] Generate arithmetic
* [x] Generate comparisons
* [x] Generate branches
* [x] Generate loops
* [x] Generate function calls
* [x] Generate returns
* [x] Generate classes — runtime-backed field storage, method calls and independent copies
* [ ] Generate lists
* [x] Generate strings — read-only UTF-8 storage with pointer/length values
* [x] Generate scalar aliases and disconnection copies
* [ ] Generate async support
* [x] Generate runtime calls
* [x] Bundle NASM 3.02 in the portable Windows distribution
* [x] Discover bundled `tools/nasm.exe` automatically
* [ ] Remove the remaining Visual Studio linker and Windows SDK requirement
* [ ] Add optimization passes
* [x] Validate generated assembly — NASM assembly and native EXE regression tests

---

# 46. Optimization

* [ ] Constant folding
* [ ] Constant propagation
* [ ] Dead-code elimination
* [ ] Dead-function elimination
* [ ] Expression simplification
* [ ] Inline small functions
* [ ] Optimize local variables
* [ ] Optimize function calls
* [ ] Optimize generated assembly
* [ ] Add optimization levels
* [ ] Benchmark compiler output
* [ ] Benchmark generated programs

---

# 47. Standard Library

Create the official standard library.

* [ ] String utilities
* [ ] Math
* [ ] Collections
* [ ] Date/time
* [ ] Random numbers
* [ ] Environment variables
* [ ] Process management
* [ ] Networking
* [ ] File system
* [ ] JSON
* [ ] Error handling
* [ ] Async
* [ ] Testing

---

# 48. Documentation

* [ ] Create official Adamantium documentation
* [x] Language overview
* [x] Installation guide
* [x] Getting started guide
* [x] Variables — currently supported declarations and mutability
* [x] Types — supported scalar types, defaults, and suffix annotations
* [x] Functions — typed parameters and named-result behavior
* [x] Classes
* [x] Enums
* [ ] Modules
* [ ] Packages
* [ ] Aliases
* [ ] Memory safety
* [ ] Error handling
* [ ] Async
* [ ] Testing
* [ ] Standard library
* [ ] CLI reference
* [ ] Compiler reference
* [ ] Professional mode
* [x] Examples
* [ ] Tutorials

---

# 49. Developer Experience

* [ ] VS Code syntax highlighting
* [ ] VS Code language extension
* [ ] Language Server Protocol (LSP)
* [ ] Autocomplete
* [ ] Go-to-definition
* [ ] Find references
* [ ] Rename symbol
* [ ] Diagnostics
* [ ] Formatting
* [ ] Code snippets
* [ ] Debugging support
* [ ] Integrated test runner

---

# 50. Tooling

* [ ] `adamantium fmt`
* [x] `adamantium test`
* [x] `adamantium check`
* [x] `adamantium build`
* [x] `adamantium run`
* [ ] `adamantium install`
* [ ] `adamantium clean`
* [ ] `adamantium doctor`
* [x] `adamantium new`
* [ ] `adamantium init`

---

# 51. GitHub Integration

* [x] Create GitHub Actions workflow
* [x] Build compiler on every push
* [x] Run compiler tests
* [x] Run Adamantium tests — native regression programs on Windows
* [x] Run formatting checks
* [x] Run static analysis
* [x] Check spelling with Typos on all three CI runners
* [x] Build release binaries
* [x] Test installed CLI on Windows x86-64 — help, version, build and run
* [x] Test Windows — compiler checks and native EXE regression tests configured
* [x] Test Linux — compiler build/tests configured; no Linux program backend
* [x] Test macOS — compiler build/tests configured; no macOS program backend
* [ ] Build documentation
* [x] Create portable Windows packaging workflow — manual runs and version tags
* [x] Publish portable ZIP as a GitHub Actions artifact
* [x] Build a standalone CLI that does not require Rust on user machines
* [x] Bundle and checksum the official NASM Windows binary
* [ ] Publish installer
* [ ] Publish packages directly on GitHub Releases

---

# 52. Compiler Testing

* [x] Lexer tests — covered through parser regression tests
* [x] Parser tests
* [x] AST tests
* [x] Type checker tests
* [x] Semantic analysis tests
* [x] Module tests — namespaces, qualified calls, use imports and nested files
* [x] Class tests — parsing, typing, visibility, construction, mutation and native copying
* [x] Enum tests
* [x] Alias tests
* [ ] Memory-safety tests
* [x] Code generation tests
* [x] NASM generation tests
* [x] Runtime tests
* [x] CLI tests — command parsing and installed Windows x86-64 CLI workflow
* [x] Integration tests
* [x] Regression tests

---

# 53. Error Messages

Create clear compiler diagnostics.

* [x] Error codes
* [x] Warning codes
* [x] Source locations
* [x] Line and column information
* [x] Error highlighting
* [x] Suggestions
* [x] "Did you mean?" suggestions
* [x] Multi-error reporting — independent project and requirement manifest errors
* [x] Context-aware diagnostics
* [x] Clear runtime panic messages

---

# 54. Cross-Platform Support

* [x] Windows support — native console executables
* [ ] Linux support
* [ ] macOS support
* [x] x86-64 backend — Windows only
* [ ] Define future ARM64 support
* [ ] Cross-platform standard library behavior
* [ ] Cross-platform file handling
* [ ] Cross-platform process handling

---

# 55. Security

* [ ] Validate package sources
* [ ] Validate package contents
* [ ] Prevent malicious package metadata
* [ ] Secure dependency installation
* [ ] Validate file-system operations
* [ ] Prevent unsafe path traversal
* [ ] Review runtime memory safety
* [ ] Review generated assembly safety
* [ ] Add security tests

---

# 56. Release System

* [ ] Define semantic versioning
* [x] Create portable Windows ZIP packaging
* [x] Add pinned SHA-256 verification for bundled NASM
* [x] Include the NASM BSD 2-Clause license in distributions
* [x] Build the Windows CLI with the static CRT
* [ ] Bundle a linker and required Windows libraries
* [ ] Create Windows installer
* [ ] Create release channels
* [ ] Stable releases
* [ ] Development releases
* [ ] Nightly releases
* [ ] Release notes
* [ ] Changelog
* [ ] Version compatibility rules
* [ ] Package compatibility rules

---

# 57. Final Language Specification

* [ ] Freeze syntax
* [ ] Freeze keyword list
* [ ] Freeze type system
* [ ] Freeze conversion rules
* [ ] Freeze memory model
* [ ] Freeze alias semantics
* [ ] Freeze class lifecycle
* [ ] Freeze module system
* [ ] Freeze package system
* [ ] Freeze async semantics
* [ ] Freeze test system
* [ ] Write complete language specification
* [ ] Write compiler specification
* [ ] Write standard library specification

---

# 58. First Stable Release

* [ ] Complete compiler
* [ ] Complete CLI
* [ ] Complete core language
* [ ] Complete memory-safety system
* [ ] Complete standard library
* [ ] Complete package manager
* [ ] Complete testing system
* [ ] Complete documentation
* [ ] Complete VS Code support
* [ ] Complete CI/CD
* [ ] Complete cross-platform builds
* [ ] Perform security audit
* [ ] Perform performance benchmarks
* [ ] Fix all critical bugs
* [ ] Tag `v1.0.0`
* [ ] Publish Adamantium 1.0
* [ ] Publish official documentation
* [ ] Publish official packages
* [ ] Announce stable release

---

# Current Priority

The recommended implementation order is:

1. [x] Lexer
2. [x] Parser
3. [x] AST
4. [x] Variables
5. [x] Types — current scalar, list, enum, class and function-value types
6. [x] Type checker
7. [x] Functions
8. [x] `if`
9. [x] `match`
10. [x] `for`
11. [x] `while`
12. [x] `until`
13. [x] `loop`
14. [x] Modules / `pack` / `use`
15. [x] Classes
16. [x] Enums
17. [ ] Memory-safety model
18. [x] Alias system — core value and symbol aliases
19. [x] NASM code generation
20. [x] Linker integration
21. [x] `adamantium build`
22. [x] `adamantium run`
23. [x] `adamantium check`
24. [x] Test system
25. [ ] `AdamantiumFiles`
26. [ ] `AdamantiumJson`
27. [ ] Package manager
28. [ ] `adamantium-async`
29. [ ] Async compiler support
30. [x] Generics
31. [x] Traits
32. [ ] Optimization
33. [ ] Documentation
34. [ ] VS Code support
35. [x] CI/CD — push and pull-request validation workflows
36. [ ] Stable release
