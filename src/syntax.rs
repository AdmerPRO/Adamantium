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
#[derive(Clone, Copy, Debug)]
pub enum Comparison {
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
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
    EnumVariant(Type, u32),
    Construct(u32, Vec<(String, Expr)>),
    Annotated(Box<Expr>, Type),
    Variable(usize),
    Field(Box<Expr>, String, Position),
    MethodCall(Box<Expr>, String, Vec<Expr>, Position),
    Negate(Box<Expr>),
    Positive(Box<Expr>),
    Binary(Operator, Box<Expr>, Box<Expr>),
    Compare(Comparison, Box<Expr>, Box<Expr>),
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
    Noop(Option<String>),
    Assign(usize, Expr),
    Disconnect(usize, usize),
    Clamp(usize, Expr, Expr),
    Print(Expr, bool),
    Call(Call),
    SetField(Expr, String, Expr),
    MethodCall(Expr),
    If(Expr, Vec<Statement>, Vec<Statement>),
    While(Expr, Vec<Statement>),
    Until(Expr, Vec<Statement>),
    Loop(Vec<Statement>),
    For(usize, Expr, Expr, Vec<Statement>),
    Match(Expr, Vec<(Expr, Vec<Statement>)>, Option<Vec<Statement>>),
    Break,
    Continue,
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
    pub owner: Option<u32>,
}
#[derive(Clone, Debug)]
pub struct ClassField {
    pub name: String,
    pub ty: Type,
    pub public: bool,
}
#[derive(Clone, Debug)]
pub struct ClassMethod {
    pub name: String,
    pub function: String,
    pub public: bool,
}
#[derive(Clone, Debug)]
pub struct ClassDefinition {
    pub id: u32,
    pub name: String,
    pub fields: Vec<ClassField>,
    pub methods: Vec<ClassMethod>,
}
#[derive(Debug)]
pub struct Program {
    pub functions: Vec<Function>,
    pub classes: Vec<ClassDefinition>,
}
struct Binding {
    slot: usize,
    initialized: bool,
    changeable: bool,
    alias: bool,
}
#[derive(Clone)]
enum SymbolAlias {
    Function(String),
    Enum(String),
    Class(String),
}
struct EnumDefinition {
    ty: Type,
    variants: HashMap<String, u32>,
}
struct Parser {
    tokens: Vec<(Token, Position)>,
    cursor: usize,
    bindings: HashMap<String, Binding>,
    result_name: Option<String>,
    types: Vec<Option<Type>>,
    declarations: Vec<(String, Position)>,
    enums: HashMap<String, EnumDefinition>,
    classes: HashMap<String, ClassDefinition>,
    loop_depth: usize,
    symbol_aliases: HashMap<String, SymbolAlias>,
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
        } else if "(){}.;=,:+-*/[]!<>".contains(c) {
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
                    "pub",
                    "if",
                    "then",
                    "else",
                    "for",
                    "in",
                    "while",
                    "until",
                    "loop",
                    "break",
                    "continue",
                    "match",
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
        if self.enums.contains_key(&name) {
            return Err(position.error(format!(
                "variable '{name}' conflicts with an enum of the same name"
            )));
        }
        if self.bindings.contains_key(&name) || self.symbol_aliases.contains_key(&name) {
            return Err(position.error(format!("variable '{name}' is already declared")));
        }
        let slot = self.types.len();
        self.declarations.push((name.clone(), position));
        self.types.push(ty);
        self.bindings.insert(
            name,
            Binding {
                slot,
                initialized,
                changeable,
                alias: false,
            },
        );
        Ok(slot)
    }
    fn alias_target(&mut self) -> Result<SymbolAlias, String> {
        let position = self.position();
        let Token::Word(target) = self.next() else {
            return Err(position.error("expected a variable, function, enum or class name"));
        };
        let target = self
            .symbol_aliases
            .get(&target)
            .cloned()
            .unwrap_or_else(|| {
                if self.enums.contains_key(&target) {
                    SymbolAlias::Enum(target.clone())
                } else if self.classes.contains_key(&target) {
                    SymbolAlias::Class(target.clone())
                } else {
                    SymbolAlias::Function(target.clone())
                }
            });
        if matches!(target, SymbolAlias::Function(_)) {
            self.symbol('(')?;
            self.symbol(')')?;
        }
        self.symbol('.')?;
        self.word("as_variable")?;
        Ok(target)
    }
    fn is_alias_target(&self) -> bool {
        let Some((Token::Word(name), _)) = self.tokens.get(self.cursor) else {
            return false;
        };
        let offset = if self.enums.contains_key(name) || self.classes.contains_key(name) {
            1
        } else {
            3
        };
        self.tokens
            .get(self.cursor + offset)
            .is_some_and(|(token, _)| token == &Token::Symbol('.'))
            && self
                .tokens
                .get(self.cursor + offset + 1)
                .is_some_and(|(token, _)| token == &Token::Word("as_variable".into()))
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
        Type::parse(&name)
            .or_else(|| self.enums.get(&name).map(|definition| definition.ty))
            .or_else(|| {
                self.classes
                    .get(&name)
                    .map(|definition| Type::Class(definition.id))
            })
            .ok_or_else(|| position.error(format!("unsupported type '{name}'")))
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
                let name = match self.symbol_aliases.get(&name) {
                    Some(
                        SymbolAlias::Function(target)
                        | SymbolAlias::Enum(target)
                        | SymbolAlias::Class(target),
                    ) => target.clone(),
                    None => name,
                };
                if self.enums.contains_key(&name) && self.peek() == &Token::Symbol('.') {
                    self.next();
                    let variant_position = self.position();
                    let Token::Word(variant) = self.next() else {
                        return Err(variant_position.error("expected an enum variant name"));
                    };
                    let definition = &self.enums[&name];
                    let value = definition.variants.get(&variant).copied().ok_or_else(|| {
                        variant_position.error(format!("enum '{name}' has no variant '{variant}'"))
                    })?;
                    Expr::EnumVariant(definition.ty, value)
                } else if self.classes.contains_key(&name) && self.peek() == &Token::Symbol('(') {
                    self.next();
                    let mut fields = Vec::new();
                    if !self.take(Token::Symbol(')')) {
                        loop {
                            let field = self.name()?;
                            self.symbol('=')?;
                            fields.push((field, self.expression(0)?));
                            if self.take(Token::Symbol(')')) {
                                break;
                            }
                            self.symbol(',')?;
                        }
                    }
                    Expr::Construct(self.classes[&name].id, fields)
                } else if self.peek() == &Token::Symbol('(') {
                    Expr::Call(self.arguments(name, position)?)
                } else {
                    if self.peek() == &Token::Symbol(':') {
                        self.reject_access()?;
                    }
                    Expr::Variable(self.variable(&name, true, position)?)
                }
            }
            _ => return Err(position.error("expected an expression")),
        };
        self.expression_tail(left, min_precedence)
    }
    fn expression_tail(&mut self, mut left: Expr, min_precedence: u8) -> Result<Expr, String> {
        while self.peek() == &Token::Symbol('.')
            && self
                .tokens
                .get(self.cursor + 1)
                .is_some_and(|(token, _)| token != &Token::Symbol('.'))
        {
            self.next();
            let position = self.position();
            let member = self.name()?;
            if self.take(Token::Symbol('(')) {
                if let Expr::Variable(slot) = &left
                    && self
                        .bindings
                        .values()
                        .any(|binding| binding.slot == *slot && !binding.changeable)
                {
                    return Err(position.error("cannot call a method on a static variable"));
                }
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
                left = Expr::MethodCall(Box::new(left), member, arguments, position);
            } else {
                left = Expr::Field(Box::new(left), member, position);
            }
        }
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
        if min_precedence == 0 {
            let comparison = match (self.peek(), self.tokens.get(self.cursor + 1).map(|x| &x.0)) {
                (Token::Symbol('='), Some(Token::Symbol('='))) => Some((Comparison::Equal, true)),
                (Token::Symbol('!'), Some(Token::Symbol('='))) => {
                    Some((Comparison::NotEqual, true))
                }
                (Token::Symbol('<'), Some(Token::Symbol('='))) => {
                    Some((Comparison::LessEqual, true))
                }
                (Token::Symbol('>'), Some(Token::Symbol('='))) => {
                    Some((Comparison::GreaterEqual, true))
                }
                (Token::Symbol('<'), _) => Some((Comparison::Less, false)),
                (Token::Symbol('>'), _) => Some((Comparison::Greater, false)),
                _ => None,
            };
            if let Some((comparison, two_symbols)) = comparison {
                self.next();
                if two_symbols {
                    self.next();
                }
                let right = self.expression(1)?;
                left = Expr::Compare(comparison, Box::new(left), Box::new(right));
                if matches!(self.peek(), Token::Symbol('=' | '!' | '<' | '>')) {
                    return Err(self
                        .position()
                        .error("chained comparisons are not supported"));
                }
            }
        }
        if min_precedence == 0 && self.take(Token::Symbol(':')) {
            left = Expr::Annotated(Box::new(left), self.type_name()?);
        }
        if matches!(self.peek(), Token::Symbol('[')) {
            return Err(self
                .position()
                .error("invalid access: indexing is not supported"));
        }
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
        if let Token::Word(keyword) = self.peek()
            && matches!(
                keyword.as_str(),
                "if" | "while" | "until" | "loop" | "for" | "match"
            )
        {
            return self.control_statement();
        }
        let statement = match self.next() {
            Token::Word(word) if word == "break" || word == "continue" => {
                if self.loop_depth == 0 {
                    return Err(position.error(format!("'{word}' can only be used inside a loop")));
                }
                if word == "break" {
                    Statement::Break
                } else {
                    Statement::Continue
                }
            }
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
                if self.is_alias_target() {
                    if !changeable {
                        return Err(position.error("symbol aliases cannot be static"));
                    }
                    let target = self.alias_target()?;
                    if self.bindings.contains_key(&name) || self.symbol_aliases.contains_key(&name)
                    {
                        return Err(
                            position.error(format!("variable '{name}' is already declared"))
                        );
                    }
                    let function = if let SymbolAlias::Function(target) = &target {
                        Some(target.clone())
                    } else {
                        None
                    };
                    self.symbol_aliases.insert(name, target);
                    self.symbol(';')?;
                    return Ok(Statement::Noop(function));
                }
                if let (
                    Token::Word(source),
                    Some((Token::Symbol('.'), _)),
                    Some((Token::Word(method), _)),
                ) = (
                    self.peek(),
                    self.tokens.get(self.cursor + 1),
                    self.tokens.get(self.cursor + 2),
                ) && method == "as_variable"
                {
                    let source = source.clone();
                    let source_slot = self.variable(&source, true, position)?;
                    self.next();
                    self.next();
                    self.next();
                    if self.bindings.contains_key(&name) || self.symbol_aliases.contains_key(&name)
                    {
                        return Err(
                            position.error(format!("variable '{name}' is already declared"))
                        );
                    }
                    let source_changeable = self.bindings[&source].changeable;
                    self.bindings.insert(
                        name,
                        Binding {
                            slot: source_slot,
                            initialized: true,
                            changeable: changeable && source_changeable,
                            alias: true,
                        },
                    );
                    self.symbol(';')?;
                    return Ok(Statement::Noop(None));
                }
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
                if self.symbol_aliases.contains_key(&name) {
                    if self.peek() == &Token::Symbol('(') {
                        let SymbolAlias::Function(target) = self.symbol_aliases[&name].clone()
                        else {
                            return Err(
                                position.error(format!("symbol alias '{name}' is not callable"))
                            );
                        };
                        Statement::Call(self.arguments(target, position)?)
                    } else {
                        if self.peek() == &Token::Symbol('.')
                            && self.tokens.get(self.cursor + 1).is_some_and(|(token, _)| {
                                matches!(token, Token::Word(method) if method == "disconect" || method == "disconnect")
                            })
                        {
                            return Err(position.error("function, enum and class aliases cannot be disconnected"));
                        }
                        self.symbol('=')?;
                        let target = self.alias_target()?;
                        let function = if let SymbolAlias::Function(target) = &target {
                            Some(target.clone())
                        } else {
                            None
                        };
                        self.symbol_aliases.insert(name, target);
                        Statement::Noop(function)
                    }
                } else if self.peek() == &Token::Symbol('(') {
                    Statement::Call(self.arguments(name, position)?)
                } else if self.take(Token::Symbol('.')) {
                    let slot = self.variable(&name, true, position)?;
                    let member_position = self.position();
                    let member = self.name()?;
                    if member == "disconect" || member == "disconnect" {
                        let binding = self.bindings.get(&name).unwrap();
                        if !binding.alias {
                            return Err(
                                position.error(format!("variable '{name}' is not an alias"))
                            );
                        }
                        let old_slot = binding.slot;
                        let new_slot = self.types.len();
                        self.types.push(None);
                        self.declarations.push((name.clone(), position));
                        self.bindings.insert(
                            name,
                            Binding {
                                slot: new_slot,
                                initialized: true,
                                changeable: true,
                                alias: false,
                            },
                        );
                        Statement::Disconnect(new_slot, old_slot)
                    } else if member == "clamp" {
                        self.writable_variable(&name, position)?;
                        self.symbol('(')?;
                        let low = self.expression(0)?;
                        self.symbol(',')?;
                        let high = self.expression(0)?;
                        self.symbol(')')?;
                        Statement::Clamp(slot, low, high)
                    } else if self.take(Token::Symbol('(')) {
                        self.writable_variable(&name, position)?;
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
                        Statement::MethodCall(Expr::MethodCall(
                            Box::new(Expr::Variable(slot)),
                            member,
                            arguments,
                            member_position,
                        ))
                    } else {
                        self.writable_variable(&name, position)?;
                        self.symbol('=')?;
                        Statement::SetField(Expr::Variable(slot), member, self.expression(0)?)
                    }
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
    fn block(&mut self) -> Result<Vec<Statement>, String> {
        self.symbol('{')?;
        let mut statements = Vec::new();
        while !self.take(Token::Symbol('}')) {
            if self.peek() == &Token::End {
                return Err(self.position().error("expected '}' before end of file"));
            }
            statements.push(self.statement()?);
        }
        Ok(statements)
    }
    fn loop_block(&mut self) -> Result<Vec<Statement>, String> {
        self.loop_depth += 1;
        let result = self.block();
        self.loop_depth -= 1;
        result
    }
    fn control_statement(&mut self) -> Result<Statement, String> {
        let position = self.position();
        let Token::Word(keyword) = self.next() else {
            unreachable!()
        };
        Ok(match keyword.as_str() {
            "if" => {
                let condition = self.expression(0)?;
                self.word("then")?;
                let yes = self.block()?;
                let no = if self.take(Token::Word("else".into())) {
                    self.block()?
                } else {
                    Vec::new()
                };
                Statement::If(condition, yes, no)
            }
            "while" => Statement::While(self.expression(0)?, self.loop_block()?),
            "until" => Statement::Until(self.expression(0)?, self.loop_block()?),
            "loop" => Statement::Loop(self.loop_block()?),
            "for" => {
                let name_position = self.position();
                let name = self.name()?;
                self.word("in")?;
                let start = self.expression(0)?;
                self.symbol('.')?;
                self.symbol('.')?;
                let end = self.expression(0)?;
                let slot = self.bind(name, true, false, None, name_position)?;
                Statement::For(slot, start, end, self.loop_block()?)
            }
            "match" => {
                let value = self.expression(0)?;
                self.symbol('{')?;
                let mut arms = Vec::new();
                let mut fallback = None;
                while !self.take(Token::Symbol('}')) {
                    if self.peek() == &Token::Word("_".into()) {
                        let fallback_position = self.position();
                        self.next();
                        if fallback.is_some() {
                            return Err(
                                fallback_position.error("match can contain only one '_' branch")
                            );
                        }
                        self.symbol('=')?;
                        self.symbol('>')?;
                        fallback = Some(self.block()?);
                    } else {
                        let pattern = self.expression(0)?;
                        self.symbol('=')?;
                        self.symbol('>')?;
                        arms.push((pattern, self.block()?));
                    }
                    self.take(Token::Symbol(','));
                    if fallback.is_some() && self.peek() != &Token::Symbol('}') {
                        return Err(self.position().error("the '_' match branch must be last"));
                    }
                }
                if arms.is_empty() && fallback.is_none() {
                    return Err(position.error("match must contain at least one branch"));
                }
                Statement::Match(value, arms, fallback)
            }
            _ => return Err(position.error("expected a control-flow statement")),
        })
    }
    fn function(&mut self) -> Result<Function, String> {
        self.function_owned(None)
    }
    fn function_owned(&mut self, owner: Option<(String, u32)>) -> Result<Function, String> {
        self.bindings.clear();
        self.types.clear();
        self.declarations.clear();
        self.result_name = None;
        self.loop_depth = 0;
        self.symbol_aliases.clear();
        let function_position = self.position();
        if let Some((_, id)) = &owner {
            self.bind(
                "self".into(),
                true,
                true,
                Some(Type::Class(*id)),
                function_position,
            )?;
        }
        self.word("fun")?;
        let source_name = self.name()?;
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
        let result = if source_name == "main" && owner.is_none() {
            if parameters != 0 {
                return Err(self.position().error("main must have no parameters"));
            }
            None
        } else if source_name == "__new__" && owner.is_some() {
            if parameters != 1 {
                return Err(self.position().error("__new__ must have no parameters"));
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
        let name = owner.as_ref().map_or_else(
            || source_name.clone(),
            |(class, _)| format!("{class}__{source_name}"),
        );
        Ok(Function {
            name,
            parameters,
            result,
            statements,
            positions,
            types: self.types.clone(),
            bindings: self.declarations.clone(),
            position: function_position,
            owner: owner.map(|(_, id)| id),
        })
    }
    fn enum_declaration(&mut self) -> Result<String, String> {
        self.word("enum")?;
        let position = self.position();
        let name = self.name()?;
        if self.enums.contains_key(&name) {
            return Err(position.error(format!("enum '{name}' is already declared")));
        }
        self.symbol('{')?;
        let mut variants = HashMap::new();
        if self.peek() == &Token::Symbol('}') {
            return Err(self
                .position()
                .error("enum must declare at least one variant"));
        }
        loop {
            let variant_position = self.position();
            let variant = self.name()?;
            let value = variants.len() as u32;
            if variants.insert(variant.clone(), value).is_some() {
                return Err(
                    variant_position.error(format!("enum variant '{variant}' is already declared"))
                );
            }
            if self.take(Token::Symbol('}')) {
                break;
            }
            self.symbol(',')?;
            if self.take(Token::Symbol('}')) {
                break;
            }
        }
        let id = self.enums.len() as u32;
        self.enums.insert(
            name.clone(),
            EnumDefinition {
                ty: Type::Enum(id),
                variants,
            },
        );
        Ok(name)
    }
    fn class_declaration(&mut self) -> Result<Vec<Function>, String> {
        self.word("class")?;
        let position = self.position();
        let name = self.name()?;
        if self.classes.contains_key(&name) {
            return Err(position.error(format!("class '{name}' is already declared")));
        }
        if self.enums.contains_key(&name) {
            return Err(position.error(format!("'{name}' is already declared as an enum")));
        }
        let id = self.classes.len() as u32;
        self.classes.insert(
            name.clone(),
            ClassDefinition {
                id,
                name: name.clone(),
                fields: Vec::new(),
                methods: Vec::new(),
            },
        );
        self.symbol('(')?;
        let mut fields = Vec::new();
        if !self.take(Token::Symbol(')')) {
            loop {
                let public = self.take(Token::Word("pub".into()));
                let field_position = self.position();
                let field = self.name()?;
                self.symbol(':')?;
                let ty = self.type_name()?;
                if matches!(ty, Type::Class(_)) {
                    return Err(field_position.error(
                        "class-typed fields are not supported yet because class copies must be independent",
                    ));
                }
                if fields.iter().any(|value: &ClassField| value.name == field) {
                    return Err(
                        field_position.error(format!("field '{field}' is already declared"))
                    );
                }
                fields.push(ClassField {
                    name: field,
                    ty,
                    public,
                });
                if self.take(Token::Symbol(')')) {
                    break;
                }
                self.symbol(',')?;
                if self.take(Token::Symbol(')')) {
                    break;
                }
            }
        }
        self.classes.get_mut(&name).unwrap().fields = fields;
        self.symbol('{')?;
        let mut functions = Vec::new();
        let mut methods = Vec::new();
        let mut has_constructor = false;
        while self.peek() != &Token::Symbol('}') {
            let public = self.take(Token::Word("pub".into()));
            if self.peek() != &Token::Word("fun".into()) {
                return Err(self.position().error("expected a class method"));
            }
            let method_name = match self.tokens.get(self.cursor + 1) {
                Some((Token::Word(value), _)) => value.clone(),
                _ => return Err(self.position().error("expected a method name")),
            };
            if methods
                .iter()
                .any(|value: &ClassMethod| value.name == method_name)
            {
                return Err(self
                    .position()
                    .error(format!("method '{method_name}' is already declared")));
            }
            if method_name == "__new__" {
                if public {
                    return Err(self.position().error("__new__ cannot be public"));
                }
                has_constructor = true;
            }
            let function = self.function_owned(Some((name.clone(), id)))?;
            methods.push(ClassMethod {
                name: method_name,
                function: function.name.clone(),
                public,
            });
            functions.push(function);
        }
        self.symbol('}')?;
        if !has_constructor {
            return Err(position.error(format!("class '{name}' must declare fun __new__()")));
        }
        self.classes.get_mut(&name).unwrap().methods = methods;
        Ok(functions)
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
        enums: HashMap::new(),
        classes: HashMap::new(),
        loop_depth: 0,
        symbol_aliases: HashMap::new(),
    };
    let mut functions = Vec::new();
    let mut signatures = HashMap::new();
    while parser.peek() != &Token::End {
        parser.reject_import()?;
        let position = parser.position();
        if parser.peek() == &Token::Word("enum".into()) {
            let name = parser.enum_declaration()?;
            if signatures.contains_key(&name) {
                return Err(position.error(format!("'{name}' is already declared as a function")));
            }
            if parser.classes.contains_key(&name) {
                return Err(position.error(format!("'{name}' is already declared as a class")));
            }
            continue;
        }
        if parser.peek() == &Token::Word("class".into()) {
            if let Some((Token::Word(class_name), _)) = parser.tokens.get(parser.cursor + 1)
                && signatures.contains_key(class_name)
            {
                return Err(
                    position.error(format!("'{class_name}' is already declared as a function"))
                );
            }
            let methods = parser.class_declaration()?;
            for method in methods {
                if signatures
                    .insert(method.name.clone(), method.parameters)
                    .is_some()
                {
                    return Err(position.error(format!(
                        "generated method name '{}' conflicts with a function",
                        method.name
                    )));
                }
                functions.push(method);
            }
            continue;
        }
        let function = parser.function()?;
        if parser.enums.contains_key(&function.name) {
            return Err(position.error(format!(
                "'{}' is already declared as an enum",
                function.name
            )));
        }
        if parser.classes.contains_key(&function.name) {
            return Err(position.error(format!(
                "'{}' is already declared as a class",
                function.name
            )));
        }
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
    fn validate_statement(
        statement: &Statement,
        signatures: &HashMap<String, usize>,
    ) -> Result<(), String> {
        match statement {
            Statement::Assign(_, expr) | Statement::Print(expr, _) => {
                validate_expr(expr, signatures)?
            }
            Statement::Noop(Some(function)) => {
                if !signatures.contains_key(function) {
                    return Err(format!("function '{function}' is not declared"));
                }
            }
            Statement::Disconnect(_, _) | Statement::Noop(None) => (),
            Statement::Clamp(_, low, high) => {
                validate_expr(low, signatures)?;
                validate_expr(high, signatures)?;
            }
            Statement::Call(call) => validate_call(call, signatures)?,
            Statement::SetField(object, _, value) => {
                validate_expr(object, signatures)?;
                validate_expr(value, signatures)?;
            }
            Statement::MethodCall(expr) => validate_expr(expr, signatures)?,
            Statement::If(condition, yes, no) => {
                validate_expr(condition, signatures)?;
                for statement in yes.iter().chain(no) {
                    validate_statement(statement, signatures)?;
                }
            }
            Statement::While(condition, body) | Statement::Until(condition, body) => {
                validate_expr(condition, signatures)?;
                for statement in body {
                    validate_statement(statement, signatures)?;
                }
            }
            Statement::Loop(body) => {
                for statement in body {
                    validate_statement(statement, signatures)?;
                }
            }
            Statement::For(_, start, end, body) => {
                validate_expr(start, signatures)?;
                validate_expr(end, signatures)?;
                for statement in body {
                    validate_statement(statement, signatures)?;
                }
            }
            Statement::Match(value, arms, fallback) => {
                validate_expr(value, signatures)?;
                for (pattern, body) in arms {
                    validate_expr(pattern, signatures)?;
                    for statement in body {
                        validate_statement(statement, signatures)?;
                    }
                }
                if let Some(body) = fallback {
                    for statement in body {
                        validate_statement(statement, signatures)?;
                    }
                }
            }
            _ => (),
        }
        Ok(())
    }
    for function in &functions {
        for statement in &function.statements {
            validate_statement(statement, &signatures)?;
        }
    }
    let mut classes = parser.classes.into_values().collect::<Vec<_>>();
    classes.sort_by_key(|class| class.id);
    Ok(Program { functions, classes })
}
fn validate_expr(expr: &Expr, signatures: &HashMap<String, usize>) -> Result<(), String> {
    match expr {
        Expr::Call(call) => validate_call(call, signatures),
        Expr::Binary(_, left, right) | Expr::Compare(_, left, right) => {
            validate_expr(left, signatures)?;
            validate_expr(right, signatures)
        }
        Expr::Negate(expr) | Expr::Positive(expr) | Expr::Annotated(expr, _) => {
            validate_expr(expr, signatures)
        }
        Expr::Construct(_, fields) => {
            for (_, value) in fields {
                validate_expr(value, signatures)?;
            }
            Ok(())
        }
        Expr::Field(object, _, _) => validate_expr(object, signatures),
        Expr::MethodCall(object, _, arguments, _) => {
            validate_expr(object, signatures)?;
            for argument in arguments {
                validate_expr(argument, signatures)?;
            }
            Ok(())
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
