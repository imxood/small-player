use bevy::window::WindowMode;
use bevy_egui::egui::TextureHandle;

use super::{
    load_icons::Icons, play_content::PlayContentView, setting_ui::SettingWindow,
    titlebar_ui::Titlebar,
};
use crate::{
    resources::theme::Theme,
    services::player::{PlayState, VideoFrame},
};

pub struct UiState {
    pub state: PlayState,
    pub maximized: bool,
    pub window_mode: WindowMode,
    pub scale_factor: f64,
    pub theme: Theme,
    pub icons: Icons,
    pub titlebar: Titlebar,
    pub setting_window: SettingWindow,
    pub play_content_view: PlayContentView,
    pub play_list: Vec<String>,
    pub choose_file: Option<String>,

    pub video: VideoFrame,
    pub video_texture: Option<TextureHandle>,

    // 暂停
    pub pause: bool,
    // 音量
    pub volume: f32,
    // 静音
    pub mute: bool,

    // 打开侧边列表
    pub open_list: bool,

    // System的FPS
    pub fps: f64,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            state: PlayState::default(),
            maximized: true,
            window_mode: WindowMode::Windowed,
            scale_factor: 1.25,
            theme: Theme::default(),
            icons: Icons::new(),
            titlebar: Titlebar::default(),
            setting_window: Default::default(),
            play_content_view: PlayContentView::new(),
            play_list: Vec::new(),
            choose_file: None,
            video: VideoFrame::default(),
            video_texture: None,
            pause: false,
            volume: 0.2,
            mute: true,
            open_list: true,
            fps: 0.0,
        }
    }
}
