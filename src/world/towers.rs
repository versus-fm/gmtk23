use std::{f32::consts::PI, time::Duration};

use bevy::{
    prelude::{
        default, Added, App, Bundle, Commands, Component, Entity, EventReader, EventWriter, Handle,
        Plugin, Quat, Query, Rect, Res, ResMut, Resource, Transform, Vec2, Visibility, Without,
    },
    sprite::{SpriteSheetBundle, TextureAtlas, TextureAtlasSprite},
    time::{Time, Timer},
};
use serde::{Deserialize, Serialize};

use crate::{textures::TextureResource, particle::{spawn_large_explosion, spawn_blood_splatter, spawn_coin}};

use super::{
    attackers::{AnimationIndices, Attacker, Grounded},
    building_configuration::{BuildingConfig, BuildingResource, BuildingType, BuildingTypeConfig},
    events::{
        DamageEvent, FieldModified, KillEvent, RemoveStructureRequest, RemovedStructureEvent,
    },
    path_finding::{a_star, Node},
};

pub const SLOT_SIZE: usize = 64;

#[derive(Resource)]
pub struct TowerField {
    pub slots: Vec<FieldSlot>,
    pub field_transform: Vec2,
    width: usize,
    height: usize,
    start: Node,
    end: Node,
}

#[derive(Clone, Copy)]
pub struct FieldSlot {
    pub entity: Entity,
    pub blocked: bool,
    occupied: bool,
}

impl Default for FieldSlot {
    fn default() -> Self {
        return Self {
            entity: Entity::PLACEHOLDER,
            blocked: false,
            occupied: false,
        };
    }
}

impl TowerField {
    pub fn new(width: usize, height: usize, field_offset: Vec2, start: Node, end: Node) -> Self {
        let mut slots: Vec<FieldSlot> = Vec::with_capacity(width * height);
        for _ in 0..slots.capacity() {
            slots.push(Default::default());
        }
        return Self {
            slots,
            width,
            height,
            field_transform: field_offset,
            start,
            end,
        };
    }

    pub fn add_structure(&mut self, entity: Entity, blocking: bool, pos: Vec2) {
        let y = pos.y as usize / SLOT_SIZE;
        let x = pos.x as usize / SLOT_SIZE;
        let i = y * self.width + x;
        if i < self.slots.len() {
            self.slots[i] = FieldSlot {
                entity,
                blocked: blocking,
                occupied: true,
            };
        }
    }

    pub fn is_occupied(&self, x: usize, y: usize) -> bool {
        let i = y * self.width + x;
        if i < self.slots.len() {
            return self.slots[i].occupied;
        } else {
            return true;
        }
    }

    pub fn is_blocked(&self, x: usize, y: usize) -> bool {
        let i = y * self.width + x;
        if i < self.slots.len() {
            return self.slots[i].blocked;
        } else {
            return true;
        }
    }

    pub fn is_node_occupied(&self, node: Node) -> bool {
        if node.x < 0 || node.y < 0 {
            return true;
        }
        return self.is_occupied(node.x as usize, node.y as usize);
    }

    pub fn is_node_blocked(&self, node: Node) -> bool {
        if node.x < 0 || node.y < 0 {
            return true;
        }
        return self.is_blocked(node.x as usize, node.y as usize);
    }

    pub fn get_width(&self) -> usize {
        return self.width;
    }

    pub fn get_height(&self) -> usize {
        return self.height;
    }

    pub fn get_start(&self) -> Node {
        return self.start;
    }

    pub fn get_end(&self) -> Node {
        return self.end;
    }

    pub fn get_start_transform(&self) -> Transform {
        return Transform::from_xyz(
            (self.start.x as usize * SLOT_SIZE) as f32,
            (self.start.y as usize * SLOT_SIZE) as f32,
            1.,
        );
    }

    pub fn get_start_transform_with_offset(&self, offset: Vec2) -> Transform {
        return Transform::from_xyz(
            (self.start.x as usize * SLOT_SIZE) as f32 + offset.x,
            (self.start.y as usize * SLOT_SIZE) as f32 + offset.y,
            1.,
        );
    }

