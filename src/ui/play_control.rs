use bevy::prelude::{info, EventWriter};
use bevy_egui::egui::{vec2, Align2, Area, Color32, Context, Label, Sense, Slider, Ui, Widget};

use crate::resources::event::PlayerEvent;

use super::ui_state::UiState;

pub struct VideoControl {
    pub volume: u8,
    pub mute: bool,
}

impl VideoControl {
    pub fn new() -> Self {
        Self {
            volume: 0,
            mute: false,
        }
    }

    pub fn show(
        ctx: &Context,
        _ui: &mut Ui,
        ui_state: &mut UiState,
        play_event: &mut EventWriter<PlayerEvent>,
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
                        play_event.send(PlayerEvent::Previous);
                    }
                    // if Label::new("⏵").sense(Sense::click()).ui(ui).clicked() {
                    //     play_event.send(PlayerEvent::Start);
                    // }
                    if Label::new("⏸").sense(Sense::click()).ui(ui).clicked() {
                        play_event.send(PlayerEvent::Pause);
                    }
                    if Label::new("⏹").sense(Sense::click()).ui(ui).clicked() {
                        play_event.send(PlayerEvent::Stop);
                    }
                    if Label::new("⏭").sense(Sense::click()).ui(ui).clicked() {
                        play_event.send(PlayerEvent::Next);
                    }
                    ui.add_space(10.);

                    let (mute_icon, mute) = if play_control.mute || play_control.volume == 0 {
                        ("🔇", false)
                    } else {
                        ("🔈", true)
                    };
                    if Label::new(mute_icon).sense(Sense::click()).ui(ui).clicked() {
                        play_event.send(PlayerEvent::Mute(mute));
                    }
                    if ui
                        .add(Slider::new(&mut play_control.volume, 0..=100).show_value(false))
                        .changed()
                    {
                        play_event.send(PlayerEvent::Volume(play_control.volume));
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
                        play_event.send(PlayerEvent::OpenFolder);
                    }
                });
            });
    }
}
