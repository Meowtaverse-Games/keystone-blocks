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

    let width_id = ui.make_persistent_id("vpl_palette_width");
    let available = ui.available_size();
    let measured = ui.data(|d| d.get_temp::<f32>(width_id));
    let palette_width = measured
        .map_or(available.x * 0.35, |w| w + PALETTE_EXTRA_WIDTH)
        .min(available.x * 0.5);

    ui.horizontal_top(|ui| {
        let content_width = ui
            .allocate_ui_with_layout(
                egui::vec2(palette_width, available.y),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_width(palette_width);
                    render_palette_panel(ui, &mut state.blocks)
                },
            )
            .inner;
        ui.data_mut(|d| d.insert_temp(width_id, content_width));

        ui.separator();

        ui.vertical(|ui| {
            render_program_panel(ui, &mut state.blocks, &type_ctx);
        });
    });
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

fn render_palette_panel(ui: &mut egui::Ui, blocks: &mut Vec<Statement>) -> f32 {
    ui.heading("➕ Add Blocks");
    ui.separator();

    ui.scope(|ui| {
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

                        item_width = item_width.max(render::palette_button(
                            ui,
                            "🏃 Move (Direction)",
                            Statement::Move(Expr::Direction(Direction::Right)),
                            blocks,
                        ));
                        item_width = item_width.max(render::palette_button(
                            ui,
                            "🔄 Turn (Direction)",
                            Statement::Turn(Expr::Direction(Direction::Left)),
                            blocks,
                        ));
                        item_width = item_width.max(render::palette_button(
                            ui,
                            "🔨 Dig (Direction)",
                            Statement::Dig(Expr::Direction(Direction::Up)),
                            blocks,
                        ));
                        item_width = item_width.max(render::palette_button(
                            ui,
                            "🔨 Place (Direction)",
                            Statement::Place(Expr::Direction(Direction::Down)),
                            blocks,
                        ));
                        item_width = item_width.max(render::palette_button(
                            ui,
                            "💬 Print (String)",
                            Statement::Print(Expr::String("hello".to_string())),
                            blocks,
                        ));
                        item_width = item_width.max(render::palette_button(
                            ui,
                            "💤 Sleep (Float)",
                            Statement::Sleep(Expr::Float(1.0)),
                            blocks,
                        ));

                        ui.add_space(10.0);
                        section_label(ui, "Variables & Events");

                        item_width = item_width.max(render::palette_button(
                            ui,
                            "📝 Let (Variable)",
                            Statement::Let("x".to_string(), Expr::Uint(1)),
                            blocks,
                        ));
                        item_width = item_width.max(render::palette_button(
                            ui,
                            "📡 Send (Event)",
                            Statement::Send(Expr::String("signal".to_string())),
                            blocks,
                        ));
                        item_width = item_width.max(render::palette_button(
                            ui,
                            "📥 Receive (Wait)",
                            Statement::Receive(Expr::String("signal".to_string())),
                            blocks,
                        ));

                        ui.add_space(10.0);
                        section_label(ui, "Nest Blocks");

                        item_width = item_width.max(render::palette_button(
                            ui,
                            "❓ If (is_touched())",
                            Statement::If(
                                Expr::Call {
                                    callee: keystone_lang::Callee::IsTouched,
                                    args: vec![],
                                },
                                Vec::new(),
                            ),
                            blocks,
                        ));
                        item_width = item_width.max(render::palette_button(
                            ui,
                            "🔁 Loop (Count)",
                            Statement::Loop(Expr::Uint(3), Vec::new()),
                            blocks,
                        ));
                        item_width = item_width.max(render::palette_button(
                            ui,
                            "🔄 While (true)",
                            Statement::While(Expr::Boolean(true), Vec::new()),
                            blocks,
                        ));

                        ui.add_space(10.0);
                        section_label(ui, "Value Blocks (Expr)");

                        ui.label(egui::RichText::new("Literals").weak().size(11.0));
                        item_width = item_width.max(render::expr_palette_button(
                            ui,
                            "Boolean (true)",
                            Expr::Boolean(true),
                        ));
                        item_width = item_width.max(render::expr_palette_button(
                            ui,
                            "Number (0)",
                            Expr::Uint(0),
                        ));
                        item_width = item_width.max(render::expr_palette_button(
                            ui,
                            "Float (0.0)",
                            Expr::Float(0.0),
                        ));
                        item_width = item_width.max(render::expr_palette_button(
                            ui,
                            "Text (\"hello\")",
                            Expr::String("hello".to_string()),
                        ));
                        item_width = item_width.max(render::expr_palette_button(
                            ui,
                            "Direction (Forward)",
                            Expr::Direction(Direction::Forward),
                        ));

                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("Variables").weak().size(11.0));
                        item_width = item_width.max(render::expr_palette_button(
                            ui,
                            "Variable (x)",
                            Expr::Var("x".to_string()),
                        ));

                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("Operators").weak().size(11.0));
                        item_width = item_width.max(render::expr_palette_button(
                            ui,
                            "Math (a + b)",
                            Expr::Binary {
                                op: Op::Add,
                                lhs: Box::new(Expr::Var("x".to_string())),
                                rhs: Box::new(Expr::Uint(1)),
                            },
                        ));
                        item_width = item_width.max(render::expr_palette_button(
                            ui,
                            "Comparison (a == b)",
                            Expr::Binary {
                                op: Op::Eq,
                                lhs: Box::new(Expr::Var("x".to_string())),
                                rhs: Box::new(Expr::Uint(0)),
                            },
                        ));
                        item_width = item_width.max(render::expr_palette_button(
                            ui,
                            "Logic (a and b)",
                            Expr::Binary {
                                op: Op::And,
                                lhs: Box::new(Expr::Boolean(true)),
                                rhs: Box::new(Expr::Boolean(true)),
                            },
                        ));
                        item_width = item_width.max(render::expr_palette_button(
                            ui,
                            "not (a)",
                            Expr::Unary {
                                op: UnaryOp::Not,
                                exp: Box::new(Expr::Boolean(true)),
                            },
                        ));

                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("Functions").weak().size(11.0));
                        item_width = item_width.max(render::expr_palette_button(
                            ui,
                            "rand()",
                            Expr::Call {
                                callee: Callee::Rand,
                                args: vec![],
                            },
                        ));
                        item_width = item_width.max(render::expr_palette_button(
                            ui,
                            "rand(n)",
                            Expr::Call {
                                callee: Callee::Rand,
                                args: vec![Box::new(Expr::Uint(10))],
                            },
                        ));
                        item_width = item_width.max(render::expr_palette_button(
                            ui,
                            "rand(a, b)",
                            Expr::Call {
                                callee: Callee::Rand,
                                args: vec![Box::new(Expr::Uint(1)), Box::new(Expr::Uint(10))],
                            },
                        ));
                        item_width = item_width.max(render::expr_palette_button(
                            ui,
                            "is_touched()",
                            Expr::Call {
                                callee: Callee::IsTouched,
                                args: vec![],
                            },
                        ));
                        item_width = item_width.max(render::expr_palette_button(
                            ui,
                            "is_empty(dir)",
                            Expr::Call {
                                callee: Callee::IsEmpty,
                                args: vec![Box::new(Expr::Direction(Direction::Forward))],
                            },
                        ));

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
    .inner
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
