use teloxide::Bot;

pub mod invoke;
pub mod local;
pub mod ollama;
pub mod store;

pub use store::Store;

use crate::local_ai::Prompts;
use crate::utils::languages::LanguageDetector;
use crate::AppConfig;

pub struct Handler;

impl Handler {
    pub async fn dispatch(config: AppConfig) -> anyhow::Result<()> {
        let AppConfig {
            invoke_ai_url,
            local_ai_url,
            teloxide_token,
            ollama_url,
            telegram_admin_user_id,
            max_in_progress,
            sqlite_path,
            enable_french_detection,
        } = config;

        let bot = Bot::new(teloxide_token);

        let http_client = reqwest::Client::new();

        let invoke = invoke::Handler::try_new(
            invoke::Config {
                invoke_ai_url,
                max_in_progress,
            },
            bot.clone(),
            http_client.clone(),
        )
        .await?;

        let prompts = Prompts::default();

        let local = local::Handler::try_new(
            local::Config {
                local_ai_url,
                max_in_progress,
            },
            bot.clone(),
            http_client.clone(),
            prompts.clone(),
        )?;

        let ollama = ollama::Handler::try_new(
            ollama::Config {
                api_uri: ollama_url,
                max_in_progress,
            },
            bot.clone(),
            http_client,
        )?;

        let store = Store::new(&sqlite_path, bot.clone()).await?;

        let mut telegram = crate::telegram::handler(crate::telegram::Context {
            cfg: crate::telegram::Config {
                admin_id: telegram_admin_user_id,
            },
            bot,
            store,
            invoke_notifier: invoke.notifier(),
            local_notifier: local.notifier(),
            ollama_notifier: ollama.notifier(),
            language: LanguageDetector::new(enable_french_detection),
            prompts,
        });

        log::info!("Starting all handlers...");
        futures::future::join4(
            invoke.start(),
            local.start(),
            ollama.start(),
            telegram.dispatch(),
        )
        .await;

        Ok(())
    }
}
