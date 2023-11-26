#![allow(unused)]

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize)]
pub struct Request {
    model: Model,
    messages: Vec<Message>,
    temperature: f32,
    max_tokens: usize,
}

impl Request {
    pub fn from_prompt(prompt: String) -> Self {
        Self {
            model: Model::Llama,
            messages: vec![Message {
                role: Role::User,
                content: prompt,
            }],
            temperature: 0.7,
            max_tokens: 750,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Response {
    object: String,
    model: Model,
    choices: Vec<Choice>,
    usage: Usage,
}

impl Response {
    pub fn message(&self) -> Option<String> {
        self.choices
            .first()
            .map(|choice| choice.message.content.clone())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
enum Model {
    /// CPU based
    #[serde(rename = "ggml-gpt4all-j.bin")]
    GgmlGpt4all,
    /// GPU based
    #[serde(rename = "llama")]
    Llama,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    role: Role,
    content: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Role {
    User,
    Assistant,
}

#[derive(Clone, Debug, Deserialize)]
struct Choice {
    index: usize,
    finish_reason: String,
    message: Message,
}

#[derive(Clone, Debug, Deserialize)]
struct Usage {
    prompt_tokens: usize,
    completion_tokens: usize,
    total_tokens: usize,
}
