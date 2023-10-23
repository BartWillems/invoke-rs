use std::sync::{Arc, OnceLock};

use futures_util::FutureExt;
use rust_socketio::asynchronous::{Client as SocketClient, ClientBuilder as SocketClientBuilder};
use rust_socketio::Payload;
use serde_json::json;
use teloxide::types::{ChatId, MessageId, UserId};

use crate::handler::invoke::{Notifier, Update};
use crate::models::invocations::{InvocationComplete, InvocationError};
use crate::models::{Enqueue, EnqueueResult};

use super::Error;

#[derive(Clone)]
pub struct InvokeAI {
    http: reqwest::Client,
    socket: SocketClient,
    notifier: Notifier,
    url: String,
}

impl InvokeAI {
    /// Connect to the Socket.IO server of the InvokeAI instance
    pub async fn connect(
        url: String,
        notifier: Notifier,
        http_client: reqwest::Client,
    ) -> Result<Self, Error> {
        let socket = Self::construct_socket_io_client(url.clone(), notifier.clone()).await?;

        let client = Self {
            http: http_client,
            socket,
            notifier,
            url,
        };

        client.subscribe().await?;

        Ok(client)
    }

    /// Create a SocketIO websocket connection to the InvokeAI instance
    async fn construct_socket_io_client(
        url: String,
        notifier: Notifier,
    ) -> Result<SocketClient, Error> {
        let url = Arc::new(url);
        SocketClientBuilder::new(format!("{url}/ws/socket.io/"))
            .namespace("/")
            .on("invocation_complete", move |payload, _client| {
                let notifier = notifier.clone();
                let url = url.clone();
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
                                notifier.notify(Update::Progress {
                                    id: invocation.id(),
                                });
                                return;
                            }

                            match invocation.image_path() {
                                Some(path) => {
                                    notifier.notify(Update::Finished {
                                        batch_id: invocation.id(),
                                        image_url: format!("{url}/api/v1/images/i/{path}/full"),
                                    });
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
            .await
            .map_err(Into::into)
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
        enqueue: Box<Enqueue>,
        chat_id: ChatId,
        user_id: UserId,
        message_id: MessageId,
    ) -> Result<EnqueueResult, Error> {
        static CELL: OnceLock<String> = OnceLock::new();

        let url = CELL.get_or_init(|| {
            let url = self.url.as_str();
            format!("{url}/api/v1/queue/default/enqueue_batch")
        });

        let res = self
            .http
            .post(url)
            .json(&enqueue)
            .send()
            .await?
            .text()
            .await?;

        let enqueued: EnqueueResult = serde_json::from_str(res.as_str()).map_err(|error| {
            log::error!("Failed to parse InvokeAI Response: {error}, payload: {res}");
            error
        })?;

        self.notifier.notify(Update::Started {
            id: enqueued.id(),
            chat_id,
            user_id,
            message_id,
        });

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
