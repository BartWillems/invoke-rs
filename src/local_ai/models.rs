#![allow(unused)]

use lingua::Language;
use serde::{Deserialize, Serialize};

/// Text generation request
#[derive(Clone, Debug, Serialize)]
pub struct Request {
    pub model: Model,
    pub messages: Vec<Message>,
    pub temperature: f32,
}

impl Request {
    pub fn from_prompt(system: String, user: String) -> Self {
        Self {
            model: Model::Llama,
            messages: vec![
                Message {
                    role: Role::System,
                    content: system,
                },
                Message {
                    role: Role::User,
                    content: user,
                },
            ],
            temperature: 0.7,
        }
    }

    pub fn tldr(content: String) -> Self {
        Self {
            model: Model::Tldr,
            messages: vec![Message {
                role: Role::User,
                content,
            }],
            temperature: 0.7,
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
        self.choices.first().map(|choice| {
            choice
                .message
                .content
                .replace("<|assistant|>", "")
                .replace("<|end|>", "")
        })
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Model {
    /// CPU based
    #[serde(rename = "ggml-gpt4all-j.bin")]
    GgmlGpt4all,
    /// GPU based
    #[serde(rename = "llama")]
    Llama,
    /// Also Lama in the background
    #[serde(rename = "tldr")]
    Tldr,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
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

/// Text to speech
#[derive(Debug, Serialize)]
pub struct TtsRequest {
    model: TtsModel,
    backend: TtsBackend,
    input: String,
}

#[derive(Debug, Serialize)]
pub enum TtsModel {
    #[serde(rename = "en-us-libritts-high.onnx")]
    EnUsLibritts,
    #[serde(rename = "nl-nathalie-x-low.onnx")]
    NlNathalie,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TtsBackend {
    Piper,
}

impl TtsRequest {
    pub fn new(mut input: String, language: Language) -> Self {
        let model = match language {
            Language::Dutch => TtsModel::NlNathalie,
            Language::English => TtsModel::EnUsLibritts,
            Language::French => {
                input = String::from("wablieft?");
                TtsModel::NlNathalie
            }
        };

        Self {
            model,
            backend: TtsBackend::Piper,
            input,
        }
    }
}
