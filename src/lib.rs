mod code;
mod colorizer;
mod renderer;

mod structs;
mod utils;
use bevy::prelude::*;
use bevy_egui::egui::{self, Color32};
use code::*;
use keystone_lang::{Callee, Direction, Expr, Op, Statement, TypeContext, UnaryOp};
use renderer as render;
pub use structs::VplState;
use structs::*;
use utils::*;

pub struct VisualProgrammingPlugin;
impl Plugin for VisualProgrammingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VplState>();
    }
}

pub fn show_vpl_contents(ui: &mut egui::Ui, state: &mut VplState) {
    state.type_ctx = TypeContext::new();
    let type_ctx = state.type_ctx.clone();

    let rect = ui.available_rect_before_wrap().intersect(ui.clip_rect());
    let available = rect.size();

    let width_id = ui.make_persistent_id("vpl_palette_width");
    let measured = ui.data(|d| d.get_temp::<f32>(width_id));
    let palette_width = measured
        .map_or(available.x * 0.35, |w| w + PALETTE_EXTRA_WIDTH)
        .min(available.x * 0.5);

    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Min)),
        |ui| {
            let content_width = ui
                .allocate_ui_with_layout(
                    egui::vec2(palette_width, available.y),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_width(palette_width);
                        render_palette_panel(ui, &mut state.blocks, &type_ctx)
                    },
                )
                .inner;
            ui.data_mut(|d| d.insert_temp(width_id, content_width));

            ui.separator();

            ui.vertical(|ui| {
                render_program_panel(ui, &mut state.blocks, &type_ctx);
            });
        },
    );
}

const PALETTE_EXTRA_WIDTH: f32 = 16.0;

pub fn generate_code_from_state(state: &VplState) -> String {
    statements_to_string(&state.blocks, 0)
}

// pub fn generate(state: ) {
//     ui.separator();
//
//     state.generated_code = statements_to_string(&state.blocks, 0);
//     ui.heading("GENERATED CODE");
//     ui.code(&state.generated_code);
// }

