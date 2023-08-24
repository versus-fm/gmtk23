use bevy::{prelude::{Resource, Entity, Plugin, App, Query, Transform, Added, ResMut, Vec2, Commands, Res, Handle, default, Color}, sprite::{SpriteSheetBundle, TextureAtlasSprite, TextureAtlas}};

use crate::textures::TextureResource;

use self::{towers::{Structure, TowerField, WallBundle, StructureBuilder, ArrowTower, TowersPlugin, SLOT_SIZE}, path_finding::{Node, a_star}, attackers::AttackersPlugin, building_configuration::BuildingResource, events::EventsPlugin, rounds::RoundPlugin};

pub mod towers;
pub mod path_finding;
pub mod attacker_controller;
pub mod defender_controller;
pub mod attackers;
pub mod building_configuration;
pub mod events;
pub mod rounds;


pub struct TowerFieldPlugin;

impl Plugin for TowerFieldPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(TowerField::new(
                16, 
                16, 
                Vec2::ZERO, 
                Node::new(2, 0), 
                Node::new(14, 15)
            ))
            .add_plugin(RoundPlugin)
            .add_plugin(EventsPlugin)
            .add_plugin(AttackersPlugin)
            .add_plugin(TowersPlugin)
            //.add_startup_system(setup)
            .add_startup_system(setup_environment); 
    }
}

fn setup(
    mut commands: Commands,
    textures: Res<TextureResource>,
    buildings: Res<BuildingResource>,
    tower_field: Res<TowerField>
) {
    commands.spawn(WallBundle::from_tower_field(&buildings, &tower_field, &textures, 0, 0));
    commands.spawn(WallBundle::from_tower_field(&buildings, &tower_field, &textures, 0, 1));
    commands.spawn(WallBundle::from_tower_field(&buildings, &tower_field, &textures, 1, 1));
    commands.spawn(WallBundle::from_tower_field(&buildings, &tower_field, &textures, 2, 1));
    commands.spawn(WallBundle::from_tower_field(&buildings, &tower_field, &textures, 0, 2));

    commands.spawn(ArrowTower::from_tower_field(&buildings, &tower_field, &textures, 12, 0));
    commands.spawn(ArrowTower::from_tower_field(&buildings, &tower_field, &textures, 10, 3));
    commands.spawn(ArrowTower::from_tower_field(&buildings, &tower_field, &textures, 12, 1));
    commands.spawn(WallBundle::from_tower_field(&buildings, &tower_field, &textures, 12, 2));
    commands.spawn(ArrowTower::from_tower_field(&buildings, &tower_field, &textures, 12, 3));
    commands.spawn(WallBundle::from_tower_field(&buildings, &tower_field, &textures, 12, 4));
    commands.spawn(ArrowTower::from_tower_field(&buildings, &tower_field, &textures, 13, 5));
    commands.spawn(WallBundle::from_tower_field(&buildings, &tower_field, &textures, 14, 6));
}

fn setup_environment(
    mut commands: Commands,
    textures: Res<TextureResource>,
    tower_field: Res<TowerField>
) {
    let width = (tower_field.get_width() * SLOT_SIZE / 16) as i32;
    let height = (tower_field.get_height() * SLOT_SIZE / 16) as i32;

    let offset = 4;

    for x in -offset..=width+offset {
        for y in -offset..=height+offset {
            if y == -offset && x == -offset {
                spawn_left_bottom_tile(&mut commands, &textures, x, y);
            } else if y == -offset && x == width+offset {
                spawn_right_bottom_tile(&mut commands, &textures, x, y);
            } else if y == height+offset && x == -offset {
                spawn_left_top_tile(&mut commands, &textures, x, y);
            } else if y == height+offset && x == width+offset {
                spawn_right_top_tile(&mut commands, &textures, x, y);
            } else if y == -offset {
                spawn_mid_tile(&mut commands, &textures, x, y);
            } else if x == -offset {
                spawn_left_tile(&mut commands, &textures, x, y);
            } else if x == width+offset {
                spawn_right_tile(&mut commands, &textures, x, y);
            } else if y == height+offset {
                spawn_top_tile(&mut commands, &textures, x, y);
            } else {
                spawn_ground_tile(&mut commands, &textures, x, y);
            }
        }
    }
}

