use bevy::prelude::*;

use crate::{
    resources::event::PlayerEvent,
    services::player::{player::Player, PlayState},
    system::GameState,
    ui::ui_state::UiState,
};

pub fn start_player(mut ui_state: ResMut<UiState>) {
    ui_state.enter_playing();
}

pub fn stop_player(mut ui_state: ResMut<UiState>, player: ResMut<Player>) {
    ui_state.exit_playing();
    player.set_play_finished();
    // log::info!("service - state: {:?}", &ui_state.play_state);
}

pub fn update_player(
    mut ui_state: ResMut<UiState>,
    mut game_state: ResMut<State<GameState>>,
    mut player: ResMut<Player>,
    mut play_evt_sender: EventWriter<PlayerEvent>,
) {
    // 更新 状态
    if let Some(state) = player.try_recv_state() {
        // log::info!("service - state: {:?}", &state);
        match state {
            PlayState::Pausing(pause) => {
                ui_state.pause = pause;
            }
            PlayState::Terminated => {
                game_state.set(GameState::Terminal).ok();
            }
            PlayState::Video(video) => {
                ui_state.video = Some(video);
                // game_state.set(GameState::Terminal).ok();
            }
            _ => {
                // ui_state.play_state = state
            }
        }
    }
    // 如果一个文件已经播放完毕
    else if player.play_finished() {
        if ui_state.looping {
            play_evt_sender.send(PlayerEvent::Next);
        } else {
            game_state.set(GameState::Terminal).ok();
        }
    }
}

pub fn restart_player(mut state: ResMut<State<GameState>>) {
    log::info!("restart");
    state.set(GameState::Playing).ok();
}
