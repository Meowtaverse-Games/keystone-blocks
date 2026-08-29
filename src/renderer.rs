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
                                    Statement::Dig(_) => "🔨 Dig",
                                    Statement::Print(_) => "💬 Print",
                                    Statement::Sleep(_) => "💤 Sleep",
                                    Statement::Let(_, _) => "📝 Let",
                                    Statement::Send(_) => "📡 Send",
                                    Statement::Receive(_) => "📥 Receive",
                                    Statement::If(_, _) => "❓ If",
                                    Statement::Loop(_, _) => "🔁 Loop",
                                    Statement::While(_, _) => "🔄 While",
                                    // _ => "📄 Statement",
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
                        Statement::Dig(expr) => {
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
                        Statement::Let(name, expr) => {
                            ui.label("var:");
                            ui.add(egui::TextEdit::singleline(name).desired_width(60.0));
                            ui.label("=");

                            match expr {
                                Expr::Uint(v) => {
                                    let mut val = *v as i64;
                                    if ui.add(egui::DragValue::new(&mut val).speed(1)).changed() {
                                        *v = val.max(0) as u32;
                                    }
                                }
                                Expr::Float(v) => {
                                    ui.add(egui::DragValue::new(v).speed(0.1));
                                }
                                Expr::String(s) => {
                                    ui.add(egui::TextEdit::singleline(s).desired_width(80.0));
                                }
                                _ => {}
                            }
                        }
                        Statement::Send(expr) => {
                            ui.label("msg:");
                            match expr {
                                Expr::String(s) => {
                                    ui.add(egui::TextEdit::singleline(s).desired_width(100.0));
                                }
                                Expr::Uint(v) => {
                                    let mut val = *v as i64;
                                    if ui.add(egui::DragValue::new(&mut val).speed(1)).changed() {
                                        *v = val.max(0) as u32;
                                    }
                                }
                                _ => {}
                            }
                        }
                        Statement::Receive(expr) => {
                            ui.label("wait msg:");
                            match expr {
                                Expr::String(s) => {
                                    ui.add(egui::TextEdit::singleline(s).desired_width(100.0));
                                }
                                Expr::Uint(v) => {
                                    let mut val = *v as i64;
                                    if ui.add(egui::DragValue::new(&mut val).speed(1)).changed() {
                                        *v = val.max(0) as u32;
                                    }
                                }
                                _ => {}
                            }
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
                            render_condition_editor(ui, cond, &current_id_str);
                        });
                        egui::CollapsingHeader::new("body")
                            .id_salt(&current_id_str)
                            .default_open(true)
                            .show(ui, |ui| {
                                render_block_list(ui, body, this_path.clone(), move_request);
                            });
                    }
                    Statement::Loop(expr, body) => {
                        egui::CollapsingHeader::new("body")
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
                    Statement::While(cond, body) => {
                        ui.horizontal(|ui| {
                            ui.label("while");
                            render_condition_editor(ui, cond, &current_id_str);
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

fn render_condition_editor(ui: &mut egui::Ui, cond: &mut Expr, id_prefix: &str) {
    #[derive(Debug, PartialEq, Clone)]
    enum ConditionKind {
        IsTouched,
        IsEmpty,
        Comparison,
        Boolean,
    }

    let current_kind = match cond {
        Expr::Call {
            callee: Callee::IsTouched,
            ..
        } => ConditionKind::IsTouched,
        Expr::Call {
            callee: Callee::IsEmpty,
            ..
        } => ConditionKind::IsEmpty,
        Expr::Binary { .. } => ConditionKind::Comparison,
        Expr::Boolean(_) => ConditionKind::Boolean,
        _ => ConditionKind::IsTouched,
    };

    let mut selected_kind = current_kind.clone();

    egui::ComboBox::from_id_salt(format!("cond_type_{}", id_prefix))
        .selected_text(match selected_kind {
            ConditionKind::IsTouched => "is_touched()",
            ConditionKind::IsEmpty => "is_empty(dir)",
            ConditionKind::Comparison => "compare",
            ConditionKind::Boolean => "bool",
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut selected_kind, ConditionKind::IsTouched, "is_touched()");
            ui.selectable_value(&mut selected_kind, ConditionKind::IsEmpty, "is_empty(dir)");
            ui.selectable_value(
                &mut selected_kind,
                ConditionKind::Comparison,
                "compare (==, <, >)",
            );
            ui.selectable_value(
                &mut selected_kind,
                ConditionKind::Boolean,
                "bool (true / false)",
            );
        });

    if format!("{:?}", selected_kind) != format!("{:?}", current_kind) {
        *cond = match selected_kind {
            ConditionKind::IsTouched => Expr::Call {
                callee: Callee::IsTouched,
                args: vec![],
            },
            ConditionKind::IsEmpty => Expr::Call {
                callee: Callee::IsEmpty,
                args: vec![Box::new(Expr::Direction(Direction::Forward))],
            },
            ConditionKind::Comparison => Expr::Binary {
                op: Op::Eq,
                lhs: Box::new(Expr::Var("x".to_string())),
                rhs: Box::new(Expr::Uint(0)),
            },
            ConditionKind::Boolean => Expr::Boolean(true),
        };
    }

    match cond {
        Expr::Call {
            callee: Callee::IsEmpty,
            args,
        } => {
            if args.is_empty() {
                args.push(Box::new(Expr::Direction(Direction::Forward)));
            }
            if let Expr::Direction(dir) = args[0].as_mut() {
                render_direction_combobox(ui, dir, &format!("is_empty_dir_{}", id_prefix));
            }
        }
        Expr::Binary { op, lhs, rhs } => {
            if let Expr::Var(var_name) = lhs.as_mut() {
                ui.add(egui::TextEdit::singleline(var_name).desired_width(50.0));
            }

            egui::ComboBox::from_id_salt(format!("bin_op_{}", id_prefix))
                .selected_text(match op {
                    Op::Eq => "==",
                    Op::Neq => "!=",
                    Op::Lt => "<",
                    Op::Gt => ">",
                    Op::Le => "<=",
                    Op::Ge => ">=",
                    _ => "==",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(op, Op::Eq, "==");
                    ui.selectable_value(op, Op::Neq, "!=");
                    ui.selectable_value(op, Op::Lt, "<");
                    ui.selectable_value(op, Op::Gt, ">");
                    ui.selectable_value(op, Op::Le, "<=");
                    ui.selectable_value(op, Op::Ge, ">=");
                });

            if let Expr::Uint(v) = rhs.as_mut() {
                let mut val = *v as i64;
                if ui.add(egui::DragValue::new(&mut val).speed(1)).changed() {
                    *v = val.max(0) as u32;
                }
            }
        }
        Expr::Boolean(b) => {
            ui.checkbox(b, "");
        }
        _ => {}
    }
}
