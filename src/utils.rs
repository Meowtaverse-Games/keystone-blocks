use keystone_lang::{Callee, Expr, Op, Statement, Type, TypeContext, expr_check};

use crate::structs::InferResult;

pub fn remove_statement_at_path(blocks: &mut Vec<Statement>, path: &[usize]) -> Option<Statement> {
    if path.is_empty() {
        return None;
    }
    if path.len() == 1 {
        if path[0] < blocks.len() {
            return Some(blocks.remove(path[0]));
        }
        return None;
    }

    let head = path[0];
    if head >= blocks.len() {
        return None;
    }

    let body = match &mut blocks[head] {
        Statement::If(_, body) => body,
        Statement::Loop(_, body) => body,
        Statement::While(_, body) => body,
        _ => return None,
    };

    remove_statement_at_path(body, &path[1..])
}

pub fn insert_statement_at_path(
    blocks: &mut Vec<Statement>,
    target_path: &[usize],
    insert_idx: usize,
    stmt: Statement,
) {
    if target_path.is_empty() {
        let idx = insert_idx.min(blocks.len());
        blocks.insert(idx, stmt);
        return;
    }

    let head = target_path[0];
    if head >= blocks.len() {
        return;
    }

    let body = match &mut blocks[head] {
        Statement::If(_, body) => body,
        Statement::Loop(_, body) => body,
        Statement::While(_, body) => body,
        _ => return,
    };

    insert_statement_at_path(body, &target_path[1..], insert_idx, stmt);
}

pub fn is_ancestor(ancestor_path: &[usize], target_path: &[usize]) -> bool {
    if ancestor_path.len() >= target_path.len() {
        return false;
    }
    target_path.starts_with(ancestor_path)
}

pub fn get_stmt_info(stmt: &Statement) -> (&'static str, &'static str) {
    match stmt {
        Statement::Move(_) => ("🏃", "Move"),
        Statement::Turn(_) => ("🔄", "Turn"),
        Statement::Dig(_) => ("🔨", "Dig"),
        Statement::Print(_) => ("💬", "Print"),
        Statement::Sleep(_) => ("💤", "Sleep"),
        Statement::Let(_, _) => ("📝", "Let"),
        Statement::Send(_) => ("📡", "Send"),
        Statement::Receive(_) => ("📥", "Receive"),
        Statement::If(_, _) => ("❓", "If"),
        Statement::Loop(_, _) => ("🔁", "Loop"),
        Statement::While(_, _) => ("🔄", "While"),
    }
}

pub fn infer_expr_type(expr: &Expr, ctx: &TypeContext) -> InferResult {
    let mut temp_ctx = ctx.clone();

    match expr_check(expr, &mut temp_ctx) {
        Ok(t) => InferResult::Type(t),
        Err(_) => match expr {
            Expr::Binary { op, .. } => match op {
                Op::Eq | Op::Neq | Op::Lt | Op::Gt | Op::Le | Op::Ge | Op::And | Op::Or => {
                    InferResult::Type(Type::Boolean)
                }
                Op::Add | Op::Sub | Op::Mul | Op::Div => InferResult::Type(Type::Uint),
            },
            Expr::Unary { exp, .. } => match infer_expr_type(exp, ctx) {
                InferResult::Type(Type::Boolean) => InferResult::Type(Type::Boolean),
                InferResult::Type(_) => InferResult::Invalid,
                InferResult::Unknown => InferResult::Type(Type::Boolean),
                InferResult::Invalid => InferResult::Invalid,
            },
            Expr::Call {
                callee: Callee::Rand,
                args,
            } => match args.len() {
                0 => InferResult::Type(Type::Float),
                1 | 2 => InferResult::Type(Type::Uint),
                _ => InferResult::Invalid,
            },
            _ => InferResult::Unknown,
        },
    }
}

pub fn is_type_compatible(expected: Option<Type>, incoming: &Expr, ctx: &TypeContext) -> bool {
    let expected_type = match expected {
        Some(t) => t,
        None => return true,
    };

    let infer = infer_expr_type(incoming, ctx);

    let is_compat = match &infer {
        InferResult::Type(t) => *t == expected_type,
        InferResult::Unknown => true,
        InferResult::Invalid => false,
    };

    // println!(
    //     "Expected: {:?}, Infer: {:?}, Result: {}",
    //     expected_type, infer, is_compat
    // );

    is_compat
}
