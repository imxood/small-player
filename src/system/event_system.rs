use bevy::{
    app::AppExit,
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
    window::{WindowId, WindowMode},
    winit::WinitWindows,
};

use crate::{
    resources::event::PlayerEvent, services::player::player::Player, ui::ui_state::UiState,
};

use super::GameState;

pub fn update_event(
    winit_windows: Res<WinitWindows>,
    diagnostics: Res<Diagnostics>,
    mut ui_state: ResMut<UiState>,
    mut game_state: ResMut<State<GameState>>,
    mut player: ResMut<Player>,
    mut player_evt: EventReader<PlayerEvent>,
    mut windows: ResMut<Windows>,
    mut exit: EventWriter<AppExit>,
) {
    if let Some(fps_diagnostic) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
        if let Some(fps_avg) = fps_diagnostic.average() {
            ui_state.fps = fps_avg;
        }
    }

    for event in player_evt.iter() {
        match event {
            /*
                窗口控制
            */
            PlayerEvent::Exit => {
                exit.send(AppExit);
            }
            PlayerEvent::Fullscreen => {
                if let Some(window) = windows.get_primary_mut() {
                    let window_mode = window.mode();
                    if window_mode == WindowMode::Fullscreen {
                        ui_state.window_mode = WindowMode::Windowed;
                    } else {
                        ui_state.window_mode = WindowMode::Fullscreen;
                    }
                    window.set_mode(ui_state.window_mode);
                }
            }
            PlayerEvent::Maximize => {
                if let Some(window) = winit_windows.get_window(WindowId::primary()) {
                    ui_state.maximized = !ui_state.maximized;
                    window.set_maximized(ui_state.maximized);
                }
            }
            PlayerEvent::Minimize => {
                if let Some(window) = windows.get_primary_mut() {
                    window.set_minimized(true);
                }
            }
            PlayerEvent::DragWindow => {
                if let Some(window) = winit_windows.get_window(WindowId::primary()) {
                    window.drag_window().ok();
                }
            }
            _ => {}
        }

        match event {
            /*
                播放控制
            */
            PlayerEvent::OpenFile => {
                if let Some(files) = rfd::FileDialog::new()
                    .add_filter("video", &["mp4"])
                    .pick_files()
                {
                    for file in files {
                        ui_state
                            .play_list
                            .push(file.into_os_string().into_string().unwrap());
                    }
                }
            }
            PlayerEvent::Terminate => {
                log::info!("停止播放");
                game_state.set(GameState::Terminal).ok();
            }
            PlayerEvent::Pause(pause) => {
                player.set_pause(*pause);
                continue;
            }
            PlayerEvent::Mute(mute) => {
                player.set_mute(*mute);
                continue;
            }
            PlayerEvent::Volume(volume) => {
                player.set_volume(*volume);
                continue;
            }
            _ => {}
        }

        // 根据循环模式, 和 当前选择文件 或者 未选择文件 的不同情况, 确定要播放的文件
        let offset = match event {
            PlayerEvent::Previous => -1,
            PlayerEvent::Play => 0,
            PlayerEvent::Next => 1,
            _ => continue,
        };

        // 播放文件
        if let Some(file) = ui_state.current_filename(offset) {
            match player.play(file.clone()) {
                Ok(_) => {
                    player.set_volume(ui_state.volume);

                    log::info!("开始播放 {}", file);
                    
                    ui_state.enter_playing();
                    game_state.set(GameState::Playing).ok();
                    continue;
                }
                Err(e) => {
                    log::info!("播放失败, E: {}", e.to_string());
                }
            }
        }
    }
}
