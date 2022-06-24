#![feature(linked_list_cursors)]
#![feature(duration_consts_float)]

use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, prelude::*};
use bevy_egui::EguiPlugin;

use resources::event::PlayerEvent;
use services::player::player::Player;
use system::{
    event_system::update_event,
    play_system::{restart_player, start_player, stop_player, update_player},
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
    // 在linux系统上, 使用gl驱动, 默认的Vulkan驱动会在屏幕关闭后, 导致程序"Timeout"退出
    if cfg!(target_os = "linux") {
        std::env::set_var("WGPU_BACKEND", "gl");
    }
    App::new()
        .insert_resource(WindowDescriptor {
            decorations: false,
            ..Default::default()
        })
        .init_resource::<UiState>()
        .init_resource::<Player>()
        .add_state(GameState::Terminal)
        .add_event::<PlayerEvent>()
        .add_plugins(DefaultPlugins)
        .add_plugin(FrameTimeDiagnosticsPlugin)
        .add_plugin(EguiPlugin)
        .add_startup_system(icon_setup)
        .add_startup_system(egui_setup)
        .add_system(update_ui.chain(update_event))
        .add_system_set(SystemSet::on_enter(GameState::Playing).with_system(start_player))
        .add_system_set(SystemSet::on_update(GameState::Playing).with_system(update_player))
        .add_system_set(SystemSet::on_exit(GameState::Playing).with_system(stop_player))
        .add_system_set(SystemSet::on_enter(GameState::Restart).with_system(restart_player))
        .run();
}
