use std::time::Duration;

use bevy::{prelude::{Plugin, App, Bundle, Component, Commands, Vec2, Transform, Query, Entity, Res}, sprite::{SpriteSheetBundle, TextureAtlasSprite}, time::{Timer, Time}};

use crate::{world::attackers::{AnimationIndices, AnimationTimer}, textures::TextureResource};
use rand::Rng;

pub struct ParticlePreset {
    sprite_name: String,
    animation_name: String,
    time_to_live: Duration,
    velocity: Vec2,
    frame_time: Duration,
    behavior: ParticleBehaviour
}

#[derive(PartialEq, PartialOrd, Clone, Copy)]
pub enum ParticleBehaviour {
    DespawnLastFrame,
    DespawnOnTTL
}

#[derive(Component)]
pub struct Particle {
    timer: Timer,
    velocity: Vec2,
    behavior: ParticleBehaviour
}

#[derive(Bundle)]
pub struct ParticleBundle {
    particle: Particle,
    animation: AnimationIndices,
    animation_timer: AnimationTimer,
    #[bundle]
    sprite: SpriteSheetBundle,
}

pub struct ParticlePlugin;

impl Plugin for ParticlePlugin {
    fn build(&self, app: &mut App) {
        app.add_system(update_particles);
    }
}

pub fn spawn_large_explosion(commands: &mut Commands, transform: &Transform, textures: &TextureResource) {
    spawn_particle(commands, &ParticlePreset {
        sprite_name: "large_explosion".to_string(),
        animation_name: "primary".to_string(),
        behavior: ParticleBehaviour::DespawnLastFrame,
        frame_time: Duration::from_secs_f32(0.2),
        time_to_live: Duration::from_secs_f32(1.5),
        velocity: Vec2::ZERO
    }, transform, textures)
}

pub fn spawn_blood_splatter(commands: &mut Commands, transform: &Transform, textures: &TextureResource) {
    spawn_particle(commands, &ParticlePreset {
        sprite_name: "blood_splatter".to_string(),
        animation_name: "primary".to_string(),
        behavior: ParticleBehaviour::DespawnLastFrame,
        frame_time: Duration::from_secs_f32(0.4),
        time_to_live: Duration::from_secs_f32(1.5),
        velocity: Vec2::new(rand::thread_rng().gen_range(-1.0..1.), rand::thread_rng().gen_range(-1.0..1.))
    }, transform, textures)
}

pub fn spawn_coin(commands: &mut Commands, transform: &Transform, textures: &TextureResource) {
    spawn_particle(commands, &ParticlePreset {
        sprite_name: "coin".to_string(),
        animation_name: "primary".to_string(),
        behavior: ParticleBehaviour::DespawnOnTTL,
        frame_time: Duration::from_secs_f32(1.2),
        time_to_live: Duration::from_secs_f32(1.5),
        velocity: Vec2::new(0., 10. + rand::thread_rng().gen_range(0.0..5.))
    }, transform, textures)
}

pub fn spawn_particle(commands: &mut Commands, preset: &ParticlePreset, transform: &Transform, textures: &TextureResource) {
    let animation = textures.get_animation(&preset.sprite_name, &preset.animation_name);
    commands.spawn(ParticleBundle {
        particle: Particle {
            timer: Timer::from_seconds(preset.time_to_live.as_secs_f32(), bevy::time::TimerMode::Once),
            velocity: preset.velocity,
            behavior: preset.behavior
        },
        animation_timer: AnimationTimer(Timer::new(preset.frame_time, bevy::time::TimerMode::Repeating)),
        sprite: SpriteSheetBundle { 
            sprite: TextureAtlasSprite::new(animation.1.start), 
            texture_atlas: animation.0.clone_weak(), 
            transform: *transform, 
            ..Default::default()
        },
        animation: AnimationIndices { start: animation.1.start, end: animation.1.end }
    });
}

pub fn update_particles(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut Particle, &mut TextureAtlasSprite, &mut AnimationTimer, &AnimationIndices)>,
    time: Res<Time>
) {
    for (entity, mut transform, mut particle, mut sprite, mut animation_timer, animation_index) in query.iter_mut() {
        particle.timer.tick(time.delta());
        animation_timer.0.tick(time.delta());
        if particle.timer.finished() {
            commands.entity(entity).despawn();
        } else {
            transform.translation += particle.velocity.extend(0.) * time.delta_seconds();
            if animation_timer.0.just_finished() {
                let index = sprite.index;
                if animation_index.start == animation_index.end && particle.behavior == ParticleBehaviour::DespawnOnTTL {
                    sprite.index = animation_index.start;
                } else if animation_index.start == animation_index.end && particle.behavior == ParticleBehaviour::DespawnLastFrame {
                    commands.entity(entity).despawn();
                } else {
                    if index > animation_index.end || index < animation_index.start {
                        sprite.index = animation_index.start;
                    } else {
                        sprite.index = if sprite.index >= animation_index.end {
                            animation_index.start
                        } else {
                            sprite.index + 1
                        }
                    }
                }
                
            }
        }

    }
}