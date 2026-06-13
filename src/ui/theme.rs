use ratatui::style::Color;

// Catppuccin Mocha-inspired palette
pub struct Theme;

impl Theme {
    pub const TEXT: Color = Color::Rgb(205, 214, 244);
    pub const SUBTEXT: Color = Color::Rgb(166, 173, 200);
    pub const GREEN: Color = Color::Rgb(166, 227, 161);
    pub const RED: Color = Color::Rgb(243, 139, 168);
    pub const BLUE: Color = Color::Rgb(137, 180, 250);
    pub const YELLOW: Color = Color::Rgb(249, 226, 175);
    pub const MAUVE: Color = Color::Rgb(203, 166, 247);
    pub const TEAL: Color = Color::Rgb(148, 226, 213);
    pub const SURFACE: Color = Color::Rgb(49, 50, 68);
    pub const OVERLAY: Color = Color::Rgb(69, 71, 90);
    pub const BASE: Color = Color::Rgb(30, 30, 46);
    pub const CURSOR: Color = Color::Rgb(88, 91, 112);
}
