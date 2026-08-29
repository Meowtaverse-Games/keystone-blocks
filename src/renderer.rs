use bevy_egui::egui;
use keystone_lang::{Direction, Expr, Statement};

use crate::{
    structs::{DraggedBlock, MoveRequest},
    utils::is_ancestor,
};

pub fn render_palette_button(
    ui: &mut egui::Ui,
    label: &str,
    stmt: Statement,
    target_blocks: &mut Vec<Statement>,
) {
    let item_id = egui::Id::new(label);
    let response = ui.dnd_drag_source(item_id, DraggedBlock::New(stmt.clone()), |ui| {
        ui.button(label)
    });

    if response.response.clicked() {
        target_blocks.push(stmt);
    }
}

pub fn render_block_list(
    ui: &mut egui::Ui,
    current_blocks: &mut Vec<Statement>,
    current_path: Vec<usize>,
    move_request: &mut Option<MoveRequest>,
) {
    let mut delete_target_idx = None;

    for idx in 0..current_blocks.len() {
        let mut this_path = current_path.clone();
        this_path.push(idx);

        let current_id_str = this_path
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join("_");
        let block_id = egui::Id::new(&current_id_str);

        let (_, dropped_payload) =
            ui.dnd_drop_zone::<DraggedBlock, _>(egui::Frame::NONE.inner_margin(2.0), |ui| {
                ui.horizontal(|ui| {
                    ui.dnd_drag_source(
                        block_id.with("drag_handle"),
                        DraggedBlock::Move {
                            path: this_path.clone(),
                        },
                        |ui| {
                            ui.horizontal(|ui| {
                                ui.label("☰").on_hover_cursor(egui::CursorIcon::Grab);
                                let label_text = match &current_blocks[idx] {
                                    Statement::Move(_) => "🏃 Move",
                                    Statement::Turn(_) => "🔄 Turn",
                                    Statement::Print(_) => "💬 Print",
                                    Statement::Sleep(_) => "💤 Sleep",
                                    Statement::If(_, _) => "❓ If",
                                    Statement::Loop(_, _) => "🔁 Loop",
                                    Statement::While(_, _) => "🔄 While",
                                    _ => "📄 Statement",
                                };
                                ui.label(egui::RichText::new(label_text).strong());
                            });
                        },
                    );

                    ui.separator();

                    match &mut current_blocks[idx] {
                        Statement::Move(expr) => {
                            if let Expr::Direction(dir) = expr {
                                render_direction_combobox(
                                    ui,
                                    dir,
                                    &format!("move_{}", current_id_str),
                                );
                            }
                        }
                        Statement::Turn(expr) => {
                            if let Expr::Direction(dir) = expr {
                                render_direction_combobox(
                                    ui,
                                    dir,
                                    &format!("turn_{}", current_id_str),
                                );
                            }
                        }
                        Statement::Print(expr) => {
                            if let Expr::String(s) = expr {
                                ui.text_edit_singleline(s);
                            }
                        }
                        Statement::Sleep(expr) => {
                            if let Expr::Float(v) = expr {
                                ui.add(egui::DragValue::new(v).speed(0.1));
                                ui.label("sec");
                            }
                        }
                        _ => {}
                    }

                    if ui.small_button("❌").clicked() {
                        delete_target_idx = Some(idx);
                    }
                });

                match &mut current_blocks[idx] {
                    Statement::If(_cond, body) => {
                        egui::CollapsingHeader::new("if is_touched()")
                            .id_salt(&current_id_str)
                            .default_open(true)
                            .show(ui, |ui| {
                                render_block_list(ui, body, this_path.clone(), move_request);
                            });
                    }
                    Statement::Loop(expr, body) => {
                        egui::CollapsingHeader::new("loop")
                            .id_salt(&current_id_str)
                            .default_open(true)
                            .show(ui, |ui| {
                                if let Expr::Uint(v) = expr {
                                    ui.horizontal(|ui| {
                                        ui.label("Repeat count:");
                                        ui.add(egui::DragValue::new(v).range(1..=10));
                                    });
                                }
                                render_block_list(ui, body, this_path.clone(), move_request);
                            });
                    }
                    Statement::While(_cond, body) => {
                        egui::CollapsingHeader::new("while true")
                            .id_salt(&current_id_str)
                            .default_open(true)
                            .show(ui, |ui| {
                                render_block_list(ui, body, this_path.clone(), move_request);
                            });
                    }
                    _ => {}
                }
            });

        if let Some(payload) = dropped_payload {
            match payload.as_ref() {
                DraggedBlock::New(new_stmt) => {
                    current_blocks.insert(idx, new_stmt.clone());
                    return;
                }
                DraggedBlock::Move { path: src_path } => {
                    if !is_ancestor(src_path, &current_path) {
                        *move_request = Some(MoveRequest {
                            src_path: src_path.clone(),
                            target_path: current_path.clone(),
                            insert_idx: idx,
                        });
                    }
                }
            }
        }
        ui.add_space(2.0);
    }

    let (_, bottom_payload) = ui.dnd_drop_zone::<DraggedBlock, _>(egui::Frame::NONE, |ui| {
        let response =
            ui.allocate_response(egui::vec2(ui.available_width(), 24.0), egui::Sense::hover());

        let color = if current_blocks.is_empty() {
            egui::Color32::from_gray(100)
        } else {
            egui::Color32::from_gray(60)
        };

        ui.painter().rect_stroke(
            response.rect,
            2.0,
            egui::Stroke::new(1.0, color),
            egui::StrokeKind::Inside,
        );

        if current_blocks.is_empty() {
            ui.painter().text(
                response.rect.center(),
                egui::Align2::CENTER_CENTER,
                "Drop here",
                egui::FontId::proportional(11.0),
                egui::Color32::from_gray(140),
            );
        }
    });

    if let Some(payload) = bottom_payload {
        match payload.as_ref() {
            DraggedBlock::New(new_stmt) => {
                current_blocks.push(new_stmt.clone());
            }
            DraggedBlock::Move { path: src_path } => {
                if !is_ancestor(src_path, &current_path) {
                    *move_request = Some(MoveRequest {
                        src_path: src_path.clone(),
                        target_path: current_path.clone(),
                        insert_idx: current_blocks.len(),
                    });
                }
            }
        }
    }

    if let Some(idx) = delete_target_idx {
        current_blocks.remove(idx);
    }
}

pub fn render_direction_combobox(ui: &mut egui::Ui, dir: &mut Direction, id_salt: &str) {
    egui::ComboBox::from_id_salt(id_salt)
        .selected_text(format!("{:?}", dir))
        .show_ui(ui, |ui| {
            ui.selectable_value(dir, Direction::Forward, "Forward");
            ui.selectable_value(dir, Direction::Back, "Back");
            ui.selectable_value(dir, Direction::Left, "Left");
            ui.selectable_value(dir, Direction::Right, "Right");
            ui.selectable_value(dir, Direction::Up, "Up");
            ui.selectable_value(dir, Direction::Down, "Down");
        });
}
