use std::collections::HashSet;

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

fn stmt_command(stmt: &Statement) -> Option<&'static str> {
    match stmt {
        Statement::Move(_) => Some("move"),
        Statement::Sleep(_) => Some("sleep"),
        Statement::Dig(_) => Some("dig"),
        Statement::Place(_) => Some("place"),
        _ => None,
    }
}

fn callee_command(callee: &Callee) -> Option<&'static str> {
    match callee {
        Callee::IsTouched => Some("is_touched"),
        Callee::IsEmpty => Some("is_empty"),
        Callee::Rand => None,
    }
}

pub fn is_stmt_allowed(stmt: &Statement, allowed: Option<&HashSet<String>>) -> bool {
    disallowed_in_program(std::slice::from_ref(stmt), allowed).is_empty()
}

pub fn is_expr_allowed(expr: &Expr, allowed: Option<&HashSet<String>>) -> bool {
    let mut found = Vec::new();
    if let Some(allowed) = allowed {
        collect_disallowed_in_expr(expr, allowed, &mut found);
    }
    found.is_empty()
}

pub fn disallowed_in_stmt(
    stmt: &Statement,
    allowed: Option<&HashSet<String>>,
) -> Vec<&'static str> {
    let mut found = Vec::new();
    if let Some(allowed) = allowed {
        if let Some(command) = stmt_command(stmt)
            && !allowed.contains(command)
        {
            found.push(command);
        }
        let expr = match stmt {
            Statement::Loop(e, _)
            | Statement::While(e, _)
            | Statement::If(e, _)
            | Statement::Print(e)
            | Statement::Move(e)
            | Statement::Turn(e)
            | Statement::Dig(e)
            | Statement::Place(e)
            | Statement::Sleep(e)
            | Statement::Let(_, e)
            | Statement::Send(e)
            | Statement::Receive(e) => e,
        };
        collect_disallowed_in_expr(expr, allowed, &mut found);
        found.sort_unstable();
        found.dedup();
    }
    found
}

pub fn disallowed_in_program(
    stmts: &[Statement],
    allowed: Option<&HashSet<String>>,
) -> Vec<&'static str> {
    let mut found = Vec::new();
    for stmt in stmts {
        found.extend(disallowed_in_stmt(stmt, allowed));
        if let Statement::Loop(_, body) | Statement::While(_, body) | Statement::If(_, body) = stmt
        {
            found.extend(disallowed_in_program(body, allowed));
        }
    }
    found.sort_unstable();
    found.dedup();
    found
}

fn collect_disallowed_in_expr(
    expr: &Expr,
    allowed: &HashSet<String>,
    found: &mut Vec<&'static str>,
) {
    match expr {
        Expr::Call { callee, args } => {
            if let Some(command) = callee_command(callee)
                && !allowed.contains(command)
            {
                found.push(command);
            }
            for arg in args {
                collect_disallowed_in_expr(arg, allowed, found);
            }
        }
        Expr::Unary { exp, .. } => collect_disallowed_in_expr(exp, allowed, found),
        Expr::Binary { lhs, rhs, .. } => {
            collect_disallowed_in_expr(lhs, allowed, found);
            collect_disallowed_in_expr(rhs, allowed, found);
        }
        _ => {}
    }
}

pub fn get_stmt_info(stmt: &Statement) -> (&'static str, &'static str) {
    match stmt {
        Statement::Move(_) => ("🏃", "Move"),
        Statement::Turn(_) => ("🔄", "Turn"),
        Statement::Dig(_) => ("🔨", "Dig"),
        Statement::Place(_) => ("✒️", "Place"),
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
