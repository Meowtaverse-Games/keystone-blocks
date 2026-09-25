use crate::utils::get_stmt_info;
use crate::{colorizer::*, utils::is_type_compatible};
use bevy_egui::egui;

use keystone_lang::{Callee, Direction, Expr, Op, Statement, Type, TypeContext, UnaryOp};

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
    type_ctx: &TypeContext,
) {
    let mut delete_target_idx = None;
    ui.spacing_mut().item_spacing.y = 0.0;

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

        ui.scope(|ui| {
            ui.visuals_mut().widgets.active.bg_fill = egui::Color32::TRANSPARENT;
            ui.visuals_mut().widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
            ui.visuals_mut().widgets.active.bg_stroke = egui::Stroke::new(1.0, colors::TEXT_WHITE);

            let (_, dropped_payload) =
                ui.dnd_drop_zone::<DraggedBlock, _>(egui::Frame::NONE, |ui| {
                    get_block_frame(block_color).show(ui, |ui| {
                        ui.vertical(|ui| {
                            render_statement_header(
                                ui,
                                &mut current_blocks[idx],
                                &this_path,
                                block_id,
                                &mut delete_target_idx,
                                idx,
                                type_ctx,
                            );

                            render_statement_body(
                                ui,
                                &mut current_blocks[idx],
                                &this_path,
                                &current_id_str,
                                block_id,
                                move_request,
                                type_ctx,
                            );
                        });
                    });
                });

            if let Some(payload) = dropped_payload {
                handle_drop(
                    payload.as_ref(),
                    current_blocks,
                    &current_path,
                    idx,
                    move_request,
                );
            }

            ui.add_space(4.0);
        });
    }

    render_bottom_drop_zone(ui, current_blocks, &current_path, move_request);

    if let Some(idx) = delete_target_idx {
        current_blocks.remove(idx);
    }
}

fn render_statement_header(
    ui: &mut egui::Ui,
    stmt: &mut Statement,
    this_path: &[usize],
    block_id: egui::Id,
    delete_target_idx: &mut Option<usize>,
    idx: usize,
    type_ctx: &TypeContext,
) {
    ui.horizontal(|ui| {
        let (icon, name) = get_stmt_info(stmt);

        ui.dnd_drag_source(
            block_id.with("drag_handle"),
            DraggedBlock::MoveStatement {
                path: this_path.to_vec(),
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

        match stmt {
            Statement::Print(expr) => {
                expr_slot(ui, expr, block_id.with("print_expr"), None, type_ctx);
            }
            Statement::Sleep(expr) => {
                expr_slot(
                    ui,
                    expr,
                    block_id.with("sleep_expr"),
                    Some(Type::Float),
                    type_ctx,
                );
                unselectable_label(ui, egui::RichText::new("sec").color(colors::TEXT_WHITE));
            }
            Statement::Move(expr) => {
                expr_slot(
                    ui,
                    expr,
                    block_id.with("move_expr"),
                    Some(Type::Direction),
                    type_ctx,
                );
            }
            Statement::Turn(expr) => {
                expr_slot(
                    ui,
                    expr,
                    block_id.with("turn_expr"),
                    Some(Type::Direction),
                    type_ctx,
                );
            }
            Statement::Dig(expr) => {
                expr_slot(
                    ui,
                    expr,
                    block_id.with("dig_expr"),
                    Some(Type::Direction),
                    type_ctx,
                );
            }
            Statement::Let(name, expr) => {
                egui::Frame::NONE
                    .inner_margin(egui::Margin::symmetric(5, 2))
                    .corner_radius(egui::CornerRadius::same(8))
                    .fill(colors::SLOT_BG)
                    .stroke(egui::Stroke::new(1.0, colors::SLOT_BORDER))
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::singleline(name)
                                .frame(false)
                                .desired_width(50.0)
                                .margin(egui::Margin::symmetric(4, 1)),
                        );
                    });
                unselectable_label(
                    ui,
                    egui::RichText::new("=").color(colors::TEXT_WHITE).strong(),
                );
                expr_slot(ui, expr, block_id.with("let_expr"), None, type_ctx);
            }
            Statement::Send(expr) => {
                expr_slot(ui, expr, block_id.with("send_expr"), None, type_ctx);
            }
            Statement::Receive(expr) => {
                expr_slot(ui, expr, block_id.with("receive_expr"), None, type_ctx);
            }
            _ => {}
        }

        if ui
            .add(egui::Button::new(egui::RichText::new("✖").color(egui::Color32::RED)).frame(false))
            .clicked()
        {
            *delete_target_idx = Some(idx);
        }
    });
}

