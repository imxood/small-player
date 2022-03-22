use bevy::prelude::*;
use bevy_egui::egui::{self, Align2, Context, Direction, Layout, ScrollArea, Sense, Window};

use crate::{defines::APP_NAME, resources::event::PlayerEvent};

use super::ui_state::UiState;

pub struct Titlebar {
    pub style_ui_open: bool,
}

impl Default for Titlebar {
    fn default() -> Self {
        Self {
            style_ui_open: Default::default(),
        }
    }
}

impl Titlebar {
    pub fn trigger_style_ui(&mut self) {
        self.style_ui_open = !self.style_ui_open;
    }

    pub fn style_ui(&mut self, ctx: &Context) {
        Window::new("style_ui")
            .collapsible(false)
            .open(&mut self.style_ui_open)
            .anchor(Align2::CENTER_CENTER, [0.0, -30.0])
            .show(ctx, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    ctx.style_ui(ui);
                });
            });
    }

    pub fn show(
        ctx: &Context,
        ui: &mut egui::Ui,
        ui_state: &mut UiState,
        player_event: &mut EventWriter<PlayerEvent>,
    ) {
        ui.horizontal(|ui| {
            // 设置 标题栏 的样式
            ui.set_style(ui_state.theme.blue_titlebar_style_clone());

            ui.with_layout(egui::Layout::left_to_right(), |ui| {
                ui.menu_button("选项", |ui| {
                    if ui.button("打开目录").clicked() {
                        ui.close_menu();
                        player_event.send(PlayerEvent::OpenFolder);
                    }
                });
                ui.menu_button("样式", |ui| {
                    ui_state.titlebar.trigger_style_ui();
                    ui.close_menu();
                });
            });
            ui.with_layout(Layout::right_to_left(), |ui| {
                // 关闭窗口
                if ui.button("✖").clicked() {
                    player_event.send(PlayerEvent::Exit);
                }
                // 最大化
                if ui.button("⛶").clicked() {
                    player_event.send(PlayerEvent::Fullscreen);
                }
                // 最小化
                if ui.button("➖").clicked() {
                    player_event.send(PlayerEvent::Minimize);
                }
                // 设置
                if ui.button("⛭").clicked() {
                    ui_state.setting_window.trigger_show();
                }

                // 标题
                let (title_rect, res) =
                    ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());

                ui.allocate_ui_at_rect(title_rect, |ui| {
                    ui.with_layout(
                        Layout::centered_and_justified(Direction::LeftToRight),
                        |ui| {
                            ui.label(APP_NAME);
                        },
                    );
                });

                if res.double_clicked() {
                    player_event.send(PlayerEvent::Maximize);
                } else if res.dragged() {
                    // 当拖动时, 如果不判断drag_delta, 直接进行 drag_window, 会导致 double_clicked 无法触发
                    let delta = res.drag_delta();
                    if delta.x != 0.0 && delta.y != 0.0 {
                        player_event.send(PlayerEvent::DragWindow);
                    }
                }
            });
        });

        ui_state.titlebar.style_ui(ctx);
    }
}
