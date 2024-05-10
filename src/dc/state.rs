use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use broadcaster::BroadcastChannel;
use deltachat::chat::{Chat, ChatId};
use deltachat::context::Context;
use deltachat::{message, EventType};
use futures::StreamExt;
use log::{debug, error, info, warn};
use num_traits::FromPrimitive;
use tokio::runtime::Runtime;
use tokio::sync::RwLock;

use super::{account, types::*};

use super::account::*;

#[derive(Debug, Clone)]
pub struct LocalState {
    rt: Arc<Runtime>,
    inner: Arc<RwLock<LocalStateInner>>,
    events: BroadcastChannel<deltachat::Event>,
}

#[derive(Debug)]
struct LocalStateInner {
    account_states: HashMap<u32, Account>,
    accounts: deltachat::accounts::Accounts,
    errors: Vec<anyhow::Error>,
}

impl LocalState {
    pub async fn new(rt: Arc<Runtime>) -> Result<Self> {
        let inner = LocalStateInner::new().await?;

        let receiver = BroadcastChannel::new();
        let sender = receiver.clone();
        let events = inner.accounts.get_event_emitter();

        rt.spawn(async move {
            while let Some(event) = events.recv().await {
                if let Err(err) = sender.send(&event).await {
                    error!("Failed to send event: {:?}", err);
                }
            }
        });

        Ok(Self {
            rt,
            inner: Arc::new(RwLock::new(inner)),
            events: receiver,
        })
    }

    async fn with_account_state<F>(&self, id: u32, f: F)
    where
        F: FnOnce(&mut account::AccountState),
    {
        let ls = self.inner.read().await;
        let account = ls.account_states.get(&id).expect("missing account");

        let state = &mut account.state.write().await;
        f(state);
    }

    pub fn subscribe_all(&self, rx: tokio::sync::mpsc::Sender<(u32, Event)>) {
        let mut events = self.events.clone();
        let ls = self.clone();

        self.rt.spawn(async move {
            while let Some(event) = events.next().await {
                match ls.handle_event(&rx, event).await {
                    Ok(_) => {}
                    Err(err) => {
                        warn!("{:?}", err);
                    }
                }
            }
        });
    }

    async fn handle_event(
        &self,
        rx: &tokio::sync::mpsc::Sender<(u32, Event)>,
        event: deltachat::Event,
    ) -> Result<()> {
        match event.typ {
            EventType::ConfigureProgress { progress, .. } => {
                if progress == 0 {
                    self.with_account_state(event.id, |state| {
                        state.logged_in = Login::Error("failed to login".into());
                    })
                    .await;
                    rx.send((event.id, Event::Configure(Progress::Error)))
                        .await?;
                } else {
                    let p = if progress == 1000 {
                        Progress::Success
                    } else {
                        self.with_account_state(event.id, |state| {
                            state.logged_in = Login::Progress(progress);
                        })
                        .await;
                        Progress::Step(progress)
                    };
                    rx.send((event.id, Event::Configure(p))).await?;
                }
            }
            EventType::ImexProgress(progress) => {
                if progress == 0 {
                    self.with_account_state(event.id, |state| {
                        state.logged_in = Login::Error("failed to import".into());
                    })
                    .await;
                    rx.send((event.id, Event::Imex(Progress::Error))).await?;
                } else {
                    let p = if progress == 1000 {
                        Progress::Success
                    } else {
                        self.with_account_state(event.id, |state| {
                            state.logged_in = Login::Progress(progress);
                        })
                        .await;
                        Progress::Step(progress)
                    };
                    rx.send((event.id, Event::Imex(p))).await?;
                }
            }
            EventType::ImapConnected(_) | EventType::SmtpConnected(_) => {
                info!("logged in");
                self.with_account_state(event.id, |state| {
                    state.logged_in = Login::Success;
                })
                .await;
                rx.send((event.id, Event::Connected)).await?;
            }
            EventType::IncomingMsg { chat_id, msg_id } => {
                let res: Result<()> = async {
                    let ctx = {
                        let ls = self.inner.read().await;
                        ls.accounts.get_account(event.id).unwrap()
                    };

                    let msg = message::Message::load_from_db(&ctx, msg_id)
                        .await
                        .map_err(|err| anyhow!("failed to load msg: {}: {}", msg_id, err))?;
                    let chat = Chat::load_from_db(&ctx, chat_id)
                        .await
                        .map_err(|err| anyhow!("failed to load chat: {:?}", err))?;

                    rx.send((
                        event.id,
                        Event::MessageIncoming {
                            chat_id: chat_id.to_u32(),
                            title: chat.get_name().to_string(),
                            body: msg.get_text(),
                        },
                    ))
                    .await?;
                    Ok(())
                }
                .await;

                res?;
            }
            EventType::MsgDelivered { chat_id, .. }
            | EventType::MsgFailed { chat_id, .. }
            | EventType::MsgsChanged { chat_id, .. }
            | EventType::MsgRead { chat_id, .. }
            | EventType::ChatModified(chat_id)
            | EventType::MsgsNoticed(chat_id) => {
                rx.send((
                    event.id,
                    Event::MessagesChanged {
                        chat_id: chat_id.to_u32(),
                    },
                ))
                .await?;
            }
            EventType::Info(msg) => {
                rx.send((event.id, Event::Log(Log::Info(msg)))).await?;
            }
            EventType::Warning(msg) => {
                rx.send((event.id, Event::Log(Log::Warning(msg)))).await?;
            }
            EventType::Error(msg) => {
                rx.send((event.id, Event::Log(Log::Error(msg)))).await?;
            }
            EventType::ConnectivityChanged => {
                // info!("changed connectivity");
            }
            _ => {
                debug!("{:?}", event);
            }
        }
        Ok(())
    }

