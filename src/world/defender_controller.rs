use std::{marker::PhantomData, time::Duration, hash::Hash};
use rand::Rng;

use bevy::{prelude::{Plugin, App, Component, Resource, Commands, ResMut, Res, EventReader, Local, Query, Transform, IntoSystemConfig, Vec3}, time::{Timer, Time}, utils::{HashSet, HashMap}};


use crate::textures::TextureResource;

use super::{towers::{StructureBuilder, WallBundle, TowerField, ArrowTower, Defender, SLOT_SIZE, Structure, CannonTower}, building_configuration::{BuildingType, BuildingResource, BuildingConfig}, events::{RoundOverEvent, KillEvent, EntityReachedEnd, RoundStartEvent, DamageEvent, FieldModified, RemovedStructureEvent}, attackers::Attacker, path_finding::{a_star, Path, Node, a_star_with_blocked_node, get_successors, get_self_with_successors, get_all_neighbors}};

#[derive(Debug)]
struct WeightedNode {
    node: Node,
    weight: f32
}

#[derive(Resource)]
struct Buildings {
    presets: HashMap<BuildingType, BuildingPreset>
}

impl Buildings {
    pub fn get_preset(&self, building_type: BuildingType) -> &BuildingPreset {
        return self.presets.get(&building_type).unwrap();
    }
}

impl Default for Buildings {
    fn default() -> Self {
        Self { presets: HashMap::new() }
    }
}

#[derive(Resource)]
pub struct ResourceStore {
    pub gold: i32,
    pub lives: i32
}

#[derive(Resource)]
pub struct DefenderConfiguration {
    pub action_cooldown: Timer,
    pub wall_weight: f32,
    pub damage_weight: f32,
    pub sell_weight: f32,
    pub estimated_damage_needed: f32,
    pub estimated_damage_potential: f32,
    pub path_length: f32,
    pub path_distance: f32,
    pub path: Path,
    pub path_hash: HashSet<Node>,
    pub can_build_wall: bool,
    pub can_build_tower: bool,
    pub num_defenders: i32,
    pub num_walls: i32,
    sell_values: Vec<WeightedNode>
}

impl DefenderConfiguration {
    pub fn is_node_adjacent_to_or_on_path(&self, node: Node) -> bool {
        let x = node.x;
        let y = node.y;
        return self.path_hash.contains(&node) ||
                self.path_hash.contains(&Node::new(x + 1, y)) ||
                self.path_hash.contains(&Node::new(x - 1, y)) ||
                self.path_hash.contains(&Node::new(x, y + 1)) ||
                self.path_hash.contains(&Node::new(x, y - 1)) ||
                self.path_hash.contains(&Node::new(x + 1, y + 1)) ||
                self.path_hash.contains(&Node::new(x - 1, y - 1)) ||
                self.path_hash.contains(&Node::new(x + 1, y - 1)) ||
                self.path_hash.contains(&Node::new(x - 1, y + 1));
    }

    pub fn get_wall_factor(&self) -> f32 {
        if self.num_walls == 0 {
            return 1.;
        } else {
            return 1. + self.num_walls as f32 / self.num_defenders as f32;
        }
    }
}

#[derive(Resource)]
pub struct RoundStats {
    pub damage_dealt: f32,
    pub round_duration: Duration,
    pub num_reached_end: i32,
    pub closest_distance_to_end: f32,
    pub num_killed: i32
}

pub struct BuildingPreset {
    building_type: BuildingType,
    dps: f32,
    aoe: bool,
    cost: i32,
    blocking: bool,
}

impl BuildingPreset {
    pub fn new(building_type: BuildingType, cost: i32, blocking: bool, aoe: bool, dps: f32) -> Self {
        return Self { cost, blocking, building_type, aoe, dps };
    }
    pub fn spawn(&self, mut commands: Commands, defenders: &BuildingResource, tower_field: &TowerField, named_textures: &TextureResource, x: usize, y: usize) {
        match self.building_type {
            BuildingType::Arrow => {
                commands.spawn(ArrowTower::from_tower_field(defenders, tower_field, named_textures, x, y));
            },
            BuildingType::Wall => {
                commands.spawn(WallBundle::from_tower_field(defenders, tower_field, named_textures, x, y));
            },
            BuildingType::Cannon => {
                commands.spawn(CannonTower::from_tower_field(defenders, tower_field, named_textures, x, y));
            }
        }
    }
}

