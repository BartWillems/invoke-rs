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
pub struct Request<'a> {
    model: &'a Model,
    prompt: String,
    stream: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct Model(String);

impl Model {
    pub fn context_length(&self) -> usize {
        4096
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
                model: &self.model,
                stream: false,
            })
            .send()
            .await?
            .text()
            .await?;

        let response = serde_json::from_str(res.as_str()).map(Self::filter_think_tags)?;

        Ok(response)
    }

    fn filter_think_tags(mut response: Response) -> Response {
        const CLOSE_TAG: &'static str = "</think>";

        if !response.model.as_str().contains("deepseek-r1") {
            return response;
        }

        if !response.response.as_str().starts_with("<think>") {
            return response;
        }

        let Some(stop_idx) = response.response.as_str().find(CLOSE_TAG) else {
            return response;
        };

        if response.response.len() > stop_idx + CLOSE_TAG.len() {
            response.response = response.response[(stop_idx + CLOSE_TAG.len())..].to_string();
        }

        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_think_tags() {
        let response = Response {
            response: "<think>asldkl</think>the rest of the content".into(),
            model: "deepseek-r1".into(),
            created_at: "foo".into(),
            done: true,
            eval_count: None,
            total_duration: None,
            load_duration: None,
            prompt_eval_count: None,
            prompt_eval_duration: None,
            eval_duration: None,
        };

        let filtered = Ollama::filter_think_tags(response);
        assert_eq!(filtered.response, "the rest of the content".to_string());

        // do it again just to be sure
        let filtered = Ollama::filter_think_tags(filtered);
        assert_eq!(filtered.response, "the rest of the content".to_string());

        // without other content
        let response = Ollama::filter_think_tags(Response {
            response: "<think>some thoughts without a result</think>".to_string(),
            ..filtered
        });
        assert_eq!(
            response.response,
            "<think>some thoughts without a result</think>".to_string()
        );
    }
}
