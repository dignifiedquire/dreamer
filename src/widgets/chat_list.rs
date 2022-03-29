use std::borrow::Cow;

use egui::{
    style::Margin, Color32, Frame, RichText, Rounding, ScrollArea, Sense, SidePanel, Stroke, Ui,
    Vec2,
};

use crate::{
    app::{FONT_REGULAR, FONT_SEMI_BOLD},
    dc::types::{ChatState, SharedState},
    image,
    state::{AppState, Command},
};

use super::avatar::Avatar;

pub fn render(ui: &mut Ui, state: &AppState) {
    SidePanel::right("chatlist")
        .frame(Frame::default().inner_margin(2.))
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

                        let chat_len = chats.chats.len();

                        for (i, chat) in chats.chats.iter().enumerate() {
                            let bg_color =
                                if Some(chat.id) == shared_state.shared_state.selected_chat_id {
                                    Color32::from_rgb(229, 253, 255)
                                } else {
                                    Color32::TRANSPARENT
                                };

                            egui::Frame::none()
                                .fill(bg_color)
                                .inner_margin(Margin::same(5.0))
                                .show(ui, |ui| {
                                    view_chat(ui, state, &shared_state.shared_state, chat);
                                });

                            // add a seperator between all chats
                            if i < chat_len - 1 {
                                ui.vertical(|ui| {
                                    ui.scope(|ui| {
                                        ui.visuals_mut().widgets.noninteractive.bg_stroke =
                                            Stroke::new(1., Color32::from_rgb(236, 237, 241));
                                        ui.separator();
                                    });
                                });
                            }
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

                let image = chat.profile_image.clone().and_then(|image_path| {
                    state.get_or_load_image(ui.ctx(), id, move |_name| {
                        image::load_image_from_path(&image_path)
                    })
                });
                ui.add(
                    Avatar::new(
                        chat.name.to_string(),
                        Vec2::splat(40.),
                        image::color_from_u32(chat.color),
                    )
                    .rounding(Rounding::same(5.))
                    .stroke(Stroke::new(1., Color32::WHITE))
                    .image(image),
                );

                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(truncate(&chat.name, 20).as_ref())
                            .family(egui::FontFamily::Name(FONT_SEMI_BOLD.into()))
                            .size(16.),
                    );

                    ui.label(
                        RichText::new(truncate(&chat.preview, 30).as_ref())
                            .family(egui::FontFamily::Name(FONT_REGULAR.into()))
                            .size(14.),
                    );
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
