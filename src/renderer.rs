use crate::code::{expr_value_problem, var_name_problem};
use crate::utils::{disallowed_in_stmt, get_stmt_info};
use crate::{colorizer::*, utils::is_type_compatible};
use bevy_egui::egui;
use std::collections::HashSet;

use keystone_lang::{Callee, Direction, Expr, Op, Statement, Type, TypeContext, UnaryOp};

use crate::{
    structs::{DraggedBlock, DraggedExpr, DropTarget},
    utils::is_ancestor,
};

const BLOCK_GAP: f32 = 4.0;
const NESTED_DROP_ZONE_MIN_WIDTH: f32 = 100.0;
const NESTED_DROP_ZONE_HEIGHT: f32 = 40.0;
const DROP_MARKER_HEIGHT: f32 = 6.0;
const DROP_MARKER_MIN_WIDTH: f32 = 80.0;
const DROP_MARKER_FILL: egui::Color32 = egui::Color32::from_rgb(255, 214, 0);
const DROP_MARKER_STROKE: egui::Color32 = egui::Color32::BLACK;

fn get_block_frame(color: egui::Color32) -> egui::Frame {
    egui::Frame::NONE
        .fill(color)
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(8, 6))
}

// If / Loop / While の中身を入れる「口」の部分(Scratch の C 型ブロック)
fn get_mouth_frame() -> egui::Frame {
    egui::Frame::NONE
        .fill(egui::Color32::from_black_alpha(100))
        .corner_radius(egui::CornerRadius::same(4))
        .inner_margin(egui::Margin::same(4))
        .outer_margin(egui::Margin {
            left: 6,
            right: 0,
            top: 4,
            bottom: 0,
        })
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_white_alpha(20)))
}

fn get_slot_frame() -> egui::Frame {
    egui::Frame::NONE
        .inner_margin(egui::Margin::symmetric(5, 2))
        .corner_radius(egui::CornerRadius::same(8))
        .fill(colors::SLOT_BG)
        .stroke(egui::Stroke::new(1.0, colors::SLOT_BORDER))
}

fn get_reporter_frame() -> egui::Frame {
    egui::Frame::NONE
        .fill(colors::OPERATORS)
        .corner_radius(egui::CornerRadius::same(12))
        .inner_margin(egui::Margin::symmetric(8, 4))
}

fn fit_text_width(ui: &egui::Ui, text: &str, min_width: f32) -> f32 {
    let font_id = egui::FontSelection::Default.resolve(ui.style());
    let width = ui.fonts_mut(|f| {
        f.layout_no_wrap(text.to_owned(), font_id, egui::Color32::WHITE)
            .size()
            .x
    });
    width.max(min_width) + 8.0
}

fn fitted_text_edit(ui: &mut egui::Ui, text: &mut String) -> egui::Response {
    let width = fit_text_width(ui, text, 20.0);
    ui.add(
        egui::TextEdit::singleline(text)
            .text_color(colors::TEXT_WHITE)
            .frame(egui::Frame::NONE)
            .desired_width(width)
            .margin(egui::Margin::symmetric(4, 1)),
    )
}

fn mark_problem(ui: &egui::Ui, response: egui::Response, problem: Option<&str>) {
    if let Some(problem) = problem {
        ui.painter().rect_stroke(
            response.rect.expand(1.0),
            egui::CornerRadius::same(4),
            egui::Stroke::new(2.0, colors::ERROR),
            egui::StrokeKind::Outside,
        );
        response.on_hover_text(problem);
    }
}

fn unselectable_label(ui: &mut egui::Ui, text: egui::RichText) -> egui::Response {
    ui.add(egui::Label::new(text).selectable(false))
}

fn white_label(ui: &mut egui::Ui, text: &str) -> egui::Response {
    unselectable_label(ui, egui::RichText::new(text).color(colors::TEXT_WHITE))
}

pub fn apply_block_spacing(ui: &mut egui::Ui) {
    ui.spacing_mut().combo_width = 0.0;
}

fn stmt_body_mut(stmt: &mut Statement) -> Option<&mut Vec<Statement>> {
    match stmt {
        Statement::If(_, body) | Statement::Loop(_, body) | Statement::While(_, body) => Some(body),
        _ => None,
    }
}

