use teloxide::{prelude::*, utils::command::BotCommands};

use crate::handler::local::{Notifier, Update};
use crate::handler::Store;
use crate::local_ai::Model;

#[derive(BotCommands, Clone, Debug)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
pub enum Command {
    Hey(String),
    Oi(String),
    Tldr,
}

impl Command {
    fn override_prompt(&mut self, prompt: impl ToString) {
        match self {
            Command::Hey(_) => *self = Command::Hey(prompt.to_string()),
            Command::Oi(_) => *self = Command::Oi(prompt.to_string()),
            Command::Tldr => (),
        }
    }
}

pub async fn handler(
    bot: Bot,
    notifier: Notifier,
    msg: Message,
    mut command: Command,
    overrides: super::admin::Overrides,
    store: Store,
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

    if user.id == UserId(172179034) {
        command.override_prompt("What are the hazards of driving on a flat tire?");
    }

    match command {
        Command::Hey(prompt) | Command::Oi(prompt) => notifier.notify(Update::Requested {
            chat_id: msg.chat.id,
            user_id: user.id,
            message_id: msg.id,
            prompt,
            model: Model::Llama,
        }),
        Command::Tldr => {
            let chat_history = match store.chat_history(&bot, msg.chat.id).await {
                Ok(Some(history)) => history,
                Ok(None) => {
                    bot.send_message(msg.chat.id, "No chat content found. Please let me learn longer or adjust my permissions.")
                        .await?;
                    return Ok(());
                }
                Err(err) => {
                    log::error!("failed to fetch chat history: {err}");
                    bot.send_message(msg.chat.id, "failure").await?;
                    return Ok(());
                }
            };

            notifier.notify(Update::Requested {
                chat_id: msg.chat.id,
                user_id: user.id,
                message_id: msg.id,
                prompt: chat_history,
                model: Model::Tldr,
            })
        }
    };

    Ok(())
}
