use std::num::NonZeroUsize;

use config::Config;
use serde::Deserialize;
use teloxide::types::UserId;

pub mod handler;
pub mod invoke_ai;
pub mod local_ai;
pub mod ollama;
pub mod store;
pub mod telegram;
pub mod utils;

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    invoke_ai_url: String,
    local_ai_url: String,
    ollama_url: String,
    teloxide_token: String,
    telegram_admin_user_id: Option<UserId>,
    max_in_progress: Option<NonZeroUsize>,
    sqlite_path: String,
    #[serde(default)]
    enable_french_detection: bool,
    #[serde(default)]
    ollama_model: ollama::Model,
    searxng_url: String,
    fact_check_path: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    env_logger::init();

    log::info!("Loading config...");
    let config = Config::builder()
        .add_source(config::Environment::with_prefix("app").try_parsing(true))
        .build()?;

    let config: AppConfig = config.try_deserialize()?;

    log::info!("Initializing...");
    handler::Handler::dispatch(config).await?;

    Ok(())
}