fn draggable_block<P: std::any::Any + Send + Sync, R>(
    ui: &mut egui::Ui,
    id: egui::Id,
    payload: P,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    if ui.ctx().is_being_dragged(id) {
        egui::DragAndDrop::set_payload(ui.ctx(), payload);

        let layer_id = egui::LayerId::new(egui::Order::Tooltip, id);
        let inner = ui.scope_builder(egui::UiBuilder::new().layer_id(layer_id), add_contents);

        if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
            let delta = pointer_pos - inner.response.rect.center();
            ui.ctx().transform_layer_shapes(
                layer_id,
                egui::emath::TSTransform::from_translation(delta),
            );
        }

        inner
    } else {
        let inner = ui.scope_builder(
            egui::UiBuilder::new()
                .id(id)
                .sense(egui::Sense::click_and_drag()),
            add_contents,
        );
        let response = inner.response.on_hover_cursor(egui::CursorIcon::Grab);
        egui::InnerResponse::new(inner.inner, response)
    }
}

fn statement_block(
    ui: &mut egui::Ui,
    stmt: &mut Statement,
    block_id: egui::Id,
    payload: DraggedBlock,
    type_ctx: &TypeContext,
    body: impl FnOnce(&mut egui::Ui, &mut Vec<Statement>),
) -> (egui::Response, egui::Rect) {
    let color = get_stmt_color(stmt);

    let inner = draggable_block(ui, block_id.with("drag"), payload, |ui| {
        get_block_frame(color)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    let header_rect = ui
                        .horizontal(|ui| {
                            ui.set_min_height(ui.spacing().interact_size.y + 6.0);
                            render_statement_header(ui, stmt, block_id, type_ctx);
                        })
                        .response
                        .rect;

                    if let Some(children) = stmt_body_mut(stmt) {
                        get_mouth_frame().show(ui, |ui| body(ui, children));
                    }
                    header_rect
                })
                .inner
            })
            .inner
    });
    (inner.response, inner.inner)
}

pub fn palette_block(
    ui: &mut egui::Ui,
    stmt: &mut Statement,
    block_id: egui::Id,
    target_blocks: &mut Vec<Statement>,
    type_ctx: &TypeContext,
) -> f32 {
    apply_block_spacing(ui);
    let payload = DraggedBlock::NewStatement(stmt.clone());

    let (response, _) = statement_block(ui, stmt, block_id, payload, type_ctx, |ui, _| {
        ui.allocate_space(egui::vec2(40.0, 8.0));
    });

    if response.clicked() {
        target_blocks.push(stmt.clone());
    }

    ui.add_space(BLOCK_GAP);
    response.rect.width()
}

pub fn palette_expr_block(
    ui: &mut egui::Ui,
    caption: Option<&str>,
    expr: &mut Expr,
    block_id: egui::Id,
    type_ctx: &TypeContext,
) -> f32 {
    apply_block_spacing(ui);
    let payload = DraggedExpr(expr.clone());

    let response = draggable_block(ui, block_id.with("drag"), payload, |ui| {
        get_reporter_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.set_min_height(ui.spacing().interact_size.y + 6.0);
                if let Some(caption) = caption {
                    unselectable_label(
                        ui,
                        egui::RichText::new(caption)
                            .color(colors::TEXT_WHITE)
                            .strong(),
                    );
                }
                expr_contents(ui, expr, block_id, type_ctx);
            });
        });
    });
    ui.add_space(BLOCK_GAP);
    response.response.rect.width()
}

fn dragged_statement(ui: &egui::Ui) -> Option<(egui::Pos2, std::sync::Arc<DraggedBlock>)> {
    let payload = egui::DragAndDrop::payload::<DraggedBlock>(ui.ctx())?;
    let pos = ui.ctx().pointer_interact_pos()?;
    ui.clip_rect().contains(pos).then_some((pos, payload))
}

fn marker_rect(left: f32, y: f32, width: f32) -> egui::Rect {
    egui::Rect::from_min_size(
        egui::pos2(left, y - DROP_MARKER_HEIGHT / 2.0),
        egui::vec2(width.max(DROP_MARKER_MIN_WIDTH), DROP_MARKER_HEIGHT),
    )
}

fn make_drop_target(
    payload: &DraggedBlock,
    list_path: &[usize],
    insert_idx: usize,
    marker: egui::Rect,
) -> DropTarget {
    let valid = match payload {
        DraggedBlock::MoveStatement { path } => {
            path.as_slice() != list_path && !is_ancestor(path, list_path)
        }
        DraggedBlock::NewStatement(_) => true,
    };
    DropTarget {
        list_path: list_path.to_vec(),
        insert_idx,
        marker: valid.then_some(marker),
    }
}

