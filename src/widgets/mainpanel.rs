use std::path::PathBuf;

use egui::{
    load::SizedTexture, style::Margin, CentralPanel, Color32, Context, Frame, Response, RichText,
    Rounding, TextEdit, TopBottomPanel, Ui, Vec2, Widget,
};
use egui_extras::{Column, TableBuilder};
use epaint::{FontId, Stroke, TextureHandle};
use log::{info, warn};

use crate::{
    app::{FONT_LIGHT, FONT_REGULAR, FONT_SEMI_BOLD},
    dc::types::{ChatMessage, InnerChatMessage, SharedState, Viewtype},
    image,
    state::{AppState, Command},
    ACCENT_COLOR,
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
                        .fill(Color32::LIGHT_GRAY)
                        .inner_margin(Margin::same(2.)),
                )
                .show_inside(ui, |ui| {
                    ui.with_layout(
                        egui::Layout::top_down_justified(egui::Align::Center),
                        |ui| {
                            let response =
                                ui.add(egui::TextEdit::singleline(&mut state.current_input));
                            if response.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                            {
                                let message = std::mem::take(&mut state.current_input);
                                if message.len() > 0 {
                                    state.send_command(Command::SendTextMessage(message));
                                }

                                let text_edit_id = response.id;

                                // reselect focus
                                ui.ctx().memory_mut(|m| m.request_focus(text_edit_id));
                            }
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
                            TableBuilder::new(ui)
                                .column(Column::remainder().at_least(100.0))
                                .stick_to_bottom(true)
                                .auto_shrink(false)
                                .body(|mut body| {
                                    info!("rendering body");
                                    let shared_state = state.shared_state();
                                    let msgs = &shared_state.message_list.messages;

                                    let mut ui_cache = state.ui_cache.blocking_write();
                                    let width = body.widths()[0];

                                    let ctx = body.ui_mut().ctx().clone();
                                    let mut cache_hits = 0;
                                    let row_heights = msgs.iter().map(|msg| {
                                        let id = msg.id().unwrap_or(u32::MAX);
                                        if let Some(height) = ui_cache.get_message_height(id, width)
                                        {
                                            cache_hits += 1;
                                            height
                                        } else {
                                            let height = calc_height(
                                                &state,
                                                &shared_state.shared_state,
                                                &ctx,
                                                width,
                                                msg,
                                            );
                                            ui_cache.set_message_height(id, width, height);
                                            height
                                        }
                                    });

                                    body.heterogeneous_rows(row_heights, |row_index, mut row| {
                                        let msg = msgs[row_index].clone();
                                        row.col(|ui| {
                                            ui.add(ChatMessageWidget {
                                                state: state.clone(),
                                                msg,
                                            });
                                        });
                                    });
                                    info!(
                                        "inserted {} rows ({} cache hits)",
                                        msgs.len(),
                                        cache_hits
                                    );
                                });
                        });
                    });
                });
        });
}

fn calc_height(
    state: &AppState,
    shared_state: &SharedState,
    ctx: &Context,
    width: f32,
    msg: &ChatMessage,
) -> f32 {
    // TODO: use font sizes
    match msg {
        ChatMessage::Message(msg) => {
            if msg.is_info {
                // single row
                18.
            } else if msg.is_first {
                // avatar
                20. + calc_line_height(state, shared_state, ctx, width, 45., msg).max(25.)
            } else {
                calc_line_height(state, shared_state, ctx, width, 0., msg)
            }
        }
        ChatMessage::DayMarker(_) => {
            // single row
            18.
        }
    }
}

fn calc_line_height(
    state: &AppState,
    shared_state: &SharedState,
    ctx: &Context,
    width: f32,
    left_padding: f32,
    msg: &InnerChatMessage,
) -> f32 {
    // TODO: Load image and calculate size
    let image_size = if msg.viewtype == Viewtype::Image || msg.viewtype == Viewtype::Gif {
        if let Some(image) = msg
            .file
            .clone()
            .and_then(|path| load_image(state, shared_state, ctx, msg.id, path))
        {
            let max_width = width - 10.;
            let [_, height] = calc_image_size(&image, max_width);
            height
        } else {
            200.
        }
    } else {
        0.
    };

    let top_margin = 10.;
    let font_size = 14.;

    let total_text_len = msg.text.len()
        + msg
            .quote
            .as_ref()
            .and_then(|q| Some(q.text.clone()))
            .map(|t| t.len())
            .unwrap_or(0);

    let text_height = if total_text_len > 0 {
        // TODO: less naive
        let num_chars = total_text_len as f32;
        let num_chars_per_line = (width - left_padding) / font_size;
        let num_lines = num_chars / num_chars_per_line;
        num_lines * (font_size + 2.)
    } else {
        0.
    };
    top_margin + text_height + image_size
}

