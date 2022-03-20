use bevy::{prelude::*, window::WindowId, winit::WinitWindows};
use bevy_egui::{EguiContext, EguiSettings};
use winit::window::Icon;

use crate::{defines::icons::ICON_LOGO, resources::fonts::load_fonts, ui::ui_state::UiState};

/// egui环境初始化
pub fn egui_setup(
    mut egui_ctx: ResMut<EguiContext>,
    mut egui_settings: ResMut<EguiSettings>,
    mut windows: ResMut<Windows>,
    ui_state: Res<UiState>,
) {
    if let Some(window) = windows.get_primary_mut() {
        window.set_maximized(ui_state.maximized);
        window.set_mode(ui_state.window_mode);
    }

    egui_settings.scale_factor = ui_state.scale_factor;

    let ctx = egui_ctx.ctx_mut();

    ctx.set_fonts(load_fonts());

    ctx.set_style(ui_state.theme.blue_style_clone());

    // ctx.set_debug_on_hover(true);
}

/// 设置应用图标
pub fn icon_setup(windows: Res<WinitWindows>) {
    let primary = windows.get_window(WindowId::primary()).unwrap();

    let (icon_rgba, icon_width, icon_height) = {
        let image = image::load_from_memory(ICON_LOGO)
            .expect("Failed to open logo icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };

    let icon = Icon::from_rgba(icon_rgba, icon_width, icon_height).unwrap();

    primary.set_window_icon(Some(icon));
}
