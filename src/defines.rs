pub const APP_NAME: &str = "小小播放器";

pub const AUDIO_FRAME_QUEUE_SIZE: usize = 15;
pub const VIDEO_FRAME_QUEUE_SIZE: usize = 3;
pub const PLAY_MIN_INTERVAL: f64 = 0.05;

pub mod icons {
    pub const ICON_LOGO: &[u8] = include_bytes!("../misc/icons/logo.jpg");
    // pub const ICON_LIST: &[u8] = include_bytes!("../misc/icons/list.svg");
    // pub const ICON_MAXIMIZE: &[u8] = include_bytes!("../misc/icons/maximize.svg");
    // pub const ICON_MENU: &[u8] = include_bytes!("../misc/icons/menu.svg");
    // pub const ICON_MINUS: &[u8] = include_bytes!("../misc/icons/minus.svg");
    // pub const ICON_PAUSE: &[u8] = include_bytes!("../misc/icons/pause.svg");
    // pub const ICON_PLAY: &[u8] = include_bytes!("../misc/icons/play.svg");
    // pub const ICON_SETTINGS: &[u8] = include_bytes!("../misc/icons/settings.svg");
    // pub const ICON_SKIP_BACK: &[u8] = include_bytes!("../misc/icons/skip-back.svg");
    // pub const ICON_SKIP_FORWARD: &[u8] = include_bytes!("../misc/icons/skip-forward.svg");
    // pub const ICON_STOP: &[u8] = include_bytes!("../misc/icons/stop-circle.svg");
    // pub const ICON_X: &[u8] = include_bytes!("../misc/icons/x.svg");
    pub const ICON_IMAGE: &[u8] = include_bytes!("../misc/icons/image.jpeg");
}

pub mod fonts {
    pub const FONT_CHINESE: &[u8] = include_bytes!("../misc/fonts/DroidSansFallbackFull.ttf");
}
