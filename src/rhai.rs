use std::collections::HashSet;

use keystone_lang::{Callee, Direction, Expr, Op, Statement, UnaryOp};

#[derive(Debug, Clone, PartialEq)]
pub struct RhaiProgram {
    pub statements: Vec<Statement>,
    pub has_comments: bool,
}

const INDENT: &str = "    ";

pub fn statements_to_rhai(statements: &[Statement]) -> String {
    let mut out = String::new();
    write_statements(&mut out, statements, 0, &mut HashSet::new());
    out
}

fn write_statements(
    out: &mut String,
    statements: &[Statement],
    depth: usize,
    visible: &mut HashSet<String>,
) {
    let indent = INDENT.repeat(depth);
    for statement in statements {
        match statement {
            Statement::Print(e) => out.push_str(&format!("{indent}print({});\n", expr(e))),
            Statement::Move(e) => out.push_str(&format!("{indent}move({});\n", direction_arg(e))),
            Statement::Turn(e) => out.push_str(&format!("{indent}turn({});\n", direction_arg(e))),
            Statement::Dig(e) => out.push_str(&format!("{indent}dig({});\n", direction_arg(e))),
            Statement::Place(e) => out.push_str(&format!("{indent}place({});\n", direction_arg(e))),
            Statement::Sleep(e) => out.push_str(&format!("{indent}sleep({});\n", expr(e))),
            Statement::Send(e) => out.push_str(&format!("{indent}send({});\n", expr(e))),
            Statement::Receive(e) => out.push_str(&format!("{indent}receive({});\n", expr(e))),
            Statement::Let(name, e) => {
                let keyword = if visible.insert(name.clone()) {
                    "let "
                } else {
                    ""
                };
                out.push_str(&format!("{indent}{keyword}{name} = {};\n", expr(e)));
            }
            Statement::Loop(count, body) => {
                out.push_str(&format!(
                    "{indent}for {} in 0..{} {{\n",
                    loop_counter_name(body, visible),
                    expr_at_least(count, PREC_ATOM)
                ));
                write_body(out, body, depth, visible);
                out.push_str(&format!("{indent}}}\n"));
            }
            Statement::While(cond, body) => {
                out.push_str(&format!("{indent}while {} {{\n", expr(cond)));
                write_body(out, body, depth, visible);
                out.push_str(&format!("{indent}}}\n"));
            }
            Statement::If(cond, body, else_body) => {
                out.push_str(&format!("{indent}if {} {{\n", expr(cond)));
                write_body(out, body, depth, visible);
                let mut else_body = else_body.as_deref();
                while let Some(branch) = else_body {
                    if let [Statement::If(cond, body, next)] = branch {
                        out.push_str(&format!("{indent}}} else if {} {{\n", expr(cond)));
                        write_body(out, body, depth, visible);
                        else_body = next.as_deref();
                    } else {
                        out.push_str(&format!("{indent}}} else {{\n"));
                        write_body(out, branch, depth, visible);
                        break;
                    }
                }
                out.push_str(&format!("{indent}}}\n"));
            }
        }
    }
}

fn loop_counter_name(body: &[Statement], visible: &HashSet<String>) -> String {
    let mut used = visible.clone();
    collect_names(body, &mut used);
    ["i", "j", "k"]
        .into_iter()
        .map(String::from)
        .chain((2..).map(|n| format!("i{n}")))
        .find(|name| !used.contains(name))
        .expect("an unused name exists")
}

fn collect_names(statements: &[Statement], names: &mut HashSet<String>) {
    for statement in statements {
        match statement {
            Statement::Let(name, e) => {
                names.insert(name.clone());
                collect_expr_names(e, names);
            }
            Statement::Loop(e, body) | Statement::While(e, body) => {
                collect_expr_names(e, names);
                collect_names(body, names);
            }
            Statement::If(e, body, else_body) => {
                collect_expr_names(e, names);
                collect_names(body, names);
                collect_names(else_body.as_deref().unwrap_or_default(), names);
            }
            Statement::Print(e)
            | Statement::Move(e)
            | Statement::Turn(e)
            | Statement::Dig(e)
            | Statement::Place(e)
            | Statement::Sleep(e)
            | Statement::Send(e)
            | Statement::Receive(e) => collect_expr_names(e, names),
        }
    }
}

fn collect_expr_names(e: &Expr, names: &mut HashSet<String>) {
    match e {
        Expr::Var(name) => {
            names.insert(name.clone());
        }
        Expr::Binary { lhs, rhs, .. } => {
            collect_expr_names(lhs, names);
            collect_expr_names(rhs, names);
        }
        Expr::Unary { exp, .. } => collect_expr_names(exp, names),
        Expr::Call { args, .. } => args.iter().for_each(|arg| collect_expr_names(arg, names)),
        _ => {}
    }
}

