use bevy_egui::egui::{Align2, Context, Window};

pub struct SettingWindow {
    /// 控制窗口显示
    open: bool,
    /// 标记着 控制窗口第一次打开
    first_open: bool,
    frame: u32,
}

impl Default for SettingWindow {
    fn default() -> Self {
        Self {
            open: false,
            first_open: true,
            frame: 0,
        }
    }
}

impl SettingWindow {
    pub fn show(&mut self, ctx: &Context) {
        if !self.open {
            return;
        }

        self.frame = self.frame.wrapping_add(1);

        let window = Window::new("setting")
            .collapsible(false)
            .open(&mut self.open);
        // 如果是第一次打开, 设置居中
        // self.frame <= 2， 使用条件的原因大概是: 第一次显示这个 window, 初始位置是不确定的. 执行两次 anchor 后才可以确定
        let window = if self.frame <= 2 || self.first_open {
            self.first_open = false;
            window.anchor(Align2::CENTER_CENTER, [0.0, -30.0])
        } else {
            window
        };

        window.show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("settings");
            });
        });
    }

    pub fn trigger_show(&mut self) {
        self.open = !self.open;
        if self.open {
            self.first_open = true;
        }
    }
}
