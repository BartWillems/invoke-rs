use std::{collections::HashMap, fmt, num::NonZeroUsize};

use serde::Deserialize;
use teloxide::{
    payloads::SendMessageSetters,
    requests::{Request as RequestExt, Requester},
    types::{ChatId, MessageId, UserId},
    Bot,
};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use crate::local_ai::{LocalAI, Prompts};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Telegram API Error {0}")]
    TelegramApi(#[from] teloxide::ApiError),
    #[error("Telegram Request Error {0}")]
    TelegramRequest(#[from] teloxide::RequestError),
    #[error("Received an update for a batch that's not in our queue")]
    NotInQueue,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct Config {
    pub local_ai_url: String,
    pub max_in_progress: Option<NonZeroUsize>,
}

/// Identifier used to identify unique requests
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Identifier {
    chat_id: ChatId,
    user_id: UserId,
    message_id: MessageId,
}

impl fmt::Debug for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Identifier({}-{}-{})",
            self.chat_id, self.user_id, self.message_id
        )
    }
}

#[derive(Debug)]
pub enum Update {
    Requested {
        chat_id: ChatId,
        user_id: UserId,
        message_id: MessageId,
        prompt: String,
        model: crate::local_ai::Model,
    },
    Finished {
        identifier: Identifier,
        response: Option<String>,
    },
    Failed {
        identifier: Identifier,
        reason: String,
    },
}

enum Response {
    Message {
        chat_id: ChatId,
        message_id: MessageId,
        message: String,
    },
    None,
}

/// Handle for sending state-change notifications
#[derive(Clone)]
pub struct Notifier {
    inner: UnboundedSender<Update>,
}

impl From<UnboundedSender<Update>> for Notifier {
    fn from(inner: UnboundedSender<Update>) -> Self {
        Self { inner }
    }
}

impl Notifier {
    /// Notify the handler about a state change
    ///
    /// Panics when the receiver is down
    pub fn notify(&self, update: Update) {
        self.inner
            .send(update)
            .expect("Failed to send update, this is bad");
    }
}

pub struct Handler {
    client: LocalAI,
    bot: Bot,
    receiver: UnboundedReceiver<Update>,
    notifier: Notifier,
    /// Maximum number of queries in progress per user
    max_in_progress: NonZeroUsize,
}

impl Handler {
    /// Initiate the telegram bot and start listening for updates
    pub fn try_new(
        config: Config,
        bot: Bot,
        http_client: reqwest::Client,
        prompts: Prompts,
    ) -> Result<Self, Error> {
        let Config {
            local_ai_url,
            max_in_progress,
        } = config;

        let (sender, receiver) = mpsc::unbounded_channel::<Update>();
        let notifier = Notifier::from(sender);

        let client = LocalAI::new(local_ai_url, notifier.clone(), http_client, prompts);

        Ok(Self {
            client,
            bot: bot.clone(),
            receiver,
            notifier,
            max_in_progress: max_in_progress.unwrap_or(NonZeroUsize::new(3).unwrap()),
        })
    }

    pub fn notifier(&self) -> Notifier {
        self.notifier.clone()
    }

    /// Start handling new requests and LocalAI progress updates
    pub async fn start(mut self) {
        log::info!("Starting local-ai handler");
        let mut queue = Queue::default();

        while let Some(update) = self.receiver.recv().await {
            let res = match self.handle(update, &mut queue).await {
                Ok(Response::None) => continue,
                Ok(Response::Message {
                    chat_id,
                    message_id,
                    message,
                }) => {
                    self.bot
                        .send_message(chat_id, message)
                        .reply_to_message_id(message_id)
                        .await
                }
                Err(error) => {
                    log::warn!("failed to generate text: {error}");
                    continue;
                }
            };

            if let Err(error) = res {
                log::error!("failed to send telegram message: {error}");
            }
        }
    }

    async fn handle(&self, update: Update, queue: &mut Queue) -> Result<Response, Error> {
        match update {
            Update::Requested {
                chat_id,
                user_id,
                message_id,
                prompt,
                model,
            } => {
                log::info!(
                    "Received request, Prompt({prompt}), ChatId({chat_id}), UserId({user_id})",
                );

                let in_progress = queue.increment_user_count(user_id);

                if in_progress > self.max_in_progress.get() {
                    queue.decrement_user_count(user_id);
                    return Ok(Response::Message {
                        chat_id,
                        message_id,
                        message: "You already have too many prompts in progress".into(),
                    });
                }

                self.client
                    .enqueue_request(
                        Identifier {
                            chat_id,
                            user_id,
                            message_id,
                        },
                        prompt,
                        model,
                    )
                    .await;
            }

            Update::Finished {
                identifier,
                response,
            } => {
                log::info!("processing finished {identifier:?}, reponse: {response:?}");

                queue.decrement_user_count(identifier.user_id);

                let response =
                    response.unwrap_or_else(|| String::from("Failed to generate a response"));

                self.bot
                    .send_message(identifier.chat_id, response)
                    .reply_to_message_id(identifier.message_id)
                    .send()
                    .await?;
            }

            Update::Failed { identifier, reason } => {
                log::error!("Failed to finish {identifier:?}, error: {reason}");

                queue.decrement_user_count(identifier.user_id);

                return Ok(Response::Message {
                    chat_id: identifier.chat_id,
                    message_id: identifier.message_id,
                    message: format!("Failed to generate text prompt, send this code to the developer: {identifier:?}"),
                });
            }
        }

        Ok(Response::None)
    }
}

#[derive(Default)]
struct Queue {
    users: HashMap<UserId, usize>,
}

impl Queue {
    fn increment_user_count(&mut self, user_id: UserId) -> usize {
        log::debug!("Incrementing user {user_id}");

        *self
            .users
            .entry(user_id)
            .and_modify(|counter| *counter += 1)
            .or_insert(1)
    }

    fn decrement_user_count(&mut self, user_id: UserId) -> usize {
        log::debug!("Decrementing user {user_id}");

        *self
            .users
            .entry(user_id)
            .and_modify(|counter| *counter -= 1)
            .or_insert(1)
    }
}
