use std::fs;

use bevy::{
    prelude::{App, AssetServer, Assets, Commands, Handle, Plugin, Res, ResMut, Resource, Vec2, Color},
    sprite::{TextureAtlas, TextureAtlasSprite},
    utils::HashMap,
};
use serde::{Deserialize, Serialize};

use crate::world::attackers::AnimationIndices;

#[derive(Resource)]
pub struct TextureResource {
    named_handles: HashMap<String, Handle<TextureAtlas>>,
    named_animations: HashMap<(String, String), AnimationIndices>,
}

impl Default for TextureResource {
    fn default() -> Self {
        Self { named_handles: HashMap::new(), named_animations: HashMap::new() }
    }
}

impl TextureResource {
    pub fn get_atlas(&self, name: &str) -> &Handle<TextureAtlas> {
        return self.named_handles.get(name).unwrap();
    }
    pub fn get_sprite(&self, name: &str, index: usize) -> (&Handle<TextureAtlas>, TextureAtlasSprite) {
        return (self.get_atlas(name), TextureAtlasSprite::new(index));
    }
    pub fn get_sprite_with_tint(&self, name: &str, index: usize, tint_color: Color) -> (&Handle<TextureAtlas>, TextureAtlasSprite) {
        let mut sprite = TextureAtlasSprite::new(index);
        sprite.color = tint_color;
        return (self.get_atlas(name), sprite);
    }
    pub fn get_animation(&self, atlas_name: &str, animation_name: &str) -> (&Handle<TextureAtlas>, &AnimationIndices) {
        return (
            self.get_atlas(atlas_name), 
            self.named_animations.get(&(
                atlas_name.to_string(), 
                animation_name.to_string())
            ).unwrap()
        );
    }

    /* Potentially dangerous stack allocation 😬, assuming sizes large enough to be a problem just aren't ever used */
    pub fn get_animations<const TSIZE: usize>(&self, atlas_name: &str, animation_name: [&str; TSIZE]) -> (&Handle<TextureAtlas>, [AnimationIndices; TSIZE]) {
        let mut result: [AnimationIndices; TSIZE] = [Default::default(); TSIZE];
        let atlas = self.get_atlas(atlas_name);
        for i in 0..TSIZE {
            result[i] = *self.named_animations.get(&(
                atlas_name.to_string(), 
                animation_name[i].to_string())
            ).unwrap();
        }
        return (atlas, result);
    }
}

#[derive(Serialize, Deserialize)]
struct AtlasDefintion {
    path: String,
    name: String,
    tile_size: [f32; 2],
    num_tiles: [usize; 2],
    animations: Option<Vec<AnimationDefinition>>
}

#[derive(Serialize, Deserialize)]
struct AnimationDefinition {
    name: String,
    start: usize,
    end: usize
}

pub struct TexturePlugin;

impl Plugin for TexturePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TextureResource>()
            .add_startup_system(setup);
    }
}

fn setup(
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut named_textures: ResMut<TextureResource>
) {
    let atlas_definitions = read_atlas_definitions();
    for atlas_definition in atlas_definitions {
        let texture_handle = asset_server.load(atlas_definition.path);
        let texture_atlas = TextureAtlas::from_grid(
            texture_handle,
            Vec2::new(atlas_definition.tile_size[0], atlas_definition.tile_size[1]),
            atlas_definition.num_tiles[0],
            atlas_definition.num_tiles[1],
            None,
            None,
        );
        let texture_atlas_handle = texture_atlases.add(texture_atlas);
        named_textures.named_handles.insert(atlas_definition.name.clone(), texture_atlas_handle);
        if let Some(animations) = atlas_definition.animations {
            for animation_definition in animations {
                named_textures.named_animations.insert(
                    (atlas_definition.name.clone(), animation_definition.name), 
                    AnimationIndices::new(
                        animation_definition.start, 
                        animation_definition.end
                    )
                );
            }
        }

    }
}

fn read_atlas_definitions() -> Vec<AtlasDefintion> {
    return match fs::read_to_string("assets/definitions.json") {
        Ok(contents) => {
            match serde_json::from_str::<Vec<AtlasDefintion>>(&contents) {
                Ok(definitions) => definitions,
                Err(err) => panic!("Failed to parse json {}", err)
            }
        },
        Err(err) => panic!("Failed to read file {}", err)
    }
}
