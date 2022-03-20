use bevy::{
    app::AppExit,
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
    window::{WindowId, WindowMode},
    winit::WinitWindows,
};

use crate::{
    resources::event::PlayerEvent, services::play_service::PlayService, ui::ui_state::UiState,
};

use super::GameState;

pub fn update_player_event(
    mut ui_state: ResMut<UiState>,
    mut events: EventReader<PlayerEvent>,
    mut exit: EventWriter<AppExit>,
    mut windows: ResMut<Windows>,
    winit_windows: Res<WinitWindows>,
    mut state: ResMut<State<GameState>>,
    diagnostics: Res<Diagnostics>,
) {
    if let Some(fps_diagnostic) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
        if let Some(fps_avg) = fps_diagnostic.average() {
            ui_state.fps = fps_avg;
        }
    }

    for event in events.iter() {
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

            /*
                播放控制
            */
            PlayerEvent::OpenFolder => {
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

            PlayerEvent::Start(filename) => {
                log::info!("开始播放 {}", filename);
                state.set(GameState::Playing).unwrap();
                // let play_service = world.iet_resource::<PlayService>();
            }
            PlayerEvent::Stop => {
                log::info!("停止播放");
                state.set(GameState::Stop).unwrap();
            }
            _ => {}
        }
    }
}
