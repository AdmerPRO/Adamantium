use crate::types::Type;
use std::collections::{HashMap, HashSet};

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

type PositionedToken = (Token, Position);
type GenericArguments = Vec<Vec<PositionedToken>>;
type GenericInstances = HashMap<String, Vec<(String, GenericArguments)>>;

#[derive(Clone, Copy, Debug)]
pub struct Position {
    line: usize,
    column: usize,
}
impl Position {
    pub fn error(self, message: impl std::fmt::Display) -> String {
        format!("{}:{}: {message}", self.line, self.column)
    }
    pub fn line(self) -> usize {
        self.line
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Operator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
}
#[derive(Clone, Copy, Debug)]
pub enum LogicalOperator {
    And,
    Or,
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
            '%' => Some(Self::Remainder),
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
    List(Vec<Expr>),
    Index(Box<Expr>, Box<Expr>, Position),
    Annotated(Box<Expr>, Type),
    Cast(Box<Expr>, Type, Position),
    Variable(usize),
    Field(Box<Expr>, String, Position),
    MethodCall(Box<Expr>, String, Vec<Expr>, Position),
    Negate(Box<Expr>),
    Positive(Box<Expr>),
    Not(Box<Expr>),
    Binary(Operator, Box<Expr>, Box<Expr>),
    Logical(LogicalOperator, Box<Expr>, Box<Expr>),
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
    Remove(usize),
    Clamp(usize, Expr, Expr),
    Print(Expr, bool),
    Call(Call),
    SetField(Expr, String, Expr),
    SetIndex(Expr, Expr, Expr),
    MethodCall(Expr),
    Message(Expr, bool, Position),
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
    pub optional_parameters: Vec<bool>,
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
    pub traits: Vec<String>,
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
#[derive(Clone)]
struct TraitMethod {
    name: String,
    parameters: Vec<Type>,
    result: Type,
}
struct TraitDefinition {
    methods: Vec<TraitMethod>,
}
struct Parser {
    tokens: Vec<(Token, Position)>,
    cursor: usize,
    bindings: HashMap<String, Binding>,
    removed_variables: HashSet<String>,
    result_name: Option<String>,
    types: Vec<Option<Type>>,
    declarations: Vec<(String, Position)>,
    enums: HashMap<String, EnumDefinition>,
    classes: HashMap<String, ClassDefinition>,
    traits: HashMap<String, TraitDefinition>,
    type_aliases: HashMap<String, Type>,
    loop_depth: usize,
    lifecycle_hook: Option<String>,
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
        } else if "(){}.;=,:+-*/%[]!<>|&$".contains(c) {
            Token::Symbol(c)
        } else {
            return Err(position.error(format!("unexpected character {c:?}")));
        };
        tokens.push((token, position));
    }
    tokens.push((Token::End, Position { line, column }));
    Ok(tokens)
}

#[derive(Clone)]
struct GenericTemplate {
    name: String,
    class: bool,
    parameters: Vec<(String, Option<String>)>,
    tokens: Vec<PositionedToken>,
    start: usize,
    end: usize,
}

fn generic_arguments(
    tokens: &[PositionedToken],
    start: usize,
) -> Result<(GenericArguments, usize), String> {
    let position = tokens[start].1;
    let mut arguments = vec![Vec::new()];
    let (mut angles, mut brackets) = (0usize, 0usize);
    let mut index = start + 1;
    while index < tokens.len() {
        match tokens[index].0 {
            Token::Symbol('<') => {
                angles += 1;
                arguments.last_mut().unwrap().push(tokens[index].clone());
            }
            Token::Symbol('>') if angles == 0 && brackets == 0 => {
                if arguments.last().is_some_and(Vec::is_empty) {
                    return Err(position.error("generic argument list cannot be empty"));
                }
                return Ok((arguments, index + 1));
            }
            Token::Symbol('>') => {
                angles = angles.saturating_sub(1);
                arguments.last_mut().unwrap().push(tokens[index].clone());
            }
            Token::Symbol('[') => {
                brackets += 1;
                arguments.last_mut().unwrap().push(tokens[index].clone());
            }
            Token::Symbol(']') => {
                brackets = brackets.saturating_sub(1);
                arguments.last_mut().unwrap().push(tokens[index].clone());
            }
            Token::Symbol(',') if angles == 0 && brackets == 0 => arguments.push(Vec::new()),
            _ => arguments.last_mut().unwrap().push(tokens[index].clone()),
        }
        index += 1;
    }
    Err(position.error("unterminated generic argument list; expected '>'"))
}

fn generic_type_name(tokens: &[(Token, Position)]) -> String {
    tokens
        .iter()
        .map(|(token, _)| match token {
            Token::Word(value) | Token::Number(value) => value.clone(),
            Token::Symbol(value) => value.to_string(),
            _ => "type".into(),
        })
        .collect::<String>()
}