pub struct DefenderController;

impl Plugin for DefenderController {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<Buildings>()
            .insert_resource(DefenderConfiguration {
                action_cooldown: Timer::from_seconds(1.5, bevy::time::TimerMode::Repeating),
                damage_weight: 1.4,
                estimated_damage_needed: 1000.,
                wall_weight: 1.0,
                sell_weight: 1.0,
                path_length: 0.,
                path_distance: 0.,
                path: Path::empty(),
                path_hash: HashSet::new(),
                estimated_damage_potential: 0.,
                sell_values: Vec::new(),
                can_build_wall: true,
                can_build_tower: true,
                num_defenders: 0,
                num_walls: 0
            })
            .insert_resource(ResourceStore {gold: 200, lives: 50})
            .insert_resource(RoundStats {
                damage_dealt: 0.,
                round_duration: Duration::from_secs(0),
                closest_distance_to_end: 0.,
                num_reached_end: 0,
                num_killed: 0
            })
            .add_startup_system(setup)
            .add_system(collect_event_stats)
            .add_system(inspect_enemies)
            .add_system(perform_an_action)
            .add_system(listen_removals)
            .add_system(listen_kills)
            .add_system(listen_goals);
    }
}

fn setup(
    mut res: ResMut<Buildings>,
    buildings: Res<BuildingResource>
) {
    if let Some(preset) = create_preset(&buildings, BuildingType::Arrow) { res.presets.insert(preset.building_type, preset); }
    if let Some(preset) = create_preset(&buildings, BuildingType::Wall) { res.presets.insert(preset.building_type, preset); }
    if let Some(preset) = create_preset(&buildings, BuildingType::Cannon) { res.presets.insert(preset.building_type, preset); }
}

fn create_preset(buildings: &BuildingResource, building_type: BuildingType) -> Option<BuildingPreset> {
    return buildings.get_building_config(&building_type).map(|config| {
        BuildingPreset::new(
            building_type,
            config.get_cost(), 
            config.get_blocking(), 
            config.is_aoe(), 
            config.get_dps()
        )
    });
}

fn collect_event_stats(
    mut resource: ResMut<ResourceStore>,
    mut round_end: EventReader<RoundOverEvent>,
    mut round_start: EventReader<RoundStartEvent>,
    mut damage: EventReader<DamageEvent>,
    mut deaths: EventReader<KillEvent>,
    mut reached_end: EventReader<EntityReachedEnd>,
    mut stats: ResMut<RoundStats>,
    mut config: ResMut<DefenderConfiguration>,
    mut round_active: Local<bool>,
    field: Res<TowerField>,
    time: Res<Time>
) {
    if !round_end.is_empty() {
        config.estimated_damage_needed = stats.damage_dealt * 1.10;
        *round_active = false;
        round_end.clear();
    }

    if !round_start.is_empty() {
        let actual_distance = field.get_start_transform().translation.truncate().distance(field.get_end_transform().translation.truncate());
        stats.damage_dealt = 0.;
        stats.closest_distance_to_end = actual_distance;
        stats.num_reached_end = 0;
        stats.round_duration = Duration::ZERO;
        *round_active = true;
        round_start.clear();
    }

    if *round_active {
        for _ in deaths.iter() {
            stats.num_killed += 1;
        }
        for _ in reached_end.iter() {
            stats.num_reached_end += 1;
        }
        for ev in damage.iter() {
            stats.damage_dealt += ev.amount;
        }
        stats.round_duration = stats.round_duration + time.delta();
    }
}

