use futures_util::FutureExt;
use rust_socketio::asynchronous::{Client as SocketClient, ClientBuilder as SocketClientBuilder};
use rust_socketio::Payload;
use serde_json::json;
use teloxide::types::{ChatId, MessageId};
use thiserror::Error;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use crate::models::invocations::{InvocationComplete, InvocationError};
use crate::models::{Enqueue, EnqueueResult};
use crate::Update;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to call InvokeAI {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Failed to connecto to SocketIO {0}")]
    SocketIO(#[from] rust_socketio::Error),
    #[error("Failed to subscribe to SocketIO queue {0}")]
    Subscription(rust_socketio::Error),
    #[error("Failed to decode JSON {0}")]
    JsonDecode(#[from] serde_json::Error),
}

#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    #[allow(unused)]
    socket: SocketClient,
    sender: UnboundedSender<Update>,
    url: String,
}

impl Client {
    /// Connect to the Socket.IO server of the InvokeAI instance
    pub async fn connect(url: String) -> Result<(Self, UnboundedReceiver<Update>), Error> {
        let (sender, receiver) = mpsc::unbounded_channel::<Update>();

        let cloned_url = url.clone();
        let updater = sender.clone();
        let socket = SocketClientBuilder::new(format!("{url}/ws/socket.io/"))
            .namespace("/")
            .on("invocation_complete", move |payload, _client| {
                let sender = updater.clone();
                let url = cloned_url.clone();
                async move {
                    match payload {
                        Payload::String(data) => {
                            let invocation: InvocationComplete =
                                match serde_json::from_str(data.as_str()) {
                                    Ok(invocation) => invocation,
                                    Err(error) => {
                                        log::error!(
                                        "Failed to parse SocketIO JSON: {error}, payload: {data}"
                                    );
                                        return;
                                    }
                                };

                            if invocation.still_in_progress() {
                                sender
                                    .send(Update::Progress {
                                        id: invocation.id(),
                                    })
                                    .map_err(|err| {
                                        log::error!("Failed to send progress udpate: {err}");
                                    })
                                    .ok();
                                return;
                            }

                            match invocation.image_path() {
                                Some(path) => {
                                    sender
                                        .send(Update::Finished {
                                            id: invocation.id(),
                                            image_url: format!("{url}/api/v1/images/i/{path}/full"),
                                        })
                                        .map_err(|err| {
                                            log::error!(
                                                "Failed to send completed image URL: {err}"
                                            );
                                        })
                                        .ok();
                                }
                                None => log::debug!("missing image, unimportant update"),
                            }
                        }
                        Payload::Binary(_) => {
                            log::warn!("unexpected binary")
                        }
                    }
                }
                .boxed()
            })
            .on("invocation_error", |payload, _client| {
                async move {
                    // let error = serde_json::from_str(payload.as_str()

                    let payload = match payload {
                        Payload::String(payload) => payload,
                        Payload::Binary(_) => {
                            log::error!("unexpected binary in invocation-error");
                            return;
                        }
                    };

                    let invocation_error = match serde_json::from_str::<InvocationError>(&payload) {
                        Ok(error) => error,
                        Err(error) => {
                            log::error!(
                                "unable to parse invocation error: {error}, payload = {payload}"
                            );
                            return;
                        }
                    };

                    log::error!("invoaction error: {invocation_error:?}");
                }
                .boxed()
            })
            .connect()
            .await?;

        let client = Self {
            http: reqwest::Client::new(),
            socket,
            sender,
            url,
        };

        client.subscribe().await?;

        Ok((client, receiver))
    }

    /// Subscribe to InvokeAI Socket.IO updates
    async fn subscribe(&self) -> Result<(), Error> {
        self.socket
            .emit("subscribe_queue", json!({"queue_id": "default"}))
            .await
            .map_err(Error::Subscription)
    }

    /// Add an image to the processing queue
    pub async fn enqueue_text_to_image(
        &self,
        input: impl Into<Enqueue>,
        chat_id: ChatId,
        message_id: MessageId,
    ) -> Result<EnqueueResult, Error> {
        let enqueue: Enqueue = input.into();

        let url = self.url.as_str();

        let res = self
            .http
            .post(format!("{url}/api/v1/queue/default/enqueue_batch"))
            .json(&enqueue)
            .send()
            .await?
            .text()
            .await?;

        let enqueued: EnqueueResult = serde_json::from_str(res.as_str()).map_err(|error| {
            log::error!("Failed to parse InvokeAI Response: {error}, payload: {res}");
            error
        })?;

        self.sender
            .send(Update::Started {
                id: enqueued.id(),
                chat_id,
                message_id,
            })
            .ok();

        Ok(enqueued)
    }

    pub async fn download_image(&self, url: String) -> Result<bytes::Bytes, Error> {
        self.http
            .get(url)
            .send()
            .await?
            .bytes()
            .await
            .map_err(Into::into)
    }
}
