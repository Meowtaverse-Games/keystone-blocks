use bevy::prelude::*;
use keystone_lang::{Expr, Statement, Type, TypeContext};

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

#[derive(Resource)]
pub struct VplState {
    pub blocks: Vec<Statement>,
    pub type_ctx: TypeContext,
    pub generated_code: String,
}

impl Default for VplState {
    fn default() -> Self {
        Self {
            blocks: Vec::new(),
            type_ctx: TypeContext::new(),
            generated_code: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum InferResult {
    Type(Type),
    Unknown,
    Invalid,
}