fn write_body(out: &mut String, body: &[Statement], depth: usize, visible: &HashSet<String>) {
    write_statements(out, body, depth + 1, &mut visible.clone());
}

const PREC_OR: u8 = 1;
const PREC_AND: u8 = 2;
const PREC_CMP: u8 = 4;
const PREC_ADD: u8 = 5;
const PREC_MUL: u8 = 6;
const PREC_UNARY: u8 = 8;
const PREC_ATOM: u8 = 9;

fn op_info(op: &Op) -> (u8, &'static str) {
    match op {
        Op::Or => (PREC_OR, "||"),
        Op::And => (PREC_AND, "&&"),
        Op::Eq => (PREC_CMP, "=="),
        Op::Neq => (PREC_CMP, "!="),
        Op::Lt => (PREC_CMP, "<"),
        Op::Gt => (PREC_CMP, ">"),
        Op::Le => (PREC_CMP, "<="),
        Op::Ge => (PREC_CMP, ">="),
        Op::Add => (PREC_ADD, "+"),
        Op::Sub => (PREC_ADD, "-"),
        Op::Mul => (PREC_MUL, "*"),
        Op::Div => (PREC_MUL, "/"),
    }
}

fn precedence(e: &Expr) -> u8 {
    match e {
        Expr::Binary { op, .. } => op_info(op).0,
        Expr::Unary { .. } => PREC_UNARY,
        _ => PREC_ATOM,
    }
}

fn expr_at_least(e: &Expr, min: u8) -> String {
    if precedence(e) < min {
        format!("({})", expr(e))
    } else {
        expr(e)
    }
}

fn direction_name(d: &Direction) -> &'static str {
    match d {
        Direction::Left => "left",
        Direction::Right => "right",
        Direction::Up => "up",
        Direction::Down => "down",
        Direction::Forward => "forward",
        Direction::Back => "back",
    }
}

fn direction_arg(e: &Expr) -> String {
    match e {
        Expr::Direction(d) => format!("\"{}\"", direction_name(d)),
        other => expr(other),
    }
}

pub(crate) fn expr_to_rhai(e: &Expr) -> String {
    expr(e)
}

