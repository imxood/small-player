use std::path::Path;

use bevy::prelude::EventWriter;
use bevy_egui::egui::{
    Align, Context, Label, Layout, Sense, SidePanel, TextEdit, TextStyle, Widget,
};

use crate::resources::event::PlayerEvent;

use super::ui_state::UiState;

pub struct VideoListView {}

impl VideoListView {
    pub fn show(
        ctx: &Context,
        ui_state: &mut UiState,
        player_event: &mut EventWriter<PlayerEvent>,
    ) {
        if !ui_state.open_list {
            return;
        }

        SidePanel::right("right_side_panel")
            .min_width(200.0)
            .default_width(250.0)
            .show(ctx, |ui| {
                ui.with_layout(
                    Layout::top_down(Align::Min).with_cross_justify(true),
                    |ui| {
                        let play_list = ui_state.play_list.clone();
                        ui.label("密码:    ");
                        // ui.add(super::password::password(&mut self.password));
                        // ui.add(egui::TextEdit::singleline(&mut self.password).password(true));
                        let res = ui.add(
                            TextEdit::singleline(&mut ui_state.password)
                                .password(true)
                                .font(TextStyle::Body),
                        );
                        if res.changed() {
                            println!("password:{}", &ui_state.password);
                        }

                        ui.label(format!("文件数: {}", play_list.len()));

                        for video in play_list.iter() {
                            let filename = Path::new(video).file_name().unwrap().to_str().unwrap();
                            let res = Label::new(filename).sense(Sense::click()).ui(ui);
                            if res.clicked() {
                                ui_state.choose_file = Some(video.clone());
                            }

                            if res.double_clicked() {
                                player_event.send(PlayerEvent::Start(video.clone()));
                            }

                            res.context_menu(|ui| {
                                ui_state.choose_file = Some(video.clone());

                                if ui.button("播放").clicked() {
                                    player_event.send(PlayerEvent::Start(video.clone()));
                                    ui.close_menu();
                                    return;
                                }
                                if ui.button("移除").clicked() {
                                    ui_state.play_list.retain(|v| v != video);
                                    ui.close_menu();
                                    return;
                                }
                            });
                        }
                    },
                );
            });
    }
}