    pub fn get_end_transform(&self) -> Transform {
        return Transform::from_xyz(
            (self.end.x as usize * SLOT_SIZE) as f32,
            (self.end.y as usize * SLOT_SIZE) as f32,
            1.,
        );
    }

    pub fn get_slot(&self, node: Node) -> Option<FieldSlot> {
        let i = node.y as usize * self.width + node.x as usize;
        if i < self.slots.len() {
            return Some(self.slots[i]);
        } else {
            return None;
        }
    }

    pub fn clear_slot(&mut self, node: Node) {
        let i = node.y as usize * self.width + node.x as usize;
        if i < self.slots.len() {
            self.slots[i].occupied = false;
            self.slots[i].entity = Entity::PLACEHOLDER;
            self.slots[i].blocked = false;
        }
    }

    pub fn distance_to_start(&self, node: Node) -> f32 {
        return Vec2::new(node.x as f32, node.y as f32)
            .distance(Vec2::new(self.start.x as f32, self.end.y as f32));
    }
}

#[derive(Component)]
pub struct Structure {
    pub building_type: BuildingType,
    pub blocking: bool,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum DamageType {
    Magic,
    Piercing,
    Crushing,
    Explosive,
}

#[derive(Deserialize, Serialize, Clone)]
pub enum ProjectileSprite {
    Static {
        name: String,
        index: usize,
        size: Vec2,
    },
    Animated {
        name: String,
        animation_name: String,
        animation: AnimationIndices,
        timer: Timer,
        size: Vec2,
    },
}

pub enum Target {
    Entity(Entity),
    Ground(Vec2),
}

pub enum ProjectileMotion {
    Fixed(Duration, Vec2),
    Velocity(f32),
    FixedArc(Duration, f32, Vec2),
}

#[derive(Component)]
pub struct Projectile {
    pub target: Target,
    pub source: Entity,
    pub projectile_motion: ProjectileMotion,
    pub damage: f32,
    pub damage_type: DamageType,
    pub splash_radius: f32,
    pub velocity: Vec2,
    pub size: Vec2,
    pub dead: bool,
    pub age: Duration,
}

trait SpriteProvider {
    fn get_sprite(&self, textures: &TextureResource)
        -> (&Handle<TextureAtlas>, TextureAtlasSprite);
}

impl ProjectileSprite {
    fn get_sprite<'a>(
        &'a self,
        textures: &'a TextureResource,
    ) -> (&Handle<TextureAtlas>, TextureAtlasSprite) {
        return match self {
            ProjectileSprite::Static { name, index, size } => textures.get_sprite(&name, *index),
            ProjectileSprite::Animated {
                name,
                animation_name,
                animation,
                timer,
                size,
            } => {
                let animation = textures.get_animation(&name, &animation_name);
                (animation.0, TextureAtlasSprite::new(animation.1.start))
            }
        };
    }
    fn get_size(&self) -> Vec2 {
        return match self {
            ProjectileSprite::Static { name, index, size } => *size,
            ProjectileSprite::Animated {
                name,
                animation_name,
                animation,
                timer,
                size,
            } => *size,
        };
    }
}

#[derive(Deserialize, Serialize)]
pub enum DefenderAttack {
    Projectile {
        damage_type: DamageType,
        damage: f32,
        projectile_speed: f32,
        sprite: ProjectileSprite,
    },
    Splash {
        damage_type: DamageType,
        damage: f32,
        travel_time: f32,
        splash_radius: f32,
        sprite: ProjectileSprite,
    },
}

pub enum TargetingStrategy {
    LeastHealth,
    ClosestGoal,
    Random,
}

#[derive(Component)]
pub struct Defender {
    pub attack_timer: Timer,
    pub attack: DefenderAttack,
    pub attack_range: f32,
    pub kill_count: usize,
    pub pending_attack: bool,
}

pub struct TowersPlugin;

impl Plugin for TowersPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(register_structures)
            .add_system(find_targets)
            .add_system(update_projectiles)
            .add_system(process_removal_requests)
            .add_system(update_projectile_motion)
            .add_system(spawn_coin_particle_on_death)
            .add_system(lost_targets);
    }
}

