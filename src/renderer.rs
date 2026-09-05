use crate::colorizer::*;
use crate::utils::get_stmt_info;
use bevy_egui::egui;
use keystone_lang::{Callee, Direction, Expr, Op, Statement};

use crate::{
    structs::{DraggedBlock, MoveRequest},
    utils::is_ancestor,
};

fn get_block_frame(color: egui::Color32) -> egui::Frame {
    egui::Frame::NONE
        .fill(color)
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(8, 6))
}

fn unselectable_label(ui: &mut egui::Ui, text: egui::RichText) -> egui::Response {
    ui.add(egui::Label::new(text).selectable(false))
}

pub fn palette_button(
    ui: &mut egui::Ui,
    label: &str,
    stmt: Statement,
    target_blocks: &mut Vec<Statement>,
) {
    let color = get_stmt_color(&stmt);
    let item_id = egui::Id::new(format!("palette_{:?}", stmt));

    let response = ui.dnd_drag_source(item_id, DraggedBlock::NewStatement(stmt.clone()), |ui| {
        get_block_frame(color).show(ui, |ui| {
            ui.horizontal(|ui| {
                unselectable_label(
                    ui,
                    egui::RichText::new("☰")
                        .color(colors::TEXT_WHITE)
                        .size(13.0),
                )
                .on_hover_cursor(egui::CursorIcon::Grab);

                unselectable_label(
                    ui,
                    egui::RichText::new(label)
                        .color(colors::TEXT_WHITE)
                        .strong(),
                );
            });
        });
    });

    if response.response.clicked() {
        target_blocks.push(stmt);
    }
}

