use bevy::prelude::*;
use bevy_egui::{
    egui::{style::Margin, CentralPanel, Color32, Frame, TopBottomPanel},
    EguiContext,
};

use crate::{
    resources::event::PlayerEvent,
    ui::{
        play_content::PlayContentView, play_control::VideoControl, play_list::VideoListView,
        titlebar_ui::Titlebar, ui_state::UiState,
    },
};

/// 注意 ui 与 业务 分离, 与业余交互部分, 使用 event, 或者其它手段
/// 此处使用 play_event, 发布 ui产生 的必要事件
pub fn update_ui(
    mut egui_ctx: ResMut<EguiContext>,
    mut ui_state: ResMut<UiState>,
    mut player_event: EventWriter<PlayerEvent>,
) {
    let ctx = egui_ctx.ctx_mut();
    let ui_state = &mut *ui_state;

    TopBottomPanel::top("top_panel").show(ctx, |ui| {
        Titlebar::show(ctx, ui, ui_state, &mut player_event)
    });

    // 设置背景
    let frame = Frame {
        inner_margin: Margin::symmetric(0.0, 6.0),
        fill: Color32::from_rgb(42, 56, 115),
        ..Default::default()
    };

    TopBottomPanel::bottom("bottom_panel")
        .frame(frame)
        .show(ctx, |ui| {
            // Titlebar::show(ctx, ui, exit, windows, &mut ui_state, winit_windows)
            VideoControl::show(ctx, ui, ui_state, &mut player_event);
        });

    // 先显示 right pannel, 再显示 center panel, 是因为一些 center panel 的宽度 需要动态计算.
    VideoListView::show(ctx, ui_state, &mut player_event);

    // 设置背景
    let frame = Frame {
        inner_margin: Margin::symmetric(0., 0.),
        ..Default::default()
    };

    CentralPanel::default().frame(frame).show(ctx, |ui| {
        PlayContentView::show(ctx, ui, ui_state);
    });

    ui_state.setting_window.show(ctx);
}