    pub async fn add_account(&self) -> Result<(u32, Context)> {
        let mut ls = self.inner.write().await;
        let id = ls.accounts.add_account().await?;
        let ctx = ls.accounts.get_account(id).unwrap();
        let account = Account::new()?;

        ls.account_states.insert(id, account);

        Ok((id, ctx.clone()))
    }

    pub async fn login(&self, id: u32, ctx: &Context, email: &str, password: &str) -> Result<()> {
        let res = self
            .inner
            .read()
            .await
            .account_states
            .get(&id)
            .unwrap()
            .login(ctx, email, password)
            .await;
        if let Err(err) = res {
            let mut ls = self.inner.write().await;
            ls.errors.push(err);
            ls.account_states.remove(&id);
            ls.accounts.remove_account(id).await?;
        }

        Ok(())
    }

    pub async fn send_account_details(
        &self,
        id: u32,
    ) -> Result<(SharedState, Option<ChatList>, Option<MessageList>)> {
        let ls = self.inner.write().await;
        let ctx = ls.accounts.get_account(id).unwrap();

        let resp = ls.to_response().await;

        let (resp2, resp3) = if let Some(account) = ls.account_states.get(&id) {
            // chat list
            let (range, len, chats) = account.load_chat_list(&ctx, None).await?;
            let resp2 = Some(ChatList { range, len, chats });

            // send selected chat if exists
            let resp3 = if let Some(_selected_chat) = account.state.read().await.selected_chat_id {
                let (chat_id, range, items, messages) =
                    account.load_message_list(&ctx, None).await?;

                Some(MessageList {
                    chat_id,
                    range,
                    items,
                    messages,
                })
            } else {
                None
            };

            (resp2, resp3)
        } else {
            (None, None)
        };

        Ok((resp, resp2, resp3))
    }

    pub async fn import(&self, ctx: &Context, id: u32, path: &Path) -> Result<()> {
        let res = self
            .inner
            .read()
            .await
            .account_states
            .get(&id)
            .unwrap()
            .import(ctx, path)
            .await;
        if let Err(err) = res {
            let mut ls = self.inner.write().await;
            ls.errors.push(err);
            ls.account_states.remove(&id);
            ls.accounts.remove_account(id).await?;
        }

        Ok(())
    }

    pub async fn get_state(&self) -> SharedState {
        self.inner.read().await.to_response().await
    }

    pub async fn select_chat(&self, account_id: u32, chat_id: u32) -> Result<MessageList> {
        let ls = self.inner.write().await;
        if let Some(account) = ls.account_states.get(&account_id) {
            let ctx = ls.accounts.get_account(account_id).unwrap();
            let chat = ChatId::new(chat_id);
            account.select_chat(&ctx, chat).await?;

            let (chat_id, range, items, messages) = account.load_message_list(&ctx, None).await?;

            Ok(MessageList {
                chat_id,
                range,
                items,
                messages,
            })
        } else {
            Err(anyhow!("invalid account: {}-{}", account_id, chat_id))
        }
    }