fn register_structures(
    query: Query<(Entity, &Structure, &Transform), Added<Structure>>,
    mut field: ResMut<TowerField>,
    mut modified_field: EventWriter<FieldModified>,
) {
    for (e, structure, transform) in &query {
        field.add_structure(e, structure.blocking, transform.translation.truncate())
    }
    if !query.is_empty() {
        modified_field.send(FieldModified);
    }
}

fn process_removal_requests(
    mut commands: Commands,
    mut field: ResMut<TowerField>,
    mut modified_field: EventWriter<FieldModified>,
    mut removed: EventWriter<RemovedStructureEvent>,
    mut requests: EventReader<RemoveStructureRequest>,
    query: Query<(Entity, &Structure)>,
) {
    for ev in requests.iter() {
        if let Some(slot) = field.get_slot(ev.node) {
            field.clear_slot(ev.node);
            if let Ok(entity) = query.get(slot.entity) {
                removed.send(RemovedStructureEvent {
                    node: ev.node,
                    building_type: entity.1.building_type,
                });
                commands.entity(entity.0).despawn();
            }
            modified_field.send(FieldModified);
        }
    }
}

fn find_targets(
    mut commands: Commands,
    mut towers: Query<(Entity, &mut Defender, &Transform)>,
    enemies: Query<(Entity, &Attacker, &Transform)>,
    textures: Res<TextureResource>,
    time: Res<Time>,
) {
    for (entity, mut defender, transform) in towers.iter_mut() {
        defender.attack_timer.tick(time.delta());
        if defender.attack_timer.just_finished() {
            defender.pending_attack = true;
        }

        if defender.pending_attack {
            // TODO: Implement Target strategy
            let maybe_target = enemies
                .iter()
                .filter(|e| {
                    e.2.translation
                        .truncate()
                        .distance(transform.translation.truncate())
                        <= defender.attack_range
                })
                .min_by(|a, b| a.1.health.total_cmp(&b.1.health))
                .take();
            if let Some(target) = maybe_target {
                defender.pending_attack = false;
                match &defender.attack {
                    DefenderAttack::Projectile {
                        damage_type,
                        damage,
                        projectile_speed,
                        sprite,
                    } => {
                        let sprite_details = sprite.get_sprite(&textures);
                        commands.spawn(ProjectileBundle {
                            projectile: Projectile {
                                damage: *damage,
                                target: Target::Entity(target.0),
                                source: entity,
                                projectile_motion: ProjectileMotion::Velocity(*projectile_speed),
                                damage_type: *damage_type,
                                splash_radius: 0.,
                                velocity: Vec2::ZERO,
                                size: sprite.get_size(),
                                dead: false,
                                age: Duration::ZERO,
                            },
                            sprite: SpriteSheetBundle {
                                sprite: sprite_details.1,
                                texture_atlas: sprite_details.0.clone_weak(),
                                transform: Transform::from_translation(transform.translation),
                                ..Default::default()
                            },
                        });
                    }
                    DefenderAttack::Splash {
                        damage_type,
                        damage,
                        travel_time,
                        splash_radius,
                        sprite,
                    } => {
                        let sprite_details = sprite.get_sprite(&textures);
                        commands.spawn(ProjectileBundle {
                            projectile: Projectile {
                                damage: *damage,
                                target: Target::Ground(target.2.translation.truncate()),
                                source: entity,
                                projectile_motion: ProjectileMotion::FixedArc(
                                    Duration::from_secs_f32(*travel_time),
                                    34.,
                                    transform.translation.truncate()
                                ),
                                damage_type: *damage_type,
                                splash_radius: *splash_radius,
                                velocity: Vec2::ZERO,
                                size: sprite.get_size(),
                                dead: false,
                                age: Duration::ZERO,
                            },
                            sprite: SpriteSheetBundle {
                                sprite: sprite_details.1,
                                texture_atlas: sprite_details.0.clone_weak(),
                                transform: Transform::from_translation(transform.translation),
                                ..Default::default()
                            },
                        });
                    }
                }
            }
        }
    }
}