fn render_statement_body(
    ui: &mut egui::Ui,
    stmt: &mut Statement,
    this_path: &[usize],
    current_id_str: &str,
    block_id: egui::Id,
    move_request: &mut Option<MoveRequest>,
    type_ctx: &TypeContext,
) {
    match stmt {
        Statement::If(cond, body) => {
            ui.horizontal(|ui| {
                unselectable_label(
                    ui,
                    egui::RichText::new("if").color(colors::TEXT_WHITE).strong(),
                );
                expr_slot(
                    ui,
                    cond,
                    block_id.with("if_cond"),
                    Some(Type::Boolean),
                    type_ctx,
                );
            });
            render_collapsing_body(
                ui,
                body,
                this_path.to_vec(),
                current_id_str,
                move_request,
                type_ctx,
            );
        }
        Statement::Loop(expr, body) => {
            ui.horizontal(|ui| {
                unselectable_label(
                    ui,
                    egui::RichText::new("🔁 loop")
                        .color(colors::TEXT_WHITE)
                        .strong(),
                );
                expr_slot(
                    ui,
                    expr,
                    block_id.with("loop_count"),
                    Some(Type::Uint),
                    type_ctx,
                );
                unselectable_label(ui, egui::RichText::new("times").color(colors::TEXT_WHITE));
            });
            render_collapsing_body(
                ui,
                body,
                this_path.to_vec(),
                current_id_str,
                move_request,
                type_ctx,
            );
        }
        Statement::While(cond, body) => {
            ui.horizontal(|ui| {
                unselectable_label(
                    ui,
                    egui::RichText::new("while")
                        .color(colors::TEXT_WHITE)
                        .strong(),
                );
                expr_slot(
                    ui,
                    cond,
                    block_id.with("while_cond"),
                    Some(Type::Boolean),
                    type_ctx,
                );
            });
            render_collapsing_body(
                ui,
                body,
                this_path.to_vec(),
                current_id_str,
                move_request,
                type_ctx,
            );
        }
        _ => {}
    }
}

fn render_collapsing_body(
    ui: &mut egui::Ui,
    body: &mut Vec<Statement>,
    this_path: Vec<usize>,
    current_id_str: &str,
    move_request: &mut Option<MoveRequest>,
    type_ctx: &TypeContext,
) {
    egui::CollapsingHeader::new(egui::RichText::new("body").color(colors::TEXT_WHITE))
        .id_salt(current_id_str)
        .default_open(true)
        .show(ui, |ui| {
            egui::Frame::new()
                .fill(egui::Color32::from_black_alpha(100))
                .corner_radius(egui::CornerRadius::same(6))
                .inner_margin(egui::Margin::same(8))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_white_alpha(20)))
                .show(ui, |ui| {
                    block_list(ui, body, this_path, move_request, type_ctx);
                });
        });
}

fn render_bottom_drop_zone(
    ui: &mut egui::Ui,
    current_blocks: &mut Vec<Statement>,
    current_path: &[usize],
    move_request: &mut Option<MoveRequest>,
) {
    ui.scope(|ui| {
        ui.visuals_mut().widgets.active.bg_fill = egui::Color32::from_white_alpha(50);
        ui.visuals_mut().widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
        ui.visuals_mut().widgets.active.bg_stroke = egui::Stroke::NONE;

        let height = if current_path.is_empty() {
            ui.available_height().max(140.0)
        } else {
            40.0
        };

        let (_, bottom_payload) = ui.dnd_drop_zone::<DraggedBlock, _>(
            egui::Frame::NONE.fill(egui::Color32::TRANSPARENT),
            |ui| {
                let available_size = egui::vec2(ui.available_width(), height);
                ui.allocate_response(available_size, egui::Sense::hover());
            },
        );

        if let Some(payload) = bottom_payload {
            handle_drop(
                payload.as_ref(),
                current_blocks,
                current_path,
                current_blocks.len(),
                move_request,
            );
        }
    });
}

