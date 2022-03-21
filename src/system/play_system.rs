use bevy::prelude::*;

use crate::{
    resources::event::PlayEvent,
    services::{play_service::PlayService, player::PlayState},
    system::GameState,
    ui::ui_state::UiState,
};

pub fn start_player(
    mut ui_state: ResMut<UiState>,
    mut commands: Commands,
    mut state: ResMut<State<GameState>>,
) {
    ui_state.state = PlayState::Start;
    let choose_file = ui_state.choose_file.take();
    if let Some(filename) = choose_file {
        match PlayService::create(filename) {
            Ok(service) => {
                commands.insert_resource(service);
                return;
            }
            Err(e) => {
                log::info!("播放失败, E: {}", e.to_string());
            }
        }
    } else {
        log::info!("未选中文件, 无法播放");
    }
    state.set(GameState::Stop).ok();
}

pub fn stop_player(mut ui_state: ResMut<UiState>, mut commands: Commands) {
    commands.remove_resource::<PlayService>();
    ui_state.state = PlayState::Stopped;
    log::info!("service - state: {:?}", &ui_state.state);
}

pub fn update_player(
    mut ui_state: ResMut<UiState>,
    mut play_service: ResMut<PlayService>,
    mut state: ResMut<State<GameState>>,
    mut play_evt: EventReader<PlayEvent>,
) {
    for event in play_evt.iter() {
        match event {
            PlayEvent::Volume(volume) => {
                play_service.set_volume(*volume);
            }
            _ => {}
        }
    }
    if let Some(state) = play_service.try_recv_state() {
        // log::info!("service - state: {:?}", &state);
        match state {
            _ => ui_state.state = state,
        }
    } else if play_service.is_stopped() {
        state.set(GameState::Stop).ok();
    }
}

pub fn restart_player(mut state: ResMut<State<GameState>>) {
    log::info!("restart");
    state.set(GameState::Playing).ok();
}
