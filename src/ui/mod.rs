

use core::fmt;

use bevy::{prelude::{Plugin, App, Res, EventWriter, ResMut, Handle, Image, World, FromWorld, Resource, AssetServer, Local, Vec2, IntoSystemConfig, Events}, time::Time};
use bevy_egui::{egui::{self, style, Color32, Ui, RichText, Align}, EguiContexts};

use crate::world::{attacker_controller::AttackerResource, events::RequestRoundStart, rounds::RoundResource, attackers::{Attacker, AttackerStats, AttackerType, UpgradeType}, defender_controller::{ResourceStore, RoundStats, DefenderConfiguration}};


const GOLD_COLOR: Color32 = Color32::from_rgb(255, 215, 0);
const LIVES_COLOR: Color32 = Color32::from_rgb(155, 16, 3);

#[derive(Resource)]
struct Images {
    rock_icon: Handle<Image>,
    coin_icon: Handle<Image>,
    heart_icon: Handle<Image>
}

impl FromWorld for Images {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.get_resource_mut::<AssetServer>().unwrap();
        Self {
            rock_icon: asset_server.load("icons/rock.png"),
            coin_icon: asset_server.load("icons/coin.png"),
            heart_icon: asset_server.load("icons/heart.png"),
        }
    }
}

#[derive(Resource)]
struct State {
    pub show_defender_params: bool
}

impl Default for State {
    fn default() -> Self {
        Self { show_defender_params: false }
    }
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<Images>()
            .init_resource::<State>()
            .add_system(top_panel)
            .add_system(defender_params)
            .add_system(side_unit_panel.after(top_panel))
            .add_system(check_victory);
    }
}

fn check_victory(
    mut contexts: EguiContexts,
    defender_resource: Res<ResourceStore>,
    mut time: ResMut<Time>,
    mut app_exit_events: ResMut<Events<bevy::app::AppExit>>
) {
    if defender_resource.lives <= 0 {
        egui::Window::new("Victory").title_bar(false).show(contexts.ctx_mut(), |ui| {
            ui.label("You Won!");
            if ui.button("Exit").clicked() {
                app_exit_events.send(bevy::app::AppExit);
            }
        });
        time.pause();
    }
}

fn top_panel(
    mut contexts: EguiContexts,
    attacker_resource: Res<AttackerResource>,
    defender_resource: Res<ResourceStore>,
    attackers: Res<AttackerStats>,
    round: Res<RoundResource>,
    mut start_round: EventWriter<RequestRoundStart>,
    mut coin_icon: Local<egui::TextureId>,
    mut heart_icon: Local<egui::TextureId>,
    mut is_initialized: Local<bool>,
    mut state: ResMut<State>,
    mut timing: ResMut<Time>,
    images: Res<Images>
) {
    if !*is_initialized {
        *is_initialized = true;
        *coin_icon = contexts.add_image(images.coin_icon.clone_weak());
        *heart_icon = contexts.add_image(images.heart_icon.clone_weak());
    }
    egui::TopBottomPanel::top("top_resource_panel").show(contexts.ctx_mut(), |ui| {
        ui.horizontal(|bar| {
            if bar.button("Start Round").clicked() {
                start_round.send(RequestRoundStart);
            }
            bar.separator();

            bar.add(egui::widgets::Image::new(*coin_icon, [22., 22.]).tint(GOLD_COLOR));
            bar.colored_label(GOLD_COLOR, attacker_resource.gold.to_string()).on_hover_ui_at_pointer(|tooltip| {
                tooltip.heading("Gold");
                tooltip.label("Shows current amount of gold");
            });
            bar.colored_label(GOLD_COLOR, format!(" + {}", attacker_resource.current_bounty)).on_hover_ui_at_pointer(|tooltip| {
                tooltip.heading("Bounty");
                tooltip.label("Shows current accumulated bounty that will be rewarded at the end of the round");
                tooltip.label("Can be be increased by: ");
                tooltip.indent(tooltip.id(), |indent| {
                    indent.label("• Reaching the end");
                    indent.label("• Having attackers die");
                });
            });
            bar.spacing();
            bar.add(egui::widgets::Image::new(*heart_icon, [16., 16.]).tint(LIVES_COLOR));
            bar.colored_label(LIVES_COLOR, defender_resource.lives.to_string()).on_hover_ui_at_pointer(|tooltip| {
                tooltip.heading("Lives");
                tooltip.label("Shows current defender lives. When this reaches 0 you win!");
            });

            bar.separator();
            let current_speed = timing.relative_speed();
            if bar.small_button("-").on_hover_text("Decrease game speed by 20%").clicked() {
                let new_speed = (current_speed - 0.2).clamp(0.4, 4.);
                timing.set_relative_speed(new_speed);
            }
            bar.label(format!("{:.2}", current_speed));
            if bar.small_button("+").on_hover_text("Increase game speed by 20%").clicked() {
                let new_speed = (current_speed + 0.2).clamp(0.4, 4.);
                timing.set_relative_speed(new_speed);
            }


            bar.with_layout(egui::Layout::right_to_left(egui::Align::Center), |bar| {
                bar.menu_button(":)", |menu| {
                    if menu.button("Defender Parameters").on_hover_text_at_pointer("Debug parameters for the defender AI").clicked() {
                        state.show_defender_params = true;
                        menu.close_menu();
                    }
                });
            });
        });
    });
}

