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
}
