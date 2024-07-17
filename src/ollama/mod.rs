use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub mod prompts;

#[derive(Clone)]
pub struct Ollama {
    api_uri: Arc<String>,
    http_client: reqwest::Client,
    model: Model,
}

#[derive(Debug, Serialize)]
pub struct Request {
    model: Model,
    prompt: String,
    stream: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub enum Model {
    #[serde(rename = "phi3:mini-128k")]
    Phi3Mini128k,
    #[serde(rename = "llama3:8b")]
    Llama3,
    #[serde(rename = "aya:8b")]
    Aya,
    #[serde(rename = "mistral:7b")]
    Mistral,
    #[default]
    #[serde(rename = "qwen2:7b")]
    Qwen2,
    #[serde(rename = "gemma2:9b")]
    Gemma2,
}

impl Model {
    pub fn context_length(&self) -> usize {
        match self {
            Model::Phi3Mini128k => 131072,
            Model::Llama3 => 8192,
            Model::Aya => 8192,
            Model::Mistral => 32768,
            Model::Qwen2 => 32768,
            Model::Gemma2 => 8192,
        }
    }
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
    pub fn new(http_client: reqwest::Client, api_uri: String, model: Model) -> Self {
        Self {
            api_uri: Arc::new(api_uri),
            http_client,
            model,
        }
    }

    pub async fn request_completion(&self, prompt: String) -> anyhow::Result<Response> {
        let res = self
            .http_client
            .post(format!("{}/api/generate", self.api_uri.as_str()))
            .json(&Request {
                prompt,
                model: self.model,
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
