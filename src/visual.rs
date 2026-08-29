use keystone_lang::{Direction, Expr, Statement};

pub trait ToCode {
    fn to_code(&self) -> String;
}

impl ToCode for Expr {
    fn to_code(&self) -> String {
        match self {
            Expr::Uint(v) => format!("{}", v),
            Expr::Float(v) => format!("{:.1}", v),
            Expr::String(s) => format!("\"{}\"", s),
            Expr::Boolean(b) => format!("{}", b),
            Expr::Direction(d) => match d {
                Direction::Left => "left".to_string(),
                Direction::Right => "right".to_string(),
                Direction::Forward => "forward".to_string(),
                Direction::Back => "back".to_string(),
                Direction::Up => "up".to_string(),
                Direction::Down => "down".to_string(),
            },
            Expr::Var(name) => name.clone(),
            Expr::Binary { op, lhs, rhs } => {
                format!("{} {:?} {}", lhs.to_code(), op, rhs.to_code())
            }
            Expr::Unary { op, exp } => format!("{:?} {}", op, exp.to_code()),
            Expr::Call { callee, args } => {
                let args_str = args
                    .iter()
                    .map(|a| a.to_code())
                    .collect::<Vec<_>>()
                    .join(", ");
                let callee_name = match callee {
                    keystone_lang::Callee::IsTouched => "is_touched",
                    keystone_lang::Callee::IsEmpty => "is_empty",
                    keystone_lang::Callee::Rand => "rand",
                };
                format!("{}({})", callee_name, args_str)
            }
        }
    }
}

pub fn statements_to_string(statements: &[Statement], indent_level: usize) -> String {
    let mut code = String::new();
    let indent = "  ".repeat(indent_level);

    for stmt in statements {
        match stmt {
            Statement::Print(e) => code.push_str(&format!("{}print {}\n", indent, e.to_code())),
            Statement::Move(e) => code.push_str(&format!("{}move {}\n", indent, e.to_code())),
            Statement::Turn(e) => code.push_str(&format!("{}turn {}\n", indent, e.to_code())),
            Statement::Dig(e) => code.push_str(&format!("{}dig {}\n", indent, e.to_code())),
            Statement::Let(name, e) => {
                code.push_str(&format!("{}{} = {}\n", indent, name, e.to_code()))
            }
            Statement::Sleep(e) => code.push_str(&format!("{}sleep {}\n", indent, e.to_code())),
            Statement::Receive(e) => code.push_str(&format!("{}receive {}\n", indent, e.to_code())),
            Statement::Send(e) => code.push_str(&format!("{}send {}\n", indent, e.to_code())),

            Statement::Loop(cond, body) => {
                code.push_str(&format!("{}loop {}\n", indent, cond.to_code()));
                code.push_str(&statements_to_string(body, indent_level + 1));
                code.push_str(&format!("{}end\n", indent));
            }
            Statement::While(cond, body) => {
                code.push_str(&format!("{}while {}\n", indent, cond.to_code()));
                code.push_str(&statements_to_string(body, indent_level + 1));
                code.push_str(&format!("{}end\n", indent));
            }
            Statement::If(cond, body) => {
                code.push_str(&format!("{}if {}\n", indent, cond.to_code()));
                code.push_str(&statements_to_string(body, indent_level + 1));
                code.push_str(&format!("{}end\n", indent));
            }
        }
    }
    code
}
