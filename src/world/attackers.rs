use bevy::{
    prelude::{
        App, Bundle, Commands, Component, Deref, DerefMut, Entity, EventReader, EventWriter, Local,
        Plugin, Query, Res, ResMut, Resource, Timer, Transform, Vec2, With, Without,
    },
    sprite::{SpriteSheetBundle, TextureAtlas, TextureAtlasSprite},
    time::{Time, TimerMode},
    utils::HashMap,
};
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::{
    textures::TextureResource,
    util::{LocalTimer, RepeatingLocalTimer},
};

use super::{
    events::{EntityReachedEnd, FieldModified},
    path_finding::{a_star, Path},
    towers::{TowerField, SLOT_SIZE},
};

#[derive(Component, Clone, Copy)]
pub struct Attacker {
    pub health: f32,
    pub max_health: f32,
    pub movement_speed: f32,
    pub velocity: Vec2,
    pub size: Vec2,
    pub bounty: i32,
    pub original_cost: i32,
    pub num_summoned: i32,
}

#[derive(Component)]
pub struct Flying;
#[derive(Component)]
pub struct Grounded;

#[derive(Component, Deref, DerefMut)]
pub struct AnimationTimer(pub Timer);

#[derive(Clone, Copy, Deserialize, Serialize, Component)]
pub struct AnimationIndices {
    pub start: usize,
    pub end: usize,
}

impl Default for AnimationIndices {
    fn default() -> Self {
        Self { start: 0, end: 0 }
    }
}

