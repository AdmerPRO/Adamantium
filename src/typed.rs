use std::collections::HashMap;
#[cfg(test)]
#[path = "typed_tests.rs"]
mod tests;
use crate::{
    syntax::{self, ClassDefinition, Comparison, Expr, LogicalOperator, Operator, Statement},
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
    Not(Box<Expression>),
    Binary(Operator, Box<Expression>, Box<Expression>),
    Compare(Comparison, Box<Expression>, Box<Expression>),
    Logical(LogicalOperator, Box<Expression>, Box<Expression>),
    Convert(Box<Expression>),
    Unwrap(Box<Expression>),
    Call(String, Vec<Expression>),
    Construct(u32, Vec<Expression>, String),
    List(Vec<Expression>),
    Index(Box<Expression>, Box<Expression>),
    Field(Box<Expression>, usize),
    MethodCall(String, Box<Expression>, Vec<Expression>),
}
pub enum Instruction {
    Noop,
    Assign(usize, Expression),
    Disconnect(usize, usize),
    Remove(Expression, Option<String>),
    Clamp(usize, Expression, Expression),
    Print(Expression, bool),
    Call(Expression),
    SetField(Expression, usize, Expression, Option<String>),
    SetIndex(Expression, Expression, Expression),
    Message(Expression, bool, usize),
    If(Expression, Vec<Instruction>, Vec<Instruction>),
    While(Expression, Vec<Instruction>),
    Until(Expression, Vec<Instruction>),
    Loop(Vec<Instruction>),
    For(usize, Expression, Expression, Vec<Instruction>),
    Match(
        Expression,
        Vec<(Expression, Vec<Instruction>)>,
        Option<Vec<Instruction>>,
    ),
    Break,
    Continue,
    Exit,
    Return,
}
pub struct Function {
    pub name: String,
    pub parameters: usize,
    pub result: Option<usize>,
    pub types: Vec<Type>,
    pub instructions: Vec<Instruction>,
    pub parameter_names: Vec<String>,
}
pub struct Program {
    pub functions: Vec<Function>,
    pub class_sizes: Vec<usize>,
    pub classes: Vec<ClassInfo>,
}
#[derive(Clone)]
pub struct ClassInfo {
    pub name: String,
    pub fields: Vec<ClassFieldInfo>,
}
#[derive(Clone)]
pub struct ClassFieldInfo {
    pub name: String,
    pub ty: Type,
    pub public: bool,
}
struct Signature {
    parameters: Vec<Type>,
    result: Type,
}
struct Checker<'a> {
    types: Vec<Option<Type>>,
    signatures: &'a HashMap<String, Signature>,
    classes: &'a [ClassDefinition],
    owner: Option<u32>,
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
    validate_operator_methods(program, &signatures)?;
    let mut functions = Vec::new();
    for function in &program.functions {
        let mut checker = Checker {
            types: function.types.clone(),
            signatures: &signatures,
            classes: &program.classes,
            owner: function.owner,
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
            parameter_names: function.bindings[..function.parameters]
                .iter()
                .map(|(name, _)| name.clone())
                .collect(),
        });
    }
    Ok(Program {
        functions,
        class_sizes: program
            .classes
            .iter()
            .map(|class| class.fields.len())
            .collect(),
        classes: program
            .classes
            .iter()
            .map(|class| ClassInfo {
                name: class.name.clone(),
                fields: class
                    .fields
                    .iter()
                    .map(|field| ClassFieldInfo {
                        name: field.name.clone(),
                        ty: field.ty,
                        public: field.public,
                    })
                    .collect(),
            })
            .collect(),
    })
}

