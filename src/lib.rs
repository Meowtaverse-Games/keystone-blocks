mod code;
mod colorizer;
mod renderer;

mod structs;
mod sync;
mod utils;
use std::collections::HashSet;

use bevy::prelude::*;
use bevy_egui::egui::{self, Color32};
use code::*;
use colorizer::colors;
use keystone_lang::{Callee, Direction, Expr, Op, Statement, TypeContext, UnaryOp};
use renderer as render;
pub use structs::VplState;
use structs::*;
pub use sync::CodeSyncStatus;
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

    if available.x < available.y {
        show_stacked(ui, rect, state, &type_ctx);
    } else {
        show_side_by_side(ui, rect, state, &type_ctx);
    }
}

fn show_side_by_side(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    state: &mut VplState,
    type_ctx: &TypeContext,
) {
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
                        render_palette_panel(
                            ui,
                            &mut state.blocks,
                            state.allowed_commands.as_ref(),
                            type_ctx,
                            false,
                        )
                    },
                )
                .inner;
            ui.data_mut(|d| d.insert_temp(width_id, content_width));

            ui.separator();

            ui.vertical(|ui| {
                render_program_panel(ui, &mut state.blocks, type_ctx);
            });
        },
    );
}

fn show_stacked(ui: &mut egui::Ui, rect: egui::Rect, state: &mut VplState, type_ctx: &TypeContext) {
    let ratio_id = ui.make_persistent_id("vpl_stacked_palette_ratio");
    let ratio = ui
        .data(|d| d.get_temp::<f32>(ratio_id))
        .unwrap_or(STACKED_PALETTE_HEIGHT_RATIO);
    let max_palette_height =
        (rect.height() - STACKED_MIN_HEIGHT - SPLITTER_HEIGHT).max(STACKED_MIN_HEIGHT);
    let palette_height = (rect.height() * ratio).clamp(STACKED_MIN_HEIGHT, max_palette_height);

    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
        |ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(rect.width(), palette_height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_width(rect.width());
                    ui.set_height(palette_height);
                    render_palette_panel(
                        ui,
                        &mut state.blocks,
                        state.allowed_commands.as_ref(),
                        type_ctx,
                        true,
                    );
                },
            );

            let splitter = horizontal_splitter(ui, rect.width());
            if splitter.dragged() {
                let new_height = palette_height + splitter.drag_delta().y;
                ui.data_mut(|d| d.insert_temp(ratio_id, new_height / rect.height()));
            }

            render_program_panel(ui, &mut state.blocks, type_ctx);
        },
    );
}

fn horizontal_splitter(ui: &mut egui::Ui, width: f32) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, SPLITTER_HEIGHT), egui::Sense::drag());
    let response = response.on_hover_cursor(egui::CursorIcon::ResizeVertical);

    let visuals = ui.style().interact(&response);
    let stroke = if response.hovered() || response.dragged() {
        visuals.fg_stroke
    } else {
        ui.visuals().widgets.noninteractive.bg_stroke
    };
    let painter = ui.painter();
    painter.hline(rect.x_range(), rect.center().y, stroke);

    let grip = egui::Rect::from_center_size(rect.center(), egui::vec2(36.0, 4.0));
    painter.rect_filled(grip, egui::CornerRadius::same(2), visuals.fg_stroke.color);

    response
}

const STACKED_PALETTE_HEIGHT_RATIO: f32 = 0.4;
const STACKED_MIN_HEIGHT: f32 = 80.0;
const SPLITTER_HEIGHT: f32 = 12.0;

pub fn show_vpl_for_code(
    ui: &mut egui::Ui,
    state: &mut VplState,
    source_id: usize,
    code: &mut String,
    allowed_commands: Option<&HashSet<String>>,
) -> bool {
    if state.allowed_commands.as_ref() != allowed_commands {
        state.allowed_commands = allowed_commands.cloned();
    }
    state.sync_from_code(source_id, code);

    if let CodeSyncStatus::SyntaxError(message) = state.sync_status() {
        show_syntax_error(ui, message);
        return false;
    }

    if let Some(message) = state.write_error() {
        show_write_error(ui, message);
    }

    let had_error = state.write_error().is_some();
    show_vpl_contents(ui, state);
    let written = state.write_back(code);
    if had_error != state.write_error().is_some() {
        ui.ctx().request_repaint();
    }
    written
}

fn show_write_error(ui: &mut egui::Ui, message: &str) {
    ui.label(
        egui::RichText::new(
            "⚠ Some blocks can't be turned into code, so your changes aren't saved yet. \
             Fix the fields outlined in red.",
        )
        .color(colors::ERROR)
        .strong(),
    )
    .on_hover_text(egui::RichText::new(message).monospace());
    ui.add_space(4.0);
}

