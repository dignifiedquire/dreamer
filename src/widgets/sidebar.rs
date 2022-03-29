use egui::{Context, Frame, SidePanel};
use epaint::Color32;

use super::{accounts, chat_list};
use crate::state::AppState;

pub fn render_sidebar(ctx: &Context, state: &AppState) {
    SidePanel::left("sidebar")
        .frame(Frame::default().fill(Color32::from_gray(250)))
        .default_width(330.)
        .min_width(330.)
        .max_width(330.)
        .resizable(false)
        .show(ctx, |ui| {
            accounts::render(ui, state);
            chat_list::render(ui, state);
        });
}
