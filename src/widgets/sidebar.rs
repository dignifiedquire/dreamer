use egui::{Context, Frame, SidePanel};

use super::{accounts, chat_list};
use crate::{state::AppState, ACCENT_COLOR};

pub fn render_sidebar(ctx: &Context, state: &AppState) {
    SidePanel::left("sidebar")
        .frame(Frame::default().fill(*ACCENT_COLOR))
        .default_width(330.)
        .min_width(330.)
        .max_width(330.)
        .resizable(false)
        .show(ctx, |ui| {
            accounts::render(ui, state);
            chat_list::render(ui, state);
        });
}
