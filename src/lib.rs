mod code;
mod colorizer;
mod renderer;

mod structs;
mod utils;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};
use code::*;
use keystone_lang::{Callee, Direction, Expr, Op, Statement};
use renderer as render;
use structs::*;
use utils::*;

pub struct VisualProgrammingPlugin;
impl Plugin for VisualProgrammingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin::default())
            .init_resource::<VplState>()
            .add_systems(EguiPrimaryContextPass, vpl_ui_system);
    }
}

fn vpl_ui_system(mut contexts: EguiContexts, mut state: ResMut<VplState>) -> Result<(), BevyError> {
    let ctx = contexts.ctx_mut()?;

    egui::Window::new("Keystone VPL Studio")
        .default_size([750.0, 500.0])
        .show(ctx, |ui| {
            ui.columns(2, |columns| {
                let ui_left = &mut columns[0];
                ui_left.heading("➕ Add Blocks");
                ui_left.separator();

                egui::ScrollArea::vertical()
                    .id_salt("left_palette_scroll")
                    .show(ui_left, |ui| {
                        render::palette_button(
                            ui,
                            "🏃 Move (Direction)",
                            Statement::Move(Expr::Direction(Direction::Forward)),
                            &mut state.blocks,
                        );
                        render::palette_button(
                            ui,
                            "🔄 Turn (Direction)",
                            Statement::Turn(Expr::Direction(Direction::Right)),
                            &mut state.blocks,
                        );
                        render::palette_button(
                            ui,
                            "🔨 Dig (Direction)",
                            Statement::Dig(Expr::Direction(Direction::Up)),
                            &mut state.blocks,
                        );
                        render::palette_button(
                            ui,
                            "💬 Print (String)",
                            Statement::Print(Expr::String("hello".to_string())),
                            &mut state.blocks,
                        );
                        render::palette_button(
                            ui,
                            "💤 Sleep (Float)",
                            Statement::Sleep(Expr::Float(1.0)),
                            &mut state.blocks,
                        );

                        ui.add_space(10.0);
                        ui.label("——— Variables & Events ———");

                        render::palette_button(
                            ui,
                            "📝 Let (Variable)",
                            Statement::Let("x".to_string(), Expr::Uint(1)),
                            &mut state.blocks,
                        );

                        render::palette_button(
                            ui,
                            "📡 Send (Event)",
                            Statement::Send(Expr::String("signal".to_string())),
                            &mut state.blocks,
                        );

                        render::palette_button(
                            ui,
                            "📥 Receive (Wait)",
                            Statement::Receive(Expr::String("signal".to_string())),
                            &mut state.blocks,
                        );

                        ui.add_space(10.0);
                        ui.label("——— Nest Blocks ———");

                        render::palette_button(
                            ui,
                            "❓ If (is_touched())",
                            Statement::If(
                                Expr::Call {
                                    callee: keystone_lang::Callee::IsTouched,
                                    args: vec![],
                                },
                                Vec::new(),
                            ),
                            &mut state.blocks,
                        );
                        render::palette_button(
                            ui,
                            "🔁 Loop (Count)",
                            Statement::Loop(Expr::Uint(3), Vec::new()),
                            &mut state.blocks,
                        );
                        render::palette_button(
                            ui,
                            "🔄 While (true)",
                            Statement::While(Expr::Boolean(true), Vec::new()),
                            &mut state.blocks,
                        );

                        ui.add_space(10.0);
                        ui.label("——— Value Blocks (Expr) ———");

                        render::expr_palette_button(ui, "Number (0)", Expr::Uint(0));
                        render::expr_palette_button(ui, "Float (0.0)", Expr::Float(0.0));
                        render::expr_palette_button(
                            ui,
                            "Text (\"hello\")",
                            Expr::String("hello".to_string()),
                        );
                        render::expr_palette_button(
                            ui,
                            "Direction (Forward)",
                            Expr::Direction(Direction::Forward),
                        );
                        render::expr_palette_button(
                            ui,
                            "rand()",
                            Expr::Call {
                                callee: Callee::Rand,
                                args: vec![],
                            },
                        );
                        render::expr_palette_button(ui, "Variable (x)", Expr::Var("x".to_string()));
                        render::expr_palette_button(
                            ui,
                            "Comparison (a == b)",
                            Expr::Binary {
                                op: Op::Eq,
                                lhs: Box::new(Expr::Var("x".to_string())),
                                rhs: Box::new(Expr::Uint(0)),
                            },
                        );
                        render::expr_palette_button(
                            ui,
                            "Math (a + b)",
                            Expr::Binary {
                                op: Op::Add,
                                lhs: Box::new(Expr::Var("x".to_string())),
                                rhs: Box::new(Expr::Uint(1)),
                            },
                        );
                        render::expr_palette_button(
                            ui,
                            "is_touched()",
                            Expr::Call {
                                callee: Callee::IsTouched,
                                args: vec![],
                            },
                        );
                        render::expr_palette_button(
                            ui,
                            "is_empty(dir)",
                            Expr::Call {
                                callee: Callee::IsEmpty,
                                args: vec![Box::new(Expr::Direction(Direction::Forward))],
                            },
                        );

                        ui.add_space(20.0);
                        if ui.button("🗑️ CLEAR ALL").clicked() {
                            state.blocks.clear();
                        }
                    });

                let ui_right = &mut columns[1];
                ui_right.heading("📝 Current Program");
                ui_right.separator();

                egui::ScrollArea::vertical()
                    .id_salt("right_program_scroll")
                    .show(ui_right, |ui| {
                        if state.blocks.is_empty() {
                            ui.label("(Drag blocks here)");
                        }

                        let mut move_request = None;

                        render::block_list(ui, &mut state.blocks, Vec::new(), &mut move_request);

                        if let Some(req) = move_request {
                            if let Some(moved_block) =
                                remove_statement_at_path(&mut state.blocks, &req.src_path)
                            {
                                let adjusted_target_path =
                                    adjust_path_after_removal(&req.target_path, &req.src_path);

                                let mut adjusted_idx = req.insert_idx;
                                if req.src_path.len() == req.target_path.len() + 1
                                    && req.src_path.starts_with(&req.target_path)
                                    && req.src_path.last().copied() < Some(req.insert_idx)
                                {
                                    adjusted_idx = adjusted_idx.saturating_sub(1);
                                }

                                insert_statement_at_path(
                                    &mut state.blocks,
                                    &adjusted_target_path,
                                    adjusted_idx,
                                    moved_block,
                                );
                            }
                        }
                    });
            });
            ui.separator();

            state.generated_code = statements_to_string(&state.blocks, 0);
            ui.heading("GENERATED CODE");
            ui.code(&state.generated_code);
        });

    Ok(())
}