fn expr(e: &Expr) -> String {
    match e {
        Expr::Uint(v) => v.to_string(),
        Expr::Float(v) => format!("{v:?}"),
        Expr::String(s) => format!("\"{s}\""),
        Expr::Boolean(b) => b.to_string(),
        Expr::Direction(d) => format!("\"{}\"", direction_name(d)),
        Expr::Var(name) => name.clone(),
        Expr::Binary { op, lhs, rhs } => {
            let (prec, symbol) = op_info(op);
            let lhs_min = if prec == PREC_CMP { prec + 1 } else { prec };
            format!(
                "{} {symbol} {}",
                expr_at_least(lhs, lhs_min),
                expr_at_least(rhs, prec + 1)
            )
        }
        Expr::Unary {
            op: UnaryOp::Not,
            exp,
        } => format!("!{}", expr_at_least(exp, PREC_UNARY)),
        Expr::Call { callee, args } => {
            let name = match callee {
                Callee::IsTouched => "is_touched",
                Callee::IsEmpty => "is_empty",
                Callee::Rand => "rand",
            };
            let args: Vec<String> = args.iter().map(|arg| direction_arg(arg)).collect();
            format!("{name}({})", args.join(", "))
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Ident(String),
    Int(u32),
    Float(f32),
    Str(String),
    Sym(&'static str),
}

impl Tok {
    fn describe(&self) -> String {
        match self {
            Tok::Ident(name) => format!("`{name}`"),
            Tok::Int(v) => format!("`{v}`"),
            Tok::Float(v) => format!("`{v:?}`"),
            Tok::Str(s) => format!("\"{s}\""),
            Tok::Sym(s) => format!("`{s}`"),
        }
    }
}

const SYMBOLS: [&str; 21] = [
    "==", "!=", "<=", ">=", "&&", "||", "..", "(", ")", "{", "}", ";", ",", "=", "<", ">", "+",
    "-", "*", "/", "!",
];

struct Lexed {
    tokens: Vec<(Tok, usize)>,
    has_comments: bool,
}

fn error_at(line: usize, message: impl Into<String>) -> String {
    format!("Line {line}: {}", message.into())
}

fn lex(source: &str) -> Result<Lexed, String> {
    let chars: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut has_comments = false;
    let mut line = 1;
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if c == '\n' {
            line += 1;
            i += 1;
        } else if c.is_whitespace() {
            i += 1;
        } else if c == '/' && next == Some('/') {
            has_comments = true;
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && next == Some('*') {
            has_comments = true;
            let start = line;
            let mut depth = 0;
            loop {
                match (chars.get(i), chars.get(i + 1)) {
                    (Some('/'), Some('*')) => {
                        depth += 1;
                        i += 2;
                    }
                    (Some('*'), Some('/')) => {
                        depth -= 1;
                        i += 2;
                        if depth == 0 {
                            break;
                        }
                    }
                    (Some(ch), _) => {
                        if *ch == '\n' {
                            line += 1;
                        }
                        i += 1;
                    }
                    (None, _) => return Err(error_at(start, "Unclosed comment.")),
                }
            }
        } else if c.is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let is_float = chars.get(i) == Some(&'.')
                && chars.get(i + 1).is_some_and(|ch| ch.is_ascii_digit());
            if is_float {
                i += 1;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
            if chars
                .get(i)
                .is_some_and(|ch| ch.is_alphanumeric() || *ch == '_')
            {
                return Err(error_at(
                    line,
                    "This number format isn't supported in blocks.",
                ));
            }
            let text: String = chars[start..i].iter().collect();
            let token = if is_float {
                Tok::Float(
                    text.parse()
                        .map_err(|_| error_at(line, "Invalid number."))?,
                )
            } else {
                Tok::Int(
                    text.parse()
                        .map_err(|_| error_at(line, "Number is too large (max 4294967295)."))?,
                )
            };
            tokens.push((token, line));
        } else if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            tokens.push((Tok::Ident(chars[start..i].iter().collect()), line));
        } else if c == '"' {
            i += 1;
            let start = i;
            while i < chars.len() && chars[i] != '"' {
                match chars[i] {
                    '\\' => {
                        return Err(error_at(
                            line,
                            "Escapes (\\) in text aren't supported in blocks.",
                        ));
                    }
                    '\n' => return Err(error_at(line, "Text must end on the same line.")),
                    _ => i += 1,
                }
            }
            if i == chars.len() {
                return Err(error_at(line, "Text is missing its closing \"."));
            }
            tokens.push((Tok::Str(chars[start..i].iter().collect()), line));
            i += 1;
        } else {
            let rest: String = chars[i..chars.len().min(i + 2)].iter().collect();
            let Some(symbol) = SYMBOLS.iter().find(|s| rest.starts_with(**s)) else {
                return Err(error_at(line, format!("`{c}` isn't supported in blocks.")));
            };
            tokens.push((Tok::Sym(symbol), line));
            i += symbol.len();
        }
    }

    Ok(Lexed {
        tokens,
        has_comments,
    })
}

const RHAI_KEYWORDS: &[&str] = &[
    "true",
    "false",
    "let",
    "const",
    "if",
    "else",
    "switch",
    "do",
    "while",
    "until",
    "loop",
    "for",
    "in",
    "continue",
    "break",
    "return",
    "throw",
    "try",
    "catch",
    "import",
    "export",
    "as",
    "global",
    "private",
    "fn",
    "Fn",
    "call",
    "curry",
    "this",
    "type_of",
    "print",
    "debug",
    "eval",
    "is_def_var",
    "is_def_fn",
    "is_shared",
    "var",
    "static",
    "shared",
    "with",
    "is",
    "goto",
    "exit",
    "match",
    "case",
    "public",
    "protected",
    "new",
    "use",
    "module",
    "package",
    "super",
    "thread",
    "spawn",
    "go",
    "await",
    "async",
    "sync",
    "yield",
    "default",
    "void",
    "null",
    "nil",
];

pub fn is_rhai_variable_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !RHAI_KEYWORDS.contains(&name)
}

pub fn parse_rhai(source: &str) -> Result<RhaiProgram, String> {
    let Lexed {
        tokens,
        has_comments,
    } = lex(source)?;
    let mut parser = Parser {
        tokens,
        pos: 0,
        scopes: vec![HashSet::new()],
        loop_counters: Vec::new(),
    };
    let statements = parser.statements()?;
    if let Some((token, line)) = parser.tokens.get(parser.pos) {
        return Err(error_at(*line, format!("Unexpected {}.", token.describe())));
    }
    Ok(RhaiProgram {
        statements,
        has_comments,
    })
}

struct Parser {
    tokens: Vec<(Tok, usize)>,
    pos: usize,
    scopes: Vec<HashSet<String>>,
    loop_counters: Vec<String>,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.tokens.get(self.pos).map(|(token, _)| token)
    }

    fn peek_at(&self, offset: usize) -> Option<&Tok> {
        self.tokens.get(self.pos + offset).map(|(token, _)| token)
    }

    fn line(&self) -> usize {
        self.tokens
            .get(self.pos)
            .or(self.tokens.last())
            .map_or(1, |(_, line)| *line)
    }

    fn fail<T>(&self, message: impl Into<String>) -> Result<T, String> {
        Err(error_at(self.line(), message))
    }

    fn unexpected<T>(&self, expected: &str) -> Result<T, String> {
        match self.peek() {
            Some(token) => self.fail(format!("Expected {expected}, found {}.", token.describe())),
            None => self.fail(format!("Expected {expected}, but the code ended.")),
        }
    }

    fn is_sym(&self, symbol: &str) -> bool {
        matches!(self.peek(), Some(Tok::Sym(s)) if *s == symbol)
    }

    fn is_ident(&self, word: &str) -> bool {
        matches!(self.peek(), Some(Tok::Ident(name)) if name == word)
    }

    fn eat_sym(&mut self, symbol: &str) -> bool {
        let found = self.is_sym(symbol);
        if found {
            self.pos += 1;
        }
        found
    }

    fn expect_sym(&mut self, symbol: &str) -> Result<(), String> {
        if self.eat_sym(symbol) {
            Ok(())
        } else {
            self.unexpected(&format!("`{symbol}`"))
        }
    }

    fn expect_ident(&mut self, word: &str) -> Result<(), String> {
        if self.is_ident(word) {
            self.pos += 1;
            Ok(())
        } else {
            self.unexpected(&format!("`{word}`"))
        }
    }

    fn is_visible(&self, name: &str) -> bool {
        self.scopes.iter().any(|scope| scope.contains(name))
    }

    fn statements(&mut self) -> Result<Vec<Statement>, String> {
        let mut statements = Vec::new();
        while self.peek().is_some() && !self.is_sym("}") {
            if self.eat_sym(";") {
                continue;
            }
            statements.push(self.statement()?);
        }
        Ok(statements)
    }

    fn end_of_simple_statement(&mut self) -> Result<(), String> {
        if self.eat_sym(";") || self.peek().is_none() || self.is_sym("}") {
            Ok(())
        } else {
            self.unexpected("`;`")
        }
    }

    fn block(&mut self) -> Result<Vec<Statement>, String> {
        self.expect_sym("{")?;
        self.scopes.push(HashSet::new());
        let body = self.statements();
        self.scopes.pop();
        let body = body?;
        self.expect_sym("}")?;
        Ok(body)
    }

    fn statement(&mut self) -> Result<Statement, String> {
        let Some(Tok::Ident(word)) = self.peek().cloned() else {
            return self.unexpected("a statement");
        };
        match word.as_str() {
            "let" => {
                self.pos += 1;
                let name = self.variable_name()?;
                if self.is_visible(&name) {
                    return self.fail(format!(
                        "`let {name}` declares `{name}` again; blocks can only show one variable per name."
                    ));
                }
                self.expect_sym("=")?;
                let value = self.expr()?;
                self.end_of_simple_statement()?;
                self.scopes
                    .last_mut()
                    .expect("there is always a scope")
                    .insert(name.clone());
                Ok(Statement::Let(name, value))
            }
            "if" => {
                self.pos += 1;
                self.if_rest()
            }
            "while" => {
                self.pos += 1;
                let cond = self.expr()?;
                Ok(Statement::While(cond, self.block()?))
            }
            "for" => {
                self.pos += 1;
                let counter = self.variable_name()?;
                if self.is_visible(&counter) {
                    return self.fail(format!(
                        "`for {counter}` hides the variable `{counter}`; use another name."
                    ));
                }
                self.expect_ident("in")?;
                if self.peek() != Some(&Tok::Int(0)) {
                    return self.fail("Only `for i in 0..N` loops can be shown as blocks.");
                }
                self.pos += 1;
                self.expect_sym("..")?;
                if self.is_sym("=") {
                    return self.fail("Only `for i in 0..N` loops can be shown as blocks.");
                }
                let count = self.unary()?;
                self.loop_counters.push(counter);
                let body = self.block();
                self.loop_counters.pop();
                Ok(Statement::Loop(count, body?))
            }
            _ if self.peek_at(1) == Some(&Tok::Sym("(")) => {
                let statement = self.command(&word)?;
                self.end_of_simple_statement()?;
                Ok(statement)
            }
            _ if self.peek_at(1) == Some(&Tok::Sym("=")) => {
                let name = self.variable_name()?;
                if !self.is_visible(&name) {
                    return self.fail(format!("`{name}` is assigned before `let {name}`."));
                }
                self.expect_sym("=")?;
                let value = self.expr()?;
                self.end_of_simple_statement()?;
                Ok(Statement::Let(name, value))
            }
            _ => self.fail(format!("`{word}` can't be shown as a block.")),
        }
    }

    fn if_rest(&mut self) -> Result<Statement, String> {
        let cond = self.expr()?;
        let body = self.block()?;
        if !self.is_ident("else") {
            return Ok(Statement::If(cond, body, None));
        }
        self.pos += 1;
        let else_body = if self.is_ident("if") {
            self.pos += 1;
            vec![self.if_rest()?]
        } else {
            self.block()?
        };
        Ok(Statement::If(cond, body, Some(else_body)))
    }

    fn variable_name(&mut self) -> Result<String, String> {
        match self.peek().cloned() {
            Some(Tok::Ident(name)) if is_rhai_variable_name(&name) => {
                self.pos += 1;
                Ok(name)
            }
            Some(Tok::Ident(name)) => self.fail(format!("`{name}` can't be a variable name.")),
            _ => self.unexpected("a variable name"),
        }
    }

    fn args(&mut self) -> Result<Vec<Expr>, String> {
        self.expect_sym("(")?;
        let mut args = Vec::new();
        if !self.is_sym(")") {
            loop {
                args.push(self.expr()?);
                if !self.eat_sym(",") {
                    break;
                }
            }
        }
        self.expect_sym(")")?;
        Ok(args)
    }

    fn single_arg(&mut self, name: &str) -> Result<Expr, String> {
        let mut args = self.args()?;
        if args.len() != 1 {
            return self.fail(format!("`{name}` takes one value."));
        }
        Ok(args.remove(0))
    }

    fn command(&mut self, name: &str) -> Result<Statement, String> {
        self.pos += 1;
        let shorthand = match name {
            "move_top" => Some(Direction::Up),
            "move_down" => Some(Direction::Down),
            "move_left" => Some(Direction::Left),
            "move_right" => Some(Direction::Right),
            _ => None,
        };
        if let Some(direction) = shorthand {
            if !self.args()?.is_empty() {
                return self.fail(format!("`{name}` takes no values."));
            }
            return Ok(Statement::Move(Expr::Direction(direction)));
        }

        let make: fn(Expr) -> Statement = match name {
            "move" => |e| Statement::Move(as_direction(e)),
            "dig" => |e| Statement::Dig(as_direction(e)),
            "place" => |e| Statement::Place(as_direction(e)),
            "sleep" => Statement::Sleep,
            "print" => Statement::Print,
            "send" => Statement::Send,
            "receive" => Statement::Receive,
            _ => return self.fail(format!("`{name}` isn't available in blocks.")),
        };
        Ok(make(self.single_arg(name)?))
    }

    fn expr(&mut self) -> Result<Expr, String> {
        self.binary_level(PREC_OR)
    }

    fn binary_level(&mut self, level: u8) -> Result<Expr, String> {
        let next = match level {
            PREC_OR => PREC_AND,
            PREC_AND => PREC_CMP,
            PREC_CMP => PREC_ADD,
            PREC_ADD => PREC_MUL,
            PREC_MUL => PREC_UNARY,
            _ => return self.unary(),
        };
        let mut lhs = self.binary_level(next)?;
        while let Some(op) = self.binary_op(level) {
            self.pos += 1;
            let rhs = self.binary_level(next)?;
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
            if level == PREC_CMP && self.binary_op(PREC_CMP).is_some() {
                return self.fail("Put parentheses around chained comparisons.");
            }
        }
        Ok(lhs)
    }

    fn binary_op(&self, level: u8) -> Option<Op> {
        let Some(Tok::Sym(symbol)) = self.peek() else {
            return None;
        };
        let op = match *symbol {
            "||" => Op::Or,
            "&&" => Op::And,
            "==" => Op::Eq,
            "!=" => Op::Neq,
            "<" => Op::Lt,
            ">" => Op::Gt,
            "<=" => Op::Le,
            ">=" => Op::Ge,
            "+" => Op::Add,
            "-" => Op::Sub,
            "*" => Op::Mul,
            "/" => Op::Div,
            _ => return None,
        };
        (op_info(&op).0 == level).then_some(op)
    }

    fn unary(&mut self) -> Result<Expr, String> {
        if self.eat_sym("!") {
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                exp: Box::new(self.unary()?),
            });
        }
        if self.is_sym("-") {
            return self.fail("Negative numbers can't be shown as blocks.");
        }
        self.primary()
    }

    fn primary(&mut self) -> Result<Expr, String> {
        let Some(token) = self.peek().cloned() else {
            return self.unexpected("a value");
        };
        match token {
            Tok::Int(v) => {
                self.pos += 1;
                Ok(Expr::Uint(v))
            }
            Tok::Float(v) => {
                self.pos += 1;
                Ok(Expr::Float(v))
            }
            Tok::Str(s) => {
                self.pos += 1;
                Ok(Expr::String(s))
            }
            Tok::Sym("(") => {
                self.pos += 1;
                let inner = self.expr()?;
                self.expect_sym(")")?;
                Ok(inner)
            }
            Tok::Ident(word) if word == "true" || word == "false" => {
                self.pos += 1;
                Ok(Expr::Boolean(word == "true"))
            }
            Tok::Ident(word) if self.peek_at(1) == Some(&Tok::Sym("(")) => {
                self.pos += 1;
                let args = self.args()?;
                match (word.as_str(), args.len()) {
                    ("is_touched", 0) => Ok(Expr::Call {
                        callee: Callee::IsTouched,
                        args: vec![],
                    }),
                    ("is_empty", 1) => Ok(Expr::Call {
                        callee: Callee::IsEmpty,
                        args: args
                            .into_iter()
                            .map(|a| Box::new(as_direction(a)))
                            .collect(),
                    }),
                    ("is_touched" | "is_empty", _) => {
                        self.fail(format!("`{word}` got the wrong number of values."))
                    }
                    _ => self.fail(format!("`{word}` isn't available in blocks.")),
                }
            }
            Tok::Ident(_) => {
                let name = self.variable_name()?;
                if !self.is_visible(&name) && self.loop_counters.contains(&name) {
                    return self.fail(format!(
                        "Blocks can't read the loop variable `{name}`; count with a variable of your own."
                    ));
                }
                if !self.is_visible(&name) {
                    return self.fail(format!("`{name}` is used before `let {name}`."));
                }
                Ok(Expr::Var(name))
            }
            other => self.fail(format!("Unexpected {}.", other.describe())),
        }
    }
}

