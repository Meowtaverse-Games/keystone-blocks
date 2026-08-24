use bevy::prelude::*;
use keystone_blocks::VisualProgrammingPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(VisualProgrammingPlugin)
        .add_systems(Startup, setup_camera_system)
        .run();
}

fn setup_camera_system(mut commands: Commands) {
    commands.spawn(Camera2d);
}