pub fn block_list(
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

        let block_color = get_stmt_color(&current_blocks[idx]);

        let (_, dropped_payload) = ui.dnd_drop_zone::<DraggedBlock, _>(egui::Frame::NONE, |ui| {
            get_block_frame(block_color).show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        let (icon, name) = get_stmt_info(&current_blocks[idx]);

                        ui.dnd_drag_source(
                            block_id.with("drag_handle"),
                            DraggedBlock::MoveStatement {
                                path: this_path.clone(),
                            },
                            |ui| {
                                ui.horizontal(|ui| {
                                    unselectable_label(
                                        ui,
                                        egui::RichText::new("☰")
                                            .color(colors::TEXT_WHITE)
                                            .size(13.0),
                                    )
                                    .on_hover_cursor(egui::CursorIcon::Grab);

                                    unselectable_label(
                                        ui,
                                        egui::RichText::new(format!("{} {}", icon, name))
                                            .color(colors::TEXT_WHITE)
                                            .strong(),
                                    )
                                    .on_hover_cursor(egui::CursorIcon::Grab);
                                });
                            },
                        );

                        ui.add_space(2.0);

                        match &mut current_blocks[idx] {
                            Statement::Print(expr) => {
                                expr_slot(ui, expr, block_id.with("print_expr"));
                            }
                            Statement::Sleep(expr) => {
                                expr_slot(ui, expr, block_id.with("sleep_expr"));
                                unselectable_label(
                                    ui,
                                    egui::RichText::new("sec").color(colors::TEXT_WHITE),
                                );
                            }
                            Statement::Move(expr) => {
                                expr_slot(ui, expr, block_id.with("move_expr"));
                            }
                            Statement::Turn(expr) => {
                                expr_slot(ui, expr, block_id.with("turn_expr"));
                            }
                            Statement::Dig(expr) => {
                                expr_slot(ui, expr, block_id.with("dig_expr"));
                            }
                            Statement::Let(name, expr) => {
                                ui.add(
                                    egui::TextEdit::singleline(name)
                                        .desired_width(50.0)
                                        .margin(egui::Margin::symmetric(4, 1)),
                                );
                                unselectable_label(
                                    ui,
                                    egui::RichText::new("=").color(colors::TEXT_WHITE).strong(),
                                );
                                expr_slot(ui, expr, block_id.with("let_expr"));
                            }
                            Statement::Send(expr) => {
                                expr_slot(ui, expr, block_id.with("send_expr"));
                            }
                            Statement::Receive(expr) => {
                                expr_slot(ui, expr, block_id.with("receive_expr"));
                            }
                            _ => {}
                        }

                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("✖").color(egui::Color32::LIGHT_RED),
                                )
                                .frame(false),
                            )
                            .clicked()
                        {
                            delete_target_idx = Some(idx);
                        }
                    });

                    match &mut current_blocks[idx] {
                        Statement::If(cond, body) => {
                            ui.horizontal(|ui| {
                                unselectable_label(
                                    ui,
                                    egui::RichText::new("if").color(colors::TEXT_WHITE).strong(),
                                );
                                expr_slot(ui, cond, block_id.with("if_cond"));
                            });

                            egui::CollapsingHeader::new(
                                egui::RichText::new("body").color(colors::TEXT_WHITE),
                            )
                            .id_salt(&current_id_str)
                            .default_open(true)
                            .show(ui, |ui| {
                                block_list(ui, body, this_path.clone(), move_request);
                            });
                        }
                        Statement::Loop(expr, body) => {
                            ui.horizontal(|ui| {
                                unselectable_label(
                                    ui,
                                    egui::RichText::new("🔁 loop")
                                        .color(colors::TEXT_WHITE)
                                        .strong(),
                                );
                                expr_slot(ui, expr, block_id.with("loop_count"));
                                unselectable_label(
                                    ui,
                                    egui::RichText::new("times").color(colors::TEXT_WHITE),
                                );
                            });

                            egui::CollapsingHeader::new(
                                egui::RichText::new("body").color(colors::TEXT_WHITE),
                            )
                            .id_salt(&current_id_str)
                            .default_open(true)
                            .show(ui, |ui| {
                                block_list(ui, body, this_path.clone(), move_request);
                            });
                        }
                        Statement::While(cond, body) => {
                            ui.horizontal(|ui| {
                                unselectable_label(
                                    ui,
                                    egui::RichText::new("while")
                                        .color(colors::TEXT_WHITE)
                                        .strong(),
                                );
                                expr_slot(ui, cond, block_id.with("while_cond"));
                            });

                            egui::CollapsingHeader::new(
                                egui::RichText::new("body").color(colors::TEXT_WHITE),
                            )
                            .id_salt(&current_id_str)
                            .default_open(true)
                            .show(ui, |ui| {
                                block_list(ui, body, this_path.clone(), move_request);
                            });
                        }
                        _ => {}
                    }
                });
            });
        });

        if let Some(payload) = dropped_payload {
            match payload.as_ref() {
                DraggedBlock::NewStatement(new_stmt) => {
                    current_blocks.insert(idx, new_stmt.clone());
                    return;
                }
                DraggedBlock::MoveStatement { path: src_path } => {
                    if !is_ancestor(src_path, &current_path) {
                        *move_request = Some(MoveRequest {
                            src_path: src_path.clone(),
                            target_path: current_path.clone(),
                            insert_idx: idx,
                        });
                    }
                }
                _ => {}
            }
        }
        ui.add_space(4.0);
    }

    let (_, bottom_payload) = ui.dnd_drop_zone::<DraggedBlock, _>(egui::Frame::NONE, |ui| {
        let response =
            ui.allocate_response(egui::vec2(ui.available_width(), 24.0), egui::Sense::hover());

        let color = if current_blocks.is_empty() {
            egui::Color32::from_gray(120)
        } else {
            egui::Color32::from_gray(70)
        };

        ui.painter().rect_stroke(
            response.rect,
            4.0,
            egui::Stroke::new(1.5, color),
            egui::StrokeKind::Inside,
        );

        if current_blocks.is_empty() {
            ui.painter().text(
                response.rect.center(),
                egui::Align2::CENTER_CENTER,
                "ここにブロックをドロップ",
                egui::FontId::proportional(11.0),
                egui::Color32::from_gray(160),
            );
        }
    });

    if let Some(payload) = bottom_payload {
        match payload.as_ref() {
            DraggedBlock::NewStatement(new_stmt) => {
                current_blocks.push(new_stmt.clone());
            }
            DraggedBlock::MoveStatement { path: src_path } => {
                if !is_ancestor(src_path, &current_path) {
                    *move_request = Some(MoveRequest {
                        src_path: src_path.clone(),
                        target_path: current_path.clone(),
                        insert_idx: current_blocks.len(),
                    });
                }
            }
            _ => {}
        }
    }

    if let Some(idx) = delete_target_idx {
        current_blocks.remove(idx);
    }
}

