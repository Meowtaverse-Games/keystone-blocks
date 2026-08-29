use bevy_egui::egui;
use keystone_lang::{Callee, Direction, Expr, Op, Statement};

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
    let response = ui.dnd_drag_source(item_id, DraggedBlock::NewStatement(stmt.clone()), |ui| {
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
                        DraggedBlock::MoveStatement {
                            path: this_path.clone(),
                        },
                        |ui| {
                            ui.horizontal(|ui| {
                                ui.label("☰").on_hover_cursor(egui::CursorIcon::Grab);
                                let label_text = match &current_blocks[idx] {
                                    Statement::Move(_) => "🏃 Move",
                                    Statement::Turn(_) => "🔄 Turn",
                                    Statement::Dig(_) => "🔨 Dig",
                                    Statement::Print(_) => "💬 Print",
                                    Statement::Sleep(_) => "💤 Sleep",
                                    Statement::Let(_, _) => "📝 Let",
                                    Statement::Send(_) => "📡 Send",
                                    Statement::Receive(_) => "📥 Receive",
                                    Statement::If(_, _) => "❓ If",
                                    Statement::Loop(_, _) => "🔁 Loop",
                                    Statement::While(_, _) => "🔄 While",
                                };
                                ui.label(egui::RichText::new(label_text).strong());
                            });
                        },
                    );

                    ui.separator();

                    match &mut current_blocks[idx] {
                        Statement::Print(expr) => {
                            render_expr_slot(ui, expr, block_id.with("print_expr"));
                        }

                        Statement::Sleep(expr) => {
                            render_expr_slot(ui, expr, block_id.with("sleep_expr"));
                            ui.label("sec");
                        }

                        Statement::Move(expr) => {
                            render_expr_slot(ui, expr, block_id.with("move_expr"));
                        }

                        Statement::Turn(expr) => {
                            render_expr_slot(ui, expr, block_id.with("turn_expr"));
                        }

                        Statement::Dig(expr) => {
                            render_expr_slot(ui, expr, block_id.with("dig_expr"));
                        }

                        Statement::Let(name, expr) => {
                            ui.add(egui::TextEdit::singleline(name).desired_width(60.0));
                            ui.label("=");
                            render_expr_slot(ui, expr, block_id.with("dig_expr"));
                        }

                        Statement::Send(expr) => {
                            render_expr_slot(ui, expr, block_id.with("send_expr"));
                        }

                        Statement::Receive(expr) => {
                            render_expr_slot(ui, expr, block_id.with("receive_expr"));
                        }
                        _ => {}
                    }

                    if ui.small_button("❌").clicked() {
                        delete_target_idx = Some(idx);
                    }
                });

                match &mut current_blocks[idx] {
                    Statement::If(cond, body) => {
                        ui.horizontal(|ui| {
                            ui.label("if");
                            render_expr_slot(ui, cond, block_id.with("if_cond"));
                        });

                        egui::CollapsingHeader::new("body")
                            .id_salt(&current_id_str)
                            .default_open(true)
                            .show(ui, |ui| {
                                render_block_list(ui, body, this_path.clone(), move_request);
                            });
                    }
                    Statement::Loop(expr, body) => {
                        ui.horizontal(|ui| {
                            ui.label("🔁 loop");
                            render_expr_slot(ui, expr, block_id.with("loop_count"));
                            ui.label("times");
                        });

                        egui::CollapsingHeader::new("body")
                            .id_salt(&current_id_str)
                            .default_open(true)
                            .show(ui, |ui| {
                                render_block_list(ui, body, this_path.clone(), move_request);
                            });
                    }
                    Statement::While(cond, body) => {
                        ui.horizontal(|ui| {
                            ui.label("while");
                            render_expr_slot(ui, cond, block_id.with("while_cond"));
                        });

                        egui::CollapsingHeader::new("body")
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

pub fn render_expr_slot(ui: &mut egui::Ui, expr: &mut Expr, slot_id: egui::Id) {
    let frame = egui::Frame::group(ui.style())
        .inner_margin(4.0)
        .corner_radius(8.0)
        .fill(egui::Color32::from_rgb(45, 55, 70));

    let (_, dropped_payload) = ui.dnd_drop_zone::<DraggedBlock, _>(frame, |ui| {
        ui.horizontal(|ui| match expr {
            Expr::Var(name) => {
                ui.add(egui::TextEdit::singleline(name).desired_width(50.0));
            }

            Expr::Uint(v) => {
                let mut val = *v as i64;
                if ui.add(egui::DragValue::new(&mut val).speed(1)).changed() {
                    *v = val.max(0) as u32;
                }
            }

            Expr::Binary { op, lhs, rhs } => {
                render_expr_slot(ui, lhs, slot_id.with("lhs"));

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

                render_expr_slot(ui, rhs, slot_id.with("rhs"));
            }

            Expr::Unary { op: _op, exp } => {
                ui.label("not");
                render_expr_slot(ui, exp, slot_id.with("inner"));
            }

            Expr::Call { callee, args } => {
                let name = match callee {
                    Callee::IsTouched => "is_touched",
                    Callee::IsEmpty => "is_empty",
                    Callee::Rand => "rand",
                };
                ui.label(format!("{}()", name));

                for (idx, arg) in args.iter_mut().enumerate() {
                    render_expr_slot(ui, arg, slot_id.with(idx));
                }
            }

            Expr::Boolean(b) => {
                ui.checkbox(b, if *b { "true" } else { "false" });
            }

            Expr::Direction(dir) => {
                render_direction_combobox(ui, dir, &slot_id.with("dir").value().to_string());
            }

            Expr::String(s) => {
                ui.add(egui::TextEdit::singleline(s).desired_width(80.0));
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

pub fn render_expr_palette_button(ui: &mut egui::Ui, label: &str, expr: Expr) {
    ui.dnd_drag_source(
        egui::Id::new(label),
        DraggedBlock::NewExpr(expr.clone()),
        |ui| {
            ui.add(
                egui::Button::new(egui::RichText::new(label).color(egui::Color32::LIGHT_GREEN))
                    .fill(egui::Color32::from_rgb(40, 60, 50)),
            );
        },
    );
}