fn update_projectile_motion(
    mut commands: Commands,
    mut projectiles: Query<(Entity, &mut Projectile, &mut Transform), Without<Attacker>>,
    mut enemies: Query<(Entity, &mut Attacker, &Transform), Without<Projectile>>,
    time: Res<Time>,
) {
    for (entity, mut projectile, mut transform) in projectiles.iter_mut() {
        projectile.age += time.delta();
        if projectile.age.as_secs_f32() < 20. {
            let maybe_target_pos: Option<Vec2> = match projectile.target {
                Target::Entity(entity) => enemies
                    .get_component::<Transform>(entity)
                    .ok()
                    .map(|transform| transform.translation.truncate()),
                Target::Ground(pos) => Some(pos),
            };
            if let Some(target_pos) = maybe_target_pos {
                match &projectile.projectile_motion {
                    ProjectileMotion::Velocity(speed) => {
                        let projectile_pos = transform.translation.truncate();
                        let direction = (target_pos - projectile_pos).normalize_or_zero();
                        projectile.velocity = direction * *speed;
                        transform.translation +=
                            projectile.velocity.extend(0.) * time.delta_seconds();
                        let angle = f32::atan2(
                            target_pos.y - projectile_pos.y,
                            target_pos.x - projectile_pos.x,
                        );
                        transform.rotation = Quat::from_rotation_z(angle - PI / 4.);
                    }
                    ProjectileMotion::Fixed(duration, start_pos) => {
                        let projectile_pos = transform.translation.truncate();
                        let factor =
                            (projectile.age.as_secs_f32() / duration.as_secs_f32()).clamp(0., 1.);
                        transform.translation = start_pos.lerp(target_pos, factor).extend(transform.translation.z);
                        let angle = f32::atan2(
                            target_pos.y - projectile_pos.y,
                            target_pos.x - projectile_pos.x,
                        );
                        transform.rotation = Quat::from_rotation_z(angle - PI / 4.);
                    }
                    ProjectileMotion::FixedArc(duration, arc, start_pos) => {
                        let projectile_pos = transform.translation.truncate();
                        let factor =
                            (projectile.age.as_secs_f32() / duration.as_secs_f32()).clamp(0., 1.);
                        let new_pos = start_pos.lerp(target_pos, factor).extend(transform.translation.z);
                        transform.translation = new_pos;
                        let angle = f32::atan2(
                            target_pos.y - projectile_pos.y,
                            target_pos.x - projectile_pos.x,
                        );
                        transform.rotation = Quat::from_rotation_z(angle - PI / 4.);
                    }
                }
            } else {
            }
        } else {
            commands.entity(entity).despawn();
        }
    }
}

fn lost_targets(
    mut commands: Commands,
    mut projectiles: Query<(Entity, &mut Projectile), Without<Attacker>>,
    mut kill_events: EventReader<KillEvent>,
) {
    for ev in kill_events.iter() {
        for (entity, mut projectile) in projectiles.iter_mut() {
            match projectile.target {
                Target::Entity(target) => {
                    if target.index() == ev.target.index() {
                        projectile.target = Target::Ground(ev.death_position);
                    }
                },
                _ => {}
            }
        }
    }
}