pub fn direction_combobox(ui: &mut egui::Ui, dir: &mut Direction, id_salt: &str) {
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

pub fn expr_slot(ui: &mut egui::Ui, expr: &mut Expr, slot_id: egui::Id) {
    let frame = egui::Frame::NONE
        .inner_margin(egui::Margin::symmetric(5, 2))
        .corner_radius(egui::CornerRadius::same(8))
        .fill(colors::SLOT_BG)
        .stroke(egui::Stroke::new(1.0, colors::SLOT_BORDER));

    let (_, dropped_payload) = ui.dnd_drop_zone::<DraggedBlock, _>(frame, |ui| {
        ui.horizontal(|ui| match expr {
            Expr::Var(name) => {
                ui.add(
                    egui::TextEdit::singleline(name)
                        .desired_width(50.0)
                        .margin(egui::Margin::symmetric(4, 1)),
                );
            }

            Expr::Uint(v) => {
                let mut val = *v as i64;
                if ui.add(egui::DragValue::new(&mut val).speed(1)).changed() {
                    *v = val.max(0) as u32;
                }
            }

            Expr::Binary { op, lhs, rhs } => {
                expr_slot(ui, lhs, slot_id.with("lhs"));

                egui::ComboBox::from_id_salt(slot_id.with("op"))
                    .selected_text(match op {
                        Op::Eq => "==",
                        Op::Neq => "!=",
                        Op::Lt => "<",
                        Op::Gt => ">",
                        Op::Le => "<=",
                        Op::Ge => ">=",
                        Op::Add => "+",
                        Op::Sub => "-",
                        Op::Mul => "*",
                        Op::Div => "/",
                        Op::And => "and",
                        Op::Or => "or",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(op, Op::Eq, "==");
                        ui.selectable_value(op, Op::Neq, "!=");
                        ui.selectable_value(op, Op::Lt, "<");
                        ui.selectable_value(op, Op::Gt, ">");
                        ui.selectable_value(op, Op::Le, "<=");
                        ui.selectable_value(op, Op::Ge, ">=");
                        ui.selectable_value(op, Op::Add, "+");
                        ui.selectable_value(op, Op::Sub, "-");
                        ui.selectable_value(op, Op::Mul, "*");
                        ui.selectable_value(op, Op::Div, "/");
                        ui.selectable_value(op, Op::And, "and");
                        ui.selectable_value(op, Op::Or, "or");
                    });

                expr_slot(ui, rhs, slot_id.with("rhs"));
            }

            Expr::Unary { op: _op, exp } => {
                unselectable_label(ui, egui::RichText::new("not").color(colors::TEXT_WHITE));
                expr_slot(ui, exp, slot_id.with("inner"));
            }

            Expr::Call { callee, args } => {
                let name = match callee {
                    Callee::IsTouched => "is_touched",
                    Callee::IsEmpty => "is_empty",
                    Callee::Rand => "rand",
                };
                unselectable_label(
                    ui,
                    egui::RichText::new(format!("{}()", name)).color(colors::TEXT_WHITE),
                );

                for (idx, arg) in args.iter_mut().enumerate() {
                    expr_slot(ui, arg, slot_id.with(idx));
                }
            }

            Expr::Boolean(b) => {
                ui.checkbox(b, if *b { "true" } else { "false" });
            }

            Expr::Direction(dir) => {
                direction_combobox(ui, dir, &slot_id.with("dir").value().to_string());
            }

            Expr::String(s) => {
                ui.add(
                    egui::TextEdit::singleline(s)
                        .desired_width(80.0)
                        .margin(egui::Margin::symmetric(4, 1)),
                );
            }

            Expr::Float(v) => {
                ui.add(egui::DragValue::new(v).speed(0.1));
            }
        });
    });

    if let Some(payload) = dropped_payload {
        if let DraggedBlock::NewExpr(new_expr) = payload.as_ref() {
            *expr = new_expr.clone();
        }
    }
}

pub fn expr_palette_button(ui: &mut egui::Ui, label: &str, expr: Expr) {
    ui.dnd_drag_source(
        egui::Id::new(label),
        DraggedBlock::NewExpr(expr.clone()),
        |ui| {
            ui.add(
                egui::Button::new(
                    egui::RichText::new(label)
                        .color(colors::TEXT_WHITE)
                        .strong(),
                )
                .fill(colors::OPERATORS)
                .corner_radius(egui::CornerRadius::same(10)),
            );
        },
    );
}