fn validate_generic_constraint(
    argument: &[(Token, Position)],
    constraint: Option<&str>,
    position: Position,
    traits: &HashSet<String>,
    implementations: &HashSet<(String, String)>,
) -> Result<(), String> {
    let name = generic_type_name(argument);
    let accepted = match constraint {
        None | Some("any") => true,
        Some("numeric") => matches!(
            name.as_str(),
            "i8" | "i16"
                | "i32"
                | "i64"
                | "int"
                | "u4"
                | "u8"
                | "u16"
                | "u32"
                | "u64"
                | "u"
                | "f32"
                | "f64"
                | "f128"
                | "float"
        ),
        Some("integer") => matches!(
            name.as_str(),
            "i8" | "i16" | "i32" | "i64" | "int" | "u4" | "u8" | "u16" | "u32" | "u64" | "u"
        ),
        Some("float") => matches!(name.as_str(), "f32" | "f64" | "f128" | "float"),
        Some("comparable") => !matches!(name.as_str(), "None") && !name.starts_with("List["),
        Some(other) if traits.contains(other) => {
            implementations.contains(&(name.clone(), other.to_string()))
        }
        Some(other) => return Err(position.error(format!("unknown generic constraint '{other}'"))),
    };
    if accepted {
        Ok(())
    } else {
        Err(position.error(format!(
            "type '{name}' does not satisfy generic constraint '{}'",
            constraint.unwrap()
        )))
    }
}