    pub async fn pin_chat(&self, account_id: u32, chat_id: u32) -> Result<Response> {
        let ls = self.inner.write().await;
        if let Some(account) = ls.account_states.get(&account_id) {
            let ctx = ls.accounts.get_account(account_id).unwrap();
            let chat = ChatId::new(chat_id);
            account.pin_chat(&ctx, chat).await?;

            let (chat_id, range, items, messages) = account.load_message_list(&ctx, None).await?;

            Ok(Response::MessageList {
                chat_id,
                range,
                items,
                messages,
            })
        } else {
            Err(anyhow!("invalid account: {}-{}", account_id, chat_id))
        }
    }

    pub async fn unpin_chat(&self, account_id: u32, chat_id: u32) -> Result<Response> {
        let ls = self.inner.write().await;
        if let Some(account) = ls.account_states.get(&account_id) {
            let ctx = ls.accounts.get_account(account_id).unwrap();
            let chat = ChatId::new(chat_id);
            account.unpin_chat(&ctx, chat).await?;

            let (chat_id, range, items, messages) = account.load_message_list(&ctx, None).await?;

            Ok(Response::MessageList {
                chat_id,
                range,
                items,
                messages,
            })
        } else {
            Err(anyhow!("invalid account: {}-{}", account_id, chat_id))
        }
    }

    pub async fn archive_chat(&self, account_id: u32, chat_id: u32) -> Result<Response> {
        let ls = self.inner.write().await;
        if let Some(account) = ls.account_states.get(&account_id) {
            let ctx = ls.accounts.get_account(account_id).unwrap();
            let chat = ChatId::new(chat_id);
            account.archive_chat(&ctx, chat).await?;

            let (chat_id, range, items, messages) = account.load_message_list(&ctx, None).await?;

            Ok(Response::MessageList {
                chat_id,
                range,
                items,
                messages,
            })
        } else {
            Err(anyhow!("invalid account: {}-{}", account_id, chat_id))
        }
    }

    pub async fn unarchive_chat(&self, account_id: u32, chat_id: u32) -> Result<Response> {
        let ls = self.inner.write().await;
        if let Some(account) = ls.account_states.get(&account_id) {
            let ctx = ls.accounts.get_account(account_id).unwrap();
            let chat = ChatId::new(chat_id);
            account.unpin_chat(&ctx, chat).await?;

            let (chat_id, range, items, messages) = account.load_message_list(&ctx, None).await?;

            Ok(Response::MessageList {
                chat_id,
                range,
                items,
                messages,
            })
        } else {
            Err(anyhow!("invalid account: {}-{}", account_id, chat_id))
        }
    }

    pub async fn accept_contact_request(&self, account_id: u32, chat_id: u32) -> Result<()> {
        let ls = self.inner.write().await;
        if let Some(account) = ls.account_states.get(&account_id) {
            let ctx = ls.accounts.get_account(account_id).unwrap();
            let chat = ChatId::new(chat_id);
            account.accept_contact_request(&ctx, chat).await?;

            Ok(())
        } else {
            Err(anyhow!("invalid account: {}-{}", account_id, chat_id))
        }
    }

    pub async fn block_contact(&self, account_id: u32, chat_id: u32) -> Result<()> {
        let ls = self.inner.write().await;
        if let Some(account) = ls.account_states.get(&account_id) {
            let ctx = ls.accounts.get_account(account_id).unwrap();
            let chat = ChatId::new(chat_id);
            account.block_contact(&ctx, chat).await?;

            Ok(())
        } else {
            Err(anyhow!("invalid account: {}-{}", account_id, chat_id))
        }
    }

    pub async fn load_chat_list(&self, range: Option<(usize, usize)>) -> Result<ChatList> {
        let ls = self.inner.read().await;
        if let Some((account, ctx)) = ls.get_selected_account().await {
            info!("Loading chat list");
            match account.load_chat_list(&ctx, range).await {
                Ok((range, len, chats)) => Ok(ChatList { range, len, chats }),
                Err(err) => {
                    warn!("Could not load chat list: {}", err);
                    // send an empty chat list to be handled by frontend
                    Ok(ChatList {
                        range: (0, 0),
                        len: 0,
                        chats: Vec::new(),
                    })
                }
            }
        } else {
            Err(anyhow!("no selected account"))
        }
    }

    pub async fn load_message_list(&self, range: Option<(usize, usize)>) -> Result<MessageList> {
        let ls = self.inner.read().await;
        if let Some((account, ctx)) = ls.get_selected_account().await {
            let (chat_id, range, items, messages) = account.load_message_list(&ctx, range).await?;

            Ok(MessageList {
                chat_id,
                range,
                items,
                messages,
            })
        } else {
            Err(anyhow!("no selected account"))
        }
    }

