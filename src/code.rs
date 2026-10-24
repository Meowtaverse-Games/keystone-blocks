use keystone_lang::{Direction, Expr, Op, Statement, UnaryOp};

pub trait ToCode {
    fn to_code(&self) -> String;
}

const PREC_UNARY: u8 = 3;
const PREC_ATOM: u8 = 7;

fn op_precedence(op: &Op) -> u8 {
    match op {
        Op::Or => 1,
        Op::And => 2,
        Op::Eq | Op::Neq | Op::Lt | Op::Gt | Op::Le | Op::Ge => 4,
        Op::Add | Op::Sub => 5,
        Op::Mul | Op::Div => 6,
    }
}

fn precedence(expr: &Expr) -> u8 {
    match expr {
        Expr::Binary { op, .. } => op_precedence(op),
        Expr::Unary { .. } => PREC_UNARY,
        _ => PREC_ATOM,
    }
}

fn to_code_with_prec(expr: &Expr, min_prec: u8) -> String {
    let code = expr.to_code();
    if precedence(expr) < min_prec {
        format!("({})", code)
    } else {
        code
    }
}

impl ToCode for Expr {
    fn to_code(&self) -> String {
        match self {
            Expr::Uint(v) => format!("{}", v),
            Expr::Float(v) => format!("{:?}", v),
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
                let op_str = match op {
                    Op::Eq => "==",
                    Op::Neq => "!=",
                    Op::Lt => "<",
                    Op::Gt => ">",
                    Op::Le => "<=",
                    Op::Ge => ">=",
                    Op::Add => "+",
                    Op::Sub => "-",
                    Op::Mul => "*",
                    Op::Div => "/",
                    Op::And => "and",
                    Op::Or => "or",
                };
                let prec = op_precedence(op);
                format!(
                    "{} {} {}",
                    to_code_with_prec(lhs, prec),
                    op_str,
                    to_code_with_prec(rhs, prec + 1)
                )
            }

            Expr::Unary { op, exp } => {
                let op_str = match op {
                    UnaryOp::Not => "not",
                };
                format!("{} {}", op_str, to_code_with_prec(exp, PREC_UNARY))
            }

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
            Statement::Place(e) => code.push_str(&format!("{}place {}\n", indent, e.to_code())),
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

#[cfg(test)]
mod tests {
    use super::*;
    use keystone_lang::parse;

    fn bin(op: Op, lhs: Expr, rhs: Expr) -> Expr {
        Expr::Binary {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
    }

    fn not(exp: Expr) -> Expr {
        Expr::Unary {
            op: UnaryOp::Not,
            exp: Box::new(exp),
        }
    }

    fn var(name: &str) -> Expr {
        Expr::Var(name.to_string())
    }

    fn assert_round_trip(expr: Expr, expected: &str) {
        let code = expr.to_code();
        assert_eq!(code, expected);
        let stmts = parse(&format!("print {code}")).unwrap();
        assert_eq!(stmts, vec![Statement::Print(expr)], "code: {code}");
    }

    #[test]
    fn lower_precedence_operand_is_parenthesized() {
        assert_round_trip(
            bin(
                Op::Mul,
                bin(Op::Add, var("x"), Expr::Uint(1)),
                Expr::Uint(2),
            ),
            "(x + 1) * 2",
        );
        assert_round_trip(
            bin(
                Op::Mul,
                Expr::Uint(2),
                bin(Op::Add, var("x"), Expr::Uint(1)),
            ),
            "2 * (x + 1)",
        );
        assert_round_trip(
            bin(Op::And, bin(Op::Or, var("a"), var("b")), var("c")),
            "(a or b) and c",
        );
    }

    #[test]
    fn higher_precedence_operand_is_not_parenthesized() {
        assert_round_trip(
            bin(
                Op::Add,
                var("x"),
                bin(Op::Mul, Expr::Uint(1), Expr::Uint(2)),
            ),
            "x + 1 * 2",
        );
        assert_round_trip(
            bin(
                Op::Or,
                bin(Op::And, not(var("a")), var("b")),
                bin(Op::Lt, var("x"), Expr::Uint(3)),
            ),
            "not a and b or x < 3",
        );
    }

    #[test]
    fn left_associativity_is_preserved() {
        assert_round_trip(
            bin(
                Op::Sub,
                bin(Op::Sub, var("x"), Expr::Uint(1)),
                Expr::Uint(2),
            ),
            "x - 1 - 2",
        );
        assert_round_trip(
            bin(
                Op::Sub,
                var("x"),
                bin(Op::Sub, Expr::Uint(1), Expr::Uint(2)),
            ),
            "x - (1 - 2)",
        );
        assert_round_trip(
            bin(
                Op::Div,
                var("x"),
                bin(Op::Mul, Expr::Uint(2), Expr::Uint(3)),
            ),
            "x / (2 * 3)",
        );
        assert_round_trip(
            bin(Op::Eq, var("a"), bin(Op::Eq, var("b"), var("c"))),
            "a == (b == c)",
        );
    }

    #[test]
    fn not_operand_and_not_as_operand() {
        assert_round_trip(not(bin(Op::And, var("a"), var("b"))), "not (a and b)");
        assert_round_trip(not(bin(Op::Eq, var("x"), Expr::Uint(1))), "not x == 1");
        assert_round_trip(not(not(var("a"))), "not not a");
        assert_round_trip(bin(Op::Eq, not(var("a")), var("b")), "(not a) == b");
        assert_round_trip(bin(Op::Eq, var("b"), not(var("a"))), "b == (not a)");
    }

    #[test]
    fn call_arguments_are_not_parenthesized() {
        assert_round_trip(
            bin(
                Op::Mul,
                Expr::Call {
                    callee: keystone_lang::Callee::Rand,
                    args: vec![
                        Box::new(bin(Op::Add, var("x"), Expr::Uint(1))),
                        Box::new(Expr::Uint(10)),
                    ],
                },
                Expr::Uint(2),
            ),
            "rand(x + 1, 10) * 2",
        );
    }

    #[test]
    fn text_parentheses_survive_reload() {
        let stmts = parse("x = (x + 1) * 2\nif not (a or b)\n  move forward\nend").unwrap();
        let code = statements_to_string(&stmts, 0);
        assert_eq!(
            code,
            "x = (x + 1) * 2\nif not (a or b)\n  move forward\nend\n"
        );
    }
}
