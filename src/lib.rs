mod renderer;
mod structs;
mod utils;
mod visual;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};
use keystone_lang::{Direction, Expr, Statement};
use renderer::*;
use structs::*;
use utils::*;
use visual::*;

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
                ui_left.heading("➕ Add Blocks (Drag or Click)");
                ui_left.separator();

                render_palette_button(
                    ui_left,
                    "🏃 Move (Direction)",
                    Statement::Move(Expr::Direction(Direction::Forward)),
                    &mut state.blocks,
                );
                render_palette_button(
                    ui_left,
                    "🔄 Turn (Direction)",
                    Statement::Turn(Expr::Direction(Direction::Right)),
                    &mut state.blocks,
                );
                render_palette_button(
                    ui_left,
                    "💬 Print (String)",
                    Statement::Print(Expr::String("hello".to_string())),
                    &mut state.blocks,
                );
                render_palette_button(
                    ui_left,
                    "💤 Sleep (Float)",
                    Statement::Sleep(Expr::Float(1.0)),
                    &mut state.blocks,
                );

                ui_left.add_space(10.0);
                ui_left.label("ーーー Nest Blocks ーーー");

                render_palette_button(
                    ui_left,
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
                render_palette_button(
                    ui_left,
                    "🔁 Loop (Count)",
                    Statement::Loop(Expr::Uint(3), Vec::new()),
                    &mut state.blocks,
                );
                render_palette_button(
                    ui_left,
                    "🔄 While (true)",
                    Statement::While(Expr::Boolean(true), Vec::new()),
                    &mut state.blocks,
                );

                ui_left.add_space(20.0);
                if ui_left.button("🗑️ Clear All").clicked() {
                    state.blocks.clear();
                }

                let ui_right = &mut columns[1];
                ui_right.heading("📝 Current Program");
                ui_right.separator();

                egui::ScrollArea::vertical().show(ui_right, |ui| {
                    if state.blocks.is_empty() {
                        ui.label("(Drag blocks here)");
                    }

                    let mut move_request = None;

                    render_block_list(ui, &mut state.blocks, Vec::new(), &mut move_request);

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