fn as_direction(e: Expr) -> Expr {
    let direction = match &e {
        Expr::String(s) => match s.as_str() {
            "up" | "top" => Direction::Up,
            "down" => Direction::Down,
            "left" => Direction::Left,
            "right" => Direction::Right,
            _ => return e,
        },
        _ => return e,
    };
    Expr::Direction(direction)
}

#[cfg(test)]
mod tests {
    use super::*;
    use keystone_lang::{Event, ExternalApi, eval_all, parse};
    use std::sync::{Arc, Mutex};

    fn mv(direction: Direction) -> Statement {
        Statement::Move(Expr::Direction(direction))
    }

    fn print(text: &str) -> Statement {
        Statement::Print(Expr::String(text.to_string()))
    }

    fn read(source: &str) -> Vec<Statement> {
        parse_rhai(source)
            .unwrap_or_else(|err| panic!("{source:?} should read: {err}"))
            .statements
    }

    fn assert_round_trip(blocks: &[Statement]) {
        let code = statements_to_rhai(blocks);
        let back = parse_rhai(&code).unwrap_or_else(|err| panic!("{err}\n{code}"));
        assert_eq!(back.statements, blocks, "written code:\n{code}");
        assert!(!back.has_comments);
    }

    const SAMPLE: &str = r#"
total = 0
loop 3
  total = total + 2
  if total == 4 and not is_touched()
    move up
  else if total > 4 or is_empty(left)
    dig down
    place right
  else
    sleep 0.5
  end
end
while total > 0
  total = total - 1
  sleep 1
end
send "go"
receive 2
print "done"
"#;

