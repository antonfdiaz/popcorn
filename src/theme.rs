use gpui_kit::rgb;

pub const APP_BG: u32 = 0x17181d;
pub const SURFACE: u32 = 0x262626;
pub const SURFACE_ALT: u32 = 0x2d2e2e;
pub const SURFACE_HOVER: u32 = 0x3a3a3b;
pub const BORDER: u32 = 0x404040;
pub const GRID: u32 = 0x2b2e37;
pub const TEXT: u32 = 0xe5e7eb;
pub const TEXT_MUTED: u32 = 0x9aa1ae;
pub const TEXT_DIM: u32 = 0x6d7481;
pub const ACCENT: u32 = 0xff7a45;
pub const ACCENT_SOFT: u32 = 0x3c2b29;
pub const BLUE: u32 = 0x5b8cff;
pub const GREEN: u32 = 0x54c6a8;
pub const STAGE: u32 = 0xf2f4f7;

pub fn color(value: u32) -> gpui_kit::Rgba {
    rgb(value)
}

pub fn init(cx: &mut gpui_kit::App) {
    use gpui_kit::component::{Theme, ThemeMode};
    Theme::change(ThemeMode::Dark, None, cx);
    Theme::update(cx, |theme| {
        theme.background = color(APP_BG).into();
        theme.foreground = color(TEXT).into();
        theme.primary = color(ACCENT).into();
        theme.primary_foreground = color(APP_BG).into();
        theme.secondary = color(SURFACE_HOVER).into();
        theme.secondary_foreground = color(TEXT).into();
        theme.border = color(BORDER).into();
    });
}
