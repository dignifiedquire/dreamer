use egui::{style::Margin, Button, Color32, Frame, RichText, ScrollArea, SidePanel, Stroke, Ui};

use crate::state::{AppState, Command};

pub fn render(ui: &mut Ui, state: &AppState) {
    let shared_state = state.shared_state();
    let accounts = &shared_state.shared_state.accounts;
    let bg = Color32::from_rgb(22, 10, 76);
    SidePanel::left("accountlist")
        .frame(Frame::default().fill(bg))
        .show_inside(ui, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                for (id, account) in accounts.iter() {
                    let name = account
                        .display_name
                        .as_ref()
                        .unwrap_or_else(|| &account.email);

                    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                        Frame::none().margin(Margin::same(4.)).show(ui, |ui| {
                            let button = Button::new(
                                RichText::new(name.chars().nth(0).unwrap())
                                    .color(Color32::WHITE)
                                    .size(30.),
                            )
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::new(1., Color32::from_rgb(236, 237, 241)));
                            if ui.add(button).clicked() {
                                state.send_command(Command::SelectAccount(*id));
                            }
                        });
                    });
                }
            });
        });
}
