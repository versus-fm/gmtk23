use std::collections::VecDeque;

use bevy::{prelude::{Plugin, App, Resource, ResMut, Commands, Res, Local, EventReader, Query, Entity, EventWriter}, time::Time};

use crate::{textures::TextureResource, util::RepeatingLocalTimer};

use super::{attackers::{AttackerType, spawn_attacker, Attacker, AttackerStats}, towers::TowerField, events::{RequestRoundStart, RoundStartEvent, RoundOverEvent}};


#[derive(Resource)]
pub struct RoundResource {
    pending_spawn_queue: VecDeque<AttackerType>,
    active_spawn_queue: VecDeque<AttackerType>,
    round_active: bool
}

impl RoundResource {
    pub fn queue(&mut self, attacker_type: &AttackerType) {
        self.pending_spawn_queue.push_back(attacker_type.clone());
    }
}

pub struct RoundPlugin;

impl Plugin for RoundPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(RoundResource {
                active_spawn_queue: VecDeque::new(),
                pending_spawn_queue: VecDeque::new(),
                round_active: false
            })
            .add_system(process_spawn_queue)
            .add_system(process_request_round_start)
            .add_system(check_round_end);
    }
}

fn process_spawn_queue(
    mut commands: Commands,
    mut round: ResMut<RoundResource>,
    field: Res<TowerField>,
    textures: Res<TextureResource>,
    mut timer: Local<RepeatingLocalTimer<1000>>,
    attackers: Res<AttackerStats>,
    time: Res<Time>
) {
    timer.timer.tick(time.delta());
    let active = round.round_active;
    let queue = &mut round.active_spawn_queue;
    if timer.timer.just_finished() && !queue.is_empty() && active {
        if let Some(next) = queue.pop_front() {
            spawn_attacker(commands, &field, &textures, next, &attackers);
        }
    }
}

fn process_request_round_start(
    mut event: EventReader<RequestRoundStart>,
    mut round: ResMut<RoundResource>,
    mut round_start: EventWriter<RoundStartEvent>
) {
    for ev in event.iter() {
        if !round.round_active && round.active_spawn_queue.is_empty() {
            round.round_active = true;
            round.active_spawn_queue = round.pending_spawn_queue.clone();
            round.pending_spawn_queue = VecDeque::new();
            round_start.send(RoundStartEvent);
        }
    }
}

fn check_round_end(
    mut round: ResMut<RoundResource>,
    query: Query<(Entity, &Attacker)>,
    mut round_end: EventWriter<RoundOverEvent>
) {
    if round.round_active && round.active_spawn_queue.is_empty() && query.is_empty() {
        round.round_active = false;
        round_end.send(RoundOverEvent);
    }
}