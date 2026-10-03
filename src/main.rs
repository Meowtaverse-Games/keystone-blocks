use bevy::prelude::*;
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
        .add_plugins(VisualProgrammingPlugin)
        .add_systems(Startup, setup_camera_system)
        .add_systems(Update, test_code_generation_and_compile_system)
        .run();
}

fn setup_camera_system(mut commands: Commands) {
    commands.spawn(Camera2d);
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
