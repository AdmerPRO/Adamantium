use crate::syntax::{Call, Expr, Program, Statement};
use std::collections::{HashMap, HashSet};

pub fn warnings(program: &Program) -> Vec<String> {
    let mut warnings = Vec::new();
    let mut graph = HashMap::new();
    for function in &program.functions {
        let mut reads = HashSet::new();
        let mut declared = (0..function.parameters).collect::<HashSet<_>>();
        let mut calls = HashSet::new();
        let mut returned = false;
        let mut warned_unreachable = false;
        for (statement, position) in function.statements.iter().zip(&function.positions) {
            if returned {
                if !warned_unreachable {
                    warnings.push(position.error("warning[W001]: unreachable code after return"));
                    warned_unreachable = true;
                }
                continue;
            }
            match statement {
                Statement::Assign(slot, expr) => {
                    declared.insert(*slot);
                    visit(expr, &mut reads, &mut calls);
                }
                Statement::Clamp(slot, low, high) => {
                    reads.insert(*slot);
                    visit(low, &mut reads, &mut calls);
                    visit(high, &mut reads, &mut calls);
                }
                Statement::Print(expr, _) => visit(expr, &mut reads, &mut calls),
                Statement::Call(call) => visit_call(call, &mut reads, &mut calls),
                Statement::Return => returned = true,
            }
        }
        // A named result is read by both explicit and implicit return.
        if let Some(result) = function.result {
            reads.insert(result);
        }
        for (slot, (name, position)) in function.bindings.iter().enumerate() {
            if declared.contains(&slot) && !reads.contains(&slot) && !name.starts_with('_') {
                let kind = if slot < function.parameters {
                    "parameter"
                } else {
                    "variable"
                };
                warnings.push(position.error(format!(
                    "warning[W002]: unused {kind} '{name}' in function '{}'",
                    function.name
                )));
            }
        }
        graph.insert(function.name.as_str(), calls);
    }
    // Follow calls from main: disconnected recursive groups are unused too.
    let mut reached = HashSet::new();
    let mut pending = vec!["main"];
    while let Some(name) = pending.pop() {
        if reached.insert(name)
            && let Some(calls) = graph.get(name)
        {
            pending.extend(calls.iter().map(String::as_str));
        }
    }
    for function in &program.functions {
        if !reached.contains(function.name.as_str()) && !function.name.starts_with('_') {
            warnings.push(function.position.error(format!(
                "warning[W003]: unused function '{}' (not reachable from main)",
                function.name
            )));
        }
    }
    warnings
}

fn visit(expr: &Expr, reads: &mut HashSet<usize>, calls: &mut HashSet<String>) {
    match expr {
        Expr::Variable(slot) => {
            reads.insert(*slot);
        }
        Expr::Call(call) => visit_call(call, reads, calls),
        Expr::Binary(_, a, b) => {
            visit(a, reads, calls);
            visit(b, reads, calls);
        }
        Expr::Negate(expr) | Expr::Positive(expr) | Expr::Annotated(expr, _) => {
            visit(expr, reads, calls)
        }
        _ => (),
    }
}
fn visit_call(call: &Call, reads: &mut HashSet<usize>, calls: &mut HashSet<String>) {
    calls.insert(call.name.clone());
    for argument in &call.arguments {
        visit(argument, reads, calls);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn analyze(source: &str) -> Vec<String> {
        let program = crate::syntax::parse(source).unwrap();
        crate::typed::check(&program).unwrap();
        warnings(&program)
    }
    #[test]
    fn ignores_dead_reads_and_calls() {
        let result = analyze(
            "fun main() { entry(); }\nfun entry() r:int {\nvar unused = 10;\nr = 0;\nreturn r;\nprint.newline(unused);\ndead();\n}\nfun dead() r:int { r = 1; }",
        );
        assert!(result.iter().any(|s| s.starts_with("6:1: warning[W001]")));
        assert!(
            result
                .iter()
                .any(|s| s.contains("unused variable 'unused'"))
        );
        assert!(result.iter().any(|s| s.contains("unused function 'dead'")));
        assert_eq!(result.len(), 3);
    }
    #[test]
    fn follows_calls_and_reports_disconnected_cycles() {
        let result = analyze(
            "fun main() { print.newline(a()); } fun a() r:int { r = b(); } fun b() r:int { r = 1; } fun x() r:int { r = y(); } fun y() r:int { r = x(); }",
        );
        assert_eq!(result.len(), 2);
        assert!(result[0].contains("unused function 'x'"));
        assert!(result[1].contains("unused function 'y'"));
    }
    #[test]
    fn distinguishes_reads_writes_parameters_and_intentional_unused_names() {
        let result = analyze(
            "fun main() { var written = 1; written = 2; var _ignored = 1; print.newline(f(1,2)); } fun f(used:int,unused:int) r:int { var value = used; value =+ 1; value.clamp(0,10); r = value; } fun _reserved() r:None {}",
        );
        assert_eq!(result.len(), 2);
        assert!(result[0].contains("unused variable 'written'"));
        assert!(result[1].contains("unused parameter 'unused'"));
    }
    #[test]
    fn result_variables_and_arguments_are_used() {
        assert!(analyze("fun main() { var a = 1; var result = add(a,2); print.newline(result); } fun add(a:int,b:int) r:int { r = a+b; return r; }").is_empty());
    }
}
