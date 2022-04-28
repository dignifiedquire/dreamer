use egui::{Context, Frame, RichText};
use epaint::{Color32, Stroke};

use crate::{
    state::{AppState, Command},
    ACCENT_COLOR, DOUBLE_LIGHT_GRAY,
};

// TODO:
// - Formvalidation
#[derive(Debug, Default)]
pub struct AddAccount {
    email: String,
    password: String,
}

impl AddAccount {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn ui(&mut self, ctx: &Context, state: &AppState) {
        let size = ctx.available_rect();

        // TODO: enable outsice click of popup to close
        egui::Area::new("Login")
            .fixed_pos(egui::pos2(0., 0.))
            .show(ctx, |ui| {
                Frame::none().outer_margin(7.).show(ui, |ui| {
                    ui.set_width(size.width() - 14.);
                    ui.set_height(size.height() - 14.);

                    ui.with_layout(
                        egui::Layout::top_down_justified(egui::Align::Center),
                        |ui| {
                            ui.set_max_width(250.);
                            Frame::none()
                                .fill(*DOUBLE_LIGHT_GRAY)
                                .inner_margin(7.)
                                .stroke(Stroke::new(1., Color32::BLACK))
                                .show(ui, |ui| {
                                    ui.vertical(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.with_layout(
                                                egui::Layout::top_down_justified(
                                                    egui::Align::Center,
                                                ),
                                                |ui| {
                                                    ui.label(
                                                        RichText::new("Login")
                                                            .size(25.)
                                                            .color(*ACCENT_COLOR),
                                                    );
                                                },
                                            );
                                            if ui.button("close").clicked() {
                                                state.send_command(Command::CloseLoginOrImport);
                                            }
                                        });
                                        ui.label("Username");
                                        ui.text_edit_singleline(&mut self.email);
                                        ui.label("Password");
                                        ui.text_edit_singleline(&mut self.password);
                                        ui.add_space(5.);

                                        if ui.button("Login").clicked() {
                                            state.send_command(Command::Login(
                                                self.email.clone(),
                                                self.password.clone(),
                                            ));
                                        }
                                        if ui.button("Import").clicked() {
                                            state.send_command(Command::OpenDialoge);
                                        }
                                    });
                                });
                        },
                    );
                });
            });
    }
}