struct ChatMessageWidget {
    state: AppState,
    msg: ChatMessage,
}

impl Widget for ChatMessageWidget {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.scope(|ui| match &self.msg {
            ChatMessage::Message(msg) => {
                if msg.is_info {
                    view_info_message(ui, &self.state, msg);
                } else if msg.is_first {
                    view_avatar_message(ui, &self.state, msg);
                } else {
                    view_simple_message(ui, &self.state, msg);
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
                                time.with_timezone(&chrono::Local)
                                    .format("%d-%m-%Y")
                                    .to_string(),
                            );
                        });
                });
            }
        })
        .response
    }
}

/// Renders an info message.
fn view_info_message(ui: &mut Ui, _state: &AppState, msg: &InnerChatMessage) -> Response {
    ui.vertical_centered(|ui| {
        let text_color = Color32::from_rgb(41, 51, 63);

        if !msg.text.is_empty() {
            ui.label(
                RichText::new(msg.text.clone())
                    .size(14.)
                    .color(text_color)
                    .family(egui::FontFamily::Name(FONT_REGULAR.into())),
            );
        } else {
            warn!("missing text on info message");
        }
    })
    .response
}

/// Renders a message with avatar.
fn view_avatar_message(ui: &mut Ui, state: &AppState, msg: &InnerChatMessage) -> Response {
    ui.add_space(10.);

    ui.horizontal(|ui| {
        let text_color = Color32::from_rgb(41, 51, 63);
        let shared_state = state.shared_state();
        let account_id = shared_state
            .shared_state
            .selected_account
            .unwrap_or_default();
        let chat_id = shared_state
            .shared_state
            .selected_chat_id
            .unwrap_or_default();
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
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(&msg.from_first_name)
                        .family(egui::FontFamily::Name(FONT_SEMI_BOLD.into()))
                        .size(12.)
                        .color(text_color),
                );
                ui.label(
                    RichText::new(
                        msg.timestamp
                            .map(|t| t.with_timezone(&chrono::Local).format("%H:%M").to_string())
                            .unwrap_or_default(),
                    )
                    .family(egui::FontFamily::Name(FONT_LIGHT.into()))
                    .size(12.)
                    .color(text_color),
                );
            });
            view_inner_message(ui, state, &shared_state.shared_state, msg);
        });
    })
    .response
}

/// Renders a message without avatar, just the content
fn view_simple_message(ui: &mut Ui, state: &AppState, msg: &InnerChatMessage) -> Response {
    ui.horizontal(|ui| {
        ui.add_space(48.);
        let shared_state = state.shared_state();
        view_inner_message(ui, state, &shared_state.shared_state, msg);
    })
    .response
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
            if let Some(text) = msg.quote.as_ref().map(|q| q.text.clone()) {
                // TODO: render other types than text

                ui.horizontal(|ui| {
                    ui.add_space(10.);
                    ui.horizontal_wrapped(|ui| {
                        ui.add(selectable_text(
                            &mut text.as_str(),
                            14.,
                            FONT_LIGHT,
                            text_color,
                        ));
                    });
                });
            }

            match msg.viewtype {
                Viewtype::Image | Viewtype::Gif => {
                    if let Some(path) = msg.file.clone() {
                        if let Some(image) = load_image(state, shared_state, ui.ctx(), msg.id, path)
                        {
                            let max_width = ui.available_width() - 10.;
                            let size = calc_image_size(&image, max_width);
                            ui.image(SizedTexture::new(image.id(), size));
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
            if !msg.text.is_empty() {
                ui.add(selectable_text(
                    &mut msg.text.as_str(),
                    14.,
                    FONT_REGULAR,
                    text_color,
                ));
            }
        });
    });
}

fn calc_image_size(image: &TextureHandle, max_width: f32) -> [f32; 2] {
    let image_size = image.size();
    if max_width < image_size[0] as f32 {
        // too wide, scale down
        let factor = image_size[0] as f32 / max_width;
        [max_width, image_size[1] as f32 / factor]
    } else {
        // wide enough
        [image_size[0] as f32, image_size[1] as f32]
    }
}

fn load_image(
    state: &AppState,
    shared_state: &SharedState,
    ctx: &Context,
    msg_id: u32,
    path: PathBuf,
) -> Option<TextureHandle> {
    let account_id = shared_state.selected_account.unwrap_or_default();
    let chat_id = shared_state.selected_chat_id.unwrap_or_default();
    let id = format!("image-{}-{}-{}", account_id, chat_id, msg_id);

    state.get_or_load_image(ctx, id, move |_name| image::load_image_from_path(&path))
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
