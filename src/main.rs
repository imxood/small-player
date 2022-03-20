#![feature(thread_is_running)]

use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, prelude::*};
use bevy_egui::EguiPlugin;

use resources::event::PlayerEvent;
use system::{
    event_system::update_player_event,
    play_system::{start_player, stop_player, update_player},
    setup_system::{egui_setup, icon_setup},
    ui_system::update_ui,
    GameState,
};
use ui::ui_state::UiState;

mod common;
mod defines;
mod error;
mod resources;
mod services;
mod system;
mod ui;

fn main() {
    App::new()
        .init_resource::<UiState>()
        .insert_resource(WindowDescriptor {
            decorations: false,
            ..Default::default()
        })
        .add_state(GameState::Stop)
        .add_event::<PlayerEvent>()
        .add_plugins(DefaultPlugins)
        .add_plugin(FrameTimeDiagnosticsPlugin)
        .add_plugin(EguiPlugin)
        .add_startup_system(icon_setup)
        .add_startup_system(egui_setup)
        .add_system(update_ui.chain(update_player_event))
        .add_system_set(SystemSet::on_enter(GameState::Playing).with_system(start_player))
        .add_system_set(SystemSet::on_update(GameState::Playing).with_system(update_player))
        .add_system_set(SystemSet::on_exit(GameState::Playing).with_system(stop_player))
        .run();
}
