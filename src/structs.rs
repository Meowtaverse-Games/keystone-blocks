use std::collections::HashSet;

use bevy::prelude::*;
use bevy_egui::egui;
use keystone_lang::{Expr, Statement, Type, TypeContext};

use crate::sync::CodeSync;

#[derive(Clone, Debug)]
pub enum DraggedBlock {
    NewStatement(Statement),
    MoveStatement { path: Vec<usize> },
}

#[derive(Clone, Debug)]
pub struct DraggedExpr(pub Expr);

#[derive(Clone, Debug)]
pub struct DropTarget {
    pub list_path: Vec<usize>,
    pub insert_idx: usize,
    pub marker: Option<egui::Rect>,
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
    pub is_visible: bool,
    pub allowed_commands: Option<HashSet<String>>,
    pub(crate) sync: CodeSync,
}

impl Default for VplState {
    fn default() -> Self {
        Self {
            blocks: Vec::new(),
            type_ctx: TypeContext::new(),
            generated_code: String::new(),
            is_visible: false,
            allowed_commands: None,
            sync: CodeSync::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum InferResult {
    Type(Type),
    Unknown,
    Invalid,
}
