use std::sync::{Arc, OnceLock};

use futures_util::FutureExt;
use rust_socketio::asynchronous::{
    Client as SocketClient, ClientBuilder as SocketClientBuilder, ReconnectSettings,
};
use rust_socketio::Payload;
use serde_json::json;
use teloxide::types::{ChatId, MessageId, UserId};

use crate::handler::invoke::{Notifier, Update};
use crate::invoke_ai::models::invocations::{InvocationComplete, InvocationError};
use crate::invoke_ai::models::{Enqueue, EnqueueResult};

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

        let reconnect_url = Arc::new(format!("{url}/ws/socket.io/"));

        SocketClientBuilder::new(reconnect_url.as_str())
            .namespace("/")
            .on("invocation_complete", move |payload, _client| {
                let notifier = notifier.clone();
                let url = url.clone();
                async move {
                    match payload {
                        Payload::Text(mut payload) => {
                            let Some(value) = payload.pop() else {
                                log::error!("empty payload array received");
                                return;
                            };

                            let invocation: InvocationComplete = match serde_json::from_value(value)
                            {
                                Ok(invocation) => invocation,
                                Err(error) => {
                                    log::error!("Failed to parse SocketIO JSON: {error}");
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
                        rest => log::warn!("unexpected data: {rest:?}"),
                    }
                }
                .boxed()
            })
            .on("invocation_error", |payload, _client| {
                async move {
                    let error = match payload {
                        Payload::Text(mut payload) => payload.pop(),
                        rest => {
                            log::warn!("unexpected data in invocation-error: {rest:?}");
                            return;
                        }
                    };

                    let Some(error) = error else {
                        log::warn!("empty error received");
                        return;
                    };

                    let invocation_error = match serde_json::from_value::<InvocationError>(error) {
                        Ok(error) => error,
                        Err(error) => {
                            log::error!(
                                "unable to parse invocation error: {error}, payload = {error}"
                            );
                            return;
                        }
                    };

                    log::error!("invoaction error: {invocation_error:?}");
                }
                .boxed()
            })
            .on_reconnect(move || {
                let url = reconnect_url.clone();
                async move {
                    let mut settings = ReconnectSettings::new();
                    settings.address(format!("{url}/ws/socket.io/"));
                    settings.auth(json!({"queue_id": "default"}));
                    settings
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

    /// Download image bytes from a given URL
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