pub fn paint_drop_marker(ui: &egui::Ui, marker: egui::Rect) {
    ui.painter().rect(
        marker,
        egui::CornerRadius::same(3),
        DROP_MARKER_FILL,
        egui::Stroke::new(1.5, DROP_MARKER_STROKE),
        egui::StrokeKind::Outside,
    );
}

pub fn block_list(
    ui: &mut egui::Ui,
    current_blocks: &mut [Statement],
    current_path: Vec<usize>,
    drop_target: &mut Option<DropTarget>,
    type_ctx: &TypeContext,
    allowed: Option<&HashSet<String>>,
) {
    ui.spacing_mut().item_spacing.y = 0.0;
    apply_block_spacing(ui);
    let dragged = dragged_statement(ui);
    let list_right = ui.max_rect().right();

    for (idx, stmt) in current_blocks.iter_mut().enumerate() {
        let mut this_path = current_path.clone();
        this_path.push(idx);

        let current_id_str = this_path
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join("_");
        let block_id = egui::Id::new(&current_id_str);
        let has_body = stmt_body_mut(stmt).is_some();

        let (response, header_rect) = statement_block(
            ui,
            stmt,
            block_id,
            DraggedBlock::MoveStatement {
                path: this_path.clone(),
            },
            type_ctx,
            |ui, children| {
                block_list(
                    ui,
                    children,
                    this_path.clone(),
                    drop_target,
                    type_ctx,
                    allowed,
                );
            },
        );
        let rect = response.rect;
        ui.add_space(BLOCK_GAP);

        mark_disallowed(ui, stmt, header_rect, block_id, allowed);

        if drop_target.is_none()
            && let Some((pos, payload)) = &dragged
        {
            let hit_rect = egui::Rect::from_min_max(
                rect.min,
                egui::pos2(list_right.max(rect.right()), rect.bottom() + BLOCK_GAP),
            );
            if hit_rect.contains(*pos) {
                let split_y = if has_body {
                    header_rect.center().y
                } else {
                    rect.center().y
                };
                *drop_target = Some(if pos.y < split_y {
                    let y = rect.top() - BLOCK_GAP / 2.0;
                    make_drop_target(
                        payload,
                        &current_path,
                        idx,
                        marker_rect(rect.left(), y, rect.width()),
                    )
                } else if has_body && pos.y < header_rect.bottom() + BLOCK_GAP {
                    let left = header_rect.left() + 8.0;
                    let y = header_rect.bottom() + BLOCK_GAP;
                    make_drop_target(
                        payload,
                        &this_path,
                        0,
                        marker_rect(left, y, DROP_MARKER_MIN_WIDTH),
                    )
                } else {
                    let y = rect.bottom() + BLOCK_GAP / 2.0;
                    make_drop_target(
                        payload,
                        &current_path,
                        idx + 1,
                        marker_rect(rect.left(), y, rect.width()),
                    )
                });
            }
        }
    }

    render_bottom_drop_zone(ui, current_blocks, &current_path, drop_target, dragged);
}

fn mark_disallowed(
    ui: &egui::Ui,
    stmt: &Statement,
    header_rect: egui::Rect,
    block_id: egui::Id,
    allowed: Option<&HashSet<String>>,
) {
    let commands = disallowed_in_stmt(stmt, allowed);
    if commands.is_empty() {
        return;
    }
    let rect = header_rect.expand(4.0);
    ui.painter().rect_stroke(
        rect,
        egui::CornerRadius::same(6),
        egui::Stroke::new(2.0, colors::ERROR),
        egui::StrokeKind::Outside,
    );
    ui.interact(rect, block_id.with("disallowed"), egui::Sense::hover())
        .on_hover_text(format!("This stone can't use: {}", commands.join(", ")));
}