fn show_syntax_error(ui: &mut egui::Ui, message: &str) {
    ui.vertical(|ui| {
        ui.label(
            egui::RichText::new("⚠ This code has a syntax error, so it can't be shown as blocks.")
                .strong(),
        );
        ui.label("Fix it in the text editor (or clear the code) to edit it with blocks.");
        ui.add_space(6.0);
        egui::ScrollArea::vertical()
            .id_salt("vpl_syntax_error_scroll")
            .show(ui, |ui| {
                ui.label(egui::RichText::new(message).monospace().weak());
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

#[derive(Clone)]
enum PaletteEntry {
    Section(&'static str),
    SubSection(&'static str),
    Stmt(Statement),
    Value(Option<&'static str>, Expr),
}

fn default_palette(allowed: Option<&HashSet<String>>) -> Vec<PaletteEntry> {
    use PaletteEntry::*;

    let touched = Expr::Call {
        callee: Callee::IsTouched,
        args: vec![],
    };
    let if_cond = if is_expr_allowed(&touched, allowed) {
        touched
    } else {
        Expr::Boolean(true)
    };

    let entries = vec![
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
        Stmt(Statement::If(if_cond, Vec::new())),
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
    ];

    let entries = entries.into_iter().filter(|entry| match entry {
        Stmt(stmt) => is_stmt_allowed(stmt, allowed),
        Value(_, expr) => is_expr_allowed(expr, allowed),
        Section(_) | SubSection(_) => true,
    });
    drop_empty_headings(entries.collect())
}

fn drop_empty_headings(entries: Vec<PaletteEntry>) -> Vec<PaletteEntry> {
    use PaletteEntry::*;

    let mut result: Vec<PaletteEntry> = Vec::with_capacity(entries.len());
    for entry in entries {
        match entry {
            Section(_) => {
                while matches!(result.last(), Some(Section(_) | SubSection(_))) {
                    result.pop();
                }
            }
            SubSection(_) => {
                if matches!(result.last(), Some(SubSection(_))) {
                    result.pop();
                }
            }
            Stmt(_) | Value(..) => {}
        }
        result.push(entry);
    }
    while matches!(result.last(), Some(Section(_) | SubSection(_))) {
        result.pop();
    }
    result
}

fn render_palette_panel(
    ui: &mut egui::Ui,
    blocks: &mut Vec<Statement>,
    allowed: Option<&HashSet<String>>,
    type_ctx: &TypeContext,
    wrapped: bool,
) -> f32 {
    let mut allowed_key = allowed.map(|set| set.iter().cloned().collect::<Vec<_>>());
    if let Some(key) = &mut allowed_key {
        key.sort();
    }
    let entries_id = ui.make_persistent_id(("vpl_palette_entries", allowed_key));
    let mut entries = ui
        .data(|d| d.get_temp::<Vec<PaletteEntry>>(entries_id))
        .unwrap_or_else(|| default_palette(allowed));

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

                            let mut idx = 0;
                            while idx < entries.len() {
                                match &entries[idx] {
                                    PaletteEntry::Section(text) => {
                                        ui.add_space(10.0);
                                        section_label(ui, text);
                                        idx += 1;
                                    }
                                    PaletteEntry::SubSection(text) => {
                                        ui.add_space(4.0);
                                        ui.label(egui::RichText::new(*text).weak().small());
                                        idx += 1;
                                    }
                                    PaletteEntry::Stmt(_) | PaletteEntry::Value(..) => {
                                        let start = idx;
                                        while idx < entries.len()
                                            && matches!(
                                                entries[idx],
                                                PaletteEntry::Stmt(_) | PaletteEntry::Value(..)
                                            )
                                        {
                                            idx += 1;
                                        }
                                        let mut render_run = |ui: &mut egui::Ui| {
                                            for (offset, entry) in
                                                entries[start..idx].iter_mut().enumerate()
                                            {
                                                let entry_idx = start + offset;
                                                let width_id =
                                                    entries_id.with(("width", entry_idx));
                                                if wrapped
                                                    && let Some(prev_width) =
                                                        ui.data(|d| d.get_temp::<f32>(width_id))
                                                    && ui.cursor().min.x > ui.max_rect().min.x
                                                    && ui.available_size_before_wrap().x
                                                        < prev_width
                                                {
                                                    ui.end_row();
                                                }
                                                let width = render_palette_entry(
                                                    ui,
                                                    entry,
                                                    entries_id.with(entry_idx),
                                                    blocks,
                                                    type_ctx,
                                                );
                                                ui.data_mut(|d| d.insert_temp(width_id, width));
                                                item_width = item_width.max(width);
                                            }
                                        };
                                        if wrapped {
                                            ui.horizontal_wrapped(|ui| {
                                                ui.spacing_mut().item_spacing.y = 4.0;
                                                render_run(ui);
                                            });
                                        } else {
                                            render_run(ui);
                                        }
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

fn render_palette_entry(
    ui: &mut egui::Ui,
    entry: &mut PaletteEntry,
    block_id: egui::Id,
    blocks: &mut Vec<Statement>,
    type_ctx: &TypeContext,
) -> f32 {
    match entry {
        PaletteEntry::Stmt(stmt) => render::palette_block(ui, stmt, block_id, blocks, type_ctx),
        PaletteEntry::Value(caption, expr) => {
            render::palette_expr_block(ui, *caption, expr, block_id, type_ctx)
        }
        PaletteEntry::Section(_) | PaletteEntry::SubSection(_) => 0.0,
    }
}

fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.separator();
    ui.label(egui::RichText::new(text).weak());
}

fn render_program_panel(ui: &mut egui::Ui, blocks: &mut Vec<Statement>, type_ctx: &TypeContext) {
    egui::ScrollArea::both()
        .id_salt("right_program_scroll")
        .show(ui, |ui| {
            let mut drop_target = None;

            render::block_list(ui, blocks, Vec::new(), &mut drop_target, type_ctx);

            if let Some(target) = drop_target
                && let Some(marker) = target.marker
            {
                render::paint_drop_marker(ui, marker);

                if ui.input(|i| i.pointer.any_released())
                    && let Some(payload) = egui::DragAndDrop::take_payload::<DraggedBlock>(ui.ctx())
                {
                    apply_drop(blocks, payload.as_ref(), target);
                }
            }
        });
}

fn apply_drop(blocks: &mut Vec<Statement>, payload: &DraggedBlock, target: DropTarget) {
    match payload {
        DraggedBlock::NewStatement(stmt) => {
            insert_statement_at_path(blocks, &target.list_path, target.insert_idx, stmt.clone());
        }
        DraggedBlock::MoveStatement { path } => {
            handle_move_request(
                blocks,
                MoveRequest {
                    src_path: path.clone(),
                    target_path: target.list_path,
                    insert_idx: target.insert_idx,
                },
            );
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn allowed(commands: &[&str]) -> HashSet<String> {
        commands.iter().map(|c| c.to_string()).collect()
    }

    fn palette_code(allowed: Option<&HashSet<String>>) -> Vec<String> {
        default_palette(allowed)
            .into_iter()
            .filter_map(|entry| match entry {
                PaletteEntry::Stmt(stmt) => Some(statements_to_string(&[stmt], 0)),
                PaletteEntry::Value(_, expr) => Some(expr.to_code()),
                PaletteEntry::Section(_) | PaletteEntry::SubSection(_) => None,
            })
            .collect()
    }

    fn uses(code: &[String], word: &str) -> bool {
        code.iter().any(|c| c.contains(word))
    }

    #[test]
    fn unrestricted_palette_shows_everything() {
        let code = palette_code(None);
        for word in ["move", "sleep", "dig", "place", "is_touched", "is_empty"] {
            assert!(uses(&code, word), "{word} is missing");
        }
    }

    #[test]
    fn palette_hides_commands_the_stone_cannot_use() {
        let code = palette_code(Some(&allowed(&["move", "sleep"])));
        assert!(uses(&code, "move") && uses(&code, "sleep"));
        for word in ["dig", "place", "is_touched", "is_empty"] {
            assert!(!uses(&code, word), "{word} should be hidden");
        }
        for word in ["turn", "print", "if true", "loop", "while", "rand("] {
            assert!(uses(&code, word), "{word} is missing");
        }
        let code = palette_code(Some(&allowed(&["move", "is_touched", "is_empty", "place"])));
        assert!(uses(&code, "place") && uses(&code, "if is_touched()"));
        assert!(!uses(&code, "dig") && !uses(&code, "sleep"));
    }

    #[test]
    fn palette_has_no_empty_headings() {
        let entries = default_palette(Some(&allowed(&[])));
        for pair in entries.windows(2) {
            let heading_then_heading = matches!(pair[0], PaletteEntry::Section(_))
                && matches!(pair[1], PaletteEntry::Section(_));
            let sub_then_heading = matches!(pair[0], PaletteEntry::SubSection(_))
                && matches!(
                    pair[1],
                    PaletteEntry::Section(_) | PaletteEntry::SubSection(_)
                );
            assert!(!heading_then_heading && !sub_then_heading);
        }
        assert!(!matches!(
            entries.last(),
            Some(PaletteEntry::Section(_) | PaletteEntry::SubSection(_))
        ));
    }

    #[test]
    fn nested_blocks_are_checked() {
        let only_move = allowed(&["move"]);
        let dig_in_loop = Statement::Loop(
            Expr::Uint(3),
            vec![Statement::Dig(Expr::Direction(Direction::Down))],
        );
        assert!(!is_stmt_allowed(&dig_in_loop, Some(&only_move)));
        assert!(is_stmt_allowed(&dig_in_loop, None));

        let not_empty = Expr::Unary {
            op: UnaryOp::Not,
            exp: Box::new(Expr::Call {
                callee: Callee::IsEmpty,
                args: vec![Box::new(Expr::Direction(Direction::Left))],
            }),
        };
        assert!(!is_expr_allowed(&not_empty, Some(&only_move)));
    }
}