    #[test]
    fn writes_canonical_rhai() {
        let blocks = parse(SAMPLE).unwrap();
        assert_eq!(
            statements_to_rhai(&blocks),
            r#"let total = 0;
for i in 0..3 {
    total = total + 2;
    if total == 4 && !is_touched() {
        move("up");
    } else if total > 4 || is_empty("left") {
        dig("down");
        place("right");
    } else {
        sleep(0.5);
    }
}
while total > 0 {
    total = total - 1;
    sleep(1);
}
send("go");
receive(2);
print("done");
"#
        );
    }

    #[test]
    fn written_rhai_reads_back_as_the_same_blocks() {
        assert_round_trip(&parse(SAMPLE).unwrap());
        assert_round_trip(&[]);
        assert_round_trip(&[Statement::If(Expr::Boolean(true), vec![], Some(vec![]))]);
        assert_round_trip(&parse("n = 2\nloop n + 1\n  move left\nend").unwrap());
        assert_round_trip(&[Statement::If(
            Expr::Boolean(false),
            vec![],
            Some(vec![
                Statement::If(Expr::Boolean(true), vec![print("a")], None),
                print("b"),
            ]),
        )]);
    }

    #[test]
    fn let_marks_the_first_assignment_in_each_block() {
        let blocks = parse("x = 1\nx = 2\nif true\n  y = 1\n  x = 3\nend\ny = \"again\"").unwrap();
        assert_eq!(
            statements_to_rhai(&blocks),
            "let x = 1;\nx = 2;\nif true {\n    let y = 1;\n    x = 3;\n}\nlet y = \"again\";\n"
        );
        assert_round_trip(&blocks);
    }