    pub async fn select_account(&self, account_id: u32) -> Result<AccountInfo> {
        let mut ls = self.inner.write().await;
        ls.select_account(account_id).await?;
        if let Some((account, _ctx)) = ls.get_selected_account().await {
            let state = account.state.read().await;

            Ok(AccountInfo {
                account: account_id,
                chat_id: state.selected_chat_id.map(|id| id.to_u32()),
                chat: state.selected_chat.as_ref().cloned(),
            })
        } else {
            Err(anyhow!("failed to select account"))
        }
    }

    pub async fn send_text_message(&self, text: String) -> Result<()> {
        let ls = self.inner.read().await;
        if let Some((account, ctx)) = ls.get_selected_account().await {
            account.send_text_message(&ctx, text).await?;
            Ok(())
        } else {
            Err(anyhow!("no account selected"))
        }
    }

    pub async fn send_file_message(
        &self,
        typ: Viewtype,
        path: String,
        text: String,
        mime: Option<String>,
    ) -> Result<()> {
        let ls = self.inner.read().await;
        if let Some((account, ctx)) = ls.get_selected_account().await {
            account
                .send_file_message(
                    &ctx,
                    Viewtype::from_i32(typ as i32).unwrap(),
                    path,
                    text,
                    mime,
                )
                .await?;
            Ok(())
        } else {
            Err(anyhow!("no account selected"))
        }
    }

    pub async fn maybe_network(&self) -> Result<()> {
        let ls = self.inner.read().await;
        ls.accounts.maybe_network().await;
        Ok(())
    }
}

impl LocalStateInner {
    pub async fn new() -> Result<Self> {
        info!("restoring local state");

        // load accounts from default dir
        let mut account_states = HashMap::new();
        let mut accounts = deltachat::accounts::Accounts::new(HOME_DIR.clone(), true).await?;
        let account_ids = accounts.get_all();

        if account_ids.is_empty() {
            panic!(
                "There are no available acccounts in your accounts.toml file: {}",
                HOME_DIR.to_str().unwrap()
            )
        }

        for id in account_ids.iter() {
            let state = Account::new()?;
            account_states.insert(*id, state);
        }

        info!("loaded state");

        accounts.start_io().await;

        info!("started io");

        Ok(Self {
            accounts,
            account_states,
            errors: Vec::new(),
        })
    }

    pub async fn get_selected_account(&self) -> Option<(&Account, deltachat::context::Context)> {
        if let Some(ctx) = self.accounts.get_selected_account() {
            let id = ctx.get_id();
            self.account_states.get(&id).map(|account| (account, ctx))
        } else {
            None
        }
    }

    pub async fn select_account(&mut self, id: u32) -> Result<()> {
        self.accounts.select_account(id).await?;
        Ok(())
    }

    pub async fn get_selected_account_id(&self) -> Option<u32> {
        self.accounts.get_selected_account().map(|ctx| ctx.get_id())
    }

    pub async fn get_selected_account_state(&self) -> Option<&Account> {
        if let Some(id) = self.get_selected_account_id().await {
            self.account_states.get(&id)
        } else {
            None
        }
    }

    pub async fn to_response(&self) -> SharedState {
        let mut accounts = HashMap::with_capacity(self.account_states.len());
        for (id, account) in self.account_states.iter() {
            let account = &account.state.read().await;
            let ctx = self.accounts.get_account(*id).unwrap();
            use deltachat::config::Config;
            let email = ctx.get_config(Config::Addr).await.unwrap().unwrap();
            let profile_image = ctx
                .get_config(Config::Selfavatar)
                .await
                .unwrap()
                .map(Into::into);
            let display_name = ctx.get_config(Config::Displayname).await.unwrap();

            accounts.insert(
                *id,
                SharedAccountState {
                    logged_in: account.logged_in.clone(),
                    email,
                    profile_image,
                    display_name,
                },
            );
        }

        let errors = self.errors.iter().map(|e| e.to_string()).collect();
        let (selected_chat_id, selected_chat) =
            if let Some(account) = self.get_selected_account_state().await {
                let state = account.state.read().await;
                (state.selected_chat_id, state.selected_chat.clone())
            } else {
                (None, None)
            };

        SharedState {
            accounts,
            errors,
            selected_account: self.get_selected_account_id().await,
            selected_chat_id: selected_chat_id.map(|s| s.to_u32()),
            selected_chat,
        }
    }
}