fn side_unit_panel(
    mut contexts: EguiContexts,
    mut attacker_resource: ResMut<AttackerResource>,
    mut round: ResMut<RoundResource>,
    mut attackers: ResMut<AttackerStats>
) {
    egui::SidePanel::right("side_panel").show(contexts.ctx_mut(), |ui| {
        let orc_warrior_cost = attackers.get_cost(AttackerType::OrcWarrior);
        let spider_cost = attackers.get_cost(AttackerType::Spider);
        let golem_cost = attackers.get_cost(AttackerType::Golem);
        if ui.button("Orc Warrior")
            .on_hover_ui(attacker_tooltip(AttackerType::OrcWarrior, &attackers))
            .clicked() && orc_warrior_cost <= attacker_resource.gold {
            attacker_resource.gold -= orc_warrior_cost;
            round.queue(&AttackerType::OrcWarrior);
        }
        if ui.button("Spider")
            .on_hover_ui(attacker_tooltip(AttackerType::Spider, &attackers))
            .clicked() && spider_cost <= attacker_resource.gold {
            attacker_resource.gold -= spider_cost;
            round.queue(&AttackerType::Spider);
        }
        if ui.button("Golem")
        .on_hover_ui(attacker_tooltip(AttackerType::Golem, &attackers))
        .clicked() && golem_cost <= attacker_resource.gold {
            attacker_resource.gold -= golem_cost;
            round.queue(&AttackerType::Golem);
        }

        ui.separator();
        ui.label("Upgrade Orc Warrior");
        ui.horizontal(|group| {
            let health_cost = attackers.get_upgrade_cost(AttackerType::OrcWarrior, UpgradeType::Health);
            let speed_cost = attackers.get_upgrade_cost(AttackerType::OrcWarrior, UpgradeType::Speed);
            let amount_cost = attackers.get_upgrade_cost(AttackerType::OrcWarrior, UpgradeType::Amount);
            let current_cold = attacker_resource.gold;
            if group.button("Health").on_hover_text(format!("Boost health by 10%. Cost: {}", health_cost)).clicked() && current_cold >= health_cost {
                attackers.apply_upgrade(AttackerType::OrcWarrior, UpgradeType::Health);
                attacker_resource.gold -= health_cost;
            }
            if group.button("Speed").on_hover_text(format!("Boost speed by 20%. Cost: {}", speed_cost)).clicked() && current_cold >= speed_cost {
                attackers.apply_upgrade(AttackerType::OrcWarrior, UpgradeType::Speed);
                attacker_resource.gold -= speed_cost;
            }
            if group.button("Amount").on_hover_text(format!("Increase amount summoned by one. Cost: {}", amount_cost)).clicked() && current_cold >= amount_cost {
                attackers.apply_upgrade(AttackerType::OrcWarrior, UpgradeType::Amount);
                attacker_resource.gold -= amount_cost;
            }
        });
        ui.separator();
        ui.label("Upgrade Spider");
        ui.horizontal(|group| {
            let health_cost = attackers.get_upgrade_cost(AttackerType::Spider, UpgradeType::Health);
            let speed_cost = attackers.get_upgrade_cost(AttackerType::Spider, UpgradeType::Speed);
            let amount_cost = attackers.get_upgrade_cost(AttackerType::Spider, UpgradeType::Amount);
            let current_cold = attacker_resource.gold;
            if group.button("Health").on_hover_text(format!("Boost health by 20%. Cost: {}", health_cost)).clicked() && current_cold >= health_cost {
                attackers.apply_upgrade(AttackerType::Spider, UpgradeType::Health);
                attacker_resource.gold -= health_cost;
            }
            if group.button("Speed").on_hover_text(format!("Boost speed by 20%. Cost: {}", speed_cost)).clicked() && current_cold >= speed_cost {
                attackers.apply_upgrade(AttackerType::Spider, UpgradeType::Speed);
                attacker_resource.gold -= speed_cost;
            }
            if group.button("Amount").on_hover_text(format!("Increase amount summoned by one. Cost: {}", amount_cost)).clicked() && current_cold >= amount_cost {
                attackers.apply_upgrade(AttackerType::Spider, UpgradeType::Amount);
                attacker_resource.gold -= amount_cost;
            }
        });
        ui.separator();
        ui.label("Upgrade Golem");
        ui.horizontal(|group| {
            let health_cost = attackers.get_upgrade_cost(AttackerType::Golem, UpgradeType::Health);
            let speed_cost = attackers.get_upgrade_cost(AttackerType::Golem, UpgradeType::Speed);
            let amount_cost = attackers.get_upgrade_cost(AttackerType::Golem, UpgradeType::Amount);
            let current_cold = attacker_resource.gold;
            if group.button("Health").on_hover_text(format!("Boost health by 10%. Cost: {}", health_cost)).clicked() && current_cold >= health_cost {
                attackers.apply_upgrade(AttackerType::Golem, UpgradeType::Health);
                attacker_resource.gold -= health_cost;
            }
            if group.button("Speed").on_hover_text(format!("Boost speed by 20%. Cost: {}", speed_cost)).clicked() && current_cold >= speed_cost {
                attackers.apply_upgrade(AttackerType::Golem, UpgradeType::Speed);
                attacker_resource.gold -= speed_cost;
            }
            if group.button("Amount").on_hover_text(format!("Increase amount summoned by one. Cost: {}", amount_cost)).clicked() && current_cold >= amount_cost {
                attackers.apply_upgrade(AttackerType::Golem, UpgradeType::Amount);
                attacker_resource.gold -= amount_cost;
            }
        })

    });
}

