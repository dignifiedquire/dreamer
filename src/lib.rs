use egui::Color32;
use lazy_static::lazy_static;
pub mod app;
mod dc;
mod image;
// mod scheduler;
mod state;
mod widgets;

lazy_static! {
    pub static ref ACCENT_COLOR: Color32 = Color32::from_rgb(22, 10, 76);
    pub static ref ACCENT_COLOR_STRONG: Color32 = Color32::from_rgb(62, 29, 211);
    pub static ref DOUBLE_LIGHT_GRAY: Color32 = Color32::from_gray(250);
}
