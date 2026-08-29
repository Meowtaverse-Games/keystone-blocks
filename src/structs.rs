use bevy::prelude::*;
use keystone_lang::Statement;

#[derive(Clone, PartialEq, Debug)]
pub enum DraggedBlock {
    New(Statement),
    Move { path: Vec<usize> },
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
