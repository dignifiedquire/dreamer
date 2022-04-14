use egui::{
    style::Margin, CentralPanel, Color32, Context, Frame, RichText, Rounding, ScrollArea, TextEdit,
    TopBottomPanel, Ui, Vec2,
};
use epaint::{FontId, Stroke};
use log::warn;

use crate::{
    app::{FONT_LIGHT, FONT_REGULAR, FONT_SEMI_BOLD},
    dc::types::{ChatMessage, InnerChatMessage, SharedState, Viewtype},
    image,
    state::{AppState, Command},
    ACCENT_COLOR, ACCENT_COLOR_STRONG,
};

use super::avatar::Avatar;

pub fn render_main_panel(ctx: &Context, state: &mut AppState) {
    CentralPanel::default()
        .frame(Frame::default().fill(Color32::WHITE))
        .show(ctx, |ui| {
            // show the input-field for new messages
            TopBottomPanel::bottom("input")
                .frame(
                    Frame::default()
                        .inner_margin(Margin::same(10.))
                        .fill(Color32::WHITE),
                )
                .show_inside(ui, |ui| {
                    ui.with_layout(
                        egui::Layout::top_down_justified(egui::Align::Center),
                        |ui| {
                            Frame::none()
                                .inner_margin(2.)
                                .fill(Color32::GRAY)
                                .show(ui, |ui| {
                                    let response = ui
                                        .add(egui::TextEdit::singleline(&mut state.current_input));
                                    if response.lost_focus()
                                        && ui.input().key_pressed(egui::Key::Enter)
                                    {
                                        let message = std::mem::take(&mut state.current_input);
                                        if message.len() > 0 {
                                            state.send_command(Command::SendTextMessage(message));
                                        }

                                        let text_edit_id = response.id;

                                        // reselect focus
                                        ui.ctx().memory().request_focus(text_edit_id);
                                    }
                                })
                        },
                    )
                });

            TopBottomPanel::top("chat")
                .frame(Frame::default().fill(Color32::WHITE))
                .min_height(ui.available_height())
                .max_height(ui.available_height())
                .show_inside(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.vertical(|ui| {
                            let chat = state.shared_state().shared_state.selected_chat.clone();
                            if let Some(chat) = chat {
                                ui.set_min_height(50.);
                                Frame::none()
                                    .fill(*ACCENT_COLOR)
                                    .inner_margin(5.)
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());

                                        ui.heading(
                                            RichText::new(format!("#{}", chat.name))
                                                .color(Color32::WHITE),
                                        );
                                        ui.label(
                                            RichText::new(format!(
                                                "Members: {}",
                                                chat.member_count
                                            ))
                                            .color(Color32::LIGHT_GRAY),
                                        );
                                    });
                            }
                        });
                        Frame::none().inner_margin(Margin::same(5.)).show(ui, |ui| {
                            ScrollArea::vertical()
                                .stick_to_bottom()
                                .auto_shrink([false; 2])
                                .show(ui, |ui| {
                                    ui.vertical(|ui| {
                                        let shared_state = state.shared_state();
                                        for msg in &shared_state.message_list.messages {
                                            egui::Frame::none()
                                                .inner_margin(Margin::same(0.))
                                                .show(ui, |ui| {
                                                    view_message(
                                                        ui,
                                                        state,
                                                        &shared_state.shared_state,
                                                        msg,
                                                    )
                                                });
                                        }
                                    });
                                });
                        });
                    });
                });
        });
}

fn view_message(ui: &mut Ui, state: &AppState, shared_state: &SharedState, msg: &ChatMessage) {
    ui.scope(|ui| match msg {
        ChatMessage::Message(msg) => {
            if msg.is_info {
                view_info_message(ui, state, msg);
            } else if msg.is_first {
                view_avatar_message(ui, state, shared_state, msg);
            } else {
                view_simple_message(ui, state, shared_state, msg);
            }
        }
        ChatMessage::DayMarker(time) => {
            // FIXME: make gray backround only as big as needed (not full width)
            ui.vertical_centered(|ui| {
                Frame::none()
                    .fill(Color32::from_gray(250))
                    .rounding(4.)
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(time.format("%Y-%m-%d").to_string())
                                .color(*ACCENT_COLOR_STRONG),
                        );
                    });
            });
        }
    });
}

/// Renders an info message.
fn view_info_message(ui: &mut Ui, _state: &AppState, msg: &InnerChatMessage) {
    ui.vertical_centered(|ui| {
        let text_color = Color32::from_rgb(41, 51, 63);

        if let Some(ref text) = msg.text {
            ui.label(
                RichText::new(text)
                    .size(16.)
                    .color(text_color)
                    .family(egui::FontFamily::Name(FONT_REGULAR.into())),
            );
        } else {
            warn!("missing text on info message");
        }
    });
}

