use std::collections::HashMap;
#[cfg(test)]
#[path = "typed_tests.rs"]
mod tests;
use crate::{
    syntax::{self, Expr, Operator, Statement},
    types::{self, Type, Value},
};

pub struct Expression {
    pub ty: Type,
    pub kind: Kind,
}
pub enum Kind {
    Constant(Value),
    String(Vec<u8>),
    Variable(usize),
    Negate(Box<Expression>),
    Binary(Operator, Box<Expression>, Box<Expression>),
    Convert(Box<Expression>),
    Call(String, Vec<Expression>),
}
pub enum Instruction {
    Assign(usize, Expression),
    Clamp(usize, Expression, Expression),
    Print(Expression, bool),
    Call(Expression),
    Return,
}
pub struct Function {
    pub name: String,
    pub parameters: usize,
    pub result: Option<usize>,
    pub types: Vec<Type>,
    pub instructions: Vec<Instruction>,
}
pub struct Program {
    pub functions: Vec<Function>,
}
struct Signature {
    parameters: Vec<Type>,
    result: Type,
}
struct Checker<'a> {
    types: Vec<Option<Type>>,
    signatures: &'a HashMap<String, Signature>,
}

pub fn check(program: &syntax::Program) -> Result<Program, String> {
    let signatures = program
        .functions
        .iter()
        .map(|f| {
            (
                f.name.clone(),
                Signature {
                    parameters: f.types[..f.parameters].iter().map(|t| t.unwrap()).collect(),
                    result: f
                        .result
                        .map(|slot| f.types[slot].unwrap())
                        .unwrap_or(Type::None),
                },
            )
        })
        .collect();
    let mut functions = Vec::new();
    for function in &program.functions {
        let mut checker = Checker {
            types: function.types.clone(),
            signatures: &signatures,
        };
        let mut instructions = Vec::new();
        for (statement, position) in function.statements.iter().zip(&function.positions) {
            let instruction = checker
                .statement(statement)
                .map_err(|e| position.error(e))?;
            instructions.push(instruction);
        }
        functions.push(Function {
            name: function.name.clone(),
            parameters: function.parameters,
            result: function.result,
            types: checker
                .types
                .into_iter()
                .map(|t| t.expect("every variable has an initializer or signature type"))
                .collect(),
            instructions,
        });
    }
    Ok(Program { functions })
}

fn promoted(a: Type, b: Type) -> Result<Type, String> {
    if !a.numeric() || !b.numeric() {
        return Err(format!(
            "arithmetic requires numeric types, found {a} and {b}"
        ));
    }
    if a == b {
        return Ok(a);
    }
    if a.floating() || b.floating() {
        return Ok(if a as u32 > b as u32 { a } else { b });
    }
    let (al, ah) = a.bounds();
    let (bl, bh) = b.bounds();
    let low = al.min(bl);
    let high = ah.max(bh);
    let candidates = if low < 0 {
        vec![Type::I8, Type::I16, Type::I32, Type::I64]
    } else {
        vec![Type::U4, Type::U8, Type::U16, Type::U32, Type::U64]
    };
    candidates
        .into_iter()
        .find(|t| {
            let (l, h) = t.bounds();
            l <= low && h >= high
        })
        .ok_or_else(|| {
            format!("no integer type can represent both {a} and {b}; specify a floating-point type")
        })
}

