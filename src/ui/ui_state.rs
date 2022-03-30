use bevy::window::WindowMode;
use bevy_egui::egui::TextureHandle;

use super::{load_icons::Icons, setting_ui::SettingWindow, titlebar_ui::Titlebar};
use crate::{resources::theme::Theme, services::player::video::VideoFrame};

pub struct UiState {
    pub maximized: bool,
    pub window_mode: WindowMode,
    pub scale_factor: f64,
    pub theme: Theme,
    pub icons: Icons,
    pub titlebar: Titlebar,
    pub setting_window: SettingWindow,
    pub play_list: Vec<String>,
    pub current_idx: Option<usize>,

    pub video: Option<VideoFrame>,
    pub video_texture: Option<TextureHandle>,

    /// 暂停
    pub pause: bool,
    /// 音量
    pub volume: f32,
    /// 静音
    pub mute: bool,
    /// 循环
    pub looping: bool,
    /// 正在播放
    pub playing: bool,

    /// 打开侧边列表
    pub open_list: bool,

    /// System的FPS
    pub fps: f64,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            maximized: true,
            window_mode: WindowMode::Windowed,
            scale_factor: 1.25,
            theme: Theme::default(),
            icons: Icons::new(),
            titlebar: Titlebar::default(),
            setting_window: Default::default(),
            play_list: vec!["/home/maxu/Videos/trailer.mp4".to_string()],
            current_idx: None,
            video: None,
            video_texture: None,
            pause: false,
            volume: 1.0,
            mute: true,
            looping: false,
            playing: false,
            open_list: true,
            fps: 0.0,
        }
    }
}

impl UiState {
    /// 进入播放状态
    pub fn enter_playing(&mut self) {
        self.playing = true;
    }

    /// 离开播放状态
    pub fn exit_playing(&mut self) {
        self.playing = false;

        self.current_idx = None;
        self.video = None;
        self.video_texture = None;
    }
}

impl UiState {
    pub fn current_filename(&mut self, offset: i32) -> Option<String> {
        let idx;

        if self.play_list.is_empty() {
            return None;
        }

        // 如果没有选一个播放, 则设置当前选中了第一个
        if let Some(idx_) = self.current_idx {
            idx = idx_;
        } else {
            self.current_idx = Some(0);
            return Some(self.play_list[0].clone());
        }

        // 如果不是循环
        if !self.looping {
            // 第一个的前一个是 None 或者 最后一个的后一个是 None
            if (idx == 0 && offset < 0) || (idx == self.play_list.len() - 1 && offset > 0) {
                return None;
            }
        }

        let idx = ((idx as i32 + offset) % (self.play_list.len() as i32)) as usize;
        if let Some(filename) = self.play_list.get(idx) {
            self.current_idx = Some(idx);
            return Some(filename.clone());
        }
        None
    }
}
