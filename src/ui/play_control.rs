use bevy::prelude::{info, EventWriter};
use bevy_egui::egui::{vec2, Align2, Area, Color32, Context, Label, Sense, Slider, Ui, Widget};

use crate::resources::event::{PlayEvent, PlayerEvent};

use super::ui_state::UiState;

pub struct VideoControl {
    pub volume: f32,
    pub mute: bool,
}

impl VideoControl {
    pub fn new() -> Self {
        Self {
            volume: 1.0,
            mute: false,
        }
    }

    pub fn show(
        ctx: &Context,
        _ui: &mut Ui,
        ui_state: &mut UiState,
        player_evt: &mut EventWriter<PlayerEvent>,
        play_evt: &mut EventWriter<PlayEvent>,
    ) {
        let play_control = &mut ui_state.play_control;
        let play_list_view = &mut ui_state.play_list_view;

        Area::new("video_ctl_1")
            .movable(false)
            .anchor(Align2::CENTER_BOTTOM, vec2(0., -8.))
            .show(ctx, |ui| {
                ui.visuals_mut().widgets.inactive.bg_fill = Color32::from_rgb(65, 105, 178);
                ui.horizontal(|ui| {
                    if Label::new("⏮").sense(Sense::click()).ui(ui).clicked() {
                        play_evt.send(PlayEvent::Previous);
                    }
                    // if Label::new("⏵").sense(Sense::click()).ui(ui).clicked() {
                    //     player_evt.send(PlayerEvent::Start);
                    // }
                    if Label::new("⏸").sense(Sense::click()).ui(ui).clicked() {
                        play_evt.send(PlayEvent::Pause);
                    }
                    if Label::new("⏹").sense(Sense::click()).ui(ui).clicked() {
                        player_evt.send(PlayerEvent::Stop);
                    }
                    if Label::new("⏭").sense(Sense::click()).ui(ui).clicked() {
                        play_evt.send(PlayEvent::Next);
                    }
                    ui.add_space(10.);

                    let (mute_icon, mute) = if play_control.mute || play_control.volume == 0.0 {
                        ("🔇", false)
                    } else {
                        ("🔈", true)
                    };
                    if Label::new(mute_icon).sense(Sense::click()).ui(ui).clicked() {
                        play_evt.send(PlayEvent::Mute(mute));
                    }
                    if ui
                        .add(Slider::new(&mut play_control.volume, 0.0..=1.0).show_value(false))
                        .changed()
                    {
                        play_evt.send(PlayEvent::Volume(play_control.volume));
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
                        play_list_view.open = !play_list_view.open;
                    }
                    if Label::new("🗁").sense(Sense::click()).ui(ui).clicked() {
                        info!("🗁");
                        player_evt.send(PlayerEvent::OpenFolder);
                    }
                });
            });
    }
}
