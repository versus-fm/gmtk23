use bevy::prelude::{Entity, Plugin, App, Vec2};

use super::{path_finding::Node, building_configuration::BuildingType};



pub struct DamageEvent {
    pub amount: f32,
    pub target: Entity
}

pub struct KillEvent {
    pub target: Entity,
    pub source: Entity,
    pub bounty: i32,
    pub original_cost: i32,
    pub group_size: i32,
    pub death_position: Vec2
}

pub struct EntityReachedEnd {
    pub entity: Entity,
    pub bounty: i32
}

pub struct RoundOverEvent;
pub struct RoundStartEvent;
pub struct RequestRoundStart;
pub struct FieldModified;

pub struct RemoveStructureRequest {
    pub node: Node
}

pub struct RemovedStructureEvent {
    pub node: Node,
    pub building_type: BuildingType
}

pub struct EventsPlugin;

impl Plugin for EventsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_event::<DamageEvent>()
            .add_event::<KillEvent>()
            .add_event::<RoundOverEvent>()
            .add_event::<RoundStartEvent>()
            .add_event::<RequestRoundStart>()
            .add_event::<FieldModified>()
            .add_event::<EntityReachedEnd>()
            .add_event::<RemoveStructureRequest>()
            .add_event::<RemovedStructureEvent>();
    }
}