fn spawn_mid_tile(
    commands: &mut Commands,
    textures: &TextureResource,
    x: i32,
    y: i32
) {
    let transform1 = Transform::from_xyz(x as f32 * 16., y as f32 * 16., 0.);
    let transform2 = Transform::from_xyz(x as f32 * 16., y as f32 * 16. - 16., 0.);
    let transform3 = Transform::from_xyz(x as f32 * 16., y as f32 * 16. - 32., 0.);
    spawn_texture(commands, textures, transform1, "outside", 700);
    spawn_texture(commands, textures, transform2, "outside", 704);
    spawn_texture(commands, textures, transform3, "outside", 718);
}

fn spawn_left_tile(
    commands: &mut Commands,
    textures: &TextureResource,
    x: i32,
    y: i32
) {
    let transform1 = Transform::from_xyz(x as f32 * 16., y as f32 * 16., 0.);
    spawn_texture(commands, textures, transform1, "outside", 626);
}

fn spawn_right_tile(
    commands: &mut Commands,
    textures: &TextureResource,
    x: i32,
    y: i32
) {
    let transform1 = Transform::from_xyz(x as f32 * 16., y as f32 * 16., 0.);
    spawn_texture(commands, textures, transform1, "outside", 630);
}

fn spawn_left_top_tile(
    commands: &mut Commands,
    textures: &TextureResource,
    x: i32,
    y: i32
) {
    let transform1 = Transform::from_xyz(x as f32 * 16., y as f32 * 16., 0.);
    spawn_texture(commands, textures, transform1, "outside", 585);
}

fn spawn_right_top_tile(
    commands: &mut Commands,
    textures: &TextureResource,
    x: i32,
    y: i32
) {
    let transform1 = Transform::from_xyz(x as f32 * 16., y as f32 * 16., 0.);
    spawn_texture(commands, textures, transform1, "outside", 587);
}

fn spawn_top_tile(
    commands: &mut Commands,
    textures: &TextureResource,
    x: i32,
    y: i32
) {
    let transform1 = Transform::from_xyz(x as f32 * 16., y as f32 * 16., 0.);
    spawn_texture(commands, textures, transform1, "outside", 586);
}

fn spawn_ground_tile(
    commands: &mut Commands,
    textures: &TextureResource,
    x: i32,
    y: i32
) {
    let transform1 = Transform::from_xyz(x as f32 * 16., y as f32 * 16., 0.);
    spawn_texture(commands, textures, transform1, "outside", 624);
}

fn spawn_left_bottom_tile(
    commands: &mut Commands,
    textures: &TextureResource,
    x: i32,
    y: i32
) {
    let transform1 = Transform::from_xyz(x as f32 * 16., y as f32 * 16., 0.);
    let transform2 = Transform::from_xyz(x as f32 * 16., y as f32 * 16. - 16., 0.);
    let transform3 = Transform::from_xyz(x as f32 * 16., y as f32 * 16. - 32., 0.);
    spawn_texture(commands, textures, transform1, "outside", 660);
    spawn_texture(commands, textures, transform2, "outside", 667);
    spawn_texture(commands, textures, transform3, "outside", 713);
}

fn spawn_right_bottom_tile(
    commands: &mut Commands,
    textures: &TextureResource,
    x: i32,
    y: i32
) {
    let transform1 = Transform::from_xyz(x as f32 * 16., y as f32 * 16., 0.);
    let transform2 = Transform::from_xyz(x as f32 * 16., y as f32 * 16. - 16., 0.);
    let transform3 = Transform::from_xyz(x as f32 * 16., y as f32 * 16. - 32., 0.);
    spawn_texture(commands, textures, transform1, "outside", 664);
    spawn_texture(commands, textures, transform2, "outside", 671);
    spawn_texture(commands, textures, transform3, "outside", 716);
}

fn spawn_texture(
    commands: &mut Commands,
    textures: &TextureResource,
    transform: Transform,
    name: &str,
    index: usize
) {
    let sprite: (&Handle<bevy::sprite::TextureAtlas>, TextureAtlasSprite) = textures.get_sprite_with_tint(name, index, Color::rgba(0.55, 0.55, 0.55, 1.));
    commands.spawn(SpriteSheetBundle { 
        sprite: sprite.1, 
        texture_atlas: sprite.0.clone_weak(), 
        transform: transform, 
        ..default()
    });
}