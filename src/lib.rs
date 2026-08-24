use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};

use keystone_lang::{Direction, Expr, Statement};

#[derive(Clone, PartialEq, Debug)]
enum DraggedBlock {
    New(Statement),
    Move { path_id: String, index: usize },
}

pub trait ToCode {
    fn to_code(&self) -> String;
}

impl ToCode for Expr {
    fn to_code(&self) -> String {
        match self {
            Expr::Uint(v) => format!("{}", v),
            Expr::Float(v) => format!("{:.1}", v),
            Expr::String(s) => format!("\"{}\"", s),
            Expr::Boolean(b) => format!("{}", b),
            Expr::Direction(d) => match d {
                Direction::Left => "left".to_string(),
                Direction::Right => "right".to_string(),
                Direction::Forward => "forward".to_string(),
                Direction::Back => "back".to_string(),
                Direction::Up => "up".to_string(),
                Direction::Down => "down".to_string(),
            },
            Expr::Var(name) => name.clone(),
            Expr::Binary { op, lhs, rhs } => {
                format!("{} {:?} {}", lhs.to_code(), op, rhs.to_code())
            }
            Expr::Unary { op, exp } => format!("{:?} {}", op, exp.to_code()),
            Expr::Call { callee, args } => {
                let args_str = args
                    .iter()
                    .map(|a| a.to_code())
                    .collect::<Vec<_>>()
                    .join(", ");
                let callee_name = match callee {
                    keystone_lang::Callee::IsTouched => "is_touched",
                    keystone_lang::Callee::IsEmpty => "is_empty",
                    keystone_lang::Callee::Rand => "rand",
                };
                format!("{}({})", callee_name, args_str)
            }
        }
    }
}

pub fn statements_to_string(statements: &[Statement], indent_level: usize) -> String {
    let mut code = String::new();
    let indent = "  ".repeat(indent_level);

    for stmt in statements {
        match stmt {
            Statement::Print(e) => code.push_str(&format!("{}print {}\n", indent, e.to_code())),
            Statement::Move(e) => code.push_str(&format!("{}move {}\n", indent, e.to_code())),
            Statement::Turn(e) => code.push_str(&format!("{}turn {}\n", indent, e.to_code())),
            Statement::Dig(e) => code.push_str(&format!("{}dig {}\n", indent, e.to_code())),
            Statement::Let(name, e) => {
                code.push_str(&format!("{}{} = {}\n", indent, name, e.to_code()))
            }
            Statement::Sleep(e) => code.push_str(&format!("{}sleep {}\n", indent, e.to_code())),
            Statement::Receive(e) => code.push_str(&format!("{}receive {}\n", indent, e.to_code())),
            Statement::Send(e) => code.push_str(&format!("{}send {}\n", indent, e.to_code())),

            Statement::Loop(cond, body) => {
                code.push_str(&format!("{}loop {}\n", indent, cond.to_code()));
                code.push_str(&statements_to_string(body, indent_level + 1));
                code.push_str(&format!("{}end\n", indent));
            }
            Statement::While(cond, body) => {
                code.push_str(&format!("{}while {}\n", indent, cond.to_code()));
                code.push_str(&statements_to_string(body, indent_level + 1));
                code.push_str(&format!("{}end\n", indent));
            }
            Statement::If(cond, body) => {
                code.push_str(&format!("{}if {}\n", indent, cond.to_code()));
                code.push_str(&statements_to_string(body, indent_level + 1));
                code.push_str(&format!("{}end\n", indent));
            }
        }
    }
    code
}

#[derive(Resource, Default)]
pub struct VplState {
    pub blocks: Vec<Statement>,
    pub generated_code: String,
}

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
                    render_block_list(ui, &mut state.blocks, "root");
                });
            });

            ui.separator();

            state.generated_code = statements_to_string(&state.blocks, 0);
            ui.heading("GENERATED CODE");
            ui.code(&state.generated_code);
        });

    Ok(())
}