fn inspect_enemies(
    query: Query<(&Attacker, &Transform)>,
    mut stats: ResMut<RoundStats>,
    field: Res<TowerField>
) {
    for (attacker, transform) in &query {
        let distance = transform.translation.truncate().distance(field.get_end_transform().translation.truncate());
        if distance < stats.closest_distance_to_end {
            stats.closest_distance_to_end = distance;
        }
    }
}

fn listen_kills(
    mut resources: ResMut<ResourceStore>,
    mut deaths: EventReader<KillEvent>
) {
    for ev in deaths.iter() {
        resources.gold += ev.bounty;
    }
}

fn listen_goals(
    mut resources: ResMut<ResourceStore>,
    mut goals: EventReader<EntityReachedEnd>
) {
    for ev in goals.iter() {
        resources.lives -= 1;
    }
}

fn listen_removals(
    mut removals: EventReader<RemovedStructureEvent>,
    mut resources: ResMut<ResourceStore>,
    buildings: Res<BuildingResource>
) {
    for ev in removals.iter() {
        resources.gold += buildings.get_cost(&ev.building_type) / 2;
    }
}

fn perform_an_action(
    field: Res<TowerField>,
    building_config: Res<BuildingResource>,
    presets: Res<Buildings>,
    textures: Res<TextureResource>,
    mut resources: ResMut<ResourceStore>,
    commands: Commands,
    mut defender_config: ResMut<DefenderConfiguration>,
    mut stats: ResMut<RoundStats>,
    /* Map for how many adjacent path nodes there are for every slot on the map. Used for placing towers on corners */
    mut adjacency_field: Local<HashMap<Node, i32>>,
    mut builds: EventReader<FieldModified>,
    mut initialized: Local<bool>,
    mut next_tower: Local<Option<BuildingType>>,
    query: Query<(&Structure, &Defender, &Transform)>,
    time: Res<Time>
) {
    if !builds.is_empty() || !*initialized {
        let actual_distance = field.get_start_transform().translation.truncate().distance(field.get_end_transform().translation.truncate());
        if let Some(path) = a_star(&field, field.get_start(), field.get_end()) {
            defender_config.path_hash.clear();
            for node in path.get_nodes() {
                defender_config.path_hash.insert(node);
            }
            defender_config.path_length = path.get_size() as f32;
            defender_config.path = path;
        }
        defender_config.path_distance = actual_distance;
        stats.closest_distance_to_end = actual_distance;

        adjacency_field.clear();
        for x in 0..field.get_width() as i32 {
            for y in 0..field.get_height() as i32 {
                let this_node = Node::new(x, y);
                if defender_config.path_hash.contains(&this_node) {
                    continue;
                }
                let mut adjacent = 0;
                for node in get_all_neighbors(this_node) {
                    if defender_config.path_hash.contains(&node) {
                        adjacent += 1;
                    }
                    /*if field.is_node_occupied(node) {
                        adjacent += 1;
                    }*/
                }
                adjacency_field.insert(this_node, adjacent);
            }
        }

        defender_config.estimated_damage_potential = 0.;
        // Roughly estimate total damage potential
        for (structure, defender, transform) in &query {
            let defender_pos = transform.translation.truncate() / SLOT_SIZE as f32;
            let defender_node = Node::new(defender_pos.x as i32, defender_pos.y as i32);
            let adjacent = (adjacency_field.get(&defender_node).copied().unwrap_or(0) as f32 * 0.4).max(1.);
            // Assume the average enemy speed, likely incorrect, but probably good enough
            let speed: f32 = 40.;
            let time_to_travel = defender.attack_range / speed;
            let dps = building_config.get_dps(&structure.building_type);
            //println!("DPS: {}, TTT: {}, Adjacency: {}, Attack Range: {}", dps, time_to_travel, adjacent, defender.attack_range);
            // Rough estimation using dps, time_to_travel in seconds, and a bonus for adjacent path nodes
            defender_config.estimated_damage_potential += dps * time_to_travel * adjacent;

            // Estimate the value of selling a tower by how many nodes in the current path it can reach
            let mut sell_value = 1.;
            let min_x = (defender_pos.x - defender.attack_range / SLOT_SIZE as f32).floor() as i32;
            let max_x = (defender_pos.x + defender.attack_range / SLOT_SIZE as f32).ceil() as i32;
            let min_y = (defender_pos.y - defender.attack_range / SLOT_SIZE as f32).floor() as i32;
            let max_y = (defender_pos.y + defender.attack_range / SLOT_SIZE as f32).ceil() as i32;
            for x in min_x..=max_x {
                for y in min_y..=max_y {
                    if defender_config.path_hash.contains(&Node::new(x, y)) {
                        sell_value -= 0.1;
                    }
                }
            }

            
            let mut index = -1;
            let mut found = false;
            for i in 0..defender_config.sell_values.len() {
                if defender_config.sell_values[i].node == defender_node {
                    index = i as i32;
                    found = true;
                    break;
                }
            }
            if found {
                defender_config.sell_values[index as usize].weight = sell_value;
            } else {
                defender_config.sell_values.push(WeightedNode { node: defender_node, weight: sell_value });
            }
        }

        defender_config.sell_values.sort_by(|a, b| a.weight.total_cmp(&b.weight));

        builds.clear();
        *initialized = true;
    }




    defender_config.action_cooldown.tick(time.delta());
    if defender_config.action_cooldown.just_finished() {

        if next_tower.is_none() {
            *next_tower = Some(if rand::thread_rng().gen_ratio(1, 7) {BuildingType::Cannon} else {BuildingType::Arrow})
        }
        //println!("Next tower will be {:?}", next_tower);

        let distance_factor = if defender_config.path_distance != 0. {
            stats.closest_distance_to_end / defender_config.path_distance
        } else {
            1.
        } + 1.;
        // How far above (or below) estimated damage needed are we.
        // If all slots are occupied on the map without disrupting path_finding we multiply the score by a large constant
        let wall_score = ((defender_config.estimated_damage_potential / defender_config.estimated_damage_needed)) * if defender_config.can_build_wall { 
            1. 
        } else { 
            -1000. 
        } * (distance_factor * 0.5) / (defender_config.get_wall_factor() * 0.2).max(1.) * defender_config.wall_weight;
        // How far below (or above) estimated damage needed are we, essentially the inverse of wall_score
        let defender_score = (1. - (defender_config.estimated_damage_potential / defender_config.estimated_damage_needed)).max(1.) * if defender_config.can_build_tower { 
            1. 
        } else { 
            -1000. 
        } * distance_factor * (defender_config.get_wall_factor() * 0.2).max(1.) * defender_config.damage_weight;
        let best_sell_score = defender_config.sell_values.last().map(|e| e.weight).unwrap_or(0.) * defender_config.sell_weight;

        /*println!("Current scores: Wall ({}), Defender ({}), Sell ({}); Distance factor: {}; Wall factor: {}; Damage Factor: {}", 
            wall_score, 
            defender_score, 
            best_sell_score,
            distance_factor, 
            defender_config.get_wall_factor(),
            (defender_config.estimated_damage_potential / defender_config.estimated_damage_needed)
        );*/

        let best_score = max_index([wall_score, defender_score]);
        if best_score == 0 {
            // wall_score
            let potential_walls = get_wall_build_actions::<5, 10>(&field, &defender_config);
            if potential_walls.is_empty() {
                defender_config.can_build_wall = false;
            } else {
                let weighted_node = &potential_walls[rand::thread_rng().gen_range(0..potential_walls.len())];
                if buy_structure(commands, &mut resources, &textures, &field, &presets, &building_config, BuildingType::Wall, weighted_node.node) {
                    defender_config.num_walls += 1;
                }
            }
        } else if best_score == 1 {
            let potential_defenders = get_defender_build_actions::<3, 10>(&adjacency_field, &field, &defender_config, next_tower.unwrap());
            if potential_defenders.is_empty() {
                defender_config.can_build_tower = false;
            } else {
                let action = &potential_defenders[rand::thread_rng().gen_range(0..potential_defenders.len())];
                if buy_structure(commands, &mut resources, &textures, &field, &presets, &building_config, action.1, action.0) {
                    defender_config.num_defenders += 1;
                    *next_tower = None;
                }
            }
        } else if best_score == 2 {
            // best_sell_score
        }
    }
}

