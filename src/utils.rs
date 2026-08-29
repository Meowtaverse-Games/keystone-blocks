use keystone_lang::Statement;

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
