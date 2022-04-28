use egui::{Context, Frame, SidePanel};

use super::{accounts, chat_list};
use crate::{state::AppState, DOUBLE_LIGHT_GRAY};

pub fn render_sidebar(ctx: &Context, state: &AppState, max_width: f32) {
    SidePanel::left("sidebar")
        .frame(Frame::default().fill(*DOUBLE_LIGHT_GRAY))
        .default_width(max_width)
        .max_width(max_width)
        .resizable(true)
        .show(ctx, |ui| {
            accounts::render(ui, state);
            chat_list::render(ui, state);
        });
}
