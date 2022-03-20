use bevy_egui::egui::{
    vec2, widgets::Image, Color32, ColorImage, Context, InnerResponse, Sense, TextureHandle, Ui,
};
use egui_extras::RetainedImage;

use crate::{defines::icons::ICON_IMAGE, services::player::PlayState};

use super::ui_state::UiState;

pub struct PlayContentView {}

impl PlayContentView {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(ctx: &Context, ui: &mut Ui, ui_state: &mut UiState) {
        ui.set_style(ui_state.theme.blue_video_content_style());
        // 视频状态
        if let PlayState::Video(video) = &ui_state.state {
            // 居中
            let InnerResponse { inner: _, response } = ui.vertical_centered_justified(|ui| {
                let width = video.width as f32;
                let height = video.height as f32;

                ui_state.video_texture = Some(ctx.load_texture(
                    "play_content_texture",
                    ColorImage::from_rgba_unmultiplied(
                        [video.width, video.height],
                        video.data.as_slice(),
                    ),
                ));

                if let Some(texture) = &ui_state.video_texture {
                    let w = ui.available_width();
                    let h = ui.available_height();
                    let img_height = w * height as f32 / width as f32;
                    let img_size = vec2(w, img_height);
                    let space_amount = (h - img_height) / 2.0;
                    ui.add_space(space_amount);
                    ui.image(texture, img_size);
                }
            });

            let response = ui.interact(response.rect, ui.id(), Sense::click());

            response.context_menu(|ui| {
                if ui.button("静音").clicked() {
                    ui.close_menu();
                }
                if ui.button("下一首").clicked() {
                    ui.close_menu();
                }
            });
        } else if let PlayState::Stopped = &ui_state.state {
            ui.heading("Stopped");
        }
    }
}