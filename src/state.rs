use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use async_std::sync::RwLock;
use egui::{ColorImage, Context, TextureHandle};
use futures::{select, stream::StreamExt};
use log::{debug, error, info, warn};

use crate::dc;
use crate::dc::types::{ChatList, Event, Log, MessageList, SharedState};
use crate::scheduler::Scheduler;

pub struct AppState {
    scheduler: Scheduler,
    shared_state: Arc<RwLock<State>>,

    pub commands: async_std::channel::Sender<Command>,
    pub current_input: String,

    pub image_cache: RefCell<HashMap<String, TextureHandle>>,
}

#[derive(Debug)]
pub enum Command {
    SelectChat(u32, u32),
    SelectAccount(u32),
    SendTextMessage(String),
}

#[derive(Debug, Default)]
pub struct State {
    pub shared_state: SharedState,
    pub message_list: MessageList,
    pub chat_list: ChatList,
}

impl AppState {
    pub fn new(frame: &epi::Frame) -> Self {
        debug!("Setting up app state");
        let mut scheduler = Scheduler::new();
        scheduler.init(frame);

        let (dc_events_sender, mut dc_events_receiver) = async_std::channel::bounded(1000);
        let (commands_sender, mut commands_receiver) = async_std::channel::bounded(1000);

        let shared_state = Arc::new(RwLock::new(State::default()));

        let ss = shared_state.clone();
        scheduler.spawn(|repaint| async move {
            let shared_state = ss;
            let dc_state = match dc::state::LocalState::new().await {
                Ok(local_state) => local_state,
                Err(err) => panic!("Can't restore local state: {}", err),
            };
            //.expect(format!("Local state could not be restored: {}", err))

            dc_state.subscribe_all(dc_events_sender);
            {
                let mut s = shared_state.write().await;
                let shared_state = dc_state.get_state().await;
                s.shared_state = shared_state;

                if let Some((id, _)) = s.shared_state.accounts.iter().nth(0) {
                    dbg!("loading account");
                    let info = dc_state.select_account(*id).await.unwrap();
                    dbg!(&info);
                    s.shared_state.selected_account = Some(info.account);
                    s.shared_state.selected_chat_id = info.chat_id;
                    s.shared_state.selected_chat = info.chat;
                }

                s.chat_list = dc_state.load_chat_list(None).await.unwrap();
                if let Some(_chat_id) = s.shared_state.selected_chat_id {
                    s.message_list = dc_state.load_message_list(None).await.unwrap();
                }

                dbg!(s);
            }

            if let Some(ref r) = repaint {
                r.request_repaint();
            }

            loop {
                select! {
                    (account, event) = dc_events_receiver.select_next_some() => {
                        match event {
                            Event::Configure(_progress) => {}
                            Event::Log(log) => match log {
                                Log::Info(msg) => debug!("[{}] {}", account, msg),
                                Log::Warning(msg) => warn!("[{}] {}", account, msg),
                                Log::Error(msg) => error!("[{}] {}", account, msg),
                            },
                            Event::Connected => {
                                info!("connected");
                                let mut s = shared_state.write().await;
                                s.shared_state = dc_state.get_state().await;
                                s.chat_list = dc_state.load_chat_list(None).await.unwrap();
                                if s.shared_state.selected_chat_id.is_some() {
                                    s.message_list = dc_state.load_message_list(None).await.unwrap();
                                }
                            }
                            Event::MessagesChanged { chat_id } | Event::MessageIncoming { chat_id, .. } => {
                                info!("new message list");
                                let mut s = shared_state.write().await;
                                s.chat_list = dc_state.load_chat_list(None).await.unwrap();
                                if let Some(old_chat_id) = s.shared_state.selected_chat_id {
                                    if chat_id == old_chat_id {
                                        s.message_list = dc_state.load_message_list(None).await.unwrap();
                                    }
                                }
                            }
                            _ => {
                                // TODO: handle other events
                            }
                        }
                        // TODO: be more selective on when to repaint
                        if let Some(ref r) = repaint {
                            r.request_repaint();
                        }
                    }
                    cmd = commands_receiver.select_next_some() => {
                        match cmd {
                            Command::SelectChat(account, chat) => {
                                let mut s = shared_state.write().await;
                                s.message_list = dc_state.select_chat(account, chat).await.unwrap();
                                s.shared_state = dc_state.get_state().await;

                                if let Some(ref r) = repaint {
                                    r.request_repaint();
                                }
                            }
                            Command::SelectAccount(account) => {
                                info!("selecting account {}", account);
                                let mut s = shared_state.write().await;
                                if s.shared_state.selected_account == Some(account) {
                                    continue;
                                }
                                dc_state.select_account(account).await.unwrap();
                                s.shared_state = dc_state.get_state().await;
                                s.chat_list = dc_state.load_chat_list(None).await.unwrap();
                                if let Some(_chat_id) = s.shared_state.selected_chat_id {
                                    s.message_list = dc_state.load_message_list(None).await.unwrap();
                                } else {
                                    s.message_list.clear();
                                }

                                if let Some(ref r) = repaint {
                                    r.request_repaint();
                                }
                            }
                            Command::SendTextMessage(msg) => {
                                dc_state.send_text_message(msg).await.unwrap();
                            }
                        }
                    }
                }
            }
        });

        AppState {
            scheduler,
            shared_state,
            current_input: Default::default(),
            commands: commands_sender,
            image_cache: Default::default(),
        }
    }

    pub fn init(&mut self) {}

    pub fn poll(&mut self, ctx: &Context) {
        let mut repaint = false;
        self.scheduler.poll();

        // while let Ok(_) = self.receiver.try_recv() {
        //     repaint = true;
        // }

        if repaint {
            ctx.request_repaint();
        }
    }

    pub fn shared_state(&self) -> async_std::sync::RwLockReadGuard<'_, State> {
        async_std::task::block_on(async move { self.shared_state.read().await })
    }

    pub fn send_command(&self, cmd: Command) {
        async_std::task::block_on(async move { self.commands.send(cmd).await })
            .expect("failed to send cmd");
    }

    pub fn get_or_load_image(
        &self,
        ctx: &Context,
        name: String,
        load: impl Fn(&str) -> ColorImage,
    ) -> TextureHandle {
        self.image_cache
            .borrow_mut()
            .entry(name)
            .or_insert_with_key(|name| {
                let image_data = load(name);
                ctx.load_texture(name, image_data)
            })
            .clone()
    }
}
