use bevy::window::WindowMode;
use bevy_egui::egui::TextureHandle;

use super::{
    load_icons::Icons, play_content::PlayContentView, play_control::VideoControl,
    play_list::VideoListView, setting_ui::SettingWindow, titlebar_ui::Titlebar,
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
    pub play_control: VideoControl,
    pub play_list_view: VideoListView,
    pub play_list: Vec<String>,
    pub choose_file: Option<String>,

    pub video: VideoFrame,
    pub video_texture: Option<TextureHandle>,

    pub fps: f64,
    // pub video_ctrl: VideoControl,
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
            play_control: VideoControl::new(),
            play_list_view: VideoListView::new(),
            play_list: vec!["/home/maxu/Videos/dde-introduction.mp4".into()],
            choose_file: None,
            video: VideoFrame::default(),
            video_texture: None,
            fps: 0.0,
        }
    }
}
