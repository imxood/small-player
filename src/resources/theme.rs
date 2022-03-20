use std::sync::Arc;

use bevy_egui::egui::{style::Margin, vec2, Color32, Rounding, Style, Visuals};

/// 此处定义了两种样式
pub struct Theme {
    /// 暗黑风格
    dark_style: Arc<Style>,
    /// 明亮风格
    light_style: Arc<Style>,
    /// 蓝色风格
    blue_style: Arc<Style>,
    blue_titlebar_style: Arc<Style>,
    blue_video_content_style: Arc<Style>,
}

impl Default for Theme {
    fn default() -> Self {
        let dark_style = Style {
            visuals: Visuals::dark(),
            ..Default::default()
        };

        let light_style = Style {
            visuals: Visuals::light(),
            ..Default::default()
        };

        let mut blue_style = Style {
            visuals: Visuals::light(),
            ..Default::default()
        };

        /*
            无法交互的元素
        */

        // 窗口背景色
        blue_style.visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(3, 45, 100);
        // 窗口边框颜色
        blue_style.visuals.widgets.noninteractive.bg_stroke.color =
            Color32::from_rgba_premultiplied(46, 46, 46, 0);
        // 字体颜色
        blue_style.visuals.widgets.noninteractive.fg_stroke.color = Color32::WHITE;

        /*
            可交互: 未激活时
        */

        // 字体颜色
        blue_style.visuals.widgets.inactive.fg_stroke.color = Color32::from_rgb(205, 205, 205);
        // 按钮背景色
        blue_style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(25, 66, 124);

        /*
            可交互: 鼠标经过时
        */

        // 字体颜色
        blue_style.visuals.widgets.hovered.fg_stroke.color = Color32::WHITE;
        // 按钮背景颜色
        blue_style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(4, 148, 210);

        /*
            可交互: 激活时
        */

        // 字体颜色
        blue_style.visuals.widgets.active.fg_stroke.color = Color32::WHITE;
        // 背景颜色
        blue_style.visuals.widgets.active.bg_fill = Color32::from_rgb(37, 95, 226);
        // 未激活时 字体颜色
        blue_style.visuals.widgets.active.fg_stroke.color = Color32::from_rgb(205, 205, 205);

        // 设置窗口的角半径
        blue_style.visuals.window_rounding = Rounding::from(0.0);
        // 滚动条宽度
        blue_style.spacing.scroll_bar_width = 2.;

        let mut blue_titlebar_style = blue_style.clone();
        // 按钮内间距
        blue_titlebar_style.spacing.button_padding = vec2(5.0, 3.0);
        // 组件间隔
        blue_titlebar_style.spacing.item_spacing = vec2(5.0, 0.0);

        let mut blue_video_content_style = blue_style.clone();
        blue_video_content_style.spacing.window_margin = Margin::same(0.);
        blue_video_content_style.spacing.button_padding = vec2(0., 0.);
        blue_video_content_style.spacing.item_spacing = vec2(0., 0.);
        blue_video_content_style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(65, 105, 178);

        Self {
            dark_style: Arc::new(dark_style),
            light_style: Arc::new(light_style),
            blue_style: Arc::new(blue_style),
            blue_titlebar_style: Arc::new(blue_titlebar_style),
            blue_video_content_style: Arc::new(blue_video_content_style),
        }
    }
}

impl Theme {
    pub fn dark_style_clone(&self) -> Arc<Style> {
        self.dark_style.clone()
    }

    pub fn light_style_clone(&self) -> Arc<Style> {
        self.light_style.clone()
    }

    pub fn blue_style_clone(&self) -> Arc<Style> {
        self.blue_style.clone()
    }

    pub fn blue_titlebar_style_clone(&self) -> Arc<Style> {
        self.blue_titlebar_style.clone()
    }

    pub fn blue_video_content_style(&self) -> Arc<Style> {
        self.blue_video_content_style.clone()
    }
}
