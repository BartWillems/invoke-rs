use teloxide::{prelude::*, utils::command::BotCommands};

use crate::handler::local::{Identifier, Update};

use super::Context;

#[derive(BotCommands, Clone, Debug)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
pub enum Command {
    Say(String),
}

impl Command {
    fn override_prompt(&mut self, prompt: impl ToString) {
        match self {
            Command::Say(_) => *self = Command::Say(prompt.to_string()),
        }
    }
}

pub async fn handler(
    ctx: Context,
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

    if user.id == UserId(172179034) {
        command.override_prompt("What are the hazards of driving on a flat tire?");
    }

    match command {
        Command::Say(prompt) => {
            let prompt = msg
                .reply_to_message()
                .and_then(|message| message.text())
                .map(ToString::to_string)
                .unwrap_or(prompt);

            let language = ctx.language.detect_language(&prompt);

            ctx.local_notifier.notify(Update::TtsRequest {
                identifier: Identifier {
                    chat_id: msg.chat.id,
                    user_id: user.id,
                    message_id: msg.id,
                },
                prompt,
                language,
            })
        }
    };

    Ok(())
}
