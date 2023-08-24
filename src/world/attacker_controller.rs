use bevy::prelude::{Plugin, App, Resource, EventReader, ResMut, Local};

use super::events::{KillEvent, RoundOverEvent, EntityReachedEnd};


#[derive(Resource)]
pub struct AttackerResource {
    pub gold: i32,
    pub current_bounty: i32
}

pub struct AttackerController;

impl Plugin for AttackerController {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(AttackerResource {gold: 200, current_bounty: 0})
            .add_system(listen_to_deaths)
            .add_system(listen_to_reached_end)
            .add_system(calculate_round_end_bounty);
    }
}

fn listen_to_deaths(
    mut deaths: EventReader<KillEvent>,
    mut attacker_resource: ResMut<AttackerResource>
) {
    for ev in deaths.iter() {
        attacker_resource.gold += ev.original_cost / ev.group_size;
    }
}

fn listen_to_reached_end(
    mut reached_end: EventReader<EntityReachedEnd>,
    mut attacker_resource: ResMut<AttackerResource>
) {
    for ev in reached_end.iter() {
        attacker_resource.gold += ev.bounty;
    }
}

fn calculate_round_end_bounty(
    mut round_end: EventReader<RoundOverEvent>,
    mut reached_end: EventReader<EntityReachedEnd>,
    mut killed: EventReader<KillEvent>,
    mut attacker_resource: ResMut<AttackerResource>,
    mut num_killed: Local<i32>,
    mut num_reached_end: Local<i32>
    
) {
    for _ in reached_end.iter() {
        *num_reached_end += 1;
    }
    for _ in killed.iter() {
        *num_killed += 1;
    }
    attacker_resource.current_bounty = *num_killed * 2 + *num_reached_end * 10;
    if !round_end.is_empty() {
        attacker_resource.gold += attacker_resource.current_bounty;
        attacker_resource.current_bounty = 0;
        *num_killed = 0;
        *num_reached_end = 0;
        round_end.clear();
    }
}