#[derive(Clone)]
enum PaletteEntry {
    Section(&'static str),
    SubSection(&'static str),
    Stmt(Statement),
    Value(Option<&'static str>, Expr),
}

fn default_palette() -> Vec<PaletteEntry> {
    use PaletteEntry::*;

    vec![
        Stmt(Statement::Move(Expr::Direction(Direction::Right))),
        Stmt(Statement::Turn(Expr::Direction(Direction::Left))),
        Stmt(Statement::Dig(Expr::Direction(Direction::Up))),
        Stmt(Statement::Place(Expr::Direction(Direction::Down))),
        Stmt(Statement::Print(Expr::String("hello".to_string()))),
        Stmt(Statement::Sleep(Expr::Float(1.0))),
        Section("Variables & Events"),
        Stmt(Statement::Let("x".to_string(), Expr::Uint(1))),
        Stmt(Statement::Send(Expr::String("signal".to_string()))),
        Stmt(Statement::Receive(Expr::String("signal".to_string()))),
        Section("Nest Blocks"),
        Stmt(Statement::If(
            Expr::Call {
                callee: Callee::IsTouched,
                args: vec![],
            },
            Vec::new(),
        )),
        Stmt(Statement::Loop(Expr::Uint(3), Vec::new())),
        Stmt(Statement::While(Expr::Boolean(true), Vec::new())),
        Section("Value Blocks (Expr)"),
        SubSection("Literals"),
        Value(Some("Boolean"), Expr::Boolean(true)),
        Value(Some("Number"), Expr::Uint(0)),
        Value(Some("Float"), Expr::Float(0.0)),
        Value(Some("Text"), Expr::String("hello".to_string())),
        Value(Some("Direction"), Expr::Direction(Direction::Forward)),
        SubSection("Variables"),
        Value(Some("Variable"), Expr::Var("x".to_string())),
        SubSection("Operators"),
        Value(
            Some("Math"),
            Expr::Binary {
                op: Op::Add,
                lhs: Box::new(Expr::Var("x".to_string())),
                rhs: Box::new(Expr::Uint(1)),
            },
        ),
        Value(
            Some("Compare"),
            Expr::Binary {
                op: Op::Eq,
                lhs: Box::new(Expr::Var("x".to_string())),
                rhs: Box::new(Expr::Uint(0)),
            },
        ),
        Value(
            Some("Logic"),
            Expr::Binary {
                op: Op::And,
                lhs: Box::new(Expr::Boolean(true)),
                rhs: Box::new(Expr::Boolean(true)),
            },
        ),
        Value(
            None,
            Expr::Unary {
                op: UnaryOp::Not,
                exp: Box::new(Expr::Boolean(true)),
            },
        ),
        SubSection("Functions"),
        Value(
            None,
            Expr::Call {
                callee: Callee::Rand,
                args: vec![],
            },
        ),
        Value(
            None,
            Expr::Call {
                callee: Callee::Rand,
                args: vec![Box::new(Expr::Uint(10))],
            },
        ),
        Value(
            None,
            Expr::Call {
                callee: Callee::Rand,
                args: vec![Box::new(Expr::Uint(1)), Box::new(Expr::Uint(10))],
            },
        ),
        Value(
            None,
            Expr::Call {
                callee: Callee::IsTouched,
                args: vec![],
            },
        ),
        Value(
            None,
            Expr::Call {
                callee: Callee::IsEmpty,
                args: vec![Box::new(Expr::Direction(Direction::Forward))],
            },
        ),
    ]
}

fn render_palette_panel(
    ui: &mut egui::Ui,
    blocks: &mut Vec<Statement>,
    type_ctx: &TypeContext,
) -> f32 {
    ui.heading("➕ Add Blocks");
    ui.separator();

    let entries_id = ui.make_persistent_id("vpl_palette_entries");
    let mut entries = ui
        .data(|d| d.get_temp::<Vec<PaletteEntry>>(entries_id))
        .unwrap_or_else(default_palette);

    let item_width = ui
        .scope(|ui| {
            ui.visuals_mut().widgets.active.bg_fill = Color32::TRANSPARENT;
            ui.visuals_mut().widgets.inactive.bg_fill = Color32::TRANSPARENT;
            ui.visuals_mut().widgets.active.bg_stroke = egui::Stroke::NONE;
            ui.visuals_mut().widgets.inactive.bg_stroke = egui::Stroke::NONE;

            let (item_width, dropped) = ui.dnd_drop_zone::<DraggedBlock, _>(
                egui::Frame::NONE.fill(egui::Color32::TRANSPARENT),
                |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("left_palette_scroll")
                        .show(ui, |ui| {
                            let mut item_width: f32 = 0.0;

                            for (idx, entry) in entries.iter_mut().enumerate() {
                                let block_id = egui::Id::new(("vpl_palette", idx));
                                match entry {
                                    PaletteEntry::Section(text) => {
                                        ui.add_space(10.0);
                                        section_label(ui, text);
                                    }
                                    PaletteEntry::SubSection(text) => {
                                        ui.add_space(4.0);
                                        ui.label(egui::RichText::new(*text).weak().size(11.0));
                                    }
                                    PaletteEntry::Stmt(stmt) => {
                                        item_width = item_width.max(render::palette_block(
                                            ui, stmt, block_id, blocks, type_ctx,
                                        ));
                                    }
                                    PaletteEntry::Value(caption, expr) => {
                                        item_width = item_width.max(render::palette_expr_block(
                                            ui, *caption, expr, block_id, type_ctx,
                                        ));
                                    }
                                }
                            }

                            // ui.add_space(20.0);
                            // if ui.button("🗑️ CLEAR ALL").clicked() {
                            //     blocks.clear();
                            // }

                            item_width
                        })
                        .inner
                },
            );
            if let Some(payload) = dropped
                && let DraggedBlock::MoveStatement { path } = payload.as_ref()
            {
                remove_statement_at_path(blocks, path);
            }
            item_width.inner
        })
        .inner;

    ui.data_mut(|d| d.insert_temp(entries_id, entries));
    item_width
}

fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.separator();
    ui.label(egui::RichText::new(text).weak());
}

fn render_program_panel(ui: &mut egui::Ui, blocks: &mut Vec<Statement>, type_ctx: &TypeContext) {
    ui.heading("📝 Current Program");
    ui.separator();

    egui::ScrollArea::vertical()
        .id_salt("right_program_scroll")
        .show(ui, |ui| {
            let mut move_request = None;

            render::block_list(ui, blocks, Vec::new(), &mut move_request, type_ctx);

            if let Some(req) = move_request {
                handle_move_request(blocks, req);
            }
        });
}

fn handle_move_request(blocks: &mut Vec<Statement>, req: MoveRequest) {
    if let Some(moved_block) = remove_statement_at_path(blocks, &req.src_path) {
        let mut target_path = req.target_path;
        let mut insert_idx = req.insert_idx;

        if req.src_path.len() == target_path.len() + 1 && req.src_path.starts_with(&target_path) {
            let src_idx = *req.src_path.last().unwrap();
            if src_idx < insert_idx {
                insert_idx = insert_idx.saturating_sub(1);
            }
        } else {
            let src_depth = req.src_path.len();
            for i in 0..target_path.len().min(src_depth) {
                if req.src_path[..i] == target_path[..i] {
                    let src_idx = req.src_path[i];
                    let target_idx = target_path[i];

                    if i == src_depth - 1 {
                        if src_idx < target_idx {
                            target_path[i] -= 1;
                        }
                        break;
                    } else if src_idx != target_idx {
                        break;
                    }
                }
            }
        }

        insert_statement_at_path(blocks, &target_path, insert_idx, moved_block);
    }
}