fn attacker_tooltip<'a>(attacker_type: AttackerType, attackers: &'a AttackerStats) -> impl FnOnce(&mut Ui) -> () + 'a {
    return move |tooltip| {
        let attacker = attackers.get_stats(attacker_type);
        tooltip.heading(attacker_type.get_name());
        tooltip.horizontal(|group| {
            group.label("Spawn amount: ");
            group.label(attacker.num_summoned.to_string());
        });
        tooltip.horizontal(|group| {
            group.label("Cost: ");
            group.label(RichText::new(attacker.original_cost.to_string()).color(GOLD_COLOR));
        });
        tooltip.horizontal(|group| {
            group.label("Defender bounty: ");
            group.label(RichText::new(attacker.bounty.to_string()).color(GOLD_COLOR));
        });
        tooltip.horizontal(|group| {
            group.label("Attacker bounty: ");
            group.label(RichText::new((attacker.original_cost / attacker.num_summoned).to_string()).color(GOLD_COLOR));
        });
        tooltip.horizontal(|group| {
            group.label("Health: ");
            group.label(RichText::new(attacker.max_health.to_string()));
        });
        tooltip.horizontal(|group| {
            group.label("Speed: ");
            group.label(format!("{} pixels/s", attacker.movement_speed));
        });
    }
}

fn defender_params(
    mut contexts: EguiContexts,
    state: Res<State>,
    resources: Res<ResourceStore>,
    round_stats: Res<RoundStats>,
    defender_config: Res<DefenderConfiguration>
) {
    if state.show_defender_params {
        egui::Window::new("Defender Params").title_bar(true).show(contexts.ctx_mut(), |window| {
            window.columns(2, |cols| {
                cols[0].label("Gold");
                cols[1].label(resources.gold.to_string());
            });
            window.columns(2, |cols| {
                cols[0].label("Max APM");
                cols[1].label(
                    ((60. / defender_config.action_cooldown.duration().as_secs_f32() * 100.).round() / 100.).to_string()
                );
            });
            window.columns(2, |cols| {
                cols[0].label("Wall weight");
                cols[1].label(defender_config.wall_weight.to_string());
            });
            window.columns(2, |cols| {
                cols[0].label("Damage weight");
                cols[1].label(defender_config.damage_weight.to_string());
            });
            window.columns(2, |cols| {
                cols[0].label("Sell weight");
                cols[1].label(defender_config.sell_weight.to_string());
            });
            window.columns(2, |cols| {
                cols[0].label("Est. Damage needed");
                cols[1].label(defender_config.estimated_damage_needed.to_string());
            });
            window.columns(2, |cols| {
                cols[0].label("Est. Damage potential");
                cols[1].label(defender_config.estimated_damage_potential.to_string());
            });
            window.columns(2, |cols| {
                cols[0].label("Path Length");
                cols[1].label(defender_config.path_length.to_string());
            });
            window.separator();
            window.label("Round stats");
            window.columns(2, |cols| {
                cols[0].label("Damage dealt");
                cols[1].label(round_stats.damage_dealt.to_string());
            });
            window.columns(2, |cols| {
                cols[0].label("Round duration");
                cols[1].label(format!("{}s", round_stats.round_duration.as_secs()));
            });
            window.columns(2, |cols| {
                cols[0].label("Number reached end");
                cols[1].label(round_stats.num_reached_end.to_string());
            });
            window.columns(2, |cols| {
                cols[0].label("Number killed");
                cols[1].label(round_stats.num_killed.to_string());
            });
            window.columns(2, |cols| {
                cols[0].label("Closest to end");
                cols[1].label(round_stats.closest_distance_to_end.to_string());
            });
        });
    }
}