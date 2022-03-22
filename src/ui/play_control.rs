use bevy::prelude::{info, EventWriter};
use bevy_egui::egui::{vec2, Align2, Area, Color32, Context, Label, Sense, Slider, Ui, Widget};

use crate::resources::event::{PlayEvent, PlayerEvent};

use super::ui_state::UiState;

#[derive(Default)]
pub struct VideoControl {
    pub mute: bool,
}

impl VideoControl {
    pub fn show(
        ctx: &Context,
        _ui: &mut Ui,
        ui_state: &mut UiState,
        player_evt: &mut EventWriter<PlayerEvent>,
        play_evt: &mut EventWriter<PlayEvent>,
    ) {
        Area::new("video_ctl_1")
            .movable(false)
            .anchor(Align2::CENTER_BOTTOM, vec2(0., -8.))
            .show(ctx, |ui| {
                ui.visuals_mut().widgets.inactive.bg_fill = Color32::from_rgb(65, 105, 178);
                ui.horizontal(|ui| {
                    if Label::new("⏮").sense(Sense::click()).ui(ui).clicked() {
                        play_evt.send(PlayEvent::Previous);
                    }
                    if ui_state.pause {
                        if Label::new("⏵").sense(Sense::click()).ui(ui).clicked() {
                            play_evt.send(PlayEvent::Pause(false));
                        }
                    } else {
                        if Label::new("⏸").sense(Sense::click()).ui(ui).clicked() {
                            play_evt.send(PlayEvent::Pause(true));
                        }
                    }
                    if Label::new("⏹").sense(Sense::click()).ui(ui).clicked() {
                        player_evt.send(PlayerEvent::Stop);
                    }
                    if Label::new("⏭").sense(Sense::click()).ui(ui).clicked() {
                        play_evt.send(PlayEvent::Next);
                    }
                    ui.add_space(10.);

                    let (mute_icon, mute) = if ui_state.mute || ui_state.volume == 0.0 {
                        ("🔇", false)
                    } else {
                        ("🔈", true)
                    };
                    if Label::new(mute_icon).sense(Sense::click()).ui(ui).clicked() {
                        play_evt.send(PlayEvent::Mute(mute));
                    }
                    if ui
                        .add(Slider::new(&mut ui_state.volume, 0.0..=1.0).show_value(false))
                        .changed()
                    {
                        play_evt.send(PlayEvent::Volume(ui_state.volume));
                    }
                });
            });

        Area::new("video_ctl_2")
            .movable(false)
            .anchor(Align2::RIGHT_BOTTOM, vec2(-20., -8.))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if Label::new("☯").sense(Sense::click()).ui(ui).clicked() {
                        info!("☯");
                        ui_state.open_list = !ui_state.open_list;
                    }
                    if Label::new("🗁").sense(Sense::click()).ui(ui).clicked() {
                        info!("🗁");
                        player_evt.send(PlayerEvent::OpenFolder);
                    }
                });
            });
    }
}
