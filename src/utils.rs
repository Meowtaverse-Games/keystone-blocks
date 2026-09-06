use keystone_lang::{Callee, Expr, Op, Statement, Type, UnaryOp};

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

pub fn adjust_path_after_removal(target_path: &[usize], removed_path: &[usize]) -> Vec<usize> {
    let mut adjusted = target_path.to_vec();

    let common_len = target_path
        .iter()
        .zip(removed_path.iter())
        .take_while(|(a, b)| a == b)
        .count();

    if common_len < removed_path.len() && common_len < target_path.len() {
        if removed_path[common_len] < target_path[common_len] {
            adjusted[common_len] -= 1;
        }
    }

    adjusted
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

pub fn infer_expr_type(expr: &Expr) -> Option<Type> {
    match expr {
        Expr::Uint(_) => Some(Type::Uint),
        Expr::Float(_) => Some(Type::Float),
        Expr::String(_) => Some(Type::String),
        Expr::Boolean(_) => Some(Type::Boolean),
        Expr::Direction(_) => Some(Type::Direction),
        Expr::Unary { op, .. } => match op {
            UnaryOp::Not => Some(Type::Boolean),
        },
        Expr::Binary { op, .. } => match op {
            Op::Eq | Op::Neq | Op::Lt | Op::Gt | Op::Le | Op::Ge | Op::And | Op::Or => {
                Some(Type::Boolean)
            }
            Op::Add | Op::Sub | Op::Mul | Op::Div => None,
        },
        Expr::Call { callee, .. } => match callee {
            Callee::IsTouched | Callee::IsEmpty => Some(Type::Boolean),
            Callee::Rand => Some(Type::Uint),
        },
        Expr::Var(_) => None,
    }
}

pub fn is_type_compatible(expected: Option<Type>, incoming: &Expr) -> bool {
    let expected_type = match expected {
        Some(t) => t,
        None => return true,
    };

    let incoming_type = match infer_expr_type(incoming) {
        Some(t) => t,
        None => return true,
    };

    expected_type == incoming_type
}