fn expand_generics(tokens: Vec<(Token, Position)>) -> Result<Vec<(Token, Position)>, String> {
    let traits = tokens
        .windows(2)
        .filter_map(|tokens| match (&tokens[0].0, &tokens[1].0) {
            (Token::Word(keyword), Token::Word(name)) if keyword == "trait" => Some(name.clone()),
            _ => None,
        })
        .collect::<HashSet<_>>();
    let mut implementations = HashSet::new();
    for index in 0..tokens.len().saturating_sub(2) {
        if tokens[index].0 != Token::Word("class".into()) {
            continue;
        }
        let Token::Word(class) = &tokens[index + 1].0 else {
            continue;
        };
        let Some(implements) = (index + 2..tokens.len())
            .take_while(|cursor| tokens[*cursor].0 != Token::Symbol('{'))
            .find(|cursor| tokens[*cursor].0 == Token::Word("implements".into()))
        else {
            continue;
        };
        for (token, _) in &tokens[implements + 1..] {
            match token {
                Token::Word(trait_name) => {
                    implementations.insert((class.clone(), trait_name.clone()));
                }
                Token::Symbol(',') => (),
                Token::Symbol('{') => break,
                _ => break,
            }
        }
    }
    let mut templates = Vec::new();
    let mut depth = 0usize;
    let mut index = 0usize;
    while index + 3 < tokens.len() {
        match tokens[index].0 {
            Token::Symbol('{') => depth += 1,
            Token::Symbol('}') => depth = depth.saturating_sub(1),
            _ => (),
        }
        if depth == 0
            && matches!(&tokens[index].0, Token::Word(word) if word == "fun" || word == "class")
            && matches!(tokens[index + 1].0, Token::Word(_))
            && tokens[index + 2].0 == Token::Symbol('<')
        {
            let Token::Word(name) = &tokens[index + 1].0 else {
                unreachable!()
            };
            let (parameter_tokens, header_end) = generic_arguments(&tokens, index + 2)?;
            let mut parameters = Vec::new();
            for parameter in parameter_tokens {
                let Some((Token::Word(parameter_name), _)) = parameter.first() else {
                    return Err(tokens[index + 2]
                        .1
                        .error("expected a generic parameter name"));
                };
                let constraint = match parameter.as_slice() {
                    [_] => None,
                    [_, (Token::Symbol(':'), _), (Token::Word(value), _)] => Some(value.clone()),
                    _ => return Err(parameter[0].1.error("expected 'T' or 'T:constraint'")),
                };
                if parameters
                    .iter()
                    .any(|(existing, _)| existing == parameter_name)
                {
                    return Err(parameter[0].1.error(format!(
                        "generic parameter '{parameter_name}' is declared twice"
                    )));
                }
                parameters.push((parameter_name.clone(), constraint));
            }
            let body = (header_end..tokens.len())
                .find(|cursor| tokens[*cursor].0 == Token::Symbol('{'))
                .ok_or_else(|| tokens[index].1.error("generic declaration requires a body"))?;
            let mut body_depth = 1usize;
            let mut end = body + 1;
            while end < tokens.len() && body_depth != 0 {
                match tokens[end].0 {
                    Token::Symbol('{') => body_depth += 1,
                    Token::Symbol('}') => body_depth -= 1,
                    _ => (),
                }
                end += 1;
            }
            if body_depth != 0 {
                return Err(tokens[body].1.error("unterminated generic declaration"));
            }
            let mut template_tokens = tokens[index..end].to_vec();
            template_tokens.drain(2..header_end - index);
            templates.push(GenericTemplate {
                name: name.clone(),
                class: tokens[index].0 == Token::Word("class".into()),
                parameters,
                tokens: template_tokens,
                start: index,
                end,
            });
            index = end;
            continue;
        }
        index += 1;
    }
    if templates.is_empty() {
        return Ok(tokens);
    }

    let mut instances = GenericInstances::new();
    let mut rewritten = Vec::new();
    index = 0;
    while index < tokens.len() {
        if let Some(template) = templates.iter().find(|template| template.start == index) {
            index = template.end;
            continue;
        }
        if let Token::Word(name) = &tokens[index].0
            && let Some(template) = templates.iter().find(|template| &template.name == name)
        {
            if tokens
                .get(index + 1)
                .is_some_and(|token| token.0 == Token::Symbol('<'))
            {
                let (arguments, end) = generic_arguments(&tokens, index + 1)?;
                if arguments.len() != template.parameters.len() {
                    return Err(tokens[index].1.error(format!(
                        "generic '{}' expects {} type arguments, found {}",
                        name,
                        template.parameters.len(),
                        arguments.len()
                    )));
                }
                for (argument, (_, constraint)) in arguments.iter().zip(&template.parameters) {
                    validate_generic_constraint(
                        argument,
                        constraint.as_deref(),
                        tokens[index].1,
                        &traits,
                        &implementations,
                    )?;
                }
                let suffix = arguments
                    .iter()
                    .map(|argument| {
                        generic_type_name(argument)
                            .replace(|c: char| !c.is_ascii_alphanumeric(), "_")
                    })
                    .collect::<Vec<_>>()
                    .join("__");
                let specialized = format!("{name}__generic__{suffix}");
                let values = instances.entry(name.clone()).or_default();
                if !values.iter().any(|(existing, _)| existing == &specialized) {
                    values.push((specialized.clone(), arguments));
                }
                rewritten.push((Token::Word(specialized), tokens[index].1));
                index = end;
                continue;
            }
            return Err(tokens[index].1.error(format!(
                "generic '{name}' requires explicit type arguments, for example {name}<int>"
            )));
        }
        rewritten.push(tokens[index].clone());
        index += 1;
    }

    let end_token = rewritten.pop().filter(|token| token.0 == Token::End);
    let mut generated_classes = Vec::new();
    let mut generated_functions = Vec::new();
    for template in &templates {
        for (specialized, arguments) in instances.remove(&template.name).unwrap_or_default() {
            let generated = if template.class {
                &mut generated_classes
            } else {
                &mut generated_functions
            };
            let substitutions = template
                .parameters
                .iter()
                .map(|(name, _)| name)
                .zip(arguments.iter())
                .collect::<HashMap<_, _>>();
            for (token_index, token) in template.tokens.iter().enumerate() {
                if token_index == 1 {
                    generated.push((Token::Word(specialized.clone()), token.1));
                } else if let Token::Word(word) = &token.0
                    && let Some(replacement) = substitutions.get(word)
                {
                    generated.extend((*replacement).clone());
                } else {
                    generated.push(token.clone());
                }
            }
        }
    }
    generated_classes.extend(rewritten);
    generated_classes.extend(generated_functions);
    let mut rewritten = generated_classes;
    rewritten.push(end_token.unwrap_or((Token::End, Position { line: 1, column: 1 })));
    Ok(rewritten)
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
                    "not",
                    "or",
                    "and",
                    "panic",
                    "warn",
                    "define",
                    "trait",
                    "implements",
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
        if self.type_aliases.contains_key(&name) {
            return Err(position.error(format!(
                "variable '{name}' conflicts with a type alias of the same name"
            )));
        }
        if self.bindings.contains_key(&name) || self.symbol_aliases.contains_key(&name) {
            return Err(position.error(format!("variable '{name}' is already declared")));
        }
        self.removed_variables.remove(&name);
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
    fn bare_function_target(&self, alias_name: &str) -> Option<String> {
        let Some((Token::Word(name), _)) = self.tokens.get(self.cursor) else {
            return None;
        };
        let empty_call = self.tokens.get(self.cursor + 1).map(|token| &token.0)
            == Some(&Token::Symbol('('))
            && self.tokens.get(self.cursor + 2).map(|token| &token.0) == Some(&Token::Symbol(')'))
            && self.tokens.get(self.cursor + 3).map(|token| &token.0) == Some(&Token::Symbol(';'));
        let mut depth = 0;
        let mut called_later = false;
        let mut index = self.cursor + 4;
        while index + 1 < self.tokens.len() {
            match self.tokens[index].0 {
                Token::Symbol('{') => depth += 1,
                Token::Symbol('}') if depth == 0 => break,
                Token::Symbol('}') => depth -= 1,
                _ => (),
            }
            if self.tokens[index].0 == Token::Word(alias_name.into())
                && self.tokens[index + 1].0 == Token::Symbol('(')
            {
                called_later = true;
                break;
            }
            index += 1;
        }
        (empty_call && called_later).then(|| name.clone())
    }
    fn variable(&self, name: &str, read: bool, position: Position) -> Result<usize, String> {
        let binding = self.bindings.get(name).ok_or_else(|| {
            if self.removed_variables.contains(name) {
                position.error(format!("variable '{name}' was removed"))
            } else {
                position.error(format!("variable '{name}' is not declared"))
            }
        })?;
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
        if name == "List" {
            self.symbol('[')?;
            let element = self.type_name()?;
            self.symbol(']')?;
            if element.id() > u32::MAX >> 3 {
                return Err(position.error("type nesting exceeds the supported depth"));
            }
            return Ok(Type::List(element.id()));
        }
        Type::parse(&name)
            .or_else(|| self.type_aliases.get(&name).copied())
            .or_else(|| self.enums.get(&name).map(|definition| definition.ty))
            .or_else(|| {
                self.classes
                    .get(&name)
                    .map(|definition| Type::Class(definition.id))
            })
            .ok_or_else(|| position.error(format!("unsupported type '{name}'")))
    }
    fn type_alias_declaration(&mut self) -> Result<String, String> {
        self.word("define")?;
        let position = self.position();
        let name = self.name()?;
        if self.type_aliases.contains_key(&name)
            || self.enums.contains_key(&name)
            || self.classes.contains_key(&name)
            || self.traits.contains_key(&name)
        {
            return Err(position.error(format!("type name '{name}' is already declared")));
        }
        self.symbol('=')?;
        let ty = self.type_name()?;
        self.symbol(';')?;
        self.type_aliases.insert(name.clone(), ty);
        Ok(name)
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
            Token::Word(value) if value == "List" => {
                self.symbol('[')?;
                let mut values = Vec::new();
                if !self.take(Token::Symbol(']')) {
                    loop {
                        values.push(self.expression(0)?);
                        if self.take(Token::Symbol(']')) {
                            break;
                        }
                        self.symbol(',')?;
                    }
                }
                Expr::List(values)
            }
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
                    Expr::Negate(Box::new(self.expression(5)?))
                }
            }
            Token::Symbol('+') => Expr::Positive(Box::new(self.expression(5)?)),
            Token::Symbol('!') => Expr::Not(Box::new(self.expression(5)?)),
            Token::Word(value) if value == "not" => Expr::Not(Box::new(self.expression(5)?)),
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
        while self.take(Token::Symbol('[')) {
            let position = self.position();
            let index = self.expression(0)?;
            self.symbol(']')?;
            left = Expr::Index(Box::new(left), Box::new(index), position);
        }
        while self.peek() == &Token::Symbol('.')
            && self
                .tokens
                .get(self.cursor + 1)
                .is_some_and(|(token, _)| token != &Token::Symbol('.'))
        {
            self.next();
            let position = self.position();
            let member = self.name()?;
            if member == "as" {
                self.symbol('(')?;
                let ty = self.type_name()?;
                self.symbol(')')?;
                left = Expr::Cast(Box::new(left), ty, position);
            } else if self.take(Token::Symbol('(')) {
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
        loop {
            let next = self.tokens.get(self.cursor + 1).map(|x| &x.0);
            let comparison = match (self.peek(), next) {
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
            let arithmetic = match self.peek() {
                Token::Symbol(c) => Operator::from_char(*c),
                _ => None,
            };
            let logical = match (self.peek(), next) {
                (Token::Word(word), _) if word == "and" => Some((LogicalOperator::And, false)),
                (Token::Word(word), _) if word == "or" => Some((LogicalOperator::Or, false)),
                (Token::Symbol('&'), Some(Token::Symbol('&'))) => {
                    Some((LogicalOperator::And, true))
                }
                (Token::Symbol('|'), Some(Token::Symbol('|'))) => Some((LogicalOperator::Or, true)),
                _ => None,
            };
            let (precedence, consume, kind) = if let Some((op, two)) = logical {
                (
                    if matches!(op, LogicalOperator::And) {
                        1
                    } else {
                        0
                    },
                    if two { 2 } else { 1 },
                    2,
                )
            } else if let Some((_, two)) = comparison {
                (2, if two { 2 } else { 1 }, 1)
            } else if let Some(op) = arithmetic {
                (
                    if matches!(
                        op,
                        Operator::Multiply | Operator::Divide | Operator::Remainder
                    ) {
                        4
                    } else {
                        3
                    },
                    1,
                    0,
                )
            } else {
                break;
            };
            if precedence < min_precedence {
                break;
            }
            for _ in 0..consume {
                self.next();
            }
            let right = self.expression(precedence + 1)?;
            left = match kind {
                0 => Expr::Binary(arithmetic.unwrap(), Box::new(left), Box::new(right)),
                1 => Expr::Compare(comparison.unwrap().0, Box::new(left), Box::new(right)),
                _ => Expr::Logical(logical.unwrap().0, Box::new(left), Box::new(right)),
            };
        }
        if min_precedence == 0 && self.take(Token::Symbol(':')) {
            left = Expr::Annotated(Box::new(left), self.type_name()?);
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
                if let Some(target) = self.bare_function_target(&name) {
                    if !changeable {
                        return Err(position.error("function values cannot be static"));
                    }
                    self.next();
                    self.next();
                    self.next();
                    if self.bindings.contains_key(&name) || self.symbol_aliases.contains_key(&name)
                    {
                        return Err(
                            position.error(format!("variable '{name}' is already declared"))
                        );
                    }
                    self.removed_variables.remove(&name);
                    self.symbol_aliases
                        .insert(name, SymbolAlias::Function(target.clone()));
                    self.symbol(';')?;
                    return Ok(Statement::Noop(Some(target)));
                }
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
                    self.removed_variables.remove(&name);
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
                    self.removed_variables.remove(&name);
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
            Token::Word(word) if word == "panic" || word == "warn" => {
                self.symbol('(')?;
                let message = self.expression(0)?;
                self.symbol(')')?;
                Statement::Message(message, word == "panic", position)
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
                if self.removed_variables.contains(&name) {
                    return Err(position.error(format!("variable '{name}' was removed")));
                }
                if self.symbol_aliases.contains_key(&name) {
                    if self.peek() == &Token::Symbol('.')
                        && self
                            .tokens
                            .get(self.cursor + 1)
                            .is_some_and(|(token, _)| token == &Token::Word("remove".into()))
                    {
                        self.next();
                        self.next();
                        self.symbol_aliases.remove(&name);
                        self.removed_variables.insert(name);
                        Statement::Noop(None)
                    } else if self.peek() == &Token::Symbol('(') {
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
                } else if self.peek() == &Token::Symbol('[') {
                    let slot = self.writable_variable(&name, position)?;
                    self.next();
                    let index = self.expression(0)?;
                    self.symbol(']')?;
                    self.symbol('=')?;
                    Statement::SetIndex(Expr::Variable(slot), index, self.expression(0)?)
                } else if self.peek() == &Token::Symbol('(') {
                    Statement::Call(self.arguments(name, position)?)
                } else if self.take(Token::Symbol('.')) {
                    let slot = self.variable(&name, true, position)?;
                    let member_position = self.position();
                    let member = self.name()?;
                    if member == "remove" {
                        if self.lifecycle_hook.as_deref() == Some("__remove__")
                            && matches!(self.types[slot], Some(Type::Class(_)))
                        {
                            return Err(
                                position.error("__remove__ cannot remove an object recursively")
                            );
                        }
                        self.bindings.remove(&name);
                        self.removed_variables.insert(name);
                        Statement::Remove(slot)
                    } else if member == "disconect" || member == "disconnect" {
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
                        if name == "self" && self.lifecycle_hook.as_deref() == Some("__change__") {
                            return Err(position.error("__change__ cannot modify self recursively"));
                        }
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
        self.removed_variables.clear();
        self.types.clear();
        self.declarations.clear();
        self.result_name = None;
        self.loop_depth = 0;
        self.lifecycle_hook = None;
        self.symbol_aliases.clear();
        let mut optional_parameters = Vec::new();
        let function_position = self.position();
        if let Some((_, id)) = &owner {
            self.bind(
                "self".into(),
                true,
                true,
                Some(Type::Class(*id)),
                function_position,
            )?;
            optional_parameters.push(false);
        }
        self.word("fun")?;
        let source_name = self.name()?;
        if owner.is_some()
            && matches!(
                source_name.as_str(),
                "__new__" | "__change__" | "__remove__"
            )
        {
            self.lifecycle_hook = Some(source_name.clone());
        }
        self.symbol('(')?;
        if !self.take(Token::Symbol(')')) {
            let mut found_optional = false;
            loop {
                let position = self.position();
                let optional = self.take(Token::Symbol('$'));
                if found_optional && !optional {
                    return Err(
                        position.error("required parameters cannot follow optional parameters")
                    );
                }
                found_optional |= optional;
                let parameter = self.name()?;
                self.symbol(':')?;
                let mut ty = self.type_name()?;
                if optional {
                    if ty.id() > u32::MAX >> 3 {
                        return Err(position.error("type nesting exceeds the supported depth"));
                    }
                    ty = Type::Optional(ty.id());
                }
                self.bind(parameter, true, true, Some(ty), position)?;
                optional_parameters.push(optional);
                if self.take(Token::Symbol(')')) {
                    break;
                }
                self.symbol(',')?;
            }
        }
        let parameters = self.bindings.len();
        let result = if source_name == "main" && owner.is_none() {
            for ty in &self.types[..parameters] {
                let ty = ty.unwrap();
                let inner = if let Type::Optional(inner) = ty {
                    Type::from_id(inner).unwrap()
                } else {
                    ty
                };
                if matches!(
                    inner,
                    Type::Class(_) | Type::List(_) | Type::Enum(_) | Type::None
                ) {
                    return Err(self.position().error(format!(
                        "main parameter type {inner} cannot be read from the command line"
                    )));
                }
            }
            None
        } else if owner.is_some()
            && matches!(
                source_name.as_str(),
                "__new__" | "__change__" | "__remove__"
            )
        {
            if parameters != 1 {
                return Err(self
                    .position()
                    .error(format!("{source_name} must have no parameters")));
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
            optional_parameters,
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
        if self.type_aliases.contains_key(&name) {
            return Err(position.error(format!("'{name}' is already declared as a type alias")));
        }
        if self.traits.contains_key(&name) {
            return Err(position.error(format!("'{name}' is already declared as a trait")));
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
    fn trait_declaration(&mut self) -> Result<String, String> {
        self.word("trait")?;
        let position = self.position();
        let name = self.name()?;
        if self.traits.contains_key(&name)
            || self.classes.contains_key(&name)
            || self.enums.contains_key(&name)
            || self.type_aliases.contains_key(&name)
        {
            return Err(position.error(format!("type name '{name}' is already declared")));
        }
        self.symbol('{')?;
        let mut methods = Vec::new();
        while !self.take(Token::Symbol('}')) {
            let method_position = self.position();
            self.word("fun")?;
            let method_name = self.name()?;
            if matches!(
                method_name.as_str(),
                "__new__" | "__change__" | "__remove__"
            ) {
                return Err(method_position.error("lifecycle hooks cannot be trait requirements"));
            }
            self.symbol('(')?;
            let mut parameters = Vec::new();
            if !self.take(Token::Symbol(')')) {
                loop {
                    let optional = self.take(Token::Symbol('$'));
                    self.name()?;
                    self.symbol(':')?;
                    let mut ty = self.type_name()?;
                    if optional {
                        ty = Type::Optional(ty.id());
                    }
                    parameters.push(ty);
                    if self.take(Token::Symbol(')')) {
                        break;
                    }
                    self.symbol(',')?;
                }
            }
            self.name()?;
            self.symbol(':')?;
            let result = self.type_name()?;
            self.symbol(';')?;
            if methods
                .iter()
                .any(|method: &TraitMethod| method.name == method_name)
            {
                return Err(method_position
                    .error(format!("trait method '{method_name}' is declared twice")));
            }
            methods.push(TraitMethod {
                name: method_name,
                parameters,
                result,
            });
        }
        if methods.is_empty() {
            return Err(position.error("trait must require at least one method"));
        }
        self.traits
            .insert(name.clone(), TraitDefinition { methods });
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
        if self.type_aliases.contains_key(&name) {
            return Err(position.error(format!("'{name}' is already declared as a type alias")));
        }
        if self.traits.contains_key(&name) {
            return Err(position.error(format!("'{name}' is already declared as a trait")));
        }
        let id = self.classes.len() as u32;
        self.classes.insert(
            name.clone(),
            ClassDefinition {
                id,
                name: name.clone(),
                fields: Vec::new(),
                methods: Vec::new(),
                traits: Vec::new(),
            },
        );
        self.symbol('(')?;
        let mut fields = Vec::new();
        if !self.take(Token::Symbol(')')) {
            loop {
                let public = self.take(Token::Word("pub".into()));
                let field_position = self.position();
                let optional = self.take(Token::Symbol('&'));
                let field = self.name()?;
                self.symbol(':')?;
                let mut ty = self.type_name()?;
                if matches!(ty, Type::Class(_)) && !optional {
                    return Err(field_position.error(
                        "class-typed fields are not supported yet because class copies must be independent",
                    ));
                }
                if optional {
                    if ty.id() > u32::MAX >> 3 {
                        return Err(
                            field_position.error("type nesting exceeds the supported depth")
                        );
                    }
                    ty = Type::Optional(ty.id());
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
        let mut implemented_traits = Vec::new();
        if self.take(Token::Word("implements".into())) {
            loop {
                let trait_position = self.position();
                let trait_name = self.name()?;
                if !self.traits.contains_key(&trait_name) {
                    return Err(
                        trait_position.error(format!("trait '{trait_name}' is not declared"))
                    );
                }
                if implemented_traits.contains(&trait_name) {
                    return Err(
                        trait_position.error(format!("trait '{trait_name}' is implemented twice"))
                    );
                }
                implemented_traits.push(trait_name);
                if !self.take(Token::Symbol(',')) {
                    break;
                }
            }
        }
        self.classes.get_mut(&name).unwrap().traits = implemented_traits.clone();
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
            if matches!(
                method_name.as_str(),
                "__new__" | "__change__" | "__remove__"
            ) && public
            {
                return Err(self
                    .position()
                    .error(format!("{method_name} cannot be public")));
            }
            if method_name == "__new__" {
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
        for trait_name in &implemented_traits {
            for requirement in &self.traits[trait_name].methods {
                let method = methods
                    .iter()
                    .find(|method| method.name == requirement.name)
                    .ok_or_else(|| {
                        position.error(format!(
                            "class '{name}' implements '{trait_name}' but is missing method '{}'",
                            requirement.name
                        ))
                    })?;
                if !method.public {
                    return Err(position.error(format!(
                        "trait method '{}.{}' must be public",
                        trait_name, requirement.name
                    )));
                }
                let function = functions
                    .iter()
                    .find(|function| function.name == method.function)
                    .unwrap();
                let parameters = function.types[1..function.parameters]
                    .iter()
                    .map(|ty| ty.unwrap())
                    .collect::<Vec<_>>();
                let result = function
                    .result
                    .map(|slot| function.types[slot].unwrap())
                    .unwrap_or(Type::None);
                if parameters != requirement.parameters || result != requirement.result {
                    return Err(position.error(format!(
                        "method '{}.{}' does not match the required signature",
                        trait_name, requirement.name
                    )));
                }
            }
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

pub fn module_dependencies(source: &str) -> Result<Vec<String>, String> {
    let tokens = lex(source)?;
    let mut dependencies = Vec::new();
    let mut index = 0;
    let mut depth = 0;
    while index < tokens.len() {
        match &tokens[index].0 {
            Token::Symbol('{') => depth += 1,
            Token::Symbol('}') => depth -= 1,
            Token::Word(word) if word == "pack" && depth == 0 => {
                let position = tokens[index].1;
                index += 1;
                let mut parts = Vec::new();
                loop {
                    let Some((Token::Word(part), _)) = tokens.get(index) else {
                        return Err(position.error("expected module path after 'pack'"));
                    };
                    parts.push(part.clone());
                    index += 1;
                    if !matches!(tokens.get(index), Some((Token::Symbol('/'), _))) {
                        break;
                    }
                    index += 1;
                }
                if !matches!(tokens.get(index), Some((Token::Symbol(';'), _))) {
                    return Err(position.error("expected ';' after module path"));
                }
                dependencies.push(parts.join("/"));
            }
            _ => (),
        }
        index += 1;
    }
    Ok(dependencies)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ExportKind {
    Function,
    Enum,
    Class,
    TypeAlias,
    Trait,
}

fn module_prefix(module: &str) -> String {
    format!("admod__{}__", module.replace('/', "__"))
}

pub fn parse_modules(files: &[(String, String)]) -> Result<Program, String> {
    let mut token_files = Vec::new();
    let mut exports = HashMap::new();
    for (module, source) in files {
        let tokens = lex(source)?;
        let mut depth = 0;
        for index in 0..tokens.len().saturating_sub(1) {
            match &tokens[index].0 {
                Token::Symbol('{') => depth += 1,
                Token::Symbol('}') => depth -= 1,
                Token::Word(keyword)
                    if depth == 0
                        && matches!(
                            keyword.as_str(),
                            "fun" | "enum" | "class" | "define" | "trait"
                        ) =>
                {
                    if let Token::Word(name) = &tokens[index + 1].0 {
                        let kind = match keyword.as_str() {
                            "fun" => ExportKind::Function,
                            "enum" => ExportKind::Enum,
                            "class" => ExportKind::Class,
                            "define" => ExportKind::TypeAlias,
                            _ => ExportKind::Trait,
                        };
                        let canonical = if module.is_empty() {
                            name.clone()
                        } else {
                            format!("{}{name}", module_prefix(module))
                        };
                        let public = kind != ExportKind::Enum
                            || (index > 0 && tokens[index - 1].0 == Token::Word("pub".into()));
                        if exports
                            .insert((module.clone(), name.clone()), (canonical, kind, public))
                            .is_some()
                        {
                            return Err(tokens[index + 1].1.error(format!(
                                "'{name}' is declared twice in module '{module}'"
                            )));
                        }
                    }
                }
                _ => (),
            }
        }
        token_files.push((module, tokens));
    }

    let mut combined = Vec::new();
    for (module, tokens) in token_files {
        let local = exports
            .iter()
            .filter(|((owner, _), _)| owner == module)
            .map(|((_, name), value)| (name.clone(), value.clone()))
            .collect::<HashMap<_, _>>();
        let mut imports = HashMap::new();
        let mut removed = HashSet::new();
        let mut index = 0;
        let mut depth = 0;
        while index < tokens.len() {
            match &tokens[index].0 {
                Token::Symbol('{') => depth += 1,
                Token::Symbol('}') => depth -= 1,
                Token::Word(keyword) if depth == 0 && keyword == "pack" => {
                    while index < tokens.len() && tokens[index].0 != Token::Symbol(';') {
                        removed.insert(index);
                        index += 1;
                    }
                    removed.insert(index);
                }
                Token::Word(keyword) if depth == 0 && keyword == "use" => {
                    let start = index;
                    index += 1;
                    let mut parts = Vec::new();
                    loop {
                        let Some((Token::Word(part), position)) = tokens.get(index) else {
                            return Err(tokens[start].1.error("expected module path after 'use'"));
                        };
                        let _ = position;
                        parts.push(part.clone());
                        index += 1;
                        if !matches!(tokens.get(index), Some((Token::Symbol('/'), _))) {
                            break;
                        }
                        index += 1;
                    }
                    if !matches!(tokens.get(index), Some((Token::Symbol(':'), _)))
                        || !matches!(tokens.get(index + 1), Some((Token::Symbol('['), _)))
                    {
                        return Err(tokens[start].1.error("expected ':[' after module path"));
                    }
                    index += 2;
                    loop {
                        let Some((Token::Word(name), position)) = tokens.get(index) else {
                            return Err(tokens[start].1.error("expected imported symbol name"));
                        };
                        let path = parts.join("/");
                        let exported = exports
                            .get(&(path.clone(), name.clone()))
                            .cloned()
                            .ok_or_else(|| {
                                position.error(format!("module '{path}' does not export '{name}'"))
                            })?;
                        if !exported.2 {
                            return Err(position
                                .error(format!("enum '{name}' is private in module '{path}'")));
                        }
                        if local.contains_key(name)
                            || imports.insert(name.clone(), exported).is_some()
                        {
                            return Err(position.error(format!(
                                "imported name '{name}' conflicts with another name"
                            )));
                        }
                        index += 1;
                        if matches!(tokens.get(index), Some((Token::Symbol(']'), _))) {
                            index += 1;
                            break;
                        }
                        if !matches!(tokens.get(index), Some((Token::Symbol(','), _))) {
                            return Err(tokens[index].1.error("expected ',' or ']' in use list"));
                        }
                        index += 1;
                    }
                    if !matches!(tokens.get(index), Some((Token::Symbol(';'), _))) {
                        return Err(tokens[start].1.error("expected ';' after use declaration"));
                    }
                    for removed_index in start..=index {
                        removed.insert(removed_index);
                    }
                }
                _ => (),
            }
            index += 1;
        }

        let mut index = 0;
        let mut depth = 0;
        while index < tokens.len() {
            if removed.contains(&index) {
                index += 1;
                continue;
            }
            if tokens[index].0 == Token::End {
                break;
            }
            if depth == 0
                && tokens[index].0 == Token::Word("pub".into())
                && tokens
                    .get(index + 1)
                    .is_some_and(|token| token.0 == Token::Word("enum".into()))
            {
                index += 1;
                continue;
            }
            let position = tokens[index].1;
            if let Token::Word(first) = &tokens[index].0 {
                let mut path_parts = vec![first.clone()];
                let mut cursor = index + 1;
                while matches!(tokens.get(cursor), Some((Token::Symbol('/'), _)))
                    && matches!(tokens.get(cursor + 1), Some((Token::Word(_), _)))
                {
                    if let Token::Word(part) = &tokens[cursor + 1].0 {
                        path_parts.push(part.clone());
                    }
                    cursor += 2;
                }
                if matches!(tokens.get(cursor), Some((Token::Symbol(':'), _)))
                    && let Some((Token::Word(name), _)) = tokens.get(cursor + 1)
                    && let Some((canonical, _, public)) =
                        exports.get(&(path_parts.join("/"), name.clone()))
                {
                    if !public && path_parts.join("/") != module.as_str() {
                        return Err(position.error(format!(
                            "enum '{name}' is private in module '{}'",
                            path_parts.join("/")
                        )));
                    }
                    combined.push((Token::Word(canonical.clone()), position));
                    index = cursor + 2;
                    continue;
                }
            }
            let mut token = tokens[index].0.clone();
            if let Token::Word(name) = &token
                && let Some((canonical, kind, _)) = local.get(name).or_else(|| imports.get(name))
            {
                let declaration = depth == 0
                    && index > 0
                    && matches!(&tokens[index - 1].0, Token::Word(word) if matches!(word.as_str(), "fun" | "enum" | "class" | "define" | "trait"));
                let next = tokens.get(index + 1).map(|value| &value.0);
                let previous = index
                    .checked_sub(1)
                    .and_then(|i| tokens.get(i))
                    .map(|value| &value.0);
                let reference = match kind {
                    ExportKind::Function => {
                        matches!(next, Some(Token::Symbol('(') | Token::Symbol('<')))
                    }
                    ExportKind::Enum => {
                        matches!(next, Some(Token::Symbol('.')))
                            || matches!(previous, Some(Token::Symbol(':')))
                    }
                    ExportKind::Class => {
                        matches!(next, Some(Token::Symbol('(') | Token::Symbol('<')))
                            || matches!(previous, Some(Token::Symbol(':')))
                    }
                    ExportKind::TypeAlias => {
                        matches!(
                            previous,
                            Some(Token::Symbol(':') | Token::Symbol('[') | Token::Symbol('='))
                        ) || (matches!(previous, Some(Token::Symbol('(')))
                            && index >= 2
                            && tokens[index - 2].0 == Token::Word("as".into()))
                    }
                    ExportKind::Trait => {
                        matches!(previous, Some(Token::Word(word)) if word == "implements")
                            || matches!(previous, Some(Token::Symbol(':')))
                            || (matches!(previous, Some(Token::Symbol(',')))
                                && tokens[..index]
                                    .iter()
                                    .rev()
                                    .take_while(|(token, _)| {
                                        !matches!(token, Token::Symbol('{') | Token::Symbol(';'))
                                    })
                                    .any(|(token, _)| token == &Token::Word("implements".into())))
                    }
                };
                if declaration || reference {
                    token = Token::Word(canonical.clone());
                }
            }
            match token {
                Token::Symbol('{') => depth += 1,
                Token::Symbol('}') => depth -= 1,
                _ => (),
            }
            combined.push((token, position));
            index += 1;
        }
    }
    combined.push((Token::End, Position { line: 1, column: 1 }));
    parse_tokens(combined)
}

#[cfg(test)]
pub fn parse(source: &str) -> Result<Program, String> {
    parse_tokens(lex(source)?)
}

fn parse_tokens(tokens: Vec<(Token, Position)>) -> Result<Program, String> {
    let tokens = expand_generics(tokens)?;
    let mut parser = Parser {
        tokens,
        cursor: 0,
        bindings: HashMap::new(),
        removed_variables: HashSet::new(),
        result_name: None,
        types: Vec::new(),
        declarations: Vec::new(),
        enums: HashMap::new(),
        classes: HashMap::new(),
        traits: HashMap::new(),
        type_aliases: HashMap::new(),
        loop_depth: 0,
        lifecycle_hook: None,
        symbol_aliases: HashMap::new(),
    };
    let mut functions = Vec::new();
    let mut signatures = HashMap::new();
    while parser.peek() != &Token::End {
        parser.reject_import()?;
        let position = parser.position();
        if parser.peek() == &Token::Word("trait".into()) {
            let name = parser.trait_declaration()?;
            if signatures.contains_key(&name) {
                return Err(position.error(format!("'{name}' is already declared as a function")));
            }
            continue;
        }
        if parser.peek() == &Token::Word("pub".into()) {
            parser.next();
            if parser.peek() != &Token::Word("enum".into()) {
                return Err(position.error("pub is currently supported only for enums"));
            }
        }
        if parser.peek() == &Token::Word("define".into()) {
            let name = parser.type_alias_declaration()?;
            if signatures.contains_key(&name) {
                return Err(position.error(format!("'{name}' is already declared as a function")));
            }
            continue;
        }
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
                    .insert(
                        method.name.clone(),
                        (
                            method.parameters
                                - method.optional_parameters.iter().filter(|x| **x).count(),
                            method.parameters,
                        ),
                    )
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
        if parser.traits.contains_key(&function.name) {
            return Err(position.error(format!(
                "'{}' is already declared as a trait",
                function.name
            )));
        }
        if parser.type_aliases.contains_key(&function.name) {
            return Err(position.error(format!(
                "'{}' is already declared as a type alias",
                function.name
            )));
        }
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
            .insert(
                function.name.clone(),
                (
                    function.parameters
                        - function.optional_parameters.iter().filter(|x| **x).count(),
                    function.parameters,
                ),
            )
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
        signatures: &HashMap<String, (usize, usize)>,
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
            Statement::Message(expr, _, _) => validate_expr(expr, signatures)?,
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
fn validate_expr(expr: &Expr, signatures: &HashMap<String, (usize, usize)>) -> Result<(), String> {
    match expr {
        Expr::Call(call) => validate_call(call, signatures),
        Expr::Binary(_, left, right)
        | Expr::Compare(_, left, right)
        | Expr::Logical(_, left, right) => {
            validate_expr(left, signatures)?;
            validate_expr(right, signatures)
        }
        Expr::Negate(expr)
        | Expr::Positive(expr)
        | Expr::Not(expr)
        | Expr::Annotated(expr, _)
        | Expr::Cast(expr, _, _) => validate_expr(expr, signatures),
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
fn validate_call(call: &Call, signatures: &HashMap<String, (usize, usize)>) -> Result<(), String> {
    let (required, total) = signatures.get(&call.name).ok_or_else(|| {
        call.position
            .error(format!("function '{}' is not declared", call.name))
    })?;
    if call.name == "main" {
        return Err(call.position.error("main cannot be called as a function"));
    }
    if call.arguments.len() < *required || call.arguments.len() > *total {
        let expected = if required == total {
            required.to_string()
        } else {
            format!("{required}..={total}")
        };
        return Err(call.position.error(format!(
            "function '{}' expects {expected} arguments, found {}",
            call.name,
            call.arguments.len()
        )));
    }
    for argument in &call.arguments {
        validate_expr(argument, signatures)?;
    }
    Ok(())
}
