use std::{collections::HashMap, num::NonZeroUsize};

use serde::Deserialize;
use teloxide::{
    payloads::{SendMessageSetters, SendPhotoSetters},
    requests::{Request as RequestExt, Requester},
    types::{ChatId, InputFile, MessageId, UserId},
    Bot,
};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use crate::{
    invoke_ai::{self, client::InvokeAI},
    models::{BatchId, Enqueue},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("InvokeAI Client error {0}")]
    InvokeAiClient(#[from] invoke_ai::Error),
    #[error("Telegram API Error {0}")]
    TelegramApi(#[from] teloxide::ApiError),
    #[error("Telegram Request Error {0}")]
    TelegramRequest(#[from] teloxide::RequestError),
    #[error("Received an update for a batch that's not in our queue")]
    NotInQueue,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct Config {
    pub invoke_ai_url: String,
    pub max_in_progress: Option<NonZeroUsize>,
}

#[derive(Debug)]
pub enum Update {
    Requested {
        enqueue: Box<Enqueue>,
        chat_id: ChatId,
        user_id: UserId,
        message_id: MessageId,
    },
    Started {
        id: BatchId,
        chat_id: ChatId,
        user_id: UserId,
        message_id: MessageId,
    },
    Progress {
        id: BatchId,
    },
    Finished {
        batch_id: BatchId,
        image_url: String,
    },
    Failed {
        batch_id: BatchId,
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
    client: InvokeAI,
    bot: Bot,
    receiver: UnboundedReceiver<Update>,
    notifier: Notifier,
    /// Maximum number of queries in progress per user
    max_in_progress: NonZeroUsize,
}

impl Handler {
    /// Initiate the telegram bot and start listening for updates
    pub async fn try_new(
        config: Config,
        bot: Bot,
        http_client: reqwest::Client,
    ) -> Result<Self, Error> {
        let Config {
            invoke_ai_url,
            max_in_progress,
        } = config;

        let (sender, receiver) = mpsc::unbounded_channel::<Update>();
        let notifier = Notifier::from(sender);

        let client = InvokeAI::connect(invoke_ai_url, notifier.clone(), http_client).await?;

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

    /// Start handling new requests and InvokeAI progress updates
    pub async fn start(mut self) {
        log::info!("Starting invoke handler");
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
                    log::warn!("failed to generate image: {error}");
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
                enqueue,
                chat_id,
                user_id,
                message_id,
            } => {
                log::info!(
                    "Received request, Prompt({}), ChatId({chat_id}), UserId({user_id})",
                    enqueue.prompt()
                );

                let in_progress = queue.increment_user_count(user_id);

                if in_progress > self.max_in_progress.get() {
                    queue.decrement_user_count(user_id);
                    return Ok(Response::Message {
                        chat_id,
                        message_id,
                        message: "You already have too many images in progress".into(),
                    });
                }

                self.client
                    .enqueue_text_to_image(enqueue, chat_id, user_id, message_id)
                    .await?;
            }

            Update::Started {
                id,
                chat_id,
                user_id,
                message_id,
            } => {
                log::info!("started processing {id:?}");
                queue.insert(
                    id,
                    QueueEntry {
                        chat_id,
                        user_id,
                        message_id,
                    },
                );
            }

            Update::Progress { id } => {
                log::debug!("processing update {id:?}");
            }

            Update::Finished {
                batch_id,
                image_url,
            } => {
                log::info!("processing finished {batch_id:?}, url: {image_url}");

                let entry = queue.remove(batch_id).ok_or(Error::NotInQueue)?;

                queue.decrement_user_count(entry.user_id);

                let bytes = self.client.download_image(image_url).await?;

                self.bot
                    .send_photo(entry.chat_id, InputFile::memory(bytes))
                    .reply_to_message_id(entry.message_id)
                    .send()
                    .await?;
            }

            Update::Failed { batch_id, reason } => {
                log::error!("Failed to finish {batch_id:?}, error: {reason}");
                let entry = queue.remove(batch_id).ok_or(Error::NotInQueue)?;

                queue.decrement_user_count(entry.user_id);

                return Ok(Response::Message {
                    chat_id: entry.chat_id,
                    message_id: entry.message_id,
                    message: format!("Failed to generate image: {reason}"),
                });
            }
        }

        Ok(Response::None)
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct QueueEntry {
    chat_id: ChatId,
    message_id: MessageId,
    user_id: UserId,
}

#[derive(Default)]
struct Queue {
    queue: HashMap<BatchId, QueueEntry>,
    users: HashMap<UserId, usize>,
}

impl Queue {
    fn insert(&mut self, id: BatchId, entry: QueueEntry) {
        self.queue.insert(id, entry);
    }

    fn remove(&mut self, id: BatchId) -> Option<QueueEntry> {
        self.queue.remove(&id)
    }

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

#[allow(unused)]
#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Bar {
        users: HashMap<UserId, usize>,
    }

    impl Bar {
        fn increment_user_count(&mut self, user_id: UserId) -> usize {
            *self
                .users
                .entry(user_id)
                .and_modify(|counter| *counter += 1)
                .or_insert(1)
        }

        fn decrement_user_count(&mut self, user_id: UserId) -> usize {
            *self
                .users
                .entry(user_id)
                .and_modify(|counter| *counter -= 1)
                .or_insert(1)
        }
    }

    #[test]
    fn foo() {
        let mut bar = Bar::default();

        assert_eq!(bar.increment_user_count(UserId(1)), 1);
        assert_eq!(bar.increment_user_count(UserId(1)), 2);
        assert_eq!(bar.increment_user_count(UserId(1)), 3);
    }
}
