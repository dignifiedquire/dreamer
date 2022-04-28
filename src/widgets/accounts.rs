use egui::{Color32, CursorIcon, Frame, Rounding, ScrollArea, SidePanel, Stroke, Ui, Vec2};

use crate::{
    state::{AppState, Command},
    ACCENT_COLOR,
};

use super::avatar::Avatar;

pub fn render(ui: &mut Ui, state: &AppState) {
    let shared_state = state.shared_state();
    let accounts = &shared_state.shared_state.accounts;
    SidePanel::left("accountlist")
        .frame(Frame::default().fill(*ACCENT_COLOR))
        .resizable(false)
        .max_width(50.)
        .min_width(50.)
        .show_inside(ui, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                for (id, account) in accounts.iter() {
                    let name = account
                        .display_name
                        .as_ref().unwrap_or(&account.email);

                    let is_active = Some(id) == shared_state.shared_state.selected_account.as_ref();

                    let fill = if is_active {
                        Color32::from_rgb(33, 32, 92)
                    } else {
                        Color32::TRANSPARENT
                    };
                    ui.add_space(10.);
                    ui.vertical_centered(|ui| {
                        ui.set_height(40.);
                        let response = ui.add(
                            Avatar::new(name.to_string(), Vec2::splat(40.), fill)
                                .stroke(Stroke::new(1., Color32::WHITE))
                                .rounding(Rounding::same(5.)),
                        );
                        if response.clicked() {
                            state.send_command(Command::SelectAccount(*id));
                        }
                        if response.hovered() {
                            ui.output().cursor_icon = CursorIcon::PointingHand;
                        }
                    });
                }

                ui.add_space(10.);

                ui.vertical_centered(|ui| {
                    ui.set_height(40.);
                    let response = ui.add(
                        Avatar::new(
                            "+".to_owned(),
                            Vec2::splat(40.),
                            Color32::from_rgb(33, 32, 92),
                        )
                        .stroke(Stroke::new(1., Color32::WHITE))
                        .rounding(Rounding::same(5.)),
                    );
                    if response.clicked() {
                        state.send_command(Command::OpenLoginOrImport);
                    }
                    if response.hovered() {
                        ui.output().cursor_icon = CursorIcon::PointingHand;
                    }
                });
            });
        });
}
