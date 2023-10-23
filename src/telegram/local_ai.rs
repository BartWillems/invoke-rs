use teloxide::{prelude::*, utils::command::BotCommands};

use crate::handler::local::{Notifier, Update};

#[derive(BotCommands, Clone, Debug)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
pub enum Command {
    Hey(String),
    Oi(String),
}

impl Command {
    fn override_prompt(&mut self, prompt: impl ToString) {
        match self {
            Command::Hey(_) => *self = Command::Hey(prompt.to_string()),
            Command::Oi(_) => *self = Command::Oi(prompt.to_string()),
        }
    }
}

pub async fn handler(
    notifier: Notifier,
    msg: Message,
    mut command: Command,
    overrides: super::admin::Overrides,
) -> Result<(), teloxide::RequestError> {
    log::info!("Received command: {command:?}, Chat ID: {}", msg.chat.id);

    let user = match msg.from() {
        Some(user) => user,
        None => {
            log::warn!("Received a command without a user");
            return Ok(());
        }
    };

    if let Some(prompt) = overrides.get_override(user.id).await {
        command.override_prompt(prompt);
    }

    match command {
        Command::Hey(prompt) | Command::Oi(prompt) => notifier.notify(Update::Requested {
            chat_id: msg.chat.id,
            user_id: user.id,
            message_id: msg.id,
            prompt,
        }),
    };

    Ok(())
}