    #[test]
    fn operators_keep_their_grouping() {
        for (keystone, rhai) in [
            ("1 + 2 * 3", "1 + 2 * 3"),
            ("(1 + 2) * 3", "(1 + 2) * 3"),
            ("10 - 4 - 3", "10 - 4 - 3"),
            ("10 - (4 - 3)", "10 - (4 - 3)"),
            ("not 1 == 2", "!(1 == 2)"),
            ("(not true) == false", "!true == false"),
            ("not not true", "!!true"),
            ("1 < 2 == true", "(1 < 2) == true"),
            ("true or false and false", "true || false && false"),
            ("(true or false) and false", "(true || false) && false"),
        ] {
            let blocks = parse(&format!("print {keystone}")).unwrap();
            assert_eq!(statements_to_rhai(&blocks), format!("print({rhai});\n"));
            assert_round_trip(&blocks);
        }
    }

    #[test]
    fn reads_common_hand_written_variations() {
        let program = parse_rhai(
            "// go around\nmove_top(); move(\"top\");\n/* nested /* comment */ */\nif (is_touched()) { move(\"left\") }\n;;",
        )
        .unwrap();
        assert!(program.has_comments);
        assert_eq!(
            program.statements,
            vec![
                mv(Direction::Up),
                mv(Direction::Up),
                Statement::If(
                    Expr::Call {
                        callee: Callee::IsTouched,
                        args: vec![]
                    },
                    vec![mv(Direction::Left)],
                    None
                ),
            ]
        );
        assert!(!parse_rhai("move(\"up\");").unwrap().has_comments);
    }

