#[allow(dead_code, unused_variables, unused_mut, unused_imports)]
mod game;

use bevy::prelude::*;
use bevy_inspector_egui::WorldInspectorPlugin;
use bevy_rapier3d::prelude::*;

use crate::game::GamePlugin;

fn main() {
    App::new()
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 0.4,
        })
        .insert_resource(ClearColor(Color::rgb(0.12, 0.12, 0.15)))
        .add_plugins(DefaultPlugins)
        .add_plugin(WorldInspectorPlugin::new())
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        // .add_plugin(RapierDebugRenderPlugin::default())
        .add_plugin(GamePlugin)
        .run();
}
