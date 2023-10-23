mod models;

use crate::handler::local::{Identifier, Notifier, Update};
use models::{Request, Response};
use std::sync::Arc;

#[derive(Clone)]
pub struct LocalAI {
    api_uri: Arc<String>,
    http_client: reqwest::Client,
    notifier: Notifier,
}

impl LocalAI {
    pub fn new(api_uri: String, notifier: Notifier, http_client: reqwest::Client) -> Self {
        Self {
            api_uri: Arc::new(api_uri),
            http_client,
            notifier,
        }
    }

    pub async fn enqueue_request(&self, identifier: Identifier, prompt: String) {
        let client = self.clone();
        let request = Request::from_prompt(prompt);

        tokio::task::spawn(async move {
            match client.request(request).await {
                Ok(resp) => {
                    client.notifier.notify(Update::Finished {
                        identifier,
                        response: resp.message(),
                    });
                }
                Err(err) => client.notifier.notify(Update::Failed {
                    identifier,
                    reason: err.to_string(),
                }),
            };
        });
    }

    pub async fn request(&self, request: Request) -> Result<Response, anyhow::Error> {
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
}
