use futures_util::FutureExt;
use rust_socketio::asynchronous::{Client as SocketClient, ClientBuilder as SocketClientBuilder};
use rust_socketio::Payload;
use serde_json::json;
use std::error::Error;
use teloxide::types::{ChatId, MessageId};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use crate::models::InvocationComplete;
use crate::models::{Enqueue, EnqueueResult};
use crate::Update;

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
    pub async fn connect(url: String) -> (Self, UnboundedReceiver<Update>) {
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
                        Payload::String(content) => {
                            let invocation: InvocationComplete =
                                serde_json::from_str(content.as_str()).unwrap();

                            if invocation.still_in_progress() {
                                sender
                                    .send(Update::Progress {
                                        id: invocation.id(),
                                    })
                                    .unwrap();
                                return;
                            }

                            match invocation.image_path() {
                                Some(path) => {
                                    sender
                                        .send(Update::Finished {
                                            id: invocation.id(),
                                            image_url: format!("{url}/api/v1/images/i/{path}/full"),
                                        })
                                        .unwrap();
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
            .connect()
            .await
            .unwrap();

        socket
            .emit("subscribe_queue", json!({"queue_id": "default"}))
            .await
            .unwrap();

        let client = Self {
            http: reqwest::Client::new(),
            socket,
            sender,
            url,
        };

        (client, receiver)
    }

    /// Add an image to the processing queue
    pub async fn enqueue_text_to_image(
        &self,
        prompt: impl Into<String>,
        chat_id: ChatId,
        message_id: MessageId,
    ) -> Result<EnqueueResult, Box<dyn Error>> {
        let enqueue: Enqueue = Enqueue::from_prompt(prompt);

        let url = self.url.as_str();

        let res = self
            .http
            .post(format!("{url}/api/v1/queue/default/enqueue_batch"))
            .json(&enqueue)
            .send()
            .await?
            .json::<EnqueueResult>()
            .await?;

        self.sender
            .send(Update::Started {
                id: res.id(),
                chat_id,
                message_id,
            })
            .ok();

        Ok(res)
    }

    pub async fn download_image(&self, url: String) -> Result<bytes::Bytes, Box<dyn Error>> {
        self.http
            .get(url)
            .send()
            .await?
            .bytes()
            .await
            .map_err(Into::into)
    }
}