fn update_projectiles(
    mut commands: Commands,
    mut enemies: Query<(Entity, &mut Attacker, &Transform), Without<Projectile>>,
    mut projectiles: Query<(Entity, &mut Projectile, &mut Transform), Without<Attacker>>,
    mut damage_events: EventWriter<DamageEvent>,
    mut kill_events: EventWriter<KillEvent>,
    textures: Res<TextureResource>,
    time: Res<Time>,
) {
    for (entity, mut projectile, mut transform) in projectiles.iter_mut() {
        if projectile.dead {
            continue;
        }
        match projectile.target {
            Target::Entity(target_entity) => match enemies.get_mut(target_entity) {
                Ok(mut target) => {
                    let target_rect = Rect::new(
                        target.2.translation.x,
                        target.2.translation.y,
                        target.2.translation.x + target.1.size.x,
                        target.2.translation.y + target.1.size.y,
                    );
                    let projectile_rect = Rect::new(
                        transform.translation.x,
                        transform.translation.y,
                        transform.translation.x + projectile.size.x,
                        transform.translation.y + projectile.size.y,
                    );
                    if !target_rect.intersect(projectile_rect).is_empty() {
                        let damage = calculate_damage(&projectile, &target.1);
                        target.1.health -= damage;
                        damage_events.send(DamageEvent {
                            amount: damage,
                            target: target.0,
                        });
                        spawn_blood_splatter(&mut commands, &target.2.clone(), &textures);
                        if target.1.health <= 0. {
                            kill_events.send(KillEvent {
                                target: target.0,
                                source: entity,
                                bounty: target.1.bounty,
                                original_cost: target.1.original_cost,
                                group_size: target.1.num_summoned,
                                death_position: target.2.translation.truncate(),
                            });
                            commands.entity(target.0).despawn();
                        }
                        projectile.dead = true;
                        commands.entity(entity).despawn();
                    }
                }
                Err(_) => {}
            },
            Target::Ground(pos) => {
                let projectile_pos = transform.translation.truncate();
                if projectile_pos.distance(pos) < 4. {
                    if projectile.splash_radius > 0. {
                        let enemies_to_damage: Vec<(
                            Entity,
                            bevy::prelude::Mut<'_, Attacker>,
                            &Transform,
                        )> = enemies
                            .iter_mut()
                            .filter(|e| {
                                e.2.translation.truncate().distance(pos) <= projectile.splash_radius
                            })
                            .collect();
                        for mut target in enemies_to_damage {
                            let damage = calculate_damage(&projectile, &target.1);
                            target.1.health -= damage;
                            damage_events.send(DamageEvent {
                                amount: damage,
                                target: target.0,
                            });
                            if target.1.health <= 0. {
                                kill_events.send(KillEvent {
                                    target: target.0,
                                    source: entity,
                                    bounty: target.1.bounty,
                                    original_cost: target.1.original_cost,
                                    group_size: target.1.num_summoned,
                                    death_position: target.2.translation.truncate(),
                                });
                                commands.entity(target.0).despawn();
                            }
                        }
                        spawn_large_explosion(&mut commands, &Transform::from_translation(pos.extend(transform.translation.z)), &textures);
                    }
                    projectile.dead = true;
                    commands.entity(entity).despawn();
                }
            }
        }
    }
}

fn calculate_damage(projectile: &Projectile, attacker: &Attacker) -> f32 {
    return projectile.damage;
}

fn spawn_coin_particle_on_death(
    mut commands: Commands,
    mut kill_events: EventReader<KillEvent>,
    textures: Res<TextureResource>,
) {
    for ev in kill_events.iter() {
        spawn_coin(&mut commands, &Transform::from_translation(ev.death_position.extend(20.)), &textures);
    }
}

#[derive(Bundle)]
pub struct ProjectileBundle {
    projectile: Projectile,
    #[bundle]
    sprite: SpriteSheetBundle,
}

pub trait StructureBuilder {
    fn from_tower_field(
        defenders: &BuildingResource,
        tower_field: &TowerField,
        named_textures: &TextureResource,
        x: usize,
        y: usize,
    ) -> Self;
}

#[derive(Bundle)]
pub struct WallBundle {
    structure: Structure,
    #[bundle]
    sprite: SpriteSheetBundle,
}

impl StructureBuilder for WallBundle {
    fn from_tower_field(
        defenders: &BuildingResource,
        tower_field: &TowerField,
        named_textures: &TextureResource,
        x: usize,
        y: usize,
    ) -> Self {
        let sprite = named_textures.get_sprite("towers", 0);
        return Self {
            structure: Structure {
                blocking: true,
                building_type: BuildingType::Wall,
            },
            sprite: SpriteSheetBundle {
                sprite: sprite.1,
                texture_atlas: sprite.0.clone_weak(),
                transform: Transform::from_xyz(
                    (x * SLOT_SIZE) as f32 + tower_field.field_transform.x,
                    (y * SLOT_SIZE) as f32 + tower_field.field_transform.y,
                    10. + (tower_field.height - y) as f32 / tower_field.height as f32,
                ),
                ..default()
            },
        };
    }
}

