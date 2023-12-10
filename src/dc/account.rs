use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::types::{ChatItem, ChatMessage, ChatState, InnerChatMessage, Login, Viewtype};
use anyhow::{anyhow, bail, ensure, Context as _, Result};
use chrono::prelude::*;
use deltachat::chat::{ChatVisibility, MessageListOptions};
use deltachat::{
    chat::{self, Chat, ChatId},
    chatlist::Chatlist,
    contact::{Contact, ContactId},
    context::Context,
    message::{self, MsgId},
    EventType,
};
use futures::StreamExt;
use lazy_static::lazy_static;
use log::*;
use num_traits::{FromPrimitive, ToPrimitive};
use serde::Serialize;
use tokio::sync::RwLock;

#[cfg(not(target_os = "linux"))]
lazy_static! {
    pub static ref HOME_DIR: PathBuf = dirs::home_dir()
        .unwrap_or_else(|| "home".into())
        .join(".deltachat")
        .into();
}

#[cfg(target_os = "linux")]
lazy_static! {
    pub static ref HOME_DIR: PathBuf = dirs::home_dir()
        .unwrap_or_else(|| "home".into())
        .join(".config/DeltaChat/accounts")
        .into();
}

#[derive(Debug)]
pub struct Account {
    pub state: Arc<RwLock<AccountState>>,
}

#[derive(Debug)]
pub struct AccountState {
    pub logged_in: Login,
    pub selected_chat_id: Option<ChatId>,
    pub selected_chat: Option<ChatState>,
}

impl Account {
    pub fn new() -> Result<Self> {
        let account = Account {
            state: Arc::new(RwLock::new(AccountState {
                logged_in: Login::default(),
                selected_chat_id: None,
                selected_chat: None,
            })),
        };

        Ok(account)
    }

    pub async fn logged_in(&self) -> bool {
        self.state.read().await.logged_in == Login::Success
    }