fn validate_operator_methods(
    program: &syntax::Program,
    signatures: &HashMap<String, Signature>,
) -> Result<(), String> {
    const ARITHMETIC: &[&str] = &["__add__", "__sub__", "__mul__", "__div__"];
    const COMPARISON: &[&str] = &["__eq__", "__ne__", "__lt__", "__le__", "__gt__", "__ge__"];
    for class in &program.classes {
        for method in &class.methods {
            if !ARITHMETIC.contains(&method.name.as_str())
                && !COMPARISON.contains(&method.name.as_str())
            {
                continue;
            }
            if !method.public {
                return Err(format!("operator method '{}' must be public", method.name));
            }
            let signature = &signatures[&method.function];
            if signature.parameters.len() != 2 || signature.parameters[1] != Type::Class(class.id) {
                return Err(format!(
                    "operator method '{}' must accept exactly one required '{}' operand",
                    method.name, class.name
                ));
            }
            if COMPARISON.contains(&method.name.as_str()) && signature.result != Type::Bool {
                return Err(format!(
                    "operator method '{}' must return bool",
                    method.name
                ));
            }
            if ARITHMETIC.contains(&method.name.as_str())
                && matches!(signature.result, Type::None | Type::Optional(_))
            {
                return Err(format!(
                    "operator method '{}' must return a non-optional value",
                    method.name
                ));
            }
        }
    }
    Ok(())
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
        return Ok(if matches!(a, Type::F128) || matches!(b, Type::F128) {
            Type::F128
        } else if matches!(a, Type::F64) || matches!(b, Type::F64) {
            Type::F64
        } else {
            Type::F32
        });
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
    fn operator_call(
        &self,
        left: &Expr,
        right: &Expr,
        method_name: &str,
    ) -> Result<Expression, String> {
        let object = self.expression(left, None)?;
        let Type::Class(id) = object.ty else {
            return Err(format!("operator '{method_name}' requires a class value"));
        };
        let class = &self.classes[id as usize];
        let method = class
            .methods
            .iter()
            .find(|method| method.name == method_name)
            .ok_or_else(|| {
                format!(
                    "class '{}' does not implement operator method '{method_name}'",
                    class.name
                )
            })?;
        let signature = &self.signatures[&method.function];
        let argument = self.expression(right, Some(Type::Class(id)))?;
        Ok(Expression {
            ty: signature.result,
            kind: Kind::MethodCall(method.function.clone(), Box::new(object), vec![argument]),
        })
    }
    fn arguments(
        &self,
        arguments: &[Expr],
        parameters: &[Type],
    ) -> Result<Vec<Expression>, String> {
        let mut result = arguments
            .iter()
            .zip(parameters)
            .map(|(argument, ty)| self.expression(argument, Some(*ty)))
            .collect::<Result<Vec<_>, _>>()?;
        for ty in &parameters[arguments.len()..] {
            if !matches!(ty, Type::Optional(_)) {
                return Err("missing required function argument".into());
            }
            result.push(Expression {
                ty: *ty,
                kind: Kind::Constant(Value::default()),
            });
        }
        Ok(result)
    }
    fn hint(&self, expr: &Expr) -> Option<Type> {
        match expr {
            Expr::Variable(slot) => self.types[*slot],
            Expr::Annotated(_, ty) => Some(*ty),
            Expr::Cast(_, ty, _) => Some(*ty),
            Expr::Call(call) => self.signatures.get(&call.name).map(|s| s.result),
            Expr::Decimal(_) => Some(Type::F64),
            Expr::String(_) => Some(Type::String),
            Expr::Bool(_) => Some(Type::Bool),
            Expr::None => Some(Type::None),
            Expr::EnumVariant(ty, _) => Some(*ty),
            Expr::Construct(id, _) => Some(Type::Class(*id)),
            Expr::List(values) => values
                .first()
                .and_then(|value| self.hint(value))
                .map(|ty| Type::List(ty.id())),
            Expr::Index(list, _, _) => self.hint(list).and_then(|ty| match ty {
                Type::List(inner) => Type::from_id(inner),
                _ => None,
            }),
            Expr::Field(_, _, _) | Expr::MethodCall(_, _, _, _) => None,
            Expr::Negate(e) | Expr::Positive(e) => self.hint(e),
            Expr::Not(_) | Expr::Logical(_, _, _) => Some(Type::Bool),
            Expr::Binary(_, a, b) => match (self.hint(a), self.hint(b)) {
                (Some(a), Some(b)) => promoted(a, b).ok(),
                (a, b) => a.or(b),
            },
            Expr::Compare(_, _, _) => Some(Type::Bool),
            _ => None,
        }
    }
    fn convert(&self, expr: Expression, to: Type) -> Result<Expression, String> {
        if expr.ty == to {
            return Ok(expr);
        }
        if matches!(to, Type::Optional(_)) {
            let wide = Type::from_id(match to {
                Type::Optional(inner) => inner,
                _ => unreachable!(),
            })
            .is_some_and(Type::wide_optional_value);
            if !wide && let Kind::Constant(value) = expr.kind {
                return Ok(Expression {
                    ty: to,
                    kind: Kind::Constant(types::convert(value, expr.ty, to)?),
                });
            }
            return Ok(Expression {
                ty: to,
                kind: Kind::Convert(Box::new(expr)),
            });
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
            Expr::EnumVariant(ty, variant) => Expression {
                ty: *ty,
                kind: Kind::Constant(Value {
                    lo: u64::from(*variant),
                    hi: 0,
                }),
            },
            Expr::Construct(id, provided) => {
                let class = &self.classes[*id as usize];
                let mut fields = Vec::new();
                for field in &class.fields {
                    let matching = provided
                        .iter()
                        .filter(|(name, _)| name == &field.name)
                        .collect::<Vec<_>>();
                    if matching.len() > 1
                        || (matching.is_empty() && !matches!(field.ty, Type::Optional(_)))
                    {
                        return Err(format!(
                            "class '{}' requires field '{}' exactly once unless it is optional",
                            class.name, field.name
                        ));
                    }
                    fields.push(if matching.is_empty() {
                        Expression {
                            ty: field.ty,
                            kind: Kind::Constant(Value::default()),
                        }
                    } else {
                        self.expression(&matching[0].1, Some(field.ty))?
                    });
                }
                if let Some(unknown) = provided
                    .iter()
                    .find(|(name, _)| !class.fields.iter().any(|field| field.name == *name))
                    .map(|(name, _)| name.as_str())
                {
                    return Err(format!(
                        "invalid field '{unknown}' for class '{}'",
                        class.name
                    ));
                }
                Expression {
                    ty: Type::Class(*id),
                    kind: Kind::Construct(*id, fields, format!("{}____new__", class.name)),
                }
            }
            Expr::List(values) => {
                let expected_element = expected.and_then(|ty| match ty {
                    Type::List(inner) => Type::from_id(inner),
                    _ => None,
                });
                if values.is_empty() && expected_element.is_none() {
                    return Err("ambiguous List type: an empty List requires an explicit type, for example List[]:List[int]".into());
                }
                let (element_ty, checked) = if let Some(element_ty) = expected_element {
                    let checked = values
                        .iter()
                        .map(|value| self.expression(value, Some(element_ty)))
                        .collect::<Result<Vec<_>, _>>()?;
                    (element_ty, checked)
                } else {
                    let checked = values
                        .iter()
                        .map(|value| self.expression(value, None))
                        .collect::<Result<Vec<_>, _>>()?;
                    let mut element_ty = checked[0].ty;
                    for value in &checked[1..] {
                        element_ty = if element_ty == value.ty {
                            element_ty
                        } else if element_ty.numeric() && value.ty.numeric() {
                            promoted(element_ty, value.ty)?
                        } else {
                            return Err(format!(
                                "ambiguous List type: elements have incompatible types {element_ty} and {}; add :List[Type]",
                                value.ty
                            ));
                        };
                    }
                    let checked = checked
                        .into_iter()
                        .map(|value| self.convert(value, element_ty))
                        .collect::<Result<Vec<_>, _>>()?;
                    (element_ty, checked)
                };
                if element_ty.id() > u32::MAX >> 3 {
                    return Err("type nesting exceeds the supported depth".into());
                }
                Expression {
                    ty: Type::List(element_ty.id()),
                    kind: Kind::List(checked),
                }
            }
            Expr::Index(list, index, position) => {
                let list = self.expression(list, None)?;
                let Type::List(inner) = list.ty else {
                    return Err(
                        position.error(format!("invalid access: {} cannot be indexed", list.ty))
                    );
                };
                let element_ty = Type::from_id(inner).ok_or("invalid List element type")?;
                Expression {
                    ty: element_ty,
                    kind: Kind::Index(
                        Box::new(list),
                        Box::new(self.expression(index, Some(Type::U64))?),
                    ),
                }
            }
            Expr::Variable(slot) => Expression {
                ty: self.types[*slot].ok_or("variable type is not known")?,
                kind: Kind::Variable(*slot),
            },
            Expr::Annotated(value, ty) => self.expression(value, Some(*ty))?,
            Expr::Cast(value, ty, position) => {
                let value = self.expression(value, None)?;
                self.convert(value, *ty)
                    .map_err(|error| position.error(error))?
            }
            Expr::Call(call) => {
                let signature = self.signatures.get(&call.name).ok_or("unknown function")?;
                let arguments = self.arguments(&call.arguments, &signature.parameters)?;
                Expression {
                    ty: signature.result,
                    kind: Kind::Call(call.name.clone(), arguments),
                }
            }
            Expr::Field(object, field_name, position) => {
                let mut object = self.expression(object, None)?;
                if let Type::Optional(inner) = object.ty
                    && Type::from_id(inner).is_some_and(|ty| matches!(ty, Type::Class(_)))
                {
                    let ty = Type::from_id(inner).unwrap();
                    object = Expression {
                        ty,
                        kind: Kind::Unwrap(Box::new(object)),
                    };
                }
                let Type::Class(id) = object.ty else {
                    return Err(
                        position.error(format!("invalid access: {} has no fields", object.ty))
                    );
                };
                let class = &self.classes[id as usize];
                let (index, field) = class
                    .fields
                    .iter()
                    .enumerate()
                    .find(|(_, field)| field.name == *field_name)
                    .ok_or_else(|| {
                        position.error(format!(
                            "invalid access: class '{}' has no field '{field_name}'",
                            class.name
                        ))
                    })?;
                if !field.public && self.owner != Some(id) {
                    return Err(
                        position.error(format!("invalid access: field '{field_name}' is private"))
                    );
                }
                Expression {
                    ty: field.ty,
                    kind: Kind::Field(Box::new(object), index),
                }
            }
            Expr::MethodCall(object, method_name, arguments, position) => {
                let mut object = self.expression(object, None)?;
                if let Type::Optional(inner) = object.ty
                    && Type::from_id(inner).is_some_and(|ty| matches!(ty, Type::Class(_)))
                {
                    let ty = Type::from_id(inner).unwrap();
                    object = Expression {
                        ty,
                        kind: Kind::Unwrap(Box::new(object)),
                    };
                }
                let Type::Class(id) = object.ty else {
                    return Err(
                        position.error(format!("invalid access: {} has no methods", object.ty))
                    );
                };
                let class = &self.classes[id as usize];
                let method = class
                    .methods
                    .iter()
                    .find(|method| method.name == *method_name)
                    .ok_or_else(|| {
                        position.error(format!(
                            "invalid access: class '{}' has no method '{method_name}'",
                            class.name
                        ))
                    })?;
                if method.name == "__new__" || (!method.public && self.owner != Some(id)) {
                    return Err(position
                        .error(format!("invalid access: method '{method_name}' is private")));
                }
                let signature = &self.signatures[&method.function];
                let method_parameters = &signature.parameters[1..];
                let required = method_parameters
                    .iter()
                    .filter(|ty| !matches!(ty, Type::Optional(_)))
                    .count();
                if arguments.len() < required || arguments.len() > method_parameters.len() {
                    return Err(position.error(format!(
                        "method '{method_name}' expects {required}..={} arguments, found {}",
                        method_parameters.len(),
                        arguments.len()
                    )));
                }
                let arguments = self.arguments(arguments, method_parameters)?;
                Expression {
                    ty: signature.result,
                    kind: Kind::MethodCall(method.function.clone(), Box::new(object), arguments),
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
            Expr::Not(value) => Expression {
                ty: Type::Bool,
                kind: Kind::Not(Box::new(self.expression(value, Some(Type::Bool))?)),
            },
            Expr::Binary(op, a, b) => {
                if matches!(self.hint(a), Some(Type::Class(_))) {
                    let method = match op {
                        Operator::Add => "__add__",
                        Operator::Subtract => "__sub__",
                        Operator::Multiply => "__mul__",
                        Operator::Divide => "__div__",
                        Operator::Remainder => {
                            return Err("class remainder operator is not supported".into());
                        }
                    };
                    self.operator_call(a, b, method)?
                } else {
                    let hint = match (self.hint(a), self.hint(b)) {
                        (Some(a), Some(b)) => Some(promoted(a, b)?),
                        (a, b) => a.or(b),
                    };
                    let literal_type = expected.filter(|t| t.numeric()).or(hint);
                    let a = self.expression(a, literal_type)?;
                    let b = self.expression(b, literal_type)?;
                    let ty = promoted(a.ty, b.ty)?;
                    if matches!(op, Operator::Remainder) && !ty.integer() {
                        return Err("remainder requires integer operands".into());
                    }
                    Expression {
                        ty,
                        kind: Kind::Binary(
                            *op,
                            Box::new(self.convert(a, ty)?),
                            Box::new(self.convert(b, ty)?),
                        ),
                    }
                }
            }
            Expr::Compare(comparison, a, b) => {
                if matches!(self.hint(a), Some(Type::Class(_))) {
                    let method = match comparison {
                        Comparison::Equal => "__eq__",
                        Comparison::NotEqual => "__ne__",
                        Comparison::Less => "__lt__",
                        Comparison::LessEqual => "__le__",
                        Comparison::Greater => "__gt__",
                        Comparison::GreaterEqual => "__ge__",
                    };
                    self.operator_call(a, b, method)?
                } else {
                    let (a, b) = if self.hint(a).is_some_and(Type::numeric)
                        || self.hint(b).is_some_and(Type::numeric)
                    {
                        let hint = match (self.hint(a), self.hint(b)) {
                            (Some(a), Some(b)) => Some(promoted(a, b)?),
                            (a, b) => a.or(b),
                        };
                        let a = self.expression(a, hint)?;
                        let b = self.expression(b, hint)?;
                        let ty = promoted(a.ty, b.ty)?;
                        (self.convert(a, ty)?, self.convert(b, ty)?)
                    } else {
                        let a = self.expression(a, None)?;
                        let b = self.expression(b, Some(a.ty))?;
                        if !matches!(comparison, Comparison::Equal | Comparison::NotEqual)
                            && !a.ty.numeric()
                        {
                            return Err(format!(
                                "ordering comparison is not supported for {}",
                                a.ty
                            ));
                        }
                        if matches!(a.ty, Type::String | Type::Class(_)) {
                            return Err(format!("comparison is not supported for {}", a.ty));
                        }
                        (a, b)
                    };
                    Expression {
                        ty: Type::Bool,
                        kind: Kind::Compare(*comparison, Box::new(a), Box::new(b)),
                    }
                }
            }
            Expr::Logical(operator, a, b) => Expression {
                ty: Type::Bool,
                kind: Kind::Logical(
                    *operator,
                    Box::new(self.expression(a, Some(Type::Bool))?),
                    Box::new(self.expression(b, Some(Type::Bool))?),
                ),
            },
        };
        if let Some(ty) = expected {
            self.convert(result, ty)
        } else {
            Ok(result)
        }
    }
    fn statement(&mut self, statement: &Statement) -> Result<Instruction, String> {
        Ok(match statement {
            Statement::Noop(_) => Instruction::Noop,
            Statement::Assign(slot, expr) => {
                let value = self.expression(expr, self.types[*slot])?;
                self.types[*slot] = Some(value.ty);
                Instruction::Assign(*slot, value)
            }
            Statement::Disconnect(destination, source) => {
                let ty = self.types[*source].ok_or("unknown alias type")?;
                if matches!(ty, Type::Enum(_) | Type::Class(_)) {
                    return Err(format!("disconect is not supported for {ty} aliases"));
                }
                self.types[*destination] = Some(ty);
                Instruction::Disconnect(*destination, *source)
            }
            Statement::Remove(slot) => {
                let ty = self.types[*slot].ok_or("unknown removed variable type")?;
                let hook = if let Type::Class(id) = ty {
                    self.classes[id as usize]
                        .methods
                        .iter()
                        .find(|method| method.name == "__remove__")
                        .map(|method| method.function.clone())
                } else {
                    None
                };
                Instruction::Remove(
                    Expression {
                        ty,
                        kind: Kind::Variable(*slot),
                    },
                    hook,
                )
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
            Statement::Message(expr, panic, position) => Instruction::Message(
                self.expression(expr, Some(Type::String))?,
                *panic,
                position.line(),
            ),
            Statement::Return => Instruction::Return,
            Statement::Exit => Instruction::Exit,
            Statement::Call(call) => {
                let signature = &self.signatures[&call.name];
                let arguments = self.arguments(&call.arguments, &signature.parameters)?;
                Instruction::Call(Expression {
                    ty: signature.result,
                    kind: Kind::Call(call.name.clone(), arguments),
                })
            }
            Statement::SetField(object, field_name, value) => {
                let object = self.expression(object, None)?;
                let Type::Class(id) = object.ty else {
                    return Err(format!("invalid access: {} has no fields", object.ty));
                };
                let class = &self.classes[id as usize];
                let (index, field) = class
                    .fields
                    .iter()
                    .enumerate()
                    .find(|(_, field)| field.name == *field_name)
                    .ok_or_else(|| {
                        format!(
                            "invalid access: class '{}' has no field '{field_name}'",
                            class.name
                        )
                    })?;
                if !field.public && self.owner != Some(id) {
                    return Err(format!("invalid access: field '{field_name}' is private"));
                }
                let hook = class
                    .methods
                    .iter()
                    .find(|method| method.name == "__change__")
                    .map(|method| method.function.clone());
                Instruction::SetField(object, index, self.expression(value, Some(field.ty))?, hook)
            }
            Statement::SetIndex(list, index, value) => {
                let list = self.expression(list, None)?;
                let Type::List(inner) = list.ty else {
                    return Err(format!("invalid access: {} cannot be indexed", list.ty));
                };
                let element_ty = Type::from_id(inner).ok_or("invalid List element type")?;
                Instruction::SetIndex(
                    list,
                    self.expression(index, Some(Type::U64))?,
                    self.expression(value, Some(element_ty))?,
                )
            }
            Statement::MethodCall(expr) => Instruction::Call(self.expression(expr, None)?),
            Statement::If(condition, yes, no) => Instruction::If(
                self.expression(condition, Some(Type::Bool))?,
                yes.iter()
                    .map(|s| self.statement(s))
                    .collect::<Result<_, _>>()?,
                no.iter()
                    .map(|s| self.statement(s))
                    .collect::<Result<_, _>>()?,
            ),
            Statement::While(condition, body) => Instruction::While(
                self.expression(condition, Some(Type::Bool))?,
                body.iter()
                    .map(|s| self.statement(s))
                    .collect::<Result<_, _>>()?,
            ),
            Statement::Until(condition, body) => Instruction::Until(
                self.expression(condition, Some(Type::Bool))?,
                body.iter()
                    .map(|s| self.statement(s))
                    .collect::<Result<_, _>>()?,
            ),
            Statement::Loop(body) => Instruction::Loop(
                body.iter()
                    .map(|s| self.statement(s))
                    .collect::<Result<_, _>>()?,
            ),
            Statement::For(slot, start, end, body) => {
                let start = self.expression(start, self.types[*slot])?;
                let end = self.expression(end, Some(start.ty))?;
                if !start.ty.integer() {
                    return Err("for range bounds must be integers".into());
                }
                self.types[*slot] = Some(start.ty);
                Instruction::For(
                    *slot,
                    start,
                    end,
                    body.iter()
                        .map(|s| self.statement(s))
                        .collect::<Result<_, _>>()?,
                )
            }
            Statement::Match(value, arms, fallback) => {
                let value = self.expression(value, None)?;
                if matches!(value.ty, Type::String | Type::Class(_)) {
                    return Err(format!("match is not supported for {}", value.ty));
                }
                let mut typed_arms = Vec::new();
                let mut patterns = Vec::new();
                for (pattern, body) in arms {
                    let pattern = self.expression(pattern, Some(value.ty))?;
                    let Kind::Constant(pattern_value) = pattern.kind else {
                        return Err("match patterns must be literals or enum variants".into());
                    };
                    if patterns.contains(&pattern_value) {
                        return Err("match contains a duplicate pattern".into());
                    }
                    patterns.push(pattern_value);
                    typed_arms.push((
                        Expression {
                            ty: pattern.ty,
                            kind: Kind::Constant(pattern_value),
                        },
                        body.iter()
                            .map(|s| self.statement(s))
                            .collect::<Result<_, String>>()?,
                    ));
                }
                let fallback = fallback
                    .as_ref()
                    .map(|body| {
                        body.iter()
                            .map(|s| self.statement(s))
                            .collect::<Result<_, String>>()
                    })
                    .transpose()?;
                Instruction::Match(value, typed_arms, fallback)
            }
            Statement::Break => Instruction::Break,
            Statement::Continue => Instruction::Continue,
        })
    }
}