impl AnimationIndices {
    pub fn new(start: usize, end: usize) -> Self {
        return Self { start, end };
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum UpgradeType {
    Speed,
    Health,
    Amount,
}


pub struct UpgradeInfo {
    pub effect: f32,
    pub cost: i32,
    pub effect_type: UpgradeEffectType,
    pub description: String
}

impl UpgradeInfo {
    pub fn apply_value_f32(&self, current_value: f32) -> f32 {
        if self.effect_type == UpgradeEffectType::Factor {
            return current_value * self.effect;
        } else {
            return current_value + self.effect;
        }
    }
    pub fn apply_value(&self, current_value: i32) -> i32 {
        if self.effect_type == UpgradeEffectType::Factor {
            return (current_value as f32 * self.effect).round() as i32;
        } else {
            return current_value + self.effect as i32;
        }
    }
}


#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum UpgradeEffectType {
    Flat,
    Factor
}

#[derive(Resource)]
pub struct AttackerStats {
    stats: HashMap<AttackerType, Attacker>,
    upgrade_map: HashMap<(AttackerType, UpgradeType), UpgradeInfo>
}

impl AttackerStats {
    pub fn get_stats(&self, attacker_type: AttackerType) -> &Attacker {
        return self.stats.get(&attacker_type).unwrap();
    }
    pub fn get_cost(&self, attacker_type: AttackerType) -> i32 {
        return self.get_stats(attacker_type).original_cost;
    }
    pub fn get_upgrade(&self, attacker_type: AttackerType, upgrade: UpgradeType) -> &UpgradeInfo {
        return self.upgrade_map.get(&(attacker_type, upgrade)).unwrap();
    }
    pub fn get_upgrade_cost(&self, attacker_type: AttackerType, upgrade: UpgradeType) -> i32 {
        return self.get_upgrade(attacker_type, upgrade).cost;
    }
    pub fn apply_upgrade(&mut self, attacker_type: AttackerType, upgrade: UpgradeType) {
        let stats = self.stats.get_mut(&attacker_type).unwrap();
        let upgrade_info = self.upgrade_map.get_mut(&(attacker_type, upgrade)).unwrap();
        upgrade_info.cost = (upgrade_info.cost as f32 * 1.3).round() as i32;
        match upgrade {
            UpgradeType::Amount => {
                stats.num_summoned = upgrade_info.apply_value(stats.num_summoned);
            }
            UpgradeType::Speed => {
                stats.movement_speed = upgrade_info.apply_value_f32(stats.movement_speed);
            },
            UpgradeType::Health => {
                stats.max_health = upgrade_info.apply_value_f32(stats.max_health);
                stats.health = upgrade_info.apply_value_f32(stats.health);
            },
        }
    }

}

impl Default for AttackerStats {
    fn default() -> Self {
        let mut stats: HashMap<AttackerType, Attacker> = HashMap::new();
        let mut upgrade_map: HashMap<(AttackerType, UpgradeType), UpgradeInfo> = HashMap::new();

        stats.insert(AttackerType::OrcWarrior, ORC_WARRIOR_STATS.clone());
        stats.insert(AttackerType::Spider, SPIDER_STATS.clone());
        stats.insert(AttackerType::Golem, GOLEM_STATS.clone());
        
        upgrade_map.insert((AttackerType::OrcWarrior, UpgradeType::Amount), UpgradeInfo { effect: 1., cost: 200, effect_type: UpgradeEffectType::Flat, description: "Increase spawn amount by 1".to_string() } );
        upgrade_map.insert((AttackerType::Spider, UpgradeType::Amount), UpgradeInfo { effect: 1., cost: 150, effect_type: UpgradeEffectType::Flat, description: "Increase spawn amount by 1".to_string() } );
        upgrade_map.insert((AttackerType::Golem, UpgradeType::Amount), UpgradeInfo { effect: 1., cost: 300, effect_type: UpgradeEffectType::Flat, description: "Increase spawn amount by 1".to_string() } );
        
        upgrade_map.insert((AttackerType::OrcWarrior, UpgradeType::Health), UpgradeInfo { effect: 1.2, cost: 120, effect_type: UpgradeEffectType::Factor, description: "Increase health by 10%".to_string() } );
        upgrade_map.insert((AttackerType::Spider, UpgradeType::Health), UpgradeInfo { effect: 1.2, cost: 150, effect_type: UpgradeEffectType::Factor, description: "Increase health by 20%".to_string() });
        upgrade_map.insert((AttackerType::Golem, UpgradeType::Health), UpgradeInfo { effect: 1.1, cost: 110, effect_type: UpgradeEffectType::Factor, description: "Increase health by 10%".to_string() });
        
        upgrade_map.insert((AttackerType::OrcWarrior, UpgradeType::Speed), UpgradeInfo { effect: 1.2, cost: 100, effect_type: UpgradeEffectType::Factor, description: "Increase speed by 20%".to_string() });
        upgrade_map.insert((AttackerType::Spider, UpgradeType::Speed), UpgradeInfo { effect: 1.2, cost: 200, effect_type: UpgradeEffectType::Factor, description: "Increase speed by 20%".to_string() } );
        upgrade_map.insert((AttackerType::Golem, UpgradeType::Speed), UpgradeInfo { effect: 1.2, cost: 100, effect_type: UpgradeEffectType::Factor, description: "Increase speed by 20%".to_string() } );

        return Self { stats: stats, upgrade_map: upgrade_map };
    }
}

#[derive(Component)]
pub struct Animations {
    up: AnimationIndices,
    down: AnimationIndices,
    left: AnimationIndices,
    right: AnimationIndices,
    idle: AnimationIndices,
}

impl Animations {
    pub fn get_animation(&self, velocity: Vec2) -> &AnimationIndices {
        if velocity.length() > 0.0 {
            // Check if we are travelling more up/down than left/right
            if f32::abs(velocity.x) < f32::abs(velocity.y) {
                return if velocity.y > 0. {
                    &self.up
                } else {
                    &self.down
                };
            } else {
                return if velocity.x > 0. {
                    &self.right
                } else {
                    &self.left
                };
            }
        }
        return &self.idle;
    }
}

pub struct AttackersPlugin;

impl Plugin for AttackersPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<AttackerStats>()
            .add_system(update_animations)
            .add_system(set_initial_pathfinding)
            .add_system(update_path_finding)
            .add_system(update_positions)
            .add_system(set_updated_pathfinding)
            .add_system(check_reached_end)
            /*.add_system(spawn_entities) */;
    }
}

fn update_animations(
    mut query: Query<(
        &Attacker,
        &Animations,
        &mut AnimationTimer,
        &mut TextureAtlasSprite,
    )>,
    time: Res<Time>,
) {
    for (attacker, animations, mut timer, mut sprite) in query.iter_mut() {
        timer.tick(time.delta());
        if timer.just_finished() {
            let index = sprite.index;
            let animation = animations.get_animation(attacker.velocity);
            if index > animation.end || index < animation.start {
                sprite.index = animation.start;
            } else {
                sprite.index = if sprite.index >= animation.end {
                    animation.start
                } else {
                    sprite.index + 1
                }
            }
        }
    }
}

fn set_initial_pathfinding(
    mut commands: Commands,
    query: Query<Entity, (Without<Flying>, Without<Path>, With<Attacker>)>,
    field: Res<TowerField>,
) {
    for entity in &query {
        match a_star(&field, field.get_start(), field.get_end()) {
            Some(path) => {
                commands.entity(entity).insert(path);
            }
            None => {}
        }
    }
}

