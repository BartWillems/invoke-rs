use thiserror::Error;

pub mod client;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to call InvokeAI {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Failed to connecto to SocketIO {0}")]
    SocketIO(#[from] rust_socketio::Error),
    #[error("Failed to subscribe to SocketIO queue {0}")]
    Subscription(rust_socketio::Error),
    #[error("Failed to decode JSON {0}")]
    JsonDecode(#[from] serde_json::Error),
}
