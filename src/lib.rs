use wasm_bindgen::prelude::*;

use bevy::{prelude::*, window::PrimaryWindow};
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiSettings};
use camera::CameraController;
use particle::ParticlePlugin;
use textures::TexturePlugin;
use ui::UiPlugin;
use world::{TowerFieldPlugin, building_configuration::BuildingResource, attacker_controller::AttackerController, defender_controller::DefenderController};

pub mod world;
pub mod textures;
pub mod util;
pub mod camera;
pub mod ui;
pub mod particle;

#[wasm_bindgen]
pub fn run() {
    let mut app = App::new();

    app
        .insert_resource(ClearColor(Color::rgb(0.04, 0.04, 0.04)))
        .insert_resource(BuildingResource::new())
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugin(EguiPlugin)
        .add_plugin(TexturePlugin)
        .add_plugin(TowerFieldPlugin)
        .add_plugin(CameraController)
        .add_plugin(AttackerController)
        .add_plugin(DefenderController)
        .add_plugin(UiPlugin)
        .add_plugin(ParticlePlugin)
        // Systems that create Egui widgets should be run during the `CoreSet::Update` set,
        // or after the `EguiSet::BeginFrame` system (which belongs to the `CoreSet::PreUpdate` set).
        .add_startup_system(setup_graphics)
        .add_system(update_ui_scale_factor)
    .run();
}


fn setup_graphics(mut commands: Commands) {
    // Add a camera so we can see the debug-render.
    let mut camera = Camera2dBundle {..Default::default()};
    commands.spawn(camera);
}

fn update_ui_scale_factor(mut egui_settings: ResMut<EguiSettings>, windows: Query<&Window, With<PrimaryWindow>>) {
    if let Ok(window) = windows.get_single() {
        egui_settings.scale_factor = 1.2 / window.scale_factor();
    }
}