fn buy_structure(
    commands: Commands,
    mut resources: &mut ResourceStore,
    textures: &TextureResource,
    field: &TowerField,
    buildings: &Buildings,
    building_config: &BuildingResource,
    building_type: BuildingType,
    node: Node
) -> bool {
    let preset = buildings.get_preset(building_type);
    if preset.cost <= resources.gold && node.x >= 0 && node.y >= 0 {
        resources.gold -= preset.cost;
        preset.spawn(commands, building_config, field, textures, node.x as usize, node.y as usize);
        return true;
    }
    return false;
}

fn max_index<const TSIZE: usize>(arr: [f32; TSIZE]) -> usize {
    let mut max: f32 = f32::MIN;
    let mut index: usize = 0;
    for i in 0..TSIZE {
        if arr[i] > max {
            max = arr[i];
            index = i;
        }
    }
    return index;
}

fn get_defender_build_actions<const TMAX_LEN: usize, const TITER: usize>(
    adjacency: &HashMap<Node, i32>, 
    field: &TowerField,
    defender_config: &DefenderConfiguration,
    building_type: BuildingType
) -> Vec<(Node, BuildingType)> {
    return get_wall_build_actions::<TMAX_LEN, TITER>(field, defender_config).iter().map(|node| (node.node, building_type)).collect();
    /*let mut vec: Vec<(Node, i32)> =  adjacency.iter()
        .map(|e| (*e.0, *e.1))
        .filter(|e| !field.is_node_occupied(e.0))
        .collect();
    vec.sort_by(|a, b| 
        a.1.cmp(&b.1)
            .then(field.distance_to_start(a.0).total_cmp(&field.distance_to_start(b.0)))
            .reverse()
    );
    return vec.iter().take(TMAX_LEN).map(|e| (e.0, BuildingType::Arrow)).collect();*/
}