fn set_updated_pathfinding(
    mut commands: Commands,
    mut field_modified: EventReader<FieldModified>,
    query: Query<(Entity, &Path), (Without<Flying>, With<Attacker>)>,
    field: Res<TowerField>,
) {
    if !field_modified.is_empty() {
        for (entity, path) in &query {
            let mut index = path.get_current_index();
            while index > 0 && field.is_node_blocked(path.get_node(index)) {
                index -= 1;
            }
            match a_star(&field, path.get_node(index), field.get_end()) {
                Some(path) => {
                    commands.entity(entity).insert(path);
                }
                None => {}
            }
        }
        field_modified.clear();
    }
}

fn check_reached_end(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &Attacker)>,
    mut reached_end: EventWriter<EntityReachedEnd>,
    tower_field: Res<TowerField>,
) {
    for (entity, mut transform, attacker) in query.iter_mut() {
        let goal = tower_field.get_end();
        let target_vec = Vec2::new(goal.x as f32, goal.y as f32) * SLOT_SIZE as f32;
        let entity_vec = transform.translation.truncate();
        if target_vec.distance(entity_vec) <= 5. {
            transform.translation = tower_field.get_start_transform().translation;
            commands.entity(entity).remove::<Path>();
            reached_end.send(EntityReachedEnd {
                entity: entity,
                bounty: attacker.bounty,
            })
        }
    }
}

fn update_path_finding(mut query: Query<(&mut Attacker, &mut Path, &Transform)>) {
    for (mut attacker, mut path, transform) in query.iter_mut() {
        let position = transform.translation.truncate();
        let mut target = path.get_target_position();
        let sizef = SLOT_SIZE as f32;
        if position.distance(target) < sizef / 4. {
            path.increment_index();
        }
        target = path.get_target_position();
        attacker.velocity = (target - position).normalize_or_zero() * attacker.movement_speed;
    }
}