    pub async fn import(&self, context: &Context, path: &Path) -> Result<()> {
        use deltachat::imex;

        imex::imex(context, imex::ImexMode::ImportBackup, path, None)
            .await
            .map_err(|err| anyhow!("{}", err))?;

        let mut events = context.get_event_emitter();
        while let Some(event) = events.next().await {
            match event.typ {
                EventType::ImexProgress(0) => {
                    bail!("Failed to import");
                }
                EventType::ImexProgress(1000) => {
                    context.start_io().await;
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }

    pub async fn login(&self, context: &Context, email: &str, password: &str) -> Result<()> {
        use deltachat::config::Config;
        self.state.write().await.logged_in = Login::Progress(0);

        context
            .set_config(Config::Addr, Some(email))
            .await
            .map_err(|err| anyhow!("{:?}", err))?;
        context
            .set_config(Config::MailPw, Some(password))
            .await
            .map_err(|err| anyhow!("{:?}", err))?;

        self.configure(context).await?;
        Ok(())
    }

    pub async fn configure(&self, context: &Context) -> Result<()> {
        use deltachat::config::Config;
        info!("configure");

        let is_configured = context.get_config_int(Config::Configured).await?;
        if is_configured == 1 {
            info!("Account already configured");
            return Ok(());
        } else {
            context
                .configure()
                .await
                .map_err(|err| anyhow!("{:?}", err))?;

            let mut events = context.get_event_emitter();
            while let Some(event) = events.next().await {
                info!("configure event {:?}", event);
                match event.typ {
                    EventType::ConfigureProgress { progress, .. } => match progress {
                        0 => {
                            bail!("Failed to login");
                        }
                        1000 => {
                            break;
                        }
                        _ => {}
                    },
                    EventType::ImapConnected(_) | EventType::SmtpConnected(_) => {
                        break;
                    }
                    _ => {}
                }
            }

            Ok(())
        }
    }

    pub async fn load_chat_list(
        &self,
        context: &Context,
        range: Option<(usize, usize)>,
    ) -> Result<((usize, usize), usize, Vec<ChatState>)> {
        if let Some((a, b)) = range {
            ensure!(a <= b, "invalid indicies");
        }

        let chatlist = Chatlist::try_load(&context, 0, None, None)
            .await
            .map_err(|err| anyhow!("failed to load chats: {:?}", err))?;
        let total_len = chatlist.len();
        let range = range.unwrap_or((0, total_len));
        let stop_index = (range.1 + 1).min(total_len);
        let start_index = range.0;
        let len = stop_index.saturating_sub(start_index);

        let mut chats = Vec::with_capacity(len);
        for i in start_index..stop_index {
            let chat_id = chatlist.get_chat_id(i).with_context(|| format!("{}", i))?;
            let (_, chat_state) = load_chat_state(context.clone(), chat_id, &chatlist)
                .await
                .with_context(|| format!("chat id: {}", chat_id))?;
            if let Some(s) = chat_state {
                chats.push(s);
            }
        }

        Ok(((start_index, stop_index - 1), total_len, chats))
    }

    pub async fn select_chat(&self, context: &Context, chat_id: ChatId) -> Result<()> {
        info!("selecting chat {:?}", chat_id);
        let mut ls = self.state.write().await;
        ls.selected_chat_id = Some(chat_id);
        let chatlist = Chatlist::try_load(&context, 0, None, None)
            .await
            .map_err(|err| anyhow!("failed to load chats: {:?}", err))?;
        let (_, selected_chat) = load_chat_state(context.clone(), chat_id, &chatlist).await?;
        ls.selected_chat = selected_chat;

        // mark as noticed
        chat::marknoticed_chat(context, chat_id)
            .await
            .map_err(|err| anyhow!("failed to mark noticed: {:?}", err))?;

        Ok(())
    }
    pub async fn pin_chat(&self, context: &Context, chat_id: ChatId) -> Result<()> {
        info!("pinning chat: {:?}", chat_id);

        chat_id
            .set_visibility(context, ChatVisibility::Pinned)
            .await?;

        Ok(())
    }

    pub async fn unpin_chat(&self, context: &Context, chat_id: ChatId) -> Result<()> {
        info!("unpinning chat: {:?}", chat_id);

        chat_id
            .set_visibility(context, ChatVisibility::Normal)
            .await?;

        Ok(())
    }

    pub async fn archive_chat(&self, context: &Context, chat_id: ChatId) -> Result<()> {
        info!("archiving chat: {:?}", chat_id);

        chat_id
            .set_visibility(context, ChatVisibility::Archived)
            .await?;

        Ok(())
    }

    pub async fn unarchive_chat(&self, context: &Context, chat_id: ChatId) -> Result<()> {
        info!("unarchiving chat: {:?}", chat_id);

        chat_id
            .set_visibility(context, ChatVisibility::Normal)
            .await?;

        Ok(())
    }

    pub async fn load_message_list(
        &self,
        context: &Context,
        range: Option<(usize, usize)>,
    ) -> Result<(u32, (usize, usize), Vec<ChatItem>, Vec<ChatMessage>)> {
        let chat_id = self.state.read().await.selected_chat_id.clone();
        if let Some(chat_id) = chat_id {
            info!("loading {:?} msgs", chat_id);

            let (chat_id, range, chat_items, chat_messages) =
                refresh_message_list(context.clone(), chat_id, range).await?;

            let msg_ids: Vec<_> = chat_messages
                .iter()
                .filter_map(|item| match item {
                    ChatMessage::Message(inner) => Some(message::MsgId::new(inner.id)),
                    ChatMessage::DayMarker(..) => None,
                })
                .collect();

            {
                let ctx = context.clone();
                message::markseen_msgs(&ctx, msg_ids).await?;
            }

            Ok((chat_id, range, chat_items, chat_messages))
        } else {
            bail!("failed to load message list, no chat selected");
        }
    }

    pub async fn send_text_message(&self, context: &Context, text: String) -> Result<()> {
        if let Some(chat_id) = self.state.read().await.selected_chat_id {
            chat::send_text_msg(&context, chat_id, text)
                .await
                .map_err(|err| anyhow!("failed to send message: {}", err))?;
        } else {
            bail!("no chat selected, can not send message");
        }

        Ok(())
    }

    pub async fn send_file_message(
        &self,
        context: &Context,
        typ: Viewtype,
        path: String,
        text: Option<String>,
        mime: Option<String>,
    ) -> Result<()> {
        if let Some(chat_id) = self.state.read().await.selected_chat_id {
            let mut msg = message::Message::new(
                deltachat::message::Viewtype::from_i32(typ.to_i32().unwrap()).unwrap(),
            );
            msg.set_text(text.unwrap_or_else(|| "".to_owned()));
            msg.set_file(path, mime.as_deref());

            chat::send_msg(&context, chat_id, &mut msg)
                .await
                .map_err(|err| anyhow!("failed to send message: {}", err))?;
        } else {
            bail!("no chat selected, can not send message");
        }

        Ok(())
    }

    pub async fn accept_contact_request(&self, context: &Context, chat_id: ChatId) -> Result<()> {
        chat_id.accept(&context).await?;
        Ok(())
    }

    pub async fn block_contact(&self, context: &Context, chat_id: ChatId) -> Result<()> {
        chat_id.block(&context).await?;
        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct RemoteEvent {
    #[serde(rename = "type")]
    typ: String,
    event: String,
}

fn get_timestamp(ts: i64) -> Option<DateTime<Utc>> {
    let naive = NaiveDateTime::from_timestamp_opt(ts, 0)?;
    Some(DateTime::from_utc(naive, Utc))
}

async fn load_chat_state(
    context: Context,
    chat_id: ChatId,
    chats: &Chatlist,
) -> Result<(Option<Chat>, Option<ChatState>)> {
    let (chat, chat_state) = if let Ok(chat) = Chat::load_from_db(&context, chat_id).await {
        if let Some(index) = chats.get_index_for_id(chat_id) {
            let lot = chats.get_summary(&context, index, Some(&chat)).await?;

            let header = lot
                .prefix
                .as_ref()
                .map(|s| s.to_string())
                .unwrap_or_default();
            let preview = lot.truncated_text(160).to_string();

            let index = chats.get_index_for_id(chat_id);
            let is_contact_request = chat.is_contact_request();

            (
                None,
                Some(ChatState {
                    id: chat_id.to_u32(),
                    index,
                    name: chat.get_name().to_string(),
                    header,
                    preview,
                    timestamp: get_timestamp(lot.timestamp),
                    state: lot.state.to_string(),
                    profile_image: chat.get_profile_image(&context).await?.map(Into::into),
                    can_send: chat.can_send(&context).await.unwrap_or_default(),
                    is_contact_request,
                    chat_type: chat.get_type().to_string(),
                    color: chat.get_color(&context).await?,
                    is_device_talk: chat.is_device_talk(),
                    is_self_talk: chat.is_self_talk(),
                    fresh_msg_cnt: chat_id.get_fresh_msg_cnt(&context).await?,
                    member_count: deltachat::chat::get_chat_contacts(&context, chat_id)
                        .await?
                        .len(),
                    is_pinned: chat.get_visibility() == ChatVisibility::Pinned,
                    is_archived: chat.get_visibility() == ChatVisibility::Archived,
                }),
            )
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    Ok((chat, chat_state))
}

async fn refresh_message_list(
    context: Context,
    chat_id: ChatId,
    range: Option<(usize, usize)>,
) -> Result<(u32, (usize, usize), Vec<ChatItem>, Vec<ChatMessage>)> {
    let chat_items: Vec<_> = chat::get_chat_msgs_ex(
        &context,
        chat_id,
        MessageListOptions {
            info_only: false,
            add_daymarker: true,
        },
    )
    .await?
    .into_iter()
    .filter_map(|item| match item {
        chat::ChatItem::Message { msg_id } => Some(ChatItem::Message(msg_id.to_u32())),
        chat::ChatItem::DayMarker { timestamp } => {
            get_timestamp(timestamp * 86_400).map(ChatItem::DayMarker)
        }
    })
    .collect();

    let total_len = chat_items.len();

    // default to the last n items
    let range = range.unwrap_or_else(|| (total_len.saturating_sub(50), total_len));

    info!(
        "loading chat messages {:?} from ({}..={})",
        chat_id, range.0, range.1
    );

    let len = range.1.saturating_sub(range.0);
    let offset = range.0;

    let mut chat_messages = Vec::with_capacity(len);
    let mut contacts = HashMap::new();

    let mut last_contact_id = None;
    let mut last_marker = true;
    for chat_item in chat_items.iter().skip(offset).take(len) {
        match chat_item {
            ChatItem::Message(msg_id) => {
                let msg = message::Message::load_from_db(&context, MsgId::new(*msg_id))
                    .await
                    .map_err(|err| anyhow!("failed to load msg: {}: {}", msg_id, err))?;

                let from = match contacts.get(&msg.get_from_id()) {
                    Some(contact) => contact,
                    None => {
                        let contact = Contact::get_by_id(&context, msg.get_from_id())
                            .await
                            .map_err(|err| {
                                anyhow!("failed to load contact: {}: {}", msg.get_from_id(), err)
                            })?;
                        contacts.insert(msg.get_from_id(), contact);
                        contacts.get(&msg.get_from_id()).unwrap()
                    }
                };

                let is_first = if last_marker {
                    true
                } else if let Some(id) = last_contact_id {
                    id != msg.get_from_id()
                } else {
                    true
                };
                last_contact_id = Some(msg.get_from_id());
                last_marker = false;
                let mut inner_msg = InnerChatMessage {
                    id: msg.get_id().to_u32(),
                    from_id: msg.get_from_id().to_u32(),
                    viewtype: Viewtype::from_i32(msg.get_viewtype().to_i32().unwrap()).unwrap(),
                    from_first_name: from.get_display_name().to_string(),
                    from_profile_image: from.get_profile_image(&context).await?.map(Into::into),
                    from_color: from.get_color(),
                    state: msg.get_state().to_string(),
                    text: {
                        let text = msg.get_text();
                        if text.is_empty() {
                            None
                        } else {
                            Some(text)
                        }
                    },
                    quote: None,
                    timestamp: get_timestamp(msg.get_sort_timestamp()),
                    is_info: msg.is_info(),
                    file: msg.get_file(&context).map(Into::into),
                    file_width: msg.get_width(),
                    file_height: msg.get_height(),
                    is_first,
                };

                if let Some(quote) = msg.quoted_message(&context).await? {
                    inner_msg.quote = Some(load_quote(&context, &mut contacts, quote).await?);
                }

                chat_messages.push(ChatMessage::Message(inner_msg));
            }
            ChatItem::DayMarker(t) => {
                chat_messages.push(ChatMessage::DayMarker(*t));
                last_marker = true;
            }
        }
    }

    Ok((chat_id.to_u32(), range, chat_items, chat_messages))
}

async fn load_quote(
    context: &Context,
    contacts: &mut HashMap<ContactId, Contact>,
    msg: message::Message,
) -> Result<Box<InnerChatMessage>> {
    let from = match contacts.get(&msg.get_from_id()) {
        Some(contact) => contact,
        None => {
            let contact = Contact::get_by_id(&context, msg.get_from_id())
                .await
                .map_err(|err| anyhow!("failed to load contact: {}: {}", msg.get_from_id(), err))?;
            contacts.insert(msg.get_from_id(), contact);
            contacts.get(&msg.get_from_id()).unwrap()
        }
    };

    Ok(Box::new(InnerChatMessage {
        id: msg.get_id().to_u32(),
        from_id: msg.get_from_id().to_u32(),
        viewtype: Viewtype::from_i32(msg.get_viewtype().to_i32().unwrap()).unwrap(),
        from_first_name: from.get_display_name().to_string(),
        from_profile_image: from.get_profile_image(&context).await?.map(Into::into),
        from_color: from.get_color(),
        state: msg.get_state().to_string(),
        text: {
            let text = msg.get_text();
            if text.is_empty() {
                None
            } else {
                Some(text)
            }
        },
        quote: None,
        timestamp: get_timestamp(msg.get_sort_timestamp()),
        is_info: msg.is_info(),
        file: msg.get_file(&context).map(Into::into),
        file_width: msg.get_width(),
        file_height: msg.get_height(),
        is_first: true,
    }))
}