    #[test]
    fn loops_read_any_unused_counter_and_write_one_the_body_does_not_use() {
        assert_eq!(
            read("for count in 0..2 { move(\"up\"); }"),
            vec![Statement::Loop(Expr::Uint(2), vec![mv(Direction::Up)])]
        );

        let blocks = parse("i = 0\nloop 2\n  j = i\n  loop 2\n    k = j\n  end\nend").unwrap();
        let code = statements_to_rhai(&blocks);
        assert_eq!(
            code,
            "let i = 0;\nfor i2 in 0..2 {\n    let j = i;\n    for i2 in 0..2 {\n        let k = j;\n    }\n}\n"
        );
        assert_round_trip(&blocks);
    }

    #[test]
    fn direction_text_is_a_direction_only_in_command_arguments() {
        assert_eq!(
            read("let d = \"left\"; move(d); print(\"left\");"),
            vec![
                Statement::Let("d".into(), Expr::String("left".into())),
                Statement::Move(Expr::Var("d".into())),
                Statement::Print(Expr::String("left".into())),
            ]
        );
    }

    #[test]
    fn rejects_what_blocks_cannot_show() {
        for source in [
            "let x = 1; if true { let x = 2; }",
            "let x = 1; let x = 2;",
            "x = 1;",
            "print(y);",
            "if true { let y = 1; } print(y);",
            "for _ in 0..3 { }",
            "for i in 0..3 { print(i); }",
            "let i = 1; for i in 0..3 { }",
            "for i in 0..=3 { }",
            "for i in 1..3 { }",
            "let _x = 1;",
            "loop { move(\"up\"); }",
            "fn go() { move(\"up\"); }",
            "turn(\"left\");",
            "let r = rand(3);",
            "let t = timestamp();",
            "print(-1);",
            "print(\"a\\nb\");",
            "print(1 < 2 < 3);",
            "print(1 == 2 == false);",
            "move(\"up\") move(\"down\");",
            "let fn = 1;",
            "let x = 1e3;",
            "let x = 4294967296;",
            "else { }",
            "if true { } else { } else { }",
            "/* open",
            "print(\"open);",
            "let c = 'a';",
            "switch x { }",
        ] {
            assert!(
                parse_rhai(source).is_err(),
                "{source:?} should not be shown as blocks"
            );
        }
    }

