use std::fs;

use bevy::{prelude::{Resource, Vec2}, utils::HashMap};
use serde::{Deserialize, Serialize};

use super::towers::{DefenderAttack, DamageType, ProjectileSprite};



#[derive(Hash, Deserialize, Serialize, PartialEq, Eq, Clone, Copy, Debug)]
pub enum BuildingType {
    Arrow,
    Wall,
    Cannon
}

#[derive(Deserialize, Serialize)]
pub struct Building {
    pub building_type: BuildingType,
    pub config: BuildingConfig
}


#[derive(Deserialize, Serialize)]
pub struct BuildingConfig {
    pub cost: i32,
    pub blocking: bool,
    pub type_config: BuildingTypeConfig
}

#[derive(Deserialize, Serialize)]
pub enum BuildingTypeConfig {
    Defender {
        attack_timer: f32,
        attack: DefenderAttack,
        attack_range: f32
    },
    Wall
}

impl BuildingConfig {
    pub fn get_damage(&self) -> f32 {
        return match &self.type_config {
            BuildingTypeConfig::Defender { attack_timer, attack, attack_range } => match attack {
                DefenderAttack::Projectile { damage_type, damage, projectile_speed, sprite } => *damage,
                DefenderAttack::Splash { damage_type, damage, travel_time, sprite, splash_radius } => *damage
            },
            BuildingTypeConfig::Wall => 0.
        }
    }
    pub fn get_dps(&self) -> f32 {
        return match &self.type_config {
            BuildingTypeConfig::Defender { attack_timer, attack, attack_range } => match attack {
                DefenderAttack::Projectile { damage_type, damage, projectile_speed, sprite } => *damage / *attack_timer,
                DefenderAttack::Splash { damage_type, damage, travel_time, sprite, splash_radius } => *damage / *attack_timer
            },
            BuildingTypeConfig::Wall => 0.
        }
    }
    pub fn get_cost(&self) -> i32 {
        return self.cost;
    }
    pub fn get_blocking(&self) -> bool {
        return self.blocking;
    }
    pub fn is_aoe(&self) -> bool {
        return match &self.type_config {
            BuildingTypeConfig::Defender { attack_timer, attack, attack_range } => match attack {
                DefenderAttack::Splash { damage_type, damage, travel_time, sprite, splash_radius } => true,
                _ => false
            },
            _ => false
        }
    }
}

#[derive(Resource)]
pub struct BuildingResource {
    buildings: HashMap<BuildingType, BuildingConfig>
}

impl BuildingResource {
    pub fn new() -> Self {
        let buildings: Vec<Building> = serde_json::from_str(&fs::read_to_string("assets/tower_definitions.json").unwrap()).unwrap();
        let mut map: HashMap<BuildingType, BuildingConfig> = HashMap::new();
        for building in buildings {
            map.insert(building.building_type, building.config);
        }
        return Self {
            buildings: map
        }
    }

    pub fn get_building_config(&self, building_type: &BuildingType) -> Option<&BuildingConfig> {
        return self.buildings.get(building_type);
    }

    pub fn get_damage(&self, building_type: &BuildingType) -> f32 {
        return self.get_building_config(building_type).map(|e| e.get_damage()).unwrap_or_default();
    }

    pub fn get_dps(&self, building_type: &BuildingType) -> f32 {
        return self.get_building_config(building_type).map(|e| e.get_dps()).unwrap_or(0.);
    }

    pub fn get_blocking(&self, building_type: &BuildingType) -> bool {
        return self.get_building_config(building_type).map(|e| e.get_blocking()).unwrap_or_default();
    }

    pub fn get_cost(&self, building_type: &BuildingType) -> i32 {
        return self.get_building_config(building_type).map(|e| e.get_cost()).unwrap_or_default();
    }
}