use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use keystone_blocks::{VisualProgrammingPlugin, VplState};
use keystone_lang::{Direction, ExternalApi, eval};
use std::sync::Arc;

struct DummyApi;

impl ExternalApi for DummyApi {
    fn is_touched(&self) -> bool {
        false
    }
    fn is_empty(&self, _dir: Direction) -> bool {
        true
    }
    fn send_signal(&self, _channel: &str) {}
    fn receive_signal(&self, _channel: &str) -> bool {
        false
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(bevy_egui::EguiPlugin::default())
        .add_plugins(VisualProgrammingPlugin)
        .add_systems(Startup, (setup_camera_system, setup_vpl_test_env_system))
        .add_systems(Update, test_code_generation_and_compile_system)
        .add_systems(EguiPrimaryContextPass, vpl_panel_system)
        .run();
}

fn setup_camera_system(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn setup_vpl_test_env_system(mut vpl_state: ResMut<VplState>) {
    vpl_state.is_visible = true;
}

fn vpl_panel_system(mut contexts: EguiContexts, mut vpl_state: ResMut<VplState>) -> Result {
    if !vpl_state.is_visible {
        return Ok(());
    }
    let ctx = contexts.ctx_mut()?;
    let rect = ctx.content_rect();
    egui::Area::new(egui::Id::new("vpl-demo"))
        .fixed_pos(rect.left_top())
        .show(ctx, |ui| {
            ui.set_min_size(rect.size());
            keystone_blocks::show_vpl_contents(ui, &mut vpl_state);
        });
    Ok(())
}

fn test_code_generation_and_compile_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    vpl_state: Res<VplState>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        println!("--- [TEST] Generating & Compiling Code ---");

        let generated_code = keystone_blocks::generate_code_from_state(&vpl_state);
        println!("Generated Code:\n{}", generated_code);

        let dummy_api = Arc::new(DummyApi);

        match eval(&generated_code, dummy_api) {
            Ok(_event_iter) => {
                println!("[SUCCESS] Compilation successful! No syntax/runtime definition errors.");
            }
            Err(e) => {
                println!("[COMPILE ERROR] Failed to evaluate generated code!");
                println!("Error details: {:?}", e);
            }
        }

        println!("--------------------------------------------");
    }
}