    #[test]
    fn written_turn_rand_and_free_directions_do_not_read_back() {
        for blocks in [
            vec![Statement::Turn(Expr::Direction(Direction::Left))],
            vec![Statement::Print(Expr::Call {
                callee: Callee::Rand,
                args: vec![],
            })],
            vec![mv(Direction::Forward)],
            vec![Statement::Let("d".into(), Expr::Direction(Direction::Left))],
        ] {
            let code = statements_to_rhai(&blocks);
            assert!(
                parse_rhai(&code).map(|p| p.statements) != Ok(blocks.clone()),
                "{code:?} should be reported as not writable"
            );
        }
    }

    #[test]
    fn rejections_explain_the_problem_with_a_line_number() {
        let err = parse_rhai("move(\"up\");\nlet x = 1;\nlet x = 2;").unwrap_err();
        assert!(err.starts_with("Line 3:"), "{err}");
        assert!(err.contains("let x"), "{err}");
    }

    struct Api;

    impl ExternalApi for Api {
        fn is_touched(&self) -> bool {
            false
        }
        fn is_empty(&self, _: Direction) -> bool {
            true
        }
        fn send_signal(&self, _: &str) {}
        fn receive_signal(&self, _: &str) -> bool {
            true
        }
    }

    fn keystone_prints(blocks: &[Statement]) -> Vec<String> {
        let code = crate::code::statements_to_string(blocks, 0);
        eval_all(&code, Arc::new(Api))
            .unwrap_or_else(|err| panic!("{err:?}\n{code}"))
            .into_iter()
            .filter_map(|event| match event {
                Event::Print(text) => Some(text),
                _ => None,
            })
            .collect()
    }

    fn rhai_prints(code: &str) -> Vec<String> {
        let printed = Arc::new(Mutex::new(Vec::new()));
        let mut engine = rhai::Engine::new();
        let sink = printed.clone();
        engine.on_print(move |text| sink.lock().unwrap().push(text.to_string()));
        for name in ["move", "dig", "place", "send", "receive"] {
            engine.register_fn(name, |_: &str| ());
        }
        engine.register_fn("send", |_: i64| ());
        engine.register_fn("receive", |_: i64| ());
        engine.register_fn("sleep", |_: f64| ());
        engine.register_fn("sleep", |_: i64| ());
        engine.register_fn("is_touched", || false);
        engine.register_fn("is_empty", |_: &str| true);
        engine
            .run(code)
            .unwrap_or_else(|err| panic!("Rhai rejected the written code: {err}\n{code}"));
        printed.lock().unwrap().clone()
    }

    #[test]
    fn real_rhai_accepts_the_written_sample() {
        rhai_prints(&statements_to_rhai(&parse(SAMPLE).unwrap()));
    }

    #[test]
    fn real_rhai_computes_the_same_values_as_keystone() {
        for keystone in [
            "1 + 2 * 3",
            "(1 + 2) * 3",
            "10 - 4 - 3",
            "10 - (4 - 3)",
            "20 / 2 / 5",
            "8 / (2 * 2)",
            "2 * 3 == 6",
            "not 1 == 2",
            "(not true) == false",
            "not true and false",
            "not (true and false)",
            "true or false and false",
            "(true or false) and false",
            "1 < 2 == true",
            "1 < 2 and 3 > 4 or 5 >= 5",
            "not not true",
            "is_touched() or is_empty(left)",
            "\"a\" == \"a\"",
        ] {
            let blocks = parse(&format!("print {keystone}")).unwrap();
            let code = statements_to_rhai(&blocks);
            assert_eq!(
                rhai_prints(&code),
                keystone_prints(&blocks),
                "{keystone} was written as {code}"
            );
        }
    }

    #[test]
    fn real_rhai_runs_the_same_control_flow_as_keystone() {
        let blocks = parse(
            r#"
total = 0
loop 3
  total = total + 2
  if total == 2
    print "two"
  else if total == 4
    print "four"
  else
    print "other"
  end
end
print total
i = 0
while i < 3
  i = i + 1
  step = i * 10
  print step
end
if true
  word = "inside"
  print word
end
word = "outside"
print word
loop 2
  if false
  else
    print "else"
  end
end
n = 0
loop 2
  loop 3
    n = n + 1
  end
end
print n
"#,
        )
        .unwrap();
        let code = statements_to_rhai(&blocks);
        let expected = keystone_prints(&blocks);
        assert_eq!(
            expected,
            [
                "two", "four", "other", "6", "10", "20", "30", "inside", "outside", "else", "else",
                "6"
            ]
        );
        assert_eq!(rhai_prints(&code), expected, "written as:\n{code}");
    }
}