fn update_positions(mut query: Query<(&Attacker, &mut Transform)>, time: Res<Time>) {
    for (attacker, mut transform) in query.iter_mut() {
        transform.translation += attacker.velocity.extend(0.) * time.delta_seconds();
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum AttackerType {
    OrcWarrior,
    Spider,
    Golem,
}

impl AttackerType {
    pub fn get_name(&self) -> &'static str {
        return match self {
            AttackerType::OrcWarrior => "Orc Warrior",
            AttackerType::Spider => "Spider",
            AttackerType::Golem => "Golem"
        };
    }
}

pub const ORC_WARRIOR_STATS: Attacker = Attacker {
    health: 140.,
    max_health: 140.,
    movement_speed: 26.,
    velocity: Vec2::ZERO,
    size: Vec2::new(26., 36.),
    bounty: 10,
    original_cost: 20,
    num_summoned: 1,
};
//pub const ORC_WARRIOR: AttackerType = AttackerType::OrcWarrior(ORC_WARRIOR_STATS);

pub const SPIDER_STATS: Attacker = Attacker {
    health: 56.,
    max_health: 56.,
    movement_speed: 51.,
    velocity: Vec2::ZERO,
    size: Vec2::new(14., 14.),
    bounty: 15,
    original_cost: 60,
    num_summoned: 3,
};
//pub const SPIDER: AttackerType = AttackerType::Spider(SPIDER_STATS);


pub const GOLEM_STATS: Attacker = Attacker {
    health: 400.,
    max_health: 400.,
    movement_speed: 13.,
    velocity: Vec2::ZERO,
    size: Vec2::new(47., 50.),
    bounty: 60,
    original_cost: 160,
    num_summoned: 1,
};

trait AttackerSpawner
where
    Self: Sized,
{
    fn spawn(field: &TowerField, textures: &TextureResource, preset: AttackerType, attackers: &AttackerStats) -> Vec<Self>;
}

fn fuzzy_transform(field: &TowerField) -> Transform {
    return field.get_start_transform_with_offset(Vec2::new(rand::thread_rng().gen_range(-16.0..16.0), rand::thread_rng().gen_range(-16.0..16.0)));
}

pub fn spawn_attacker(
    mut commands: Commands,
    field: &TowerField,
    textures: &TextureResource,
    preset: AttackerType,
    attackers: &AttackerStats
) {
    match preset {
        AttackerType::OrcWarrior => {
            for ele in OrcWarrior::spawn(field, textures, preset, attackers) {
                commands.spawn(ele);
            }
        }
        AttackerType::Spider => {
            for ele in Spider::spawn(field, textures, preset, attackers) {
                commands.spawn(ele);
            }
        },
        AttackerType::Golem => {
            for ele in Golem::spawn(field, textures, preset, attackers) {
                commands.spawn(ele);
            }
        }
    }
}

#[derive(Bundle)]
pub struct OrcWarrior {
    attacker: Attacker,
    grounded: Grounded,
    timer: AnimationTimer,
    animations: Animations,
    #[bundle]
    sprite: SpriteSheetBundle,
}

impl AttackerSpawner for OrcWarrior {
    fn spawn(field: &TowerField, textures: &TextureResource, preset: AttackerType, attackers: &AttackerStats) -> Vec<Self> {
        let animations = textures.get_animations(
            "orc1",
            [
                "orc1_down_walk",
                "orc1_left_walk",
                "orc1_right_walk",
                "orc1_up_walk",
                "orc1_idle",
            ],
        );
        return match preset {
            AttackerType::OrcWarrior => {
                let attacker = attackers.get_stats(preset);
                let mut results: Vec<Self> = Vec::new();
                for i in 0..attacker.num_summoned {
                    results.push(Self {
                        attacker: attacker.clone(),
                        animations: Animations {
                            up: animations.1[3],
                            down: animations.1[0],
                            left: animations.1[1],
                            right: animations.1[2],
                            idle: animations.1[4],
                        },
                        sprite: SpriteSheetBundle {
                            sprite: TextureAtlasSprite::new(animations.1[4].start),
                            texture_atlas: animations.0.clone_weak(),
                            transform: fuzzy_transform(field),
                            ..Default::default()
                        },
                        grounded: Grounded,
                        timer: AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
                    });
                }
                results
            }
            _ => panic!(),
        };
    }
}

#[derive(Bundle)]
pub struct Spider {
    attacker: Attacker,
    grounded: Grounded,
    timer: AnimationTimer,
    animations: Animations,
    #[bundle]
    sprite: SpriteSheetBundle,
}


impl AttackerSpawner for Spider {
    fn spawn(field: &TowerField, textures: &TextureResource, preset: AttackerType, attackers: &AttackerStats) -> Vec<Self> {
        let animations = textures.get_animations(
            "monster1",
            [
                "spider1_down_walk",
                "spider1_left_walk",
                "spider1_right_walk",
                "spider1_up_walk",
                "spider1_idle",
            ],
        );
        return match preset {
            AttackerType::Spider => {
                let attacker = attackers.get_stats(preset);
                let mut results: Vec<Self> = Vec::new();
                for i in 0..attacker.num_summoned {
                    results.push(Self {
                        attacker: attacker.clone(),
                        animations: Animations {
                            up: animations.1[3],
                            down: animations.1[0],
                            left: animations.1[1],
                            right: animations.1[2],
                            idle: animations.1[4],
                        },
                        sprite: SpriteSheetBundle {
                            sprite: TextureAtlasSprite::new(animations.1[4].start),
                            texture_atlas: animations.0.clone_weak(),
                            transform: fuzzy_transform(field),
                            ..Default::default()
                        },
                        grounded: Grounded,
                        timer: AnimationTimer(Timer::from_seconds(0.06, TimerMode::Repeating)),
                    })
                }
                results
            },
            _ => panic!(),
        };
    }
}


#[derive(Bundle)]
pub struct Golem {
    attacker: Attacker,
    grounded: Grounded,
    timer: AnimationTimer,
    animations: Animations,
    #[bundle]
    sprite: SpriteSheetBundle,
}


impl AttackerSpawner for Golem {
    fn spawn(field: &TowerField, textures: &TextureResource, preset: AttackerType, attackers: &AttackerStats) -> Vec<Self> {
        let animations = textures.get_animations(
            "golem1",
            [
                "golem1_down_walk",
                "golem1_left_walk",
                "golem1_right_walk",
                "golem1_up_walk",
                "golem1_idle",
            ],
        );
        return match preset {
            AttackerType::Golem => {
                let attacker = attackers.get_stats(preset);
                let mut results: Vec<Self> = Vec::new();
                for i in 0..attacker.num_summoned {
                    results.push(Self {
                        attacker: attacker.clone(),
                        animations: Animations {
                            up: animations.1[3],
                            down: animations.1[0],
                            left: animations.1[1],
                            right: animations.1[2],
                            idle: animations.1[4],
                        },
                        sprite: SpriteSheetBundle {
                            sprite: TextureAtlasSprite::new(animations.1[4].start),
                            texture_atlas: animations.0.clone_weak(),
                            transform: fuzzy_transform(field),
                            ..Default::default()
                        },
                        grounded: Grounded,
                        timer: AnimationTimer(Timer::from_seconds(0.3, TimerMode::Repeating)),
                    })
                }
                results
            },
            _ => panic!(),
        };
    }
}
