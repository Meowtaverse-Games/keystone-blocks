use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};

use keystone_lang::{Direction, Expr, Statement};

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
                ui_left.heading("➕ Add Blocks");
                ui_left.separator();

                if ui_left.button("🏃 Move (Direction)").clicked() {
                    state
                        .blocks
                        .push(Statement::Move(Expr::Direction(Direction::Forward)));
                }
                if ui_left.button("🔄 Turn (Direction)").clicked() {
                    state
                        .blocks
                        .push(Statement::Turn(Expr::Direction(Direction::Forward)));
                }
                if ui_left.button("💬 Print (String)").clicked() {
                    state
                        .blocks
                        .push(Statement::Print(Expr::String("hello".to_string())));
                }
                if ui_left.button("💤 Sleep (Float)").clicked() {
                    state.blocks.push(Statement::Sleep(Expr::Float(1.0)));
                }

                ui_left.add_space(10.0);
                ui_left.label("ーーー Nest Blocks ーーー");

                if ui_left.button("❓ If (is_touched())").clicked() {
                    state.blocks.push(Statement::If(
                        Expr::Call {
                            callee: keystone_lang::Callee::IsTouched,
                            args: vec![],
                        },
                        Vec::new(),
                    ));
                }
                if ui_left.button("🔁 Loop (Count)").clicked() {
                    state
                        .blocks
                        .push(Statement::Loop(Expr::Uint(3), Vec::new()));
                }
                if ui_left.button("🔄 While (true)").clicked() {
                    state
                        .blocks
                        .push(Statement::While(Expr::Boolean(true), Vec::new()));
                }

                ui_left.add_space(20.0);
                if ui_left.button("🗑️ Clear All").clicked() {
                    state.blocks.clear();
                }

                let ui_right = &mut columns[1];
                ui_right.heading("📝 Current Program");
                ui_right.separator();

                egui::ScrollArea::vertical().show(ui_right, |ui| {
                    if state.blocks.is_empty() {
                        ui.label("(No blocks added yet)");
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

fn render_block_list(ui: &mut egui::Ui, blocks: &mut Vec<Statement>, path_id: &str) {
    let mut delete_target_idx = None;
    let mut swap_target = None;
    let len = blocks.len();

    for idx in 0..len {
        let current_id_str = format!("{}_{}", path_id, idx);

        ui.horizontal(|ui| {
            ui.add_enabled_ui(idx > 0, |ui| {
                if ui.button("⬆").clicked() {
                    swap_target = Some((idx, idx - 1));
                }
            });
            ui.add_enabled_ui(idx + 1 < len, |ui| {
                if ui.button("⬇").clicked() {
                    swap_target = Some((idx, idx + 1));
                }
            });

            match &mut blocks[idx] {
                Statement::Move(expr) => {
                    ui.label("🏃 Move");
                    if let Expr::Direction(dir) = expr {
                        render_direction_combobox(ui, dir, &format!("move_{}", current_id_str));
                    }
                }
                Statement::Turn(expr) => {
                    ui.label("🔄 Turn");
                    if let Expr::Direction(dir) = expr {
                        render_direction_combobox(ui, dir, &format!("turn_{}", current_id_str));
                    }
                }
                Statement::Print(expr) => {
                    ui.label("💬 Print");
                    if let Expr::String(s) = expr {
                        ui.text_edit_singleline(s);
                    }
                }
                Statement::Sleep(expr) => {
                    ui.label("💤 Sleep");
                    if let Expr::Float(v) = expr {
                        ui.add(egui::DragValue::new(v).speed(0.1));
                        ui.label("sec");
                    }
                }

                Statement::If(_cond, body) => {
                    ui.label("❓ [If]");
                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                        egui::CollapsingHeader::new("if is_touched()")
                            .id_salt(&current_id_str)
                            .default_open(true)
                            .show(ui, |ui| {
                                render_block_list(ui, body, &current_id_str);
                                ui.horizontal(|ui| {
                                    if ui.button("➕ Add Move inside").clicked() {
                                        body.push(Statement::Move(Expr::Direction(
                                            Direction::Forward,
                                        )));
                                    }
                                });
                            });
                    });
                }
                Statement::Loop(expr, body) => {
                    ui.label("🔁 [Loop]");
                    let count_str = if let Expr::Uint(v) = expr {
                        v.to_string()
                    } else {
                        "3".to_string()
                    };

                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                        egui::CollapsingHeader::new(format!("loop {}", count_str))
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
                    ui.label("🔄 [While]");
                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                        egui::CollapsingHeader::new("while true")
                            .id_salt(&current_id_str)
                            .default_open(true)
                            .show(ui, |ui| {
                                render_block_list(ui, body, &current_id_str);
                            });
                    });
                }
                _ => {
                    ui.label("📄 Statement");
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("❌").clicked() {
                    delete_target_idx = Some(idx);
                }
            });
        });
        ui.add_space(4.0);
    }

    if let Some((from, to)) = swap_target {
        blocks.swap(from, to);
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