fn render_statement_header(
    ui: &mut egui::Ui,
    stmt: &mut Statement,
    block_id: egui::Id,
    type_ctx: &TypeContext,
) {
    let (icon, name) = get_stmt_info(stmt);
    unselectable_label(
        ui,
        egui::RichText::new(format!("{} {}", icon, name))
            .color(colors::TEXT_WHITE)
            .strong(),
    );

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
            white_label(ui, "sec");
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
        Statement::Place(expr) => {
            expr_slot(
                ui,
                expr,
                block_id.with("place_expr"),
                Some(Type::Direction),
                type_ctx,
            );
        }
        Statement::Let(name, expr) => {
            get_slot_frame().show(ui, |ui| {
                let response = fitted_text_edit(ui, name);
                mark_problem(ui, response, var_name_problem(name));
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
        Statement::If(cond, _) => {
            expr_slot(
                ui,
                cond,
                block_id.with("if_cond"),
                Some(Type::Boolean),
                type_ctx,
            );
        }
        Statement::Loop(expr, _) => {
            expr_slot(
                ui,
                expr,
                block_id.with("loop_count"),
                Some(Type::Uint),
                type_ctx,
            );
            white_label(ui, "times");
        }
        Statement::While(cond, _) => {
            expr_slot(
                ui,
                cond,
                block_id.with("while_cond"),
                Some(Type::Boolean),
                type_ctx,
            );
        }
    }
}

fn render_bottom_drop_zone(
    ui: &mut egui::Ui,
    current_blocks: &[Statement],
    current_path: &[usize],
    drop_target: &mut Option<DropTarget>,
    dragged: Option<(egui::Pos2, std::sync::Arc<DraggedBlock>)>,
) {
    let size = if current_path.is_empty() {
        egui::vec2(ui.available_width(), ui.available_height().max(140.0))
    } else {
        egui::vec2(
            ui.min_rect().width().max(NESTED_DROP_ZONE_MIN_WIDTH),
            NESTED_DROP_ZONE_HEIGHT,
        )
    };
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());

    if current_path.is_empty() && current_blocks.is_empty() {
        ui.painter().text(
            rect.left_top() + egui::vec2(4.0, 8.0),
            egui::Align2::LEFT_TOP,
            "Drag blocks here",
            egui::TextStyle::Body.resolve(ui.style()),
            ui.visuals().weak_text_color(),
        );
    }

    if drop_target.is_none()
        && let Some((pos, payload)) = dragged
        && rect.contains(pos)
    {
        let width = if current_path.is_empty() {
            DROP_MARKER_MIN_WIDTH * 1.5
        } else {
            rect.width()
        };
        *drop_target = Some(make_drop_target(
            &payload,
            current_path,
            current_blocks.len(),
            marker_rect(rect.left(), rect.top() + DROP_MARKER_HEIGHT, width),
        ));
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
    if !egui::DragAndDrop::has_payload_of_type::<DraggedExpr>(ui.ctx()) {
        get_slot_frame().show(ui, |ui| {
            ui.horizontal(|ui| expr_contents(ui, expr, slot_id, type_ctx));
        });
        return;
    }

    ui.scope(|ui| {
        ui.visuals_mut().widgets.active.bg_fill = colors::SLOT_BG;
        ui.visuals_mut().widgets.inactive.bg_fill = colors::SLOT_BG;
        ui.visuals_mut().widgets.inactive.bg_stroke = egui::Stroke::new(1.0, colors::SLOT_BORDER);
        ui.visuals_mut().widgets.active.bg_stroke = egui::Stroke::new(2.0, DROP_MARKER_FILL);

        let (_, dropped_payload) = ui.dnd_drop_zone::<DraggedExpr, _>(get_slot_frame(), |ui| {
            ui.horizontal(|ui| expr_contents(ui, expr, slot_id, type_ctx));
        });

        if let Some(payload) = dropped_payload
            && is_type_compatible(expected_type, &payload.0, type_ctx)
        {
            *expr = payload.0.clone();
        }
    });
}

fn expr_contents(ui: &mut egui::Ui, expr: &mut Expr, slot_id: egui::Id, type_ctx: &TypeContext) {
    match expr {
        Expr::Var(name) => {
            let response = fitted_text_edit(ui, name);
            mark_problem(ui, response, expr_value_problem(expr));
        }

        Expr::Uint(v) => {
            let mut val = *v as i64;
            if ui.add(egui::DragValue::new(&mut val).speed(1)).changed() {
                *v = val.max(0) as u32;
            }
        }

        Expr::Float(v) => {
            let response = ui.add(egui::DragValue::new(v).speed(0.1).range(0.0..=f64::MAX));
            mark_problem(ui, response, expr_value_problem(expr));
        }

        Expr::String(s) => {
            let response = fitted_text_edit(ui, s);
            mark_problem(ui, response, expr_value_problem(expr));
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
            white_label(ui, label_text);
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

            white_label(ui, name);
            white_label(ui, "(");

            let arg_expected_type = match callee {
                Callee::IsTouched => None,
                Callee::IsEmpty => Some(Type::Direction),
                Callee::Rand => Some(Type::Uint),
            };

            for (idx, arg) in args.iter_mut().enumerate() {
                if idx > 0 {
                    white_label(ui, ", ");
                }

                expr_slot(
                    ui,
                    arg,
                    slot_id.with(idx),
                    arg_expected_type.clone(),
                    type_ctx,
                );
            }

            white_label(ui, ")");
        }
    }
}
