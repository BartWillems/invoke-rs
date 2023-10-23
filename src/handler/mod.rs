use std::num::NonZeroUsize;

use serde::Deserialize;
use teloxide::{types::UserId, Bot};

pub mod invoke;
pub mod local;

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct Config {
    invoke_ai_url: String,
    local_ai_url: String,
    teloxide_token: String,
    telegram_admin_user_id: Option<UserId>,
    max_in_progress: Option<NonZeroUsize>,
}

pub struct Handler;

impl Handler {
    pub async fn dispatch(config: Config) -> anyhow::Result<()> {
        let Config {
            invoke_ai_url,
            local_ai_url,
            teloxide_token,
            telegram_admin_user_id,
            max_in_progress,
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

        let local = local::Handler::try_new(
            local::Config {
                local_ai_url,
                max_in_progress,
            },
            bot.clone(),
            http_client,
        )?;

        let mut telegram = crate::telegram::handler(
            bot,
            invoke.notifier(),
            local.notifier(),
            crate::telegram::Config {
                admin_id: telegram_admin_user_id,
            },
        );

        log::info!("Starting all handlers...");
        futures::future::join3(invoke.start(), local.start(), telegram.dispatch()).await;

        Ok(())
    }
}