fn get_wall_build_actions<const TMAX_LEN: usize, const TITER: usize>(
    field: &TowerField,
    defender_config: &DefenderConfiguration
) -> Vec<WeightedNode> {
    let mut results: Vec<WeightedNode> = Vec::with_capacity(TMAX_LEN);
    let mut seen: HashSet<Node> = HashSet::new();
    let mut i = 0;
    for node in defender_config.path.get_nodes() {
        for current_candidate in get_self_with_successors(node) {
            i+=1;
            if seen.contains(&current_candidate) {
                continue;
            } else {
                seen.insert(current_candidate);
            }
            if results.len() < TMAX_LEN {
                if let Some(weighted_node) = get_wall_build_action(field, defender_config, current_candidate) {
                    results.push(weighted_node);
                }
            } else if i < TITER {
                if let Some(weighted_node) = get_wall_build_action(field, defender_config, current_candidate) {
                    let mut index: i32 = -1;
                    let mut min: f32 = f32::MAX;
                    for j in 0..results.len() {
                        if results[j].weight < min {
                            min = results[j].weight;
                            index = j as i32;
                        }
                    }
                    if index != -1 {
                        results[index as usize] = weighted_node;
                    }
                }
            } else {
                return results;
            }
        }
    }
    return results;
}

fn get_wall_build_action(field: &TowerField, defender_config: &DefenderConfiguration, node: Node) -> Option<WeightedNode> {
    if !defender_config.is_node_adjacent_to_or_on_path(node) || field.is_node_occupied(node) {
        return None;
    }
    let weight = if let Some(path) = a_star_with_blocked_node(field, field.get_start(), field.get_end(), Some(node)) {
        path.get_size()
    } else {
        0
    } as f32;

    if weight > 0. {
        return Some(WeightedNode {node, weight});
    } else {
        return None;
    }
    
}

fn get_sell_actions() -> Vec<Node> {
    return Vec::new();
}