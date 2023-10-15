use teloxide::prelude::Update as TelegramUpdate;
use teloxide::prelude::*;
use teloxide::types::UserId;
use tokio::sync::mpsc::UnboundedSender;

use crate::handler::Update;

mod admin;
mod invoke_ai;

#[derive(Clone, Copy)]
pub struct Config {
    pub admin_id: Option<UserId>,
}

pub fn handler(
    bot: Bot,
    sender: UnboundedSender<Update>,
    cfg: Config,
) -> Dispatcher<Bot, teloxide::RequestError, teloxide::dispatching::DefaultKey> {
    let overrides = admin::Overrides::default();

    let handler = TelegramUpdate::filter_message()
        .branch(
            dptree::filter(|cfg: Config, msg: Message| match cfg.admin_id {
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
        );

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![sender, overrides, cfg])
        .default_handler(|_| async {})
        .build()
}