fn handle_drop(
    payload: &DraggedBlock,
    current_blocks: &mut Vec<Statement>,
    current_path: &[usize],
    insert_idx: usize,
    move_request: &mut Option<MoveRequest>,
) {
    match payload {
        DraggedBlock::NewStatement(new_stmt) => {
            if insert_idx <= current_blocks.len() {
                current_blocks.insert(insert_idx, new_stmt.clone());
            }
        }
        DraggedBlock::MoveStatement { path: src_path } => {
            if !is_ancestor(src_path, current_path) {
                *move_request = Some(MoveRequest {
                    src_path: src_path.clone(),
                    target_path: current_path.to_vec(),
                    insert_idx,
                });
            }
        }
        _ => {}
    }
}

pub fn direction_combobox(ui: &mut egui::Ui, dir: &mut Direction, id_salt: &str) {
    ui.scope(|ui| {
        ui.visuals_mut().widgets.hovered.bg_stroke = egui::Stroke::NONE;
        ui.visuals_mut().widgets.active.bg_stroke = egui::Stroke::NONE;

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
    });
}

pub fn binary_combobox(ui: &mut egui::Ui, op: &mut Op, id_salt: &str) {
    ui.scope(|ui| {
        ui.visuals_mut().widgets.hovered.bg_stroke = egui::Stroke::NONE;
        ui.visuals_mut().widgets.active.bg_stroke = egui::Stroke::NONE;

        let available_ops: &[(Op, &str)] = match op {
            Op::Eq | Op::Neq | Op::Lt | Op::Gt | Op::Le | Op::Ge => &[
                (Op::Eq, "=="),
                (Op::Neq, "!="),
                (Op::Lt, "<"),
                (Op::Gt, ">"),
                (Op::Le, "<="),
                (Op::Ge, ">="),
            ],
            Op::Add | Op::Sub | Op::Mul | Op::Div => &[
                (Op::Add, "+"),
                (Op::Sub, "-"),
                (Op::Mul, "*"),
                (Op::Div, "/"),
            ],
            Op::And | Op::Or => &[(Op::And, "and"), (Op::Or, "or")],
        };

        let current_text = available_ops
            .iter()
            .find(|(candidate, _)| candidate == op)
            .map(|(_, text)| *text)
            .unwrap_or("?");

        egui::ComboBox::from_id_salt(id_salt)
            .selected_text(current_text)
            .show_ui(ui, |ui| {
                for (candidate_op, label) in available_ops {
                    ui.selectable_value(op, candidate_op.clone(), *label);
                }
            });
    });
}

