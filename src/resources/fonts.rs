use bevy_egui::egui::{FontData, FontDefinitions, FontFamily};

use crate::defines::fonts::FONT_CHINESE;

pub fn load_fonts() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    let chinese_data = FontData::from_static(FONT_CHINESE);
    fonts.font_data.insert("chinese".into(), chinese_data);

    fonts
        .families
        .get_mut(&FontFamily::Proportional)
        .unwrap()
        .insert(0, "chinese".into());

    fonts
}
