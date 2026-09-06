use crate::types::Type;
use std::collections::HashMap;

#[cfg(test)]
#[path = "syntax_tests.rs"]
mod tests;

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Word(String),
    Number(String),
    String(String),
    Symbol(char),
    End,
}

#[derive(Clone, Copy, Debug)]
pub struct Position {
    line: usize,
    column: usize,
}
impl Position {
    pub fn error(self, message: impl std::fmt::Display) -> String {
        format!("{}:{}: {message}", self.line, self.column)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Operator {
    Add,
    Subtract,
    Multiply,
    Divide,
}
impl Operator {
    fn from_char(c: char) -> Option<Self> {
        match c {
            '+' => Some(Self::Add),
            '-' => Some(Self::Subtract),
            '*' => Some(Self::Multiply),
            '/' => Some(Self::Divide),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum Expr {
    Integer(i128),
    Decimal(String),
    String(Vec<u8>),
    Bool(bool),
    None,
    Annotated(Box<Expr>, Type),
    Variable(usize),
    Negate(Box<Expr>),
    Positive(Box<Expr>),
    Binary(Operator, Box<Expr>, Box<Expr>),
    Call(Call),
}
#[derive(Debug)]
pub struct Call {
    pub name: String,
    pub arguments: Vec<Expr>,
    pub position: Position,
}
#[derive(Debug)]
pub enum Statement {
    Assign(usize, Expr),
    Clamp(usize, Expr, Expr),
    Print(Expr, bool),
    Call(Call),
    Return,
}
#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub parameters: usize,
    pub result: Option<usize>,
    pub statements: Vec<Statement>,
    pub positions: Vec<Position>,
    pub types: Vec<Option<Type>>,
    pub bindings: Vec<(String, Position)>,
    pub position: Position,
}
#[derive(Debug)]
pub struct Program {
    pub functions: Vec<Function>,
}
struct Binding {
    slot: usize,
    initialized: bool,
    changeable: bool,
}
struct Parser {
    tokens: Vec<(Token, Position)>,
    cursor: usize,
    bindings: HashMap<String, Binding>,
    result_name: Option<String>,
    types: Vec<Option<Type>>,
    declarations: Vec<(String, Position)>,
}

fn lex(source: &str) -> Result<Vec<(Token, Position)>, String> {
    let mut chars = source.chars().peekable();
    let (mut line, mut column) = (1, 1);
    let mut tokens = Vec::new();
    while let Some(c) = chars.next() {
        let position = Position { line, column };
        column += 1;
        if c == '\n' {
            line += 1;
            column = 1;
            continue;
        }
        if c.is_whitespace() {
            continue;
        }
        if c == '/' && chars.peek() == Some(&'/') {
            while chars.peek().is_some_and(|c| *c != '\n') {
                chars.next();
                column += 1;
            }
            continue;
        }
        if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            column += 1;
            loop {
                let c = chars
                    .next()
                    .ok_or_else(|| position.error("unterminated block comment; expected '*/'"))?;
                if c == '\n' {
                    line += 1;
                    column = 1;
                } else {
                    column += 1;
                }
                if c == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    column += 1;
                    break;
                }
            }
            continue;
        }
        let token = if c.is_ascii_alphabetic() || c == '_' {
            let mut word = c.to_string();
            while chars
                .peek()
                .is_some_and(|c| c.is_ascii_alphanumeric() || *c == '_')
            {
                word.push(chars.next().unwrap());
                column += 1;
            }
            Token::Word(word)
        } else if c.is_ascii_digit() {
            let mut number = c.to_string();
            while chars.peek().is_some_and(char::is_ascii_digit) {
                number.push(chars.next().unwrap());
                column += 1;
            }
            if chars.peek() == Some(&'.')
                && chars.clone().nth(1).is_some_and(|c| c.is_ascii_digit())
            {
                number.push(chars.next().unwrap());
                column += 1;
                while chars.peek().is_some_and(char::is_ascii_digit) {
                    number.push(chars.next().unwrap());
                    column += 1;
                }
            }
            if chars.peek().is_some_and(|c| *c == 'e' || *c == 'E') {
                number.push(chars.next().unwrap());
                column += 1;
                if chars.peek().is_some_and(|c| *c == '+' || *c == '-') {
                    number.push(chars.next().unwrap());
                    column += 1;
                }
                if !chars.peek().is_some_and(char::is_ascii_digit) {
                    return Err(position.error("expected exponent digits"));
                }
                while chars.peek().is_some_and(char::is_ascii_digit) {
                    number.push(chars.next().unwrap());
                    column += 1;
                }
            }
            Token::Number(number)
        } else if c == '"' {
            let mut value = String::new();
            loop {
                let c = chars
                    .next()
                    .ok_or_else(|| position.error("unterminated string"))?;
                column += 1;
                match c {
                    '"' => break,
                    '\n' | '\r' => return Err(position.error("raw newline in string; use \\n")),
                    '\\' => {
                        let escaped = chars
                            .next()
                            .ok_or_else(|| position.error("unterminated escape"))?;
                        column += 1;
                        value.push(match escaped {
                            'n' => '\n',
                            'r' => '\r',
                            't' => '\t',
                            '0' => '\0',
                            '\\' => '\\',
                            '"' => '"',
                            _ => return Err(position.error("unsupported string escape")),
                        });
                    }
                    _ => value.push(c),
                }
            }
            Token::String(value)
        } else if "(){}.;=,:+-*/[]".contains(c) {
            Token::Symbol(c)
        } else {
            return Err(position.error(format!("unexpected character {c:?}")));
        };
        tokens.push((token, position));
    }
    tokens.push((Token::End, Position { line, column }));
    Ok(tokens)
}

impl Parser {
    fn peek(&self) -> &Token {
        &self.tokens[self.cursor].0
    }
    fn position(&self) -> Position {
        self.tokens[self.cursor].1
    }
    fn next(&mut self) -> Token {
        let token = self.peek().clone();
        if token != Token::End {
            self.cursor += 1;
        }
        token
    }
    fn take(&mut self, token: Token) -> bool {
        if self.peek() == &token {
            self.next();
            true
        } else {
            false
        }
    }
    fn expect(&mut self, expected: Token) -> Result<(), String> {
        if self.take(expected.clone()) {
            Ok(())
        } else {
            Err(self
                .position()
                .error(format!("expected {expected:?}, found {:?}", self.peek())))
        }
    }
    fn symbol(&mut self, c: char) -> Result<(), String> {
        self.expect(Token::Symbol(c))
    }
    fn word(&mut self, word: &str) -> Result<(), String> {
        self.expect(Token::Word(word.into()))
    }
    fn name(&mut self) -> Result<String, String> {
        let position = self.position();
        match self.next() {
            Token::Word(name)
                if ![
                    "fun",
                    "var",
                    "variable",
                    "print",
                    "return",
                    "static",
                    "stc",
                    "ch",
                    "changeable",
                    "true",
                    "false",
                    "offset",
                    "oofset",
                    "List",
                    "enum",
                    "class",
                    "use",
                    "pack",
                ]
                .contains(&name.as_str())
                    && Type::parse(&name).is_none() =>
            {
                Ok(name)
            }
            _ => Err(position.error("expected a name (reserved words cannot be used as names)")),
        }
    }
    fn bind(
        &mut self,
        name: String,
        initialized: bool,
        changeable: bool,
        ty: Option<Type>,
        position: Position,
    ) -> Result<usize, String> {
        if self.bindings.contains_key(&name) {
            return Err(position.error(format!("variable '{name}' is already declared")));
        }
        let slot = self.bindings.len();
        self.declarations.push((name.clone(), position));
        self.types.push(ty);
        self.bindings.insert(
            name,
            Binding {
                slot,
                initialized,
                changeable,
            },
        );
        Ok(slot)
    }
    fn variable(&self, name: &str, read: bool, position: Position) -> Result<usize, String> {
        let binding = self
            .bindings
            .get(name)
            .ok_or_else(|| position.error(format!("variable '{name}' is not declared")))?;
        if read && !binding.initialized {
            return Err(position.error(format!("variable '{name}' is not initialized")));
        }
        Ok(binding.slot)
    }
    fn type_name(&mut self) -> Result<Type, String> {
        let position = self.position();
        let Token::Word(name) = self.next() else {
            return Err(position.error("expected a type name"));
        };
        Type::parse(&name).ok_or_else(|| position.error(format!("unsupported type '{name}'")))
    }
    fn writable_variable(&self, name: &str, position: Position) -> Result<usize, String> {
        let slot = self.variable(name, false, position)?;
        if !self.bindings[name].changeable {
            return Err(position.error(format!("cannot modify static variable '{name}'")));
        }
        Ok(slot)
    }
    fn arguments(&mut self, name: String, position: Position) -> Result<Call, String> {
        if self.bindings.contains_key(&name) {
            return Err(
                position.error(format!("invalid access: variable '{name}' is not callable"))
            );
        }
        self.symbol('(')?;
        let mut arguments = Vec::new();
        if !self.take(Token::Symbol(')')) {
            loop {
                arguments.push(self.expression(0)?);
                if self.take(Token::Symbol(')')) {
                    break;
                }
                self.symbol(',')?;
            }
        }
        Ok(Call {
            name,
            arguments,
            position,
        })
    }
    fn expression(&mut self, min_precedence: u8) -> Result<Expr, String> {
        let position = self.position();
        let left = match self.next() {
            Token::Number(value) if value.contains(['.', 'e', 'E']) => Expr::Decimal(value),
            Token::Number(value) => Expr::Integer(
                value
                    .parse()
                    .map_err(|_| position.error("integer literal is too large"))?,
            ),
            Token::String(value) => Expr::String(value.into_bytes()),
            Token::Word(value) if value == "true" || value == "false" => {
                Expr::Bool(value == "true")
            }
            Token::Word(value) if value == "None" => Expr::None,
            Token::Symbol('-') => {
                // Read the negative magnitude directly to allow i64::MIN.
                if let Token::Number(value) = self.peek() {
                    if value.contains(['.', 'e', 'E']) {
                        let decimal = Expr::Decimal(format!("-{value}"));
                        self.next();
                        return self.expression_tail(decimal, min_precedence);
                    }
                    let number = format!("-{value}")
                        .parse()
                        .map_err(|_| position.error("integer literal is too large"))?;
                    self.next();
                    Expr::Integer(number)
                } else {
                    Expr::Negate(Box::new(self.expression(3)?))
                }
            }
            Token::Symbol('+') => Expr::Positive(Box::new(self.expression(3)?)),
            Token::Symbol('(') => {
                let inner = self.expression(0)?;
                self.symbol(')')?;
                inner
            }
            Token::Word(name) => {
                self.reject_access()?;
                if self.peek() == &Token::Symbol('(') {
                    Expr::Call(self.arguments(name, position)?)
                } else {
                    Expr::Variable(self.variable(&name, true, position)?)
                }
            }
            _ => return Err(position.error("expected an expression")),
        };
        self.expression_tail(left, min_precedence)
    }
    fn expression_tail(&mut self, mut left: Expr, min_precedence: u8) -> Result<Expr, String> {
        self.reject_access()?;
        while let Token::Symbol(c) = self.peek() {
            let Some(operator) = Operator::from_char(*c) else {
                break;
            };
            let precedence = if matches!(operator, Operator::Multiply | Operator::Divide) {
                2
            } else {
                1
            };
            if precedence < min_precedence {
                break;
            }
            self.next();
            let right = self.expression(precedence + 1)?;
            left = Expr::Binary(operator, Box::new(left), Box::new(right));
        }
        if min_precedence == 0 && self.take(Token::Symbol(':')) {
            left = Expr::Annotated(Box::new(left), self.type_name()?);
        }
        self.reject_access()?;
        Ok(left)
    }
    fn reject_access(&self) -> Result<(), String> {
        let qualified = self.peek() == &Token::Symbol(':')
            && self
                .tokens
                .get(self.cursor + 1)
                .is_some_and(|(token, _)| *token == Token::Symbol(':'));
        if qualified || matches!(self.peek(), Token::Symbol('.' | '[')) {
            return Err(self.position().error("invalid access: fields, indexing and qualified names are not supported; clamp is a standalone statement"));
        }
        Ok(())
    }
    fn statement(&mut self) -> Result<Statement, String> {
        self.reject_import()?;
        let position = self.position();
        let statement = match self.next() {
            Token::Word(word) if word == "var" || word == "variable" => {
                let changeable = if self.take(Token::Word("static".into()))
                    || self.take(Token::Word("stc".into()))
                {
                    false
                } else {
                    if !self.take(Token::Word("ch".into())) {
                        self.take(Token::Word("changeable".into()));
                    }
                    true
                };
                let name = self.name()?;
                self.symbol('=')?;
                let value = self.expression(0)?;
                let slot = self.bind(name, true, changeable, None, position)?;
                Statement::Assign(slot, value)
            }
            Token::Word(word) if word == "print" => {
                self.symbol('.')?;
                let newline = match self.next() {
                    Token::Word(method) if method == "newline" => true,
                    Token::Word(method) if method == "sameline" => false,
                    _ => return Err(position.error("expected newline or sameline")),
                };
                self.symbol('(')?;
                let output = Statement::Print(self.expression(0)?, newline);
                self.symbol(')')?;
                output
            }
            Token::Word(word) if word == "return" => {
                let Some(result_name) = self.result_name.clone() else {
                    return Err(position.error("main has no named result to return"));
                };
                self.word(&result_name)?;
                self.variable(&result_name, true, position)?;
                Statement::Return
            }
            Token::Word(name) => {
                if self.peek() == &Token::Symbol('(') {
                    Statement::Call(self.arguments(name, position)?)
                } else if self.take(Token::Symbol('.')) {
                    self.writable_variable(&name, position)?;
                    let slot = self.variable(&name, true, position)?;
                    if !self.take(Token::Word("clamp".into())) {
                        return Err(self.position().error(
                            "invalid access: the only supported variable method is clamp(min,max)",
                        ));
                    }
                    self.symbol('(')?;
                    let low = self.expression(0)?;
                    self.symbol(',')?;
                    let high = self.expression(0)?;
                    self.symbol(')')?;
                    Statement::Clamp(slot, low, high)
                } else {
                    self.reject_access()?;
                    let slot = self.writable_variable(&name, position)?;
                    let equals = self.position();
                    self.symbol('=')?;
                    // Compound operators must be adjacent: =- differs from = -value.
                    let adjacent = self.position().line == equals.line
                        && self.position().column == equals.column + 1;
                    let operator = match self.peek() {
                        Token::Symbol(c) if adjacent => Operator::from_char(*c),
                        _ => None,
                    };
                    let value = if let Some(operator) = operator {
                        self.variable(&name, true, position)?;
                        self.next();
                        Expr::Binary(
                            operator,
                            Box::new(Expr::Variable(slot)),
                            Box::new(self.expression(0)?),
                        )
                    } else {
                        self.expression(0)?
                    };
                    self.bindings.get_mut(&name).unwrap().initialized = true;
                    Statement::Assign(slot, value)
                }
            }
            _ => return Err(position.error("expected a statement")),
        };
        self.symbol(';')?;
        Ok(statement)
    }
    fn function(&mut self) -> Result<Function, String> {
        self.bindings.clear();
        self.types.clear();
        self.declarations.clear();
        self.result_name = None;
        let function_position = self.position();
        self.word("fun")?;
        let name = self.name()?;
        self.symbol('(')?;
        if !self.take(Token::Symbol(')')) {
            loop {
                let position = self.position();
                let parameter = self.name()?;
                self.symbol(':')?;
                let ty = self.type_name()?;
                self.bind(parameter, true, true, Some(ty), position)?;
                if self.take(Token::Symbol(')')) {
                    break;
                }
                self.symbol(',')?;
            }
        }
        let parameters = self.bindings.len();
        let result = if name == "main" {
            if parameters != 0 {
                return Err(self.position().error("main must have no parameters"));
            }
            None
        } else {
            let position = self.position();
            let result_name = self.name()?;
            self.symbol(':')?;
            let ty = self.type_name()?;
            let slot = self.bind(
                result_name.clone(),
                ty == Type::None,
                true,
                Some(ty),
                position,
            )?;
            self.result_name = Some(result_name);
            Some(slot)
        };
        self.symbol('{')?;
        let mut statements = Vec::new();
        let mut positions = Vec::new();
        if let Some(slot) = result
            && self.types[slot] == Some(Type::None)
        {
            statements.push(Statement::Assign(slot, Expr::None));
            positions.push(function_position);
        }
        let mut returned = false;
        while self.peek() != &Token::Symbol('}') {
            positions.push(self.position());
            let statement = self.statement()?;
            returned |= matches!(statement, Statement::Return);
            statements.push(statement);
        }
        if !returned && let Some(name) = &self.result_name {
            self.variable(name, true, self.position())?;
        }
        self.symbol('}')?;
        Ok(Function {
            name,
            parameters,
            result,
            statements,
            positions,
            types: self.types.clone(),
            bindings: self.declarations.clone(),
            position: function_position,
        })
    }
    fn reject_import(&self) -> Result<(), String> {
        if let Token::Word(keyword) = self.peek()
            && (keyword == "use" || keyword == "pack")
        {
            return Err(self.position().error(format!("invalid import: '{keyword}' cannot be resolved because modules and imports are not supported yet")));
        }
        Ok(())
    }
}

pub fn parse(source: &str) -> Result<Program, String> {
    let tokens = lex(source)?;
    let mut parser = Parser {
        tokens,
        cursor: 0,
        bindings: HashMap::new(),
        result_name: None,
        types: Vec::new(),
        declarations: Vec::new(),
    };
    let mut functions = Vec::new();
    let mut signatures = HashMap::new();
    while parser.peek() != &Token::End {
        parser.reject_import()?;
        let position = parser.position();
        let function = parser.function()?;
        if signatures
            .insert(function.name.clone(), function.parameters)
            .is_some()
        {
            return Err(position.error(format!("function '{}' is already declared", function.name)));
        }
        functions.push(function);
    }
    if !signatures.contains_key("main") {
        return Err("1:1: program must declare fun main()".into());
    }
    for function in &functions {
        for statement in &function.statements {
            match statement {
                Statement::Assign(_, expr) | Statement::Print(expr, _) => {
                    validate_expr(expr, &signatures)?
                }
                Statement::Clamp(_, low, high) => {
                    validate_expr(low, &signatures)?;
                    validate_expr(high, &signatures)?;
                }
                Statement::Call(call) => validate_call(call, &signatures)?,
                _ => (),
            }
        }
    }
    Ok(Program { functions })
}
fn validate_expr(expr: &Expr, signatures: &HashMap<String, usize>) -> Result<(), String> {
    match expr {
        Expr::Call(call) => validate_call(call, signatures),
        Expr::Binary(_, left, right) => {
            validate_expr(left, signatures)?;
            validate_expr(right, signatures)
        }
        Expr::Negate(expr) | Expr::Positive(expr) | Expr::Annotated(expr, _) => {
            validate_expr(expr, signatures)
        }
        _ => Ok(()),
    }
}
fn validate_call(call: &Call, signatures: &HashMap<String, usize>) -> Result<(), String> {
    let arity = signatures.get(&call.name).ok_or_else(|| {
        call.position
            .error(format!("function '{}' is not declared", call.name))
    })?;
    if call.name == "main" {
        return Err(call.position.error("main cannot be called as a function"));
    }
    if *arity != call.arguments.len() {
        return Err(call.position.error(format!(
            "function '{}' expects {arity} arguments, found {}",
            call.name,
            call.arguments.len()
        )));
    }
    for argument in &call.arguments {
        validate_expr(argument, signatures)?;
    }
    Ok(())
}