pub fn expr_slot(
    ui: &mut egui::Ui,
    expr: &mut Expr,
    slot_id: egui::Id,
    expected_type: Option<Type>,
    type_ctx: &TypeContext,
) {
    let frame = egui::Frame::NONE
        .inner_margin(egui::Margin::symmetric(5, 2))
        .corner_radius(egui::CornerRadius::same(8))
        .fill(colors::SLOT_BG)
        .stroke(egui::Stroke::new(1.0, colors::SLOT_BORDER));

    ui.scope(|ui| {
        ui.visuals_mut().widgets.active.bg_fill = colors::SLOT_BG;
        ui.visuals_mut().widgets.inactive.bg_fill = colors::SLOT_BG;
        ui.visuals_mut().widgets.active.bg_stroke = egui::Stroke::new(1.0, colors::TEXT_WHITE);

        let (_, dropped_payload) = ui.dnd_drop_zone::<DraggedBlock, _>(frame, |ui| {
            ui.horizontal(|ui| match expr {
                Expr::Var(name) => {
                    ui.add(
                        egui::TextEdit::singleline(name)
                            .frame(false)
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

                Expr::Float(v) => {
                    ui.add(egui::DragValue::new(v).speed(0.1));
                }

                Expr::String(s) => {
                    ui.add(
                        egui::TextEdit::singleline(s)
                            .frame(false)
                            .desired_width(80.0)
                            .margin(egui::Margin::symmetric(4, 1)),
                    );
                }

                Expr::Boolean(b) => {
                    ui.checkbox(b, if *b { "true" } else { "false" });
                }

                Expr::Direction(dir) => {
                    direction_combobox(ui, dir, &slot_id.with("dir").value().to_string());
                }

                Expr::Binary { op, lhs, rhs } => {
                    let child_expected_type = match op {
                        Op::Add | Op::Sub | Op::Mul | Op::Div => Some(Type::Uint),
                        Op::Eq | Op::Neq | Op::Lt | Op::Gt | Op::Le | Op::Ge => Some(Type::Uint),
                        Op::And | Op::Or => Some(Type::Boolean),
                    };

                    expr_slot(
                        ui,
                        lhs,
                        slot_id.with("lhs"),
                        child_expected_type.clone(),
                        type_ctx,
                    );

                    binary_combobox(ui, op, &slot_id.with("Op").value().to_string());

                    expr_slot(ui, rhs, slot_id.with("rhs"), child_expected_type, type_ctx);
                }

                Expr::Unary { op, exp } => {
                    let label_text = match op {
                        UnaryOp::Not => "not",
                    };
                    unselectable_label(
                        ui,
                        egui::RichText::new(label_text).color(colors::TEXT_WHITE),
                    );
                    expr_slot(
                        ui,
                        exp,
                        slot_id.with("inner"),
                        Some(Type::Boolean),
                        type_ctx,
                    );
                }

                Expr::Call { callee, args } => {
                    let name = match callee {
                        Callee::IsTouched => "is_touched",
                        Callee::IsEmpty => "is_empty",
                        Callee::Rand => "rand",
                    };

                    unselectable_label(ui, egui::RichText::new(name).color(colors::TEXT_WHITE));

                    unselectable_label(ui, egui::RichText::new("(").color(colors::TEXT_WHITE));

                    let arg_expected_type = match callee {
                        Callee::IsTouched => None,
                        Callee::IsEmpty => Some(Type::Direction),
                        Callee::Rand => Some(Type::Uint),
                    };

                    for (idx, arg) in args.iter_mut().enumerate() {
                        if idx > 0 {
                            unselectable_label(
                                ui,
                                egui::RichText::new(", ").color(colors::TEXT_WHITE),
                            );
                        }

                        expr_slot(
                            ui,
                            arg,
                            slot_id.with(idx),
                            arg_expected_type.clone(),
                            type_ctx,
                        );
                    }

                    unselectable_label(ui, egui::RichText::new(")").color(colors::TEXT_WHITE));
                }
            });
        });

        if let Some(payload) = dropped_payload {
            if let DraggedBlock::NewExpr(new_expr) = payload.as_ref() {
                if is_type_compatible(expected_type, new_expr, type_ctx) {
                    *expr = new_expr.clone();
                }
            }
        }
    });
}

pub fn expr_palette_button(ui: &mut egui::Ui, label: &str, expr: Expr) {
    let item_id = egui::Id::new(format!("expr_palette_{}", label));

    ui.dnd_drag_source(item_id, DraggedBlock::NewExpr(expr.clone()), |ui| {
        egui::Frame::NONE
            .fill(colors::OPERATORS)
            .corner_radius(egui::CornerRadius::same(12))
            .inner_margin(egui::Margin::symmetric(8, 4))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    unselectable_label(
                        ui,
                        egui::RichText::new("")
                            .color(colors::TEXT_WHITE.linear_multiply(0.7))
                            .size(11.0),
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
}