#[derive(Bundle)]
pub struct ArrowTower {
    structure: Structure,
    defender: Defender,
    grounded: Grounded,
    #[bundle]
    sprite: SpriteSheetBundle,
}

impl StructureBuilder for ArrowTower {
    fn from_tower_field(
        defenders: &BuildingResource,
        tower_field: &TowerField,
        named_textures: &TextureResource,
        x: usize,
        y: usize,
    ) -> Self {
        let tower_sprite = named_textures.get_sprite("towers", 4);
        let config = defenders.get_building_config(&BuildingType::Arrow).unwrap();
        match &config.type_config {
            BuildingTypeConfig::Defender {
                attack_timer,
                attack,
                attack_range,
            } => match attack {
                DefenderAttack::Projectile {
                    damage_type,
                    damage,
                    projectile_speed,
                    sprite,
                } => {
                    return Self {
                        structure: Structure {
                            blocking: config.blocking,
                            building_type: BuildingType::Arrow,
                        },
                        sprite: SpriteSheetBundle {
                            sprite: tower_sprite.1,
                            texture_atlas: tower_sprite.0.clone_weak(),
                            transform: Transform::from_xyz(
                                (x * SLOT_SIZE) as f32 + tower_field.field_transform.x,
                                (y * SLOT_SIZE) as f32 + tower_field.field_transform.y,
                                10. + (tower_field.height - y) as f32 / tower_field.height as f32,
                            ),
                            ..default()
                        },
                        defender: Defender {
                            attack_timer: Timer::from_seconds(
                                *attack_timer,
                                bevy::time::TimerMode::Repeating,
                            ),
                            attack: DefenderAttack::Projectile {
                                damage_type: *damage_type,
                                damage: *damage,
                                projectile_speed: *projectile_speed,
                                sprite: sprite.clone(),
                            },
                            kill_count: 0,
                            attack_range: *attack_range,
                            pending_attack: false,
                        },
                        grounded: Grounded,
                    }
                }
                _ => panic!(),
            },
            BuildingTypeConfig::Wall => panic!(),
        }
    }
}

#[derive(Bundle)]
pub struct CannonTower {
    structure: Structure,
    defender: Defender,
    grounded: Grounded,
    #[bundle]
    sprite: SpriteSheetBundle,
}

impl StructureBuilder for CannonTower {
    fn from_tower_field(
        defenders: &BuildingResource,
        tower_field: &TowerField,
        named_textures: &TextureResource,
        x: usize,
        y: usize,
    ) -> Self {
        let tower_sprite = named_textures.get_sprite("towers", 1);
        let config = defenders
            .get_building_config(&BuildingType::Cannon)
            .unwrap();
        match &config.type_config {
            BuildingTypeConfig::Defender {
                attack_timer,
                attack,
                attack_range,
            } => match attack {
                DefenderAttack::Splash {
                    damage_type,
                    damage,
                    travel_time,
                    sprite,
                    splash_radius,
                } => {
                    return Self {
                        structure: Structure {
                            blocking: config.blocking,
                            building_type: BuildingType::Cannon,
                        },
                        sprite: SpriteSheetBundle {
                            sprite: tower_sprite.1,
                            texture_atlas: tower_sprite.0.clone_weak(),
                            transform: Transform::from_xyz(
                                (x * SLOT_SIZE) as f32 + tower_field.field_transform.x,
                                (y * SLOT_SIZE) as f32 + tower_field.field_transform.y,
                                10. + (tower_field.height - y) as f32 / tower_field.height as f32,
                            ),
                            ..default()
                        },
                        defender: Defender {
                            attack_timer: Timer::from_seconds(
                                *attack_timer,
                                bevy::time::TimerMode::Repeating,
                            ),
                            attack: DefenderAttack::Splash {
                                damage_type: *damage_type,
                                damage: *damage,
                                splash_radius: *splash_radius,
                                travel_time: *travel_time,
                                sprite: sprite.clone(),
                            },
                            kill_count: 0,
                            attack_range: *attack_range,
                            pending_attack: false,
                        },
                        grounded: Grounded,
                    }
                }
                _ => panic!(),
            },
            BuildingTypeConfig::Wall => panic!(),
        }
    }
}
