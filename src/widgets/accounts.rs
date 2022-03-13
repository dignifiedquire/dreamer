use egui::{Button, Color32, Frame, Layout, RichText, ScrollArea, SidePanel, Stroke, Ui};

use crate::state::{AppState, Command};

const MIN_WIDTH: f32 = 50.;

pub fn render(ui: &mut Ui, state: &AppState) {
    let shared_state = state.shared_state();
    let accounts = &shared_state.shared_state.accounts;
    let bg = Color32::from_rgb(22, 10, 76);

    SidePanel::left("accountlist")
        .frame(Frame::default().fill(bg))
        .show_inside(ui, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                ui.set_width(MIN_WIDTH);
                ui.visuals_mut().extreme_bg_color = bg;
                ui.with_layout(Layout::top_down_justified(egui::Align::Min), |ui| {
                    for (id, account) in accounts.iter() {
                        let name = account
                            .display_name
                            .as_ref()
                            .unwrap_or_else(|| &account.email);

                        let button = Button::new(RichText::new(name.chars().nth(0).unwrap()))
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::new(1., Color32::from_rgb(236, 237, 241)));
                        if ui.add(button).clicked() {
                            state.send_command(Command::SelectAccount(*id));
                        }
                    }
                });
            });
        });
}
