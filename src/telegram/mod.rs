use teloxide::prelude::Update as TelegramUpdate;
use teloxide::prelude::*;
use teloxide::types::UserId;

use crate::handler::invoke;
use crate::handler::local;
use crate::handler::Store;
use crate::local_ai::Prompts;
use crate::utils::languages::LanguageDetector;

pub mod admin;
mod invoke_ai;
mod local_ai;

#[derive(Clone, Copy)]
pub struct Config {
    pub admin_id: Option<UserId>,
}

#[derive(Clone)]
pub struct Context {
    pub cfg: Config,
    pub bot: Bot,
    pub store: Store,
    pub invoke_notifier: invoke::Notifier,
    pub local_notifier: local::Notifier,
    pub language: LanguageDetector,
    pub prompts: Prompts,
}

pub fn handler(
    context: Context,
) -> Dispatcher<Bot, teloxide::RequestError, teloxide::dispatching::DefaultKey> {
    let overrides = admin::Overrides::default();

    let handler = TelegramUpdate::filter_message()
        .branch(
            dptree::filter(|ctx: Context, msg: Message| match ctx.cfg.admin_id {
                Some(admin_user_id) => msg
                    .from()
                    .map(|user| user.id == admin_user_id)
                    .unwrap_or_default(),
                None => false,
            })
            .filter_command::<admin::AdminCommands>()
            .endpoint(admin::handler),
        )
        .branch(
            dptree::entry()
                .filter_command::<invoke_ai::Command>()
                .endpoint(invoke_ai::handler),
        )
        .branch(
            dptree::entry()
                .filter_command::<local_ai::Command>()
                .endpoint(local_ai::handler),
        )
        .branch(dptree::entry().endpoint(catch_all));

    Dispatcher::builder(context.bot.clone(), handler)
        .dependencies(dptree::deps![context, overrides])
        .default_handler(|_| async {})
        .build()
}

async fn catch_all(ctx: Context, msg: Message) -> Result<(), teloxide::RequestError> {
    let (_store, _french) = tokio::join!(
        tokio::task::spawn(store_message(ctx.clone(), msg.clone())),
        tokio::task::spawn(detect_french(ctx.clone(), msg.clone()))
    );

    Ok(())
}

/// Respond "wablieft?" if the message is french
async fn detect_french(ctx: Context, msg: Message) {
    let Some(txt) = msg.text() else {
        return;
    };

    if txt.is_empty() {
        return;
    }

    if ctx.language.has_french(txt.to_string()) {
        ctx.bot
            .send_message(msg.chat.id, "wablieft?")
            .reply_to_message_id(msg.id)
            .send()
            .await
            .inspect_err(|err| log::error!("failed to respond wablieft: {err}"))
            .ok();
    }
}

async fn store_message(ctx: Context, msg: Message) {
    ctx.store
        .store_message(msg)
        .await
        .inspect_err(|err| log::error!("failed to store message: {err}"))
        .ok();
}
