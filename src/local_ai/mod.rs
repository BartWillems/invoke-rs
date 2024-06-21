use crate::handler::local::{Identifier, Notifier, ResponseVariant, Update};
use bytes::Bytes;
use lingua::Language;
use models::Response;
use std::sync::Arc;

pub mod models;
pub mod prompts;

pub use models::{Message, Model, Request, Role, TtsRequest};
pub use prompts::Prompts;

#[derive(Clone)]
pub struct LocalAI {
    api_uri: Arc<String>,
    http_client: reqwest::Client,
    notifier: Notifier,
    prompts: Prompts,
}

impl LocalAI {
    pub fn new(
        api_uri: String,
        notifier: Notifier,
        http_client: reqwest::Client,
        prompts: Prompts,
    ) -> Self {
        Self {
            api_uri: Arc::new(api_uri),
            http_client,
            notifier,
            prompts,
        }
    }

    pub fn enqueue_raw_request(&self, identifier: Identifier, request: Request) {
        let client = self.clone();

        tokio::task::spawn(async move {
            match client.request_chat(request).await {
                Ok(resp) => {
                    client.notifier.notify(Update::Finished {
                        identifier,
                        response: resp
                            .message()
                            .map(ResponseVariant::Text)
                            .unwrap_or_default(),
                    });
                }
                Err(err) => client.notifier.notify(Update::Failed {
                    identifier,
                    reason: err.to_string(),
                }),
            };
        });
    }

    /// Request using the global system prompt
    pub async fn enqueue_request(&self, identifier: Identifier, prompt: String, model: Model) {
        let request = match model {
            Model::Tldr => Request::tldr(prompt),
            Model::GgmlGpt4all | Model::Llama => {
                Request::from_prompt(self.prompts.get_prompt().await, prompt)
            }
        };

        log::debug!("request: {request:?}");

        self.enqueue_raw_request(identifier, request);
    }

    pub async fn enqueue_tts_request(
        &self,
        identifier: Identifier,
        prompt: String,
        language: Language,
    ) {
        let client = self.clone();

        let request = TtsRequest::new(prompt, language);

        tokio::task::spawn(async move {
            match client.request_tts(request).await {
                Ok(bytes) => {
                    client.notifier.notify(Update::Finished {
                        identifier,
                        response: ResponseVariant::Audio(bytes),
                    });
                }
                Err(err) => client.notifier.notify(Update::Failed {
                    identifier,
                    reason: err.to_string(),
                }),
            };
        });
    }

    async fn request_chat(&self, request: Request) -> Result<Response, anyhow::Error> {
        let res = self
            .http_client
            .post(format!("{}/v1/chat/completions", self.api_uri))
            .json(&request)
            .send()
            .await?
            .text()
            .await?;

        log::debug!("Response: {res:?}");

        serde_json::from_str(res.as_str()).map_err(Into::into)
    }

    async fn request_tts(&self, request: TtsRequest) -> Result<Bytes, anyhow::Error> {
        self.http_client
            .post(format!("{}/tts", self.api_uri))
            .json(&request)
            .send()
            .await?
            .bytes()
            .await
            .map_err(Into::into)
    }
}