/// Renders a message with avatar.
fn view_avatar_message(
    ui: &mut Ui,
    state: &AppState,
    shared_state: &SharedState,
    msg: &InnerChatMessage,
) {
    ui.add_space(10.);

    ui.horizontal(|ui| {
        let text_color = Color32::from_rgb(41, 51, 63);

        let account_id = shared_state.selected_account.unwrap_or_default();
        let chat_id = shared_state.selected_chat_id.unwrap_or_default();
        let id = format!("profile-image-{}-{}-{}", account_id, chat_id, msg.from_id);

        let image = msg.from_profile_image.clone().and_then(|image_path| {
            state.get_or_load_image(ui.ctx(), id, move |_name| {
                image::load_image_from_path(&image_path)
            })
        });
        ui.add(
            Avatar::new(
                msg.from_first_name.to_string(),
                Vec2::splat(40.),
                image::color_from_u32(msg.from_color),
            )
            .rounding(Rounding::same(5.))
            .stroke(Stroke::new(1., Color32::WHITE))
            .image(image),
        );

        ui.vertical(|ui| {
            ui.label(
                RichText::new(&msg.from_first_name)
                    .family(egui::FontFamily::Name(FONT_SEMI_BOLD.into()))
                    .size(18.)
                    .color(text_color),
            );

            view_inner_message(ui, state, shared_state, msg);
        });
    });
}

/// Renders a message without avatar, just the content
fn view_simple_message(
    ui: &mut Ui,
    state: &AppState,
    shared_state: &SharedState,
    msg: &InnerChatMessage,
) {
    ui.horizontal(|ui| {
        ui.add_space(48.);
        view_inner_message(ui, state, shared_state, msg);
    });
}

fn view_inner_message(
    ui: &mut Ui,
    state: &AppState,
    shared_state: &SharedState,
    msg: &InnerChatMessage,
) {
    ui.horizontal_wrapped(|ui| {
        let text_color = Color32::from_rgb(41, 51, 63);
        ui.visuals_mut().override_text_color = Some(text_color);

        // TODO: render other message types

        ui.vertical(|ui| {
            if let Some(text) = msg.quote.as_ref().and_then(|q| q.text.as_ref()) {
                // TODO: render other types than text

                ui.horizontal(|ui| {
                    ui.add_space(10.);
                    ui.horizontal_wrapped(|ui| {
                        ui.add(selectable_text(
                            &mut text.as_str(),
                            18.,
                            FONT_LIGHT,
                            text_color,
                        ));
                    });
                });
            }

            match msg.viewtype {
                Viewtype::Image | Viewtype::Gif => {
                    if let Some(path) = msg.file.clone() {
                        let account_id = shared_state.selected_account.unwrap_or_default();
                        let chat_id = shared_state.selected_chat_id.unwrap_or_default();
                        let id = format!("image-{}-{}-{}", account_id, chat_id, msg.id);

                        if let Some(image) = state.get_or_load_image(ui.ctx(), id, move |_name| {
                            image::load_image_from_path(&path)
                        }) {
                            let max_width = ui.available_width() - 10.;
                            let image_size = image.size();

                            let size = if max_width < image_size[0] as f32 {
                                // too wide, scale down
                                let factor = image_size[0] as f32 / max_width;
                                [max_width, image_size[1] as f32 / factor]
                            } else {
                                // wide enough
                                [image_size[0] as f32, image_size[1] as f32]
                            };
                            ui.image(image.id(), size);
                        }
                    }
                }
                Viewtype::Audio
                | Viewtype::Sticker
                | Viewtype::Video
                | Viewtype::VideochatInvitation
                | Viewtype::Voice
                | Viewtype::Webxdc
                | Viewtype::File => {
                    let content = format!("{:?} not yet supported", msg.viewtype);
                    ui.add(selectable_text(
                        &mut content.as_str(),
                        14.,
                        FONT_REGULAR,
                        text_color,
                    ));
                }
                Viewtype::Unknown => {}
                Viewtype::Text => { /* Text rendering is done below */ }
            }

            // render additional in all cases text
            if let Some(ref text) = msg.text {
                ui.add(selectable_text(
                    &mut text.as_str(),
                    18.,
                    FONT_REGULAR,
                    text_color,
                ));
            }
        });
    });
}

fn selectable_text<'a>(
    content: &'a mut &'a str,
    size: f32,
    font_name: &str,
    color: Color32,
) -> TextEdit<'a> {
    TextEdit::multiline(content)
        .font(FontId::new(size, egui::FontFamily::Name(font_name.into())))
        .text_color(color)
        .desired_rows(1)
        .desired_width(f32::INFINITY)
        .frame(false)
}