fn render_palette_button(
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

fn render_block_list(ui: &mut egui::Ui, blocks: &mut Vec<Statement>, path_id: &str) {
    let mut delete_target_idx = None;

    for idx in 0..blocks.len() {
        let current_id_str = format!("{}_{}", path_id, idx);
        let block_id = egui::Id::new(&current_id_str);

        let (_, dropped_payload) =
            ui.dnd_drop_zone::<DraggedBlock, _>(egui::Frame::NONE.inner_margin(2.0), |ui| {
                ui.horizontal(|ui| {
                    ui.dnd_drag_source(
                        block_id.with("drag_handle"),
                        DraggedBlock::Move {
                            path_id: path_id.to_string(),
                            index: idx,
                        },
                        |ui| {
                            ui.horizontal(|ui| {
                                ui.label("☰").on_hover_cursor(egui::CursorIcon::Grab);
                                let label_text = match &blocks[idx] {
                                    Statement::Move(_) => "🏃 Move",
                                    Statement::Turn(_) => "🔄 Turn",
                                    Statement::Print(_) => "💬 Print",
                                    Statement::Sleep(_) => "💤 Sleep",
                                    Statement::If(_, _) => "❓ [If]",
                                    Statement::Loop(_, _) => "🔁 [Loop]",
                                    Statement::While(_, _) => "🔄 [While]",
                                    _ => "📄 Statement",
                                };
                                ui.label(egui::RichText::new(label_text).strong());
                            });
                        },
                    );

                    ui.separator();

                    match &mut blocks[idx] {
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
                        Statement::If(_cond, body) => {
                            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                                egui::CollapsingHeader::new("if is_touched()")
                                    .id_salt(&current_id_str)
                                    .default_open(true)
                                    .show(ui, |ui| {
                                        render_block_list(ui, body, &current_id_str);
                                    });
                            });
                        }
                        Statement::Loop(expr, body) => {
                            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
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
                                        render_block_list(ui, body, &current_id_str);
                                    });
                            });
                        }
                        Statement::While(_cond, body) => {
                            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                                egui::CollapsingHeader::new("while true")
                                    .id_salt(&current_id_str)
                                    .default_open(true)
                                    .show(ui, |ui| {
                                        render_block_list(ui, body, &current_id_str);
                                    });
                            });
                        }
                        _ => {}
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("❌").clicked() {
                            delete_target_idx = Some(idx);
                        }
                    });
                });
            });

        if let Some(payload) = dropped_payload {
            match payload.as_ref() {
                DraggedBlock::New(new_stmt) => {
                    blocks.insert(idx, new_stmt.clone());
                    return;
                }
                DraggedBlock::Move {
                    path_id: src_path,
                    index: src_idx,
                } => {
                    if src_path == path_id && *src_idx != idx {
                        let moved = blocks.remove(*src_idx);
                        let insert_idx = if *src_idx < idx { idx - 1 } else { idx };
                        blocks.insert(insert_idx, moved);
                        return;
                    }
                    if src_path != path_id {
                        blocks.insert(idx, blocks[*src_idx].clone());
                        return;
                    }
                }
            }
        }
        ui.add_space(2.0);
    }

    let (_, bottom_payload) = ui.dnd_drop_zone::<DraggedBlock, _>(egui::Frame::NONE, |ui| {
        let response =
            ui.allocate_response(egui::vec2(ui.available_width(), 30.0), egui::Sense::hover());
        ui.painter().rect_stroke(
            response.rect,
            2.0,
            egui::Stroke::new(1.0, egui::Color32::from_gray(60)),
            egui::StrokeKind::Inside,
        );
    });

    if let Some(payload) = bottom_payload {
        match payload.as_ref() {
            DraggedBlock::New(new_stmt) => {
                blocks.push(new_stmt.clone());
            }
            DraggedBlock::Move {
                path_id: src_path,
                index: src_idx,
            } => {
                if src_path == path_id {
                    let moved = blocks.remove(*src_idx);
                    blocks.push(moved);
                }
            }
        }
    }

    if let Some(idx) = delete_target_idx {
        blocks.remove(idx);
    }
}

fn render_direction_combobox(ui: &mut egui::Ui, dir: &mut Direction, id_salt: &str) {
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
