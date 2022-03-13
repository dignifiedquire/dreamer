use egui::{Color32, Context, Frame, SidePanel};

use crate::state::AppState;

use super::{accounts, chat_list};

pub fn render_sidebar(ctx: &Context, state: &AppState) {
    let bg = Color32::from_rgb(22, 10, 76);

    SidePanel::left("sidebar")
        .frame(Frame::default().fill(bg))
        .default_width(330.)
        .min_width(330.)
        .max_width(330.)
        .resizable(false)
        .show(ctx, |ui| {
            accounts::render(ui, state);
            chat_list::render(ui, state);
        });
}
