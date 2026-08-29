use bevy::prelude::*;
use keystone_lang::{Expr, Statement};

#[derive(Clone, Debug)]
pub enum DraggedBlock {
    NewStatement(Statement),
    MoveStatement { path: Vec<usize> },

    NewExpr(Expr),
}

#[derive(Clone, Debug)]
pub struct MoveRequest {
    pub src_path: Vec<usize>,
    pub target_path: Vec<usize>,
    pub insert_idx: usize,
}

#[derive(Resource, Default)]
pub struct VplState {
    pub blocks: Vec<Statement>,
    pub generated_code: String,
}
