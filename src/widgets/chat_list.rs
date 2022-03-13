use std::borrow::Cow;

use egui::{style::Margin, Color32, Frame, RichText, ScrollArea, Sense, SidePanel, Stroke, Ui};

use crate::{
    app::{FONT_SF_PRO_REGULAR, FONT_SF_PRO_SEMIBOLD},
    dc::types::{ChatState, SharedState},
    image,
    state::{AppState, Command},
};

pub fn render(ui: &mut Ui, state: &AppState) {
    SidePanel::right("chatlist")
        .frame(Frame::default().fill(Color32::WHITE))
        .min_width(280.)
        .max_width(280.)
        .resizable(false)
        .show_inside(ui, |ui| {
            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        let shared_state = state.shared_state();
                        let chats = &shared_state.chat_list;

                        for chat in &chats.chats {
                            let bg_color =
                                if Some(chat.id) == shared_state.shared_state.selected_chat_id {
                                    Color32::from_rgb(236, 238, 249)
                                } else {
                                    Color32::TRANSPARENT
                                };

                            egui::Frame::none()
                                .fill(bg_color)
                                .margin(Margin::same(5.0))
                                .show(ui, |ui| {
                                    view_chat(ui, state, &shared_state.shared_state, chat)
                                });
                        }
                    });
                })
        });
}

fn truncate(text: &String, len: usize) -> Cow<'_, String> {
    if text.len() <= len {
        Cow::Borrowed(text)
    } else {
        Cow::Owned(format!(
            "{} ...",
            text.chars().take(len).collect::<String>()
        ))
    }
}

fn view_chat(ui: &mut Ui, state: &AppState, shared_state: &SharedState, chat: &ChatState) {
    let response = ui
        .add_enabled_ui(true, |ui| {
            ui.horizontal(|ui| {
                ui.set_width(280.);

                let account_id = shared_state.selected_account.unwrap_or_default();
                let chat_id = chat.id;
                let id = format!("profile-chat-image-{}-{}", account_id, chat_id);
                let image = state.get_or_load_image(ui.ctx(), id, |_name| {
                    if let Some(ref path) = chat.profile_image {
                        image::load_image_from_path(path).unwrap()
                    } else {
                        image::default_avatar(&chat.name, chat.color)
                    }
                });

                ui.image(image.id(), [40., 40.]);

                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(truncate(&chat.name, 20).as_ref())
                            .family(egui::FontFamily::Name(FONT_SF_PRO_SEMIBOLD.into()))
                            .size(16.),
                    );

                    ui.label(
                        RichText::new(truncate(&chat.preview, 30).as_ref())
                            .family(egui::FontFamily::Name(FONT_SF_PRO_REGULAR.into()))
                            .size(14.),
                    );
                });

                ui.scope(|ui| {
                    ui.visuals_mut().widgets.noninteractive.bg_stroke =
                        Stroke::new(1., Color32::from_rgb(236, 237, 241));
                    ui.separator();
                });
            })
        })
        .response;

    let response = response.interact(Sense::click());
    if response.clicked() {
        let account = shared_state.selected_account.unwrap();
        state.send_command(Command::SelectChat(account, chat.id));
    }
}
