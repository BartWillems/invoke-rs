use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct Ollama {
    api_uri: Arc<String>,
    http_client: reqwest::Client,
}

#[derive(Debug, Serialize)]
pub struct Request {
    model: Models,
    prompt: String,
    stream: bool,
}

#[derive(Clone, Copy, Debug, Serialize)]
enum Models {
    #[serde(rename = "phi3")]
    Phi3,
}

#[derive(Debug, Deserialize)]
pub struct Response {
    pub model: String,
    pub created_at: String,
    pub response: String,
    pub done: bool,
    /// time spent generating the response
    pub total_duration: Option<usize>,
    /// time spent in nanoseconds loading the model
    pub load_duration: Option<usize>,
    /// number of tokens in the prompt
    pub prompt_eval_count: Option<usize>,
    /// time spent in nanoseconds evaluating the prompt
    pub prompt_eval_duration: Option<usize>,
    /// number of tokens in the response
    pub eval_count: Option<usize>,
    /// time in nanoseconds spent generating the response
    pub eval_duration: Option<usize>,
}

impl Ollama {
    pub fn new(http_client: reqwest::Client, api_uri: String) -> Self {
        Self {
            api_uri: Arc::new(api_uri),
            http_client,
        }
    }

    pub async fn request_completion(&self, prompt: String) -> anyhow::Result<Response> {
        let res = self
            .http_client
            .post(format!("{}/api/generate", self.api_uri.as_str()))
            .json(&Request {
                prompt,
                model: Models::Phi3,
                stream: false,
            })
            .send()
            .await?
            .text()
            .await?;

        let response = serde_json::from_str(res.as_str())?;

        Ok(response)
    }
}
