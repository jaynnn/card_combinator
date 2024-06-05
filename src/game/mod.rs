pub mod animate;
pub mod camera;
pub mod card;
pub mod progress_bar;
pub mod tile;

use std::f32::consts::PI;

use self::camera::PlayerCameraPlugin;
use crate::game::{
    card::{Card, CardBundle, CardPlugin, CardType},
    progress_bar::{ProgressBar, ProgressBarBundle, ProgressBarPlugin},
    tile::TilePlugin,
};
use bevy::prelude::*;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(CardPlugin)
            .add_plugins(PlayerCameraPlugin)
            .add_plugins(ProgressBarPlugin)
            .add_plugins(TilePlugin)
            .add_systems(Startup, setup);
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    commands.spawn(CardBundle {
        transform: Transform::from_xyz(-0.5, 0.0, 0.0),
        card: Card::from(CardType::Villager),
        ..default()
    });
    commands.spawn(CardBundle {
        transform: Transform::from_xyz(0.5, 0.0, 0.0),
        card: Card::from(CardType::Villager),
        ..default()
    });

    // commands.spawn(CardBundle {
    //     transform: Transform::from_xyz(0.0, 3.0, 0.0),
    //     card: Card::from(CardType::Goblin),
    //     ..default()
    // });

    // commands.spawn(CardBundle {
    //     transform: Transform::from_xyz(1.0, 0.0, 0.0),
    //     card: Card {
    //         card_type: CardType::Log,
    //         ..default()
    //     },
    //     ..default()
    // });
}