impl Checker<'_> {
    fn hint(&self, expr: &Expr) -> Option<Type> {
        match expr {
            Expr::Variable(slot) => self.types[*slot],
            Expr::Annotated(_, ty) => Some(*ty),
            Expr::Call(call) => self.signatures.get(&call.name).map(|s| s.result),
            Expr::Decimal(_) => Some(Type::F64),
            Expr::String(_) => Some(Type::String),
            Expr::Bool(_) => Some(Type::Bool),
            Expr::None => Some(Type::None),
            Expr::Negate(e) | Expr::Positive(e) => self.hint(e),
            Expr::Binary(_, a, b) => match (self.hint(a), self.hint(b)) {
                (Some(a), Some(b)) => promoted(a, b).ok(),
                (a, b) => a.or(b),
            },
            _ => None,
        }
    }
    fn convert(&self, expr: Expression, to: Type) -> Result<Expression, String> {
        if expr.ty == to {
            return Ok(expr);
        }
        if !expr.ty.numeric() || !to.numeric() || (expr.ty.floating() && to.integer()) {
            return Err(format!("cannot assign or convert {} to {to}", expr.ty));
        }
        if let Kind::Constant(value) = expr.kind {
            return Ok(Expression {
                ty: to,
                kind: Kind::Constant(types::convert(value, expr.ty, to)?),
            });
        }
        Ok(Expression {
            ty: to,
            kind: Kind::Convert(Box::new(expr)),
        })
    }
    fn expression(&self, expr: &Expr, expected: Option<Type>) -> Result<Expression, String> {
        let result = match expr {
            Expr::Integer(value) => {
                let ty = expected.filter(|t| t.numeric()).unwrap_or(Type::I32);
                Expression {
                    ty,
                    kind: Kind::Constant(types::literal(&value.to_string(), ty)?),
                }
            }
            Expr::Decimal(text) => {
                let ty = expected.filter(|t| t.floating()).unwrap_or(Type::F64);
                Expression {
                    ty,
                    kind: Kind::Constant(types::literal(text, ty)?),
                }
            }
            Expr::String(bytes) => Expression {
                ty: Type::String,
                kind: Kind::String(bytes.clone()),
            },
            Expr::Bool(value) => Expression {
                ty: Type::Bool,
                kind: Kind::Constant(Value {
                    lo: *value as u64,
                    hi: 0,
                }),
            },
            Expr::None => Expression {
                ty: Type::None,
                kind: Kind::Constant(Value::default()),
            },
            Expr::Variable(slot) => Expression {
                ty: self.types[*slot].ok_or("variable type is not known")?,
                kind: Kind::Variable(*slot),
            },
            Expr::Annotated(value, ty) => self.expression(value, Some(*ty))?,
            Expr::Call(call) => {
                let signature = self.signatures.get(&call.name).ok_or("unknown function")?;
                let arguments = call
                    .arguments
                    .iter()
                    .zip(&signature.parameters)
                    .map(|(a, t)| self.expression(a, Some(*t)))
                    .collect::<Result<_, _>>()?;
                Expression {
                    ty: signature.result,
                    kind: Kind::Call(call.name.clone(), arguments),
                }
            }
            Expr::Negate(value) => {
                let value = self.expression(value, expected)?;
                if !value.ty.numeric() || value.ty.unsigned() {
                    return Err(format!("cannot negate {}", value.ty));
                }
                Expression {
                    ty: value.ty,
                    kind: Kind::Negate(Box::new(value)),
                }
            }
            Expr::Positive(value) => {
                let value = self.expression(value, expected)?;
                if !value.ty.numeric() {
                    return Err(format!("unary '+' is not supported for {}", value.ty));
                }
                value
            }
            Expr::Binary(op, a, b) => {
                let hint = match (self.hint(a), self.hint(b)) {
                    (Some(a), Some(b)) => Some(promoted(a, b)?),
                    (a, b) => a.or(b),
                };
                let literal_type = expected.filter(|t| t.numeric()).or(hint);
                let a = self.expression(a, literal_type)?;
                let b = self.expression(b, literal_type)?;
                let ty = promoted(a.ty, b.ty)?;
                Expression {
                    ty,
                    kind: Kind::Binary(
                        *op,
                        Box::new(self.convert(a, ty)?),
                        Box::new(self.convert(b, ty)?),
                    ),
                }
            }
        };
        if let Some(ty) = expected {
            self.convert(result, ty)
        } else {
            Ok(result)
        }
    }
    fn statement(&mut self, statement: &Statement) -> Result<Instruction, String> {
        Ok(match statement {
            Statement::Assign(slot, expr) => {
                let value = self.expression(expr, self.types[*slot])?;
                self.types[*slot] = Some(value.ty);
                Instruction::Assign(*slot, value)
            }
            Statement::Clamp(slot, low, high) => {
                let ty = self.types[*slot].ok_or("unknown variable type")?;
                if !ty.numeric() {
                    return Err(format!("clamp is not supported for {ty}"));
                }
                Instruction::Clamp(
                    *slot,
                    self.expression(low, Some(ty))?,
                    self.expression(high, Some(ty))?,
                )
            }
            Statement::Print(expr, newline) => {
                Instruction::Print(self.expression(expr, None)?, *newline)
            }
            Statement::Return => Instruction::Return,
            Statement::Call(call) => {
                let signature = &self.signatures[&call.name];
                let arguments = call
                    .arguments
                    .iter()
                    .zip(&signature.parameters)
                    .map(|(a, t)| self.expression(a, Some(*t)))
                    .collect::<Result<_, _>>()?;
                Instruction::Call(Expression {
                    ty: signature.result,
                    kind: Kind::Call(call.name.clone(), arguments),
                })
            }
        })
    }
}
