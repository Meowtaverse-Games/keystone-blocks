use bevy_egui::egui;
use keystone_lang::Statement;

pub mod colors {
    use bevy_egui::egui::Color32;

    pub const MOTION: Color32 = Color32::from_rgb(76, 151, 255);
    pub const LOOKS: Color32 = Color32::from_rgb(153, 102, 255);
    pub const EVENT: Color32 = Color32::from_rgb(255, 94, 162);
    pub const CONTROL: Color32 = Color32::from_rgb(255, 171, 25);
    pub const VARIABLES: Color32 = Color32::from_rgb(255, 140, 0);
    pub const OPERATORS: Color32 = Color32::from_rgb(89, 192, 89);

    pub const SLOT_BG: Color32 = Color32::from_rgb(35, 40, 50);
    pub const SLOT_BORDER: Color32 = Color32::from_rgb(20, 25, 30);
    pub const TEXT_WHITE: Color32 = Color32::WHITE;
}

pub fn get_stmt_color(stmt: &Statement) -> egui::Color32 {
    match stmt {
        Statement::Move(_) | Statement::Turn(_) | Statement::Dig(_) => colors::MOTION,
        Statement::Print(_) => colors::LOOKS,
        Statement::Sleep(_)
        | Statement::If(_, _)
        | Statement::Loop(_, _)
        | Statement::While(_, _) => colors::CONTROL,
        Statement::Let(_, _) => colors::VARIABLES,
        Statement::Send(_) | Statement::Receive(_) => colors::EVENT,
    }
}
